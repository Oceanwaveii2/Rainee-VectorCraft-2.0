// Original VectorCraft effects pilot, MIT OR Apache-2.0.
// Values are premultiplied in the caller's working color space. No implicit gamma conversion.
struct Params {
    width: u32,
    height: u32,
    radius: u32,
    axis: u32,
}
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var target: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<storage, read> weights: array<f32>;

@compute @workgroup_size(8, 8)
fn blur(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) { return; }
    let origin = vec2<i32>(id.xy);
    let axis = select(vec2<i32>(0, 1), vec2<i32>(1, 0), params.axis == 0u);
    var sum = vec4<f32>(0.0);
    let radius = i32(params.radius);
    for (var offset = -radius; offset <= radius; offset += 1) {
        let p = origin + offset * axis;
        // Transparent outside the supplied padded effect tile, not edge-clamped color.
        if (p.x >= 0 && p.y >= 0 && p.x < i32(params.width) && p.y < i32(params.height)) {
            sum += textureLoad(source, p, 0) * weights[u32(offset + radius)];
        }
    }
    textureStore(target, origin, sum);
}
