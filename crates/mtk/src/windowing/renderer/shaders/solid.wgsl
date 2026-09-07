struct PushConstants {
    screen_size: vec2<f32>,
}
var<immediate> pc: PushConstants;

@group(0) @binding(0) var blurred_texture: texture_2d<f32>;
@group(0) @binding(1) var blurred_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) in_vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) quad_size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radii: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) border_widths: vec4<f32>,
    @location(6) shadow_color: vec4<f32>,
    @location(7) shadow_params: vec4<f32>,
    @location(8) effects: vec4<f32>,
    @location(9) clip_rect: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragP: vec2<f32>,
    @location(1) fragQuadSize: vec2<f32>,
    @location(2) fragBorderRadii: vec4<f32>,
    @location(3) fragColor: vec4<f32>,
    @location(4) fragBorderColor: vec4<f32>,
    @location(5) fragBorderWidths: vec4<f32>,
    @location(6) fragShadowColor: vec4<f32>,
    @location(7) fragShadowParams: vec4<f32>,
    @location(8) fragEffects: vec4<f32>,
    @location(9) fragClipRect: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    let shadow_spread = in.shadow_params.x;
    let expansion = max(2.0, shadow_spread * 2.8);
    let logical_center = in.pos + in.quad_size * 0.5;

    let expand_dir = positions[in.in_vertex_index] * 2.0 - 1.0;
    let physical_p = (positions[in.in_vertex_index] * in.quad_size) + in.pos + expand_dir * expansion;

    let ndc_x = (physical_p.x / pc.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (physical_p.y / pc.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.fragP = physical_p - logical_center;
    out.fragQuadSize = in.quad_size;
    out.fragBorderRadii = in.border_radii;
    out.fragColor = in.color;
    out.fragBorderColor = in.border_color;
    out.fragBorderWidths = in.border_widths;
    out.fragShadowColor = in.shadow_color;
    out.fragShadowParams = in.shadow_params;
    out.fragEffects = in.effects;
    out.fragClipRect = in.clip_rect;
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
    if in.fragClipRect.z > 0.0 && in.fragClipRect.w > 0.0 {
        if in.clip_position.x < in.fragClipRect.x ||
            in.clip_position.x > in.fragClipRect.x + in.fragClipRect.z ||
            in.clip_position.y < in.fragClipRect.y ||
            in.clip_position.y > in.fragClipRect.y + in.fragClipRect.w {
            discard;
        }
    }

    let p = in.fragP;
    let b = in.fragQuadSize * 0.5;

    let dist = sdRoundedBox(p, b, in.fragBorderRadii);

    // Outer edge alpha (smooth anti-aliasing)
    let outer_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, dist), 0.0, 1.0);

    var base_color = in.fragColor;
    let vibrancy = in.fragEffects.x;
    let vibrancy_darkness = in.fragEffects.y;

    // Frosted Glass Vibrancy sampling
    if vibrancy > 0.0 {
        let screen_uv = in.clip_position.xy / pc.screen_size;
        let bg_sample = textureSample(blurred_texture, blurred_sampler, screen_uv);

        let luma = dot(bg_sample.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let vibrant_rgb = mix(vec3<f32>(luma), bg_sample.rgb, 1.0 + vibrancy * 0.75);
        let dark_rgb = mix(vibrant_rgb, vibrant_rgb * (1.0 - vibrancy_darkness), vibrancy);
        let tinted_rgb = mix(dark_rgb, in.fragColor.rgb, in.fragColor.a);

        base_color = vec4<f32>(tinted_rgb, 1.0);
    }

    var boxColor = base_color;

    let border_widths = in.fragBorderWidths;
    let has_border = border_widths.x > 0.0 || border_widths.y > 0.0 || border_widths.z > 0.0 || border_widths.w > 0.0;
    if has_border {
        let inner_b = b - vec2<f32>(border_widths.w + border_widths.y, border_widths.x + border_widths.z) * 0.5;
        let inner_offset = vec2<f32>(border_widths.w - border_widths.y, border_widths.x - border_widths.z) * 0.5;

        let p_inner = p - inner_offset;
        let min_border = min(min(border_widths.x, border_widths.y), min(border_widths.z, border_widths.w));
        let inner_radii = max(vec4<f32>(0.0), in.fragBorderRadii - vec4<f32>(min_border));
        let inner_dist = sdRoundedBox(p_inner, inner_b, inner_radii);

        let inner_alpha = clamp(1.0 - smoothstep(-0.75, 0.75, inner_dist), 0.0, 1.0);

        let c1 = min(inner_alpha, outer_alpha);
        let c2 = max(0.0, outer_alpha - inner_alpha);

        let bg_a = base_color.a * c1;
        let bd_a = in.fragBorderColor.a * c2;
        let total_a = bg_a + bd_a;

        if total_a > 0.0 {
            let rgb = (base_color.rgb * bg_a + in.fragBorderColor.rgb * bd_a) / total_a;
            boxColor = vec4<f32>(rgb, total_a);
        } else {
            boxColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    } else {
        boxColor.a = boxColor.a * outer_alpha;
    }

    let shadow_spread = in.fragShadowParams.x;
    let shadow_power = in.fragShadowParams.y;
    let alpha = in.fragShadowParams.z;

    // Outer Drop Shadow & Glow computation
    var shadowColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if shadow_spread > 0.0 && dist > -2.0 {
        let sigma = max(1.0, shadow_spread * 0.45);
        let d = max(0.0, dist);
        let shadow_falloff = exp(-0.5 * (d * d) / (sigma * sigma));
        let s_alpha = shadow_falloff * in.fragShadowColor.a * shadow_power;
        shadowColor = vec4<f32>(in.fragShadowColor.rgb, s_alpha);
    }

    // Composite: Box over Drop Shadow
    let final_rgb = boxColor.rgb * boxColor.a + shadowColor.rgb * shadowColor.a * (1.0 - boxColor.a);
    let final_a = boxColor.a + shadowColor.a * (1.0 - boxColor.a);

    var finalColor = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if final_a > 0.001 {
        finalColor = vec4<f32>(final_rgb / final_a, final_a);
    }

    finalColor.a = finalColor.a * alpha;
    return finalColor;
}
