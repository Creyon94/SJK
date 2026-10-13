// Saber blade: a smooth glow capsule and a flat hot core, blended GL_ONE GL_ONE into the
// legacy effect layer (display values, per-channel clamp; `effect_layer.rs`).
//
// Stock draws the glow as RB_SurfaceSaberGlow's chain of camera-facing sprites, one every
// 0.65·radius from the tip down, each 0.017 wider, plus a hilt sprite of radius 5.5-5.75
// (rd-vanilla tr_surface.cpp:470-490), and the core as RB_SurfaceLine's flat quad from the
// tip to one unit behind the hilt (tr_surface.cpp:511-566, CG_DoSaber cg_players.c:5359).
// SJK keeps one smooth capsule (owner decision, step 526i) with the same brightness: the
// chain's sum is integrated along the blade instead of summed per sprite, which is its
// continuous limit. For sprites at density 1/(0.65 r) along a blade whose projection is
// `shaft` long, a point at (x, y) receives
//     (L / (0.65 r)) · mean over v in [v_lo, v_hi] of glow(u, v),
//     u = 0.5 + x / 2r, v_hi = 0.5 + y / 2r, v_lo = 0.5 + (y - shaft) / 2r,
// and the mean comes from the glow image's column integral, `glow_integral`.
//
// Blade skins (saber_skins.rs) have their own grey glow/core pair and are coloured and
// animated here from the instance's time and seed and the skin's parameters, which come
// from its blade-skin file (blade_skin_file.rs) in the uniform array `skins`: a hot core
// with a fringe, a corona graded from its rim to the inside, granulation drifting along
// the blade, flame tongues licking outward, shimmer, and flares running from the hilt to
// the tip, swelling the glow; optionally lightning arcs, drifting motes and a turning hue,
// and since 10/10/2026 a sputtering edge and length, glitches, hologram scan lines, a
// heartbeat, falling embers, veins, a team or surroundings tint and the wearer's name in
// glyphs (afterimages are instances of their own); since 11/10/2026 a glint at the tip (an
// instance of its own, a quad round the tip), wisps of smoke rising off the blade, heat
// haze (`fragment_haze`, drawn first into the effect layer, bending the scene behind it)
// and an echo (an afterimage swaying beside the blade). The sections a wearer switched off
// (`off`, a bit each, blade_skin_extras.rs `Section`) are drawn as absent: `skin_at`
// zeroes their lanes, as a file without them has. A skin's blade ends round: past the tip the corona widens, grades from its inside to its
// rim and licks out about the tip, and the core line narrows to a rounded point over its
// last `tip` half-widths (the stock line ends flat, hidden under its glow). Nothing here
// knows any one skin. The dynamic glow pass shades the glow the same way, so its bloom
// follows the flares and arcs.
struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var glow_texture: texture_2d<f32>;
@group(1) @binding(1)
var core_texture: texture_2d<f32>;
@group(1) @binding(2)
var saber_sampler: sampler;
// Row j holds the glow image's mean of rows 0..j-1 per column; row 0 is zero.
@group(1) @binding(3)
var glow_integral: texture_2d<f32>;
// The core line is sampled as stock samples it: rd-vulkan's box mip chain, trilinear with its
// default 2x anisotropy (saber.rs `Samplers`). At normal distances the line is a few pixels
// wide for 64 texels, and the mip average widens the white core's fringe.
@group(1) @binding(4)
var core_sampler: sampler;

// One loaded skin's parameters (saber_skins.rs `SkinUniform`, blade_skin_file.rs for the
// meaning of each). `grain` is the granulation in [0, 1] and `flare` the flares' sum.
struct Skin {
    // Core: rgb the hot centre's colour, w how much a flare brightens it.
    core_white: vec4<f32>,
    // Fringe colours, cool and hot; w: fringe_heat base and grain.
    core_fringe_cool: vec4<f32>,
    core_fringe_hot: vec4<f32>,
    // Fringe brightness: base, grain, flare; w the rounded tip's length in half-widths.
    core_fringe: vec4<f32>,
    // The core's breathing across: amount, rate, along, seed.
    core_breathe: vec4<f32>,
    // Corona colours, rim cool and hot and the inside; w: inner_width, rim heat by
    // grain and by flare.
    rim_cool: vec4<f32>,
    rim_hot: vec4<f32>,
    inner: vec4<f32>,
    // The inside colour's share: base, grain, flare.
    inner_mix: vec4<f32>,
    // The glow's brightness: base, grain, flare.
    brightness: vec4<f32>,
    // Reach (widest, in stock capsules), swell by grain and by flare.
    swell: vec4<f32>,
    // Granulation octaves: scale, speed, evolve, offset.
    grain_coarse: vec4<f32>,
    grain_fine: vec4<f32>,
    // Octave weights, then the contrast's low and high.
    grain_mix: vec4<f32>,
    // Flares: rate, rate per track, rate per seed, track count.
    flare_rate: vec4<f32>,
    // Phase per seed and per track, the lit threshold, overshoot.
    flare_shape: vec4<f32>,
    // Size and its jitter.
    flare_size: vec4<f32>,
    // Shimmer waves: amount, rate, along, seed.
    shimmer_a: vec4<f32>,
    shimmer_b: vec4<f32>,
    // Tongues: along, offset, out, speed; then edge low and high, low, range.
    tongue_a: vec4<f32>,
    tongue_b: vec4<f32>,
    // Lightning arcs: colour and brightness; width, halo, jag, kinks; count, rate,
    // threshold, decay; reach, span low and high, tip share; jitter, crawl. No arcs: zeros.
    arc_color: vec4<f32>,
    arc_shape: vec4<f32>,
    arc_strike: vec4<f32>,
    arc_place: vec4<f32>,
    arc_motion: vec4<f32>,
    // Motes: colour and brightness; density, cells, rings, size; drift along and out,
    // twinkle, stretch; inner, outer, focus. No motes: zeros.
    mote_color: vec4<f32>,
    mote_field: vec4<f32>,
    mote_motion: vec4<f32>,
    mote_band: vec4<f32>,
    // Hue turning: turns a second, per unit along, per capsule radius out. None: zeros.
    hue: vec4<f32>,
    // The later sections (blade_skin_effects.rs); each zeros when absent.
    // Sputter: ragged, scale, speed; cut, rate, threshold.
    sputter_edge: vec4<f32>,
    sputter_cut: vec4<f32>,
    // Glitch: split, blocks, rate, threshold; shift, flash.
    glitch_a: vec4<f32>,
    glitch_b: vec4<f32>,
    // Scan (hologram): wireframe colour and brightness; lines, speed, depth, rings; edge,
    // width, hollow; jitter and its rate.
    scan_color: vec4<f32>,
    scan_lines: vec4<f32>,
    scan_edge: vec4<f32>,
    scan_jitter: vec4<f32>,
    // Pulse (heartbeat): rate, amount, second, gap; width, swell.
    pulse_a: vec4<f32>,
    pulse_b: vec4<f32>,
    // Embers: colour and brightness; density, cells, fall, life; size, spread.
    ember_color: vec4<f32>,
    ember_field: vec4<f32>,
    ember_shape: vec4<f32>,
    // Veins: colour and brightness; scale, speed, width, core.
    vein_color: vec4<f32>,
    vein_shape: vec4<f32>,
    // Team colours, red (w the amount), blue and none.
    team_red: vec4<f32>,
    team_blue: vec4<f32>,
    team_none: vec4<f32>,
    // The light where it is: amount, saturate, floor.
    ambient: vec4<f32>,
    // Glyphs: colour and brightness; size, spacing, speed, width.
    glyph_color: vec4<f32>,
    glyph_shape: vec4<f32>,
    // The third set (blade_skin_extras.rs); each zeros when absent.
    // Star (the tip's glint): colour and brightness; rays, length, width, halo; spin,
    // twinkle, depth, and the glints circling the blade (0: one at the tip). Their orbit:
    // how far below the tip (units), how far out (units), turns a second, tilt (radians).
    star_color: vec4<f32>,
    star_shape: vec4<f32>,
    star_motion: vec4<f32>,
    star_orbit: vec4<f32>,
    // Black holes in the glints' places: the ring's colour and brightness; the horizon's
    // radius (units), the lens (horizon radii), the swirl's turns a second, the quad's
    // reach (units).
    hole_color: vec4<f32>,
    hole_shape: vec4<f32>,
    // Wisps: colour and brightness; rise, speed, scale, curl; density.
    wisp_color: vec4<f32>,
    wisp_shape: vec4<f32>,
    wisp_field: vec4<f32>,
    // Heat haze: strength (units), scale, speed, reach (units past the glow).
    haze: vec4<f32>,
}

