// Material maps in OpenJK rend2's convention (`material_maps.rs`), appended to the stage
// program of a material-mapped world stage (`material_map_program.rs`). The normal map
// keeps its encoded normal in RGB and, for parallax, depth (255 - height) in alpha. The
// specular map was converted at load to rend2's spec/gloss layout or to occlusion,
// roughness, metalness, specular (`material_map_images.rs`). The emission map (SJK's `_e`)
// is an sRGB colour, read linear, added unlit.
struct MaterialMapParams {
    // rend2 normalScale: x/y strength, z unused, w parallax depth.
    normal_scale: vec4<f32>,
    // rend2 specularScale.
    specular_scale: vec4<f32>,
    // x flags: 1 normal map, 2 parallax, 4 diffuse in the secondary bundle, 8 two-sided,
    // 16 emission map, 32 in the glow pass for the emission only;
    // y specular layout: 0 none, 1 spec/gloss, 2 packed; z parallax bias.
    control: vec4<f32>,
};
@group(1) @binding(8) var material_map_normal: texture_2d<f32>;
@group(1) @binding(9) var material_map_specular: texture_2d<f32>;
// Per flattened world vertex: packed tangent and handedness, packed light-grid direction
// with the surface's reflection probe + 1 in its fourth byte.
@group(1) @binding(10) var<storage, read> material_map_frames: array<vec2<u32>>;
@group(1) @binding(11) var<uniform> material_map: MaterialMapParams;
@group(1) @binding(12) var material_map_sampler: sampler;
@group(1) @binding(17) var material_map_emission: texture_2d<f32>;

struct MaterialMapFrame { tangent: vec4<f32>, light: vec3<f32>, probe: f32 };
// The frame of vertex `index` turned with its instance (identity for the static world).
// Vertices past the flattened world (models appended later) have none: no tangent, and
// a zero light direction that leaves the lightmap's response unchanged.
fn material_map_frame(index: u32, rotation: vec4<f32>) -> MaterialMapFrame {
    if index >= arrayLength(&material_map_frames) {
        return MaterialMapFrame(vec4(0.0), vec3(0.0), 0.0);
    }
    let packed = material_map_frames[index];
    let tangent = unpack4x8snorm(packed.x);
    return MaterialMapFrame(vec4(rotate_vector(rotation, tangent.xyz), tangent.w),
        rotate_vector(rotation, unpack4x8snorm(packed.y).xyz), f32(packed.y >> 24u));
}

struct MaterialMapSurface {
    // Interpolated vertex normal, turned toward the viewer on two-sided faces.
    geometric: vec3<f32>,
    // The normal map's normal in world space; `geometric` without a usable map.
    normal: vec3<f32>,
    // Parallax offset of the diffuse texture coordinates.
    uv_offset: vec2<f32>,
    // The specular map's texel at the (offset) diffuse coordinates.
    specular: vec4<f32>,
    // The emission map's colour (linear) at the same coordinates; black without one.
    emission: vec3<f32>,
};
var<private> material_map_surface: MaterialMapSurface;
// Squared GGX roughness the normal map's variation within the pixel adds (specular
// anti-aliasing, `material_map_prepare`).
var<private> material_map_kernel: f32;
var<private> material_map_albedo: vec3<f32>;
// Specular light, added after the albedo product and dynamic-light modulation.
var<private> material_map_highlight: vec3<f32>;

fn material_map_flags() -> u32 { return u32(material_map.control.x); }
fn material_map_layout() -> u32 { return u32(material_map.control.y); }

