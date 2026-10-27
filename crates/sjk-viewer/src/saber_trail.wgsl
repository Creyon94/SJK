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

// The scene behind the effects (`effect_layer.rs` `original`), for the heat wake: bound only
// for `fragment_wake`, drawn with the blades' heat haze before any other effect.
@group(2) @binding(0)
var wake_scene: texture_2d<f32>;
@group(2) @binding(1)
var wake_sampler: sampler;

// A vertex style's bit for a trail that bends what is behind it (`saber_trail::WAKE`), and
// the style itself in the low byte (`saber_trail::STYLE`).
const WAKE: u32 = 0x100u;
const STYLE: u32 = 0xffu;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // rgb the vertex colour; a the slice's age, 0 new to 1 gone (the plasma, flare and wake
    // read it; the stock blend ignores alpha).
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) style: u32,
    @location(3) world: vec3<f32>,
    // The flare's V at its slice's far edge, past the tip (`saber_trail::FLARE_REACH`).
    @location(4) @interpolate(flat) past: f32,
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) style: u32,
    @location(4) past: f32,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4(position, 1.0);
    // Stock fade (`CTrail::Update`, codemp/cgame/FxPrimitives.cpp:1771-1789):
    // `curST[0] = ST[0] * perc + (ST[0] + 1) * (1 - perc)`, clamped at 1.0,
    // computed per vertex before rasterization; `ST[1]` never moves and the
    // colour is left untouched. `color.a` carries `perc` (remaining life).
    let age = 1.0 - color.a;
    output.uv = vec2(min(uv.x + age, 1.0), uv.y);
    output.color = vec4(color.rgb, age);
    output.style = style;
    output.world = position;
    output.past = past;
    return output;
}

