struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) start_color: vec4<f32>,
    @location(4) end_color: vec4<f32>,
    @location(5) parameters: vec2<f32>,
    @location(6) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) start_color: vec4<f32>,
    @location(3) end_color: vec4<f32>,
    @location(4) parameters: vec2<f32>,
    @location(5) uv: vec2<f32>,
}

@group(0) @binding(0) var icon_texture: texture_2d<f32>;
@group(0) @binding(1) var icon_sampler: sampler;

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4(input.position, 0.0, 1.0);
    output.local = input.local;
    output.size = input.size;
    output.start_color = input.start_color;
    output.end_color = input.end_color;
    output.parameters = input.parameters;
    output.uv = input.uv;
    return output;
}

// SJK's strafe aura, the same expressions as `sjk_ui::strafe_aura_shade` (aura.rs).
fn strafe_gain(speed: f32, acceleration: f32, wishspeed: f32, angle: f32) -> f32 {
    let along = speed * cos(angle);
    let add = clamp(wishspeed - along, 0.0, acceleration);
    let grow = 2.0 * along * add + add * add;
    let next = sqrt(max(speed * speed + grow, 0.0));
    if next + speed > 0.0 {
        return grow / (next + speed);
    }
    return 0.0;
}

// WGSL's pow is undefined for a negative base.
fn square(x: f32) -> f32 {
    return x * x;
}

fn wrap_angle(angle: f32) -> f32 {
    return angle - 6.2831855 * floor((angle + 3.1415927) / 6.2831855);
}

