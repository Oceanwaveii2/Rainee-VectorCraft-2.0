//! Portable canvas-backed glass. Original implementation: bounded CPU scattering cache,
//! GPU-composited refractive meshes. No framebuffer readbacks or continuous animation.
//! This is not Apple's proprietary compositor. Chrome outside the canvas stays opaque.
use std::sync::Arc;

use egui::{Color32, Rect, Shape, TextureHandle, pos2, vec2};

use crate::{VectorcraftApp, canvas::Xf, theme::Tokens};

const EDGE: usize = 384;
const MAX_SURFACES: usize = 24;
fn id() -> egui::Id {
    egui::Id::new("rainee-glass-material")
}
fn setting_id() -> egui::Id {
    egui::Id::new("rainee-glass-strength")
}

#[derive(Clone, Default)]
struct State {
    source: Option<Arc<egui::ColorImage>>,
    generation: u64,
    key: Vec<u64>,
    texture: Option<TextureHandle>,
    rebuilds: u64,
    surfaces: usize,
    prepare_ms: f64,
}

pub fn configure(ctx: &egui::Context, enabled: bool, strength: f64) {
    let value = if enabled && strength.is_finite() { (strength.clamp(0.0, 100.0) / 100.0) as f32 } else { 0.0 };
    ctx.data_mut(|d| {
        d.insert_temp(setting_id(), value);
        if value == 0.0 {
            d.remove::<State>(id());
        }
    });
}

fn strength(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp(setting_id())).unwrap_or(0.0)
}

pub fn needs_source(ctx: &egui::Context) -> bool {
    strength(ctx) > 0.0 && ctx.data(|d| d.get_temp::<State>(id())).is_none_or(|s| s.source.is_none())
}

/// Called only for a new canvas upload. Keep at most 384² pixels, not a second full raster.
pub fn capture(ctx: &egui::Context, img: &vectorcraft_render::Rendered) {
    if strength(ctx) == 0.0 || img.width == 0 || img.height == 0 {
        return;
    }
    let (w, h) = dimensions(img.width as f32, img.height as f32);
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let sx = x * img.width as usize / w;
            let sy = y * img.height as usize / h;
            let at = (sy * img.width as usize + sx) * 4;
            let Some(p) = img.pixels.get(at..at + 4) else { return };
            pixels.push(Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]));
        }
    }
    ctx.data_mut(|d| {
        let mut s = d.get_temp::<State>(id()).unwrap_or_default();
        s.source = Some(Arc::new(egui::ColorImage::new([w, h], pixels)));
        s.generation = s.generation.wrapping_add(1);
        d.insert_temp(id(), s);
    });
}

fn dimensions(w: f32, h: f32) -> (usize, usize) {
    let scale = EDGE as f32 / w.max(h).max(1.0);
    ((w * scale).round().clamp(1.0, EDGE as f32) as usize, (h * scale).round().clamp(1.0, EDGE as f32) as usize)
}

fn surface(shape: &Shape, fill: Color32) -> bool {
    match shape {
        Shape::Rect(r) => r.fill == fill && r.blur_width == 0.0 && r.rect.width() >= 80.0 && r.rect.height() >= 40.0,
        Shape::Vec(children) => children.iter().any(|s| surface(s, fill)),
        _ => false,
    }
}

// Dark material must stay charcoal over a white artboard, not turn into a gray sheet.
fn material_tint(panel: Color32, amount: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(panel.r(), panel.g(), panel.b(), (252.0 - amount.clamp(0.0, 1.0) * 20.0) as u8)
}

fn decorate(shape: &mut Shape, fill: Color32, panel: Color32, screen: Rect, texture: egui::TextureId, amount: f32) {
    match shape {
        Shape::Vec(children) => {
            for child in children {
                decorate(child, fill, panel, screen, texture, amount);
            }
        }
        Shape::Rect(r) if r.fill == fill && r.blur_width == 0.0 && r.rect.width() >= 80.0 && r.rect.height() >= 40.0 => {
            let mesh = lens(r.rect, r.corner_radius.nw as f32, screen, texture, amount);
            let mut tint = r.clone();
            tint.fill = material_tint(panel, amount);
            tint.stroke = egui::Stroke::new(1.0, Color32::from_white_alpha((18.0 + amount * 28.0) as u8));
            *shape = Shape::Vec(vec![Shape::mesh(mesh), Shape::Rect(tint)]);
        }
        _ => {}
    }
}

