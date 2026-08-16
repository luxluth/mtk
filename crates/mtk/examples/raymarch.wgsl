struct Uniforms {
    time: f32,
    aspect: f32,
    mouse_x: f32,
    mouse_y: f32,
    resolution_x: f32,
    resolution_y: f32,
    mouse_pressed: f32,
    _pad: f32,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );
    let pos = positions[vertex_index];
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos;
    return out;
}

fn rotateX(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
}

fn rotateY(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

fn rotateZ(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
}

fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdTorus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

fn map(p: vec3<f32>) -> f32 {
    let t = uniforms.time * 0.7;
    var q = rotateY(p, t * 0.4);
    q = rotateX(q, t * 0.3);

    // Inner pulsating core
    let d_core = sdSphere(p, 0.45 + 0.06 * sin(uniforms.time * 3.0));

    // Outer lattice gyroid
    let scale = 3.6;
    let gyroid = abs(dot(sin(q * scale), cos(q.zxy * scale))) / scale - 0.04;
    let bounding_sphere = sdSphere(p, 1.4);
    let d_lattice = max(gyroid, bounding_sphere);

    // Outer orbiting cyber rings
    let ring_p1 = rotateX(rotateY(p, t * 0.6), 1.1);
    let d_ring1 = sdTorus(ring_p1, vec2<f32>(1.8, 0.03));

    let ring_p2 = rotateZ(rotateY(p, -t * 0.5), 0.9);
    let d_ring2 = sdTorus(ring_p2, vec2<f32>(1.6, 0.025));

    return min(min(d_lattice, d_core), min(d_ring1, d_ring2));
}

fn calcNormal(p: vec3<f32>) -> vec3<f32> {
    let eps = 0.002;
    let e = vec2<f32>(eps, -eps);
    return normalize(
        e.xyy * map(p + e.xyy) +
        e.yyx * map(p + e.yyx) +
        e.yxy * map(p + e.yxy) +
        e.xxx * map(p + e.xxx)
    );
}

fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.5, 0.5, 0.5);
    let b = vec3<f32>(0.5, 0.5, 0.5);
    let c = vec3<f32>(1.0, 1.0, 1.0);
    let d = vec3<f32>(0.0, 0.33, 0.67);
    return a + b * cos(6.28318 * (c * t + d));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    uv.x = uv.x * uniforms.aspect;

    // Interactive mouse orbiting
    let rot_y = (uniforms.mouse_x - 0.5) * 4.0;
    let rot_x = (uniforms.mouse_y - 0.5) * 3.0;

    var ro = vec3<f32>(0.0, 0.0, 3.6);
    ro = rotateX(ro, rot_x);
    ro = rotateY(ro, rot_y);

    let ta = vec3<f32>(0.0, 0.0, 0.0);
    let ww = normalize(ta - ro);
    let uu = normalize(cross(ww, vec3<f32>(0.0, 1.0, 0.0)));
    let vv = normalize(cross(uu, ww));
    let rd = normalize(uv.x * uu + uv.y * vv + 1.6 * ww);

    var dist = 0.0;
    var glow = 0.0;
    var hit = false;
    var p = ro;

    for (var i = 0; i < 90; i++) {
        p = ro + rd * dist;
        let d = map(p);
        glow += 0.016 / (0.01 + abs(d) * abs(d) * 45.0);

        if d < 0.001 {
            hit = true;
            break;
        }
        if dist > 10.0 {
            break;
        }
        dist += d * 0.8;
    }

    var col = vec3<f32>(0.015, 0.015, 0.035);

    // Subtle background ambient gradient
    let bg_grad = 0.15 * (1.0 - length(in.uv) * 0.6);
    col += vec3<f32>(0.04, 0.08, 0.2) * bg_grad;

    if hit {
        let nor = calcNormal(p);
        let light_dir = normalize(vec3<f32>(1.5, 2.0, 2.5));
        let diff = max(dot(nor, light_dir), 0.0);

        let half_dir = normalize(light_dir - rd);
        let spec = pow(max(dot(nor, half_dir), 0.0), 32.0);
        let fresnel = pow(1.0 - max(dot(-rd, nor), 0.0), 3.0);

        let base_col = palette(length(p) * 0.5 + uniforms.time * 0.2);
        col = base_col * (diff * 0.8 + 0.2) + vec3<f32>(1.0, 1.0, 1.0) * spec * 0.8 + base_col * fresnel * 1.5;
    }

    // Add neon volumetric glow
    let glow_col = palette(uniforms.time * 0.12 + 0.5);
    col += glow_col * glow * 0.14;

    // Tonemapping & gamma correction
    col = col / (col + vec3<f32>(1.0));
    col = pow(col, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(col, 1.0);
}
