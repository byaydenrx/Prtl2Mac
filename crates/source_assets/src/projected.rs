//! Projected texture lights (`env_projectedtexture`): Portal 2's shadowed
//! spotlights. Each light projects a cookie texture
//! (`effects/flashlight001`...) through a square frustum and adds
//! `albedo * colour * cookie * N.L * attenuation * shadow` to the surfaces
//! inside it, on top of their baked light.
//!
//! A game spawns entities with [`ProjectedLight`]; the light sits at the
//! entity's `GlobalTransform` and shines along its local +X with local +Y up
//! (Source's forward and up, as `entities::source_angles_to_world` builds
//! them). Every frame `gather_lights` picks up to [`MAX_LIGHTS`] enabled
//! lights nearest the main camera, gives the nearest [`MAX_SHADOWS`]
//! shadowed ones a shadow map, and writes them to one storage buffer
//! ([`ProjectedLightAssets::lights`]) that both draw paths read:
//! - the Valve-shader path draws each lit surface again with an additive
//!   light pass (`projected_render.rs`, `shaders/projected_light.wgsl`)
//!   after its main pass, and renders the shadow maps from the same
//!   meshes (`shaders/projected_shadow.wgsl`);
//! - the fallback `SourceMaterial` adds the same light in its fragment
//!   shader (`shaders/source_material.wgsl`).
//! The light maths is shared (`shaders/projected_common.wgsl`).
//!
//! `P2_NOPROJTEX=1` turns every projected light off, for comparison.
//!
//! Attenuation follows the public description of Source's flashlight
//! state for this entity: constant 0, linear 100, quadratic 0 (so full
//! strength within 100 units, then 100/d), faded to zero between 60% of
//! `farz` and `farz`. Colour is `lightcolor` rgb/255 * intensity/255 *
//! `brightnessscale`. These are estimates from public documentation, not
//! measured against the game.

use crate::fs::GameFs;
use crate::plugin::SourceGame;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::Shader;
use std::sync::Arc;

/// Lights drawn at once (Portal 2 maps have 1-9; the nearest win).
pub const MAX_LIGHTS: usize = 8;
/// Lights with a shadow map at once.
pub const MAX_SHADOWS: usize = 4;
/// Shadow map size (texels per side).
pub const SHADOW_SIZE: u32 = 2048;
/// Cookie textures are resampled to this size in one texture array.
pub const COOKIE_SIZE: u32 = 256;
pub const MAX_COOKIES: usize = 8;

/// Source flashlight attenuation for `env_projectedtexture` (see the module docs).
pub const LINEAR_ATTEN: f32 = 100.0;

/// An `env_projectedtexture`. Distances are world units (metres).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProjectedLight {
    pub enabled: bool,
    /// Linear colour times intensity ([`light_color`]).
    pub color: Vec3,
    /// Full field of view, degrees (the frustum is square).
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    /// Cookie texture name (`effects/flashlight001`).
    pub cookie: String,
    pub shadows: bool,
    /// Lights brushes too (`lightworld`); models always.
    pub light_world: bool,
}

impl Default for ProjectedLight {
    fn default() -> Self {
        ProjectedLight {
            enabled: true,
            color: Vec3::ONE,
            fov: 90.0,
            near: 4.0 * shared::SOURCE_UNIT,
            far: 750.0 * shared::SOURCE_UNIT,
            cookie: "effects/flashlight001".into(),
            shadows: true,
            light_world: true,
        }
    }
}

/// `lightcolor` ("R G B intensity", 0..255 each) times `brightnessscale`.
pub fn light_color(lightcolor: &str, brightness: f32) -> Vec3 {
    let v: Vec<f32> = lightcolor.split_whitespace().filter_map(crate::kv::parse_f32).collect();
    let rgb = if v.len() >= 3 { Vec3::new(v[0], v[1], v[2]) } else { Vec3::splat(255.0) };
    let intensity = v.get(3).copied().unwrap_or(255.0);
    rgb / 255.0 * (intensity / 255.0) * brightness
}

/// Source flashlight attenuation at `dist` units with `far` the light's far
/// plane (units), mirrored by `projected_attenuation` in the shaders.
pub fn attenuation(dist: f32, far: f32) -> f32 {
    let linear = (LINEAR_ATTEN / dist.max(1e-3)).clamp(0.0, 1.0);
    let end = ((dist - far) / (0.6 * far - far)).clamp(0.0, 1.0);
    linear * end
}

