//! Deliberately not a pressure average: use the bundle leader's authored dynamics.
use super::{Result, error};
use vectorcraft_doc::{Node, StrokeLayer};
use vectorcraft_geom::{PathEl, Point};

pub(super) fn valid(st: &StrokeLayer) -> bool {
    let ordered = |p: &[(f64, f64, f64)]| {
        p.len() <= 16384
            && p.iter().all(|p| p.0.is_finite() && (0.0..=1.0).contains(&p.0) && p.1.is_finite() && p.2.is_finite())
            && p.windows(2).all(|w| w[0].0 <= w[1].0)
    };
    st.profile.as_ref().is_none_or(|p| ordered(&p.points) && p.points.iter().all(|p| (0.0..=4096.0).contains(&p.1) && (0.0..=4096.0).contains(&p.2)))
        && st.nib.as_ref().is_none_or(|p| ordered(&p.points) && p.points.iter().all(|p| p.1.abs() <= 1e6 && (0.05..=1.0).contains(&p.2)))
}

pub(super) fn same_base(a: &Node, b: &Node) -> bool {
    let base = |n: &Node| {
        let mut appearance = n.appearance.clone();
        if let Some(s) = appearance.stroke_mut() {
            s.profile = None;
            s.nib = None;
        }
        appearance
    };
    base(a) == base(b)
}

fn lengths(points: &[Point]) -> Vec<f64> {
    let mut out = vec![0.0];
    for w in points.windows(2) {
        out.push(out.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
    }
    out
}

pub(super) fn transfer(node: &mut Node, target: &[Point]) -> Result<()> {
    let Some(stroke) = node.appearance.stroke() else { return Ok(()) };
    if stroke.profile.is_none() && stroke.nib.is_none() {
        return Ok(());
    }
    let mut source = Vec::new();
    if let Some(path) = node.path_data() {
        kurbo::flatten(path.to_bezpath().iter(), 0.05, |el| {
            if let PathEl::MoveTo(p) | PathEl::LineTo(p) = el {
                source.push(p);
            }
        });
    }
    if source.len() < 2 || source.len() > 16384 || target.len() < 2 {
        return Err(error("Profile transfer exceeds geometry limits"));
    }
    let sl = lengths(&source);
    let tl = lengths(target);
    let source_len = sl.last().copied().unwrap_or(0.0);
    let target_len = tl.last().copied().unwrap_or(0.0);
    if source_len < 1e-9 || target_len < 1e-9 {
        return Err(error("Degenerate profile transfer"));
    }
    let reverse = source[0].distance(target[target.len() - 1]) + source[source.len() - 1].distance(target[0])
        < source[0].distance(target[0]) + source[source.len() - 1].distance(target[target.len() - 1]);
    let map = |t: f64| {
        let length = t * source_len;
        let i = sl.partition_point(|s| *s < length).clamp(1, source.len() - 1);
        let u = ((length - sl[i - 1]) / (sl[i] - sl[i - 1]).max(1e-9)).clamp(0.0, 1.0);
        let point = source[i - 1].lerp(source[i], u);
        let mut best = (f64::INFINITY, 0.0);
        for (j, pair) in target.windows(2).enumerate() {
            let v = pair[1] - pair[0];
            let u = ((point - pair[0]).dot(v) / v.hypot2().max(1e-9)).clamp(0.0, 1.0);
            let dist = point.distance(pair[0].lerp(pair[1], u));
            if dist < best.0 {
                best = (dist, (tl[j] + u * (tl[j + 1] - tl[j])) / target_len);
            }
        }
        best.1
    };
    let remap = |points: &[(f64, f64, f64)], width: bool| -> Result<Vec<(f64, f64, f64)>> {
        // Bounded quadratic projection, at most 1M segment/knot comparisons.
        if points.len().saturating_mul(target.len()) > 1_000_000 {
            return Err(error("Pressure/tilt transfer work budget exceeded; originals unchanged"));
        }
        let mut out: Vec<_> = points
            .iter()
            .map(|&(t, a, b)| {
                let (a, b) = if width && reverse { (b, a) } else { (a, b) };
                (map(t), a, b)
            })
            .collect();
        if reverse {
            out.reverse();
        }
        if out.windows(2).any(|w| w[1].0 + 1e-7 < w[0].0) {
            return Err(error("Ambiguous pressure/tilt projection; originals unchanged"));
        }
        // Maintain discontinuities, stable ordering and all authored knot values.
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(out)
    };
    let profile = stroke.profile.as_ref().map(|p| remap(&p.points, true)).transpose()?;
    let nib = stroke.nib.as_ref().map(|p| remap(&p.points, false)).transpose()?;
    if let Some(stroke) = node.appearance.stroke_mut() {
        if let (Some(p), Some(points)) = (&mut stroke.profile, profile) {
            p.points = points;
        }
        if let (Some(p), Some(points)) = (&mut stroke.nib, nib) {
            p.points = points;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_color::{Color, Paint};
    use vectorcraft_doc::{Appearance, NibProfile, WidthProfile};
    use vectorcraft_geom::{PathData, SubPath};
    #[test]
    fn reversed_fit_preserves_pressure_knots_and_absolute_tilt() {
        let source = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let mut node = Node::path(
            vectorcraft_doc::NodeId(1),
            PathData::single(SubPath::polyline(&source, false)),
            Appearance::basic(Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 3.0),
        );
        let s = node.appearance.stroke_mut().unwrap();
        s.profile = Some(WidthProfile { points: vec![(0.0, 0.2, 0.8), (0.3, 1.0, 2.0), (1.0, 0.0, 0.0)] });
        s.nib = Some(NibProfile { points: vec![(0.0, 30.0, 0.5), (1.0, 60.0, 0.7)] });
        transfer(&mut node, &[Point::new(100.0, 1.0), Point::new(0.0, 1.0)]).unwrap();
        let s = node.appearance.stroke().unwrap();
        assert_eq!(s.profile.as_ref().unwrap().points, vec![(0.0, 0.0, 0.0), (0.7, 2.0, 1.0), (1.0, 0.8, 0.2)]);
        assert_eq!(s.nib.as_ref().unwrap().points, vec![(0.0, 60.0, 0.7), (1.0, 30.0, 0.5)]);
        assert!(valid(s));
    }
}
