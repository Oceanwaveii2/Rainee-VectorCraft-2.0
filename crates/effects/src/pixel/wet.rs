//! A conservative finite-volume diffusion model, not a shallow-water/Navier-Stokes solver.
//! Water spreads; mobile pigment transfers only between wet cells; evaporation and paper
//! absorption deposit pigment irreversibly. Pairwise flux conserves pigment before RGBA8
//! quantisation. The simulation is recomputed from source art, never accumulated on repaints.
//! At most 512² cells and 64 iterations; large rasters use an explicitly documented coarse
//! grid. Transparent boundaries are impermeable. A distance mask bounds paint to `spread`.

use super::PixelSpace;

#[derive(Clone, Copy, Default)]
struct Cell {
    water: f32,
    mobile: [f32; 4],
    fixed: [f32; 4],
}

fn solve(cells: &mut [Cell], w: usize, distance: &[usize], radius: usize, drying: f32, paper: f32, steps: u32) {
    let mut next = cells.to_vec();
    for _ in 0..steps.min(64) {
        next.copy_from_slice(cells);
        for i in 0..cells.len() {
            for j in [(i % w + 1 < w).then_some(i + 1), (i + w < cells.len()).then_some(i + w)].into_iter().flatten() {
                if distance[i].max(distance[j]) > radius {
                    continue;
                }
                let water_flux = 0.18 * (cells[i].water - cells[j].water);
                next[i].water -= water_flux;
                next[j].water += water_flux;
                // CFL <= .18 * four neighbours; each nonnegative channel stays nonnegative.
                let mobility = (cells[i].water.min(cells[j].water) * 4.0).clamp(0.0, 1.0);
                for k in 0..4 {
                    let flux = 0.18 * mobility * (cells[i].mobile[k] - cells[j].mobile[k]);
                    next[i].mobile[k] -= flux;
                    next[j].mobile[k] += flux;
                }
            }
        }
        for (i, c) in next.iter_mut().enumerate() {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            let hash = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663);
            let roughness = (hash.wrapping_mul(2654435761) & 255) as f32 / 255.0;
            c.water *= 1.0 - drying;
            let deposit = (0.015 + paper * roughness * 0.08 + (1.0 - c.water.min(1.0)) * drying).clamp(0.0, 1.0);
            for k in 0..4 {
                let amount = c.mobile[k] * deposit;
                c.mobile[k] -= amount;
                c.fixed[k] += amount;
            }
        }
        cells.copy_from_slice(&next);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply(px: &mut [[u8; 4]], w: usize, h: usize, space: &PixelSpace, spread: f64, water: f64, drying: f64, paper: f64, steps: u32) {
    if water <= 0.0 || spread <= 0.0 || w == 0 || h == 0 || w.checked_mul(h) != Some(px.len()) {
        return;
    }
    // Integer averaging gives a reproducible bounded grid; never allocates proportional to
    // untrusted effect radii. This is a quality tier, not unlimited-resolution fluid physics.
    let stride = w.max(h).div_ceil(512).max(1);
    let gw = w.div_ceil(stride);
    let gh = h.div_ceil(stride);
    let mut cells = vec![Cell::default(); gw * gh];
    let mut counts = vec![0usize; cells.len()];
    for (i, p) in px.iter().enumerate() {
        let j = (i / w / stride) * gw + (i % w / stride);
        counts[j] += 1;
        for (v, c) in cells[j].mobile.iter_mut().zip(p) {
            *v += *c as f32 / 255.0;
        }
    }
    let mut distance = vec![usize::MAX / 4; cells.len()];
    for (i, c) in cells.iter_mut().enumerate() {
        for v in &mut c.mobile {
            *v /= counts[i].max(1) as f32;
        }
        c.water = c.mobile[3] * water as f32;
        if c.mobile[3] > 0.0 {
            distance[i] = 0;
        }
    }
    for i in 0..distance.len() {
        if i % gw > 0 {
            distance[i] = distance[i].min(distance[i - 1] + 1);
        }
        if i >= gw {
            distance[i] = distance[i].min(distance[i - gw] + 1);
        }
    }
    for i in (0..distance.len()).rev() {
        if i % gw + 1 < gw {
            distance[i] = distance[i].min(distance[i + 1] + 1);
        }
        if i + gw < distance.len() {
            distance[i] = distance[i].min(distance[i + gw] + 1);
        }
    }
    let radius = (spread / (space.px.max(1e-9) * stride as f64)).floor().clamp(0.0, 4096.0) as usize;
    solve(&mut cells, gw, &distance, radius, drying as f32, paper as f32, steps);
    for (i, p) in px.iter_mut().enumerate() {
        let c = cells[(i / w / stride) * gw + i % w / stride];
        let values: [f32; 4] = std::array::from_fn(|k| c.mobile[k] + c.fixed[k]);
        let a = values[3].clamp(0.0, 1.0);
        *p = std::array::from_fn(|k| (values[k].clamp(0.0, a) * 255.0).round() as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_geom::{Affine, Point};
    #[test]
    fn flux_conserves_pigment_and_drying_stops_motion() {
        let mut cells = vec![Cell::default(); 25];
        cells[12] = Cell { water: 1.0, mobile: [0.5, 0.1, 0.2, 0.5], ..Default::default() };
        let initial = cells.clone();
        solve(&mut cells, 5, &[0; 25], 4, 0.04, 0.5, 32);
        for k in 0..4 {
            let total: f32 = cells.iter().map(|c| c.mobile[k] + c.fixed[k]).sum();
            assert!((total - initial[12].mobile[k]).abs() < 1e-5);
        }
        assert!(cells[11].mobile[3] + cells[11].fixed[3] > 0.0);
        assert!(cells[12].fixed[3] > 0.0);
        let mut dry = initial;
        for c in &mut dry {
            c.water = 0.0;
        }
        solve(&mut dry, 5, &[0; 25], 4, 1.0, 0.5, 32);
        assert_eq!(dry[11].mobile[3] + dry[11].fixed[3], 0.0);
    }
    #[test]
    fn wet_output_is_deterministic_bounded_and_premultiplied() {
        let space = PixelSpace { to_doc: Affine::IDENTITY, px: 1.0, center: Point::new(4.0, 4.0) };
        let mut pixels = vec![[0; 4]; 81];
        pixels[40] = [120, 0, 0, 120];
        pixels[41] = [0, 0, 120, 120];
        let original = pixels.clone();
        let mut other = pixels.clone();
        apply(&mut pixels, 9, 9, &space, 3.0, 1.0, 0.04, 0.3, 32);
        apply(&mut other, 9, 9, &space, 3.0, 1.0, 0.04, 0.3, 32);
        assert_eq!(pixels, other);
        assert_ne!(pixels, original);
        assert!(pixels[40][0] > 0 && pixels[40][2] > 0);
        assert_eq!(pixels[0], [0; 4]);
        assert!(pixels.iter().all(|p| p[..3].iter().all(|v| *v <= p[3])));
        apply(&mut other, 9, 9, &space, 3.0, 0.0, 0.04, 0.3, 32);
        assert_eq!(pixels, other);
    }
}
