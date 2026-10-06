// Weather: rain streaks, sprite clouds (snow, dust, mist) and splashes, drawn into the
// display-space effect layer after the effects, as rd-vanilla draws its world effects
// last. Every particle is derived from its instance index and the flow the CPU
// integrates: no vertex or instance buffer exists.
//
// Instance numbers: bits 16.. are the cloud slot (0-4) or 7 for splashes, bit 15 is the
// far rain layer, bits 0-14 the particle.
struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    shader_time: f32,
    forward: vec3<f32>,
    view_flags: f32,
};

struct Cloud {
    // rgb display colour, a the reference's alpha target.
    color: vec4<f32>,
    // x half width, y streak length or sprite half height, z image layer, w spin.
    shape: vec4<f32>,
    // xyz box corner relative to the camera, w near-layer count.
    box_min: vec4<f32>,
    // xyz box size, w far-layer count.
    box_size: vec4<f32>,
    // x far layer's horizontal scale, y far layer's opacity, zw unused.
    layers: vec4<f32>,
    // Per mass bucket: xyz velocity.
    velocity: array<vec4<f32>, 8>,
    // Per mass bucket: xyz flow offset, wrapped to the far box (x, y) and the box (z).
    offset: array<vec4<f32>, 8>,
};

struct Weather {
    // First cell x, y of the cover window, and one past its last.
    window: vec4<f32>,
    // x cell size, y cover texture size, z cover in use, w weather time (seconds).
    cover: vec4<f32>,
    // xy viewport pixels, z global fog's 1 / depthForOpaque (0 none), w splash count.
    view: vec4<f32>,
    // rgb light tint, w the cloud whose rain splashes.
    light: vec4<f32>,
    clouds: array<Cloud, 5>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> weather: Weather;
@group(1) @binding(1) var cover_map: texture_2d<f32>;
@group(1) @binding(2) var images: texture_2d_array<f32>;
@group(1) @binding(3) var image_sampler: sampler;
@group(2) @binding(0) var scene_depth: texture_depth_2d;

// Additive (GL_ONE GL_ONE) or alpha-blended sprites.
override additive: bool = true;

const SPLASH: u32 = 1u;
const LIQUID: u32 = 2u;
const HIDDEN: vec4<f32> = vec4(2.0, 2.0, 2.0, 1.0);
const TAU: f32 = 6.2831853;

// PCG hash (Jarzynski and Olano, 2020).
fn pcg(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash3(value: u32) -> vec3<f32> {
    let a = pcg(value);
    let b = pcg(a);
    let c = pcg(b);
    return vec3(f32(a), f32(b), f32(c)) * (1.0 / 4294967296.0);
}

// Two triangles: corners (-,-) (+,-) (+,+) and (-,-) (+,+) (-,+).
fn corner(vertex: u32) -> vec2<f32> {
    let x = f32((0x16u >> vertex) & 1u);
    let y = f32((0x34u >> vertex) & 1u);
    return vec2(x, y) * 2.0 - 1.0;
}

// The weather span of the column under `p`: x bottom, y top, z flags. Columns outside
// the surveyed window are covered; without a cover everything is open.
fn cover_at(p: vec2<f32>) -> vec3<f32> {
    if weather.cover.z == 0.0 { return vec3(-3.0e38, 3.0e38, 0.0); }
    let cell = floor(p / weather.cover.x);
    if any(cell < weather.window.xy) || any(cell >= weather.window.zw) {
        return vec3(3.0e38, -3.0e38, 0.0);
    }
    let size = i32(weather.cover.y);
    let texel = ((vec2<i32>(cell) % size) + size) % size;
    return textureLoad(cover_map, texel, 0).xyz;
}

fn open_at(p: vec3<f32>) -> bool {
    let span = cover_at(p.xy);
    return p.z >= span.x && p.z <= span.y;
}

// The global fog between the eye and `p`, as the world's GL_EXP2 fog: 1 clear, 0 lost.
fn fog_clear(p: vec3<f32>) -> f32 {
    let ratio = dot(p - camera.position, normalize(camera.forward)) * weather.view.z;
    return exp(-5.541263545 * ratio * ratio);
}

fn wrap(value: vec3<f32>, size: vec3<f32>) -> vec3<f32> {
    return value - size * floor(value / size);
}

fn wrap2(value: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    return value - size * floor(value / size);
}

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    seed: vec3<f32>,
    // Distance and box-edge fades, not yet the cover.
    fade: f32,
};

// Where particle `index` of `cloud` is now: anchored in the world, carried by its mass
// bucket's flow, wrapped into the box around the camera.
fn particle(cloud: Cloud, slot: u32, index: u32, far: bool) -> Particle {
    var result: Particle;
    let salt = slot * 0x85EBCA77u + select(0u, 0xC2B2AE3Du, far);
    result.seed = hash3(index * 0x9E3779B1u + salt);
    let bucket = pcg(index ^ salt ^ 0x27D4EB2Fu) & 7u;
    let scale = select(vec3(1.0), vec3(cloud.layers.x, cloud.layers.x, 1.0), far);
    let size = cloud.box_size.xyz * scale;
    let low = cloud.box_min.xyz * scale;
    let anchor = result.seed * size + cloud.offset[bucket].xyz;
    let local = low + wrap(anchor - (camera.position + low), size);
    result.position = camera.position + local;
    result.velocity = cloud.velocity[bucket].xyz;
    let edge = abs(local - (low + 0.5 * size)) / (0.5 * size);
    let distance = length(local);
    var fade = 1.0 - smoothstep(0.8, 1.0, max(max(edge.x, edge.y), edge.z));
    fade *= smoothstep(16.0, 64.0, distance);
    if far {
        // The far layer starts where the near one thins out.
        let near = 0.5 * min(cloud.box_size.x, cloud.box_size.y);
        fade *= smoothstep(0.6 * near, near, length(local.xy)) * cloud.layers.y;
    }
    result.fade = fade * fog_clear(result.position);
    return result;
}

struct StreakOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    // x across the streak (-1..1), y along it (0 head, 1 tail).
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) opacity: f32,
};

