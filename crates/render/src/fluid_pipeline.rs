//! Sparse Active Tile Allocator (128x128 Rgba16Float Ping-Pong Storage Tiles)
//! & Compute Fluid Simulation Harness for VectorCraft Super Brushes.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use vectorcraft_brush::optics_ext::{BrushOpticsExt, evaluate_stroke_reservoir};
use vectorcraft_doc::StrokeLayer;
use vectorcraft_geom::BezPath;

use crate::km_optics::kubelka_munk_glaze;

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

impl SimParamsUniform {
    /// Constructs simulation parameters from a brush's `BrushOpticsExt` settings.
    #[must_use]
    pub fn from_brush_optics(ext: &BrushOpticsExt) -> Self {
        Self {
            viscosity: ext.lbm_viscosity.clamp(0.05, 1.0),
            capillary_strength: ext.capillary_bleed.clamp(0.0, 1.0),
            edge_pooling_factor: ext.coffee_ring_pooling.clamp(0.0, 1.5),
            ..Self::default()
        }
    }

    /// Safe 32-byte little-endian serializer matching `struct SimParams` in `lbm_bleed.wgsl`.
    /// Uses safe little-endian serialization to satisfy `#![forbid(unsafe_code)]`.
    #[must_use]
    pub fn to_wgsl_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let words: [[u8; 4]; 8] = [
            self.viscosity.to_le_bytes(),
            self.evaporation_rate.to_le_bytes(),
            self.capillary_strength.to_le_bytes(),
            self.delta_time.to_le_bytes(),
            self.edge_pooling_factor.to_le_bytes(),
            self.surface_tension.to_le_bytes(),
            self.porosity_coupling.to_le_bytes(),
            self.particle_count.to_le_bytes(),
        ];
        for (i, w) in words.iter().enumerate() {
            let base = i * 4;
            if let Some(slot) = buf.get_mut(base..base + 4) {
                slot.copy_from_slice(w);
            }
        }
        buf
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

impl BrushParticle {
    /// Safe 64-byte little-endian serializer matching `struct BrushParticle` in `lbm_bleed.wgsl`
    /// (including WGSL 16-byte alignment padding before and after `vec4<f32>` / `vec3<u32>`).
    #[must_use]
    pub fn to_wgsl_bytes(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        let words: [[u8; 4]; 16] = [
            self.position[0].to_le_bytes(),
            self.position[1].to_le_bytes(),
            self.velocity[0].to_le_bytes(),
            self.velocity[1].to_le_bytes(),
            self.radius.to_le_bytes(),
            self.wetness.to_le_bytes(),
            0u32.to_le_bytes(), // WGSL align(16) padding before pigment_rgba: vec4<f32>
            0u32.to_le_bytes(),
            self.pigment_rgba[0].to_le_bytes(),
            self.pigment_rgba[1].to_le_bytes(),
            self.pigment_rgba[2].to_le_bytes(),
            self.pigment_rgba[3].to_le_bytes(),
            self.splatter_seed.to_le_bytes(),
            self.pad[0].to_le_bytes(),
            self.pad[1].to_le_bytes(),
            self.pad[2].to_le_bytes(),
        ];
        for (i, w) in words.iter().enumerate() {
            let base = i * 4;
            if let Some(slot) = buf.get_mut(base..base + 4) {
                slot.copy_from_slice(w);
            }
        }
        buf
    }

    /// Serializes a slice of `BrushParticle` instances into a contiguous WGSL storage buffer.
    #[must_use]
    pub fn slice_to_wgsl_bytes(particles: &[Self]) -> Vec<u8> {
        let mut out = Vec::with_capacity(particles.len().saturating_mul(64));
        for pt in particles {
            out.extend_from_slice(&pt.to_wgsl_bytes());
        }
        out
    }
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