/// World -> light clip space (wgpu depth 0..1; `w` is the distance along
/// the light's forward axis).
pub fn light_view_proj(t: &GlobalTransform, fov: f32, near: f32, far: f32) -> Mat4 {
    let (_, rot, pos) = t.to_scale_rotation_translation();
    let view = Mat4::look_to_rh(pos, rot * Vec3::X, rot * Vec3::Y);
    let near = near.max(1e-3);
    Mat4::perspective_rh(fov.to_radians().clamp(0.01, 3.1), 1.0, near, far.max(near * 1.01)) * view
}

/// Does a sphere touch the frustum of `view_proj` (depth 0..1)?
pub fn sphere_in_frustum(view_proj: &Mat4, center: Vec3, radius: f32) -> bool {
    let m = view_proj.transpose();
    let (r0, r1, r2, r3) = (m.x_axis, m.y_axis, m.z_axis, m.w_axis);
    let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2];
    planes.iter().all(|p| {
        let len = p.truncate().length().max(1e-6);
        (p.truncate().dot(center) + p.w) / len >= -radius
    })
}

/// Which lights to draw: indices of up to [`MAX_LIGHTS`] enabled lights
/// nearest `camera`, each with its shadow map layer (the nearest
/// [`MAX_SHADOWS`] shadowed ones get one).
pub fn select_lights(lights: &[(Vec3, bool)], camera: Vec3) -> Vec<(usize, Option<u32>)> {
    let mut order: Vec<usize> = (0..lights.len()).collect();
    order.sort_by(|&a, &b| lights[a].0.distance_squared(camera).total_cmp(&lights[b].0.distance_squared(camera)));
    order.truncate(MAX_LIGHTS);
    let mut next = 0u32;
    order
        .into_iter()
        .map(|i| {
            let layer = (lights[i].1 && (next as usize) < MAX_SHADOWS).then(|| {
                next += 1;
                next - 1
            });
            (i, layer)
        })
        .collect()
}

/// One light as the shaders read it (`ProjLight` in `projected_common.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuProjectedLight {
    /// World -> light clip.
    pub view_proj: [[f32; 4]; 4],
    /// xyz world position, w far plane (metres).
    pub position: [f32; 4],
    /// rgb linear colour * intensity, w near plane (metres).
    pub color: [f32; 4],
    /// x shadow layer (-1: none), y cookie layer, z 1 = lights brushes,
    /// w metres per Source unit.
    pub params: [f32; 4],
}

/// The whole light buffer (`ProjLights`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuProjectedLights {
    /// x = light count.
    pub info: [u32; 4],
    pub lights: [GpuProjectedLight; MAX_LIGHTS],
}

impl Default for GpuProjectedLights {
    fn default() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}

/// The textures and buffer the lights are drawn with, shared by every
/// material and the Valve light pass.
#[derive(Resource, Clone, ExtractResource)]
pub struct ProjectedLightAssets {
    /// [`GpuProjectedLights`], rewritten when the lights change (it stays in
    /// the main world for that).
    pub lights: Handle<ShaderBuffer>,
    /// Cookie texture array ([`MAX_COOKIES`] layers of [`COOKIE_SIZE`]²),
    /// filled from the render world.
    pub cookies: Handle<Image>,
    /// Shadow maps (R32Float distance / far, [`MAX_SHADOWS`] layers),
    /// rendered in the render world.
    pub shadows: Handle<Image>,
    /// Kept so the `source_assets::projected` import stays loaded.
    #[allow(dead_code)]
    pub(crate) common_shader: Handle<Shader>,
    pub(crate) light_shader: Handle<Shader>,
    pub(crate) shadow_shader: Handle<Shader>,
}

/// One drawn light, for the render world (culling and shadow maps).
#[derive(Clone, Debug)]
pub struct FrameLight {
    pub view_proj: Mat4,
    pub position: Vec3,
    pub far: f32,
    pub shadow_layer: Option<u32>,
    pub light_world: bool,
}

