//! Map fog at run time. `!player SetFogController <name>` (61 of the 63
//! campaign maps switch fog this way as the player moves between areas)
//! blends to that `env_fog_controller` over its `foglerptime`; `SetColor`,
//! `SetStartDist` and `SetEndDist` on the active controller change it at
//! once (sp_a1_intro1's vault turns the light ride fog dense). Maps start on
//! the master controller (source_assets `map_fog_controller`). Feeds the
//! cameras' `DistanceFog`, the clear colour and the Valve shaders' fog.
//! Not done: the `*LerpTo`/`StartFogTransition` inputs (unused by the
//! campaign), far Z, height fog.
use bevy::prelude::*;
use source_assets::plugin::MapFog;
use source_assets::valve_render::ValveFog;

use crate::io::{EntInput, Ents, GameEnt, PLAYER};
use crate::level::LevelLoaded;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FogState>().add_systems(
            Update,
            (fog_inputs, apply_fog).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic),
        );
    }
}

/// One controller's fog: colour 0-255 (gamma), distances in Source units.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FogParams {
    color: Vec3,
    start: f32,
    end: f32,
    max_density: f32,
}

impl FogParams {
    fn of(e: &GameEnt) -> Self {
        let enabled = e.get("fogenable").is_some_and(|v| v.trim() == "1");
        FogParams {
            color: e.get("fogcolor").and_then(parse_color).unwrap_or(Vec3::splat(128.0)),
            start: e.get_f32("fogstart").unwrap_or(0.0),
            end: e.get_f32("fogend").unwrap_or(10000.0),
            max_density: if enabled { e.get_f32("fogmaxdensity").unwrap_or(1.0).clamp(0.0, 1.0) } else { 0.0 },
        }
    }

    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        FogParams {
            color: a.color.lerp(b.color, t),
            start: a.start + (b.start - a.start) * t,
            end: a.end + (b.end - a.end) * t,
            max_density: a.max_density + (b.max_density - a.max_density) * t,
        }
    }

    fn linear(&self) -> Vec3 {
        (self.color / 255.0).powf(2.2)
    }

    fn map_fog(&self) -> MapFog {
        let c = self.linear();
        MapFog {
            color: Color::linear_rgb(c.x, c.y, c.z),
            start: self.start * shared::SOURCE_UNIT,
            end: self.end * shared::SOURCE_UNIT,
            max_density: self.max_density,
        }
    }

    fn valve(&self) -> ([f32; 4], [f32; 3]) {
        let c = self.linear();
        (source_assets::valve::fog_params(self.start, self.end, self.max_density), [c.x, c.y, c.z])
    }
}

fn parse_color(v: &str) -> Option<Vec3> {
    let n: Vec<f32> = v.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    (n.len() >= 3).then(|| Vec3::new(n[0], n[1], n[2]))
}

/// The player's fog: the active controller and a blend towards it.
#[derive(Resource, Default)]
struct FogState {
    active: Option<usize>,
    from: Option<FogParams>,
    to: Option<FogParams>,
    blend_start: f32,
    blend_time: f32,
    current: Option<FogParams>,
}

impl FogState {
    /// The fog at `now`, blending `from` -> `to`.
    fn at(&self, now: f32) -> Option<FogParams> {
        let to = self.to?;
        let Some(from) = self.from else { return Some(to) };
        if self.blend_time <= 0.0 {
            return Some(to);
        }
        let t = ((now - self.blend_start) / self.blend_time).clamp(0.0, 1.0);
        Some(FogParams::lerp(&from, &to, t))
    }

    /// Switch to `params`, blending from the fog shown now over `secs`.
    fn switch(&mut self, params: FogParams, now: f32, secs: f32) {
        self.from = self.at(now);
        self.to = Some(params);
        self.blend_start = now;
        self.blend_time = secs.max(0.0);
    }
}

/// The controller a map starts on: the enabled master (spawnflags 1), else
/// the first enabled one (as source_assets' loader picks it).
fn initial_controller(ents: &Ents) -> Option<usize> {
    let enabled = |e: &GameEnt| e.class == "env_fog_controller" && e.get("fogenable").is_some_and(|v| v.trim() == "1");
    let list = &ents.list;
    (0..list.len())
        .find(|&i| enabled(&list[i]) && list[i].spawnflags & 1 != 0)
        .or_else(|| (0..list.len()).find(|&i| enabled(&list[i])))
}

