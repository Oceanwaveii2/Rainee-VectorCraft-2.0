//! Nested zoom levels crossfade without changing document grid or snap spacing.
use super::{Color32, Point, Pos2, Rect, Shape, Stroke, Xf, pos2, vec2};

pub(super) fn level(base: f64, zoom: f64) -> Option<(f64, f32)> {
    if !base.is_finite() || base <= 0.0 || !zoom.is_finite() || zoom <= 0.0 {
        return None;
    }
    let octave = (24.0 / (base * zoom)).log2().ceil().clamp(-60.0, 60.0);
    let major = base * 2.0_f64.powf(octave);
    let fade = ((major * zoom * 0.5 - 12.0) / 12.0).clamp(0.0, 1.0) as f32;
    Some((major, fade))
}

pub(super) fn paint(p: &egui::Painter, xf: &Xf, base: f64, color: Color32, dots: bool) {
    let Some((major, fade)) = level(base, xf.zoom) else { return };
    let corners = [xf.rect.left_top(), xf.rect.right_top(), xf.rect.right_bottom(), xf.rect.left_bottom()].map(|q| xf.to_doc(q));
    let view = corners.iter().fold(Rect::from_points(corners[0], corners[0]), |b, q| b.union_pt(*q));
    if ![view.x0, view.y0, view.x1, view.y1].iter().all(|v| v.is_finite()) {
        return;
    }
    let ppp = p.ctx().pixels_per_point().max(0.1);
    let align = |q: Pos2| pos2((q.x * ppp).floor() / ppp + 0.5 / ppp, (q.y * ppp).floor() / ppp + 0.5 / ppp);
    let axis = |lo: f64, hi: f64, step: f64| {
        let first = (lo / step).floor();
        (0..2048u32).map(move |i| (first + f64::from(i)) * step).take_while(move |v| *v <= hi + step)
    };
    if !dots {
        let stroke = |v: f64| Stroke::new(1.0 / ppp, color.gamma_multiply(if (v / major - (v / major).round()).abs() < 1e-6 { 1.0 } else { fade }));
        for x in axis(view.x0, view.x1, major * 0.5) {
            let a = xf.to_screen(Point::new(x, view.y0));
            let b = xf.to_screen(Point::new(x, view.y1));
            p.line_segment(if xf.rot.abs() < 1e-9 { [align(a), align(b)] } else { [a, b] }, stroke(x));
        }
        for y in axis(view.y0, view.y1, major * 0.5) {
            let a = xf.to_screen(Point::new(view.x0, y));
            let b = xf.to_screen(Point::new(view.x1, y));
            p.line_segment(if xf.rot.abs() < 1e-9 { [align(a), align(b)] } else { [a, b] }, stroke(y));
        }
    }
    let step = major * 0.5;
    let mut mesh = egui::Mesh::default();
    let is_major = |v: f64| (v / major - (v / major).round()).abs() < 1e-6;
    let mut count = 0;
    'dots: for x in axis(view.x0, view.x1, step) {
        for y in axis(view.y0, view.y1, step) {
            if count >= 65_536 {
                break 'dots;
            }
            let major_dot = is_major(x) && is_major(y);
            if !dots && major_dot {
                continue;
            }
            let screen = xf.to_screen(Point::new(x, y));
            let dot_px = ppp.round().max(1.0);
            let offset = if (dot_px as u32).is_multiple_of(2) { 0.0 } else { 0.5 };
            let q = pos2(((screen.x * ppp).floor() + offset) / ppp, ((screen.y * ppp).floor() + offset) / ppp);
            if !xf.rect.contains(q) {
                continue;
            }
            let alpha = if major_dot { 1.0 } else { fade };
            if alpha <= 0.0 {
                continue;
            }
            mesh.add_colored_rect(egui::Rect::from_center_size(q, vec2(dot_px / ppp, dot_px / ppp)), color.gamma_multiply(alpha));
            count += 1;
        }
    }
    if !mesh.is_empty() {
        p.add(Shape::mesh(mesh));
    }
}

/// Pixel Preview is a measurement grid: never replace its one-document-pixel cells with LOD.
pub(super) fn paint_pixels(p: &egui::Painter, xf: &Xf, color: Color32) {
    let points = [xf.rect.left_top(), xf.rect.right_top(), xf.rect.right_bottom(), xf.rect.left_bottom()].map(|q| xf.to_doc(q));
    let bounds = points.iter().fold(Rect::from_points(points[0], points[0]), |b, q| b.union_pt(*q));
    if ![bounds.x0, bounds.y0, bounds.x1, bounds.y1].iter().all(|v| v.is_finite()) {
        return;
    }
    let ppp = p.ctx().pixels_per_point().max(0.1);
    let align = |q: Pos2| pos2((q.x * ppp).floor() / ppp + 0.5 / ppp, (q.y * ppp).floor() / ppp + 0.5 / ppp);
    for axis in [false, true] {
        let (lo, hi) = if axis { (bounds.y0, bounds.y1) } else { (bounds.x0, bounds.x1) };
        for i in 0..4096 {
            let at = lo.floor() + f64::from(i);
            if at > hi + 1.0 {
                break;
            }
            let (a, b) =
                if axis { (Point::new(bounds.x0, at), Point::new(bounds.x1, at)) } else { (Point::new(at, bounds.y0), Point::new(at, bounds.y1)) };
            let (a, b) = (xf.to_screen(a), xf.to_screen(b));
            p.line_segment(if xf.rot.abs() < 1e-9 { [align(a), align(b)] } else { [a, b] }, Stroke::new(1.0 / ppp, color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_spacing_at_extreme_zoom_and_continuous_nested_transition() {
        for zoom in [0.0001, 0.01, 0.1, 1.0, 32.0, 10000.0] {
            let (step, fade) = level(72.0 / 8.0, zoom).unwrap();
            assert!((24.0 - 1e-8..48.0 + 1e-8).contains(&(step * zoom)));
            assert!((0.0..=1.0).contains(&fade));
        }
        let (a, fa) = level(9.0, 48.0 / 9.0 - 1e-7).unwrap();
        let (b, fb) = level(9.0, 48.0 / 9.0 + 1e-7).unwrap();
        assert_eq!(a, b * 2.0);
        assert!(fa > 0.99999 && fb < 1e-6);
        assert!(level(0.0, 1.0).is_none());
        assert!(level(1.0, f64::NAN).is_none());
    }
}