@vertex fn vertex_streak(@builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32) -> StreakOutput {
    var output: StreakOutput;
    output.position = HIDDEN;
    let slot = instance >> 16u;
    let far = ((instance >> 15u) & 1u) != 0u;
    let cloud = weather.clouds[slot];
    let p = particle(cloud, slot, instance & 0x7FFFu, far);
    if p.fade <= 0.002 { return output; }
    let speed = length(p.velocity);
    let direction = select(vec3(0.0, 0.0, -1.0), p.velocity / max(speed, 0.001), speed > 1.0);
    let length_ = cloud.shape.y * (0.8 + 0.4 * p.seed.z);
    let head = p.position;
    let tail = head - direction * length_;
    // Wholly under a roof or the ground, or above the sky: skip the quad.
    let head_span = cover_at(head.xy);
    let tail_span = cover_at(tail.xy);
    if (head.z < head_span.x && tail.z < tail_span.x) || (head.z > head_span.y && tail.z > tail_span.y) {
        return output;
    }
    let c = corner(vertex);
    let along = c.y * 0.5 + 0.5;
    let centre = mix(head, tail, along);
    var side = cross(direction, camera.position - centre);
    if dot(side, side) < 1e-6 { side = cross(direction, vec3(0.0, 0.0, 1.0)); }
    side = normalize(side);
    // Keep a streak at least about a pixel wide; a thinner one gets fainter instead
    // of flickering in and out.
    let a = camera.view_projection * vec4(centre, 1.0);
    let b = camera.view_projection * vec4(centre + side, 1.0);
    if a.w <= 1.0 || b.w <= 1.0 { return output; }
    let pixels = length((b.xy / b.w - a.xy / a.w) * weather.view.xy * 0.5);
    let half_width = max(cloud.shape.x, 0.6 / max(pixels, 0.0001));
    let world = centre + side * c.x * half_width;
    output.position = camera.view_projection * vec4(world, 1.0);
    output.world = world;
    output.uv = vec2(c.x, along);
    // Drops vary: some catch more light than others. x is the streak's opacity, so a
    // thinner or farther streak is fainter rather than whiter; yzw the cloud's colour.
    let catch_light = 0.55 + 0.45 * fract(p.seed.x * 7.31 + p.seed.y * 3.17);
    output.opacity = catch_light * cloud.color.a * p.fade * (cloud.shape.x / half_width);
    output.color = cloud.color.rgb;
    return output;
}

