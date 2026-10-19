//! `env_projectedtexture`: Portal 2's shadowed spotlights, drawn by
//! `source_assets::projected`. Each one becomes an entity with a
//! [`ProjectedLight`] when the level loads, on if spawnflag 1 ("Enabled")
//! is set. Every frame it takes its entity's current transform (parented
//! lights ride trains and Wheatley through `movers`/`viewcontrol`, which keep
//! `Ents` transforms current), turns to face its `target` if it has one, and
//! fades its colour toward the last `LightColor`.
//!
//! Inputs: TurnOn, TurnOff, LightColor, FOV, EnableShadows,
//! SpotlightTexture, SetNearZ, SetFarZ, LightWorld, Target. `lightonlytarget`
//! is not honoured (such a light lights everything in its frustum).
//!
//! `colortransitiontime` is treated as Source does it as far as we know:
//! a rate, each 0..255 colour channel (and the intensity) moves
//! `rate * 255` per second toward its target (0 = instant).
use std::collections::HashMap;

use bevy::prelude::*;
use source_assets::projected::{light_color, ProjectedLight};

use crate::io::{EntInput, Ents, GameEnt};
use crate::level::{GameSet, LevelLoaded, LevelOwned};

pub struct ProjectedLightsPlugin;

impl Plugin for ProjectedLightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectedLights>()
            .add_systems(Update, (spawn_lights, light_inputs).chain().after(crate::io::run_io).in_set(GameSet::Logic))
            // after every Update system moved the entities they ride on
            .add_systems(PostUpdate, update_lights.before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Light entity per `env_projectedtexture` entity index.
#[derive(Resource, Default)]
struct ProjectedLights(HashMap<usize, Entity>);

#[derive(Component, Debug, Clone)]
struct LightState {
    index: usize,
    /// `lightcolor` now and where it is heading (0..255 rgb, intensity).
    current: Vec4,
    target: Vec4,
    brightness: f32,
    /// Colour change rate (`colortransitiontime`).
    rate: f32,
    /// Entity name to face (`target`).
    aim: Option<String>,
}

/// "R G B intensity" (0..255); missing parts are 255.
pub fn parse_color4(s: &str) -> Vec4 {
    let v: Vec<f32> = s.split_whitespace().filter_map(source_assets::kv::parse_f32).collect();
    let g = |i: usize| v.get(i).copied().unwrap_or(255.0);
    Vec4::new(g(0), g(1), g(2), g(3))
}

/// Move each component of `cur` at most `step` toward `target`.
pub fn approach(cur: Vec4, target: Vec4, step: f32) -> Vec4 {
    let d = target - cur;
    cur + d.clamp(Vec4::splat(-step), Vec4::splat(step))
}

/// A rotation whose local +X (a light's forward) points along `dir`, local
/// +Y as close to world up as it can be.
pub fn aim_rotation(dir: Vec3) -> Option<Quat> {
    let x = dir.try_normalize()?;
    let z = x.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::Z);
    let y = z.cross(x);
    Some(Quat::from_mat3(&Mat3::from_cols(x, y, z)))
}

fn flag(v: &str) -> bool {
    source_assets::kv::parse_f32(v).is_some_and(|f| f != 0.0)
}

/// The light an `env_projectedtexture` starts as, with its colour state.
fn initial(e: &GameEnt, index: usize) -> (ProjectedLight, LightState) {
    let u = shared::SOURCE_UNIT;
    let color = parse_color4(e.get("lightcolor").unwrap_or("255 255 255 200"));
    let brightness = e.get_f32("brightnessscale").unwrap_or(1.0);
    let light = ProjectedLight {
        enabled: e.spawnflags & 1 != 0 || e.get("startactive").is_some_and(flag),
        color: light_color(&format!("{} {} {} {}", color.x, color.y, color.z, color.w), brightness),
        fov: e.get_f32("lightfov").unwrap_or(90.0),
        near: e.get_f32("nearz").unwrap_or(4.0) * u,
        far: e.get_f32("farz").unwrap_or(750.0) * u,
        cookie: e.get("texturename").filter(|t| !t.trim().is_empty()).unwrap_or("effects/flashlight001").trim().to_string(),
        shadows: e.get("enableshadows").is_some_and(flag),
        light_world: e.get("lightworld").is_none_or(flag),
    };
    let state = LightState {
        index,
        current: color,
        target: color,
        brightness,
        rate: e.get_f32("colortransitiontime").unwrap_or(0.5),
        aim: e.get("target").map(|t| t.trim().to_string()).filter(|t| !t.is_empty()),
    };
    (light, state)
}