@group(2) @binding(0)
var<storage, read> skins: array<Skin, MAX_SKINS>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    // Across and along the projected blade, world units; along is 0 at the hilt.
    @location(1) blade: vec2<f32>,
    // The projected shaft's length; for a glint or hole, how hidden it is behind the blade.
    @location(2) @interpolate(flat) shaft: f32,
    @location(3) @interpolate(flat) radius: f32,
    @location(4) @interpolate(flat) length: f32,
    // Hilt sprite radius for the glow; zero selects the core line.
    @location(5) @interpolate(flat) hilt: f32,
    // KIND_RETAIL, KIND_NEUTRAL, or KIND_SKIN plus the skin's number.
    @location(6) @interpolate(flat) kind: u32,
    // Presentation seconds (wrapped) and the blade's seed in [0, 1).
    @location(7) @interpolate(flat) animation: vec2<f32>,
    // Who wears it (saber_persona.rs): the light where it is and the afterimage's
    // brightness; the team and the name's letters.
    @location(8) @interpolate(flat) persona: vec4<u32>,
    // The world's down in the projected blade's plane (across, along).
    @location(9) @interpolate(flat) down: vec2<f32>,
    // A chroma skin's turn to its wearer's colour (saber_skins.rs `Chroma`); 0 otherwise.
    @location(10) @interpolate(flat) chroma: f32,
    // The sections its wearer switched off (`OFF_*` bits); 0 otherwise.
    @location(11) @interpolate(flat) off: u32,
}

// TaystJK's CG_DoSaber submits the core RT_LINE twice (taystjk cgame cg_players.c:6429 and 6465;
// the second adds `saber` where `sbak` was meant). The owner's reference look is TaystJK's. Two
// GL_ONE GL_ONE adds with a clamp after each are one add of twice the colour.
const CORE_DRAWS: f32 = 2.0;
// Slot of the engine-generated neutral glow/core pair (saber_rgb.rs), and of the first
// loaded blade skin's (saber_rgb.rs `SKIN_MATERIAL`); skins loaded at once.
const NEUTRAL_MATERIAL: u32 = 6u;
const SKIN_MATERIAL: u32 = 7u;
const MAX_SKINS: u32 = 256u;
const KIND_RETAIL: u32 = 0u;
const KIND_NEUTRAL: u32 = 1u;
const KIND_SKIN: u32 = 2u;
// Colour alpha of a skin's first glint (saber.rs `STAR`), glint n's `STAR - n`; a glow's
// is its hilt sprite's radius, a core's zero.
const STAR: f32 = -1.0;
// How bright a glint shows behind the blade, where its glow covers it.
const GLINT_BEHIND: f32 = 0.3;
// The sections a wearer may switch off, a bit each (blade_skin_extras.rs `Section`).
const OFF_ARCS: u32 = 1u;
const OFF_MOTES: u32 = 2u;
const OFF_HUE: u32 = 4u;
const OFF_SPUTTER: u32 = 8u;
const OFF_GLITCH: u32 = 16u;
const OFF_SCAN: u32 = 32u;
const OFF_PULSE: u32 = 64u;
const OFF_EMBERS: u32 = 256u;
const OFF_VEINS: u32 = 512u;
const OFF_TEAM: u32 = 1024u;
const OFF_AMBIENT: u32 = 2048u;
const OFF_GLYPHS: u32 = 4096u;
const OFF_STAR: u32 = 8192u;
const OFF_WISPS: u32 = 16384u;
const OFF_HAZE: u32 = 32768u;
const OFF_HOLES: u32 = 131072u;
// RB_SurfaceSaberGlow: sprite spacing and per-sprite growth.
const SPACING: f32 = 0.65;
const GROWTH: f32 = 0.017;

const QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    vec2(-1.0, 0.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
);

// Sprite radius at fraction `along` of the blade (0 hilt, 1 tip): the chain starts at the
// tip with the drawn radius and every sprite toward the hilt is GROWTH wider.
fn chain_radius(radius: f32, length: f32, along: f32) -> f32 {
    return radius + GROWTH * (1.0 - along) * length / (SPACING * radius);
}

// One blade instance, as the vertex buffer holds it (saber.rs `Instance`).
struct BladeInstance {
    @location(0) base: vec3<f32>,
    @location(1) length: f32,
    @location(2) direction: vec3<f32>,
    @location(3) radius: f32,
    @location(4) color: vec4<f32>,
    @location(5) material: u32,
    @location(6) animation: vec2<f32>,
    @location(7) persona: vec4<u32>,
    @location(8) chroma: f32,
    @location(9) off: u32,
}

// How far (units) a skin's glow quad reaches past the glow for the sections its wearer
// left on: the wisps' rise and the haze's reach (blade_skin_extras.rs `room`).
fn skin_room(skin: Skin, off: u32) -> f32 {
    var room = 0.0;
    if (off & OFF_WISPS) == 0u && skin.wisp_color.w > 0.0 { room = skin.wisp_shape.x; }
    if (off & OFF_HAZE) == 0u && skin.haze.x > 0.0 { room = max(room, skin.haze.w); }
    return room;
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    return blade_vertex(vertex_index, blade);
}

// The haze's quads: a skin's glow quad when its haze is on, for the blade itself (not an
// afterimage); every other instance is folded away.
@vertex
fn vertex_haze(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    var hazy = output.kind >= KIND_SKIN && output.hilt > 0.0 && (blade.persona.x >> 24u) == 255u
        && (blade.off & OFF_HAZE) == 0u;
    if hazy {
        hazy = skins[skin_index(output)].haze.x > 0.0;
    }
    if !hazy {
        output.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
    }
    return output;
}

fn blade_vertex(vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    let blade_base = blade.base;
    let blade_length = blade.length;
    let blade_direction = blade.direction;
    let blade_radius = blade.radius;
    let blade_color = blade.color;
    let blade_material = blade.material;
    let blade_animation = blade.animation;
    let blade_persona = blade.persona;
    let blade_chroma = blade.chroma;
    let direction = normalize(blade_direction);
    let view_direction = normalize(camera.position - (blade_base + direction * blade_length * 0.5));
    var side = cross(direction, view_direction);
    let projected = length(side);
    if projected < 0.001 {
        // Pick an axis that cannot be parallel, including a blade aimed along Z.
        let axis = select(vec3(0.0,0.0,1.0),vec3(0.0,1.0,0.0),abs(view_direction.z) > 0.9);
        side = cross(axis,view_direction);
    }
    side = normalize(side);
    let up = normalize(cross(view_direction,side));
    let local = QUAD[vertex_index];
    let hilt = blade_color.a;
    var output: VertexOutput;
    output.kind = KIND_RETAIL;
    if blade_material == NEUTRAL_MATERIAL { output.kind = KIND_NEUTRAL; }
    var reach = 1.0;
    var room = 0.0;
    var star = 0.0;
    var glints = 0.0;
    var orbit = vec4(0.0);
    output.off = 0u;
    if blade_material >= SKIN_MATERIAL && blade_material < SKIN_MATERIAL + MAX_SKINS {
        output.kind = KIND_SKIN + (blade_material - SKIN_MATERIAL);
        output.off = blade.off;
        let skin = skins[blade_material - SKIN_MATERIAL];
        // A skin's glow reaches further, for its flares and shimmer, and its quad further
        // still for its wisps and haze.
        reach = skin.swell.x;
        room = skin_room(skin, blade.off);
        star = skin.star_shape.y;
        if skin.hole_shape.x > 0.0 { star = skin.hole_shape.w; }
        glints = skin.star_motion.w;
        orbit = skin.star_orbit;
    }
    output.animation = blade_animation;
    output.persona = blade_persona;
    output.chroma = blade_chroma;
    output.down = vec2(-side.z, -up.z);
    var world: vec3<f32>;
    if hilt <= STAR {
        // A skin's glint (or black hole): a square facing the camera, as wide as the rays
        // are long (the hole's reach), round the tip, or round its place on the orbit
        // below it; `blade` is the place on it from its heart, `shaft` how hidden it is.
        let n = STAR - hilt;
        let size = max(star, 0.001) * glint_scale(n);
        var heart = blade_base + direction * blade_length;
        output.shaft = 0.0;
        if glints > 0.0 {
            // Round the blade's axis, measured from the camera's side of it (the near side
            // at a quarter turn), so the circle reads as a ring however the blade is seen;
            // the tilt lifts the near side and drops the far one.
            let toward = normalize(cross(side, direction));
            let angle = TAU * (blade_animation.x * orbit.z + blade_animation.y + n / glints);
            let across = cos(angle);
            let near = sin(angle);
            heart += direction * (orbit.y * tan(orbit.w) * near - min(orbit.x, blade_length))
                + (side * across + toward * near) * orbit.y;
            // Behind the blade, and close enough to its axis for the glow to cover it.
            let behind = smoothstep(0.1, -0.5, near);
            let covered = 1.0 - smoothstep(0.6, 1.6, abs(across) * orbit.y / max(blade_radius, 0.001));
            output.shaft = behind * covered;
        }
        let corner = vec2(local.x, local.y * 2.0 - 1.0);
        world = heart + (side * corner.x + up * corner.y) * size;
        output.blade = corner * size;
    } else if hilt > 0.0 {
        // Wide enough for the grown hilt-end sprites and the hilt sprite itself, and the
        // skin's room.
        let extent = max(chain_radius(blade_radius, blade_length, 0.0) * reach, hilt) + room;
        let shaft = blade_length * projected;
        world = blade_base + direction * (local.y * blade_length)
            + (side * local.x + up * (local.y * 2.0 - 1.0)) * extent;
        output.blade = vec2(local.x * extent, local.y * (shaft + 2.0 * extent) - extent);
        output.shaft = shaft;
    } else {
        // RB_SurfaceLine: flat, from one unit behind the hilt to the tip.
        world = blade_base + direction * mix(-1.0, blade_length, local.y)
            + side * (local.x * blade_radius);
        output.blade = vec2(local.x * blade_radius, local.y);
        output.shaft = 0.0;
    }
    output.clip_position = camera.view_projection * vec4(world, 1.0);
    output.color = blade_color.rgb;
    output.radius = blade_radius;
    output.length = blade_length;
    output.hilt = hilt;
    return output;
}

