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
            if fov < 360.0 {
                let cos = fwd.dot(to.normalize_or_zero());
                let half = (fov * 0.5).to_radians();
                ok &= cos >= half.cos();
            }
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

/// Hints on screen: (ent, caption, key, until).
#[derive(Resource, Default)]
struct Hints(Vec<(usize, String, String, f32)>);

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
                hints.0.push((msg.target, caption, key, ents.now + timeout));
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

fn draw_hints(mut commands: Commands, hints: Res<Hints>, fonts: Option<Res<UiFonts>>, roots: Query<Entity, With<HintRoot>>) {
    if !hints.is_changed() {
        return;
    }
    let Some(fonts) = fonts else { return };
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
            for (_, caption, key, _) in &hints.0 {
                p.spawn((
                    Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ))
                .with_children(|row| {
                    if !key.is_empty() {
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