/// Run after windows are laid out. Replace only their shared material fill, never content,
/// controls, hit-testing, menus' text, toolbar icons or document colors.
pub fn finish(app: &VectorcraftApp, ctx: &egui::Context) {
    let amount = strength(ctx);
    if amount == 0.0 {
        return;
    }
    let fill = ctx.global_style().visuals.window_fill;
    let layers: Vec<_> = ctx.memory(|m| m.layer_ids().filter(|l| matches!(l.order, egui::Order::Middle | egui::Order::Foreground)).collect());
    let mut targets = Vec::new();
    ctx.graphics(|g| {
        for layer in &layers {
            if let Some(list) = g.get(*layer) {
                for (i, entry) in list.all_entries().enumerate() {
                    if surface(&entry.shape, fill) && targets.len() < MAX_SURFACES {
                        targets.push((*layer, egui::layers::ShapeIdx(i)));
                    }
                }
            }
        }
    });
    let mut state = ctx.data(|d| d.get_temp::<State>(id())).unwrap_or_default();
    state.surfaces = targets.len();
    if targets.is_empty() {
        ctx.data_mut(|d| d.insert_temp(id(), state));
        return;
    }
    let (Some(st), Some(view), Some(canvas), Some(k), Some(source)) =
        (app.session.active(), app.view(), app.canvas_rect, &app.canvas.key, state.source.as_ref())
    else {
        return;
    };
    // Never present another document's cached art, even for one frame.
    if k.doc != st.uid as usize {
        return;
    }
    let screen = ctx.content_rect();
    if !screen.is_finite() || screen.width() <= 0.0 || screen.height() <= 0.0 {
        return;
    }
    let xf = Xf::new(canvas, view);
    let t = Tokens::get(ctx);
    let bg = st.doc.setup.resolved_backdrop_rgb();
    let paper = st.doc.setup.resolved_artboard_color();
    let color_key = |rgb: [u8; 3]| u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 255]) as u64;
    let key = vec![
        state.generation,
        st.uid,
        st.revision,
        view.zoom.to_bits(),
        view.center.x.to_bits(),
        view.center.y.to_bits(),
        view.rotation.to_bits(),
        screen.width().to_bits() as u64,
        screen.height().to_bits() as u64,
        canvas.min.x.to_bits() as u64,
        canvas.min.y.to_bits() as u64,
        canvas.max.x.to_bits() as u64,
        canvas.max.y.to_bits() as u64,
        u32::from_le_bytes(t.panel.to_array()) as u64,
        amount.to_bits() as u64,
        color_key(bg),
        paper.map_or(0, color_key),
        app.ui.view.artboards as u64,
    ];
    if state.key != key || state.texture.is_none() {
        let start = crate::now_ms();
        let (w, h) = dimensions(screen.width(), screen.height());
        let mut rgba = image::RgbaImage::new(w as u32, h as u32);
        let source_rect = Rect::from_center_size(canvas.center(), vec2(k.w as f32 / k.ppp, k.h as f32 / k.ppp));
        let old = Xf { rect: source_rect, zoom: k.zoom, center: vectorcraft_geom::Point::new(k.cx, k.cy), rot: k.rot.to_radians() };
        for y in 0..h {
            for x in 0..w {
                let p = screen.min + vec2((x as f32 + 0.5) * screen.width() / w as f32, (y as f32 + 0.5) * screen.height() / h as f32);
                let mut rgb = [t.panel.r(), t.panel.g(), t.panel.b()];
                if canvas.contains(p) {
                    let doc = xf.to_doc(p);
                    rgb = if app.ui.view.artboards && st.doc.artboards.iter().any(|a| a.rect.contains(doc)) { paper.unwrap_or(bg) } else { bg };
                    let uv = if let Some([x0, y0, x1, y1]) = k.pixel {
                        pos2(((doc.x - x0 as f64) / (x1 - x0).max(1) as f64) as f32, ((doc.y - y0 as f64) / (y1 - y0).max(1) as f64) as f32)
                    } else {
                        let q = old.to_screen(doc);
                        pos2((q.x - source_rect.left()) / source_rect.width(), (q.y - source_rect.top()) / source_rect.height())
                    };
                    if (0.0..1.0).contains(&uv.x) && (0.0..1.0).contains(&uv.y) {
                        let sx = (uv.x * source.size[0] as f32) as usize;
                        let sy = (uv.y * source.size[1] as f32) as usize;
                        if let Some(c) = source.pixels.get(sy * source.size[0] + sx) {
                            let a = u32::from(c.a());
                            for (channel, src) in rgb.iter_mut().zip([c.r(), c.g(), c.b()]) {
                                *channel = (u32::from(src) + u32::from(*channel) * (255 - a) / 255).min(255) as u8;
                            }
                        }
                    }
                }
                rgba.put_pixel(x as u32, y as u32, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
            }
        }
        let blurred = image::imageops::blur(&rgba, 1.5 + amount * 2.5);
        let color = egui::ColorImage::from_rgba_unmultiplied([w, h], blurred.as_raw());
        match &mut state.texture {
            Some(tex) => tex.set(color, egui::TextureOptions::LINEAR),
            None => state.texture = Some(ctx.load_texture("rainee-glass-scattering", color, egui::TextureOptions::LINEAR)),
        }
        state.key = key;
        state.rebuilds += 1;
        state.prepare_ms = crate::now_ms() - start;
    }
    if let Some(tex) = &state.texture {
        ctx.graphics_mut(|g| {
            for (layer, index) in targets {
                if let Some(list) = g.get_mut(layer) {
                    list.mutate_shape(index, |entry| decorate(&mut entry.shape, fill, t.panel, screen, tex.id(), amount));
                }
            }
        });
    }
    ctx.data_mut(|d| d.insert_temp(id(), state));
}