fn glow(u: f32, v: f32) -> vec3<f32> {
    if u < 0.0 || u > 1.0 || v < 0.0 || v > 1.0 { return vec3(0.0); }
    return textureSampleLevel(glow_texture, saber_sampler, vec2(u, v), 0.0).rgb;
}

fn integral(u: f32, v: f32) -> vec3<f32> {
    let rows = f32(textureDimensions(glow_integral).y) - 1.0;
    let y = (clamp(v, 0.0, 1.0) * rows + 0.5) / (rows + 1.0);
    return textureSampleLevel(glow_integral, saber_sampler, vec2(u, y), 0.0).rgb;
}

fn glow_capsule(input: VertexOutput) -> vec3<f32> {
    let x = input.blade.x;
    let y = input.blade.y;
    let r = chain_radius(input.radius, input.length,
        clamp(y / max(input.shaft, 0.0001), 0.0, 1.0));
    var sum = vec3(0.0);
    let u = 0.5 + x / (2.0 * r);
    if u >= 0.0 && u <= 1.0 {
        let high = 0.5 + y / (2.0 * r);
        let low = high - input.shaft / (2.0 * r);
        let count = input.length / (SPACING * r);
        if high - low < 1.0 / 64.0 {
            // Seen almost end-on the sprites stack: `count` copies of one texel.
            sum = count * glow(u, 0.5 * (high + low));
        } else {
            sum = count * (integral(u, high) - integral(u, low)) / (high - low);
        }
    }
    // The hilt sprite, centred on the blade's base.
    let hilt = 0.5 + vec2(x, y) / (2.0 * input.hilt);
    return sum + glow(hilt.x, hilt.y);
}

// --- Blade skins ---------------------------------------------------------------------------

const PI: f32 = 3.14159265;
const TAU: f32 = 6.2831853;
// Most lightning arcs a skin may have (blade_skin_file.rs `MAX_ARCS`).
const MAX_ARCS: i32 = 4;

// PCG hash of a lattice point, in [0, 1].
fn skin_hash(cell: vec2<i32>) -> f32 {
    var v = bitcast<u32>(cell.x) * 747796405u + 2891336453u;
    v = v ^ (bitcast<u32>(cell.y) * 2654435761u);
    v = v * 747796405u + 2891336453u;
    let word = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    return f32((word >> 22u) ^ word) / 4294967295.0;
}

