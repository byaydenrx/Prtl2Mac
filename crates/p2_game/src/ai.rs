//! Scripted checks on the player and on-screen instructor hints.
//!
//! * `ai_script_conditions`: once enabled, waits until the player has looked
//!   at its target (within `PlayerTargetFOV`, optionally in line of sight)
//!   for `RequiredTime` seconds, then fires `OnConditionsSatisfied`. The
//!   intro's "look at the ceiling / the floor / the art" steps run on these.
//! * `env_instructor_hint`: `ShowHint` puts its caption and key on screen
//!   until `EndHint` or its timeout.
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
            .add_systems(Update, (script_conditions, hint_inputs, draw_hints).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
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

/// Hints on screen: (ent, caption, key, until, on-screen icon).
#[derive(Resource, Default)]
struct Hints(Vec<(usize, String, String, f32, String)>);

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
                let timeout = e.get_f32("hint_timeout").filter(|t| *t > 0.0).unwrap_or(8.0);
                hints.0.retain(|h| h.0 != msg.target);
                let icon = e.get("hint_icon_onscreen").unwrap_or("icon_tip").to_string();
                hints.0.push((msg.target, caption, key, ents.now + timeout, icon));
            }
            "endhint" | "kill" => hints.0.retain(|h| h.0 != msg.target),
            _ => {}
        }
    }
    let now = ents.now;
    hints.0.retain(|h| h.3 > now);
}

#[derive(Component)]
struct HintRoot;

#[allow(clippy::too_many_arguments)]
fn draw_hints(
    mut commands: Commands,
    hints: Res<Hints>,
    fonts: Option<Res<UiFonts>>,
    roots: Query<Entity, With<HintRoot>>,
    game: Res<p2_assets::GameFsRes>,
    mut icons: ResMut<HintIcons>,
    mut images: ResMut<Assets<Image>>,
) {
    if !hints.is_changed() {
        return;
    }
    let Some(fonts) = fonts else { return };
    icons.load(&game);
    for r in &roots {
        commands.entity(r).despawn();
    }
    if hints.0.is_empty() {
        return;
    }
    let font = |size: f32| TextFont { font: fonts.bold.clone().into(), font_size: bevy::text::FontSize::Px(size), ..default() };
    commands
        .spawn((
            HintRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Percent(22.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            GlobalZIndex(4),
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|p| {
            for (_, caption, key, _, icon) in &hints.0 {
                // A key hint shows the key on the instructor's key icon;
                // others their own icon (the tip bulb, mostly).
                let key_icon = if key.chars().count() > 1 { "icon_key_wide" } else { "icon_key_generic" };
                let art = if icon == "use_binding" && !key.is_empty() { icons.icon(key_icon, &game, &mut images) } else { icons.icon(icon, &game, &mut images) };
                p.spawn((
                    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ))
                .with_children(|row| {
                    match (&art, icon == "use_binding" && !key.is_empty()) {
                        (Some((image, rect)), true) => {
                            let w = 40.0 * rect.width() / rect.height().max(1.0);
                            row.spawn((
                                Node { width: Val::Px(w), height: Val::Px(40.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                ImageNode { image: image.clone(), rect: Some(*rect), ..default() },
                                // dark on the light key cap (white text vanished into it)
                                children![(Text::new(key.clone()), font(15.0), TextColor(Color::srgb(0.08, 0.08, 0.08)))],
                            ));
                        }
                        (Some((image, rect)), false) => {
                            row.spawn((Node { width: Val::Px(40.0), height: Val::Px(40.0), ..default() }, ImageNode { image: image.clone(), rect: Some(*rect), ..default() }));
                        }
                        _ => {}
                    }
                    if !key.is_empty() && !(icon == "use_binding" && art.is_some()) {
                        row.spawn((
                            Node { padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)), ..default() },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                            children![(Text::new(key.clone()), font(18.0), TextColor(Color::BLACK))],
                        ));
                    }
                    row.spawn((Text::new(caption.clone()), font(20.0), TextColor(Color::WHITE)));
                });
            }
        });
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
