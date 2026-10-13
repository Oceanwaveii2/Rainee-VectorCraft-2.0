//! Owner-reference layout: relocate existing panels; never duplicate editor property state.
use crate::{VectorcraftApp, floating, panels, state::UiState, theme::Tokens};
use egui::{Layout, Stroke, Ui, vec2};

const COLORS: [(&str, &str); 2] = [("color", "Color"), ("swatches", "Swatches")];
const APPEARANCE: [(&str, &str); 3] = [("stroke", "Stroke"), ("transparency", "Transparency"), ("gradient", "Gradient")];

pub(crate) fn owns(state: &UiState, id: &str) -> bool {
    state.left_working_dock
        && matches!(id, "color" | "swatches" | "stroke" | "transparency" | "gradient" | "brushes")
        && floating::group_of(state, id).is_none()
}

fn group(app: &mut VectorcraftApp, ui: &mut Ui, tabs: &[(&str, &str)], active: usize, height: f32) -> usize {
    let upgraded = app.ui.procreate_panels;
    let tokens = Tokens::get(ui.ctx());
    let mut active = active.min(tabs.len().saturating_sub(1));
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), Layout::top_down(egui::Align::Min), |ui| {
        ui.set_min_height(height);
        if upgraded {
            Tokens::style_rainee(ui);
        }
        ui.horizontal(|ui| {
            for (i, (_, label)) in tabs.iter().enumerate() {
                if ui.selectable_label(active == i, tl!(*label)).clicked() {
                    active = i;
                }
            }
            let rect = ui.available_rect_before_wrap();
            panels::panel_menu(app, ui, tabs[active].0, egui::Rect::from_min_size(rect.right_top() - vec2(18.0, 0.0), vec2(18.0, 18.0)));
        });
        let id = tabs[active].0;
        if id == "color" {
            ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("rainee-color-field-height"), (height - 180.0).clamp(70.0, 220.0)));
        }
        egui::ScrollArea::vertical().id_salt(("working-body", id)).auto_shrink([false, false]).max_height((height - 30.0).max(12.0)).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            if floating::group_of(&app.ui, id).is_some() {
                ui.label(tl!("Panel is floating"));
            } else {
                egui::Frame::NONE
                    .fill(tokens.panel)
                    .corner_radius(if upgraded { 10 } else { 0 })
                    .inner_margin(if upgraded { 10 } else { 6 })
                    .show(ui, |ui| panels::show_icon_panel(app, ui, id));
            }
        });
        let bottom = ui.min_rect().left_bottom();
        ui.painter().line_segment([bottom, bottom + vec2(ui.available_width(), 0.0)], Stroke::new(1.0, tokens.border));
    });
    active
}

pub(crate) fn color_height(ctx: &egui::Context) -> f32 {
    ctx.data(|data| data.get_temp::<f32>(egui::Id::new("rainee-color-field-height"))).unwrap_or(100.0)
}

pub(crate) fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    if !app.ui.left_working_dock {
        return;
    }
    // Existing Window-panel shortcuts target the relocated surface, not a duplicate popup.
    if let Some(id) = app.ui.open_panel.clone().filter(|id| owns(&app.ui, id)) {
        if let Some(i) = COLORS.iter().position(|p| p.0 == id) {
            app.ui.left_color_tab = i;
        }
        if let Some(i) = APPEARANCE.iter().position(|p| p.0 == id) {
            app.ui.left_appearance_tab = i;
        }
        app.ui.open_panel = None;
    }
    let tokens = Tokens::get(ui.ctx());
    egui::Panel::left("rainee-working-dock")
        .resizable(true)
        .default_size(if app.ui.procreate_panels { 320.0 } else { 260.0 })
        .size_range(220.0..=420.0)
        .frame(egui::Frame::NONE.fill(tokens.panel).stroke(Stroke::new(1.0, tokens.border)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let h = ui.available_height().max(120.0);
            app.ui.left_color_tab = group(app, ui, &COLORS, app.ui.left_color_tab, h * 0.38);
            let upgraded = app.ui.procreate_panels;
            app.ui.left_appearance_tab = group(app, ui, &APPEARANCE, app.ui.left_appearance_tab, h * if upgraded { 0.20 } else { 0.37 });
            group(app, ui, &[("brushes", if upgraded { "Brush Library" } else { "Brushes" })], 0, h * if upgraded { 0.42 } else { 0.25 });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn new_workspace_persists_layout_without_editing_documents() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width":300,"height":300})).unwrap();
        let before = app.session.doc().unwrap().doc.clone();
        app.run("window.workspace", json!({"name":crate::workspaces::RAINEE_1_0})).unwrap();
        assert!(app.ui.left_working_dock && app.ui.toolbar_double && app.ui.control_bar);
        assert!(owns(&app.ui, "color") && owns(&app.ui, "brushes") && !owns(&app.ui, "align"));
        app.ui.left_color_tab = 1;
        app.ui.left_appearance_tab = 2;
        app.run("window.workspace.new", json!({"name":"My Drawing"})).unwrap();
        app.run("window.workspace", json!({"name":"Essentials"})).unwrap();
        assert!(!app.ui.left_working_dock);
        app.run("window.workspace", json!({"name":"My Drawing"})).unwrap();
        assert_eq!((app.ui.left_color_tab, app.ui.left_appearance_tab), (1, 2));
        let state: UiState = serde_json::from_value(serde_json::to_value(&app.ui).unwrap()).unwrap();
        assert!(state.left_working_dock);
        assert_eq!(app.session.doc().unwrap().doc, before);
    }
    #[test]
    fn panel_shortcuts_route_to_existing_left_surface() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        app.run("window.workspace", json!({"name":crate::workspaces::RAINEE_1_0})).unwrap();
        app.ui.open_panel = Some("gradient".into());
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| show(&mut app, ui));
        output.textures_delta.clear();
        assert_eq!(app.ui.left_appearance_tab, 2);
        assert!(app.ui.open_panel.is_none());
    }

    #[test]
    fn owner_layout_reserves_a_usable_canvas_at_low_and_high_dpi() {
        for scale in [1.0, 2.0] {
            let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), crate::Services::default());
            app.fonts_ready = true;
            app.run("file.new", json!({"width":300,"height":300})).unwrap();
            app.run("window.workspace", json!({"name":crate::workspaces::RAINEE_1_0})).unwrap();
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            ctx.set_pixels_per_point(scale);
            let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1440.0, 900.0))), ..Default::default() };
            let mut output = ctx.run_ui(raw, |ui| app.ui(ui));
            output.textures_delta.clear();
            let canvas = app.canvas_rect.expect("drawing canvas");
            assert!(canvas.width() > 500.0 && canvas.height() > 600.0, "{canvas:?}");
            assert!(canvas.left() > 270.0, "paint dock precedes canvas");
            assert!((70.0..=220.0).contains(&color_height(&ctx)));
        }
    }
}