// Smooth value noise in [0, 1].
fn skin_noise(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = skin_hash(cell);
    let b = skin_hash(cell + vec2(1, 0));
    let c = skin_hash(cell + vec2(0, 1));
    let d = skin_hash(cell + vec2(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Piecewise-linear noise in [-1, 1]: straight runs between random corners, one a unit of
// `k`, on lattice row `row`.
fn skin_zigzag(k: f32, row: i32) -> f32 {
    let i = floor(k);
    let a = skin_hash(vec2(i32(i), row));
    let b = skin_hash(vec2(i32(i) + 1, row));
    return mix(a, b, k - i) * 2.0 - 1.0;
}

// The skin's number, below MAX_SKINS.
fn skin_index(input: VertexOutput) -> u32 {
    return min(input.kind - KIND_SKIN, MAX_SKINS - 1u);
}

// The skin this instance wears, with the sections its wearer switched off zeroed: drawn as
// absent, as a file without them is (every section draws nothing when its lanes are zeros).
fn skin_at(input: VertexOutput) -> Skin {
    var skin = skins[skin_index(input)];
    let off = input.off;
    if off == 0u {
        return skin;
    }
    if (off & OFF_ARCS) != 0u { skin.arc_strike = vec4(0.0); }
    if (off & OFF_MOTES) != 0u { skin.mote_field = vec4(0.0); }
    if (off & OFF_HUE) != 0u { skin.hue = vec4(0.0); }
    if (off & OFF_SPUTTER) != 0u {
        skin.sputter_edge = vec4(0.0);
        skin.sputter_cut = vec4(0.0);
    }
    if (off & OFF_GLITCH) != 0u { skin.glitch_a = vec4(0.0); }
    if (off & OFF_SCAN) != 0u {
        skin.scan_lines = vec4(0.0);
        skin.scan_jitter = vec4(0.0);
    }
    if (off & OFF_PULSE) != 0u { skin.pulse_a = vec4(0.0); }
    if (off & OFF_EMBERS) != 0u { skin.ember_field = vec4(0.0); }
    if (off & OFF_VEINS) != 0u {
        skin.vein_color = vec4(0.0);
        skin.vein_shape = vec4(0.0);
    }
    if (off & OFF_TEAM) != 0u { skin.team_red = vec4(0.0); }
    if (off & OFF_AMBIENT) != 0u { skin.ambient = vec4(0.0); }
    if (off & OFF_GLYPHS) != 0u {
        skin.glyph_color = vec4(0.0);
        skin.glyph_shape = vec4(0.0);
    }
    if (off & OFF_STAR) != 0u { skin.star_color = vec4(0.0); }
    if (off & OFF_WISPS) != 0u { skin.wisp_color = vec4(0.0); }
    if (off & OFF_HAZE) != 0u { skin.haze = vec4(0.0); }
    if (off & OFF_HOLES) != 0u {
        skin.hole_color = vec4(0.0);
        skin.hole_shape = vec4(0.0);
    }
    return skin;
}

// How far out the glow's own parts may reach (embers, arcs and motes fade out before it):
// the grown chain's widest at the reach, or the hilt sprite. The quad reaches further by
// the skin's room (vertex_main's `extent`), for its wisps and haze only, so switching
// those off changes nothing else.
fn skin_extent(skin: Skin, input: VertexOutput) -> f32 {
    return max(chain_radius(input.radius, input.length, 0.0) * skin.swell.x, input.hilt);
}

// Granulation at `along` world units from the hilt: two octaves drifting toward the tip and
// slowly changing as they go.
fn skin_granulation(skin: Skin, along: f32, time: f32, seed: f32) -> f32 {
    let c = skin.grain_coarse;
    let f = skin.grain_fine;
    let coarse = skin_noise(vec2(along * c.x - time * c.y, time * c.z + seed * c.w));
    let fine = skin_noise(vec2(along * f.x - time * f.y, time * f.z + seed * f.w));
    let mixed = skin.grain_mix;
    return smoothstep(mixed.z, mixed.w, coarse * mixed.x + fine * mixed.y);
}

// Flares at `along`: tracks each sending a bright knot from the hilt to the tip once a cycle,
// lit on the cycles whose draw reaches the threshold, so they come now and then and never in
// step.
fn skin_flares(skin: Skin, along: f32, length: f32, time: f32, seed: f32) -> f32 {
    let rates = skin.flare_rate;
    let shape = skin.flare_shape;
    let size = skin.flare_size;
    var total = 0.0;
    for (var track = 0; track < i32(rates.w); track++) {
        let rate = rates.x + rates.y * f32(track) + rates.z * seed;
        let phase = time * rate + seed * shape.x + f32(track) * shape.y;
        let cycle = floor(phase);
        let travel = fract(phase);
        let lit = step(shape.z, skin_hash(vec2(i32(cycle), track * 7919 + i32(seed * 4096.0))));
        let position = travel * (length + 2.0 * shape.w) - shape.w;
        let width = size.x + size.y * skin_hash(vec2(i32(cycle), track + 31));
        let offset = (along - position) / width;
        total += lit * sin(PI * travel) * exp(-offset * offset);
    }
    return total;
}

// One wave of shimmer or breathing: amount · sin(time · rate + along · along + seed · seed).
fn skin_wave(wave: vec4<f32>, time: f32, along: f32, seed: f32) -> f32 {
    return wave.x * sin(time * wave.y + along * wave.z + seed * wave.w);
}

// Along the blade from the hilt, world units: the glow's projected `y`, or the core line's
// texture `v` (one unit behind the hilt to the tip).
fn skin_along(input: VertexOutput) -> f32 {
    if input.hilt > 0.0 {
        return clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0) * input.length;
    }
    return mix(-1.0, input.length, input.blade.y);
}

// How much of the core line's width is left `along` units from the hilt: all of it until the
// last `cap` units, then a quarter circle down to nothing at the tip, so the line ends in a
// rounded point instead of a square.
fn skin_tip_taper(along: f32, length: f32, cap: f32) -> f32 {
    let into = clamp((along - (length - cap)) / max(cap, 0.0001), 0.0, 1.0);
    // Exactly 1 along the shaft, whatever the GPU's square root.
    return select(sqrt(max(1.0 - into * into, 0.0001)), 1.0, into <= 0.0);
}

// Lightning arcs at `along` (world units from the hilt, running on past the tip) and `x`
// (signed across): `count` arcs, each struck anew `rate` times a second at a random place
// (strikes whose draw is under `threshold` stay dark), a jagged filament leaving the blade,
// bulging out to one side up to `reach` capsule radii (`r`) and coming back a `span` of the
// blade further on, crawling `crawl` units a second; a share `tip` of the strikes leap from
// just below the tip into the air (up to `room` units past it) and die out there. A strike
// flashes and fades by `decay`, re-shaped `jitter` times a second (the crackle). A filament
// thinner than a pixel widens to it and dims, so it does not break up at a distance.
fn skin_arcs(
    skin: Skin, along: f32, x: f32, r: f32, length: f32, room: f32, pixel: f32, time: f32,
    seed: f32,
) -> f32 {
    let strike = skin.arc_strike;
    let place = skin.arc_place;
    let shape = skin.arc_shape;
    let motion = skin.arc_motion;
    let seeded = i32(seed * 4096.0);
    let width = max(shape.x, pixel);
    var total = 0.0;
    for (var arc = 0; arc < min(i32(strike.x), MAX_ARCS); arc++) {
        let phase = time * strike.y + seed * 3.1 + f32(arc) * 0.618;
        let cycle = i32(floor(phase));
        let age = fract(phase);
        let row = arc * 7919 + seeded;
        if skin_hash(vec2(cycle, row)) < strike.z {
            continue;
        }
        let pick = skin_hash(vec2(cycle, row + 101));
        let side = select(-1.0, 1.0, skin_hash(vec2(cycle, row + 303)) < 0.5);
        var t: f32;
        var bulge: f32;
        var fade = 1.0;
        if skin_hash(vec2(cycle, row + 404)) < place.w {
            // From just below the tip into the air past it, bending away and dying out.
            let span = (0.8 + 0.6 * pick) * room;
            t = (along - (length - 0.35 * span)) / span;
            bulge = sin(0.5 * PI * clamp(t, 0.0, 1.0));
            fade = 1.0 - clamp(t, 0.0, 1.0);
        } else {
            let span = mix(place.y, place.z, pick) * length;
            let start = skin_hash(vec2(cycle, row + 202)) * (length - span)
                + motion.y * age / max(strike.y, 0.0001);
            t = (along - start) / span;
            bulge = sin(PI * clamp(t, 0.0, 1.0));
        }
        if t < 0.0 || t > 1.0 {
            continue;
        }
        let jagged = i32(floor(time * motion.x + f32(arc) * 0.37)) * 13 + arc * 131 + seeded;
        let k = along * shape.w + f32(arc) * 17.0;
        let jag = skin_zigzag(k, jagged) + 0.45 * skin_zigzag(k * 2.7 + 5.0, jagged + 57);
        let offset = side * bulge * place.x * r + jag * shape.z * r * sqrt(bulge);
        let d = (x - offset) / width;
        let filament = shape.x / width * exp(-d * d) + shape.y * 0.35 * exp(-d * d / 16.0);
        total += pow(1.0 - age, strike.w) * fade * filament;
    }
    return total;
}

// Motes at `around` (world units along the blade and on round its tip) and `out` (capsule
// radii from the blade's axis, or from the tip past it), on the `left` or right: a field of
// specks in cells, `cells` a unit along by `rings` a radius out, a share `density` of them
// holding one, `size` of its cell across and `stretch` times that along. The field drifts
// `drift_along` units a second toward the tip and `drift_out` radii a second outward
// (negative: inward); each speck twinkles about `twinkle` times a second; they show between
// `inner` and `outer` radii out, by `focus` only toward the tip, and not behind the hilt
// (`along`, unclamped).
fn skin_motes(
    skin: Skin, around: f32, out: f32, left: bool, along: f32, length: f32, time: f32,
    seed: f32,
) -> f32 {
    let field = skin.mote_field;
    let motion = skin.mote_motion;
    let band = skin.mote_band;
    let p_along = (around - time * motion.x) * field.y;
    let column = i32(floor(p_along));
    // Each column of cells is staggered outward by its own amount, so they do not line up.
    let p_out = (out - time * motion.y) * field.z + skin_hash(vec2(column, 4242));
    let cell = vec2(column, i32(floor(p_out)) + i32(seed * 4096.0) * 3 + select(0, 7777, left));
    if skin_hash(cell) >= field.x {
        return 0.0;
    }
    let half = vec2(field.w * motion.w, field.w);
    let centre = half + (1.0 - 2.0 * half)
        * vec2(skin_hash(cell + vec2(0, 911)), skin_hash(cell + vec2(37, 0)));
    let d = (vec2(fract(p_along), fract(p_out)) - centre) / half;
    let falloff = max(1.0 - dot(d, d), 0.0);
    var twinkle = 1.0;
    if motion.z > 0.0 {
        let rate = motion.z * (0.5 + skin_hash(cell + vec2(3, 5)));
        twinkle = 0.5 + 0.5 * sin(TAU * (time * rate + skin_hash(cell + vec2(7, 2))));
    }
    let span = band.y - band.x;
    let shown = smoothstep(band.x, band.x + 0.25 * span, out)
        * (1.0 - smoothstep(band.y - 0.4 * span, band.y, out));
    let toward_tip = mix(1.0, smoothstep(0.6 * length, length, around), band.z);
    return falloff * falloff * twinkle * shown * toward_tip * smoothstep(0.0, 2.0, along);
}

// `color` turned `turns` round the grey axis (a hue shift keeping its brightness and
// saturation), negative channels clipped: the blend adds.
fn skin_turn_hue(color: vec3<f32>, turns: f32) -> vec3<f32> {
    let axis = vec3(0.57735027);
    let c = cos(TAU * turns);
    let s = sin(TAU * turns);
    let turned = color * c + cross(axis, color) * s + axis * dot(axis, color) * (1.0 - c);
    return max(turned, vec3(0.0));
}

// --- Later effects (blade_skin_effects.rs): each draws nothing when its lanes are zeros ---

// Where the first heartbeat peaks in a cycle, seconds (saber_skins.rs `Beat::FIRST`).
const BEAT_FIRST: f32 = 0.1;

// The afterimage's brightness from the instance (1 for a blade itself).
fn persona_fade(input: VertexOutput) -> f32 {
    return f32(input.persona.x >> 24u) / 255.0;
}

// The light where the blade is, at full brightness (saber_persona.rs `pack_light`).
fn persona_light(input: VertexOutput) -> vec3<f32> {
    let packed = input.persona.x;
    return vec3(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u))
        / 255.0;
}

// The wearer's letter `index` (5 bits; 0 ends the name).
fn persona_letter(input: VertexOutput, index: u32) -> u32 {
    let lanes = vec3(input.persona.y & 0x3fffffffu, input.persona.z, input.persona.w);
    return (lanes[index / 6u] >> (5u * (index % 6u))) & 31u;
}

// The heartbeat at `time`: two Gaussian beats a cycle, on the clock alone so the blade
// and its light (saber_skins.rs `Beat::at`) beat together.
fn skin_beat(skin: Skin, time: f32) -> f32 {
    let p = skin.pulse_a;
    if p.x <= 0.0 {
        return 0.0;
    }
    let into = fract(time * p.x) / p.x;
    let width = max(skin.pulse_b.x, 0.001);
    let a = (into - BEAT_FIRST) / width;
    let b = (into - BEAT_FIRST - p.w) / width;
    return p.y * (exp(-a * a) + p.z * exp(-b * b));
}

// How long the blade shows this moment: all of it, or, on a sputtering draw, cut short.
fn skin_cut(skin: Skin, length: f32, time: f32, seed: f32) -> f32 {
    let c = skin.sputter_cut;
    if c.x <= 0.0 || c.y <= 0.0 {
        return length;
    }
    let cycle = i32(floor(time * c.y));
    let seeded = i32(seed * 4096.0);
    if skin_hash(vec2(cycle, 5151 + seeded)) < c.z {
        return length;
    }
    return length * (1.0 - c.x * skin_hash(vec2(cycle, 6161 + seeded)));
}

// Glitch at `along` units: x the jump (capsule radii), y the colour split (radii), z the
// flash. A block glitches on its own draws; now and then the whole blade does.
fn skin_glitch(skin: Skin, along: f32, time: f32, seed: f32) -> vec3<f32> {
    let g = skin.glitch_a;
    if g.y <= 0.0 {
        return vec3(0.0);
    }
    let frame = i32(floor(time * g.z));
    let seeded = i32(seed * 4096.0);
    let block = i32(floor(along / g.y));
    let whole = skin_hash(vec2(frame, 919 + seeded)) >= 0.5 + 0.5 * g.w;
    let draw = skin_hash(vec2(block * 31 + seeded, frame));
    if !whole && draw < g.w {
        return vec3(0.0, g.x, 0.0);
    }
    let jump = skin.glitch_b.x * (skin_hash(vec2(block + 77, frame * 3 + seeded)) * 2.0 - 1.0);
    return vec3(select(jump, jump * 0.5, whole), 2.0 * g.x, skin.glitch_b.y);
}

// The hologram's jitter: the whole projection thrown sideways (capsule radii) on some draws.
fn skin_jitter(skin: Skin, time: f32, seed: f32) -> f32 {
    let j = skin.scan_jitter;
    if j.x <= 0.0 || j.y <= 0.0 {
        return 0.0;
    }
    let frame = i32(floor(time * j.y));
    let seeded = i32(seed * 4096.0);
    if skin_hash(vec2(frame, 4321 + seeded)) < 0.75 {
        return 0.0;
    }
    return j.x * (skin_hash(vec2(frame, 8765 + seeded)) * 2.0 - 1.0);
}

// Veins: ridged noise along and across (`x` units from the axis), bright where it crosses
// its middle, a network of cracks running along the blade.
fn skin_veins(skin: Skin, along: f32, x: f32, time: f32, seed: f32) -> f32 {
    let v = skin.vein_shape;
    if v.x <= 0.0 {
        return 0.0;
    }
    let p = vec2(along * v.x - time * v.y + seed * 31.0, x * v.x * 1.7 + seed * 17.0);
    let n = 0.65 * skin_noise(p) + 0.35 * skin_noise(p * 2.3 + vec2(7.0, 3.0));
    return 1.0 - smoothstep(0.5 * v.z, v.z, abs(n - 0.5));
}

// The colour a skin takes from its wearer's team or the light where it is: rgb (largest
// channel 1) and how much; none, zero.
fn skin_tint(skin: Skin, input: VertexOutput) -> vec4<f32> {
    if skin.team_red.w > 0.0 {
        let team = input.persona.y >> 30u;
        var color = skin.team_none.rgb;
        if team == 1u { color = skin.team_red.rgb; }
        if team == 2u { color = skin.team_blue.rgb; }
        return vec4(color, skin.team_red.w);
    }
    let a = skin.ambient;
    if a.x > 0.0 {
        let light = persona_light(input);
        let grey = vec3(dot(light, vec3(0.299, 0.587, 0.114)));
        let color = max(mix(grey, light, a.y), vec3(a.z));
        return vec4(color / max(max(color.r, color.g), max(color.b, 0.0001)), a.x);
    }
    return vec4(0.0);
}

// `color` recoloured toward `tint` at its own strongest channel's level.
fn skin_apply_tint(color: vec3<f32>, tint: vec4<f32>) -> vec3<f32> {
    if tint.w <= 0.0 {
        return color;
    }
    let level = max(color.r, max(color.g, color.b));
    return mix(color, level * tint.rgb, tint.w);
}

// Embers dripping from the blade and falling, at `p` (across, along the projected blade,
// units) with `down` the world's down in that plane and `scale` projected units a world
// unit along. Each drip place `cells` a unit along drips (if its draw is under `density`)
// one ember a `life`, accelerating `fall` units a second; where gravity runs along the
// blade, they are pushed off to either side. A fragment reads three places a side.
fn skin_embers(
    skin: Skin, p: vec2<f32>, down: vec2<f32>, scale: f32, blade_length: f32, time: f32,
    seed: f32, pixel: f32,
) -> f32 {
    let f = skin.ember_field;
    let shape = skin.ember_shape;
    let seeded = i32(seed * 4096.0);
    var total = 0.0;
    for (var s = 0; s < 2; s++) {
        let side = select(-1.0, 1.0, s == 1);
        var d = down;
        if abs(d.x) < 0.35 {
            d.x = side * 0.35;
        } else if sign(d.x) != side {
            continue;
        }
        d = normalize(d);
        if p.x * side < 0.0 {
            continue;
        }
        let travel_here = p.x / d.x;
        let start = (p.y - travel_here * d.y) / max(scale, 0.0001);
        let k = i32(floor(start * f.y));
        for (var dk = -1; dk <= 1; dk++) {
            let column = k + dk;
            let row = column * 2 + s + seeded * 5;
            if skin_hash(vec2(row, 1717)) >= f.x {
                continue;
            }
            let origin = (f32(column) + skin_hash(vec2(row, 2727))) / f.y;
            if origin < 0.5 || origin > blade_length {
                continue;
            }
            let phase = fract(time / f.w + skin_hash(vec2(row, 3737)));
            let travel = f.z * f.w * phase * phase;
            let stray = shape.y * (skin_hash(vec2(row, 4747)) * 2.0 - 1.0);
            let q = vec2(0.0, origin * scale) + d * travel + vec2(-d.y, d.x) * stray * travel * 0.3;
            let size = shape.x * (1.0 - 0.5 * phase);
            let width = max(size, pixel);
            let e = length(p - q) / width;
            total += exp(-e * e) * pow(1.0 - phase, 1.5) * size / width;
        }
    }
    return total;
}

// SJK's stroke glyphs: sixteen strokes on a 3 × 3 grid (six across, six up, four
// diagonals from the middle), each letter a fixed handful of them drawn from its code.
const GLYPH_STROKES: array<vec4<f32>, 16> = array<vec4<f32>, 16>(
    vec4(0.0, 0.0, 1.0, 0.0), vec4(1.0, 0.0, 2.0, 0.0),
    vec4(0.0, 1.0, 1.0, 1.0), vec4(1.0, 1.0, 2.0, 1.0),
    vec4(0.0, 2.0, 1.0, 2.0), vec4(1.0, 2.0, 2.0, 2.0),
    vec4(0.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 0.0, 2.0),
    vec4(1.0, 0.0, 1.0, 1.0), vec4(1.0, 1.0, 1.0, 2.0),
    vec4(2.0, 0.0, 2.0, 1.0), vec4(2.0, 1.0, 2.0, 2.0),
    vec4(1.0, 1.0, 0.0, 0.0), vec4(1.0, 1.0, 2.0, 0.0),
    vec4(1.0, 1.0, 0.0, 2.0), vec4(1.0, 1.0, 2.0, 2.0),
);

// The strokes letter `code` draws: three to seven of the sixteen, the same every time
// (saber_persona.rs's tests pin that every letter differs).
fn glyph_bits(code: u32) -> u32 {
    var v = code * 747796405u + 2891336453u;
    v = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    let h = (v >> 22u) ^ v;
    var w = (code + 101u) * 2654435761u;
    w = w ^ (w >> 15u);
    w = w * 2246822519u;
    w = w ^ (w >> 13u);
    var bits = (h & w) & 0xffffu;
    bits = bits | (1u << (w % 16u)) | (1u << ((w >> 8u) % 16u)) | (1u << ((h >> 4u) % 16u));
    // At most seven strokes: drop the lowest while there are more.
    for (var i = 0; i < 16 && countOneBits(bits) > 7u; i++) {
        bits = bits & (bits - 1u);
    }
    return bits;
}

// Letter `code`'s strokes at `uv` (0 to 1 across and along the glyph), `width` a stroke's
// half-width and `soft` a pixel, both in glyph units.
fn glyph(code: u32, uv: vec2<f32>, width: f32, soft: f32) -> f32 {
    let bits = glyph_bits(code);
    var nearest = 4.0;
    for (var i = 0u; i < 16u; i++) {
        if (bits & (1u << i)) == 0u {
            continue;
        }
        let stroke = GLYPH_STROKES[i];
        let a = vec2(0.12, 0.08) + stroke.xy * vec2(0.38, 0.42);
        let b = vec2(0.12, 0.08) + stroke.zw * vec2(0.38, 0.42);
        let ab = b - a;
        let t = clamp(dot(uv - a, ab) / dot(ab, ab), 0.0, 1.0);
        nearest = min(nearest, length(uv - a - ab * t));
    }
    let half = max(width, soft);
    return (1.0 - smoothstep(half - soft, half + soft, nearest)) * width / half;
}

// Glyphs at `along` units from the hilt and `x` across: the wearer's name, a letter a cell
// of `size` along plus `spacing`, scrolling toward the tip, a cell left empty between
// repeats; only along the blade, not round its tip.
fn skin_glyphs(
    skin: Skin, input: VertexOutput, along: f32, x: f32, time: f32, pixel: f32,
) -> f32 {
    let g = skin.glyph_shape;
    let half_width = g.x * 0.3;
    if g.x <= 0.0 || abs(x) > half_width + pixel || along < 1.0 || along > input.length - 1.0 {
        return 0.0;
    }
    var count = 0u;
    for (var i = 0u; i < 18u; i++) {
        if persona_letter(input, i) == 0u {
            break;
        }
        count = i + 1u;
    }
    let cell = g.x + g.y;
    let k = (along - time * g.z) / cell;
    let v = fract(k) * cell / g.x;
    if v > 1.0 || count == 0u {
        return 0.0;
    }
    let index = i32(floor(k));
    let slot = u32(((index % i32(count + 1u)) + i32(count + 1u)) % i32(count + 1u));
    if slot == count {
        return 0.0;
    }
    let uv = vec2(x / (2.0 * half_width) + 0.5, v);
    let ends = smoothstep(1.0, 3.0, along) * (1.0 - smoothstep(input.length - 3.0, input.length - 1.0, along));
    return glyph(persona_letter(input, slot), uv, g.w * 0.5, pixel / g.x) * ends;
}

// --- The third set (blade_skin_extras.rs): each draws nothing when its lanes are zeros ---

// The world's up in the projected blade's plane (across, along), from `down`; seen from
// straight above or below, where it has no length, the blade's own direction.
fn skin_up(down: vec2<f32>) -> vec2<f32> {
    let size = length(down);
    return select(vec2(0.0, 1.0), -down / max(size, 0.0001), size > 0.001);
}

// Rising noise at `p` (across, along the projected blade, units) for a skin's wisps and
// haze: two octaves of value noise in the plane turned so `up` runs up it, rising `speed`
// units a second and warped by `curl` (the smoke curling as it goes).
fn skin_rising(
    p: vec2<f32>, up: vec2<f32>, scale: f32, speed: f32, curl: f32, time: f32, seed: f32,
) -> f32 {
    let q = vec2(dot(p, vec2(up.y, -up.x)), dot(p, up) - time * speed) * scale
        + vec2(seed * 37.0, seed * 11.0);
    let warp = vec2(skin_noise(q * 0.7 + vec2(3.1, time * 0.21)),
        skin_noise(q * 0.7 + vec2(7.7, -time * 0.17))) - 0.5;
    let w = q + curl * 2.0 * warp;
    return 0.65 * skin_noise(w) + 0.35 * skin_noise(w * 2.3 + vec2(5.2, 1.3));
}

// Where a point `p` is from the blade (the segment from the hilt to `shaft` along it):
// x how high it is above the nearest point of the blade, along `up`, y how far aside.
fn skin_above(p: vec2<f32>, shaft: f32, up: vec2<f32>) -> vec2<f32> {
    let v = p - vec2(0.0, clamp(p.y, 0.0, shaft));
    return vec2(dot(v, up), abs(dot(v, vec2(up.y, -up.x))));
}

// Wisps at `p`: smoke curling off the blade and rising up to `rise` units above it, a
// sheet over a blade held level, a plume over the tip of one held upright, thin along its
// sides; `r` the glow's radius there.
fn skin_wisps(skin: Skin, input: VertexOutput, r: f32, time: f32, seed: f32) -> f32 {
    let w = skin.wisp_shape;
    let up = skin_up(input.down);
    let at = skin_above(input.blade, input.shaft, up);
    let rise = max(w.x, 0.001);
    let risen = clamp(at.x / rise, 0.0, 1.0);
    // Mostly above the blade, wider as it rises, gone at the top.
    let width = r * (0.7 + 1.6 * risen);
    let column = exp(-(at.y * at.y) / (width * width));
    let shown = smoothstep(-0.6 * r, 0.4 * r, at.x) * (1.0 - smoothstep(0.55 * rise, rise, at.x));
    let n = skin_rising(input.blade, up, w.z, w.y, w.w, time, seed);
    let density = skin.wisp_field.x;
    let smoke = smoothstep(1.0 - density, 1.0 - density + 0.3, n);
    return smoke * column * shown * pow(1.0 - risen, 1.5);
}

// The tip's glint at `p` (units from the tip, facing the camera): `rays` rays turning
// `spin` turns a second, every other one shorter, tapering to their ends, and a round glow
// at the heart, twinkling `twinkle` times a second by `depth`.
fn skin_star(skin: Skin, p: vec2<f32>, time: f32, seed: f32, pixel: f32) -> f32 {
    let shape = skin.star_shape;
    let motion = skin.star_motion;
    let size = max(shape.y, 0.001);
    let rays = max(shape.x, 2.0);
    let r = length(p);
    let turn = atan2(p.y, p.x) / TAU + time * motion.x + seed;
    let k = turn * rays;
    let nearest = round(k);
    let reach = select(size, 0.55 * size, (i32(nearest) & 1) == 1 && rays >= 4.0);
    let aside = r * sin(abs(k - nearest) * TAU / rays);
    let along = clamp(1.0 - r / reach, 0.0, 1.0);
    let width = max(shape.z * reach * along, pixel);
    let ray = exp(-(aside * aside) / (width * width)) * along * along
        * (shape.z * reach * along + 0.0001) / width;
    let heart = shape.w * exp(-(r * r) / (0.03 * size * size));
    let twinkle = 1.0 - motion.z * (0.5 + 0.25 * sin(TAU * time * motion.y + seed * 13.0)
        + 0.25 * sin(TAU * time * motion.y * 1.618 + seed * 29.0));
    return (ray + heart) * twinkle;
}

// The glow, `pixel` world units across a screen pixel: the glow at `x` across (the glitch
// and the hologram's jitter move it, and split its colours), then what stays where the
// blade is: embers, glyphs, the sputtering cut and an afterimage's fading.
fn skin_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let glitch = skin_glitch(skin, along, time, seed);
    let x = input.blade.x - (glitch.x + skin_jitter(skin, time, seed)) * r;
    var glow: vec3<f32>;
    if glitch.y > 0.0 {
        let split = glitch.y * r;
        glow = vec3(
            skin_glow_at(input, pixel, x + split).r,
            skin_glow_at(input, pixel, x).g,
            skin_glow_at(input, pixel, x - split).b,
        ) * (1.0 + glitch.z);
    } else {
        glow = skin_glow_at(input, pixel, x);
    }
    let extent = skin_extent(skin, input);
    let beyond = max(input.blade.y - input.shaft, 0.0);
    let room = (1.0 - smoothstep(0.75, 1.0, abs(input.blade.x) / extent))
        * (1.0 - smoothstep(0.75, 1.0, beyond / extent))
        * (1.0 - smoothstep(0.75, 1.0, -min(input.blade.y, 0.0) / extent));
    if skin.ember_field.x > 0.0 {
        let scale = input.shaft / max(input.length, 0.0001);
        let embers = skin_embers(skin, input.blade, input.down, scale, input.length, time, seed,
            pixel);
        glow += embers * room * skin.ember_color.rgb * skin.ember_color.w;
    }
    if skin.glyph_color.w > 0.0 {
        glow += skin_glyphs(skin, input, along, x, time, pixel) * skin.glyph_color.rgb
            * skin.glyph_color.w;
    }
    if skin.wisp_color.w > 0.0 {
        var smoke = skin_wisps(skin, input, r, time, seed) * skin.wisp_color.rgb
            * skin.wisp_color.w;
        smoke = skin_apply_tint(smoke, skin_tint(skin, input));
        if input.chroma != 0.0 {
            smoke = skin_turn_hue(smoke, input.chroma);
        }
        glow += smoke;
    }
    let cut = skin_cut(skin, input.length, time, seed);
    if cut < input.length {
        // Past the cut nothing shows, the embers already falling aside.
        let reach = select(along, input.length + beyond, beyond > 0.0);
        glow *= 1.0 - smoothstep(cut - 1.5, cut + 0.5, reach);
    }
    return glow * persona_fade(input);
}

