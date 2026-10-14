//! Sparse Active Tile Allocator (128x128 Rgba16Float Ping-Pong Storage Tiles)
//! & Compute Fluid Simulation Harness for VectorCraft Super Brushes.

#![forbid(unsafe_code)]

use std::collections::HashMap;

pub const TILE_SIZE: u32 = 128;
pub const DRY_MOISTURE_THRESHOLD: f32 = 0.001;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SimParamsUniform {
    pub viscosity: f32,
    pub evaporation_rate: f32,
    pub capillary_strength: f32,
    pub delta_time: f32,
    pub edge_pooling_factor: f32,
    pub surface_tension: f32,
    pub porosity_coupling: f32,
    pub particle_count: u32,
}

impl Default for SimParamsUniform {
    fn default() -> Self {
        Self {
            viscosity: 0.42,
            evaporation_rate: 0.0015,
            capillary_strength: 0.88,
            delta_time: 0.05,
            edge_pooling_factor: 0.92,
            surface_tension: 0.65,
            porosity_coupling: 0.75,
            particle_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BrushParticle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub radius: f32,
    pub wetness: f32,
    pub pigment_rgba: [f32; 4],
    pub splatter_seed: u32,
    pub pad: [u32; 3],
}

#[derive(Debug, Clone)]
pub struct SimTile {
    pub coord: [i32; 2],
    pub is_wet: bool,
    pub moisture_level: f32,
    pub ping_pong_parity: bool,
    pub fluid_grid: Vec<[f32; 4]>,
    pub pigment_grid: Vec<[f32; 4]>,
}

impl SimTile {
    #[must_use]
    pub fn new(coord: [i32; 2]) -> Self {
        let len = (TILE_SIZE * TILE_SIZE) as usize;
        Self {
            coord,
            is_wet: true,
            moisture_level: 0.0,
            ping_pong_parity: false,
            fluid_grid: vec![[0.0; 4]; len],
            pigment_grid: vec![[0.0; 4]; len],
        }
    }

    #[inline]
    #[must_use]
    pub fn idx(x: u32, y: u32) -> usize {
        (y * TILE_SIZE + x) as usize
    }

    pub fn update_wetness_state(&mut self) -> bool {
        let mut max_h = 0.0_f32;
        for cell in &self.fluid_grid {
            if cell[2] > max_h {
                max_h = cell[2];
            }
        }
        self.moisture_level = max_h;
        self.is_wet = max_h > DRY_MOISTURE_THRESHOLD;
        self.is_wet
    }
}

pub struct SparseTileAllocator {
    pub tiles: HashMap<[i32; 2], SimTile>,
    pub paper_grain: Vec<f32>,
    pub params: SimParamsUniform,
}

impl SparseTileAllocator {
    #[must_use]
    pub fn new(params: SimParamsUniform) -> Self {
        let mut paper_grain = vec![0.5_f32; (TILE_SIZE * TILE_SIZE) as usize];
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let fx = x as f32 / TILE_SIZE as f32;
                let fy = y as f32 / TILE_SIZE as f32;
                let fiber = 0.5
                    + 0.25 * (fx * std::f32::consts::TAU * 8.0).sin() * (fy * std::f32::consts::TAU * 6.0).cos()
                    + 0.15 * ((fx + fy) * std::f32::consts::TAU * 16.0).sin()
                    + 0.10 * ((fx * 31.0).sin() * (fy * 29.0).cos());
                let i = SimTile::idx(x, y);
                if let Some(slot) = paper_grain.get_mut(i) {
                    *slot = fiber.clamp(0.05, 0.95);
                }
            }
        }
        Self {
            tiles: HashMap::new(),
            paper_grain,
            params,
        }
    }