fn strafe_aura(input: VertexOutput) -> vec4<f32> {
    // start_color = (speed, acceleration, wish speed, base angle), end_color = (radius,
    // width, span, time), parameters.x = opacity, uv = the quad's corner from the centre.
    let speed = input.start_color.x;
    let acceleration = input.start_color.y;
    let wishspeed = input.start_color.z;
    let base = input.start_color.w;
    let radius = input.end_color.x;
    let half = input.end_color.y * 0.5;
    let span = input.end_color.z;
    let t = input.end_color.w;
    let pixel = input.uv + input.local * input.size;
    let across = (length(pixel) - radius) / half;
    if abs(across) > 1.6 || half <= 0.0 || speed <= 0.0 {
        discard;
    }
    let turn = wrap_angle(-1.5707964 - atan2(pixel.y, pixel.x));
    let fade = 1.0 - smoothstep(span - 0.15, span, abs(turn));
    let peak = acos(clamp((wishspeed - acceleration) / speed, -1.0, 1.0));
    let top = strafe_gain(speed, acceleration, wishspeed, peak);
    if fade <= 0.0 || top <= 0.0 {
        discard;
    }
    let angle = base + turn;
    let n = strafe_gain(speed, acceleration, wishspeed, angle) / top;
    let to_best = min(abs(wrap_angle(angle - peak)), abs(wrap_angle(angle + peak)));
    let flow = 0.5 + 0.5 * sin(to_best * 40.0 - t * 5.0 + sin(across * 3.0 + t * 1.7) * 1.2);
    let flicker = 0.5 + 0.5 * (sin(turn * 23.0 + t * 2.3 + across * 4.0) * sin(turn * 11.0 - t * 1.3));
    var pulse = 1.0;
    if strafe_gain(speed, acceleration, wishspeed, base) / top > 0.9 {
        pulse = 1.0 + 0.35 * sin(t * 9.0);
    }
    let seam = exp(-square(to_best / 0.02)) * exp(-square(across / 0.55));
    var rgb = vec3(1.0);
    var a = 0.0;
    if n > 0.0 {
        let hot = n * n;
        let flame = across / (0.35 + 0.65 * n) + (flow - 0.5) * 0.35;
        let body = exp(-flame * flame * 3.0);
        let core = exp(-square(across / 0.18));
        rgb = vec3(0.22 + 0.63 * hot, 0.5 + 0.45 * hot, 1.0);
        a = (body * (0.3 + 0.4 * flow + 0.2 * flicker) * sqrt(n) + core * 0.6 * hot) * pulse;
    } else if n < 0.0 {
        rgb = vec3(1.0, 0.28, 0.2);
        a = min(-n, 1.0) * 0.55 * exp(-square(across / 0.3)) * (0.7 + 0.3 * flow);
    }
    if seam > 0.0 {
        rgb = rgb + (vec3(1.0) - rgb) * seam;
        a = max(a, seam * 0.95 * min(pulse, 1.0));
    }
    a = clamp(a * fade * input.parameters.x, 0.0, 1.0);
    if a <= 0.0 {
        discard;
    }
    return vec4(rgb, a);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.parameters.y > 4.5 {
        return strafe_aura(input);
    }
    if input.parameters.y > 3.5 {
        // Straight stroke: the quad runs along the line with one pixel of margin on
        // each side; coverage fades across one pixel at the stroke's edges.
        // end_color.x = stroke width in pixels.
        let across = abs(input.local.y - 0.5) * input.size.y - input.end_color.x * 0.5;
        let coverage = clamp(0.5 - across, 0.0, 1.0);
        if coverage <= 0.0 {
            discard;
        }
        return vec4(input.start_color.rgb, input.start_color.a * coverage);
    }
    if input.parameters.y > 2.5 {
        // Arc stroke with round caps: distance to the nearest point of the centre
        // line, which is the circle's arc clamped to its angular range.
        // end_color = (radius, width, start angle, sweep) in pixels and radians. The
        // knockout stripe, where the stroke is left out, is parameters.x (left edge), uv.x
        // (right edge) and uv.y (half height), in pixels from the centre; a zero height
        // is no stripe.
        let radius = input.end_color.x;
        let half_width = input.end_color.y * 0.5;
        let sweep = input.end_color.w;
        let pixel = input.local * input.size - input.size * 0.5;
        let middle = input.end_color.z + sweep * 0.5;
        var delta = atan2(pixel.y, pixel.x) - middle;
        // Wrap to (-pi, pi] so the arc may cross the +/-pi seam.
        delta = delta - 6.2831855 * floor((delta + 3.1415927) / 6.2831855);
        let limit = abs(sweep) * 0.5;
        let angle = middle + clamp(delta, -limit, limit);
        let nearest = radius * vec2(cos(angle), sin(angle));
        let distance = length(pixel - nearest) - half_width;
        var coverage = clamp(0.5 - distance, 0.0, 1.0);
        if input.uv.y > 0.0 {
            // The same rounded-box distance as the panels, of an unrounded stripe
            // (`sjk_ui::knockout_coverage`). The panel there has this stroke's colour and
            // alpha, so only what it leaves is added (`sjk_ui::knockout_remainder`).
            let q = abs(vec2(pixel.x - (input.parameters.x + input.uv.x) * 0.5, pixel.y))
                - vec2((input.uv.x - input.parameters.x) * 0.5, input.uv.y);
            let outside = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0);
            let knock = clamp(0.5 - outside, 0.0, 1.0);
            coverage = clamp(
                (coverage - knock) / max(1.0 - input.start_color.a * knock, 1.0e-4),
                0.0,
                1.0,
            );
        }
        if coverage <= 0.0 {
            discard;
        }
        return vec4(input.start_color.rgb, input.start_color.a * coverage);
    }
    if input.parameters.y > 1.5 {
        return textureSample(icon_texture, icon_sampler, input.uv) * input.start_color;
    }
    let radius = input.parameters.x;
    // A negative parameters.y is a rounded border: its magnitude is the ring width.
    let ring = -input.parameters.y;
    var coverage = 1.0;
    if radius > 0.0 || ring > 0.0 {
        // Signed distance to the rounded outline in pixels; the edge is
        // anti-aliased by fading alpha across one pixel of it (and across
        // the inner edge of a ring) instead of a hard discard.
        let pixel = input.local * input.size;
        let half_size = input.size * 0.5;
        let q = abs(pixel - half_size) - (half_size - vec2(radius));
        let distance = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
        coverage = clamp(0.5 - distance, 0.0, 1.0);
        if ring > 0.0 {
            coverage *= clamp(distance + ring + 0.5, 0.0, 1.0);
        }
        if coverage <= 0.0 {
            discard;
        }
    }
    let amount = select(input.local.x, input.local.y, input.parameters.y > 0.5);
    let color = mix(input.start_color, input.end_color, amount);
    return vec4(color.rgb, color.a * coverage);
}
