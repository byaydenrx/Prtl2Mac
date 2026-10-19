//! Render-world side of [`crate::projected`] for surfaces drawn by Portal
//! 2's own shaders ([`crate::valve_render`]):
//!
//! 1. cookie pixels are copied into the cookie texture array when new;
//! 2. each shadowed light renders a shadow map (distance / far, R32Float)
//!    from the Valve meshes inside its frustum, submitted before the frame's
//!    main passes;
//! 3. after the Valve main pass, every lit, opaque surface touched by a
//!    light frustum is drawn again additively with `projected_light.wgsl`
//!    (albedo from its base texture, same depth, no depth writes).
//!
//! Translucent surfaces and decals/overlays get no projected light (decals
//! would be lit twice, once through the brush under them); unlit surfaces
//! cast shadows but aren't lit. Meshes drawn by the fallback `SourceMaterial`
//! are lit by its own shader but don't cast shadows.

use crate::projected::{sphere_in_frustum, FrameLight, ProjectedFrame, ProjectedLightAssets, COOKIE_SIZE, SHADOW_SIZE};
use crate::valve::FatVertex;
use crate::valve_render::{prepare_surfaces, valve_main_pass, ExtractedSurfaces, PreparedSurfaces};
use crate::vmt::BlendKind;
use bevy::core_pipeline::core_3d::{main_transparent_pass_3d, CORE_3D_DEPTH_FORMAT};
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::Core3dSystems;
use bevy::math::Mat4;
use bevy::mesh::VertexBufferLayout;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::storage::GpuShaderBuffer;
use bevy::render::texture::GpuImage;
use bevy::render::view::{ExtractedView, ViewDepthTexture, ViewTarget};
use bevy::render::{Render, RenderApp, RenderSystems};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn build(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
    render_app
        .init_resource::<ProjectedRender>()
        .add_systems(Render, prepare_projected.in_set(RenderSystems::PrepareBindGroups).after(prepare_surfaces))
        .add_systems(
            Core3d,
            projected_light_pass
                .in_set(Core3dSystems::MainPass)
                .after(valve_main_pass)
                .before(main_transparent_pass_3d),
        );
}

/// One surface in the light pass (`Draw` in `projected_light.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightRecord {
    world_from_local: [[f32; 4]; 4],
    clip_from_world: [[f32; 4]; 4],
    base_u: [f32; 4],
    base_v: [f32; 4],
    camera: [f32; 4],
    fog: [f32; 4],
    info: [u32; 4],
}

/// One shadow caster for one light (`ShadowDraw` in `projected_shadow.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowRecord {
    clip_from_local: [[f32; 4]; 4],
    base_u: [f32; 4],
    base_v: [f32; 4],
    info: [u32; 4],
}

const FLAG_ALPHA_TEST: u32 = 1;
const FLAG_BRUSH: u32 = 2;

struct LightDraw {
    pipeline: CachedRenderPipelineId,
    mesh: usize,
    base: u32,
}

#[derive(Resource, Default)]
struct ProjectedRender {
    draws_layout: Option<BindGroupLayoutDescriptor>,
    lights_layout: Option<BindGroupLayoutDescriptor>,
    light_pipelines: HashMap<(bool, TextureFormat, u32), CachedRenderPipelineId>,
    shadow_pipeline: Option<CachedRenderPipelineId>,
    cookie_sampler: Option<Sampler>,
    shadow_depth: Option<TextureView>,
    /// What each cookie layer was last filled from.
    uploaded: Vec<usize>,
    draw_buffer: Option<Buffer>,
    shadow_buffer: Option<Buffer>,
    draws_bind: Option<BindGroup>,
    lights_bind: Option<BindGroup>,
    views: HashMap<Entity, Vec<LightDraw>>,
}

fn storage_layout(label: &'static str) -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        label,
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    )
}

fn lights_layout() -> BindGroupLayoutDescriptor {
    let texture = |binding, filterable| BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::FRAGMENT,
        ty: BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable },
            view_dimension: TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    };
    BindGroupLayoutDescriptor::new(
        "projected_lights",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
            texture(1, true),
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::FRAGMENT, ty: BindingType::Sampler(SamplerBindingType::Filtering), count: None },
            texture(3, false),
        ],
    )
}