/// This frame's lights, extracted to the render world.
#[derive(Resource, Clone, Default, ExtractResource)]
pub struct ProjectedFrame {
    pub lights: Vec<FrameLight>,
    pub gpu: GpuProjectedLights,
    /// Cookie pixels by layer (RGBA8 sRGB, [`COOKIE_SIZE`]²).
    pub cookies: Vec<Arc<Vec<u8>>>,
}

/// Cookie names by atlas layer.
#[derive(Resource, Default)]
struct CookieAtlas {
    names: Vec<String>,
}

/// `P2_NOPROJTEX` was set.
#[derive(Resource, Clone, Copy, Default)]
struct ProjectedDisabled(bool);

fn array_image(format: TextureFormat, size: u32, layers: u32, usage: TextureUsages) -> Image {
    let mut img = Image::new_uninit(
        Extent3d { width: size, height: size, depth_or_array_layers: layers },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_descriptor.usage |= usage;
    img.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::D2Array), ..default() });
    img
}

pub struct ProjectedLightPlugin;

impl Plugin for ProjectedLightPlugin {
    fn build(&self, app: &mut App) {
        let disabled = std::env::var_os("P2_NOPROJTEX").is_some();
        let world = app.world_mut();
        let (Some(_), Some(_), Some(_)) =
            (world.get_resource::<Assets<Image>>(), world.get_resource::<Assets<ShaderBuffer>>(), world.get_resource::<Assets<Shader>>())
        else {
            return;
        };
        let lights = world
            .resource_mut::<Assets<ShaderBuffer>>()
            .add(ShaderBuffer::new(bytemuck::bytes_of(&GpuProjectedLights::default()), RenderAssetUsages::default()));
        let mut cookie = array_image(TextureFormat::Rgba8UnormSrgb, COOKIE_SIZE, MAX_COOKIES as u32, TextureUsages::COPY_DST);
        cookie.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
        let shadow = array_image(
            TextureFormat::R32Float,
            SHADOW_SIZE,
            MAX_SHADOWS as u32,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
        );
        let mut images = world.resource_mut::<Assets<Image>>();
        let (cookies, shadows) = (images.add(cookie), images.add(shadow));
        let mut shaders = world.resource_mut::<Assets<Shader>>();
        let common_shader = shaders.add(Shader::from_wgsl(include_str!("shaders/projected_common.wgsl"), "source_assets/projected_common.wgsl"));
        let light_shader = shaders.add(Shader::from_wgsl(include_str!("shaders/projected_light.wgsl"), "source_assets/projected_light.wgsl"));
        let shadow_shader = shaders.add(Shader::from_wgsl(include_str!("shaders/projected_shadow.wgsl"), "source_assets/projected_shadow.wgsl"));
        app.insert_resource(ProjectedLightAssets { lights, cookies, shadows, common_shader, light_shader, shadow_shader })
            .insert_resource(ProjectedDisabled(disabled))
            .init_resource::<CookieAtlas>()
            .init_resource::<ProjectedFrame>()
            .add_plugins((ExtractResourcePlugin::<ProjectedFrame>::default(), ExtractResourcePlugin::<ProjectedLightAssets>::default()))
            .add_systems(PostUpdate, gather_lights.after(bevy::transform::TransformSystems::Propagate));
        crate::projected_render::build(app);
    }
}

/// `src` (RGBA8, `w` x `h`) resampled bilinearly to `size` x `size`.
pub fn resample_rgba(src: &[u8], w: u32, h: u32, size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let px = |x: u32, y: u32, c: usize| src[((y.min(h - 1) * w + x.min(w - 1)) * 4) as usize + c] as f32;
    for y in 0..size {
        let fy = ((y as f32 + 0.5) * h as f32 / size as f32 - 0.5).max(0.0);
        let (y0, ty) = (fy.floor() as u32, fy.fract());
        for x in 0..size {
            let fx = ((x as f32 + 0.5) * w as f32 / size as f32 - 0.5).max(0.0);
            let (x0, tx) = (fx.floor() as u32, fx.fract());
            for c in 0..4 {
                let top = px(x0, y0, c) * (1.0 - tx) + px(x0 + 1, y0, c) * tx;
                let bottom = px(x0, y0 + 1, c) * (1.0 - tx) + px(x0 + 1, y0 + 1, c) * tx;
                out[((y * size + x) * 4) as usize + c] = (top * (1.0 - ty) + bottom * ty).round() as u8;
            }
        }
    }
    out
}