// rend2 `CalcNormal` (`lightall.glsl`): the map's x/y scaled by normalScale, z rebuilt on
// the 0.5-radius sphere, then the tangent frame; the bitangent is sign * cross(N, T).
fn material_map_prepare(input: VertexOutput) {
    let flags = material_map_flags();
    let view = camera.camera_position - input.world_position;
    var geometric = normalize(input.world_normal);
    if (flags & 8u) != 0u && dot(geometric, view) < 0.0 { geometric = -geometric; }
    let uv = select(input.stage_uv, input.secondary_uv, (flags & 4u) != 0u);
    let along = input.material_tangent.xyz - geometric*dot(geometric, input.material_tangent.xyz);
    let framed = dot(along, along) > 1e-6;
    let tangent = along*inverseSqrt(max(dot(along, along), 1e-12));
    let bitangent = select(1.0, -1.0, input.material_tangent.w < 0.0)*cross(geometric, tangent);
    var offset = vec2(0.0);
    if (flags & 3u) == 3u {
        offset = material_map_parallax(uv, view, tangent, bitangent, geometric)*f32(framed);
    }
    // Flags and layout are per stage (uniform), so each map is only read when present.
    var texel = vec4(0.5, 0.5, 1.0, 1.0);
    if (flags & 1u) != 0u {
        texel = textureSample(material_map_normal, material_map_sampler, uv + offset);
    }
    var specular = vec4(0.0);
    if material_map_layout() != 0u {
        specular = textureSample(material_map_specular, material_map_sampler, uv + offset);
    }
    var emission = vec3(0.0);
    if (flags & 16u) != 0u {
        emission = textureSample(material_map_emission, material_map_sampler, uv + offset).rgb;
    }
    var normal = geometric;
    if framed {
        // `r_normalMapStrength` (`world_lighting_mode.rs`): zero bits are strength 1.
        let strength = f32((((point_lights.metadata.z >> 16u) + 64u) & 255u))/64.0;
        var n = texel.rgb - vec3(0.5);
        n = vec3(n.xy*material_map.normal_scale.xy*strength, 0.0);
        n.z = sqrt(clamp((0.25 - n.x*n.x) - n.y*n.y, 0.0, 1.0));
        normal = normalize(n.x*tangent + n.y*bitangent + n.z*geometric);
    }
    // Specular anti-aliasing (Kaplanyan and Hoffman 2016, as Tokuyoshi and Kaplanyan
    // 2019 bound it): bumps finer than a pixel cannot be shown, so their spread of
    // normals widens the highlight and blurs the reflection instead of sparkling and
    // swimming as the view moves. The normal's screen-space variation is the variance
    // added to the squared roughness, at most 0.18.
    let dx = dpdx(normal);
    let dy = dpdy(normal);
    material_map_kernel = min(0.5*(dot(dx, dx) + dot(dy, dy)), 0.18);
    material_map_surface = MaterialMapSurface(geometric, normal, offset, specular, emission);
    material_map_highlight = vec3(0.0);
    // One probe per surface: every vertex carries the same index.
    material_map_probe = u32(round(input.material_light.w));
    material_map_reflected = vec3(0.0);
}

