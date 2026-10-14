//! 2D Quadratic Error Function (QEF) Star Junction Solver, Kochanek-Bartels (TCB)
//! Tension-Continuity-Bias Cubic Hermite Spline Fitter, and Neural UDF Width Extractor.
//!
//! Reference:
//! - Ju et al. (SIGGRAPH 2002), "Dual Contouring of Hermite Data"
//! - Kochanek & Bartels (SIGGRAPH 1984), "Interpolating Splines with Local Tension, Continuity, and Bias Control"

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use vectorcraft_geom::{Anchor, AnchorKind, PathData, Point, SubPath, Vec2};

/// Vectorization engine mode controlled by `super_trace_ext.rs` (`superEngineMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuperEngineMode {
    /// Classic high-speed polygon + least-squares cubic Bézier fitter (`<5ms`).
    #[default]
    Classic,
    /// Dual Contouring 2D QEF star-junction solver + distance-field width profile recovery.
    NeuralUdf,
    /// Kochanek-Bartels (TCB) cubic Hermite spline fitter with explicit corner tension.
    TcbSpline,
    /// Rebelle-compatible paint strata contour solver with overlap glazing margin.
    RebelleStrata,
}

impl SuperEngineMode {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::Classic
    }
}

#[must_use]
pub fn default_tcb_tension() -> f64 {
    0.35
}

#[must_use]
pub fn is_default_tcb_tension(v: &f64) -> bool {
    (*v - 0.35).abs() < 1e-9
}

#[must_use]
pub fn default_preserve_star_junctions() -> bool {
    true
}

#[must_use]
pub fn is_true(v: &bool) -> bool {
    *v
}

/// 2D Quadratic Error Function (QEF) accumulator:
/// minimizes $E(x) = \sum_i (n_i \cdot (x - p_i))^2 + \lambda \|x - \bar{p}\|^2$
/// via regularized $2 \times 2$ Tikhonov normal equations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QefSolver2D {
    pub ata_00: f64,
    pub ata_01: f64,
    pub ata_11: f64,
    pub atb_x: f64,
    pub atb_y: f64,
    pub mass_point: Point,
    pub count: u32,
}

impl QefSolver2D {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulates a directed line/plane constraint with unit normal `normal` passing through `point`.
    pub fn add_plane(&mut self, point: Point, normal: Vec2) {
        let len = normal.hypot();
        if len <= 1e-9 || !len.is_finite() {
            return;
        }
        let nx = normal.x / len;
        let ny = normal.y / len;
        let d = nx * point.x + ny * point.y;

        self.ata_00 += nx * nx;
        self.ata_01 += nx * ny;
        self.ata_11 += ny * ny;
        self.atb_x += nx * d;
        self.atb_y += ny * d;

        let c = f64::from(self.count);
        self.mass_point = Point::new(
            (self.mass_point.x * c + point.x) / (c + 1.0),
            (self.mass_point.y * c + point.y) / (c + 1.0),
        );
        self.count = self.count.saturating_add(1);
    }

    /// Accumulates a tangent ray passing through `point` with direction `tangent`
    /// by converting the tangent into its orthogonal unit normal `(-ty, tx)`.
    pub fn add_tangent_line(&mut self, point: Point, tangent: Vec2) {
        self.add_plane(point, Vec2::new(-tangent.y, tangent.x));
    }

    /// Solves for the optimal 2D star-junction vertex with Tikhonov regularization `reg_lambda`.
    /// Never panics on rank-deficient (parallel) line intersections; smoothly falls back to `fallback`.
    #[must_use]
    pub fn solve(&self, fallback: Point, reg_lambda: f64) -> Point {
        if self.count == 0 {
            return fallback;
        }
        let anchor = if self.count > 0 { self.mass_point } else { fallback };
        let reg = reg_lambda.clamp(1e-6, 10.0);
        let a00 = self.ata_00 + reg;
        let a01 = self.ata_01;
        let a11 = self.ata_11 + reg;

        let bx = self.atb_x + reg * anchor.x;
        let by = self.atb_y + reg * anchor.y;

        let det = a00 * a11 - a01 * a01;
        if det.abs() <= 1e-12 || !det.is_finite() {
            return anchor;
        }
        let x = (a11 * bx - a01 * by) / det;
        let y = (a00 * by - a01 * bx) / det;
        if x.is_finite() && y.is_finite() {
            Point::new(x, y)
        } else {
            anchor
        }
    }
}