// Value noise in 3D, 0 to 1, smooth: the plasma's flames and the wake's shimmer, laid in the
// world so they run on unbroken from one slice to the next.
fn trail_hash(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3(0.1031, 0.1030, 0.0973));
    let r = q + dot(q, q.yxz + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn trail_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(trail_hash(i), trail_hash(i + vec3(1.0, 0.0, 0.0)), u.x);
    let b = mix(trail_hash(i + vec3(0.0, 1.0, 0.0)), trail_hash(i + vec3(1.0, 1.0, 0.0)), u.x);
    let c = mix(trail_hash(i + vec3(0.0, 0.0, 1.0)), trail_hash(i + vec3(1.0, 0.0, 1.0)), u.x);
    let d = mix(trail_hash(i + vec3(0.0, 1.0, 1.0)), trail_hash(i + vec3(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

// Two octaves, rising with the slice's age as heat does.
fn trail_flames(world: vec3<f32>, age: f32, scale: f32) -> f32 {
    let p = world * scale - vec3(0.0, 0.0, age * 2.4);
    return trail_noise(p) * 0.65 + trail_noise(p * 2.3 + vec3(5.2, 1.3, 7.7)) * 0.35;
}

// The plasma's colour as it cools (`fresh` 1 just left, 0 gone): white gold, the trail's own
// colour, then a deep red going dark. `tint` is the trail colour normalised to its brightest
// channel, so another blade's plasma would cool from its own colour.
fn plasma_colour(tint: vec3<f32>, fresh: f32) -> vec3<f32> {
    let white = vec3(1.0, 0.95, 0.8);
    let hot = mix(tint, white, 0.45);
    let deep = tint * vec3(0.75, 0.18, 0.1);
    let warm = mix(deep, tint, smoothstep(0.35, 0.75, fresh));
    return mix(warm, hot, smoothstep(0.82, 1.0, fresh));
}

// The plasma (style 5, the Sun's): white-hot where the blade just passed, cooling to deep red
// as the slice ages, brightest toward the tip, its trailing edge torn into rising flames.
fn plasma(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let flames = trail_flames(input.world, input.color.a, 0.11);
    // The edge the flames tear: further out where the noise is high, more ragged by the tip.
    let reach = 0.3 + 0.65 * flames * mix(1.0, 0.75, along);
    let body = smoothstep(reach - 0.2, reach + 0.04, fresh);
    // Strong toward the tip, fading out well before the hilt.
    let span = (1.0 - smoothstep(0.35, 0.85, along)) * (0.4 + 0.6 * (1.0 - along));
    // A white-hot seam where the tip just passed.
    let seam = exp(-along * along * 260.0) * smoothstep(0.7, 1.0, fresh);
    let level = body * span * (0.2 + 0.8 * fresh * fresh);
    return plasma_colour(tint, fresh) * level * 0.7 + vec3(1.0, 0.92, 0.75) * seam * 0.6;
}

// The flare (style 6): the plasma, and past the tip (V below 0, `saber_trail::FLARE_REACH`)
// flames and thin magnetic loops licking off it, as prominences off the Sun's limb.
fn flare(input: VertexOutput, tint: vec3<f32>, past: f32) -> vec3<f32> {
    if input.uv.y >= 0.0 {
        return plasma(input, tint);
    }
    // 0 at the tip to 1 at the slice's far edge.
    let up = clamp(input.uv.y / min(past, -0.0001), 0.0, 1.0);
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let tongues = trail_flames(input.world + vec3(13.0, 4.0, 9.0), input.color.a, 0.16);
    let lick = smoothstep(up - 0.12, up + 0.04, tongues * 1.15 - 0.12);
    // Thin loops where a second noise crosses its middle.
    let loops_noise = trail_flames(input.world * 0.6 + vec3(2.0, 31.0, 5.0), input.color.a * 0.5, 0.2);
    let loops = (1.0 - smoothstep(0.0, 0.045, abs(loops_noise - 0.5))) * (1.0 - up * 0.7);
    let level = (lick * (1.0 - up) * 0.9 + loops * 0.6) * smoothstep(0.0, 0.5, fresh);
    return plasma_colour(tint, fresh * (1.0 - 0.45 * up)) * level;
}

// Both stock materials blend GL_ONE GL_ONE, so only rgb matters here (style 3, a
// blade skin's blur trail, is SJK's own):
// `saberTrail` = blurglow (rgbGen vertex) + blurcore (rgbGen identity),
// `swordTrail` = swordtrail (rgbGen vertex).
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let style = input.style & STYLE;
    if style == 2u { discard; }
    if style == 1u {
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
    if style == 5u || style == 6u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 5u {
            return vec4(plasma(input, tint), 1.0);
        }
        return vec4(flare(input, tint, input.past), 1.0);
    }
    if style == 4u {
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
    if style == 3u {
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

// A heat wake (a slice with `WAKE`, the Sun's extra): what is behind the trail shimmers,
// the air bent by heat, strongest where the blade just passed and gone as the slice ages and
// at its edges, so the scene there is unchanged and the merge sees no seam. Drawn first in
// the effect pass with the blades' heat haze, replacing what it covers with the scene's
// pixels read at an offset (no blending), as `saber.wgsl`'s `fragment_haze` does.
@fragment
fn fragment_wake(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow): world units across a pixel.
    let dx = dpdx(input.world);
    let dy = dpdy(input.world);
    if (input.style & WAKE) == 0u { discard; }
    let size = vec2<f32>(textureDimensions(wake_scene));
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    // Gone at both edges along the blade and toward the slice's end.
    let fade = sin(along * 3.14159) * smoothstep(0.0, 0.45, fresh);
    let age = input.color.a;
    let p = input.world * 0.09 - vec3(0.0, 0.0, age * 6.0);
    let n = vec2(trail_noise(p), trail_noise(p + vec3(19.1, 7.3, 3.7))) - 0.5;
    // 2.5 units of bend at most, as pixels here, never more than 2 % of the screen
    // (`HAZE_REACH`, the room the effect rectangle keeps).
    let units = max(max(length(dx), length(dy)), 0.0001);
    var pixels = n * 2.0 * 2.5 * fade / units;
    let most = 0.02 * max(size.x, size.y);
    let len = length(pixels);
    if len > most {
        pixels *= most / len;
    }
    let uv = (input.clip_position.xy + pixels) / size;
    return vec4(textureSampleLevel(wake_scene, wake_sampler, uv, 0.0).rgb, 1.0);
}
