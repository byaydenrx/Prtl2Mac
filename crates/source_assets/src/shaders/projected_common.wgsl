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

// Bilinear 2x2 comparison of `depth` (distance / far) at texel position `p`
// (texel units, already shifted by -0.5 so integers are texel centres).
fn projected_shadow_tap(shadows: texture_2d_array<f32>, layer: i32, p: vec2<f32>, depth: f32) -> f32 {
    let hi = vec2<i32>(textureDimensions(shadows)) - 1;
    let base = floor(p);
    let f = p - base;
    let b = vec2<i32>(base);
    let s00 = select(0.0, 1.0, depth <= textureLoad(shadows, clamp(b, vec2<i32>(0), hi), layer, 0).r);
    let s10 = select(0.0, 1.0, depth <= textureLoad(shadows, clamp(b + vec2<i32>(1, 0), vec2<i32>(0), hi), layer, 0).r);
    let s01 = select(0.0, 1.0, depth <= textureLoad(shadows, clamp(b + vec2<i32>(0, 1), vec2<i32>(0), hi), layer, 0).r);
    let s11 = select(0.0, 1.0, depth <= textureLoad(shadows, clamp(b + vec2<i32>(1, 1), vec2<i32>(0), hi), layer, 0).r);
    return mix(mix(s00, s10, f.x), mix(s01, s11, f.x), f.y);
}

// Soft shadow test like Source's flashlight PCF: a 3x3 Gaussian (1 2 1)
// of bilinear comparisons one texel apart, so edges fade over ~4 texels
// instead of stepping per texel (T-117).
fn projected_shadow(shadows: texture_2d_array<f32>, layer: i32, uv: vec2<f32>, depth: f32) -> f32 {
    let p = uv * vec2<f32>(textureDimensions(shadows)) - 0.5;
    var sum = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let w = f32((2 - abs(x)) * (2 - abs(y)));
            sum += w * projected_shadow_tap(shadows, layer, p + vec2<f32>(f32(x), f32(y)), depth);
        }
    }
    return sum / 16.0;
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