/// Kochanek-Bartels (TCB) spline parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TcbParams {
    /// Tension $T \in [-1.0, 1.0]$: $+1$ yields tight/sharp tangents, $-1$ yields wide slack loops.
    pub tension: f64,
    /// Continuity $C \in [-1.0, 1.0]$: $-1$ yields hard $C^0$ cusps, $0$ yields $C^1$ Catmull-Rom continuity.
    pub continuity: f64,
    /// Bias $B \in [-1.0, 1.0]$: $+1$ biases toward incoming chord, $-1$ toward outgoing chord.
    pub bias: f64,
}

impl Default for TcbParams {
    fn default() -> Self {
        Self {
            tension: 0.35,
            continuity: 0.0,
            bias: 0.0,
        }
    }
}

/// Fits a polygon or polyline with a Kochanek-Bartels (TCB) cubic Hermite spline, converting
/// each Hermite span into an exact `SubPath` of `Anchor` control handles (`h_in`, `h_out`).
/// Vertices whose turn angle exceeds `corner_angle_deg` receive hard corner continuity ($C = -1$).
#[must_use]
pub fn fit_tcb_spline(pts: &[Point], closed: bool, params: TcbParams, corner_angle_deg: f64) -> Option<SubPath> {
    let n = pts.len();
    if (closed && n < 3) || (!closed && n < 2) {
        return None;
    }
    let t_global = params.tension.clamp(-1.0, 1.0);
    let b_global = params.bias.clamp(-1.0, 1.0);
    let corner_cos = corner_angle_deg.clamp(5.0, 175.0).to_radians().cos();

    let mut anchors = Vec::with_capacity(n);
    for i in 0..n {
        let Some(&p_curr) = pts.get(i) else { continue };
        let p_prev = if i > 0 {
            pts.get(i - 1).copied().unwrap_or(p_curr)
        } else if closed {
            pts.get(n - 1).copied().unwrap_or(p_curr)
        } else {
            p_curr
        };
        let p_next = if i + 1 < n {
            pts.get(i + 1).copied().unwrap_or(p_curr)
        } else if closed {
            pts.first().copied().unwrap_or(p_curr)
        } else {
            p_curr
        };

        let chord_in = p_curr - p_prev;
        let chord_out = p_next - p_curr;
        let len_in = chord_in.hypot();
        let len_out = chord_out.hypot();

        let is_endpoint = !closed && (i == 0 || i + 1 == n);
        let is_sharp_corner = if len_in > 1e-6 && len_out > 1e-6 {
            let cos_theta = (chord_in.dot(chord_out) / (len_in * len_out)).clamp(-1.0, 1.0);
            cos_theta < corner_cos
        } else {
            true
        };

        let c_local = if is_sharp_corner { -0.85 } else { params.continuity.clamp(-1.0, 1.0) };
        let t_local = if is_sharp_corner { (t_global + 0.5).clamp(-1.0, 1.0) } else { t_global };

        // Outgoing tangent D_i^+ and incoming tangent D_i^-
        let d_out = if is_endpoint && i + 1 == n {
            Vec2::ZERO
        } else if is_endpoint && i == 0 {
            chord_out * (0.5 * (1.0 - t_local))
        } else {
            let a = (1.0 - t_local) * (1.0 + b_global) * (1.0 + c_local) * 0.5;
            let b = (1.0 - t_local) * (1.0 - b_global) * (1.0 - c_local) * 0.5;
            chord_in * a + chord_out * b
        };

        let d_in = if is_endpoint && i == 0 {
            Vec2::ZERO
        } else if is_endpoint && i + 1 == n {
            chord_in * (0.5 * (1.0 - t_local))
        } else {
            let a = (1.0 - t_local) * (1.0 + b_global) * (1.0 - c_local) * 0.5;
            let b = (1.0 - t_local) * (1.0 - b_global) * (1.0 + c_local) * 0.5;
            chord_in * a + chord_out * b
        };

        // Exact Hermite-to-Bézier conversion:
        // H_out(i) = P_i + D_i^+ / 3, H_in(i) = P_i - D_i^- / 3
        let h_out = p_curr + d_out * (1.0 / 3.0);
        let h_in = p_curr - d_in * (1.0 / 3.0);
        let kind = if is_sharp_corner || is_endpoint {
            AnchorKind::Corner
        } else {
            AnchorKind::Smooth
        };

        anchors.push(Anchor { p: p_curr, h_in, h_out, kind });
    }

    Some(SubPath { anchors, closed })
}

