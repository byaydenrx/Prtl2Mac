//! Image-space motion blur, the way Source's `dev/motion_blur` pass does it
//! (`mat_motion_blur_*` convars in client.dll): one blur vector for the whole
//! screen per frame instead of per-pixel motion vectors. Turning the view
//! smears along the turn; falling faster than `mat_motion_blur_falling_min`
//! (8) smears vertically when looking level and radially (streaks out from
//! the middle of the screen) when looking down or up, reaching full strength
//! at `mat_motion_blur_falling_max` (20). Off with Options' Motion Blur
//! (`mat_motion_blur_enabled`).
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::extract_component::{ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer_sized};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
    ColorWrites, FilterMode, FragmentState, LoadOp, Operations, PipelineCache, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, StoreOp, TextureFormat, TextureSampleType,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::sync_component::SyncComponent;
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use p2_core::MainCamera;
use shared::{CharacterVelocity, Player};

use crate::settings::Settings;

pub struct MotionBlurPlugin;

impl Plugin for MotionBlurPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "motion_blur.wgsl");
        app.add_plugins((ExtractComponentPlugin::<ViewBlur>::default(), UniformComponentPlugin::<ViewBlur>::default()))
            .add_systems(PostUpdate, update_blur.after(TransformSystems::Propagate));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<BlurPipeline>>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(Render, prepare_pipelines.in_set(RenderSystems::Prepare))
            .add_systems(Core3d, blur_pass.in_set(Core3dSystems::PostProcess));
    }
}

/// `mat_motion_blur_falling_min` / `_max`, read as falling speed in m/s
/// (8 m/s ~ 315 units/s, 20 m/s ~ 790 units/s).
const FALLING_MIN: f32 = 8.0;
const FALLING_MAX: f32 = 20.0;
/// `mat_motion_blur_percent_of_screen_max`: longest blur, fraction of the screen.
const MAX_BLUR: f32 = 0.04;
/// Rotation blur per screen fraction turned this frame (`rotation_intensity`
/// times a half-frame shutter).
const ROTATION_SCALE: f32 = 0.5;
/// Blurs shorter than this (screen fraction) are skipped.
const MIN_BLUR: f32 = 0.0004;
/// A bigger jump in one frame is a teleport (portal, setpos, level change),
/// not a turn: no rotation blur for it.
const TELEPORT_ANGLE: f32 = 0.35;

/// The main camera's blur this frame, in screen UV.
#[derive(Component, ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewBlur {
    /// Directional blur (UV, y down), half length each way.
    pub dir: Vec2,
    /// Radial blur: a pixel at `uv` is smeared toward the centre by
    /// `(uv - 0.5) * radial`.
    pub radial: f32,
    pub _pad: f32,
}

impl SyncComponent for ViewBlur {
    type Target = ViewBlur;
}

impl ExtractComponent for ViewBlur {
    type QueryData = &'static ViewBlur;
    type QueryFilter = With<Camera>;
    type Out = ViewBlur;
    fn extract_component(b: &ViewBlur) -> Option<ViewBlur> {
        // Under ~half a pixel (idle view sway) costs a pass for nothing.
        (b.dir.length() > MIN_BLUR || b.radial > MIN_BLUR).then_some(*b)
    }
}

/// The blur for a camera that turned by (`yaw`, `pitch`) radians (camera
/// space, right and up positive) this frame and falls at `fall` m/s, looking
/// along `fwd_y` (the view direction's world up component).
pub fn blur_for(yaw: f32, pitch: f32, hfov: f32, vfov: f32, fall: f32, fwd_y: f32) -> ViewBlur {
    let mut dir = Vec2::ZERO;
    if yaw.abs().max(pitch.abs()) < TELEPORT_ANGLE {
        dir = Vec2::new(yaw / hfov, -pitch / vfov) * ROTATION_SCALE;
    }
    let falling = ((fall - FALLING_MIN) / (FALLING_MAX - FALLING_MIN)).clamp(0.0, 1.0);
    let level = 1.0 - fwd_y.abs();
    dir.y += falling * level * MAX_BLUR;
    let len = dir.length();
    if len > MAX_BLUR {
        dir *= MAX_BLUR / len;
    }
    // Radial: twice the length at the screen edge (the offset grows from 0 in
    // the middle to 0.5 * radial at the edge).
    let radial = falling * fwd_y.abs() * MAX_BLUR * 2.0;
    ViewBlur { dir, radial, _pad: 0.0 }
}

