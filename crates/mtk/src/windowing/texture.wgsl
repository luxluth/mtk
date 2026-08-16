struct ImmediateData {
    color: vec4<f32>,
    pos: vec2<f32>,
    screen_size: vec2<f32>,
    quad_size: vec2<f32>,
    src_offset: vec2<f32>,
    src_size: vec2<f32>,
    border_radius: f32,
    alpha: f32,
    shadow_spread: f32,
    shadow_power: f32,
    vibrancy: f32,
    vibrancy_darkness: f32,
    passes: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    border_widths: vec4<f32>, // top, right, bottom, left
    border_color: vec4<f32>,
}
var<immediate> imm: ImmediateData;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragP: vec2<f32>,
    @location(1) fragQuadSize: vec2<f32>,
    @location(2) fragBorderRadius: f32,
    @location(3) fragAlpha: f32,
    @location(4) uv: vec2<f32>,
}

@group(0) @binding(0) var t_texture: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    let logical_center = imm.pos + imm.quad_size * 0.5;
    let pos_uv = positions[in_vertex_index];
    let physical_p = (pos_uv * imm.quad_size) + imm.pos;

    let ndc_x = (physical_p.x / imm.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (physical_p.y / imm.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.fragP = physical_p - logical_center;
    out.fragQuadSize = imm.quad_size;
    out.fragBorderRadius = imm.border_radius;
    out.fragAlpha = imm.alpha;
    out.uv = pos_uv;
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
    let outer_alpha = 1.0 - smoothstep(-0.75, 0.75, dist);

    var sampled = textureSample(t_texture, s_sampler, in.uv);
    sampled = sampled * imm.color;

    let has_border = imm.border_widths.x > 0.0 || imm.border_widths.y > 0.0 || imm.border_widths.z > 0.0 || imm.border_widths.w > 0.0;
    var outColor = sampled;

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

    outColor.a = outColor.a * in.fragAlpha;
    return outColor;
}
