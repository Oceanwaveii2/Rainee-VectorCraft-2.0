//! Bounded desktop columns. Child content never feeds its desired width back into the window.
use super::*;
use egui::{Color32, Rect, Stroke, pos2};

pub(super) fn show(app: &mut VectorcraftApp, ui: &mut Ui, studio: &mut Studio) {
    let width = ui.available_width().max(1.0);
    let height = ui.available_height().clamp(200.0, 720.0);
    let (body, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let gap = 12.0;
    let nav = (width * 0.20).clamp(142.0, 205.0).min(width * 0.28);
    let settings = (width * 0.32).min(360.0);
    let middle = body.left() + nav + gap;
    let right = middle + settings + gap;
    let rects = [
        Rect::from_min_max(body.min, pos2(body.left() + nav, body.bottom())),
        Rect::from_min_max(pos2(middle, body.top()), pos2(middle + settings, body.bottom())),
        Rect::from_min_max(pos2(right, body.top()), body.max),
    ];
    let t = Tokens::get(ui.ctx());
    for x in [middle - gap * 0.5, right - gap * 0.5] {
        ui.painter().line_segment([pos2(x, body.top()), pos2(x, body.bottom())], Stroke::new(1.0, t.divider));
    }
    for (i, rect) in rects.into_iter().enumerate() {
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt(("studio-column", i)).max_rect(rect));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        child.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        match i {
            0 => {
                egui::ScrollArea::vertical()
                    .id_salt("studio-navigation")
                    .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        let mut rects = Vec::new();
                        for (page, name) in studio_controls::PAGES.iter().enumerate() {
                            let (r, hit) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                            rects.push(((*name).to_owned(), [r.left(), r.top(), r.width(), r.height()]));
                            let active = page == studio.page;
                            ui.painter().rect_filled(
                                r,
                                6.0,
                                if active {
                                    t.row_selected
                                } else if hit.hovered() {
                                    t.hover
                                } else {
                                    Color32::TRANSPARENT
                                },
                            );
                            crate::icons::paint(
                                ui,
                                studio_controls::ICONS[page],
                                Rect::from_center_size(pos2(r.left() + 15.0, r.center().y), vec2(16.0, 16.0)),
                                if active { Color32::WHITE } else { t.text_dim },
                            );
                            ui.painter().text(
                                pos2(r.left() + 31.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                *name,
                                egui::FontId::proportional(13.0),
                                if active { Color32::WHITE } else { t.text },
                            );
                            if hit.on_hover_text(*name).clicked() {
                                studio.page = page;
                            }
                        }
                        set_pstate(ui.ctx(), "studio-section-rects", rects);
                    });
            }
            1 => {
                egui::ScrollArea::vertical()
                    .id_salt(("studio-settings", studio.page))
                    .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        ui.set_max_width(rect.width());
                        studio_controls::show(app, ui, studio);
                    });
            }
            _ => {
                child.label(egui::RichText::new("Stroke preview").color(t.text_dim));
                let (r, _) = child.allocate_exact_size(vec2(child.available_width(), 90.0), Sense::hover());
                if let Some(tex) = preview_tone(&child, &studio.draft, r.size(), true) {
                    child.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                child.add_space(12.0);
                drawing_pad(&mut child, studio);
            }
        }
    }
}