fn fog_inputs(
    mut inputs: MessageReader<EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    time: Res<Time>,
    mut state: ResMut<FogState>,
) {
    let now = time.elapsed_secs();
    let Some(ents) = ents else {
        inputs.clear();
        loaded.clear();
        return;
    };
    if loaded.read().count() > 0 {
        let active = initial_controller(&ents);
        let params = active.map(|i| FogParams::of(&ents.list[i]));
        *state = FogState { active, to: params, current: params, ..default() };
    }
    for msg in inputs.read() {
        let param = msg.param.trim();
        if msg.target == PLAYER {
            if msg.input != "setfogcontroller" {
                continue;
            }
            let Some(i) = ents.find(param, PLAYER, msg.activator).into_iter().find(|&i| ents.list[i].class == "env_fog_controller") else {
                debug!("portal2: SetFogController {param:?}: no such env_fog_controller");
                continue;
            };
            if state.active == Some(i) {
                continue;
            }
            let e = &ents.list[i];
            debug!("portal2: fog controller {:?}", e.name);
            state.active = Some(i);
            let secs = e.get_f32("foglerptime").unwrap_or(0.0);
            state.switch(FogParams::of(e), now, secs);
            continue;
        }
        if ents.list[msg.target].class != "env_fog_controller" || state.active != Some(msg.target) {
            continue;
        }
        let Some(mut to) = state.to else { continue };
        let number = param.parse::<f32>().ok();
        match msg.input.as_str() {
            "setcolor" => to.color = parse_color(param).unwrap_or(to.color),
            "setstartdist" => to.start = number.unwrap_or(to.start),
            "setenddist" => to.end = number.unwrap_or(to.end),
            _ => continue,
        }
        debug!("portal2: fog {} {param}", msg.input);
        state.switch(to, now, 0.0);
    }
}

fn apply_fog(
    time: Res<Time>,
    mut state: ResMut<FogState>,
    mut valve: ResMut<ValveFog>,
    mut clear: ResMut<ClearColor>,
    mut cams: Query<&mut DistanceFog>,
) {
    let fog = state.at(time.elapsed_secs());
    let changed = fog != state.current;
    if changed {
        state.current = fog;
    }
    let Some(fog) = fog else {
        if valve.0.is_some() {
            valve.0 = None;
        }
        return;
    };
    let map = fog.map_fog();
    if changed || valve.0.is_none() {
        valve.0 = Some(fog.valve());
        clear.0 = map.color;
    }
    let distance = map.distance_fog();
    // Every camera when the fog changes; new cameras (portal views, given
    // the load-time fog by level.rs) as they appear.
    for mut f in &mut cams {
        if changed || f.is_added() {
            *f = distance.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fog(c: f32, start: f32, end: f32) -> FogParams {
        FogParams { color: Vec3::splat(c), start, end, max_density: 1.0 }
    }

    #[test]
    fn switching_blends_over_lerp_time() {
        let mut s = FogState { to: Some(fog(100.0, 500.0, 8000.0)), ..default() };
        s.switch(fog(50.0, 128.0, 2500.0), 10.0, 2.0);
        assert_eq!(s.at(10.0), Some(fog(100.0, 500.0, 8000.0)));
        let mid = s.at(11.0).unwrap();
        assert!((mid.color.x - 75.0).abs() < 1e-3 && (mid.end - 5250.0).abs() < 1e-2);
        assert_eq!(s.at(13.0), Some(fog(50.0, 128.0, 2500.0)));
        // An immediate change mid-blend jumps there.
        s.switch(fog(10.0, 0.0, 100.0), 11.0, 0.0);
        assert_eq!(s.at(11.0), Some(fog(10.0, 0.0, 100.0)));
    }

    #[test]
    fn disabled_controller_has_no_density() {
        let kv = |k: &str, v: &str| (k.to_string(), v.to_string());
        let e = GameEnt::logic("env_fog_controller", "fog", vec![kv("fogcolor", "50 70 80"), kv("fogenable", "0"), kv("fogend", "2500")]);
        let p = FogParams::of(&e);
        assert_eq!(p.color, Vec3::new(50.0, 70.0, 80.0));
        assert_eq!((p.end, p.max_density), (2500.0, 0.0));
    }
}