/// Apply one input; `true` if it was a light input.
fn apply_input(light: &mut ProjectedLight, state: &mut LightState, input: &str, param: &str) -> bool {
    let u = shared::SOURCE_UNIT;
    let num = || source_assets::kv::parse_f32(param);
    match input {
        "turnon" => light.enabled = true,
        "turnoff" => light.enabled = false,
        "lightcolor" => {
            state.target = parse_color4(param);
            if state.rate <= 0.0 {
                state.current = state.target;
            }
        }
        "fov" => light.fov = num().unwrap_or(light.fov).clamp(1.0, 175.0),
        "enableshadows" => light.shadows = flag(param),
        "spotlighttexture" => light.cookie = param.trim().to_string(),
        "setnearz" => light.near = num().map_or(light.near, |v| v * u),
        "setfarz" => light.far = num().map_or(light.far, |v| v * u),
        "lightworld" => light.light_world = flag(param),
        "target" => state.aim = Some(param.trim().to_string()).filter(|t| !t.is_empty()),
        _ => return false,
    }
    true
}

fn spawn_lights(mut commands: Commands, mut loaded: MessageReader<LevelLoaded>, ents: Option<Res<Ents>>, mut lights: ResMut<ProjectedLights>) {
    if loaded.read().count() == 0 {
        return;
    }
    lights.0.clear();
    let Some(ents) = ents else { return };
    for (i, e) in ents.list.iter().enumerate().filter(|(_, e)| e.class == "env_projectedtexture") {
        let (light, state) = initial(e, i);
        let id = commands
            .spawn((Name::new(format!("env_projectedtexture {}", e.name)), light, state, e.transform, Visibility::default(), LevelOwned))
            .id();
        lights.0.insert(i, id);
    }
}

fn light_inputs(mut inputs: MessageReader<EntInput>, lights: Res<ProjectedLights>, mut q: Query<(&mut ProjectedLight, &mut LightState)>) {
    for msg in inputs.read() {
        let Some(&e) = lights.0.get(&msg.target) else { continue };
        let Ok((mut light, mut state)) = q.get_mut(e) else { continue };
        if !apply_input(&mut light, &mut state, &msg.input, &msg.param) {
            trace!("env_projectedtexture: unhandled input {}", msg.input);
        }
    }
}

