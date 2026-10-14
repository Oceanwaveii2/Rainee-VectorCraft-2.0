//! Original implementation of the 1 Euro equations: https://gery.casiez.net/1euro/
//! (Casiez, Roussel and Vogel, CHI 2012). Prediction is disposable overlay data.
use vectorcraft_geom::{Point, Vec2};

#[path = "stylus_tracker.rs"]
mod tracker;
pub use tracker::{OneEuro1D, Tracker};

/// Continuous attraction: its displacement tends to zero at the tolerance boundary.
/// Shift explicitly locks an angle; ordinary freehand remains untouched by default.
#[allow(clippy::too_many_arguments)]
pub fn magnet(
    p: Point,
    origin: Point,
    g: &crate::distort::perspective::PerspectiveGrid,
    tolerance: f64,
    strength: f64,
    enabled: bool,
    angle: f64,
    shift: bool,
    offset: f64,
) -> Point {
    let d = p - origin;
    if angle > 0.0 && shift && d.hypot() > 1e-9 {
        let a = ((d.y.atan2(d.x).to_degrees() - offset) / angle).round() * angle + offset;
        let (s, c) = a.to_radians().sin_cos();
        return origin + Vec2::new(c, s) * d.hypot();
    }
    if !enabled || !g.visible || tolerance <= 0.0 || d.hypot() < 1e-9 {
        return p;
    }
    let candidates = if g.kind <= 3 {
        let mut targets = vec![Point::new(g.vp_left, g.horizon)];
        if g.kind >= 2 {
            targets.push(Point::new(g.vp_right, g.horizon));
        }
        if g.kind == 3 {
            targets.push(Point::new(g.vp_vertical[0], g.vp_vertical[1]));
        }
        let mut nearest = targets
            .into_iter()
            .filter_map(|v| {
                let ray = v - origin;
                let len = ray.hypot2();
                (len > 1e-9).then(|| origin + ray * (d.dot(ray) / len))
            })
            .collect::<Vec<_>>();
        if g.kind <= 2 {
            nearest.push(Point::new(origin.x, p.y));
        }
        nearest
    } else {
        g.lines(crate::distort::perspective::Plane::Left)
            .into_iter()
            .filter_map(|(a, b)| {
                let v = b - a;
                let len = v.hypot2();
                (len > 1e-9).then(|| a + v * ((p - a).dot(v) / len).clamp(0.0, 1.0))
            })
            .collect::<Vec<_>>()
    };
    // Blend nearby attractions instead of switching abruptly at equidistant rays.
    let mut displacement = Vec2::ZERO;
    let mut total = 0.0;
    for q in candidates {
        let weight = (1.0 - q.distance(p) / tolerance).clamp(0.0, 1.0).powi(2);
        displacement += (q - p) * weight;
        total += weight;
    }
    p + displacement * (strength.clamp(0.0, 1.0) / total.max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_jitter_and_fast_motion() {
        let mut slow = Tracker::default();
        let mut fast = Tracker::default();
        let mut jitter = 0.0;
        let mut lag = 0.0;
        for i in 0..240 {
            let t = i as f64 / 120.0;
            let p = slow.sample(Point::new(0.0, if i % 2 == 0 { 1.0 } else { -1.0 }), Some(t), true, 1.0, 0.01);
            if i > 120 {
                jitter += p.y.abs();
            }
            let p = fast.sample(Point::new(t * 1200.0, 0.0), Some(t), true, 1.0, 0.01);
            if i > 120 {
                lag += t * 1200.0 - p.x;
            }
        }
        assert!(jitter / 119.0 < 0.1);
        assert!(lag / 119.0 < 20.0);
        eprintln!(
            "synthetic 120 Hz: jitter {:.4} vs raw 1.0 point; lag {:.3} points = {:.3} ms at 1200 points/s",
            jitter / 119.0,
            lag / 119.0,
            lag / 119.0 / 1.2
        );
    }
    #[test]
    fn bypass_and_clock_discontinuity() {
        let mut t = Tracker::default();
        for i in 0..40 {
            let p = Point::new(i as f64, 4.0);
            assert_eq!(t.sample(p, Some(i as f64 / 120.0), false, 1.0, 0.01), p);
        }
        assert_eq!(t.sample(Point::new(100.0, 0.0), Some(0.0), true, 1.0, 0.01), Point::new(100.0, 0.0));
        assert!(t.prediction(14.0, 1.0).is_none());
    }
    #[test]
    fn prediction_is_bounded_and_expires_at_pause_or_reversal() {
        let mut t = Tracker::default();
        for i in 0..8 {
            t.sample(Point::new(i as f64 * 10.0, 0.0), Some(i as f64 / 120.0), false, 1.0, 0.01);
        }
        let (a, b) = t.prediction(14.0, 2.0).unwrap();
        assert!(a.distance(b) <= 12.0);
        t.tick(0.04);
        assert!(t.prediction(14.0, 2.0).is_none());
        t.sample(Point::new(60.0, 0.0), Some(8.0 / 120.0), false, 1.0, 0.01);
        assert!(t.prediction(14.0, 2.0).is_none());
    }
    #[test]
    fn magnetic_boundary_strength_axis_and_angle_bypass() {
        use crate::distort::perspective::PerspectiveGrid;
        let mut g = PerspectiveGrid::preset(1, vectorcraft_geom::Rect::new(0.0, 0.0, 800.0, 600.0));
        g.vp_left = 1000.0;
        g.horizon = 0.0;
        let origin = Point::ZERO;
        let snap = |p, enabled, strength, angle, shift| magnet(p, origin, &g, 12.0, strength, enabled, angle, shift, 0.0);
        for y in [11.999, 12.0, 12.001] {
            let p = Point::new(100.0, y);
            assert!(p.distance(snap(p, true, 1.0, 0.0, false)) < 1e-6);
        }
        let p = Point::new(100.0, 4.0);
        assert_eq!(snap(p, false, 1.0, 0.0, false), p);
        assert_eq!(snap(p, true, 0.0, 0.0, false), p);
        assert!(snap(p, true, 1.0, 0.0, false).y < snap(p, true, 0.5, 0.0, false).y);
        assert!(snap(Point::new(4.0, 100.0), true, 1.0, 0.0, false).x < 4.0);
        for angle in [15.0, 30.0, 45.0] {
            let q = snap(p, false, 1.0, angle, true);
            let units = (q - origin).y.atan2((q - origin).x).to_degrees() / angle;
            assert!((units - units.round()).abs() < 1e-9);
            assert_eq!(snap(p, false, 1.0, angle, false), p);
        }
        // Crossing the equal-distance boundary of horizontal and vertical rays is continuous.
        let a = snap(Point::new(4.0, 4.0 - 1e-6), true, 1.0, 0.0, false);
        let b = snap(Point::new(4.0, 4.0 + 1e-6), true, 1.0, 0.0, false);
        assert!(a.distance(b) < 1e-5);
    }
    #[test]
    fn corner_and_long_pause_restart_prediction_without_unbounded_filter_lag() {
        let mut t = Tracker::default();
        for i in 0..60 {
            t.sample(Point::new(i as f64 * 4.0, 0.0), Some(i as f64 / 120.0), true, 1.0, 0.01);
        }
        // Prediction is suppressed at a sharp corner when actual velocity changes direction.
        let mut raw = Tracker::default();
        for i in 0..8 {
            raw.sample(Point::new(i as f64 * 4.0, 0.0), Some(i as f64 / 120.0), false, 1.0, 0.01);
        }
        raw.sample(Point::new(28.0, 4.0), Some(8.0 / 120.0), false, 1.0, 0.01);
        assert!(raw.prediction(14.0, 1.0).is_none());
        let p = Point::new(236.0, 100.0);
        assert_eq!(t.sample(p, Some(2.0), true, 1.0, 0.01), p);
        assert!(t.prediction(14.0, 1.0).is_none());
    }
}
