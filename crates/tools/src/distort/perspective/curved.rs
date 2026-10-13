//! Nonlinear 2D reference projections. They are not planar homographies and cannot attach art.
use super::{PerspectiveGrid, Plane};
use vectorcraft_geom::Point;

impl PerspectiveGrid {
    /// Cylindrical (360° longitude) or equidistant hemispherical (180° field) projection.
    /// Distance is pixels/point-units per radian; horizon and origin.x locate the optical centre.
    pub fn project_direction(&self, v: [f64; 3]) -> Option<Point> {
        if !(4..=5).contains(&self.kind) || !v.iter().all(|x| x.is_finite()) || !self.distance.is_finite() || self.distance <= 0.0 {
            return None;
        }
        let [x, y, z] = v;
        let radial = x.hypot(z);
        let (u, w) = if self.kind == 4 {
            if radial < 1e-9 {
                return None;
            }
            (x.atan2(z), -y / radial)
        } else {
            let length = radial.hypot(y);
            if length < 1e-9 || z < 0.0 {
                return None;
            }
            let theta = (z / length).clamp(-1.0, 1.0).acos();
            let r = x.hypot(y);
            if r < 1e-9 { (0.0, 0.0) } else { (theta * x / r, -theta * y / r) }
        };
        Some(Point::new(self.origin[0] + self.distance * u, self.horizon + self.distance * w))
    }

    /// Sampled projected straight-world reference lines, bounded independently of zoom.
    pub(super) fn curved_lines(&self, plane: Plane) -> Vec<(Point, Point)> {
        if plane != Plane::Left || !self.cell.is_finite() || !self.distance.is_finite() || self.distance <= 0.0 {
            return vec![];
        }
        let spacing = (self.cell / self.distance).clamp(0.1, 1.0);
        let mut out = vec![];
        let mut curve = |points: Vec<Option<Point>>| {
            for pair in points.windows(2) {
                if let [Some(a), Some(b)] = pair
                    && a.distance(*b) < self.distance
                {
                    out.push((*a, *b));
                }
            }
        };
        if self.kind == 4 {
            let count = (std::f64::consts::PI / spacing).ceil() as i32;
            for i in -count..=count {
                let theta = i as f64 * std::f64::consts::PI / f64::from(count);
                curve((-24..=24).map(|j| self.project_direction([theta.sin(), j as f64 / 12.0, theta.cos()])).collect());
            }
            for z in [-1.0, 1.0] {
                let count = (2.0 / spacing).ceil() as i32;
                for i in -count..=count {
                    curve((-48..=48).map(|j| self.project_direction([j as f64 / 6.0, f64::from(i) * spacing, z])).collect());
                }
            }
        } else {
            let count = (3.0 / spacing).ceil() as i32;
            for i in -count..=count {
                for vertical in [false, true] {
                    curve(
                        (-48..=48)
                            .map(|j| {
                                let (a, b) = (f64::from(i) * spacing, j as f64 / 6.0);
                                self.project_direction(if vertical { [a, b, 1.0] } else { [b, a, 1.0] })
                            })
                            .collect(),
                    );
                }
            }
            let r = self.distance * std::f64::consts::FRAC_PI_2;
            curve(
                (0..=128)
                    .map(|i| {
                        let (s, c) = (i as f64 * std::f64::consts::TAU / 128.0).sin_cos();
                        Some(Point::new(self.origin[0] + r * c, self.horizon + r * s))
                    })
                    .collect(),
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_geom::Rect;
    #[test]
    fn cardinal_vanishing_points_and_non_planar_projection() {
        let mut g = PerspectiveGrid::preset(4, Rect::new(0.0, 0.0, 800.0, 600.0));
        g.validate().unwrap();
        let centre = Point::new(g.origin[0], g.horizon);
        assert_eq!(g.project_direction([0.0, 0.0, 1.0]), Some(centre));
        let right = g.project_direction([1.0, 0.0, 0.0]).unwrap();
        assert!((right.x - centre.x - g.distance * std::f64::consts::FRAC_PI_2).abs() < 1e-8);
        assert!(g.homography(Plane::Left).is_none());
        assert!(!g.lines(Plane::Left).is_empty());
        let dense = g.lines(Plane::Left).len();
        g.cell = g.distance;
        assert!(g.lines(Plane::Left).len() < dense);
        g.kind = 5;
        g.validate().unwrap();
        let up = g.project_direction([0.0, 1.0, 0.0]).unwrap();
        assert!((up.y - centre.y + g.distance * std::f64::consts::FRAC_PI_2).abs() < 1e-8);
        assert!(g.project_direction([0.0, 0.0, -1.0]).is_none());
        assert!(g.project_direction([0.0, 0.0, 0.0]).is_none());
        assert!(!g.lines(Plane::Left).is_empty());
    }
}