fn attribute(location: u32, offset: u64) -> VertexAttribute {
    VertexAttribute { format: VertexFormat::Float32x4, offset, shader_location: location }
}

fn light_pipeline(
    shader: &Handle<bevy::shader::Shader>,
    layouts: Vec<BindGroupLayoutDescriptor>,
    double_sided: bool,
    format: TextureFormat,
    samples: u32,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some("projected light".into()),
        layout: layouts,
        immediate_size: 0,
        vertex: VertexState {
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some("vertex".into()),
            buffers: vec![VertexBufferLayout {
                array_stride: FatVertex::STRIDE,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![attribute(0, 0), attribute(1, 16), attribute(2, 36)],
            }],
        },
        primitive: PrimitiveState {
            front_face: FrontFace::Ccw,
            cull_mode: if double_sided { None } else { Some(Face::Back) },
            ..default()
        },
        // Same depth as the surface drawn by the main pass (reverse Z); a
        // small bias toward the camera keeps the recomputed depth on top.
        depth_stencil: Some(DepthStencilState {
            format: CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: default(),
            bias: DepthBiasState { constant: 4, slope_scale: 1.0, clamp: 0.0 },
        }),
        multisample: MultisampleState { count: samples, ..default() },
        fragment: Some(FragmentState {
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format,
                blend: Some(BlendState {
                    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
                    alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
                }),
                write_mask: ColorWrites::ALL,
            })],
        }),
        zero_initialize_workgroup_memory: true,
    }
}

fn shadow_pipeline(shader: &Handle<bevy::shader::Shader>, layouts: Vec<BindGroupLayoutDescriptor>) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some("projected shadow".into()),
        layout: layouts,
        immediate_size: 0,
        vertex: VertexState {
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some("vertex".into()),
            buffers: vec![VertexBufferLayout {
                array_stride: FatVertex::STRIDE,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![attribute(0, 0), attribute(2, 36)],
            }],
        },
        // Both faces cast: chamber walls are often single brushes seen from inside.
        primitive: PrimitiveState { front_face: FrontFace::Ccw, cull_mode: None, ..default() },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Less),
            stencil: default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState { format: TextureFormat::R32Float, blend: None, write_mask: ColorWrites::ALL })],
        }),
        zero_initialize_workgroup_memory: true,
    }
}

/// A surface that may be lit or cast shadows this frame.
struct Candidate {
    mesh: usize,
    world_from_local: Mat4,
    base_u: [f32; 4],
    base_v: [f32; 4],
    slot: u32,
    flags: u32,
    cutoff: f32,
    double_sided: bool,
    /// Gets the light pass (lit, has a base texture binding).
    lit: bool,
    fog: [f32; 4],
    /// Bit per light whose frustum it touches.
    mask: u32,
}