// rend2 `GetParallaxOffset` and `RayIntersectDisplaceMap`: march the view ray through the
// depth in the normal map's alpha, 16 linear then 8 binary steps.
fn material_map_parallax(uv: vec2<f32>, view: vec3<f32>, tangent: vec3<f32>,
    bitangent: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    let size = vec2<f32>(textureDimensions(material_map_normal));
    let tangent_view = vec3(dot(view, tangent), dot(view, bitangent), dot(view, normal));
    let square = select(vec3(size.y/size.x, 1.0, 1.0), vec3(1.0, size.x/size.y, 1.0),
        size.y <= size.x);
    let direction = normalize(tangent_view*square);
    let dx = dpdx(uv);
    let dy = dpdy(uv);
    // rend2 offsets by depth / cos, which grows without bound toward grazing views and
    // makes the texture swim as the view moves; and where the height map is minified
    // the march reads mip levels that no longer hold the relief. The depth fades out
    // below about 20 degrees above the surface and from 1.5 to 4 texels per pixel, and
    // the offset is limited to depth / 0.35 (Welsh's offset limiting) on the way.
    // `r_parallaxStrength` (lighting-mode bits 2-7, `world_lighting_mode.rs`, where zero
    // bits are the default 0.1) scales the depth; 0 leaves the surface flat.
    let strength = f32((((point_lights.metadata.z >> 2u) + 4u) & 63u))/40.0;
    let texels = max(length(dx*size), length(dy*size));
    let fade = strength*smoothstep(0.15, 0.35, direction.z)*(1.0 - smoothstep(1.5, 4.0, texels));
    if fade <= 0.0 { return vec2(0.0); }
    let ds = direction.xy*(-material_map.normal_scale.w*fade/max(direction.z, 0.35));
    let bias = material_map.control.z;
    let start = uv - bias*ds;
    var size_step = 1.0/16.0;
    var depth = 0.0;
    var best = 1.0;
    for (var i = 0; i < 15; i++) {
        depth += size_step;
        if depth >= textureSampleGrad(material_map_normal, material_map_sampler,
            start + ds*depth, dx, dy).a {
            best = depth;
            break;
        }
    }
    depth = best;
    for (var i = 0; i < 8; i++) {
        size_step *= 0.5;
        if depth >= textureSampleGrad(material_map_normal, material_map_sampler,
            start + ds*depth, dx, dy).a {
            best = depth;
            depth -= 2.0*size_step;
        }
        depth += size_step;
    }
    let before = textureSampleGrad(material_map_normal, material_map_sampler,
        start + ds*(depth - size_step), dx, dy).a - depth + size_step;
    let after = textureSampleGrad(material_map_normal, material_map_sampler,
        start + ds*depth, dx, dy).a - depth;
    let delta = before - after;
    best += select(0.0, before/delta, delta > 0.0)*size_step;
    return ds*(best - bias);
}

fn material_map_remember_albedo(primary: vec4<f32>, secondary: vec4<f32>) {
    material_map_albedo = select(primary.rgb, secondary.rgb, (material_map_flags() & 4u) != 0u);
}

struct MaterialMapResponse {
    specular: vec3<f32>,
    roughness: f32,
    occlusion: f32,
    metalness: f32,
};
// rend2's two specular paths (`lightall.glsl` main): spec/gloss scales the colour and
// turns gloss into roughness; packed maps (ORMS after the swizzle, `ORMS *= scale.zwxy`)
// take the albedo as metal colour.
fn material_map_response() -> MaterialMapResponse {
    let texel = material_map_surface.specular;
    let scale = material_map.specular_scale;
    if material_map_layout() == 1u {
        return MaterialMapResponse(texel.rgb*scale.xyz,
            material_map_filtered(mix(1.0, 0.01, texel.a*(1.0 - scale.w))), 1.0, 0.0);
    }
    let orms = texel*scale.zwxy;
    return MaterialMapResponse(mix(vec3(0.08*orms.w), material_map_albedo, orms.z),
        material_map_filtered(mix(0.01, 1.0, orms.y)), orms.x, orms.z);
}

// `roughness` widened by the pixel's spread of mapped normals (`material_map_kernel`).
fn material_map_filtered(roughness: f32) -> f32 {
    return min(sqrt(roughness*roughness + material_map_kernel), 1.0);
}

// rend2 `CalcSpecular`: GGX distribution, joint Smith visibility, its Schlick variant.
fn material_map_brdf(normal: vec3<f32>, light: vec3<f32>, view: vec3<f32>,
    specular: vec3<f32>, roughness: f32) -> vec3<f32> {
    let half = normalize(light + view);
    let nl = clamp(dot(normal, light), 0.0, 1.0);
    let ne = abs(dot(normal, view)) + 1e-5;
    let nh = clamp(dot(normal, half), 0.0, 1.0);
    let vh = clamp(dot(view, half), 0.0, 1.0);
    let fc = pow(1.0 - vh, 5.0);
    let fresnel = clamp(50.0*specular.g, 0.0, 1.0)*fc + (1.0 - fc)*specular;
    let a2 = roughness*roughness;
    let d = (nh*a2 - nh)*nh + 1.0;
    let distribution = a2/(3.14159265*d*d);
    let visibility = 0.5/(nl*(ne*(1.0 - roughness) + roughness)
        + ne*(nl*(1.0 - roughness) + roughness));
    return distribution*fresnel*visibility;
}

