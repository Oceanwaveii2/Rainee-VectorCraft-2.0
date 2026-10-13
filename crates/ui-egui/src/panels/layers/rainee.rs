//! Thumbnail-led desktop adaptation; editor hierarchy and commands stay authoritative.
use super::*;
use vectorcraft_color::BlendMode;

fn target_key() -> egui::Id {
    key("rainee-blend-target")
}

pub(super) fn expanded_target(ui: &Ui, document: u64) -> Option<NodeId> {
    let target = ui.data(|d| d.get_temp::<(u64, NodeId)>(target_key()));
    if target.is_some_and(|(uid, _)| uid != document) {
        ui.data_mut(|d| d.remove::<(u64, NodeId)>(target_key()));
        return None;
    }
    target.map(|(_, id)| id)
}

pub(super) fn inline_blend(app: &mut VectorcraftApp, ui: &mut Ui) {
    let Some((uid, id)) = ui.data(|d| d.get_temp::<(u64, NodeId)>(target_key())) else { return };
    let node = app.session.active().filter(|d| d.uid == uid).and_then(|d| d.doc.node(id)).cloned();
    let Some(node) = node else {
        ui.data_mut(|d| d.remove::<(u64, NodeId)>(target_key()));
        return;
    };
    let tokens = Tokens::get(ui.ctx());
    egui::Frame::NONE.fill(tokens.input).corner_radius(8).inner_margin(10).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(node.display_name()).strong());
            if ui.small_button("×").on_hover_text("Close blend controls").clicked() {
                ui.data_mut(|d| d.remove::<(u64, NodeId)>(target_key()));
            }
        });
        let editable = app
            .session
            .active()
            .is_some_and(|st| st.doc.ancestry(id).unwrap_or_default().iter().all(|ancestor| st.doc.node(*ancestor).is_none_or(|n| !n.locked)));
        ui.add_enabled_ui(editable && !node.locked, |ui| {
            let mut opacity = node.opacity * 100.0;
            ui.horizontal(|ui| {
                ui.label("Opacity");
                ui.label(format!("{opacity:.0}%"));
            });
            let width = ui.available_width().max(80.0);
            let (value, phase) = widgets::color_slider(ui, "rainee-layer-opacity", node.opacity, width, &|_| tokens.row_selected);
            if let Some(value) = value {
                opacity = value * 100.0;
                super::super::live_run(app, "Layer opacity", "transparency.set", json!({"ids":[id.0],"item":null,"opacity":opacity}), phase);
            }
            let mut group: u8 = super::super::pstate(ui.ctx(), "rainee-blend-group");
            ui.horizontal_wrapped(|ui| {
                for (i, label) in ["All", "Darken", "Lighten", "Contrast", "Difference", "Color"].iter().enumerate() {
                    if ui.selectable_label(group == i as u8, *label).clicked() {
                        group = i as u8;
                    }
                }
            });
            super::super::set_pstate(ui.ctx(), "rainee-blend-group", group);
            egui::ScrollArea::vertical().id_salt("rainee-blend-modes").max_height(150.0).show(ui, |ui| {
                for mode in BlendMode::ALL {
                    if group != 0 && mode.group() != group {
                        continue;
                    }
                    if ui.selectable_label(node.blend == mode, mode.label()).clicked() {
                        if let Err(e) = app.run("transparency.set", json!({"ids":[id.0],"item":null,"blend":mode.label()})) {
                            app.status(e);
                        }
                    }
                }
            });
        });
    });
}

