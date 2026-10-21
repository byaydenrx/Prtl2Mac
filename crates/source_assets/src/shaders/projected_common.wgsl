// Projected texture lights (env_projectedtexture), shared by the Valve
// light pass and the fallback SourceMaterial. Layout: `projected.rs`.
#define_import_path source_assets::projected

struct ProjLight {
    // world -> light clip (depth 0..1, w = distance along the light's forward)
    view_proj: mat4x4<f32>,
    // xyz world position, w far plane (metres)
    position: vec4<f32>,
    // rgb linear colour * intensity, w near plane (metres)
    color: vec4<f32>,
    // x shadow layer (-1 none), y cookie layer, z 1 = lights brushes, w metres per unit
    params: vec4<f32>,
};

struct ProjLights {
    // x = light count
    info: vec4<u32>,
    lights: array<ProjLight, 8>,
};

// Source flashlight attenuation: linear 100 (full strength within 100
// units), faded out between 60% of far and far. Mirrors
// `projected::attenuation`.
fn projected_attenuation(dist: f32, far: f32) -> f32 {
    let linear = clamp(100.0 / max(dist, 0.001), 0.0, 1.0);
    let end = clamp((dist - far) / (0.6 * far - far), 0.0, 1.0);
    return linear * end;
}

// Weight of texel column/row `k` (0..5 = offsets -2..3 from floor(p)) in a
// binomial (1 4 6 4 1) row of bilinear taps at fraction `f`; sums to 16.
fn projected_shadow_weight(k: i32, f: f32) -> f32 {
    var b = array<f32, 7>(0.0, 1.0, 4.0, 6.0, 4.0, 1.0, 0.0);
    return b[k + 1] * (1.0 - f) + b[k] * f;
}

// Soft shadow test like Source's flashlight 5x5 Gaussian PCF: a binomial
// (1 4 6 4 1) grid of bilinear comparisons one texel apart, folded into
// per-texel weights over the 6x6 texels it touches (36 loads, as many as
// the old 3x3), so edges fade over ~6 texels instead of ~4 (T-117).
fn projected_shadow(shadows: texture_2d_array<f32>, layer: i32, uv: vec2<f32>, depth: f32) -> f32 {
    let p = uv * vec2<f32>(textureDimensions(shadows)) - 0.5;
    let base = floor(p);
    let f = p - base;
    let b = vec2<i32>(base) - 2;
    let hi = vec2<i32>(textureDimensions(shadows)) - 1;
    var sum = 0.0;
    for (var y = 0; y < 6; y++) {
        var row = 0.0;
        for (var x = 0; x < 6; x++) {
            let t = clamp(b + vec2<i32>(x, y), vec2<i32>(0), hi);
            let lit = select(0.0, 1.0, depth <= textureLoad(shadows, t, layer, 0).r);
            row += projected_shadow_weight(x, f.x) * lit;
        }
        sum += projected_shadow_weight(y, f.y) * row;
    }
    return sum / 256.0;
}

// One light's colour reaching `world` (metres) with normal `n`; multiply
// by albedo (and the exposure) to light a surface.
fn projected_light(
    l: ProjLight,
    world: vec3<f32>,
    n: vec3<f32>,
    cookies: texture_2d_array<f32>,
    cookie_sampler: sampler,
    shadows: texture_2d_array<f32>,
) -> vec3<f32> {
    let clip = l.view_proj * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return vec3<f32>(0.0);
    }
    let ndc = clip.xyz / clip.w;
    if abs(ndc.x) > 1.0 || abs(ndc.y) > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return vec3<f32>(0.0);
    }
    let to_light = l.position.xyz - world;
    let dist = length(to_light);
    let ndl = max(dot(n, to_light / max(dist, 0.0001)), 0.0);
    if ndl <= 0.0 {
        return vec3<f32>(0.0);
    }
    let unit = l.params.w;
    let atten = projected_attenuation(dist / unit, l.position.w / unit);
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + 0.5;
    let cookie = textureSampleLevel(cookies, cookie_sampler, uv, i32(l.params.y), 0.0).rgb;
    var shadow = 1.0;
    if l.params.x >= 0.0 {
        // slope-scaled bias against acne on grazing surfaces
        let bias = 0.0015 + 0.004 * (1.0 - ndl);
        shadow = projected_shadow(shadows, i32(l.params.x), uv, clip.w / l.position.w - bias);
    }
    return l.color.rgb * cookie * (ndl * atten * shadow);
}