// Diffuse light for the albedo product from `direct` (irradiance on a surface facing
// `light`, received with `facing`) and `ambient`; with a specular map, its highlight goes
// to `material_map_highlight` and metal loses its diffuse share, as in rend2.
fn material_map_shade(world: vec3<f32>, direct: vec3<f32>, light: vec3<f32>, facing: f32,
    ambient: vec3<f32>) -> vec3<f32> {
    if material_map_layout() == 0u { return direct*facing + ambient; }
    let response = material_map_response();
    let view = normalize(camera.camera_position - world);
    // Lambert divides by pi where SJK's light units do not: the highlight is scaled by pi.
    material_map_highlight += direct*facing*3.14159265*material_map_brdf(
        material_map_surface.normal, light, view, response.specular, response.roughness);
    return (direct*facing + ambient*response.occlusion)*(1.0 - response.metalness);
}

// Diffuse light from a light of irradiance `direct` on a surface facing `light`,
// received with `facing`; its highlight, scaled by `highlight`, goes to
// `material_map_highlight`. Metal loses the diffuse share, as in `material_map_shade`.
fn material_map_shade_light(world: vec3<f32>, direct: vec3<f32>, light: vec3<f32>,
    facing: f32, highlight: f32) -> vec3<f32> {
    if material_map_layout() == 0u { return direct*facing; }
    let response = material_map_response();
    if highlight > 0.0 {
        let view = normalize(camera.camera_position - world);
        material_map_highlight += direct*facing*highlight*3.14159265*material_map_brdf(
            material_map_surface.normal, light, view, response.specular, response.roughness);
    }
    return direct*facing*(1.0 - response.metalness);
}

// A lightmap texel under a material map: rend2's lightmap response (`lightall.glsl`,
// USE_LIGHTMAP) with the light grid's direction standing in for a deluxemap. The baked
// light is taken as arriving along that direction, divided by the face's own cosine
// (at most 4x) and received by the mapped normal; what the face did not receive stays
// ambient. A flat map reproduces the texel exactly.
fn material_map_baked(input: VertexOutput, texel: vec4<f32>) -> vec4<f32> {
    let surface = material_map_surface;
    let grid = input.material_light.xyz;
    let length_squared = dot(grid, grid);
    let light = select(surface.geometric, grid*inverseSqrt(max(length_squared, 1e-12)),
        length_squared > 1e-6);
    let received = clamp(dot(surface.geometric, light), 0.0, 1.0);
    let direct = texel.rgb/max(received, 0.25);
    let ambient = max(texel.rgb - direct*received, vec3(0.0));
    let facing = clamp(dot(surface.normal, light), 0.0, 1.0);
    let lit = material_map_shade(input.world_position, direct, light, facing, ambient);
    if material_map_layout() != 0u {
        material_map_highlight += material_map_reflection(input.world_position,
            material_map_response()).rgb;
    }
    return vec4(lit, texel.a);
}

// Vertex-lit paint (`rgbGen vertex`/`exactVertex` on a `LIGHTMAP_BY_VERTEX` surface): its
// vertex colour is the baked light, as a lightmapped surface's lightmap texel is, so the
// same response turns it toward the mapped normal (rend2 lights vertex-lit surfaces
// through the same path). Real-time lighting reads the light buffer instead, through
// `material_map_lightmap` in `apply_lighting_mode`; models and fullbright pass through.
fn material_map_vertex_light(input: VertexOutput, color: vec4<f32>, flags: u32,
    fullbright: bool) -> vec4<f32> {
    let rgb = i32(stage.generators.x);
    if fullbright || realtime_active() || (flags & 1u) == 0u || !(rgb == 2 || rgb == 3)
        || input.entity_control.x > 0.5 || input.light_ambient.a != 0.0 {
        return color;
    }
    return vec4(material_map_baked(input, vec4(color.rgb, 1.0)).rgb, color.a);
}