fn lens(rect: Rect, radius: f32, screen: Rect, texture: egui::TextureId, amount: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture);
    const N: usize = 24;
    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    for y in 0..=N {
        let py = rect.top() + rect.height() * y as f32 / N as f32;
        let dy = (radius - (py - rect.top()).min(rect.bottom() - py)).max(0.0);
        let inset = radius - (radius * radius - dy * dy).max(0.0).sqrt();
        for x in 0..=N {
            let px = rect.left() + inset + (rect.width() - 2.0 * inset) * x as f32 / N as f32;
            let p = pos2(px, py);
            let edge = (px - rect.left()).min(rect.right() - px).min(py - rect.top()).min(rect.bottom() - py);
            let bend = (1.0 - edge / 24.0).clamp(0.0, 1.0).powi(2) * amount * 7.0;
            let delta = (rect.center() - p).normalized() * bend;
            let q = p + delta;
            let uv = pos2(((q.x - screen.left()) / screen.width()).clamp(0.0, 1.0), ((q.y - screen.top()) / screen.height()).clamp(0.0, 1.0));
            mesh.vertices.push(egui::epaint::Vertex { pos: p, uv, color: Color32::WHITE });
        }
    }
    for y in 0..N {
        for x in 0..N {
            let i = (y * (N + 1) + x) as u32;
            mesh.add_triangle(i, i + 1, i + (N + 1) as u32);
            mesh.add_triangle(i + 1, i + (N + 2) as u32, i + (N + 1) as u32);
        }
    }
    mesh
}

