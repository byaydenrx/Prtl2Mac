//! Auto-exposure, as Portal 2's HDR tonemapping does it: the picture is
//! scaled so the scene in view looks evenly bright, within the limits each
//! map sets through its `env_tonemap_controller` (`SetAutoExposureMin` /
//! `SetAutoExposureMax`, `SetTonemapRate`). The global instance every map
//! uses gives lit chambers 0.5..3 and dark places 1..5, so dark rooms (the
//! wrecked relaxation vault, the underground) open up while lit chambers sit
//! at their maximum, which is the fixed exposure (3) they were tuned at.
//!
//! The brightness is measured from the rendered scene: a tiny probe camera
//! follows the main one into a 64x36 HDR image that is read back from the
//! GPU every few frames.
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use p2_core::*;
use source_assets::valve_render::ValveExposure;

use crate::io::{EntInput, Ents};
use crate::level::{CurrentMap, GameSet, LevelLoaded};

pub struct ExposurePlugin;

impl Plugin for ExposurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoExposure>()
            .init_resource::<ExposureProbe>()
            .add_systems(Update, (tonemap_inputs, adapt).chain().after(crate::io::run_io).in_set(GameSet::Logic))
            .add_systems(PostUpdate, follow_main_camera.after(TransformSystems::Propagate));
    }
}

/// Probe image size: 64 RGBA16F texels make a 512-byte row, which a GPU
/// copy needs no padding for.
const PROBE_W: u32 = 64;
const PROBE_H: u32 = 36;
/// The probe renders every this many frames.
const PROBE_EVERY: u32 = 3;

/// Brightness the view's typical (60th percentile) texel is scaled to,
/// tuned so lit chambers reach their maximum exposure and the dark parts of
/// the game open up the way they do in Portal 2.
const TARGET: f32 = 0.45;

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

/// The probe camera's image and what the last read of it measured.
#[derive(Resource, Default)]
pub struct ExposureProbe {
    camera: Option<Entity>,
    /// 60th percentile luminance of the view at exposure 1.
    pub luminance: Option<f32>,
    /// Exposure the probe rendered at (shaders apply it), to undo it.
    rendered_at: f32,
    frame: u32,
}

#[derive(Component)]
struct ProbeCamera;

/// Opt-in (`P2_AUTOEXPOSURE`) until the target brightness is calibrated.
fn enabled() -> bool {
    std::env::var_os("P2_AUTOEXPOSURE").is_some() && std::env::var_os("P2_EXPOSURE").is_none()
}

