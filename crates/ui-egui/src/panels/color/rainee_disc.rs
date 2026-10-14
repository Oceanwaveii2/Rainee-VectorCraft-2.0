//! Smooth square-to-disc mapping keeps every saturation/value pair reachable.
use egui::{Color32, Mesh, Pos2, Shape, Vec2, vec2};
use vectorcraft_color::Color;

pub(super) fn to_disc(p: Vec2) -> Vec2 {
    vec2(p.x * (1.0 - p.y * p.y * 0.5).max(0.0).sqrt(), p.y * (1.0 - p.x * p.x * 0.5).max(0.0).sqrt())
}
pub(super) fn to_square(p: Vec2) -> Vec2 {
    let p = p / p.length().max(1.0);
    let root = 2.0_f32.sqrt();
    let f = |x: f32, y: f32| {
        (((2.0 + x * x - y * y + 2.0 * root * x).max(0.0).sqrt() - (2.0 + x * x - y * y - 2.0 * root * x).max(0.0).sqrt()) * 0.5).clamp(-1.0, 1.0)
    };
    vec2(f(p.x, p.y), f(p.y, p.x))
}
pub(super) fn mesh(center: Pos2, radius: f32, hue: f32) -> Shape {
    const N: u32 = 32;
    let mut mesh = Mesh::default();
    for y in 0..=N {
        for x in 0..=N {
            let (s, v) = (x as f32 / N as f32, 1.0 - y as f32 / N as f32);
            let p = center + to_disc(vec2(s * 2.0 - 1.0, 1.0 - v * 2.0)) * radius;
            let [r, g, b, _] = Color::from_hsb(hue, s, v).to_rgba8(1.0);
            mesh.colored_vertex(p, Color32::from_rgb(r, g, b));
        }
    }
    for y in 0..N {
        for x in 0..N {
            let i = y * (N + 1) + x;
            mesh.add_triangle(i, i + 1, i + N + 1);
            mesh.add_triangle(i + 1, i + N + 2, i + N + 1);
        }
    }
    Shape::mesh(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_saturation_value_extremes_and_interior_picks_remain_reachable() {
        for p in [vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0), vec2(-0.6, 0.35)] {
            let d = to_disc(p);
            assert!(d.length() <= 1.00001);
            assert!((to_square(d) - p).length() < 0.001, "{p:?}");
        }
        assert!(matches!(mesh(Pos2::ZERO, 100.0, 40.0), Shape::Mesh(m) if m.is_valid()));
    }
}
