//! Scripted checks on the player and on-screen instructor hints.
//!
//! * `ai_script_conditions`: once enabled, waits until the player has looked
//!   at its target (within `PlayerTargetFOV`, optionally in line of sight)
//!   for `RequiredTime` seconds, then fires `OnConditionsSatisfied`. The
//!   intro's "look at the ceiling / the floor / the art" steps run on these.
//! * `env_instructor_hint`: `ShowHint` puts its caption and key on screen
//!   until `EndHint` or its timeout (0: none), drawn as Portal 2's game instructor
//!   does: the icon (key cap) and a white InstructorTitle caption, no box,
//!   over its `hint_target` or, when static, under the crosshair.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::campaign::Campaign;
use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::level::LevelLoaded;
use crate::ui::UiFonts;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hints>()
            .init_resource::<HintIcons>()
            .add_systems(Update, (script_conditions, hint_inputs, draw_hints, place_hints).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

/// Seconds each enabled condition has been satisfied in a row.
#[derive(Default)]
struct Held(std::collections::HashMap<usize, f32>);

#[allow(clippy::too_many_arguments)]
fn script_conditions(
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    players: Query<(Entity, &InterpolatedFeet, &Crouch, &ViewAngles), With<Player>>,
    spatial: SpatialQuery,
    mut held: Local<Held>,
    mut fire: MessageWriter<FireOutput>,
    mut fired: Local<std::collections::HashSet<usize>>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        held.0.clear();
        fired.clear();
    }
    let Some(ents) = ents else { return };
    let Ok((player, feet, crouch, view)) = players.single() else { return };
    let eye = feet.curr + Vec3::Y * crouch.eye_height;
    let fwd = view.forward();
    for (i, e) in ents.list.iter().enumerate() {
        if e.class != "ai_script_conditions" || !e.enabled || e.killed || fired.contains(&i) {
            continue;
        }
        // 1 = must be true, 0 = must be false, 2 = don't care (Source's tristate).
        let tri = |k: &str| e.get(k).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(2);
        let target = e.get("target").and_then(|t| ents.find(t, i, Some(PLAYER)).first().copied());
        let mut ok = true;
        if let Some(t) = target {
            let to = ents.list[t].transform.translation - eye;
            let fov = e.get_f32("playertargetfov").unwrap_or(360.0);
            let true_cone = e.get("playertargetfovtruecone").is_some_and(|v| v.trim() == "1");
            ok &= in_fov(fwd, to, fov, true_cone);
            if let Some(r) = e.get_f32("playertargetproximity").filter(|r| *r > 0.0) {
                ok &= (ents.list[t].transform.translation - feet.curr).length() <= r * shared::SOURCE_UNIT;
            }
            let los = tri("playertargetlos");
            if los != 2 {
                let dist = to.length();
                let filter = SpatialQueryFilter::from_excluded_entities([player]);
                let clear = spatial.cast_ray(eye, Dir3::new(to).unwrap_or(Dir3::Y), (dist - 0.1).max(0.0), true, &filter).is_none();
                ok &= clear == (los == 1);
            }
        }
        let need = e.get_f32("requiredtime").unwrap_or(0.0);
        let t = held.0.entry(i).or_insert(0.0);
        if ok {
            *t += time.delta_secs();
            if *t >= need {
                fired.insert(i);
                fire.write(FireOutput { ent: i, event: "OnConditionsSatisfied".into(), activator: Some(PLAYER) });
            }
        } else {
            *t = 0.0;
        }
    }
}

/// Is a point in direction `to` inside a view cone of `fov` degrees around
/// `fwd`? A negative `fov` asks the opposite, outside a cone of `-fov`
/// degrees (sp_a1_intro7: Wheatley waits until the player turns away,
/// PlayerTargetFOV -100). Without `true_cone` only horizontal directions
/// count, as in Source.
fn in_fov(fwd: Vec3, to: Vec3, fov: f32, true_cone: bool) -> bool {
    if fov.abs() >= 360.0 {
        return true;
    }
    let flat = |v: Vec3| if true_cone { v } else { Vec3::new(v.x, 0.0, v.z) };
    let cos = flat(fwd).normalize_or_zero().dot(flat(to).normalize_or_zero());
    if fov >= 0.0 {
        cos >= (fov * 0.5).to_radians().cos()
    } else {
        cos < (-fov * 0.5).to_radians().cos()
    }
}

