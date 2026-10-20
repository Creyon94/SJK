struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var blur_glow: texture_2d<f32>;
@group(1) @binding(1)
var blur_core: texture_2d<f32>;
@group(1) @binding(2)
var sword_trail: texture_2d<f32>;
@group(1) @binding(3)
var trail_sampler: sampler;
@group(1) @binding(4)
var black_trail: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) style: u32,
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) style: u32,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4(position, 1.0);
    // Stock fade (`CTrail::Update`, codemp/cgame/FxPrimitives.cpp:1771-1789):
    // `curST[0] = ST[0] * perc + (ST[0] + 1) * (1 - perc)`, clamped at 1.0,
    // computed per vertex before rasterization; `ST[1]` never moves and the
    // colour is left untouched. `color.a` carries `perc` (remaining life).
    let age = 1.0 - color.a;
    output.uv = vec2(min(uv.x + age, 1.0), uv.y);
    output.color = vec4(color.rgb, 1.0);
    output.style = style;
    return output;
}

// Both stock materials blend GL_ONE GL_ONE, so only rgb matters here (style 3, a
// blade skin's blur trail, is SJK's own):
// `saberTrail` = blurglow (rgbGen vertex) + blurcore (rgbGen identity),
// `swordTrail` = swordtrail (rgbGen vertex).
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.style == 2u { discard; }
    if input.style == 1u {
        let sword = textureSample(sword_trail, trail_sampler, input.uv);
        return vec4(sword.rgb * input.color.rgb, 1.0);
    }
    let glow = textureSample(blur_glow, trail_sampler, input.uv);
    let core = textureSample(blur_core, trail_sampler, input.uv);
    // A blade skin (style 3): glow and core both take the skin's trail colour, deepened
    // (normalised to its brightest channel, then squared) so a pale colour does not wash
    // out to white once the two add up.
    // The Clone Wars blade (style 4, Creyon's round 19): comic speed lines streaking back
    // along the swing instead of the blur, each in its own lane across the swept band with its
    // own length, the blade's colour round a white core, and a bold line where the tip went;
    // they draw back toward the blade as the trail fades.
    if input.style == 4u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        let lanes = 14.0;
        let across = input.uv.y * lanes;
        let lane = floor(across);
        let pick = fract(sin(lane * 12.9898 + 4.1) * 43758.547);
        let reach = 0.25 + 0.6 * fract(sin(lane * 78.233 + 1.7) * 43758.547);
        let shift = (fract(sin(lane * 39.346 + 8.3) * 43758.547) - 0.5) * 0.4;
        // more of them toward the tip, where the blade moves fastest; each tapering to a point
        let shown = step(0.25 + 0.5 * input.uv.y, pick);
        let half = (0.06 + 0.12 * pick) * (1.0 - 0.8 * clamp(input.uv.x / reach, 0.0, 1.0));
        let aa = max(fwidth(across), 0.0001);
        let line = (1.0 - smoothstep(half - aa, half + aa, abs(fract(across) - 0.5 - shift)))
            * shown * (1.0 - smoothstep(reach - 0.1, reach, input.uv.x));
        let aa_tip = max(fwidth(input.uv.y), 0.0001);
        let tip = (1.0 - smoothstep(0.035 - aa_tip, 0.035 + aa_tip, input.uv.y))
            * (1.0 - smoothstep(0.7, 0.95, input.uv.x));
        let lit = max(line, tip) * smoothstep(0.0, 0.02, input.uv.x);
        let core = max(line, tip) * 0.55;
        return vec4((tint * 0.9 * lit + vec3(core * lit)) * (1.0 - input.uv.x * 0.5), 1.0);
    }
    if input.style == 3u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4((glow.rgb + core.rgb * 0.6) * tint * tint, 1.0);
    }
    return vec4(glow.rgb * input.color.rgb + core.rgb, 1.0);
}

@fragment
fn fragment_black(input: VertexOutput) -> @location(0) vec4<f32> {
    // Explicit gradients keep sampling valid for mixed ordinary/black vertices.
    let dx = dpdx(input.uv);
    let dy = dpdy(input.uv);
    if input.style != 2u { discard; }
    let texel = textureSampleGrad(black_trail, trail_sampler, input.uv, dx, dy);
    // EJK's blacktrail shader draws the same alpha stage twice.
    return vec4(texel.rgb, 1.0 - (1.0 - texel.a) * (1.0 - texel.a));
}
