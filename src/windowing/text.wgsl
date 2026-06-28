struct TextInstance {
    pos: vec2<f32>,
    size: vec2<f32>,
    uv_pos: vec2<f32>,
    uv_size: vec2<f32>,
    color: vec4<f32>,
}

struct TextInstances {
    instances: array<TextInstance>,
}

@group(0) @binding(0) var<storage, read> instance_data: TextInstances;
@group(0) @binding(1) var t_atlas: texture_2d<f32>;
@group(0) @binding(2) var s_atlas: sampler;

struct PushConstants {
    screen_size: vec2<f32>,
}
var<immediate> pc: PushConstants;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    let instance = instance_data.instances[in_instance_index];

    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );
    let pos = positions[in_vertex_index];

    let screen_pos = instance.pos + pos * instance.size;
    let clip_pos = vec2<f32>(screen_pos.x / pc.screen_size.x * 2.0 - 1.0, 1.0 - screen_pos.y / pc.screen_size.y * 2.0);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = instance.uv_pos + pos * instance.uv_size;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(t_atlas, s_atlas, in.uv).r;
    var out_c = in.color;
    out_c.a = out_c.a * alpha;
    return out_c;
}