/// An instructor hint on screen.
struct Hint {
    ent: usize,
    caption: String,
    /// The bound key's name ("E"), empty for none.
    key: String,
    until: f32,
    /// `hint_icon_onscreen` (`use_binding` = the key cap).
    icon: String,
    /// `hint_target` when not `hint_static`: the hint floats over it.
    target: Option<usize>,
    /// `hint_icon_offset`: Source units above the target.
    offset: f32,
}

#[derive(Resource, Default)]
struct Hints(Vec<Hint>);

/// The game instructor's icons (`scripts/instructor_textures.txt`): each a
/// region of an icon sheet texture, loaded when first shown.
#[derive(Resource, Default)]
struct HintIcons {
    loaded: bool,
    /// icon name -> (sheet, x, y, width, height)
    regions: std::collections::HashMap<String, (String, f32, f32, f32, f32)>,
    /// sheet -> (image, width, height)
    sheets: std::collections::HashMap<String, Option<(Handle<Image>, f32, f32)>>,
}

impl HintIcons {
    fn load(&mut self, game: &p2_assets::GameFsRes) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(text) = game.read("scripts/instructor_textures.txt") else {
            warn!("portal2: scripts/instructor_textures.txt not found, hints without icons");
            return;
        };
        let kv = source_assets::kv::parse(&String::from_utf8_lossy(&text));
        // Icons are blocks with a "file" anywhere under the root
        // ("sprites/640_hud" { "TextureData" { "icon_tip" { ... } } }).
        fn walk(kv: &source_assets::kv::Kv, out: &mut std::collections::HashMap<String, (String, f32, f32, f32, f32)>) {
            for (name, v) in &kv.entries {
                let source_assets::kv::KvValue::Block(b) = v else { continue };
                match (b.get("file"), b.get_f32("x"), b.get_f32("y"), b.get_f32("width"), b.get_f32("height")) {
                    (Some(file), Some(x), Some(y), Some(w), Some(h)) => {
                        out.insert(name.to_ascii_lowercase(), (file.to_ascii_lowercase(), x, y, w, h));
                    }
                    _ => walk(b, out),
                }
            }
        }
        walk(&kv, &mut self.regions);
        debug!("portal2: {} instructor icons", self.regions.len());
    }

    /// An icon's image and its pixel rectangle.
    fn icon(&mut self, name: &str, game: &p2_assets::GameFsRes, images: &mut Assets<Image>) -> Option<(Handle<Image>, Rect)> {
        let Some((file, x, y, w, h)) = self.regions.get(&name.to_ascii_lowercase()).cloned() else {
            debug!("portal2: no instructor icon {name} ({} known)", self.regions.len());
            return None;
        };
        let sheet = self
            .sheets
            .entry(file.clone())
            .or_insert_with(|| {
                let vtf = source_assets::vtf::VtfTexture::parse(&game.read(&format!("materials/{file}.vtf"))?).ok()?;
                let (sw, sh) = (vtf.width as f32, vtf.height as f32);
                Some((images.add(vtf.into_image(false, true)), sw, sh))
            })
            .clone()?;
        let _ = sheet.2;
        Some((sheet.0, Rect::new(x, y, x + w, y + h)))
    }
}

/// A `+duck`-style binding as the key the player presses.
fn binding_key(b: &str) -> &'static str {
    match b.trim().trim_start_matches('+').to_ascii_lowercase().as_str() {
        "duck" => "CTRL",
        "jump" => "SPACE",
        "use" => "E",
        "attack" => "LEFT MOUSE",
        "attack2" => "RIGHT MOUSE",
        "forward" | "back" | "moveleft" | "moveright" => "W A S D",
        "zoom" => "Z",
        _ => "",
    }
}

fn hint_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    campaign: Res<Campaign>,
    mut hints: ResMut<Hints>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        hints.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "env_instructor_hint" {
            continue;
        }
        match msg.input.as_str() {
            "showhint" => {
                let caption = campaign.text(e.get("hint_caption").unwrap_or(""));
                let key = binding_key(e.get("hint_binding").unwrap_or("")).to_string();
                let until = hint_until(ents.now, e.get_f32("hint_timeout"));
                hints.0.retain(|h| h.ent != msg.target);
                let icon = e.get("hint_icon_onscreen").unwrap_or("icon_tip").to_string();
                let target = if e.get("hint_static").is_some_and(|v| v.trim() == "0") {
                    ents.find(e.get("hint_target").unwrap_or(""), msg.target, msg.activator).into_iter().next()
                } else {
                    None
                };
                let offset = e.get_f32("hint_icon_offset").unwrap_or(0.0);
                hints.0.push(Hint { ent: msg.target, caption, key, until, icon, target, offset });
            }
            "endhint" | "kill" => hints.0.retain(|h| h.ent != msg.target),
            _ => {}
        }
    }
    // Only touch the list when one times out: a change rebuilds the rows,
    // and rows rebuilt every frame are never laid out.
    let now = ents.now;
    if hints.0.iter().any(|h| h.until <= now) {
        hints.0.retain(|h| h.until > now);
    }
}

