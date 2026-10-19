// Portal 2 Refract and Water materials (`glass.rs`): the opaque scene behind
// (Bevy's transmission copy) offset by a normal map, tinted, with a
// reflection; water also fades into its fog colour.
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, view_transmission_texture, view_transmission_sampler},
}

struct GlassParams {
    refract_tint: vec4<f32>,
    reflect_tint: vec4<f32>,
    envmap_tint: vec4<f32>,
    fog_color: vec4<f32>,
    fog: vec4<f32>,
    normal_u: vec4<f32>,
    normal_v: vec4<f32>,
    base_u: vec4<f32>,
    base_v: vec4<f32>,
    flags: u32,
};

const FLAG_WATER: u32 = 1u;
const FLAG_NORMAL: u32 = 2u;
const FLAG_BASE: u32 = 4u;
const FLAG_TINT_TEXTURE: u32 = 8u;
const FLAG_ENVMAP: u32 = 16u;
const FLAG_LIGHTMAP: u32 = 32u;
const FLAG_VERTEX_LIGHT: u32 = 64u;

// Estimates, see glass.rs.
const REFRACT_SCALE: f32 = 0.1;
const WATER_DEPTH: f32 = 64.0;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: GlassParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var normal_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var normal_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var base_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var base_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var tint_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var tint_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var env_tex: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var env_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var lm_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var lm_samp: sampler;

fn apply_rows(u: vec4<f32>, v: vec4<f32>, uv: vec2<f32>) -> vec2<f32> {
    let p = vec3<f32>(uv, 1.0);
    return vec2<f32>(dot(u.xyz, p), dot(v.xyz, p));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS_A
    let uv = in.uv;
#else
    let uv = vec2<f32>(0.0);
#endif
    let flags = params.flags;
    let exposure = params.fog.y;
    let unit = params.fog.z;
    let world = in.world_position.xyz;

    // tangent-space normal (flat when there is no normal map)
    let n_sample = textureSample(normal_tex, normal_samp, apply_rows(params.normal_u, params.normal_v, uv)).xyz * 2.0 - 1.0;
    var n_ts = vec3<f32>(0.0, 0.0, 1.0);
    if (flags & FLAG_NORMAL) != 0u {
        n_ts = normalize(n_sample);
    }

    // surface frame from screen derivatives (no tangent attributes needed)
    var n_geo = normalize(in.world_normal);
    if !is_front {
        n_geo = -n_geo;
    }
    let dp1 = dpdx(world);
    let dp2 = dpdy(world);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let dp2perp = cross(dp2, n_geo);
    let dp1perp = cross(n_geo, dp1);
    let t = dp2perp * duv1.x + dp1perp * duv2.x;
    let b = dp2perp * duv1.y + dp1perp * duv2.y;
    let inv = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-12));
    let n = normalize(t * inv * n_ts.x + b * inv * n_ts.y + n_geo * n_ts.z);

    let to_eye = view.world_position - world;
    let dist_units = length(to_eye) / unit;
    let v = to_eye / max(length(to_eye), 1e-5);

    // the scene behind, pushed by the normal map
    let size = vec2<f32>(textureDimensions(view_transmission_texture));
    let screen = in.position.xy / size;
    let amount = params.refract_tint.w * REFRACT_SCALE * min(1.0, 128.0 / max(dist_units, 1.0));
    let ruv = clamp(screen + n_ts.xy * vec2<f32>(1.0, -1.0) * amount, vec2<f32>(0.0), vec2<f32>(1.0));
    var behind = textureSampleLevel(view_transmission_texture, view_transmission_sampler, ruv, 0.0).rgb;
    let blur = params.envmap_tint.w;
    if blur > 0.0 {
        // $bluramount: a small box around the refracted point
        let o = blur * 2.0 / size;
        behind = (behind
            + textureSampleLevel(view_transmission_texture, view_transmission_sampler, ruv + vec2<f32>(o.x, o.y), 0.0).rgb
            + textureSampleLevel(view_transmission_texture, view_transmission_sampler, ruv + vec2<f32>(-o.x, o.y), 0.0).rgb
            + textureSampleLevel(view_transmission_texture, view_transmission_sampler, ruv + vec2<f32>(o.x, -o.y), 0.0).rgb
            + textureSampleLevel(view_transmission_texture, view_transmission_sampler, ruv - o, 0.0).rgb) / 5.0;
    }
    let base_uv = apply_rows(params.base_u, params.base_v, uv);
    var refracted = behind * params.refract_tint.rgb;
    let tint = textureSample(tint_tex, tint_samp, base_uv).rgb;
    if (flags & FLAG_TINT_TEXTURE) != 0u {
        refracted *= tint;
    }

    // reflection: the material's cubemap (water: or the map's sky)
    let env = textureSample(env_tex, env_samp, reflect(-v, n)).rgb * exposure;
    let has_env = (flags & FLAG_ENVMAP) != 0u;

    var color: vec3<f32>;
    if (flags & FLAG_WATER) != 0u {
        // fog along the view ray through an assumed depth of water
        let ndv_geo = max(dot(n_geo, v), 0.15);
        let ray = WATER_DEPTH / ndv_geo;
        let fog = clamp((ray - params.fog_color.w) / (params.fog.x - params.fog_color.w), 0.0, 1.0);
        let body = mix(refracted, params.fog_color.rgb * exposure, fog);
        var reflection = vec3<f32>(0.0);
        if has_env {
            reflection = env * params.reflect_tint.rgb;
        }
        let fresnel = 0.02 + 0.98 * pow(1.0 - max(dot(n, v), 0.0), 5.0);
        color = mix(body, reflection, fresnel);
    } else {
        color = refracted;
        if has_env {
            color += env * params.envmap_tint.rgb;
        }
    }

    // frames and dirt over the glass, lit like the surface around them
    let base = textureSample(base_tex, base_samp, base_uv);
    if (flags & FLAG_BASE) != 0u {
        var light = vec3<f32>(1.0);
        if (flags & FLAG_LIGHTMAP) != 0u {
#ifdef VERTEX_UVS_B
            light = textureSample(lm_tex, lm_samp, in.uv_b).rgb;
#endif
        }
        if (flags & FLAG_VERTEX_LIGHT) != 0u {
#ifdef VERTEX_COLORS
            light = in.color.rgb;
#endif
        }
        color = mix(color, base.rgb * light * exposure, base.a);
    }
    return vec4<f32>(color, 1.0);
}