// The glow at `x` across (the fragment's own, or moved by a glitch).
fn skin_glow_at(input: VertexOutput, pixel: f32, x: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let grain = skin_granulation(skin, along, time, seed);
    let flare = skin_flares(skin, along, input.length, time, seed);
    let beat = skin_beat(skin, time);
    // Shimmer and swell: the corona breathes a little, a flare widens it, a beat too.
    let shimmer = skin_wave(skin.shimmer_a, time, along, seed)
        + skin_wave(skin.shimmer_b, time, along, seed);
    let swell = skin.swell;
    var widen = min(1.0 + shimmer + swell.y * (grain - 0.5) + swell.z * flare
        + skin.pulse_b.y * beat, swell.x);
    // Past the tip (`beyond`, projected units) the capsule ends round: it widens about the
    // tip as it widens about the shaft, and the distance out is taken from the tip, so the
    // corona's grading and its tongues go round the end instead of running on straight.
    let beyond = max(input.blade.y - input.shaft, 0.0);
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let radial = select(abs(x), length(vec2(x, beyond)), beyond > 0.0);
    let around = select(along, along + r * atan2(beyond, max(abs(x), 0.0001)), beyond > 0.0);
    let ragged = skin.sputter_edge;
    if ragged.x > 0.0 {
        // A frayed edge: the width wanders by noise, each side its own.
        let fray = skin_noise(vec2(around * ragged.y - time * ragged.z,
            time * ragged.z * 0.37 + seed * 13.0 + select(0.0, 50.0, x < 0.0)));
        widen = clamp(widen * (1.0 + ragged.x * (fray * 2.0 - 1.0)), 0.3, swell.x);
    }
    var shaded = input;
    shaded.blade = vec2(x / widen, min(input.blade.y, input.shaft) + beyond / widen);
    let capsule = glow_capsule(shaded) / widen;
    let out = radial / (r * widen);
    // Flame tongues licking outward in the corona's edge.
    let ta = skin.tongue_a;
    let tb = skin.tongue_b;
    let tongue = skin_noise(vec2(around * ta.x + seed * ta.y, out * ta.z - time * ta.w));
    let edge = smoothstep(tb.x, tb.y, out);
    let flicker = mix(1.0, tb.z + tb.w * tongue, edge);
    // The inside colour by the core, the rim's at the edge; granulation and flares heat it.
    let inner = 1.0 - smoothstep(0.0, skin.rim_cool.w, out);
    let rim = mix(skin.rim_cool.rgb, skin.rim_hot.rgb,
        clamp(grain * skin.rim_hot.w + flare * skin.inner.w, 0.0, 1.0));
    let share = skin.inner_mix;
    let color = mix(rim, skin.inner.rgb,
        clamp(inner * inner * (share.x + share.y * grain) + flare * share.z, 0.0, 1.0));
    let light = skin.brightness;
    let brightness = (light.x + light.y * grain + light.z * flare) * flicker * (1.0 + beat);
    var glow = capsule * color * brightness;
    if skin.vein_color.w > 0.0 {
        let veins = skin_veins(skin, along, x, time, seed);
        glow += veins * (1.0 - smoothstep(0.15, 0.6, out)) * capsule.r * skin.vein_color.rgb
            * skin.vein_color.w * (1.0 + beat);
    }
    let scan = skin.scan_lines;
    if scan.x > 0.0 {
        // A hologram: scan lines darken it as they run, its inside is hollow, and a
        // wireframe (side lines and rings) is drawn over it.
        let lines = 1.0 - scan.z * (0.5 + 0.5 * sin(TAU * (around * scan.x - time * scan.y)));
        let e = skin.scan_edge;
        glow *= lines * (1.0 - e.z * (1.0 - smoothstep(0.0, e.x, out)));
        let soft = pixel / max(r * widen, 0.0001);
        // Not behind the hilt, where the place along stops at the hilt.
        let shown = smoothstep(-pixel, 0.0, input.blade.y);
        let sides = (1.0 - smoothstep(e.y, e.y + soft, abs(out - e.x))) * shown;
        var rings = 0.0;
        if scan.w > 0.0 {
            let f = fract(around * scan.w);
            let units = min(f, 1.0 - f) / scan.w;
            let half = e.y * r * 0.5;
            rings = (1.0 - smoothstep(half, half + pixel, units))
                * (1.0 - smoothstep(e.x, e.x + soft, out)) * shown;
        }
        glow += (sides + rings) * lines * skin.scan_color.rgb * skin.scan_color.w;
    }
    if skin.arc_strike.x > 0.0 || skin.mote_field.x > 0.0 {
        // Both fade out before the quad's edge (vertex_main's `extent`) instead of being cut.
        let extent = skin_extent(skin, input);
        let room = (1.0 - smoothstep(0.75, 1.0, abs(x) / extent))
            * (1.0 - smoothstep(0.75, 1.0, beyond / extent));
        if skin.arc_strike.x > 0.0 {
            // Along the axis and straight on past the tip.
            let reach = select(input.blade.y / max(input.shaft, 0.0001) * input.length,
                input.length + beyond, beyond > 0.0);
            let arcs = skin_arcs(skin, reach, x, r, input.length, extent, pixel, time, seed);
            glow += arcs * room * skin.arc_color.rgb * skin.arc_color.w;
        }
        if skin.mote_field.x > 0.0 {
            let unclamped = input.blade.y / max(input.shaft, 0.0001) * input.length;
            let motes = skin_motes(skin, around, radial / r, x < 0.0, unclamped, input.length,
                time, seed);
            glow += motes * room * skin.mote_color.rgb * skin.mote_color.w;
        }
    }
    glow = skin_apply_tint(glow, skin_tint(skin, input));
    let hue = skin.hue;
    if any(hue.xyz != vec3(0.0)) {
        glow = skin_turn_hue(glow, time * hue.x + around * hue.y + radial / r * hue.z);
    }
    // A chroma takes its wearer's colour: everything it drew, turned at once.
    if input.chroma != 0.0 {
        glow = skin_turn_hue(glow, input.chroma);
    }
    return glow;
}