// A falling drop shows a blurred, tinted image of the bright sky around it more than a
// white line: rain is drawn over the scene as a faint streak of the light where the camera
// is, cooled toward the overcast blue-grey, brightest along its core.
const RAIN_TINT: vec3<f32> = vec3(0.80, 0.85, 0.92);
const RAIN_OPACITY: f32 = 0.42;

@fragment fn fragment_streak(input: StreakOutput) -> @location(0) vec4<f32> {
    if !open_at(input.world) { discard; }
    // Soft across, brightest just behind the head, fading along the tail.
    let across = 1.0 - input.uv.x * input.uv.x;
    let along = smoothstep(0.0, 0.08, input.uv.y) * pow(1.0 - input.uv.y, 1.2);
    let alpha = clamp(input.opacity * across * along * RAIN_OPACITY, 0.0, 1.0);
    // The reference's grey rain (0.5) is neutral; acid rain keeps its green.
    let hue = input.color / max(max(input.color.r, max(input.color.g, input.color.b)), 0.001);
    let core = 1.0 + 0.25 * across * across;
    return vec4(min(weather.light.rgb * RAIN_TINT * hue * core, vec3(1.0)), alpha);
}

struct SpriteOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) layer: i32,
};

@vertex fn vertex_sprite(@builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32) -> SpriteOutput {
    var output: SpriteOutput;
    output.position = HIDDEN;
    let slot = instance >> 16u;
    let cloud = weather.clouds[slot];
    let p = particle(cloud, slot, instance & 0x7FFFu, false);
    if p.fade <= 0.002 || !open_at(p.position) { return output; }
    let forward = normalize(camera.forward);
    var right = cross(forward, vec3(0.0, 0.0, 1.0));
    if dot(right, right) < 1e-4 { right = vec3(0.0, 1.0, 0.0); }
    right = normalize(right);
    let up = cross(right, forward);
    // `mRotation`: turning quads, ±0.7 radians a second.
    let angle = p.seed.x * TAU + weather.cover.w * (p.seed.y - 0.5) * 1.4 * cloud.shape.w;
    let turn = vec2(cos(angle), sin(angle));
    let c = corner(vertex);
    let r = vec2(c.x * turn.x - c.y * turn.y, c.x * turn.y + c.y * turn.x);
    let world = p.position + right * r.x * cloud.shape.x + up * r.y * cloud.shape.y;
    output.position = camera.view_projection * vec4(world, 1.0);
    output.uv = c * 0.5 + 0.5;
    output.layer = i32(cloud.shape.z);
    output.color = vec4(cloud.color.rgb * weather.light.rgb, cloud.color.a * p.fade);
    return output;
}

// The scene's depth gap along the view, in world units (soft_particles.wgsl's).
fn depth_gap(fragment: vec4<f32>) -> f32 {
    let scene = textureLoad(scene_depth, vec2<i32>(fragment.xy), 0);
    if scene >= 1.0 { return 1.0e6; }
    let w = 1.0 / fragment.w;
    let origin = camera.view_projection * vec4(camera.position, 1.0);
    if abs(w - origin.w) < 0.000001 { return 1.0e6; }
    let slope = (fragment.z * w - origin.z) / (w - origin.w);
    let denominator = scene - slope;
    if abs(denominator) < 0.00000001 { return 1.0e6; }
    let behind = (origin.z - slope * origin.w) / denominator;
    let row = vec3(camera.view_projection[0].w, camera.view_projection[1].w,
        camera.view_projection[2].w);
    return (behind - w) / max(length(row), 0.000001);
}

@fragment fn fragment_sprite(input: SpriteOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(images, image_sampler, input.uv, input.layer);
    let soft = clamp(depth_gap(input.position) / 24.0, 0.0, 1.0);
    let alpha = input.color.a * soft;
    if additive {
        return vec4(input.color.rgb * alpha * texel.rgb, 0.0);
    }
    return vec4(input.color.rgb * texel.rgb, alpha * texel.a);
}

