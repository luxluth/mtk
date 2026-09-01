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

    var shadowColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if (imm.shadow_spread > 0.0 && dist > -2.0) {
        let sigma = max(1.0, imm.shadow_spread * 0.45);
        let d = max(0.0, dist);
        let shadow_falloff = exp(-0.5 * (d * d) / (sigma * sigma));
        let s_alpha = shadow_falloff * imm.shadow_color.a * imm.shadow_power;
        shadowColor = vec4<f32>(imm.shadow_color.rgb, s_alpha);
    }

    let final_rgb = outColor.rgb * outColor.a + shadowColor.rgb * shadowColor.a * (1.0 - outColor.a);
    let final_a = outColor.a + shadowColor.a * (1.0 - outColor.a);

    return vec4<f32>(final_rgb, final_a) * in.fragAlpha;
}
