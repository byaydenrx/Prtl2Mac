//! Source's `_rt_WaterReflection`: water whose material reflects the world
//! (`source_assets::glass::WorldReflectWater`, e.g. the GLaDOS chamber's
//! `nature/water_glados_01`) samples a view of the scene mirrored in its
//! plane. The camera here renders that view: the main camera's pose
//! mirrored in the nearest such plane below the eye, with an oblique near
//! plane at the water so nothing under it (the floor the eye would sit
//! below) is drawn. A mirror is not a rotation, so the camera keeps a
//! proper rotation (mirrored forward and up) and the shader samples it
//! x-flipped. The water itself is drawn in the main view only
//! (`LAYER_MAIN_VIEW_ONLY`), so the camera (layer 0) never samples its own
//! target.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Exposure, Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use p2_core::*;
use p2_render::PortalProjection;
use source_assets::glass::{WaterReflection, WorldReflectWater};

/// The reflection target's size as a fraction of the window.
const SCALE: f32 = 0.5;

pub struct WaterReflectionPlugin;

impl Plugin for WaterReflectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, water_main_only)
            .add_systems(PostUpdate, update_camera.before(TransformSystems::Propagate));
    }
}

#[derive(Component)]
struct WaterReflectionCamera;

/// World-reflecting water is drawn by the main camera only.
fn water_main_only(mut commands: Commands, q: Query<Entity, Added<WorldReflectWater>>) {
    for e in &q {
        commands.entity(e).insert(RenderLayers::layer(LAYER_MAIN_VIEW_ONLY));
    }
}

/// Mirrors `eye` (position, forward, up) in the plane `y = h`.
pub fn mirror(pos: Vec3, forward: Vec3, up: Vec3, h: f32) -> (Vec3, Vec3, Vec3) {
    let m = |v: Vec3| Vec3::new(v.x, -v.y, v.z);
    (Vec3::new(pos.x, 2.0 * h - pos.y, pos.z), m(forward), m(up))
}

/// The oblique clip plane `(n, w)` in view space of a camera with rotation
/// `rot` at `pos` that keeps only what is above `y = h`.
pub fn clip_plane(rot: Quat, pos: Vec3, h: f32) -> Vec4 {
    (rot.inverse() * Vec3::Y).extend(pos.y - h)
}

#[allow(clippy::type_complexity)]
fn update_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    target: Res<WaterReflection>,
    mut params: ParamSet<(TransformHelper, Query<(Entity, &mut Camera, &mut Transform, &mut Projection), (With<WaterReflectionCamera>, Without<MainCamera>)>)>,
    main: Query<(Entity, &Camera, &Projection, Option<&Exposure>, Option<&DistanceFog>), (With<MainCamera>, Without<WaterReflectionCamera>)>,
    water: Query<(&WorldReflectWater, &GlobalTransform)>,
) {
    let Some((main_e, main_cam, main_proj, exposure, fog)) = main.iter().next() else { return };
    let Ok(eye) = params.p0().compute_global_transform(main_e) else { return };
    let eye_pos = eye.translation();
    // the highest world-reflecting water plane below the eye
    let plane = water
        .iter()
        .map(|(w, g)| g.transform_point(Vec3::Y * w.height).y)
        .filter(|&h| h < eye_pos.y)
        .max_by(f32::total_cmp);
    let mut cam = params.p1();
    let Ok((cam_e, mut camera, mut tf, mut proj)) = cam.single_mut() else {
        if plane.is_some() {
            commands.spawn((
                Name::new("Water reflection"),
                WaterReflectionCamera,
                Camera3d::default(),
                Camera { order: -10_000, is_active: false, ..default() },
                RenderTarget::Image(target.0.clone().into()),
                Projection::custom(PortalProjection::default()),
                Hdr,
                Tonemapping::None,
                DebandDither::Disabled,
                Msaa::Off,
                RenderLayers::layer(0),
                Transform::default(),
            ));
        }
        return;
    };
    let Some(h) = plane else {
        camera.is_active = false;
        return;
    };
    let size = main_cam.physical_target_size().unwrap_or(UVec2::new(1280, 720)).max(UVec2::ONE);
    let want = (size.as_vec2() * SCALE).as_uvec2().max(UVec2::ONE);
    if let Some(img) = images.get(&target.0) {
        if img.size() != want {
            // a fresh texture (resizing would copy the old one)
            let mut img = Image::new_target_texture(want.x, want.y, TextureFormat::Rgba16Float, None);
            img.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
            let _ = images.insert(&target.0, img);
        }
    }
    let (pos, fwd, up) = mirror(eye_pos, eye.forward().as_vec3(), eye.up().as_vec3(), h);
    *tf = Transform::from_translation(pos).looking_to(fwd, up);
    let p = match main_proj {
        Projection::Perspective(p) => p.clone(),
        _ => PerspectiveProjection::default(),
    };
    *proj = Projection::custom(PortalProjection {
        fov: p.fov,
        aspect_ratio: size.x as f32 / size.y as f32,
        near: p.near,
        far: p.far,
        clip_plane: Some(clip_plane(tf.rotation, pos, h)),
    });
    camera.is_active = true;
    let mut ec = commands.entity(cam_e);
    match exposure {
        Some(x) => ec.insert(*x),
        None => ec.remove::<Exposure>(),
    };
    match fog {
        Some(f) => ec.insert(f.clone()),
        None => ec.remove::<DistanceFog>(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrored_camera_is_below_and_sees_only_above() {
        let (pos, fwd, up) = mirror(Vec3::new(1.0, 3.0, 2.0), Vec3::new(0.0, -0.6, -0.8), Vec3::new(0.0, 0.8, -0.6), 1.0);
        assert_eq!(pos, Vec3::new(1.0, -1.0, 2.0));
        assert_eq!(fwd, Vec3::new(0.0, 0.6, -0.8));
        let tf = Transform::from_translation(pos).looking_to(fwd, up);
        let plane = clip_plane(tf.rotation, pos, 1.0);
        let view = |w: Vec3| tf.compute_affine().inverse().transform_point3(w);
        let side = |w: Vec3| plane.truncate().dot(view(w)) + plane.w;
        // above the water is kept, below (and the camera itself) is clipped
        assert!(side(Vec3::new(0.0, 2.0, -5.0)) > 0.0);
        assert!(side(Vec3::new(0.0, 0.5, -5.0)) < 0.0);
        assert!(side(pos) < 0.0);
    }
}
