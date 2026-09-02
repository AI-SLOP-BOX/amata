// GPU Vector Path Rendering Shader
// Handles: path fill/stroke, glow, blur, shadow effects

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    zoom: f32,
    pan: vec2<f32>,
    glow_radius: f32,
    glow_intensity: f32,
    blur_radius: f32,
    shadow_offset: vec2<f32>,
    shadow_blur: f32,
    shadow_opacity: f32,
    opacity: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Apply zoom and pan
    let world = (in.position + uniforms.pan) * uniforms.zoom;
    // Convert to clip space (-1 to 1)
    let clip = vec2<f32>(
        (world.x / uniforms.resolution.x) * 2.0 - 1.0,
        1.0 - (world.y / uniforms.resolution.y) * 2.0
    );
    out.clip_position = vec4<f32>(clip, 0.0, 1.0);
    out.color = in.color;
    out.tex_coord = in.tex_coord;
    out.world_pos = in.position;
    return out;
}

// Gaussian blur helper
fn gaussian(x: f32, sigma: f32) -> f32 {
    let coeff = 1.0 / (sqrt(2.0 * 3.14159265) * sigma);
    let exp_val = exp(-(x * x) / (2.0 * sigma * sigma));
    return coeff * exp_val;
}

// Signed distance from point to line segment
fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

// Glow effect: accumulate glow from nearby path edges
fn compute_glow(world_pos: vec2<f32>, path_points: array<vec2<f32>, 64>, point_count: u32) -> f32 {
    var glow: f32 = 0.0;
    let radius = uniforms.glow_radius;
    for (var i: u32 = 0u; i < point_count - 1u; i = i + 1u) {
        let a = path_points[i];
        let b = path_points[i + 1u];
        let dist = sd_segment(world_pos, a, b);
        if dist < radius {
            glow = glow + (1.0 - dist / radius);
        }
    }
    return glow;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

    // Apply base opacity
    color.a = color.a * uniforms.opacity;

    // Glow effect (if enabled)
    if uniforms.glow_intensity > 0.0 {
        // Glow is computed from the vertex color alpha as a proxy
        let glow_factor = color.a * uniforms.glow_intensity;
        let glow_color = vec4<f32>(color.rgb, glow_factor * 0.5);
        color = mix(color, glow_color, uniforms.glow_intensity * 0.3);
    }

    // Shadow offset
    if uniforms.shadow_opacity > 0.0 {
        let shadow_alpha = uniforms.shadow_opacity * color.a * 0.5;
        // Shadow is darker version shifted by offset
        let shadow_color = vec4<f32>(0.0, 0.0, 0.0, shadow_alpha);
        // Simple shadow: blend based on distance from edge
        let edge_dist = abs(in.tex_coord.x - 0.5) + abs(in.tex_coord.y - 0.5);
        let shadow_blend = smoothstep(0.5, 0.5 + uniforms.shadow_blur * 0.01, edge_dist);
        color = mix(shadow_color, color, shadow_blend);
    }

    // Blur effect (simple box blur approximation via alpha)
    if uniforms.blur_radius > 0.0 {
        let blur_amount = uniforms.blur_radius * 0.01;
        let edge_dist = min(
            min(in.tex_coord.x, 1.0 - in.tex_coord.x),
            min(in.tex_coord.y, 1.0 - in.tex_coord.y)
        );
        color.a = color.a * smoothstep(0.0, blur_amount, edge_dist);
    }

    // Premultiplied alpha output
    return vec4<f32>(color.rgb * color.a, color.a);
}

// Compute shader for Gaussian blur post-processing
@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var output_texture: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2)
var<uniform> blur_uniforms: Uniforms;

@compute @workgroup_size(8, 8)
fn cs_blur(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    if id.x >= dims.x || id.y >= dims.y {
        return;
    }

    let sigma = blur_uniforms.blur_radius;
    let radius = u32(ceil(sigma * 2.0));
    var total_color = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var dx: i32 = -i32(radius); dx <= i32(radius); dx = dx + 1) {
        for (var dy: i32 = -i32(radius); dy <= i32(radius); dy = dy + 1) {
            let sample_pos = vec2<i32>(i32(id.x) + dx, i32(id.y) + dy);
            if sample_pos.x >= 0 && sample_pos.x < i32(dims.x)
                && sample_pos.y >= 0 && sample_pos.y < i32(dims.y) {
                let weight = gaussian(f32(dx), sigma) * gaussian(f32(dy), sigma);
                let sample = textureLoad(input_texture, vec2<u32>(u32(sample_pos.x), u32(sample_pos.y)), 0);
                total_color = total_color + sample * f32(weight);
                total_weight = total_weight + f32(weight);
            }
        }
    }

    if total_weight > 0.0 {
        total_color = total_color / f32(total_weight);
    }

    textureStore(output_texture, id.xy, total_color);
}
