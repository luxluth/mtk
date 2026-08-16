struct ImmediateData {
    color: vec4<f32>,
    pos: vec2<f32>,
    screen_size: vec2<f32>,
    quad_size: vec2<f32>,
    border_radius: f32,
    alpha: f32,
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
    border_widths: vec4<f32>, // top, right, bottom, left
    shadow_spread: f32,
    shadow_power: f32,
    vibrancy: f32,
    vibrancy_darkness: f32,
    passes: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
var<immediate> imm: ImmediateData;

@group(0) @binding(0) var blurred_texture: texture_2d<f32>;
@group(0) @binding(1) var blurred_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragP: vec2<f32>,
    @location(1) fragQuadSize: vec2<f32>,
    @location(2) fragBorderRadius: f32,
    @location(3) fragAlpha: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    let expansion = max(2.0, imm.shadow_spread * 2.8);
    let logical_center = imm.pos + imm.quad_size * 0.5;

    let expand_dir = positions[in_vertex_index] * 2.0 - 1.0;
    let physical_p = (positions[in_vertex_index] * imm.quad_size) + imm.pos + expand_dir * expansion;

    let ndc_x = (physical_p.x / imm.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (physical_p.y / imm.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.fragP = physical_p - logical_center;
    out.fragQuadSize = imm.quad_size;
    out.fragBorderRadius = imm.border_radius;
    out.fragAlpha = imm.alpha;
    return out;
}

fn sdRoundedBox(p: vec2<f32>, b: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(b.x, b.y));
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.fragP;
    let b = in.fragQuadSize * 0.5;

    let dist = sdRoundedBox(p, b, in.fragBorderRadius);

    // Outer edge alpha (smooth anti-aliasing)
    let outer_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, dist), 0.0, 1.0);

    var base_color = imm.color;

    // Frosted Glass Vibrancy sampling
    if (imm.vibrancy > 0.0) {
        let screen_uv = in.clip_position.xy / imm.screen_size;
        let bg_sample = textureSample(blurred_texture, blurred_sampler, screen_uv);

        let luma = dot(bg_sample.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let vibrant_rgb = mix(vec3<f32>(luma), bg_sample.rgb, 1.0 + imm.vibrancy * 0.75);
        let dark_rgb = mix(vibrant_rgb, vibrant_rgb * (1.0 - imm.vibrancy_darkness), imm.vibrancy);
        let tinted_rgb = mix(dark_rgb, imm.color.rgb, imm.color.a);

        base_color = vec4<f32>(tinted_rgb, 1.0);
    }

    var boxColor = base_color;

    let has_border = imm.border_widths.x > 0.0 || imm.border_widths.y > 0.0 || imm.border_widths.z > 0.0 || imm.border_widths.w > 0.0;
    if has_border {
        let inner_b = b - vec2<f32>(imm.border_widths.w + imm.border_widths.y, imm.border_widths.x + imm.border_widths.z) * 0.5;
        let inner_offset = vec2<f32>(imm.border_widths.w - imm.border_widths.y, imm.border_widths.x - imm.border_widths.z) * 0.5;

        let p_inner = p - inner_offset;
        let min_border = min(min(imm.border_widths.x, imm.border_widths.y), min(imm.border_widths.z, imm.border_widths.w));
        let inner_radius = max(0.0, in.fragBorderRadius - min_border);
        let inner_dist = sdRoundedBox(p_inner, inner_b, inner_radius);

        let inner_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, inner_dist), 0.0, 1.0);

        let c1 = min(inner_alpha, outer_alpha);
        let c2 = max(0.0, outer_alpha - inner_alpha);

        let bg_a = base_color.a * c1;
        let bd_a = imm.border_color.a * c2;
        let total_a = bg_a + bd_a;

        if total_a > 0.0 {
            let rgb = (base_color.rgb * bg_a + imm.border_color.rgb * bd_a) / total_a;
            boxColor = vec4<f32>(rgb, total_a);
        } else {
            boxColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    } else {
        boxColor.a = boxColor.a * outer_alpha;
    }

    // Outer Drop Shadow & Glow computation
    var shadowColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if (imm.shadow_spread > 0.0 && dist > -2.0) {
        let sigma = max(1.0, imm.shadow_spread * 0.45);
        let d = max(0.0, dist);
        let shadow_falloff = exp(-0.5 * (d * d) / (sigma * sigma));
        let s_alpha = shadow_falloff * imm.shadow_color.a * imm.shadow_power;
        shadowColor = vec4<f32>(imm.shadow_color.rgb, s_alpha);
    }

    // Composite: Box over Drop Shadow
    let final_rgb = boxColor.rgb * boxColor.a + shadowColor.rgb * shadowColor.a * (1.0 - boxColor.a);
    let final_a = boxColor.a + shadowColor.a * (1.0 - boxColor.a);

    var finalColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if (final_a > 0.001) {
        finalColor = vec4<f32>(final_rgb / final_a, final_a);
    }

    finalColor.a = finalColor.a * in.fragAlpha;
    return finalColor;
}
