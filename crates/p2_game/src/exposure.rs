//! Auto-exposure, as Portal 2's HDR tonemapping does it: the image is
//! scaled so that where the player stands looks evenly bright, within the
//! limits each map sets through its `env_tonemap_controller`
//! (`SetAutoExposureMin` / `SetAutoExposureMax`, `SetTonemapRate`). Dark
//! places (the wrecked relaxation vault, the underground) open up; most
//! lit chambers sit at their map's maximum.
use bevy::prelude::*;
use p2_core::*;
use source_assets::valve_render::ValveExposure;

use crate::io::{EntInput, Ents};
use crate::level::{CurrentMap, GameSet, LevelLoaded};

pub struct ExposurePlugin;

impl Plugin for ExposurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoExposure>().add_systems(Update, (tonemap_inputs, adapt).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

/// The scale a lit room with this much ambient light gets: brightness the
/// fixed exposure (3) was tuned for, so lit chambers look as before.
const KEY: f32 = 0.06;

#[derive(Resource, Debug, Clone, Copy)]
pub struct AutoExposure {
    pub min: f32,
    pub max: f32,
    /// How fast it adapts (Source `SetTonemapRate`, roughly per second).
    pub rate: f32,
    pub current: f32,
}

impl Default for AutoExposure {
    fn default() -> Self {
        AutoExposure { min: 0.5, max: 3.0, rate: 1.0, current: source_assets::DEFAULT_LIGHT_SCALE }
    }
}

fn tonemap_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut auto: ResMut<AutoExposure>, mut loaded: MessageReader<LevelLoaded>) {
    if loaded.read().count() > 0 {
        *auto = AutoExposure::default();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class != "env_tonemap_controller" {
            continue;
        }
        let v = source_assets::kv::parse_f32(&msg.param);
        match (msg.input.as_str(), v) {
            ("setautoexposuremin", Some(v)) => auto.min = v.max(0.01),
            ("setautoexposuremax", Some(v)) => auto.max = v.max(0.01),
            ("settonemaprate", Some(v)) => auto.rate = v.max(0.01),
            _ => {}
        }
    }
}

fn adapt(
    time: Res<Time>,
    map: Option<Res<CurrentMap>>,
    cache: Res<source_assets::SourceCache>,
    mut auto: ResMut<AutoExposure>,
    mut exposure: ResMut<ValveExposure>,
    players: Query<(&Transform, &Crouch), With<Player>>,
    settings: Res<crate::settings::Settings>,
) {
    let Some(map) = map else { return };
    // Opt-in until it measures the rendered image: the ambient light at the
    // eye reads near zero in many lit places (elevators, chambers).
    if map.menu || std::env::var_os("P2_AUTOEXPOSURE").is_none() {
        if let Some(v) = std::env::var("P2_EXPOSURE").ok().and_then(|v| v.parse().ok()) {
            exposure.0 = v;
        }
        return;
    }
    let Ok((tf, crouch)) = players.single() else { return };
    let Some(loaded) = cache.maps.get(&map.info.name) else { return };
    let eye = (tf.translation + Vec3::Y * crouch.eye_height) / shared::SOURCE_UNIT;
    let src = Vec3::new(eye.x, -eye.z, eye.y);
    let lum = loaded.ambient_at(src).average().dot(Vec3::new(0.2126, 0.7152, 0.0722)).max(1e-4);
    let (lo, hi) = (auto.min.min(auto.max), auto.max.max(auto.min));
    let target = (KEY / lum).clamp(lo, hi);
    let k = 1.0 - (-time.delta_secs() * auto.rate * 4.0).exp();
    auto.current += (target - auto.current) * k;
    if std::env::var_os("P2_TRACE").is_some() && (time.elapsed_secs() * 2.0).fract() < time.delta_secs() * 2.0 {
        info!("TRACE exposure lum {lum:.4} target {target:.2} now {:.2} (min {} max {})", auto.current, auto.min, auto.max);
    }
    let _ = &settings;
    exposure.0 = std::env::var("P2_EXPOSURE").ok().and_then(|v| v.parse().ok()).unwrap_or(auto.current);
}
