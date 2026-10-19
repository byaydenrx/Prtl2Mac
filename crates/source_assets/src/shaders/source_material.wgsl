// Source-style surface: albedo x baked light (lightmap, per-vertex, or none).
#import bevy_pbr::{
    forward_io::VertexOutput,
    pbr_types,
    pbr_functions::main_pass_post_lighting_processing,
}
#import source_assets::projected::{ProjLights, projected_light}

struct SourceMaterialParams {
    color: vec4<f32>,
    light_scale: f32,
    alpha_cutoff: f32,
    flags: u32,
    // StandardMaterialFlags bits (fog, alpha mode) for Bevy's post-lighting step
    pbr_flags: u32,
    // $basetexturetransform rows (texture proxies scroll and flip it)
    uv_transform_u: vec4<f32>,
    uv_transform_v: vec4<f32>,
};

const FLAG_LIGHTMAP: u32 = 1u;
const FLAG_VERTEX_LIGHT: u32 = 2u;
const FLAG_UNLIT: u32 = 4u;
const FLAG_BLEND2: u32 = 8u;
const FLAG_ALPHA_TEST: u32 = 16u;
const FLAG_IGNORE_TEX_ALPHA: u32 = 32u;
const FLAG_LIGHT_ONLY: u32 = 64u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: SourceMaterialParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var lm_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var lm_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var base2_tex: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var base2_samp: sampler;
// projected texture lights (env_projectedtexture), see projected.rs
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var<storage, read> proj_lights: ProjLights;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var proj_cookies: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var proj_cookie_samp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var proj_shadows: texture_2d_array<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS_A
    let uv = in.uv;
#else
    let uv = vec2<f32>(0.0);
#endif
    let base_uv = vec2<f32>(
        dot(params.uv_transform_u.xyz, vec3<f32>(uv, 1.0)),
        dot(params.uv_transform_v.xyz, vec3<f32>(uv, 1.0)),
    );
    var albedo = textureSample(base_tex, base_samp, base_uv);
    let albedo2 = textureSample(base2_tex, base2_samp, uv);
    let lm = textureSample(lm_tex, lm_samp,
#ifdef VERTEX_UVS_B
        in.uv_b
#else
        vec2<f32>(0.0)
#endif
    ).rgb;

#ifdef VERTEX_COLORS
    if (params.flags & FLAG_BLEND2) != 0u {
        albedo = mix(albedo, albedo2, clamp(in.color.a, 0.0, 1.0));
    }
#endif

    var light = vec3<f32>(1.0);
    if (params.flags & FLAG_LIGHTMAP) != 0u {
        light = lm * params.light_scale;
    }
#ifdef VERTEX_COLORS
    if (params.flags & FLAG_VERTEX_LIGHT) != 0u {
        light = in.color.rgb * params.light_scale;
    }
#endif
    if (params.flags & FLAG_UNLIT) != 0u {
        light = vec3<f32>(1.0);
    }

    var alpha = params.color.a;
    if (params.flags & FLAG_IGNORE_TEX_ALPHA) == 0u {
        alpha = alpha * albedo.a;
    }
    if (params.flags & FLAG_ALPHA_TEST) != 0u && alpha < params.alpha_cutoff {
        discard;
    }
    // Projected lights add to the baked light (brushes only when the light
    // lights the world), scaled like it.
    var projected = vec3<f32>(0.0);
    if (params.flags & FLAG_UNLIT) == 0u {
        var n = normalize(in.world_normal);
        if !is_front {
            n = -n;
        }
        let brush = (params.flags & FLAG_LIGHTMAP) != 0u;
        let count = min(proj_lights.info.x, 8u);
        for (var i = 0u; i < count; i++) {
            let l = proj_lights.lights[i];
            if brush && l.params.z < 0.5 {
                continue;
            }
            projected += projected_light(l, in.world_position.xyz, n, proj_cookies, proj_cookie_samp, proj_shadows);
        }
    }
    var color = vec4<f32>(albedo.rgb * params.color.rgb * (light + projected * params.light_scale), alpha);
    if (params.flags & FLAG_LIGHT_ONLY) != 0u {
        color = vec4<f32>(light, 1.0);
    }
    // Fog, in-shader tonemapping (non-HDR cameras) and alpha premultiply,
    // exactly as StandardMaterial does.
    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.world_position = in.world_position;
    pbr_input.frag_coord = in.position;
    pbr_input.material.flags = params.pbr_flags;
    return main_pass_post_lighting_processing(pbr_input, color);
}