fn write_buffer(device: &RenderDevice, queue: &RenderQueue, slot: &mut Option<Buffer>, label: &'static str, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let needed = bytes.len() as u64;
    let mut remade = false;
    if slot.as_ref().is_none_or(|b| b.size() < needed) {
        *slot = Some(device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: needed.next_power_of_two().max(1 << 14),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        remade = true;
    }
    queue.write_buffer(slot.as_ref().unwrap(), 0, bytes);
    remade
}

#[allow(clippy::too_many_arguments)]
fn prepare_projected(
    frame: Option<Res<ProjectedFrame>>,
    assets: Option<Res<ProjectedLightAssets>>,
    extracted: Res<ExtractedSurfaces>,
    prepared: Res<PreparedSurfaces>,
    mut pr: ResMut<ProjectedRender>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    views: Query<(Entity, &ExtractedView, &ViewTarget, &Msaa, Option<&bevy::camera::visibility::RenderLayers>)>,
) {
    let pr = &mut *pr;
    pr.views.clear();
    let (Some(frame), Some(assets)) = (frame, assets) else { return };
    let (Some(cookies), Some(shadows), Some(light_buf)) = (images.get(&assets.cookies), images.get(&assets.shadows), buffers.get(&assets.lights)) else {
        return;
    };
    if pr.draws_layout.is_none() {
        pr.draws_layout = Some(storage_layout("projected_draws"));
        pr.lights_layout = Some(lights_layout());
        pr.cookie_sampler = Some(device.create_sampler(&SamplerDescriptor {
            label: Some("projected cookie sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }));
        let depth = device.create_texture(&TextureDescriptor {
            label: Some("projected shadow depth"),
            size: Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        pr.shadow_depth = Some(depth.create_view(&TextureViewDescriptor::default()));
    }

    // ---- cookies ----
    for (layer, data) in frame.cookies.iter().enumerate() {
        let id = Arc::as_ptr(data) as usize;
        if pr.uploaded.get(layer) == Some(&id) {
            continue;
        }
        queue.write_texture(
            TexelCopyTextureInfo { texture: &cookies.texture, mip_level: 0, origin: Origin3d { x: 0, y: 0, z: layer as u32 }, aspect: TextureAspect::All },
            data,
            TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(COOKIE_SIZE * 4), rows_per_image: Some(COOKIE_SIZE) },
            Extent3d { width: COOKIE_SIZE, height: COOKIE_SIZE, depth_or_array_layers: 1 },
        );
        pr.uploaded.resize(layer + 1, 0);
        pr.uploaded[layer] = id;
    }

    let Some(textures_layout) = prepared.textures_layout.clone() else { return };
    let Some(texture_bind) = prepared.texture_bind.clone() else { return };
    if frame.lights.is_empty() || extracted.draws.is_empty() {
        return;
    }
    let draws_layout = pr.draws_layout.clone().unwrap();
    let lights_layout = pr.lights_layout.clone().unwrap();

    // ---- candidates ----
    let s2w = crate::valve::source_to_world_matrix();
    let mut candidates: Vec<Candidate> = Vec::new();
    for (d, model, over) in &extracted.draws {
        if matches!(d.blend, BlendKind::Blend | BlendKind::Additive) || d.decal {
            continue;
        }
        let key = (Arc::as_ptr(&d.vertices) as usize, Arc::as_ptr(&d.indices) as usize);
        let Some(&mesh) = prepared.mesh_index.get(&key) else { continue };
        let world_from_local = *model * s2w;
        let (lo, hi) = prepared.meshes[mesh].bounds;
        let scale = world_from_local.x_axis.truncate().length().max(world_from_local.y_axis.truncate().length()).max(world_from_local.z_axis.truncate().length());
        let center = world_from_local.transform_point3((lo + hi) * 0.5);
        let radius = (hi - lo).length() * 0.5 * scale;
        let mask = frame
            .lights
            .iter()
            .enumerate()
            .filter(|(_, l)| sphere_in_frustum(&l.view_proj, center, radius))
            .fold(0u32, |m, (i, _)| m | 1 << i);
        if mask == 0 {
            continue;
        }
        let vs = || d.vs_consts.iter().chain(over.iter().flat_map(|o| &o.vs_consts));
        let row = |r: u16, default: [f32; 4]| vs().filter(|(x, _)| *x == r).last().map_or(default, |(_, v)| *v);
        let base = d.textures.iter().find(|t| t.register == 0);
        let slot = base.map_or(0, |t| {
            let frame = over.as_ref().and_then(|o| o.frames.iter().find(|(r, _)| *r == 0)).and_then(|(_, f)| t.frames.get(*f as usize));
            let image = frame.unwrap_or(&t.image);
            d3d9_sm3::texture_slot_word(prepared.slot_of.get(&image.id()).copied().unwrap_or(0), t.sampler.index())
        });
        let key = d.pass.key;
        let unlit = key.vs.starts_with("vertexlit") && key.vs_static == 16;
        let mut flags = 0;
        if d.blend == BlendKind::Mask {
            flags |= FLAG_ALPHA_TEST;
        }
        if key.ps.starts_with("lightmappedgeneric") {
            flags |= FLAG_BRUSH;
        }
        let u = shared::SOURCE_UNIT;
        candidates.push(Candidate {
            mesh,
            world_from_local,
            base_u: row(48, [1.0, 0.0, 0.0, 0.0]),
            base_v: row(49, [0.0, 1.0, 0.0, 0.0]),
            slot,
            flags,
            cutoff: key.alpha_ref.map_or(0.0, |r| r as f32 / 255.0),
            double_sided: d.double_sided,
            lit: !unlit && base.is_some(),
            fog: [d.fog[0], d.fog[1], d.fog[2], d.fog[3] / u],
            mask,
        });
    }

    // ---- shadow maps ----
    let shadowed: Vec<(usize, &FrameLight, u32)> = frame.lights.iter().enumerate().filter_map(|(i, l)| Some((i, l, l.shadow_layer?))).collect();
    if !shadowed.is_empty() {
        let pipeline_id = *pr
            .shadow_pipeline
            .get_or_insert_with(|| pipeline_cache.queue_render_pipeline(shadow_pipeline(&assets.shadow_shader, vec![draws_layout.clone(), textures_layout.clone()])));
        if let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) {
            let mut records: Vec<ShadowRecord> = Vec::new();
            let mut ranges: Vec<(u32, Vec<(usize, u32)>)> = Vec::new();
            for &(i, l, layer) in &shadowed {
                let mut list = Vec::new();
                for c in candidates.iter().filter(|c| c.mask & (1 << i) != 0) {
                    list.push((c.mesh, records.len() as u32));
                    records.push(ShadowRecord {
                        clip_from_local: (l.view_proj * c.world_from_local).to_cols_array_2d(),
                        base_u: c.base_u,
                        base_v: c.base_v,
                        info: [c.slot, c.flags & FLAG_ALPHA_TEST, c.cutoff.to_bits(), l.far.to_bits()],
                    });
                }
                ranges.push((layer, list));
            }
            if records.is_empty() {
                // something to bind even with no casters
                records.push(bytemuck::Zeroable::zeroed());
            }
            write_buffer(&device, &queue, &mut pr.shadow_buffer, "projected shadow draws", bytemuck::cast_slice(&records));
            let layout = pipeline_cache.get_bind_group_layout(&draws_layout);
            let bind = device.create_bind_group(
                "projected shadow draws",
                &layout,
                &[BindGroupEntry { binding: 0, resource: pr.shadow_buffer.as_ref().unwrap().as_entire_binding() }],
            );
            let depth = pr.shadow_depth.as_ref().unwrap();
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("projected shadows") });
            for (layer, list) in &ranges {
                let view = shadows.texture.create_view(&TextureViewDescriptor {
                    label: Some("projected shadow layer"),
                    dimension: Some(TextureViewDimension::D2),
                    base_array_layer: *layer,
                    array_layer_count: Some(1),
                    ..default()
                });
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("projected shadow"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        // nothing in the way: as far as the far plane
                        ops: Operations { load: LoadOp::Clear(LinearRgba::WHITE.into()), store: StoreOp::Store },
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: depth,
                        depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: StoreOp::Discard }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, Some(&*bind), &[]);
                pass.set_bind_group(1, Some(&*texture_bind), &[]);
                for &(mesh, base) in list {
                    let m = &prepared.meshes[mesh];
                    pass.set_vertex_buffer(0, (*m.vertex).slice(..));
                    pass.set_index_buffer((*m.index).slice(..), IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, base..base + 1);
                }
            }
            queue.submit([encoder.finish()]);
        }
    }

    // ---- light pass, per view ----
    let mut records: Vec<LightRecord> = Vec::new();
    for (view_entity, view, target, msaa, layers) in &views {
        if layers.is_some_and(|l| !l.intersects(&bevy::camera::visibility::RenderLayers::layer(0))) {
            continue;
        }
        let clip_from_world = view.clip_from_world.unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        let eye = view.world_from_view.translation();
        let mut list = Vec::new();
        for c in candidates.iter().filter(|c| c.lit) {
            let key = (c.double_sided, target.main_texture_format(), msaa.samples());
            let pipeline = *pr.light_pipelines.entry(key).or_insert_with(|| {
                pipeline_cache.queue_render_pipeline(light_pipeline(
                    &assets.light_shader,
                    vec![draws_layout.clone(), textures_layout.clone(), lights_layout.clone()],
                    key.0,
                    key.1,
                    key.2,
                ))
            });
            list.push(LightDraw { pipeline, mesh: c.mesh, base: records.len() as u32 });
            records.push(LightRecord {
                world_from_local: c.world_from_local.to_cols_array_2d(),
                clip_from_world: clip_from_world.to_cols_array_2d(),
                base_u: c.base_u,
                base_v: c.base_v,
                camera: [eye.x, eye.y, eye.z, extracted.exposure],
                fog: c.fog,
                info: [c.slot, c.flags, c.mask, c.cutoff.to_bits()],
            });
        }
        list.sort_by_key(|d| d.pipeline.id());
        pr.views.insert(view_entity, list);
    }
    if records.is_empty() {
        return;
    }
    write_buffer(&device, &queue, &mut pr.draw_buffer, "projected light draws", bytemuck::cast_slice(&records));
    let layout = pipeline_cache.get_bind_group_layout(&draws_layout);
    pr.draws_bind = Some(device.create_bind_group(
        "projected light draws",
        &layout,
        &[BindGroupEntry { binding: 0, resource: pr.draw_buffer.as_ref().unwrap().as_entire_binding() }],
    ));
    let layout = pipeline_cache.get_bind_group_layout(&lights_layout);
    pr.lights_bind = Some(device.create_bind_group(
        "projected lights",
        &layout,
        &[
            BindGroupEntry { binding: 0, resource: light_buf.buffer.as_entire_binding() },
            BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&cookies.texture_view) },
            BindGroupEntry { binding: 2, resource: BindingResource::Sampler(pr.cookie_sampler.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&shadows.texture_view) },
        ],
    ));
}

