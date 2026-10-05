// The classic profile's model preview: the stage actor drawn into a target of
// its own in the scene's format, turned into the display values the UI draws
// (as `effect_layer.wgsl`'s `display` does, at a neutral exposure). Coverage
// comes from depth, so the background is transparent; an edge pixel averages
// its covered 3x3 neighbours and takes their share as alpha, which smooths the
// silhouette by a pixel.
//
// ENCODING: 0 the scene stores display values already (UNORM target), 1 it is
// an sRGB target sampled as linear, 2 it is a floating HDR scene.
override ENCODING: u32 = 2u;
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_depth_2d;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

// `post_hdr.wgsl`'s shoulder: identity below a peak of 0.9, one multiplier for RGB.
fn hdr_map(c: vec3<f32>) -> vec3<f32> {
    let peak = max(c.r, max(c.g, c.b));
    if peak <= 0.9 { return c; }
    let excess = peak - 0.9;
    let mapped = 0.9 + 0.1 * excess / (excess + 0.1);
    return c * (mapped / peak);
}

fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    return select(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c > vec3(0.0031308));
}

fn display(c: vec3<f32>) -> vec3<f32> {
    if ENCODING == 0u { return clamp(c, vec3(0.0), vec3(1.0)); }
    if ENCODING == 1u { return srgb_encode(clamp(c, vec3(0.0), vec3(1.0))); }
    return clamp(srgb_encode(hdr_map(max(c, vec3(0.0)))), vec3(0.0), vec3(1.0));
}

fn covered(at: vec2<i32>) -> bool {
    return textureLoad(depth, at, 0) < 1.0;
}

@fragment fn fs_main(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let at = vec2<i32>(p.xy);
    let size = vec2<i32>(textureDimensions(scene));
    if covered(at) {
        return vec4(display(textureLoad(scene, at, 0).rgb), 1.0);
    }
    var sum = vec3(0.0);
    var count = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let near = clamp(at + vec2(x, y), vec2(0), size - 1);
            if covered(near) {
                sum += textureLoad(scene, near, 0).rgb;
                count += 1.0;
            }
        }
    }
    if count == 0.0 { return vec4(0.0); }
    return vec4(display(sum / count), count / 9.0);
}