pub fn telemetry(ctx: &egui::Context) -> serde_json::Value {
    let s = ctx.data(|d| d.get_temp::<State>(id())).unwrap_or_default();
    serde_json::json!({"enabled":strength(ctx)>0.0,"surfaces":s.surfaces,"rebuilds":s.rebuilds,"prepareMs":s.prepare_ms,"maxTextureEdge":EDGE,"mode":"canvas-backed-scattering-and-lensing"})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_glass_stays_charcoal_even_above_white_paper() {
        for amount in [0.0, 0.55, 1.0] {
            let c = material_tint(Color32::from_gray(30), amount);
            let on_white = u32::from(c.r()) + 255 - u32::from(c.a());
            assert!(on_white <= 52, "bright artwork must not wash out the UI");
            assert!(c.a() < 255, "retain subtle background color transmission");
        }
    }

    #[test]
    fn window_material_caches_and_releases_on_opaque_mode() {
        use serde_json::json;
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.splash.open = false;
        app.run("file.new", json!({"width":400,"height":300})).unwrap();
        app.run("window.workspace", json!({"name":"Rainee 2.0"})).unwrap();
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        crate::theme::apply_workspace(&ctx, Default::default(), true, 65.0, 55.0, false);
        // The native harness tests the asynchronous renderer. Here provide a completed
        // canvas upload explicitly so the material/cache test is independent of scheduling.
        app.canvas_rect = Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0)));
        app.canvas.key =
            Some(crate::CacheKey { doc: app.session.active().unwrap().uid as usize, zoom: 1.0, w: 8, h: 8, ppp: 1.0, ..Default::default() });
        capture(&ctx, &vectorcraft_render::Rendered { width: 8, height: 8, pixels: [40, 80, 120, 255].repeat(64) });
        let mut time = 0.0;
        let mut frame = || {
            time += 0.25;
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))),
                    time: Some(time),
                    ..Default::default()
                },
                |ui| {
                    egui::Window::new("Glass test").fade_in(false).default_pos(pos2(450.0, 200.0)).show(ui.ctx(), |ui| {
                        ui.set_min_size(vec2(240.0, 180.0));
                        ui.label("Material test");
                    });
                    finish(&app, ui.ctx());
                },
            )
            .textures_delta
            .clear();
        };
        for _ in 0..5 {
            frame();
        }
        let first = telemetry(&ctx);
        assert!(first["surfaces"].as_u64().unwrap() >= 1, "{first}");
        assert!(first["rebuilds"].as_u64().unwrap() >= 1, "{first}");
        frame();
        assert_eq!(telemetry(&ctx)["rebuilds"], first["rebuilds"], "idle does not re-blur");
        configure(&ctx, false, 55.0);
        assert_eq!(telemetry(&ctx)["enabled"], false);
        assert!(ctx.data(|d| d.get_temp::<State>(id())).is_none());
    }
    #[test]
    fn lens_is_bounded_and_bends_only_sampling_not_geometry() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(220.0, 160.0));
        let screen = Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 600.0));
        let a = lens(rect, 12.0, screen, egui::TextureId::Managed(1), 0.0);
        let b = lens(rect, 12.0, screen, egui::TextureId::Managed(1), 1.0);
        assert!(a.is_valid() && b.is_valid());
        assert_eq!(a.vertices.len(), 625);
        assert!(a.vertices.iter().zip(&b.vertices).all(|(a, b)| a.pos == b.pos));
        assert!(a.vertices.iter().zip(&b.vertices).any(|(a, b)| a.uv != b.uv));
        assert!(
            b.vertices.iter().all(|v| v.pos.is_finite() && rect.contains(v.pos) && (0.0..=1.0).contains(&v.uv.x) && (0.0..=1.0).contains(&v.uv.y))
        );
    }
    #[test]
    fn disabled_material_releases_cache_and_sampling_is_bounded() {
        let ctx = egui::Context::default();
        configure(&ctx, true, 55.0);
        assert!(needs_source(&ctx));
        configure(&ctx, false, 100.0);
        assert!(!needs_source(&ctx));
        for (w, h) in [(1.0, 1.0), (8000.0, 4000.0), (10.0, 90000.0)] {
            let (w, h) = dimensions(w, h);
            assert!(w * h <= EDGE * EDGE && w > 0 && h > 0);
        }
    }
}