fn projected_light_pass(
    view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthTexture)>,
    pr: Res<ProjectedRender>,
    prepared: Res<PreparedSurfaces>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (camera, target, depth) = view.into_inner();
    let Some(draws) = pr.views.get(&view_entity).filter(|d| !d.is_empty()) else { return };
    let (Some(db), Some(tb), Some(lb)) = (&pr.draws_bind, &prepared.texture_bind, &pr.lights_bind) else { return };
    let color_attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("projected_lights"),
        color_attachments: &color_attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(viewport) = camera.viewport.as_ref() {
        pass.set_camera_viewport(viewport);
    }
    pass.set_bind_group(0, db, &[]);
    pass.set_bind_group(1, tb, &[]);
    pass.set_bind_group(2, lb, &[]);
    for d in draws {
        let Some(pipeline) = pipeline_cache.get_render_pipeline(d.pipeline) else { continue };
        let mesh = &prepared.meshes[d.mesh];
        pass.set_render_pipeline(pipeline);
        pass.set_vertex_buffer(0, mesh.vertex.slice(..));
        pass.set_index_buffer(mesh.index.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.count, 0, d.base..d.base + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse and validate a shader with the shared module pasted in place of
    /// its `#import` (what Bevy's shader composer does).
    fn validate(src: &str) {
        let common = include_str!("shaders/projected_common.wgsl").replace("#define_import_path source_assets::projected", "");
        let body: String = src.lines().filter(|l| !l.starts_with("#import")).collect::<Vec<_>>().join("\n");
        let module = naga::front::wgsl::parse_str(&format!("{common}\n{body}")).unwrap_or_else(|e| panic!("{}", e.emit_to_string(src)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{e:?}"));
    }

    #[test]
    fn shaders_validate() {
        validate(include_str!("shaders/projected_light.wgsl"));
        validate(include_str!("shaders/projected_shadow.wgsl"));
        validate("");
    }

    #[test]
    fn records_match_the_shader_structs() {
        // Draw: two mat4x4 + five vec4; ShadowDraw: mat4x4 + three vec4
        assert_eq!(std::mem::size_of::<LightRecord>(), 64 * 2 + 16 * 5);
        assert_eq!(std::mem::size_of::<ShadowRecord>(), 64 + 16 * 3);
    }
}
