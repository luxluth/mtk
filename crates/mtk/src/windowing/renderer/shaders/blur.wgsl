struct BlurParams {
    src_size: vec2<f32>,
    dst_size: vec2<f32>,
    offset: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
var<immediate> params: BlurParams;

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var linear_sampler: sampler;

@compute @workgroup_size(8, 8)
fn cs_downsample(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u32(params.dst_size.x) || id.y >= u32(params.dst_size.y)) {
        return;
    }
    let uv = (vec2<f32>(id.xy) + 0.5) / params.dst_size;
    let half_pixel = (vec2<f32>(params.offset) + 0.5) / params.src_size;

    var color = textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(-half_pixel.x, -half_pixel.y), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>( half_pixel.x, -half_pixel.y), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(-half_pixel.x,  half_pixel.y), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>( half_pixel.x,  half_pixel.y), 0.0);

    textureStore(dst_tex, vec2<i32>(id.xy), color * 0.25);
}

@compute @workgroup_size(8, 8)
fn cs_upsample(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u32(params.dst_size.x) || id.y >= u32(params.dst_size.y)) {
        return;
    }
    let uv = (vec2<f32>(id.xy) + 0.5) / params.dst_size;
    let half_pixel = (vec2<f32>(params.offset) + 0.5) / params.src_size;

    var color = textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(-half_pixel.x * 2.0, 0.0), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>( half_pixel.x * 2.0, 0.0), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(0.0, -half_pixel.y * 2.0), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(0.0,  half_pixel.y * 2.0), 0.0);
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(-half_pixel.x, -half_pixel.y), 0.0) * 2.0;
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>( half_pixel.x, -half_pixel.y), 0.0) * 2.0;
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>(-half_pixel.x,  half_pixel.y), 0.0) * 2.0;
    color += textureSampleLevel(src_tex, linear_sampler, uv + vec2<f32>( half_pixel.x,  half_pixel.y), 0.0) * 2.0;

    textureStore(dst_tex, vec2<i32>(id.xy), color / 12.0);
}