fn update_lights(time: Res<Time>, ents: Option<Res<Ents>>, mut q: Query<(&mut ProjectedLight, &mut LightState, &mut Transform)>) {
    let Some(ents) = ents else { return };
    let dt = time.delta_secs();
    for (mut light, mut state, mut tf) in &mut q {
        let Some(e) = ents.list.get(state.index) else { continue };
        if e.killed && light.enabled {
            light.enabled = false;
        }
        let mut world = e.transform;
        if let Some(aim) = &state.aim {
            if let Some(&t) = ents.find(aim, state.index, None).first() {
                if let Some(rot) = aim_rotation(ents.list[t].transform.translation - world.translation) {
                    world.rotation = rot;
                }
            }
        }
        if *tf != world {
            *tf = world;
        }
        if state.current != state.target {
            state.current = if state.rate > 0.0 { approach(state.current, state.target, state.rate * 255.0 * dt) } else { state.target };
            let c = state.current;
            light.color = light_color(&format!("{} {} {} {}", c.x, c.y, c.z, c.w), state.brightness);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(kv: &[(&str, &str)], spawnflags: u32) -> GameEnt {
        let mut e = GameEnt::logic("env_projectedtexture", "light_door_00", kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        e.spawnflags = spawnflags;
        e
    }

    #[test]
    fn spawn_state_from_keyvalues() {
        let e = ent(
            &[
                ("texturename", "effects/flashlight001_intro"),
                ("lightfov", "70"),
                ("nearz", "8"),
                ("farz", "512"),
                ("lightcolor", "252 247 239 100"),
                ("brightnessscale", "2"),
                ("enableshadows", "1"),
                ("lightworld", "1"),
                ("target", "door_target"),
            ],
            2,
        );
        let (l, s) = initial(&e, 5);
        assert!(!l.enabled, "spawnflag 1 (Enabled) not set");
        assert_eq!(l.cookie, "effects/flashlight001_intro");
        assert_eq!(l.fov, 70.0);
        assert!((l.far - 512.0 * shared::SOURCE_UNIT).abs() < 1e-6);
        assert!(l.shadows && l.light_world);
        assert_eq!(l.color, light_color("252 247 239 100", 2.0));
        assert_eq!(s.aim.as_deref(), Some("door_target"));
        assert!(initial(&ent(&[], 3), 0).0.enabled);
        // defaults
        let (l, _) = initial(&ent(&[], 1), 0);
        assert_eq!((l.cookie.as_str(), l.fov, l.shadows), ("effects/flashlight001", 90.0, false));
    }

    #[test]
    fn inputs_switch_and_change_the_light() {
        let (mut l, mut s) = initial(&ent(&[("lightcolor", "255 255 255 255")], 0), 0);
        assert!(apply_input(&mut l, &mut s, "turnon", ""));
        assert!(l.enabled);
        assert!(apply_input(&mut l, &mut s, "turnoff", ""));
        assert!(!l.enabled);
        apply_input(&mut l, &mut s, "fov", "45");
        apply_input(&mut l, &mut s, "spotlighttexture", "effects/flashlight002");
        apply_input(&mut l, &mut s, "enableshadows", "1");
        apply_input(&mut l, &mut s, "setfarz", "100");
        assert_eq!((l.fov, l.cookie.as_str(), l.shadows), (45.0, "effects/flashlight002", true));
        assert!((l.far - 100.0 * shared::SOURCE_UNIT).abs() < 1e-6);
        apply_input(&mut l, &mut s, "lightcolor", "0 0 0 0");
        assert_eq!(s.target, Vec4::ZERO);
        assert_eq!(s.current, Vec4::splat(255.0), "fades at the transition rate");
        assert!(!apply_input(&mut l, &mut s, "alwaysupdateon", ""));
    }

    #[test]
    fn colour_transitions_at_its_rate() {
        let cur = Vec4::new(0.0, 100.0, 255.0, 50.0);
        let t = Vec4::new(255.0, 100.0, 0.0, 60.0);
        let next = approach(cur, t, 51.0);
        assert_eq!(next, Vec4::new(51.0, 100.0, 204.0, 60.0));
        // rate 0.5 reaches a full-range change in 2 s
        let mut c = Vec4::ZERO;
        for _ in 0..120 {
            c = approach(c, Vec4::splat(255.0), 0.5 * 255.0 / 60.0);
        }
        assert!((c - Vec4::splat(255.0)).abs().max_element() < 1e-3);
        assert_eq!(parse_color4("10 20 30"), Vec4::new(10.0, 20.0, 30.0, 255.0));
    }

    #[test]
    fn aims_its_forward_axis() {
        for dir in [Vec3::X, Vec3::new(0.0, -1.0, 1.0), Vec3::new(-3.0, 0.5, 2.0), Vec3::NEG_Y] {
            let q = aim_rotation(dir).unwrap();
            assert!((q * Vec3::X - dir.normalize()).length() < 1e-4, "{dir}");
            assert!((q * Vec3::Y).y >= -1e-4, "stays upright");
        }
        assert!(aim_rotation(Vec3::ZERO).is_none());
    }
}
