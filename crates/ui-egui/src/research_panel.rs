//! UI and MCP use the same registered engine actions.
use crate::VectorcraftApp;
use serde_json::{Value, json};
fn run(app: &mut VectorcraftApp, ctx: &egui::Context, id: &str, p: Value) {
    let e = app.run(id, p).err();
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("research-error"), e);
    });
}
pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.research_open {
        return;
    }
    let mut open = true;
    let status = app.session.research_tools.status();
    egui::Window::new("Research Tools · Experimental").drag_area(egui::WindowDrag::TitleBar).open(&mut open).default_width(460.0).default_height(600.0).show(ctx,|ui|{
        let mut tab=ctx.data(|d|d.get_temp::<usize>(egui::Id::new("research-tab"))).unwrap_or(if status["kind"]=="junction"{1}else{2});
        ui.horizontal(|ui|{ui.selectable_value(&mut tab,0,"Prediction");ui.selectable_value(&mut tab,1,"Junctions");ui.selectable_value(&mut tab,2,"Natural brushes");});
        ctx.data_mut(|d|{d.insert_temp(egui::Id::new("research-tab"),tab);});
        ui.label(format!("State: {}",status["state"].as_str().unwrap_or("idle")));
        ui.label(status["message"].as_str().unwrap_or(""));
        if let Some(Some(e))=ctx.data(|d|d.get_temp::<Option<String>>(egui::Id::new("research-error"))){ui.label(format!("Error: {e}"));}
        if app.session.research_tools.running() && ui.button("Cancel job").clicked(){run(app,ctx,"assist.cancel",json!({}));}
        let height=(ctx.content_rect().height()-160.0).clamp(160.0,650.0);
        egui::ScrollArea::vertical().max_height(height).show(ui,|ui|{
            if tab==0 {
            ui.heading("Learned stroke-head prediction");
            for tool in ["pencil","paintbrush"] {
                let options=app.session.tool_options_of(tool);let mut enabled=options["learnedPrediction"].as_bool().unwrap_or(false);
                if ui.checkbox(&mut enabled,format!("{tool}: experimental learned prediction")).changed(){run(app,ctx,"tool.setOption",json!({"tool":tool,"values":{"learnedPrediction":enabled,"predictionMs":if enabled && options["predictionMs"].as_f64().unwrap_or(0.0)==0.0{14.0}else{options["predictionMs"].as_f64().unwrap_or(0.0)}}}));}
                let live=app.session.tool_options_of(tool);
                ui.small(format!("{} · {} model inferences",live["learnedStatus"].as_str().unwrap_or("off"),live["learnedInferences"].as_u64().unwrap_or(0)));
            }
            ui.small("Display only; committed paths never contain predicted points. Measured native pen channels required. Turning this off preserves the existing predictor settings.");
            }
            if tab==1 {
            ui.separator();
            ui.heading("Junction suggestions");
            let mut gap=ctx.data(|d|d.get_temp::<f64>(egui::Id::new("research-gap"))).unwrap_or(8.0);
            let mut confidence=ctx.data(|d|d.get_temp::<f64>(egui::Id::new("research-confidence"))).unwrap_or(0.5);
            ui.add(egui::Slider::new(&mut gap,0.1..=32.0).text("Maximum gap (pt)"));
            ui.add(egui::Slider::new(&mut confidence,0.0..=1.0).text("Minimum confidence"));
            ctx.data_mut(|d|{d.insert_temp(egui::Id::new("research-gap"),gap);d.insert_temp(egui::Id::new("research-confidence"),confidence);});
            if ui.add_enabled(!app.session.research_tools.running(),egui::Button::new("Suggest junctions for selected strokes")).clicked(){run(app,ctx,"assist.junction.preview",json!({"gap":gap,"confidence":confidence}));}
            }
            if tab==2 {
            ui.separator();ui.heading("Neural natural brush appearance");
            let mut seed=ctx.data(|d|d.get_temp::<u32>(egui::Id::new("research-seed"))).unwrap_or(594);
            ui.horizontal(|ui|{ui.label("Style seed");ui.add(egui::DragValue::new(&mut seed));});
            ctx.data_mut(|d|{d.insert_temp(egui::Id::new("research-seed"),seed);});
            let mut color=ctx.data(|d|d.get_temp::<[f32;3]>(egui::Id::new("research-color"))).unwrap_or([0.1,0.25,0.65]);
            ui.horizontal(|ui|{ui.label("Appearance color");ui.color_edit_button_rgb(&mut color);});
            ctx.data_mut(|d|{d.insert_temp(egui::Id::new("research-color"),color);});
            ui.small("NeuBE CUDA raster appearance over ordinary open vector strokes. Sources are retained; this first version needs manual regeneration after source edits (not a live paint engine). 512 px preview limit.");
            if ui.add_enabled(!app.session.research_tools.running(),egui::Button::new("Generate neural brush preview")).clicked(){run(app,ctx,"assist.brush.preview",json!({"seed":seed,"color":color}));}
            if ui.button("Release selected neural appearance to edit source vectors").clicked(){run(app,ctx,"assist.brush.release",json!({}));}
            }
            if status["state"]=="ready" && ((tab==1&&status["kind"]=="junction")||(tab==2&&status["kind"]=="brush")) {
                if let Some(p)=app.session.research_tools.proposal.clone() {
                    if p.kind=="junction" {for (i,j) in p.junctions.iter().enumerate().take(64){ui.horizontal(|ui|{
                        ui.label(format!("{} → {} · {} · {:.0}%",j.a,j.b,j.kind,j.confidence*100.0));
                        if ui.button("Accept this join").clicked(){run(app,ctx,"assist.accept",json!({"job":status["job"],"indices":[i]}));}
                    });}} else {
                        ui.horizontal(|ui|{
                            if ui.button("Accept appearance + retain vectors").clicked(){run(app,ctx,"assist.accept",json!({"job":status["job"]}));}
                            if ui.button("Reject preview").clicked(){run(app,ctx,"assist.reject",json!({}));}
                        });
                        let job=app.session.research_tools.job;
                        let key=egui::Id::new("neube-preview");
                        let texture=ctx.data(|d|d.get_temp::<(u64,egui::TextureHandle)>(key)).filter(|(id,_)|*id==job).map(|(_,t)|t).or_else(||{
                            let img=image::load_from_memory(&p.png).ok()?.to_rgba8();
                            let texture=ctx.load_texture("NeuBE proposal",egui::ColorImage::from_rgba_unmultiplied([img.width() as usize,img.height() as usize],img.as_raw()),egui::TextureOptions::LINEAR);
                            ctx.data_mut(|d|{d.insert_temp(key,(job,texture.clone()));});Some(texture)
                        });
                        if let Some(t)=texture{let size=t.size_vec2();let scale=(420.0/size.x.max(420.0)).min(240.0/size.y.max(240.0));ui.image((t.id(),size*scale));}
                    }
                    ui.small(format!("Telemetry: {}",p.telemetry));
                }
                if tab==1 && ui.button("Reject preview").clicked(){run(app,ctx,"assist.reject",json!({}));}
            }
        });
    });
    app.research_open = open;
}