// Highlights of the dynamic lights the stage's modulation pass adds (rend2
// `CalcDynamicLightContribution`), with the existing radial falloff.
fn material_map_point_highlights(input: VertexOutput) {
    // Fullbright (lighting mode bit 1) shows no lighting at all.
    if material_map_layout() == 0u || (point_lights.metadata.z & 1u) != 0u { return; }
    let response = material_map_response();
    let view = normalize(camera.camera_position - input.world_position);
    var remaining = fragment_point_mask;
    if realtime_active() { remaining &= ~point_lights.metadata.w; }
    while remaining != 0u {
        let index = firstTrailingBit(remaining);
        remaining &= remaining - 1u;
        let light = point_lights.lights[index];
        if light.color.w > 0.0 && realtime_active() { continue; }
        let delta = light.origin_radius.xyz - input.world_position;
        let distance = length(delta);
        if distance >= light.origin_radius.w || distance <= 0.0001 { continue; }
        let direction = delta/distance;
        let facing = max(dot(material_map_surface.normal, direction), 0.0);
        material_map_highlight += light.color.rgb*(1.0 - distance/light.origin_radius.w)*facing
            *3.14159265*material_map_brdf(material_map_surface.normal, direction, view,
                response.specular, response.roughness);
    }
}

// The emission map's light: unlit, so neither the lightmap nor shadows dim it, scaled by
// `r_emissionStrength` (lighting-mode bits 24-31, `world_lighting_mode.rs`, where zero
// bits are strength 1). The fullbright and lightmap views (mode bits 0-1) show none.
fn material_map_emitted() -> vec3<f32> {
    let mode = point_lights.metadata.z;
    if (mode & 3u) != 0u { return vec3(0.0); }
    let strength = f32((((mode >> 24u) + 32u) & 255u))/32.0;
    return material_map_surface.emission*strength;
}

// The dynamic glow pass (`material_map_program::glow_source`): a stage drawn there only for
// its emission map writes the emission, so only the emitting texels get a halo; a stage
// with an authored `glow` keeps its colour, as stock draws glowing stages.
fn material_map_glow(output: vec4<f32>) -> vec4<f32> {
    if (material_map_flags() & 32u) != 0u { return vec4(material_map_emitted(), output.a); }
    return output;
}

// The stage's final colour: the highlights and the emission added after the albedo
// product and dynamic-light modulation, or one of the `r_materialMapsDebug` views
// (lighting-mode bits 8-10, `material_maps::DEBUG_SHIFT`). Only material-mapped stages run
// this program, so the views leave every other surface as it is. A uniform branch:
// nothing to pay when off.
fn material_map_finish(output: vec4<f32>) -> vec4<f32> {
    let lit = vec4(output.rgb + material_map_highlight + material_map_emitted(), output.a);
    let view = (point_lights.metadata.z >> 8u) & 7u;
    // 5: the scene without probe reflections (`material_map_reflection`).
    if view == 0u || view == 5u { return lit; }
    if view == 6u {
        // The emission map alone (before `r_emissionStrength`), black without one.
        return vec4(material_map_surface.emission, lit.a);
    }
    if view == 4u {
        // The probe reflection alone, black where no captured probe serves the surface.
        return vec4(material_map_reflected, lit.a);
    }
    let surface = material_map_surface;
    if view == 1u {
        // The mapped normal in world space, as an object-space normal map shows it.
        return vec4(surface.normal*0.5 + 0.5, lit.a);
    }
    if view == 2u {
        // Which maps the stage found: red parallax, green normal, blue specular.
        let flags = material_map_flags();
        let found = vec3(f32((flags & 2u) != 0u), f32((flags & 1u) != 0u),
            f32(material_map_layout() != 0u));
        return vec4(mix(lit.rgb, found, 0.6), lit.a);
    }
    // The normal map's relief: the mapped normal's departure from the face, four times
    // over, on grey. A flat (or missing) normal map stays uniformly grey.
    return vec4(clamp(vec3(0.5) + 4.0*(surface.normal - surface.geometric), vec3(0.0),
        vec3(1.0)), lit.a);
}