/// When a hint shown at `now` goes: `hint_timeout` seconds later, or never
/// for 0 or no timeout (the FGD default: "0 will persist until stopped with
/// EndHint").
fn hint_until(now: f32, timeout: Option<f32>) -> f32 {
    match timeout {
        Some(t) if t > 0.0 => now + t,
        _ => f32::INFINITY,
    }
}

/// One hint's row; the index into [`Hints`].
#[derive(Component)]
struct HintRow(usize);

/// One vgui unit (480 units = the screen height) in `Vh`.
const UNIT_VH: f32 = 100.0 / 480.0;
/// The instructor's icon: 32 units square (measured on Portal 2's "Pick Up"
/// and "Create Blue Portal" hints, 30 px on a 450 px tall screen).
const HINT_ICON: f32 = 32.0;
/// InstructorTitle: UniversLTStd-BoldCn tall 18 (basemodui_scheme.res);
/// vgui `tall` is the Windows cell, 1.265 em for Univers Cn.
const HINT_CAPTION: f32 = 18.0 / 1.265;
/// InstructorKeyBindings on PC HiDef: UniversLTStd-BoldCn tall 12.
const HINT_KEY: f32 = 12.0 / 1.265;
/// Static hints sit under the crosshair: Portal 2's "Create Blue Portal"
/// hint is centred two thirds of the way down the screen.
const HINT_STATIC_Y: f32 = 2.0 / 3.0;

#[allow(clippy::too_many_arguments)]
fn draw_hints(
    mut commands: Commands,
    hints: Res<Hints>,
    fonts: Option<Res<UiFonts>>,
    rows: Query<Entity, With<HintRow>>,
    game: Res<p2_assets::GameFsRes>,
    mut icons: ResMut<HintIcons>,
    mut images: ResMut<Assets<Image>>,
) {
    if !hints.is_changed() {
        return;
    }
    let Some(fonts) = fonts else { return };
    icons.load(&game);
    for r in &rows {
        commands.entity(r).despawn();
    }
    let vh = |u: f32| Val::Vh(u * UNIT_VH);
    let font = |u: f32| TextFont { font: fonts.title.clone().into(), font_size: bevy::text::FontSize::Vh(u * UNIT_VH), ..default() };
    for (i, hint) in hints.0.iter().enumerate() {
        // A key hint shows the key on the instructor's key cap; others
        // their own icon (the tip bulb, mostly).
        let on_key = hint.icon == "use_binding" && !hint.key.is_empty();
        let key_icon = if hint.key.chars().count() > 1 { "icon_key_wide" } else { "icon_key_generic" };
        let art = if on_key { icons.icon(key_icon, &game, &mut images) } else { icons.icon(&hint.icon, &game, &mut images) };
        commands
            .spawn((
                HintRow(i),
                // Off screen until `place_hints` has its laid-out size
                // (hidden nodes are not laid out).
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-10000.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: vh(4.0),
                    ..default()
                },
                GlobalZIndex(4),
                bevy::picking::Pickable::IGNORE,
            ))
            .with_children(|row| {
                if let Some((image, rect)) = &art {
                    let w = HINT_ICON * rect.width() / rect.height().max(1.0);
                    let mut icon = row.spawn((
                        Node { width: vh(w), height: vh(HINT_ICON), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        ImageNode { image: image.clone(), rect: Some(*rect), ..default() },
                    ));
                    if on_key {
                        // dark on the light key cap
                        icon.with_child((Text::new(hint.key.clone()), font(HINT_KEY), TextColor(Color::srgb(0.08, 0.08, 0.08))));
                    }
                }
                if !hint.key.is_empty() && !(on_key && art.is_some()) {
                    row.spawn((
                        Node { padding: UiRect::axes(vh(4.0), vh(1.0)), ..default() },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                        children![(Text::new(hint.key.clone()), font(HINT_KEY), TextColor(Color::BLACK))],
                    ));
                }
                row.spawn((
                    Text::new(hint.caption.clone()),
                    font(HINT_CAPTION),
                    TextColor(Color::WHITE),
                    TextShadow { offset: Vec2::splat(1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8) },
                ));
            });
    }
}