/// Samples a chamfer distance grid along `path` to recover normalized per-knot width factors `(u, factor)`
/// suitable for `vectorcraft_doc::WidthProfile` reconstruction in `NeuralUdf` mode.
#[must_use]
pub fn extract_udf_width_factors(
    path: &PathData,
    chamfer_dt: &[u16],
    w: usize,
    h: usize,
    mean_width: f64,
) -> Vec<(f64, f64)> {
    let Some(sp) = path.subpaths.first() else { return Vec::new() };
    if sp.anchors.len() < 2 || w == 0 || h == 0 || mean_width <= 1e-6 {
        return Vec::new();
    }
    let n = sp.anchors.len();
    let mut cum = Vec::with_capacity(n);
    let mut total = 0.0_f64;
    cum.push(0.0);
    for i in 1..n {
        if let (Some(prev), Some(curr)) = (sp.anchors.get(i - 1), sp.anchors.get(i)) {
            total += prev.p.distance(curr.p);
            cum.push(total);
        }
    }
    if total <= 1e-6 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n);
    for (a, &s) in sp.anchors.iter().zip(&cum) {
        let px = (a.p.x.floor() as isize).clamp(0, w.saturating_sub(1) as isize) as usize;
        let py = (a.p.y.floor() as isize).clamp(0, h.saturating_sub(1) as isize) as usize;
        let d_raw = chamfer_dt.get(py * w + px).copied().unwrap_or(3);
        let local_w = (2.0 * f64::from(d_raw) / 3.0 - 1.0).max(0.5);
        let factor = (local_w / mean_width).clamp(0.1, 4.0);
        out.push(((s / total).clamp(0.0, 1.0), factor));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qef_solver_recovers_exact_star_junction_and_handles_parallel_lines() {
        // 3 lines meeting at (50.0, 35.0): horizontal y=35, vertical x=50, diagonal y - 35 = x - 50
        let target = Point::new(50.0, 35.0);
        let mut qef = QefSolver2D::new();
        qef.add_tangent_line(Point::new(20.0, 35.0), Vec2::new(1.0, 0.0));
        qef.add_tangent_line(Point::new(50.0, 10.0), Vec2::new(0.0, 1.0));
        qef.add_tangent_line(Point::new(30.0, 15.0), Vec2::new(1.0, 1.0));

        let solved = qef.solve(Point::new(48.0, 36.0), 1e-5);
        assert!(solved.distance(target) < 1e-2, "expected {target:?}, got {solved:?}");

        // Rank-deficient case: two parallel horizontal lines
        let mut degenerate = QefSolver2D::new();
        degenerate.add_tangent_line(Point::new(10.0, 20.0), Vec2::new(1.0, 0.0));
        degenerate.add_tangent_line(Point::new(30.0, 20.0), Vec2::new(1.0, 0.0));
        let deg_pt = degenerate.solve(Point::new(20.0, 20.0), 1e-3);
        assert!(deg_pt.x.is_finite() && deg_pt.y.is_finite());
        assert!((deg_pt.y - 20.0).abs() < 1e-2);
    }

    #[test]
    fn tcb_spline_respects_tension_and_hard_corners() {
        let pts = vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(0.0, 50.0),
        ];
        let low_t = fit_tcb_spline(&pts, true, TcbParams { tension: -0.5, continuity: 0.0, bias: 0.0 }, 45.0).unwrap();
        let high_t = fit_tcb_spline(&pts, true, TcbParams { tension: 0.9, continuity: 0.0, bias: 0.0 }, 45.0).unwrap();
        let d_low = low_t.anchors[0].p.distance(low_t.anchors[0].h_out);
        let d_high = high_t.anchors[0].p.distance(high_t.anchors[0].h_out);
        assert!(d_high < d_low, "higher tension must shorten outgoing handle length ({d_high} vs {d_low})");
    }
}