struct SplashOutput {
    @builtin(position) position: vec4<f32>,
    // Ground: x across (-1..1), y up (0..1). Water: the ripple's plane (-1..1).
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    // x age (0..1), y 1 on water, z the droplets' spread.
    @location(2) @interpolate(flat) shape: vec3<f32>,
};

@vertex fn vertex_splash(@builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32) -> SplashOutput {
    var output: SplashOutput;
    output.position = HIDDEN;
    let rain = weather.clouds[u32(weather.light.w)];
    let index = instance & 0x7FFFu;
    let timing = hash3(index * 0x632BE5ABu + 0x1234567u);
    let life = 0.28 + 0.2 * timing.x;
    let clock = weather.cover.w / life + timing.y;
    let age = fract(clock);
    let seed = hash3(index * 0x9E3779B1u ^ (u32(i32(floor(clock))) * 0x85EBCA77u));
    let radius = 0.5 * min(rain.box_size.x, rain.box_size.y);
    // Anchored in the world as the streaks are: the spot repeats every box width, and
    // the copy inside the box around the camera is the one drawn, so a splash stays
    // where it struck while the camera moves and only wraps at the box's edge.
    let low = camera.position.xy - radius;
    let ground = low + wrap2(seed.xy * (2.0 * radius) - low, vec2(2.0 * radius));
    let span = cover_at(ground);
    let flags = u32(span.z);
    if (flags & SPLASH) == 0u || span.x > span.y || abs(span.x - camera.position.z) > 1200.0 {
        return output;
    }
    let base = vec3(ground, span.x);
    let distance = length(base - camera.position);
    let fade = (1.0 - smoothstep(0.7 * radius, radius, length(ground - camera.position.xy)))
        * smoothstep(24.0, 64.0, distance) * fog_clear(base);
    if fade <= 0.002 { return output; }
    let c = corner(vertex);
    let water = (flags & LIQUID) != 0u;
    var world: vec3<f32>;
    if water {
        let size = 3.0 + 13.0 * age;
        world = base + vec3(c * size, 0.5);
        output.uv = c;
    } else {
        let forward = normalize(camera.forward);
        var right = cross(forward, vec3(0.0, 0.0, 1.0));
        if dot(right, right) < 1e-4 { right = vec3(0.0, 1.0, 0.0); }
        right = normalize(right);
        let size = 6.0 + 4.0 * seed.z;
        world = base + right * c.x * size + vec3(0.0, 0.0, (c.y * 0.5 + 0.5) * size * 1.2);
        output.uv = vec2(c.x, c.y * 0.5 + 0.5);
    }
    output.position = camera.view_projection * vec4(world, 1.0);
    // x the splash's opacity; blended as the streaks are.
    output.color = vec3(rain.color.a * fade);
    output.shape = vec3(age, select(0.0, 1.0, water), 0.6 + 0.6 * seed.z);
    return output;
}

@fragment fn fragment_splash(input: SplashOutput) -> @location(0) vec4<f32> {
    let age = input.shape.x;
    var light = 0.0;
    if input.shape.y > 0.5 {
        // A ripple: a thin ring that widens and fades.
        let ring = length(input.uv) - (0.25 + 0.7 * age);
        light = exp(-ring * ring * 300.0) * (1.0 - age) * 1.5;
    } else {
        // A crown of droplets thrown up and out, falling back by the end, over a brief
        // flash where the drop struck.
        for (var k = 0; k < 5; k++) {
            let lean = (f32(k) - 2.0) * 0.38 * input.shape.z;
            let drop = vec2(lean * age, (1.9 - abs(lean) * 0.6) * age - 1.9 * age * age);
            let offset = (input.uv - drop) * vec2(1.0, 1.6);
            light += exp(-dot(offset, offset) * 90.0);
        }
        let strike = input.uv * vec2(1.6, 9.0);
        light = light * (1.0 - age * age) + exp(-dot(strike, strike)) * (1.0 - age) * (1.0 - age);
    }
    let alpha = clamp(input.color.x * light * 0.9, 0.0, 1.0);
    return vec4(min(weather.light.rgb * RAIN_TINT * 1.1, vec3(1.0)), alpha);
}