/// A cookie texture's pixels for the atlas; white (a plain spotlight) when
/// it can't be read.
fn cookie_pixels(fs: Option<&GameFs>, name: &str) -> Vec<u8> {
    let decoded = fs.and_then(|fs| {
        let data = fs.read(&format!("materials/{}.vtf", crate::vmt::canonical_name(name)))?;
        let tex = crate::vtf::VtfTexture::parse(&data).ok()?;
        let px = tex.rgba8_level0()?;
        Some(resample_rgba(&px, tex.width, tex.height, COOKIE_SIZE))
    });
    decoded.unwrap_or_else(|| {
        warn!("source_assets: projected texture cookie {name:?} not found; using white");
        vec![255; (COOKIE_SIZE * COOKIE_SIZE * 4) as usize]
    })
}

/// Pick this frame's lights and write them where both draw paths read them.
#[allow(clippy::too_many_arguments)]
fn gather_lights(
    lights: Query<(&ProjectedLight, &GlobalTransform, &InheritedVisibility)>,
    cameras: Query<(&Camera, &GlobalTransform, Option<&bevy::camera::RenderTarget>), With<Camera3d>>,
    game: Res<SourceGame>,
    disabled: Res<ProjectedDisabled>,
    assets: Res<ProjectedLightAssets>,
    mut atlas: ResMut<CookieAtlas>,
    mut frame: ResMut<ProjectedFrame>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // the player's view: the first active camera drawing to a window
    // (portal views render to images)
    let camera = cameras
        .iter()
        .filter(|(c, _, target)| c.is_active && target.is_none_or(|t| matches!(t, bevy::camera::RenderTarget::Window(_))))
        .min_by_key(|(c, _, _)| c.order)
        .map(|(_, t, _)| t.translation())
        .unwrap_or(Vec3::ZERO);
    let active: Vec<(&ProjectedLight, &GlobalTransform)> = if disabled.0 {
        Vec::new()
    } else {
        lights.iter().filter(|(l, _, v)| l.enabled && v.get() && l.color.max_element() > 0.0).map(|(l, t, _)| (l, t)).collect()
    };
    let picks = select_lights(&active.iter().map(|(l, t)| (t.translation(), l.shadows)).collect::<Vec<_>>(), camera);
    let mut gpu = GpuProjectedLights::default();
    let mut out = Vec::new();
    for (slot, (i, shadow)) in picks.into_iter().enumerate() {
        let (l, t) = active[i];
        let layer = match atlas.names.iter().position(|n| *n == l.cookie) {
            Some(layer) => layer,
            None if atlas.names.len() < MAX_COOKIES => {
                frame.cookies.push(Arc::new(cookie_pixels(game.fs.as_deref(), &l.cookie)));
                atlas.names.push(l.cookie.clone());
                atlas.names.len() - 1
            }
            None => 0,
        };
        let view_proj = light_view_proj(t, l.fov, l.near, l.far);
        let pos = t.translation();
        gpu.lights[slot] = GpuProjectedLight {
            view_proj: view_proj.to_cols_array_2d(),
            position: [pos.x, pos.y, pos.z, l.far],
            color: [l.color.x, l.color.y, l.color.z, l.near],
            params: [shadow.map_or(-1.0, |s| s as f32), layer as f32, l.light_world as u32 as f32, shared::SOURCE_UNIT],
        };
        out.push(FrameLight { view_proj, position: pos, far: l.far, shadow_layer: shadow, light_world: l.light_world });
    }
    gpu.info[0] = out.len() as u32;
    frame.lights = out;
    if frame.gpu != gpu {
        frame.gpu = gpu;
        if let Some(mut b) = buffers.get_mut(&assets.lights) {
            b.data = Some(bytemuck::bytes_of(&gpu).to_vec());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn colour_from_keyvalues() {
        let c = light_color("252 247 239 100", 2.0);
        assert!(approx(c.x, 252.0 / 255.0 * 100.0 / 255.0 * 2.0), "{c}");
        assert!(approx(c.z, 239.0 / 255.0 * 100.0 / 255.0 * 2.0), "{c}");
        // no intensity: full
        assert_eq!(light_color("255 0 0", 1.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(light_color("", 1.0), Vec3::ONE);
    }

    #[test]
    fn flashlight_attenuation() {
        assert_eq!(attenuation(50.0, 750.0), 1.0);
        assert!(approx(attenuation(200.0, 750.0), 0.5));
        // fades out between 60% of far and far
        assert!(approx(attenuation(450.0, 750.0), 100.0 / 450.0));
        assert!(approx(attenuation(600.0, 750.0), 100.0 / 600.0 * 0.5));
        assert_eq!(attenuation(800.0, 750.0), 0.0);
    }

    #[test]
    fn frustum_points_along_source_forward() {
        // a light at the origin facing Source +X (no rotation)
        let t = GlobalTransform::IDENTITY;
        let vp = light_view_proj(&t, 90.0, 0.1, 10.0);
        let c = vp * Vec4::new(5.0, 0.0, 0.0, 1.0);
        assert!(approx(c.w, 5.0), "w is the distance along forward: {c}");
        assert!(approx(c.x / c.w, 0.0) && approx(c.y / c.w, 0.0));
        assert!((0.0..=1.0).contains(&(c.z / c.w)));
        // up (+Y) is up in the cookie, the 90 degree edge at 45 degrees
        let e = vp * Vec4::new(5.0, 5.0, 0.0, 1.0);
        assert!(approx(e.y / e.w, 1.0), "{e}");
        assert!(sphere_in_frustum(&vp, Vec3::new(5.0, 0.0, 0.0), 0.1));
        assert!(!sphere_in_frustum(&vp, Vec3::new(-5.0, 0.0, 0.0), 0.1), "behind");
        assert!(!sphere_in_frustum(&vp, Vec3::new(20.0, 0.0, 0.0), 0.1), "beyond far");
        assert!(sphere_in_frustum(&vp, Vec3::new(20.0, 0.0, 0.0), 10.5), "big sphere reaches in");
        assert!(!sphere_in_frustum(&vp, Vec3::new(5.0, 8.0, 0.0), 1.0), "outside the cone");
        // turned to face world +Z (yaw -90 in Source terms)
        let t = GlobalTransform::from(Transform::from_rotation(crate::entities::source_angles_to_world(Vec3::new(0.0, -90.0, 0.0))));
        let vp = light_view_proj(&t, 60.0, 0.1, 10.0);
        let c = vp * Vec4::new(0.0, 0.0, 4.0, 1.0);
        assert!(approx(c.w, 4.0) && approx(c.x / c.w, 0.0), "{c}");
    }

    #[test]
    fn nearest_lights_win_and_get_shadows() {
        let mut lights: Vec<(Vec3, bool)> = (0..12).map(|i| (Vec3::new(i as f32, 0.0, 0.0), true)).collect();
        lights[1].1 = false;
        let picks = select_lights(&lights, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(picks.len(), MAX_LIGHTS);
        assert_eq!(picks.iter().map(|p| p.0).collect::<Vec<_>>(), (0..MAX_LIGHTS).collect::<Vec<_>>());
        assert_eq!(picks[0].1, Some(0));
        assert_eq!(picks[1].1, None, "unshadowed");
        assert_eq!(picks[2].1, Some(1));
        let shadowed = picks.iter().filter(|p| p.1.is_some()).count();
        assert_eq!(shadowed, MAX_SHADOWS);
        assert!(select_lights(&[], Vec3::ZERO).is_empty());
    }

    #[test]
    fn gpu_layout_matches_the_shaders() {
        // ProjLight is 7 vec4s, ProjLights a vec4 header then the array
        assert_eq!(std::mem::size_of::<GpuProjectedLight>(), 112);
        assert_eq!(std::mem::size_of::<GpuProjectedLights>(), 16 + 112 * MAX_LIGHTS);
    }

    #[test]
    fn resamples_cookies() {
        // 2x2 checker to 4x4: corners keep their colours
        let src = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
        let out = resample_rgba(&src, 2, 2, 4);
        assert_eq!(out.len(), 64);
        assert_eq!(&out[..4], &[255, 0, 0, 255]);
        assert_eq!(&out[60..], &[255, 255, 255, 255]);
        let flat = resample_rgba(&[9, 9, 9, 9], 1, 1, 8);
        assert!(flat.iter().all(|&b| b == 9));
    }
}