// The core line: `uv` its texture coordinates (`core_coordinates`) and `footprint` how far
// they move in a pixel, which softens the rounded tip's edge.
fn skin_core(input: VertexOutput, texel: vec4<f32>, uv: vec2<f32>, footprint: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let grain = skin_granulation(skin, along, time, seed);
    let flare = skin_flares(skin, along, input.length, time, seed);
    let beat = skin_beat(skin, time);
    let bright = skin.core_fringe;
    var fringe = mix(skin.core_fringe_cool.rgb, skin.core_fringe_hot.rgb,
        skin.core_fringe_cool.w + skin.core_fringe_hot.w * grain)
        * (bright.x + bright.y * grain + bright.z * flare);
    fringe = skin_apply_tint(fringe, skin_tint(skin, input));
    let hue = skin.hue;
    if any(hue.xyz != vec3(0.0)) {
        fringe = skin_turn_hue(fringe, time * hue.x + along * hue.y);
    }
    var core = CORE_DRAWS * (skin.core_white.rgb * texel.r * (1.0 + skin.core_white.w * flare)
        + fringe * texel.g) * (1.0 + beat);
    if skin.vein_color.w > 0.0 {
        let x = (uv.x - 0.5) * 2.0 * input.radius;
        core += skin_veins(skin, along, x, time, seed) * (texel.r + texel.g) * skin.vein_color.rgb
            * skin.vein_shape.w * (1.0 + beat);
    }
    let scan = skin.scan_lines;
    if scan.x > 0.0 {
        core *= 1.0 - scan.z * (0.5 + 0.5 * sin(TAU * (along * scan.x - time * scan.y)));
    }
    // In the rounded tip the line is cut at the narrowing edge `core_coordinates` maps to the
    // texture's border (which the sampler would otherwise smear out to the quad's corners).
    if along > input.length - skin.core_fringe.w * input.radius {
        core *= clamp((0.5 - abs(uv.x - 0.5)) / max(footprint, 0.00001) + 0.5, 0.0, 1.0);
    }
    let cut = skin_cut(skin, input.length, time, seed);
    if cut < input.length {
        core *= 1.0 - smoothstep(cut - 1.5, cut + 0.5, along);
    }
    if input.chroma != 0.0 {
        core = skin_turn_hue(core, input.chroma);
    }
    return core;
}

