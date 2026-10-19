// Shadow maps for projected lights: each texel stores the distance along
// the light's forward axis over its far plane (`projected_render.rs`).
struct ShadowDraw {
    // Source-unit model space -> light clip
    clip_from_local: mat4x4<f32>,
    base_u: vec4<f32>,
    base_v: vec4<f32>,
    // x base texture slot word, y flags (1 alpha test), z alpha cutoff (f32 bits), w far (f32 bits, metres)
    info: vec4<u32>,
};

@group(0) @binding(0) var<storage, read> draws: array<ShadowDraw>;
@group(1) @binding(0) var sm3_textures_2d: binding_array<texture_2d<f32>>;
@group(1) @binding(3) var sm3_samplers: binding_array<sampler>;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) distance: f32,
    @location(1) uv: vec2<f32>,
    @interpolate(flat) @location(2) draw: u32,
};

@vertex
fn vertex(@builtin(instance_index) draw: u32, @location(0) position: vec4<f32>, @location(2) texcoord: vec4<f32>) -> VertexOut {
    let d = draws[draw];
    var out: VertexOut;
    out.clip = d.clip_from_local * vec4<f32>(position.xyz, 1.0);
    out.distance = out.clip.w;
    let tc = vec4<f32>(texcoord.xy, 0.0, 1.0);
    out.uv = vec2<f32>(dot(d.base_u, tc), dot(d.base_v, tc));
    out.draw = draw;
    return out;
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let d = draws[in.draw];
    let slot = d.info.x;
    let albedo = textureSample(sm3_textures_2d[slot & 0xffffu], sm3_samplers[slot >> 16u], in.uv);
    if (d.info.y & 1u) != 0u && albedo.a < bitcast<f32>(d.info.z) {
        discard;
    }
    return vec4<f32>(in.distance / bitcast<f32>(d.info.w), 0.0, 0.0, 0.0);
}
