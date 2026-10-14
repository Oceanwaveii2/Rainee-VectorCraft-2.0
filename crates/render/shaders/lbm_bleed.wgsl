struct SimParams {
    viscosity: f32,
    evaporation_rate: f32,
    capillary_strength: f32,
    delta_time: f32,
    edge_pooling_factor: f32,
    surface_tension: f32,
    porosity_coupling: f32,
    particle_count: u32,
};

struct BrushParticle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    radius: f32,
    wetness: f32,
    pigment_rgba: vec4<f32>,
    splatter_seed: u32,
    _pad: vec3<u32>,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var t_paper_grain: texture_2d<f32>;
@group(0) @binding(2) var t_fluid_in: texture_2d<f32>;
@group(0) @binding(3) var t_pigment_in: texture_2d<f32>;
@group(0) @binding(4) var t_fluid_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var t_pigment_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(6) var<storage, read> particles: array<BrushParticle>;

@compute @workgroup_size(16, 16)
fn cs_simulate_bleed(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = vec2<i32>(id.xy);
    let dims = vec2<i32>(textureDimensions(t_fluid_in));
    if (coords.x >= dims.x || coords.y >= dims.y) {
        return;
    }

    let c_max = dims - vec2<i32>(1, 1);
    let center_fluid = textureLoad(t_fluid_in, coords, 0);
    let center_pigment = textureLoad(t_pigment_in, coords, 0);
    let paper = textureLoad(t_paper_grain, coords, 0).r;

    let c_n = clamp(coords + vec2<i32>(0, 1), vec2<i32>(0), c_max);
    let c_s = clamp(coords + vec2<i32>(0, -1), vec2<i32>(0), c_max);
    let c_e = clamp(coords + vec2<i32>(1, 0), vec2<i32>(0), c_max);
    let c_w = clamp(coords + vec2<i32>(-1, 0), vec2<i32>(0), c_max);

    let north = textureLoad(t_fluid_in, c_n, 0);
    let south = textureLoad(t_fluid_in, c_s, 0);
    let east  = textureLoad(t_fluid_in, c_e, 0);
    let west  = textureLoad(t_fluid_in, c_w, 0);

    let laplacian_h = 0.25 * (north.z + south.z + east.z + west.z) - center_fluid.z;
    let permeability = mix(1.0, 1.0 - paper, params.porosity_coupling);
    let fiber_pull = params.capillary_strength * (1.0 - paper) * permeability;

    var new_h = center_fluid.z + (laplacian_h * params.viscosity * permeability + fiber_pull * 0.05) * params.delta_time;
    new_h = max(0.0, new_h - params.evaporation_rate * params.delta_time);

    let p_n = textureLoad(t_pigment_in, c_n, 0);
    let p_s = textureLoad(t_pigment_in, c_s, 0);
    let p_e = textureLoad(t_pigment_in, c_e, 0);
    let p_w = textureLoad(t_pigment_in, c_w, 0);
    let laplacian_p = (p_n + p_s + p_e + p_w) - (4.0 * center_pigment);

    var new_pigment = center_pigment;
    if (new_h > 0.001) {
        let diffusion_step = laplacian_p * (center_fluid.z * 0.12) * params.delta_time;
        let grad_h = vec2<f32>(east.z - west.z, north.z - south.z) * 0.5;
        let capillary_ring_transfer = center_pigment * ((laplacian_h * 1.8 + length(grad_h) * 2.2) * params.edge_pooling_factor) * params.delta_time;
        new_pigment = clamp(new_pigment + diffusion_step + capillary_ring_transfer, vec4<f32>(0.0), vec4<f32>(4.0));
    }

    let damped_vel = center_fluid.xy * max(0.0, 1.0 - 0.15 * params.delta_time);
    textureStore(t_fluid_out, coords, vec4<f32>(damped_vel, new_h, permeability));
    textureStore(t_pigment_out, coords, new_pigment);
}