// The core line's texture coordinates; a skin's core breathes a little across, narrows to
// its rounded tip and moves with a glitch or the hologram's jitter (in core half-widths,
// the glow's radius about three of them).
fn core_coordinates(input: VertexOutput) -> vec2<f32> {
    var across = input.blade.x / (2.0 * input.radius) * core_stretch(input);
    if input.kind >= KIND_SKIN {
        let along = mix(-1.0, input.length, input.blade.y);
        let skin = skin_at(input);
        let moved = skin_glitch(skin, along, input.animation.x, input.animation.y).x
            + skin_jitter(skin, input.animation.x, input.animation.y);
        across -= 1.5 * moved;
    }
    return vec2(0.5 + across, 1.0 - input.blade.y);
}

// How much a skin's core line is stretched across here: its breathing, and its rounded tip,
// where the width left goes to nothing (1 for a stock line).
fn core_stretch(input: VertexOutput) -> f32 {
    if input.kind < KIND_SKIN {
        return 1.0;
    }
    let along = mix(-1.0, input.length, input.blade.y);
    let skin = skin_at(input);
    return (1.0 + skin_wave(skin.core_breathe, input.animation.x, along, input.animation.y))
        / skin_tip_taper(along, input.length, skin.core_fringe.w * input.radius);
}

// The colour split a glitch gives the core at this fragment, in texture units across.
fn core_split(input: VertexOutput) -> f32 {
    if input.kind < KIND_SKIN {
        return 0.0;
    }
    let skin = skin_at(input);
    let along = mix(-1.0, input.length, input.blade.y);
    return 1.5 * skin_glitch(skin, along, input.animation.x, input.animation.y).y;
}

// Glint n's size: every other one of an orbit is smaller, a companion to the first.
fn glint_scale(n: f32) -> f32 {
    return select(1.0, 0.7, (i32(round(n)) & 1) == 1);
}

// A black hole's light at `p` (units from its heart, facing the camera): a thin photon
// ring just outside the horizon and a fainter accretion swirl round it, two spiral arms
// turning `spin` turns a second; nothing inside the horizon.
fn hole_ring(skin: Skin, p: vec2<f32>, time: f32, seed: f32, pixel: f32) -> f32 {
    let size = skin.hole_shape.x;
    let r = length(p);
    let width = max(0.07 * size, pixel);
    let ring = exp(-pow((r - 1.15 * size) / width, 2.0));
    let turn = atan2(p.y, p.x) / TAU;
    let arms = 0.5 + 0.5 * sin(TAU * (2.0 * turn + time * skin.hole_shape.z + seed)
        + 2.5 * r / size);
    let swirl = exp(-pow((r - 1.7 * size) / (0.55 * size), 2.0)) * (0.3 + 0.7 * arms) * 0.6;
    return (ring + swirl) * smoothstep(0.9 * size, 1.05 * size, r);
}