fn tonemap_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut auto: ResMut<AutoExposure>, mut loaded: MessageReader<LevelLoaded>, mut probe: ResMut<ExposureProbe>) {
    if loaded.read().count() > 0 {
        *auto = AutoExposure::default();
        probe.luminance = None;
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

/// Spawns the probe the first time there is a main camera, then keeps it
/// on the main camera's pose and field of view, rendering every few frames.
#[allow(clippy::type_complexity)]
fn follow_main_camera(
    mut commands: Commands,
    mut probe: ResMut<ExposureProbe>,
    mut images: ResMut<Assets<Image>>,
    exposure: Res<ValveExposure>,
    main: Query<(&GlobalTransform, &Projection), (With<MainCamera>, Without<ProbeCamera>)>,
    mut cams: Query<(&mut Transform, &mut Projection, &mut Camera), With<ProbeCamera>>,
) {
    if !enabled() {
        return;
    }
    let Ok((g, proj)) = main.single() else { return };
    let Some(cam) = probe.camera else {
        let mut image = Image::new_target_texture(PROBE_W, PROBE_H, TextureFormat::Rgba16Float, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let image = images.add(image);
        let cam = commands
            .spawn((
                Name::new("Exposure probe"),
                ProbeCamera,
                Camera3d::default(),
                Camera { order: -10_000, ..default() },
                RenderTarget::Image(image.clone().into()),
                Hdr,
                Tonemapping::None,
                Msaa::Off,
                RenderLayers::layer(0),
                proj.clone(),
                g.compute_transform(),
            ))
            .id();
        commands.spawn((Name::new("Exposure probe readback"), Readback::texture(image))).observe(read_probe);
        probe.camera = Some(cam);
        return;
    };
    let Ok((mut tf, mut p, mut camera)) = cams.get_mut(cam) else { return };
    *tf = g.compute_transform();
    if let (Projection::Perspective(a), Projection::Perspective(b)) = (&*p, proj) {
        if a.fov != b.fov || a.aspect_ratio != b.aspect_ratio {
            *p = proj.clone();
        }
    }
    probe.frame = probe.frame.wrapping_add(1);
    let on = probe.frame % PROBE_EVERY == 0;
    if camera.is_active != on {
        camera.is_active = on;
    }
    if on {
        probe.rendered_at = exposure.0;
    }
}

fn half_to_f32(h: u16) -> f32 {
    let s = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = ((h >> 10) & 0x1f) as i32;
    let m = (h & 0x3ff) as f32;
    match e {
        0 => s * m * 2f32.powi(-24),
        31 => 0.0,
        _ => s * (1.0 + m / 1024.0) * 2f32.powi(e - 15),
    }
}

/// The view's typical brightness from the probe image, at exposure 1.
fn read_probe(ev: On<ReadbackComplete>, mut probe: ResMut<ExposureProbe>) {
    let data = &ev.data;
    if data.len() < (PROBE_W * PROBE_H * 8) as usize || probe.rendered_at <= 0.0 {
        return;
    }
    let mut lum: Vec<f32> = data
        .chunks_exact(8)
        .take((PROBE_W * PROBE_H) as usize)
        .map(|p| {
            let c = |k: usize| half_to_f32(u16::from_le_bytes([p[k], p[k + 1]])).max(0.0);
            0.2126 * c(0) + 0.7152 * c(2) + 0.0722 * c(4)
        })
        .collect();
    if let Some(path) = std::env::var_os("P2_PROBE_DUMP") {
        let mut out = format!("P6\n{PROBE_W} {PROBE_H}\n255\n").into_bytes();
        for p in data.chunks_exact(8).take((PROBE_W * PROBE_H) as usize) {
            for k in [0, 2, 4] {
                out.push((half_to_f32(u16::from_le_bytes([p[k], p[k + 1]])).clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
        let _ = std::fs::write(path, out);
    }
    let k = lum.len() * 6 / 10;
    let (_, l, _) = lum.select_nth_unstable_by(k, f32::total_cmp);
    probe.luminance = Some((*l / probe.rendered_at).max(1e-5));
}

fn adapt(time: Res<Time>, map: Option<Res<CurrentMap>>, probe: Res<ExposureProbe>, mut auto: ResMut<AutoExposure>, mut exposure: ResMut<ValveExposure>) {
    if let Some(v) = std::env::var("P2_EXPOSURE").ok().and_then(|v| v.parse().ok()) {
        exposure.0 = v;
        return;
    }
    let Some(map) = map else { return };
    if map.menu || !enabled() {
        return;
    }
    let Some(lum) = probe.luminance else { return };
    let (lo, hi) = (auto.min.min(auto.max), auto.max.max(auto.min));
    let target = (TARGET / lum).clamp(lo, hi);
    // Source adapts in log space at its tonemap rate.
    let k = 1.0 - (-time.delta_secs() * auto.rate * 4.0).exp();
    auto.current = (auto.current.ln() + (target.ln() - auto.current.ln()) * k).exp();
    if std::env::var_os("P2_TRACE").is_some() && (time.elapsed_secs() * 2.0).fract() < time.delta_secs() * 2.0 {
        info!("TRACE exposure lum {lum:.4} target {target:.2} now {:.2} (min {} max {})", auto.current, auto.min, auto.max);
    }
    exposure.0 = auto.current;
}