fn update_blur(
    mut commands: Commands,
    settings: Option<Res<Settings>>,
    player: Query<&CharacterVelocity, With<Player>>,
    cams: Query<(Entity, &GlobalTransform, &Projection), With<MainCamera>>,
    mut last: Local<Option<Quat>>,
) {
    let Ok((cam, xf, proj)) = cams.single() else {
        *last = None;
        return;
    };
    let rot = xf.rotation();
    let prev = last.replace(rot);
    let enabled = settings.is_none_or(|s| s.motion_blur);
    let blur = match (enabled, prev, proj) {
        (true, Some(prev), Projection::Perspective(p)) => {
            let local = prev.inverse() * (rot * Vec3::NEG_Z);
            let yaw = local.x.atan2(-local.z);
            let pitch = local.y.atan2(-local.z);
            let fall = player.iter().next().map_or(0.0, |v| -v.y);
            let hfov = 2.0 * ((p.fov * 0.5).tan() * p.aspect_ratio).atan();
            blur_for(yaw, pitch, hfov, p.fov, fall, (rot * Vec3::NEG_Z).y)
        }
        _ => ViewBlur::default(),
    };
    if blur.dir.length() > MIN_BLUR || blur.radial > MIN_BLUR {
        debug!("view blur {:?} fall {:.1} m/s", blur, player.iter().next().map_or(0.0, |v| -v.y));
    }
    commands.entity(cam).insert(blur);
}

#[derive(Resource)]
struct BlurPipeline {
    sampler: Sampler,
    layout: BindGroupLayoutDescriptor,
    fullscreen: FullscreenShader,
    shader: Handle<Shader>,
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    assets: Res<AssetServer>,
) {
    let entries = BindGroupLayoutEntries::sequential(
        ShaderStages::FRAGMENT,
        (
            texture_2d(TextureSampleType::Float { filterable: true }),
            sampler(SamplerBindingType::Filtering),
            uniform_buffer_sized(false, Some(ViewBlur::min_size())),
        ),
    );
    commands.insert_resource(BlurPipeline {
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        layout: BindGroupLayoutDescriptor::new("view_blur_layout", &entries),
        fullscreen: fullscreen.clone(),
        shader: load_embedded_asset!(assets.as_ref(), "motion_blur.wgsl"),
    });
}

impl SpecializedRenderPipeline for BlurPipeline {
    type Key = TextureFormat;
    fn specialize(&self, format: TextureFormat) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("view_blur_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
struct BlurPipelineId(CachedRenderPipelineId);

fn prepare_pipelines(
    mut commands: Commands,
    cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlurPipeline>>,
    pipeline: Res<BlurPipeline>,
    views: Query<(Entity, &ExtractedView), With<ViewBlur>>,
) {
    for (e, view) in &views {
        let id = pipelines.specialize(&cache, &pipeline, view.target_format);
        commands.entity(e).insert(BlurPipelineId(id));
    }
}

fn blur_pass(
    view: ViewQuery<(&ViewTarget, &BlurPipelineId, &ViewBlur)>,
    pipeline: Res<BlurPipeline>,
    cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<ViewBlur>>,
    mut ctx: RenderContext,
) {
    let (target, id, _) = view.into_inner();
    let Some(render_pipeline) = cache.get_render_pipeline(id.0) else { return };
    let Some(binding) = uniforms.uniforms().binding() else { return };
    let post = target.post_process_write();
    let bind_group = ctx.render_device().create_bind_group(
        Some("view_blur_bind_group"),
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, &pipeline.sampler, binding.clone())),
    );
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("view_blur"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations { load: LoadOp::Clear(default()), store: StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(render_pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: f32 = 1.6;
    const V: f32 = 1.0;

    #[test]
    fn standing_still_is_sharp() {
        assert_eq!(blur_for(0.0, 0.0, H, V, 0.0, 0.0), ViewBlur::default());
        // Slow falls (a jump, a short drop) don't blur.
        assert_eq!(blur_for(0.0, 0.0, H, V, FALLING_MIN, 0.0), ViewBlur::default());
    }

    #[test]
    fn falling_level_blurs_vertically_looking_down_radially() {
        let level = blur_for(0.0, 0.0, H, V, FALLING_MAX, 0.0);
        assert!((level.dir.y - MAX_BLUR).abs() < 1e-6 && level.radial == 0.0);
        let down = blur_for(0.0, 0.0, H, V, FALLING_MAX + 10.0, -1.0);
        assert!(down.dir.length() < 1e-6 && (down.radial - 2.0 * MAX_BLUR).abs() < 1e-6);
        let half = blur_for(0.0, 0.0, H, V, (FALLING_MIN + FALLING_MAX) / 2.0, 0.0);
        assert!((half.dir.y - MAX_BLUR / 2.0).abs() < 1e-6);
    }

    #[test]
    fn turning_blurs_along_the_turn_but_teleports_dont() {
        let turn = blur_for(0.05, 0.0, H, V, 0.0, 0.0);
        assert!(turn.dir.x > 0.0 && turn.dir.y == 0.0 && turn.dir.length() <= MAX_BLUR + 1e-6);
        assert_eq!(blur_for(1.5, 0.0, H, V, 0.0, 0.0), ViewBlur::default());
    }
}