    /// Composites this tile's pigment field onto a premultiplied RGBA8 target buffer
    /// using Kubelka-Munk optical glazing over `substrate_rgb`.
    pub fn composite_onto_rgba8(
        &self,
        dst: &mut [u8],
        dst_w: u32,
        dst_h: u32,
         substrate_rgb: [u8; 3],
    ) {
        let base_x = self.coord[0].saturating_mul(TILE_SIZE as i32);
        let base_y = self.coord[1].saturating_mul(TILE_SIZE as i32);
        for ty in 0..TILE_SIZE {
            let py = base_y + ty as i32;
            if py < 0 || py >= dst_h as i32 {
                continue;
            }
            for tx in 0..TILE_SIZE {
                let px = base_x + tx as i32;
                if px < 0 || px >= dst_w as i32 {
                    continue;
                }
                let cell_idx = Self::idx(tx, ty);
                let Some(&pigment) = self.pigment_grid.get(cell_idx) else { continue };
                let conc = pigment[3];
                if conc <= 1e-4 {
                    continue;
                }
                let r_u8 = (pigment[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                let g_u8 = (pigment[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                let b_u8 = (pigment[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                let glazed = kubelka_munk_glaze([r_u8, g_u8, b_u8], conc.clamp(0.02, 4.0), substrate_rgb);
                let alpha = conc.clamp(0.0, 1.0);
                let dst_offset = ((py as u32 * dst_w + px as u32) * 4) as usize;
                if let Some(slot) = dst.get_mut(dst_offset..dst_offset + 4) {
                    let inv_a = 1.0 - alpha;
                    slot[0] = (f32::from(glazed[0]) * alpha + f32::from(slot[0]) * inv_a).round().clamp(0.0, 255.0) as u8;
                    slot[1] = (f32::from(glazed[1]) * alpha + f32::from(slot[1]) * inv_a).round().clamp(0.0, 255.0) as u8;
                    slot[2] = (f32::from(glazed[2]) * alpha + f32::from(slot[2]) * inv_a).round().clamp(0.0, 255.0) as u8;
                    slot[3] = (alpha * 255.0 + f32::from(slot[3]) * inv_a).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
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

    /// Converts an editable vector stroke (`BezPath` + `StrokeLayer`) and its `BrushOpticsExt`
    /// reservoir profile into `BrushParticle` packets and injects them into the sparse $128 \times 128$ tiles.
    pub fn emit_stroke_particles(
        &mut self,
        bp: &BezPath,
        stroke: &StrokeLayer,
        ext: &BrushOpticsExt,
        seed: u64,
    ) -> Vec<BrushParticle> {
        let knots = evaluate_stroke_reservoir(bp, stroke, ext, seed);
        let [r, g, b] = stroke.paint.color().map_or([0.0, 0.0, 0.0], |c| c.to_rgb());
        let base_radius = (stroke.width.max(0.5) * 0.5) as f32;
        let mut particles = Vec::with_capacity(knots.len());

        for (idx, knot) in knots.iter().enumerate() {
            let pt = BrushParticle {
                position: [knot.point.x as f32, knot.point.y as f32],
                velocity: [knot.tangent.x as f32, knot.tangent.y as f32],
                radius: base_radius,
                wetness: knot.wetness,
                pigment_rgba: [r, g, b, knot.film_thickness.clamp(0.05, 4.0)],
                splatter_seed: (seed as u32).wrapping_add(idx as u32),
                pad: [0; 3],
            };
            particles.push(pt);
        }
        self.params.particle_count = particles.len() as u32;
        self.inject_particles(&particles);
        particles
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

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_brush::optics_ext::RebelleMedia;
    use vectorcraft_color::{Color, Paint};

    #[test]
    fn wgsl_serializers_match_exact_byte_alignments_without_unsafe() {
        let params = SimParamsUniform::default();
        let bytes = params.to_wgsl_bytes();
        assert_eq!(bytes.len(), 32);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), params.viscosity);

        let pt = BrushParticle {
            position: [12.5, 34.0],
            velocity: [1.0, 0.0],
            radius: 4.0,
            wetness: 0.9,
            pigment_rgba: [0.1, 0.4, 0.8, 1.0],
            splatter_seed: 42,
            pad: [0; 3],
        };
        let pbytes = BrushParticle::slice_to_wgsl_bytes(&[pt]);
        assert_eq!(pbytes.len(), 64);
        assert_eq!(f32::from_le_bytes(pbytes[0..4].try_into().unwrap()), 12.5);
        assert_eq!(u32::from_le_bytes(pbytes[48..52].try_into().unwrap()), 42);
    }

    #[test]
    fn stroke_emission_allocates_tiles_and_composites_kubelka_munk_glaze() {
        let ext = BrushOpticsExt {
            rebelle_media: Some(RebelleMedia::WatercolorWash),
            lbm_viscosity: 0.40,
            capillary_bleed: 0.90,
            coffee_ring_pooling: 1.0,
            ..Default::default()
        };
        let mut alloc = SparseTileAllocator::new(SimParamsUniform::from_brush_optics(&ext));
        let mut bp = BezPath::new();
        bp.move_to((20.0, 20.0));
        bp.line_to((80.0, 20.0));
        let stroke = StrokeLayer::new(Paint::solid(Color::rgb8(10, 80, 200)), 6.0);

        let emitted = alloc.emit_stroke_particles(&bp, &stroke, &ext, 77);
        assert!(!emitted.is_empty());
        assert!(alloc.tiles.contains_key(&[0, 0]));
        assert_eq!(alloc.step_simulation(), 1);

        let mut canvas = vec![255u8; 128 * 128 * 4];
        if let Some(tile) = alloc.tiles.get(&[0, 0]) {
            tile.composite_onto_rgba8(&mut canvas, 128, 128, [250, 248, 242]);
        }
        // Center of the stroke at (50, 20) should be glazed with blue pigment (R < 200, B > R).
        let idx = ((20 * 128 + 50) * 4) as usize;
        assert!(canvas[idx] < 200 && canvas[idx + 2] > canvas[idx]);
    }
}
