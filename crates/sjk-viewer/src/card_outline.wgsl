// The player card's outline ring (card_outline.rs): every pixel outside the body's
// mask within `radius` pixels of it, its outer edge softened over a pixel.
struct Settings {
    color: vec4<f32>,
    // Output size in pixels, the radius in pixels, the lookups along one direction.
    shape: vec4<f32>,
};
@group(0) @binding(0) var mask: texture_2d<f32>;
@group(0) @binding(1) var mask_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: Settings;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

fn covered(pixel: vec2<f32>) -> bool {
    return textureSampleLevel(mask, mask_sampler, pixel / settings.shape.xy, 0.0).r > 0.5;
}

@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = position.xy;
    if covered(pixel) {
        discard;
    }
    let radius = settings.shape.z;
    let steps = max(i32(settings.shape.w), 1);
    // Walk out ring by ring; sixteen directions a ring.
    var nearest = radius + 2.0;
    for (var step = 1; step <= steps; step++) {
        let distance = radius * f32(step) / f32(steps);
        for (var i = 0; i < 16; i++) {
            let angle = f32(i) * 0.3926991;
            if covered(pixel + vec2<f32>(cos(angle), sin(angle)) * distance) {
                nearest = distance;
                break;
            }
        }
        if nearest <= radius {
            break;
        }
    }
    let alpha = 1.0 - smoothstep(radius - 1.0, radius + 0.5, nearest);
    if alpha <= 0.0 {
        discard;
    }
    return vec4<f32>(settings.color.rgb, settings.color.a * alpha);
}
