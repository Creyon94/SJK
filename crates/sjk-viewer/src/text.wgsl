struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0)
var font_texture: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

// Glyphs beyond the font's 256 code-page slots (`text/wide.rs`): R8 coverage.
@group(0) @binding(2)
var wide_texture: texture_2d<f32>;

// A vertex whose u is at least this offset samples the wide atlas at u - offset
// (`text::wide::U_OFFSET`); every vertex of a glyph quad is on the same side.
const WIDE_U_OFFSET: f32 = 2.0;

fn wide_coverage(uv: vec2<f32>) -> f32 {
    return textureSample(wide_texture, font_sampler, uv - vec2(WIDE_U_OFFSET, 0.0)).r;
}

fn is_wide(uv: vec2<f32>) -> bool {
    return uv.x >= WIDE_U_OFFSET - 0.5;
}

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Both atlases are sampled in uniform control flow; the vertex's u picks one.
    let font = textureSample(font_texture, font_sampler, input.uv).a;
    let coverage = select(font, wide_coverage(input.uv), is_wide(input.uv));
    return vec4(input.color.rgb, input.color.a * coverage);
}

// Retail bitmap fonts are uploaded as signed distance fields (`text/sdf.rs`): alpha
// 0.5 on the glyph edge. The edge is rebuilt one screen pixel wide at any scale, so
// magnified game fonts stay sharp instead of showing the atlas's bilinear ramp.
@fragment
fn fragment_sdf(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = textureSample(font_texture, font_sampler, input.uv).a;
    // Change of the encoded distance across one screen pixel; the clamp keeps a
    // heavily minified glyph from washing out to half coverage.
    let pixel = clamp(length(vec2(dpdx(distance), dpdy(distance))), 1.0e-4, 0.5);
    let field = clamp((distance - 0.5) / pixel + 0.5, 0.0, 1.0);
    // Wide glyphs are coverage, not distance.
    let coverage = select(field, wide_coverage(input.uv), is_wide(input.uv));
    return vec4(input.color.rgb, input.color.a * coverage);
}