    pub fn inject_particles(&mut self, particles: &[BrushParticle]) {
        let ts = TILE_SIZE as f32;
        for pt in particles {
            let r = pt.radius * 3.0;
            let min_tile_x = ((pt.position[0] - r) / ts).floor() as i32;
            let max_tile_x = ((pt.position[0] + r) / ts).floor() as i32;
            let min_tile_y = ((pt.position[1] - r) / ts).floor() as i32;
            let max_tile_y = ((pt.position[1] + r) / ts).floor() as i32;

            for ty in min_tile_y..=max_tile_y {
                for tx in min_tile_x..=max_tile_x {
                    let tile = self.tiles.entry([tx, ty]).or_insert_with(|| SimTile::new([tx, ty]));
                    tile.is_wet = true;
                    let local_pos = [pt.position[0] - tx as f32 * ts, pt.position[1] - ty as f32 * ts];
                    let sigma = pt.radius.max(0.5);
                    let two_sigma_sq = 2.0 * sigma * sigma;
                    let x0 = (local_pos[0] - r).floor().max(0.0) as u32;
                    let x1 = (local_pos[0] + r).ceil().min((TILE_SIZE - 1) as f32) as u32;
                    let y0 = (local_pos[1] - r).floor().max(0.0) as u32;
                    let y1 = (local_pos[1] + r).ceil().min((TILE_SIZE - 1) as f32) as u32;

                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            let dx = (x as f32 + 0.5) - local_pos[0];
                            let dy = (y as f32 + 0.5) - local_pos[1];
                            let dist_sq = dx * dx + dy * dy;
                            if dist_sq <= r * r {
                                let kernel = (-dist_sq / two_sigma_sq).exp();
                                let idx = SimTile::idx(x, y);
                                let inj_h = pt.wetness * kernel;
                                if let Some(f_cell) = tile.fluid_grid.get_mut(idx) {
                                    f_cell[0] = pt.velocity[0];
                                    f_cell[1] = pt.velocity[1];
                                    f_cell[2] = (f_cell[2] + inj_h).min(4.0);
                                }
                                if let Some(p_cell) = tile.pigment_grid.get_mut(idx) {
                                    for c in 0..4 {
                                        p_cell[c] = (p_cell[c] + pt.pigment_rgba[c] * kernel).min(4.0);
                                    }
                                }
                            }
                        }
                    }
                    tile.update_wetness_state();
                }
            }
        }
    }

    pub fn step_simulation(&mut self) -> usize {
        let mut active_count = 0;
        let params = self.params;
        let paper = &self.paper_grain;

        for tile in self.tiles.values_mut() {
            if !tile.is_wet {
                continue;
            }
            active_count += 1;
            let mut next_fluid = tile.fluid_grid.clone();
            let mut next_pigment = tile.pigment_grid.clone();

            for y in 0..TILE_SIZE {
                for x in 0..TILE_SIZE {
                    let idx = SimTile::idx(x, y);
                    let cf = tile.fluid_grid.get(idx).copied().unwrap_or([0.0; 4]);
                    let cp = tile.pigment_grid.get(idx).copied().unwrap_or([0.0; 4]);
                    let p_grain = paper.get(idx).copied().unwrap_or(0.5);

                    let yn = (y + 1).min(TILE_SIZE - 1);
                    let ys = y.saturating_sub(1);
                    let xe = (x + 1).min(TILE_SIZE - 1);
                    let xw = x.saturating_sub(1);

                    let n = tile.fluid_grid.get(SimTile::idx(x, yn)).map_or(0.0, |v| v[2]);
                    let s = tile.fluid_grid.get(SimTile::idx(x, ys)).map_or(0.0, |v| v[2]);
                    let e = tile.fluid_grid.get(SimTile::idx(xe, y)).map_or(0.0, |v| v[2]);
                    let w = tile.fluid_grid.get(SimTile::idx(xw, y)).map_or(0.0, |v| v[2]);

                    let lap_h = (n + s + e + w) - 4.0 * cf[2];
                    let permeability = 1.0 - params.porosity_coupling * p_grain;
                    let fiber_pull = params.capillary_strength * (1.0 - p_grain) * permeability;

                    let mut new_h = cf[2] + (lap_h * params.viscosity * permeability + fiber_pull * 0.008) * params.delta_time;
                    new_h = (new_h - params.evaporation_rate * params.delta_time).max(0.0);

                    if new_h > DRY_MOISTURE_THRESHOLD {
                        let pn = tile.pigment_grid.get(SimTile::idx(x, yn)).copied().unwrap_or([0.0; 4]);
                        let ps = tile.pigment_grid.get(SimTile::idx(x, ys)).copied().unwrap_or([0.0; 4]);
                        let pe = tile.pigment_grid.get(SimTile::idx(xe, y)).copied().unwrap_or([0.0; 4]);
                        let pw = tile.pigment_grid.get(SimTile::idx(xw, y)).copied().unwrap_or([0.0; 4]);
                        let grad_mag = ((e - w).powi(2) + (n - s).powi(2)).sqrt() * 0.5;

                        if let Some(p_out) = next_pigment.get_mut(idx) {
                            for c in 0..4 {
                                let lap_p = (pn[c] + ps[c] + pe[c] + pw[c]) - 4.0 * cp[c];
                                let diff = lap_p * (cf[2] * 0.12) * params.delta_time;
                                let ring_transfer = cp[c] * ((lap_h * 1.8 + grad_mag * 2.2) * params.edge_pooling_factor) * params.delta_time;
                                p_out[c] = (cp[c] + diff + ring_transfer).clamp(0.0, 4.0);
                            }
                        }
                    }

                    if let Some(f_out) = next_fluid.get_mut(idx) {
                        *f_out = [cf[0] * 0.98, cf[1] * 0.98, new_h, permeability];
                    }
                }
            }

            tile.fluid_grid = next_fluid;
            tile.pigment_grid = next_pigment;
            tile.ping_pong_parity = !tile.ping_pong_parity;
            tile.update_wetness_state();
        }
        active_count
    }
}