// A skin's glint, coloured as its glow is (the wearer's tint and chroma, an afterimage's
// fading), dimmed behind the blade; each of an orbit twinkles and turns at its own pace.
// A black hole's ring and swirl instead when the skin has holes.
fn star_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let n = STAR - input.hilt;
    let scale = glint_scale(n);
    var glint: vec3<f32>;
    if skin.hole_shape.x > 0.0 {
        glint = hole_ring(skin, input.blade / scale, input.animation.x,
            input.animation.y + n * 0.37, pixel / scale) * skin.hole_color.rgb
            * skin.hole_color.w;
    } else if skin.star_color.w > 0.0 {
        glint = skin_star(skin, input.blade / scale, input.animation.x,
            input.animation.y + n * 0.37, pixel / scale) * skin.star_color.rgb
            * skin.star_color.w;
    } else {
        return vec3(0.0);
    }
    glint = skin_apply_tint(glint, skin_tint(skin, input));
    if input.chroma != 0.0 {
        glint = skin_turn_hue(glint, input.chroma);
    }
    return glint * persona_fade(input) * (1.0 - (1.0 - GLINT_BEHIND) * input.shaft);
}

// Dynamic glow: the glow capsule only; the core line's shader has no `glow` stage.
@fragment
fn fragment_glow(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow).
    let pixel = fwidth(input.blade.x);
    if input.hilt <= STAR { return vec4(star_glow(input, pixel), 1.0); }
    if input.hilt <= 0.0 { discard; }
    if input.kind >= KIND_SKIN { return vec4(skin_glow(input, pixel), 1.0); }
    return vec4(glow_capsule(input) * input.color, 1.0);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // v runs from the tip (0) to behind the hilt (1), as DoLine's texture coordinates do.
    // Derivatives are taken before any branch (uniform control flow); the glow ignores the
    // core's and a skin's glow takes the pixel's size.
    let core_uv = core_coordinates(input);
    let core_dx = dpdx(core_uv);
    let core_dy = dpdy(core_uv);
    let pixel = fwidth(input.blade.x);
    if input.hilt <= STAR {
        return vec4(star_glow(input, pixel), 1.0);
    }
    if input.hilt > 0.0 {
        if input.kind >= KIND_SKIN { return vec4(skin_glow(input, pixel), 1.0); }
        return vec4(glow_capsule(input) * input.color, 1.0);
    }
    // How far the core's `u` moves in a pixel, from the straight line's across scaled by the
    // stretch here: the derivatives of `core_uv` itself blow up on the tip's last row, where
    // the stretch runs to a hundred within one pixel, and the rounded tip's cut then left
    // half the line's width lit there, a bar across the tip.
    let footprint = pixel / (2.0 * input.radius) * core_stretch(input);
    var texel = textureSampleGrad(core_texture, core_sampler, core_uv, core_dx, core_dy);
    let split = core_split(input);
    if split > 0.0 {
        // A glitch parts the core's colours: red one way, blue the other.
        let offset = vec2(split, 0.0);
        let red = textureSampleGrad(core_texture, core_sampler, core_uv + offset, core_dx, core_dy);
        let blue = textureSampleGrad(core_texture, core_sampler, core_uv - offset, core_dx, core_dy);
        let skin = skin_core(input, texel, core_uv, footprint);
        return vec4(
            skin_core(input, red, core_uv + offset, footprint).r,
            skin.g,
            skin_core(input, blue, core_uv - offset, footprint).b,
            1.0,
        );
    }
    if input.kind >= KIND_SKIN {
        return vec4(skin_core(input, texel, core_uv, footprint), 1.0);
    }
    if input.kind == KIND_NEUTRAL {
        // Neutral core: red = white-hot core, green = tinted fringe.
        return vec4(CORE_DRAWS * (vec3(texel.r) + input.color * texel.g), 1.0);
    }
    return vec4(CORE_DRAWS * texel.rgb * input.color, 1.0);
}

// --- Heat haze: drawn first into the effect layer, replacing its pixels ---------------------

// The scene as the effect layer holds it before any effect (effect_layer.rs `original`).
@group(3) @binding(0)
var haze_scene: texture_2d<f32>;
@group(3) @binding(1)
var haze_sampler: sampler;

// The scene behind a skin's glow bent by its heat: a rising, curling shift (at most
// `strength` world units, turned into pixels with the quad's own derivatives), strongest
// by the blade and above it, nothing at the reach's edge, so the quad's edge is the scene
// itself, unchanged.
@fragment
fn fragment_haze(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow): units across and along a pixel.
    let dx = dpdx(input.blade);
    let dy = dpdy(input.blade);
    let size = vec2<f32>(textureDimensions(haze_scene));
    let here = input.clip_position.xy;
    let skin = skin_at(input);
    let h = skin.haze;
    let up = skin_up(input.down);
    let at = skin_above(input.blade, input.shaft, up);
    let distance = length(at);
    let r = chain_radius(input.radius, input.length,
        clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0)) * skin.swell.x;
    // Full by the glow, gone at its edge plus the reach; more above the blade (heat rises).
    let fall = 1.0 - smoothstep(0.3 * r, r + h.w, distance);
    let rising = mix(0.55, 1.0, smoothstep(-r, r, at.x));
    let time = input.animation.x;
    let seed = input.animation.y;
    let a = skin_rising(input.blade, up, h.y, h.z, 0.6, time, seed);
    let b = skin_rising(input.blade + vec2(17.3, -9.1), up, h.y, h.z, 0.6, time, seed);
    let shift = (vec2(a, b) - 0.5) * 2.0 * h.x * fall * rising;
    // Never more than a few dozen pixels, however close.
    let pixels = to_pixels(shift, dx, dy, 0.02 * max(size.x, size.y));
    let uv = (here + pixels) / size;
    return vec4(textureSampleLevel(haze_scene, haze_sampler, uv, 0.0).rgb, 1.0);
}

// `shift` in the quad's units as pixels (the inverse of its Jacobian `dx`, `dy`), at
// most `most` long.
fn to_pixels(shift: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, most: f32) -> vec2<f32> {
    let det = dx.x * dy.y - dx.y * dy.x;
    var pixels = vec2(0.0);
    if abs(det) > 1e-8 {
        pixels = vec2(shift.x * dy.y - shift.y * dy.x, -shift.x * dx.y + shift.y * dx.x) / det;
    }
    let len = length(pixels);
    if len > most {
        pixels *= most / len;
    }
    return pixels;
}

// The black holes' quads (the lens and the shade): a skin's glint instances when it has
// holes its wearer left on; every other instance is folded away.
@vertex
fn vertex_hole(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    var hole = output.kind >= KIND_SKIN && output.hilt <= STAR && (blade.off & OFF_HOLES) == 0u;
    if hole {
        hole = skins[skin_index(output)].hole_shape.x > 0.0;
    }
    if !hole {
        output.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
    }
    return output;
}

// A black hole's lens, drawn with the haze before any glow: the scene round it bent as
// a point mass bends light, what is behind it drawn into a ring (`lens` horizon radii
// past the horizon) and squeezed toward it, fading to the scene unbent at the quad's
// edge (its alpha), and weaker while it is behind the blade.
@fragment
fn fragment_lens(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow).
    let dx = dpdx(input.blade);
    let dy = dpdy(input.blade);
    let size = vec2<f32>(textureDimensions(haze_scene));
    let skin = skin_at(input);
    let scale = glint_scale(STAR - input.hilt);
    let horizon = skin.hole_shape.x * scale;
    let reach = skin.hole_shape.w * scale;
    let ring = horizon * (1.0 + skin.hole_shape.y);
    let p = input.blade;
    let r = length(p);
    let fade = 1.0 - smoothstep(0.6 * reach, reach, r);
    let shift = -p / max(r, 0.0001) * (ring * ring / max(r, horizon)) * fade;
    let pixels = to_pixels(shift, dx, dy, 0.06 * max(size.x, size.y));
    let uv = (input.clip_position.xy + pixels) / size;
    let seen = textureSampleLevel(haze_scene, haze_sampler, uv, 0.0).rgb;
    return vec4(seen, fade * (1.0 - 0.6 * input.shaft));
}

// A black hole's horizon, drawn after the glow: black, over the blade's glow when the
// hole is in front of it; behind the blade (and close to its axis) the glow shows.
@fragment
fn fragment_shade(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = fwidth(input.blade.x);
    let skin = skin_at(input);
    let horizon = skin.hole_shape.x * glint_scale(STAR - input.hilt);
    let disc = clamp((horizon - length(input.blade)) / max(pixel, 0.0001) + 0.5, 0.0, 1.0);
    return vec4(0.0, 0.0, 0.0, disc * (1.0 - input.shaft));
}
