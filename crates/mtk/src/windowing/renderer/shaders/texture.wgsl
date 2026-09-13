struct ImmediateData {
    color: vec4<f32>,
    pos: vec2<f32>,
    screen_size: vec2<f32>,
    quad_size: vec2<f32>,
    alpha: f32,
    _pad0: f32,
    border_radii: vec4<f32>, // tl, tr, br, bl
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
    border_widths: vec4<f32>, // top, right, bottom, left
    shadow_offset: vec2<f32>,
    shadow_blur: f32,
    shadow_spread: f32,
    shadow_inset: f32,
    vibrancy: f32,
    vibrancy_darkness: f32,
    passes: f32,
}
var<immediate> imm: ImmediateData;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragP: vec2<f32>,
    @location(1) fragQuadSize: vec2<f32>,
    @location(2) fragBorderRadii: vec4<f32>,
    @location(3) fragAlpha: f32,
}

@group(0) @binding(0) var t_texture: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    let offset = imm.shadow_offset;
    let blur_radius = imm.shadow_blur;
    let spread_radius = imm.shadow_spread;
    let is_inset = imm.shadow_inset > 0.5;
    let has_shadow = imm.shadow_color.a > 0.0 && (blur_radius > 0.0 || spread_radius != 0.0 || length(offset) > 0.0);

    var expand_left = 2.0;
    var expand_right = 2.0;
    var expand_top = 2.0;
    var expand_bottom = 2.0;

    if has_shadow && !is_inset {
        let blur_ext = blur_radius * 2.5;
        let spread_ext = max(0.0, spread_radius);
        expand_left = max(2.0, max(0.0, -offset.x) + spread_ext + blur_ext);
        expand_right = max(2.0, max(0.0, offset.x) + spread_ext + blur_ext);
        expand_top = max(2.0, max(0.0, -offset.y) + spread_ext + blur_ext);
        expand_bottom = max(2.0, max(0.0, offset.y) + spread_ext + blur_ext);
    }

    let unit_pos = positions[in_vertex_index];
    let physical_p = vec2<f32>(
        mix(imm.pos.x - expand_left, imm.pos.x + imm.quad_size.x + expand_right, unit_pos.x),
        mix(imm.pos.y - expand_top, imm.pos.y + imm.quad_size.y + expand_bottom, unit_pos.y)
    );

    let logical_center = imm.pos + imm.quad_size * 0.5;
    let ndc_x = (physical_p.x / imm.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (physical_p.y / imm.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.fragP = physical_p - logical_center;
    out.fragQuadSize = imm.quad_size;
    out.fragBorderRadii = imm.border_radii;
    out.fragAlpha = imm.alpha;
    return out;
}

fn sdRoundedBox(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    let rad = select(vec2<f32>(r.x, r.w), vec2<f32>(r.y, r.z), p.x > 0.0);
    let radius = min(select(rad.x, rad.y, p.y > 0.0), min(b.x, b.y));
    let q = abs(p) - b + vec2<f32>(radius, radius);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.fragP;
    let b = in.fragQuadSize * 0.5;
    let dist = sdRoundedBox(p, b, in.fragBorderRadii);
    let outer_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, dist), 0.0, 1.0);

    let img_uv = clamp((p + b) / in.fragQuadSize, vec2<f32>(0.0), vec2<f32>(1.0));
    var sampled = textureSample(t_texture, s_sampler, img_uv);
    sampled = sampled * imm.color;

    let has_border = imm.border_widths.x > 0.0 || imm.border_widths.y > 0.0 || imm.border_widths.z > 0.0 || imm.border_widths.w > 0.0;
    var outColor = sampled;

    if has_border {
        let inner_b = b - vec2<f32>(imm.border_widths.w + imm.border_widths.y, imm.border_widths.x + imm.border_widths.z) * 0.5;
        let inner_offset = vec2<f32>(imm.border_widths.w - imm.border_widths.y, imm.border_widths.x - imm.border_widths.z) * 0.5;
        let p_inner = p - inner_offset;
        let min_border = min(min(imm.border_widths.x, imm.border_widths.y), min(imm.border_widths.z, imm.border_widths.w));
        let inner_radii = max(vec4<f32>(0.0), in.fragBorderRadii - vec4<f32>(min_border));
        let inner_dist = sdRoundedBox(p_inner, inner_b, inner_radii);
        let inner_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, inner_dist), 0.0, 1.0);

        let c1 = min(inner_alpha, outer_alpha);
        let c2 = max(0.0, outer_alpha - inner_alpha);

        let bg_a = sampled.a * c1;
        let bd_a = imm.border_color.a * c2;
        let total_a = bg_a + bd_a;

        if total_a > 0.0 {
            let rgb = (sampled.rgb * bg_a + imm.border_color.rgb * bd_a) / total_a;
            outColor = vec4<f32>(rgb, total_a);
        } else {
            outColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    } else {
        outColor.a = outColor.a * outer_alpha;
    }

    let offset = imm.shadow_offset;
    let blur_radius = imm.shadow_blur;
    let spread_radius = imm.shadow_spread;
    let is_inset = imm.shadow_inset > 0.5;

    // Box Shadow computation (Outer Drop Shadow and Inner Inset Shadow)
    var shadowColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if imm.shadow_color.a > 0.0 {
        if is_inset {
            // Inset Shadow calculation
            let inner_b = b - vec2<f32>(imm.border_widths.w + imm.border_widths.y, imm.border_widths.x + imm.border_widths.z) * 0.5;
            let inner_offset = vec2<f32>(imm.border_widths.w - imm.border_widths.y, imm.border_widths.x - imm.border_widths.z) * 0.5;
            let p_inner = p - inner_offset;
            let min_border = min(min(imm.border_widths.x, imm.border_widths.y), min(imm.border_widths.z, imm.border_widths.w));
            let inner_radii = max(vec4<f32>(0.0), in.fragBorderRadii - vec4<f32>(min_border));

            let p_shadow = p_inner - offset;
            let dist_inner = sdRoundedBox(p_shadow, inner_b, inner_radii);
            let d_inward = -dist_inner - spread_radius;

            var s_alpha = 0.0;
            if blur_radius > 0.5 {
                s_alpha = clamp(1.0 - smoothstep(0.0, blur_radius * 1.5, d_inward), 0.0, 1.0);
            } else {
                s_alpha = select(0.0, 1.0, d_inward <= 0.0);
            }

            let mask = clamp(1.0 - smoothstep(-0.75, 0.75, dist_inner), 0.0, 1.0);
            s_alpha = s_alpha * imm.shadow_color.a * mask;
            shadowColor = vec4<f32>(imm.shadow_color.rgb, s_alpha);

            // Composite inset shadow over outColor background
            let tinted_rgb = mix(outColor.rgb, shadowColor.rgb, shadowColor.a);
            outColor = vec4<f32>(tinted_rgb, outColor.a);
        } else {
            // Outer Drop Shadow calculation
            let p_shadow = p - offset;
            let b_shadow = b;
            let r_shadow = max(vec4<f32>(0.0), in.fragBorderRadii + vec4<f32>(spread_radius));
            let dist_shadow = sdRoundedBox(p_shadow, b_shadow, r_shadow) - spread_radius;

            var s_alpha = 0.0;
            if blur_radius > 0.5 {
                let sigma = max(0.5, blur_radius * 0.5);
                let d = max(0.0, dist_shadow);
                let falloff = exp(-0.5 * (d * d) / (sigma * sigma));
                s_alpha = select(falloff, 1.0, dist_shadow <= 0.0);
            } else {
                s_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, dist_shadow), 0.0, 1.0);
            }

            s_alpha = s_alpha * imm.shadow_color.a;
            shadowColor = vec4<f32>(imm.shadow_color.rgb, s_alpha);
        }
    }

    var final_rgb = outColor.rgb * outColor.a;
    var final_a = outColor.a;

    if !is_inset && shadowColor.a > 0.0 {
        final_rgb = outColor.rgb * outColor.a + shadowColor.rgb * shadowColor.a * (1.0 - outColor.a);
        final_a = outColor.a + shadowColor.a * (1.0 - outColor.a);
    }

    var finalColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if final_a > 0.001 {
        finalColor = vec4<f32>(final_rgb / final_a, final_a);
    }

    finalColor.a = finalColor.a * in.fragAlpha;
    return finalColor;
}
