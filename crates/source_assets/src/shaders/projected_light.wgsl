// Additive projected-light pass over surfaces drawn by Portal 2's own
// shaders (`projected_render.rs`): albedo * sum of lights, times the
// exposure, faded by the map's fog.
#import source_assets::projected::{ProjLights, projected_light}

struct Draw {
    // Source-unit model space -> world (metres)
    world_from_local: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    // base texture transform rows (dot with (u, v, 0, 1))
    base_u: vec4<f32>,
    base_v: vec4<f32>,
    // xyz camera (world), w exposure
    camera: vec4<f32>,
    // Valve fog params: x -start/range, z max density, w 1/range (per metre)
    fog: vec4<f32>,
    // x base texture slot word, y flags (1 alpha test, 2 brush), z light mask, w alpha cutoff (f32 bits)
    info: vec4<u32>,
};

@group(0) @binding(0) var<storage, read> draws: array<Draw>;
@group(1) @binding(0) var sm3_textures_2d: binding_array<texture_2d<f32>>;
@group(1) @binding(3) var sm3_samplers: binding_array<sampler>;
@group(2) @binding(0) var<storage, read> lights: ProjLights;
@group(2) @binding(1) var cookies: texture_2d_array<f32>;
@group(2) @binding(2) var cookie_sampler: sampler;
@group(2) @binding(3) var shadows: texture_2d_array<f32>;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @interpolate(flat) @location(3) draw: u32,
};

@vertex
fn vertex(
    @builtin(instance_index) draw: u32,
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) texcoord: vec4<f32>,
) -> VertexOut {
    let d = draws[draw];
    let world = d.world_from_local * vec4<f32>(position.xyz, 1.0);
    var out: VertexOut;
    out.clip = d.clip_from_world * world;
    out.world = world.xyz;
    out.normal = (d.world_from_local * vec4<f32>(normal.xyz, 0.0)).xyz;
    let tc = vec4<f32>(texcoord.xy, 0.0, 1.0);
    out.uv = vec2<f32>(dot(d.base_u, tc), dot(d.base_v, tc));
    out.draw = draw;
    return out;
}

@fragment
fn fragment(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let d = draws[in.draw];
    let slot = d.info.x;
    let albedo = textureSample(sm3_textures_2d[slot & 0xffffu], sm3_samplers[slot >> 16u], in.uv);
    if (d.info.y & 1u) != 0u && albedo.a < bitcast<f32>(d.info.w) {
        discard;
    }
    var n = normalize(in.normal);
    if !front {
        n = -n;
    }
    let brush = (d.info.y & 2u) != 0u;
    var sum = vec3<f32>(0.0);
    let count = min(lights.info.x, 8u);
    for (var i = 0u; i < count; i++) {
        let l = lights.lights[i];
        if (d.info.z & (1u << i)) == 0u || (brush && l.params.z < 0.5) {
            continue;
        }
        sum += projected_light(l, in.world, n, cookies, cookie_sampler, shadows);
    }
    // linear fog, as the Valve vertex shaders compute it
    let dist = distance(in.world, d.camera.xyz);
    let fog = clamp(dist * d.fog.w + d.fog.x, 0.0, d.fog.z);
    return vec4<f32>(albedo.rgb * sum * d.camera.w * (1.0 - fog), 0.0);
}