pub(super) fn row(ui: &mut Ui, view: &View, item: &Row, out: &mut Out) {
    let n = &item.node;
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), view.h), Sense::click_and_drag());
    let highlighted = if view.rows.is_empty() { Some(n.id) == view.current && n.is_layer() } else { view.rows.contains(&n.id) };
    ui.painter().rect_filled(
        r.shrink(2.0),
        7.0,
        if highlighted {
            view.t.row_selected
        } else if resp.hovered() {
            view.t.hover
        } else {
            view.t.panel
        },
    );
    if view.reveal == Some(n.id) {
        ui.scroll_to_rect(r, None);
    }
    let mut x = r.left() + 7.0 + item.depth as f32 * 10.0;
    let mut disclosure_clicked = false;
    if item.opens {
        let dr = egui::Rect::from_center_size(egui::pos2(x + 6.0, r.center().y), vec2(14.0, 24.0));
        icons::paint(ui, if item.open { "chevron-down" } else { "chevron-right" }, dr, view.t.text_dim);
        if ui.interact(dr, ui.id().with(("rainee-disclosure", n.id.0)), Sense::click()).clicked() {
            disclosure_clicked = true;
            out.toggle.push((n.id, !item.open));
        }
    }
    x += 16.0;
    let thumb_r = row_thumb(ui, view, n, r, x);
    x = thumb_r.right() + 9.0;
    if n.locked {
        icons::paint(ui, "lock", egui::Rect::from_min_size(thumb_r.left_top(), vec2(14.0, 14.0)), view.t.text_dim);
    }
    let name_r = egui::Rect::from_min_max(egui::pos2(x, r.top() + 4.0), egui::pos2((r.right() - 57.0).max(x), r.bottom() - 4.0));
    let renaming: Option<(u64, String)> = ui.data(|d| d.get_temp(rename_id()));
    if let Some((id, mut name)) = renaming.filter(|(id, _)| *id == n.id.0) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(name_r));
        let edit = child.text_edit_singleline(&mut name);
        if !edit.has_focus() && !edit.lost_focus() {
            edit.request_focus();
        }
        if edit.lost_focus() {
            ui.data_mut(|d| d.remove::<(u64, String)>(rename_id()));
            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) && name != n.display_name() {
                out.actions.push(("layer.setProps".into(), json!({"ids":[id],"name":name})));
            }
        } else {
            ui.data_mut(|d| d.insert_temp(rename_id(), (id, name)));
        }
    } else {
        ui.painter().with_clip_rect(name_r).text(
            egui::pos2(x, r.center().y),
            egui::Align2::LEFT_CENTER,
            n.display_name(),
            egui::FontId::proportional(14.5),
            if item.dim { view.t.text_dim } else { view.t.text },
        );
    }
    let badge_r = egui::Rect::from_center_size(egui::pos2(r.right() - 41.0, r.center().y), vec2(22.0, 24.0));
    let badge = ui.interact(badge_r, ui.id().with(("rainee-blend", n.id.0)), Sense::click());
    let label = blend_badge(n.blend);
    ui.painter().text(badge_r.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), view.t.text);
    if badge.on_hover_text(format!("{} · {:.0}% opacity", n.blend.label(), n.opacity * 100.0)).clicked() {
        if let Some(uid) = ui.data(|d| d.get_temp::<u64>(egui::Id::new("layers-doc"))) {
            ui.data_mut(|d| {
                if d.get_temp::<(u64, NodeId)>(target_key()) == Some((uid, n.id)) {
                    d.remove::<(u64, NodeId)>(target_key());
                } else {
                    d.insert_temp(target_key(), (uid, n.id));
                }
            });
        }
    }
    let eye_r = egui::Rect::from_center_size(egui::pos2(r.right() - 14.0, r.center().y), vec2(20.0, 24.0));
    icons::paint(ui, if n.visible { "eye" } else { "eye-off" }, eye_r, view.t.text_dim);
    let eye = ui.interact(eye_r, ui.id().with(("rainee-visible", n.id.0)), Sense::click());
    if eye.on_hover_text(if n.visible { "Hide layer / object" } else { "Show layer / object" }).clicked() {
        out.actions.push(("layer.setProps".into(), json!({"ids":[n.id.0],"visible":!n.visible})));
    }
    let mods = ui.input(|i| i.modifiers);
    if resp.clicked()
        && !disclosure_clicked
        && !badge_r.contains(resp.interact_pointer_pos().unwrap_or_default())
        && !eye_r.contains(resp.interact_pointer_pos().unwrap_or_default())
    {
        out.click = Some((n.id, mods));
    }
    if resp.double_clicked() && name_r.contains(resp.interact_pointer_pos().unwrap_or_default()) {
        ui.data_mut(|d| d.insert_temp(rename_id(), (n.id.0, n.display_name())));
    }
    resp.context_menu(|ui| {
        if ui.button("Target appearance").clicked() {
            out.actions.push(("layer.target".into(), json!({"id":n.id.0})));
            ui.close();
        }
        if ui.button(if n.locked { "Unlock" } else { "Lock" }).clicked() {
            out.actions.push(("layer.setProps".into(), json!({"ids":[n.id.0],"locked":!n.locked})));
            ui.close();
        }
        if ui.button("Layer / Object Options").clicked() {
            out.actions.push(("ui.layerOptions".into(), json!({"ids":[n.id.0]})));
            ui.close();
        }
    });
    if resp.drag_started() {
        let ids = if view.rows.contains(&n.id) { view.rows.clone() } else { vec![n.id] };
        egui::DragAndDrop::set_payload(ui.ctx(), LayersDrag::Rows(ids));
    }
    drop_target(ui, view, n, r, x, item.open, &resp, out);
}

fn blend_badge(mode: BlendMode) -> &'static str {
    match mode.label() {
        "Normal" => "N",
        "Multiply" => "M",
        "Screen" => "S",
        "Overlay" => "O",
        "Darken" => "D",
        "Lighten" => "L",
        "Color Burn" => "CB",
        "Color Dodge" => "CD",
        "Soft Light" => "SL",
        "Hard Light" => "HL",
        "Difference" => "Df",
        "Exclusion" => "Ex",
        "Hue" => "H",
        "Saturation" => "Sa",
        "Color" => "C",
        "Luminosity" => "Lu",
        _ => "•",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rainee_rows_and_inline_controls_are_non_mutating_until_input() {
        for dpi in [1.0, 2.0] {
            let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
            app.run("file.new", json!({})).unwrap();
            app.run("shape.rectangle", json!({"x":10,"y":10,"width":50,"height":30})).unwrap();
            app.run("window.workspace", json!({"name":"Rainee 2.0"})).unwrap();
            let before = app.session.doc().unwrap().doc.clone();
            let st = app.session.active().unwrap();
            let target = (st.uid, st.doc.layers.first().unwrap().id);
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            ctx.set_pixels_per_point(dpi);
            ctx.data_mut(|d| d.insert_temp(target_key(), target));
            let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(420.0, 900.0))), ..Default::default() };
            let mut output = ctx.run_ui(raw, |ui| super::super::show(&mut app, ui));
            output.textures_delta.clear();
            assert_eq!(app.session.doc().unwrap().doc, before);
            assert!(!output.shapes.is_empty());
            app.run("file.new", json!({})).unwrap();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| inline_blend(&mut app, ui));
            output.textures_delta.clear();
            assert!(ctx.data(|d| d.get_temp::<(u64, NodeId)>(target_key())).is_none(), "must clear cross-document target");
        }
    }
}
