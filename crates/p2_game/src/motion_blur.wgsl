// Source-style image-space motion blur: one blur vector for the screen
// (turning, falling level) plus a radial part (falling looking down/up).
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct ViewBlur {
    dir: vec2<f32>,
    radial: f32,
    _pad: f32,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> blur: ViewBlur;

const SAMPLES: i32 = 15;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let step = blur.dir + (in.uv - vec2(0.5)) * blur.radial;
    var sum = vec4(0.0);
    for (var i = 0; i < SAMPLES; i++) {
        let t = f32(i) / f32(SAMPLES - 1) - 0.5;
        sum += textureSampleLevel(screen, samp, clamp(in.uv + step * t, vec2(0.0), vec2(1.0)), 0.0);
    }
    return sum / f32(SAMPLES);
}
