//! Classic Nostalgic Launch Screen with retro loading ticker and custom artwork display.
use crate::VectorcraftApp;
use crate::theme::Tokens;
use egui::{Align2, Color32, CornerRadius, Frame, Margin, Pos2, Rect, Stroke, vec2};
use std::path::PathBuf;

pub struct SplashState {
    pub open: bool,
    pub start_time: f64,
    pub texture: Option<egui::TextureHandle>,
    pub loaded_path: Option<PathBuf>,
}

impl Default for SplashState {
    fn default() -> Self {
        Self { open: true, start_time: 0.0, texture: None, loaded_path: None }
    }
}

const SPLASH_DURATION_SECS: f64 = 9.5;

/// Stages for the retro Adobe-style startup ticker
const INIT_STAGES: &[(f64, &str)] = &[
    (0.00, "Reading user preferences and workspaces..."),
    (1.40, "Initializing predictive inking & stroke modeler..."),
    (3.00, "Calibrating infinite spatial canvas & custom artboards..."),
    (4.80, "Loading procedural brush libraries & color palettes..."),
    (6.60, "Connecting native display surface, pen digitizers & ARAP solvers..."),
    (8.40, "Ready."),
];

const DEFAULT_SPLASH_BYTES: &[u8] = include_bytes!("../../../../assets/splash_placeholder.png");

pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.splash.open {
        return;
    }

    // Support drag-and-drop image directly onto splash to customize
    let mut dropped_image: Option<PathBuf> = None;
    ctx.input(|i| {
        for file in &i.raw.dropped_files {
            let path = file.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                dropped_image = Some(path.to_path_buf());
                break;
            }
        }
    });
    if let Some(new_art) = dropped_image {
        app.ui.splash_image_path = Some(new_art.to_string_lossy().to_string());
        app.splash.texture = None;
        app.splash.loaded_path = None;
    }

    let now = ctx.input(|i| i.time);
    if app.splash.start_time <= 0.0 {
        app.splash.start_time = now;
    }

    let elapsed = now - app.splash.start_time;
    if elapsed >= SPLASH_DURATION_SECS {
        app.splash.open = false;
        return;
    }

    // Load custom image if configured, or use embedded binary fallback
    let custom_path = app.ui.splash_image_path.as_deref().filter(|p| !p.trim().is_empty());
    let desired_path = custom_path.map(PathBuf::from);

    if app.splash.texture.is_none() || app.splash.loaded_path != desired_path {
        let loaded_img = desired_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .or_else(|| image::load_from_memory(DEFAULT_SPLASH_BYTES).ok());

        if let Some(dyn_img) = loaded_img {
            let rgba = dyn_img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_img = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let tex = ctx.load_texture("splash_custom_art", color_img, egui::TextureOptions::LINEAR);
            app.splash.texture = Some(tex);
            app.splash.loaded_path = desired_path;
        }
    }

    // Repaint to keep the progress animation moving smoothly
    ctx.request_repaint();

    // User can click anywhere (after brief initial display) or press Escape/Space to dismiss immediately
    let dismissed_by_key = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Space));
    let dismissed_by_click = elapsed > 0.15 && ctx.input(|i| i.pointer.any_pressed());
    if dismissed_by_key || dismissed_by_click {
        app.splash.open = false;
        return;
    }

    let _t = Tokens::get(ctx);
    let card_w = 620.0;
    let card_h = 390.0;

    let modal_id = egui::Id::new("retro_splash_modal");
    let screen = ctx.content_rect();

    egui::Area::new(modal_id).order(egui::Order::Foreground).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ctx, |ui| {
        // Darken backdrop slightly
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(110));

        let frame = Frame::window(&ctx.global_style())
            .fill(Color32::from_rgb(22, 25, 33))
            .stroke(Stroke::new(1.0, Color32::from_rgb(52, 60, 78)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(0));

        frame.show(ui, |ui| {
            ui.set_width(card_w);
            ui.set_height(card_h);

            let rect = ui.max_rect();
            let art_h = 295.0;
            let art_rect = Rect::from_min_size(rect.min, vec2(card_w, art_h));

            // Paint artwork image or gradient banner
            if let Some(tex) = &app.splash.texture {
                ui.painter().image(tex.id(), art_rect, Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
            } else {
                ui.painter().rect_filled(art_rect, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, Color32::from_rgb(30, 36, 48));
            }

            // Title overlay badge
            let title_pos = art_rect.min + vec2(20.0, 24.0);
            ui.painter().text(title_pos, Align2::LEFT_TOP, "Rainee VectorCraft", egui::FontId::proportional(26.0), Color32::WHITE);
            ui.painter().text(
                title_pos + vec2(2.0, 32.0),
                Align2::LEFT_TOP,
                format!("Edition v{} • Creative Engine", env!("CARGO_PKG_VERSION")),
                egui::FontId::proportional(12.0),
                Color32::from_rgb(140, 215, 255),
            );

            // Artist / Contributor credit watermark on the bottom right of the art card
            let credit = if custom_path.is_some() { "Custom Artwork Studio Edition" } else { "Featured Artwork • Rainee Design Core" };
            ui.painter().text(
                art_rect.right_bottom() - vec2(16.0, 14.0),
                Align2::RIGHT_BOTTOM,
                credit,
                egui::FontId::proportional(11.0),
                Color32::from_white_alpha(170),
            );

            // Bottom control / initialization ticker bar
            let ticker_rect = Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + art_h), rect.max);
            ui.painter().rect_filled(ticker_rect, CornerRadius { nw: 0, ne: 0, sw: 10, se: 10 }, Color32::from_rgb(16, 18, 24));

            // Determine ticker step
            let current_stage = INIT_STAGES.iter().rfind(|(time, _)| elapsed >= *time).map(|(_, text)| *text).unwrap_or("Starting engine...");

            let progress = (elapsed / SPLASH_DURATION_SECS).clamp(0.0, 1.0) as f32;

            // Ticker status text
            ui.painter().text(
                ticker_rect.min + vec2(20.0, 22.0),
                Align2::LEFT_TOP,
                current_stage,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(205, 215, 230),
            );

            // "Click anywhere to continue" hint
            ui.painter().text(
                ticker_rect.right_top() + vec2(-20.0, 22.0),
                Align2::RIGHT_TOP,
                "Click to skip",
                egui::FontId::proportional(11.0),
                Color32::from_white_alpha(90),
            );

            // Smooth progress bar
            let bar_margin = 20.0;
            let bar_y = ticker_rect.max.y - 20.0;
            let bar_w = card_w - (bar_margin * 2.0);
            let bar_bg = Rect::from_min_size(Pos2::new(rect.min.x + bar_margin, bar_y), vec2(bar_w, 4.0));
            let bar_fg = Rect::from_min_size(Pos2::new(rect.min.x + bar_margin, bar_y), vec2(bar_w * progress, 4.0));

            ui.painter().rect_filled(bar_bg, CornerRadius::same(2), Color32::from_rgb(32, 38, 50));
            ui.painter().rect_filled(bar_fg, CornerRadius::same(2), Color32::from_rgb(0, 180, 240));
        });
    });
}