/// Where a hint's row is centred, in logical pixels: over its target while
/// that is in front of the camera (kept on screen), else under the
/// crosshair, stacked by `slot`.
fn hint_anchor(target: Option<Vec2>, screen: Vec2, size: Vec2, slot: usize) -> Vec2 {
    match target {
        Some(p) => {
            let half = size * 0.5;
            p.clamp(half, (screen - half).max(half))
        }
        None => Vec2::new(screen.x * 0.5, screen.y * HINT_STATIC_Y + slot as f32 * size.y * 1.25),
    }
}

fn place_hints(
    hints: Res<Hints>,
    ents: Option<Res<Ents>>,
    mut rows: Query<(&HintRow, &mut Node, &ComputedNode)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    props: Query<(&crate::props::Prop, &GlobalTransform)>,
    globals: Query<&GlobalTransform>,
) {
    let Some(ents) = ents else { return };
    let Ok((cam, cam_tf)) = camera.single() else { return };
    // The view may render offscreen first: project to NDC, then the window.
    let Ok(window) = window.single() else { return };
    let screen = window.size();
    let mut slot = 0usize;
    for (row, mut node, computed) in &mut rows {
        let Some(hint) = hints.0.get(row.0) else { continue };
        let size = computed.size() * computed.inverse_scale_factor();
        if size.x <= 0.0 {
            continue;
        }
        // The target where it is drawn now (cubes move).
        let target = hint.target.and_then(|i| {
            let e = ents.list.get(i)?;
            let at = props
                .iter()
                .find(|(p, _)| p.ent == i)
                .map(|(_, g)| g.translation())
                .or_else(|| e.entity.and_then(|r| globals.get(r).ok()).map(|g| g.translation()))
                .unwrap_or(e.transform.translation);
            let at = at + Vec3::Y * hint.offset * 0.0254;
            // Behind the camera: fall back to under the crosshair.
            if (at - cam_tf.translation()).dot(*cam_tf.forward()) <= 0.0 {
                return None;
            }
            let ndc = cam.world_to_ndc(cam_tf, at)?;
            Some(Vec2::new((ndc.x + 1.0) * 0.5 * screen.x, (1.0 - ndc.y) * 0.5 * screen.y))
        });
        if target.is_none() {
            slot += 1;
        }
        let c = hint_anchor(target, screen, size, slot.saturating_sub(1));
        node.left = Val::Px(c.x - size.x * 0.5);
        node.top = Val::Px(c.y - size.y * 0.5);
    }
}

#[cfg(test)]
mod hint_tests {
    use super::{hint_anchor, hint_until};
    use bevy::prelude::Vec2;

    #[test]
    fn hints_sit_on_target_or_under_crosshair() {
        let (screen, size) = (Vec2::new(1280.0, 720.0), Vec2::new(200.0, 30.0));
        assert_eq!(hint_anchor(Some(Vec2::new(500.0, 300.0)), screen, size, 0), Vec2::new(500.0, 300.0));
        // kept on screen
        assert_eq!(hint_anchor(Some(Vec2::new(-50.0, 900.0)), screen, size, 0), Vec2::new(100.0, 705.0));
        // static: centred, two thirds down, stacked
        assert_eq!(hint_anchor(None, screen, size, 0), Vec2::new(640.0, 480.0));
        assert_eq!(hint_anchor(None, screen, size, 1).y, 480.0 + 37.5);
    }

    #[test]
    fn zero_or_no_timeout_waits_for_endhint() {
        assert_eq!(hint_until(10.0, Some(5.0)), 15.0);
        assert_eq!(hint_until(10.0, Some(0.0)), f32::INFINITY);
        assert_eq!(hint_until(10.0, None), f32::INFINITY);
    }
}

#[cfg(test)]
mod fov_tests {
    use super::in_fov;
    use bevy::prelude::Vec3;

    #[test]
    fn negative_fov_means_looking_away() {
        let fwd = Vec3::NEG_Z;
        let (ahead, behind) = (Vec3::new(0.1, -0.3, -5.0), Vec3::new(0.0, 0.2, 5.0));
        assert!(in_fov(fwd, ahead, 100.0, false));
        assert!(!in_fov(fwd, behind, 100.0, false));
        assert!(!in_fov(fwd, ahead, -100.0, false));
        assert!(in_fov(fwd, behind, -100.0, false));
        assert!(in_fov(fwd, behind, 360.0, false));
    }
}
