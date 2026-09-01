use clap::Parser;
use eframe::egui;
use egui::Vec2;
use irasu_illustrator::cli::{Cli, run_cli};
use irasu_illustrator::core::boolean::{execute_pathfinder, BooleanOp};
use irasu_illustrator::core::document::Object;
use irasu_illustrator::core::state::{AppState, Tool};
use irasu_illustrator::ui::canvas::CanvasWidget;
use irasu_illustrator::ui::panels::{
    AlignPanel, AudioWavePanel, AxonometricPanel, DeformPanel, EffectsPanel, EnvelopePanel,
    FlowFieldPanel, FormulaPanel, GradientMeshPanel, HalftonePanel, IsometricPanel, KnifePanel,
    LSystemPanel, LayerPanel, MeshWarpPanel, MorphPanel, NeonGlowPanel, OffsetPanel,
    PathfinderPanel, PolarPanel, PresetPanel, PropertyPanel, QrCodePanel, RevolvePanel,
    ScatterBrushPanel, SymmetryPanel, TracePanel, VfxTrailPanel, VoronoiPanel,
};
use irasu_illustrator::ui::timeline_widget::TimelineWidget;

struct IrasuApp {
    state: AppState,
    canvas: CanvasWidget,
}

impl Default for IrasuApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            canvas: CanvasWidget::new(),
        }
    }
}

impl eframe::App for IrasuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Timeline animation playback tick
        if self.state.timeline.is_playing {
            self.state.timeline.advance_frame();
            self.state.timeline.apply_to_document(&mut self.state.document);
            ctx.request_repaint();
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if !i.modifiers.ctrl && !i.modifiers.mac_cmd && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) { self.state.current_tool = Tool::Select; }
                if i.key_pressed(egui::Key::A) { self.state.current_tool = Tool::Node; }
                if i.key_pressed(egui::Key::P) { self.state.current_tool = Tool::Pen; }
                if i.key_pressed(egui::Key::N) { self.state.current_tool = Tool::Pencil; }
                if i.key_pressed(egui::Key::U) { self.state.current_tool = Tool::Rectangle; }
                if i.key_pressed(egui::Key::O) { self.state.current_tool = Tool::Ellipse; }
                if i.key_pressed(egui::Key::S) { self.state.current_tool = Tool::Star; }
                if i.key_pressed(egui::Key::G) { self.state.current_tool = Tool::Polygon; }
                if i.key_pressed(egui::Key::L) { self.state.current_tool = Tool::Line; }
                if i.key_pressed(egui::Key::T) { self.state.current_tool = Tool::Text; }
                if i.key_pressed(egui::Key::I) { self.state.current_tool = Tool::Eyedropper; }
                if i.key_pressed(egui::Key::H) { self.state.current_tool = Tool::Hand; }
            }

            // Escape to cancel pen
            if i.key_pressed(egui::Key::Escape) {
                self.canvas.pen_state.cancel();
                self.state.selected_ids.clear();
            }

            // Enter to finish pen path
            if i.key_pressed(egui::Key::Enter) {
                if let Some(obj) = self.canvas.pen_state.finish_path(
                    self.state.fill_color,
                    self.state.stroke_color,
                    self.state.stroke_width,
                ) {
                    let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(obj));
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                }
            }

            // Delete selected
            if i.key_pressed(egui::Key::Delete) || (i.key_pressed(egui::Key::Backspace) && !i.modifiers.ctrl && !i.modifiers.mac_cmd) {
                let ids: Vec<String> = self.state.selected_ids.clone();
                for id in &ids {
                    let obj = self.state.document.remove_object(id);
                    if let Some(obj) = obj {
                        let layer_idx = self
                            .state
                            .document
                            .layers
                            .iter()
                            .position(|l| l.objects.iter().any(|o| &o.id == id))
                            .unwrap_or(0);
                        let cmd = Box::new(irasu_illustrator::core::history::RemoveObjectCommand::new(
                            obj, layer_idx, 0,
                        ));
                        self.state.undo_manager.execute(cmd, &mut self.state.document);
                    }
                }
                self.state.selected_ids.clear();
            }

            // Undo
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                self.state.undo_manager.undo(&mut self.state.document);
            }

            // Redo
            if ((i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Y))
                || ((i.modifiers.ctrl || i.modifiers.mac_cmd) && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                self.state.undo_manager.redo(&mut self.state.document);
            }

            // Select All
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::A) {
                self.state.selected_ids = self.state.document.all_objects()
                    .filter(|(_, o)| o.visible && !o.locked)
                    .map(|(_, o)| o.id.clone())
                    .collect();
            }

            // Group (Ctrl+G)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::G)
                && self.state.selected_ids.len() >= 2
            {
                let mut objs = Vec::new();
                for id in &self.state.selected_ids {
                    if let Some(o) = self.state.document.remove_object(id) {
                        objs.push(o);
                    }
                }
                let group = Object::new_group("Group", objs);
                let gid = group.id.clone();
                let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(group));
                self.state.undo_manager.execute(cmd, &mut self.state.document);
                self.state.selected_ids = vec![gid];
            }

            // Zoom to fit all (Ctrl+0)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num0) {
                zoom_to_fit(&mut self.state);
            }

            // Zoom to 100% (Ctrl+1)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num1) {
                self.state.zoom = 1.0;
                self.state.pan_x = 0.0;
                self.state.pan_y = 0.0;
            }

            // Arrow key nudging
            let nudge = if i.modifiers.shift { 10.0 } else { 1.0 };
            let mut nudge_x = 0.0;
            let mut nudge_y = 0.0;
            if i.key_pressed(egui::Key::ArrowLeft) { nudge_x -= nudge; }
            if i.key_pressed(egui::Key::ArrowRight) { nudge_x += nudge; }
            if i.key_pressed(egui::Key::ArrowUp) { nudge_y -= nudge; }
            if i.key_pressed(egui::Key::ArrowDown) { nudge_y += nudge; }

            if nudge_x != 0.0 || nudge_y != 0.0 {
                for id in &self.state.selected_ids {
                    let id = id.clone();
                    for (_, obj) in self.state.document.all_objects_mut() {
                        if obj.id == id {
                            obj.transform.x += nudge_x;
                            obj.transform.y += nudge_y;
                            break;
                        }
                    }
                }
            }

            // Swap Fill and Stroke (Shift+X)
            if i.modifiers.shift && i.key_pressed(egui::Key::X) {
                std::mem::swap(&mut self.state.fill_color, &mut self.state.stroke_color);
            }

            // Default Fill and Stroke (D)
            if !i.modifiers.ctrl && !i.modifiers.mac_cmd && !i.modifiers.alt && i.key_pressed(egui::Key::D) {
                self.state.fill_color = [1.0, 1.0, 1.0, 1.0];
                self.state.stroke_color = [0.0, 0.0, 0.0, 1.0];
                self.state.stroke_width = 1.0;
            }

            // Set Fill to None (Slash /)
            if !i.modifiers.ctrl && !i.modifiers.mac_cmd && !i.modifiers.alt && i.key_pressed(egui::Key::Slash) {
                self.state.fill_color = [0.0, 0.0, 0.0, 0.0];
            }

            // Zoom In (Ctrl + Plus / Equal)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus)) {
                self.state.zoom = (self.state.zoom * 1.25).clamp(0.01, 100.0);
            }

            // Zoom Out (Ctrl + Minus)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Minus) {
                self.state.zoom = (self.state.zoom / 1.25).clamp(0.01, 100.0);
            }

            // Duplicate / Transform Again (Ctrl+D)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::D) && !self.state.selected_ids.is_empty() {
                let mut duplicated = Vec::new();
                for id in &self.state.selected_ids {
                    if let Some((_, obj)) = self.state.document.all_objects().find(|(_, o)| &o.id == id) {
                        let mut dup = obj.clone();
                        dup.id = uuid::Uuid::new_v4().to_string();
                        dup.name = format!("{} Copy", obj.name);
                        dup.transform.x += 15.0;
                        dup.transform.y += 15.0;
                        duplicated.push(dup);
                    }
                }
                for dup in duplicated {
                    let new_id = dup.id.clone();
                    let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(dup));
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                    self.state.selected_ids = vec![new_id];
                }
            }

            // Copy (Ctrl+C)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::C) {
                self.state.clipboard.clear();
                for id in &self.state.selected_ids {
                    if let Some((_, obj)) = self.state.document.all_objects().find(|(_, o)| &o.id == id) {
                        self.state.clipboard.push(obj.clone());
                    }
                }
            }

            // Paste (Ctrl+V)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::V) {
                self.state.selected_ids.clear();
                let mut offset = 0.0;
                for obj in &self.state.clipboard {
                    let mut new_obj = obj.clone();
                    new_obj.id = uuid::Uuid::new_v4().to_string();
                    new_obj.name = format!("{} (copy)", obj.name);
                    new_obj.transform.x += 20.0 + offset;
                    new_obj.transform.y += 20.0 + offset;
                    offset += 15.0;
                    let id = new_obj.id.clone();
                    let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(new_obj));
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                    self.state.selected_ids.push(id);
                }
            }

            // Duplicate (Ctrl+D)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::D) {
                let ids: Vec<String> = self.state.selected_ids.clone();
                let mut new_objs = Vec::new();
                for id in &ids {
                    if let Some((_, obj)) = self.state.document.all_objects().find(|(_, o)| &o.id == id) {
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} (copy)", obj.name);
                        new_obj.transform.x += 20.0;
                        new_obj.transform.y += 20.0;
                        new_objs.push(new_obj);
                    }
                }
                let mut new_ids = Vec::new();
                for obj in new_objs {
                    let new_id = obj.id.clone();
                    let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(obj));
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                    new_ids.push(new_id);
                }
                self.state.selected_ids = new_ids;
            }
        });

        // Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Document").clicked() {
                        self.state.document = Default::default();
                        self.state.undo_manager.clear();
                        self.state.selected_ids.clear();
                        ui.close_menu();
                    }
                    if ui.button("Open SVG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                self.state.document = irasu_illustrator::io::svg::parse_svg_document(&content);
                                self.state.document.name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Untitled")
                                    .to_string();
                                self.state.undo_manager.clear();
                                self.state.selected_ids.clear();
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save Project (.json)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
                            let _ = irasu_illustrator::io::project::save_project(
                                &self.state.document,
                                &path,
                            );
                        }
                        ui.close_menu();
                    }
                    if ui.button("Load Project (.json)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            if let Ok(doc) = irasu_illustrator::io::project::load_project(&path) {
                                self.state.document = doc;
                                self.state.undo_manager.clear();
                                self.state.selected_ids.clear();
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export SVG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .save_file()
                        {
                            let svg = irasu_illustrator::io::svg::export_svg(&self.state.document);
                            let _ = std::fs::write(&path, svg);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("🎬 VFX Pipeline", |ui| {
                        if ui.button("Export for AEVFX Studio Comp (.json)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("AEVFX Comp", &["json", "aevfx"])
                                .save_file()
                            {
                                let comp = irasu_illustrator::io::vfx::doc_to_aevfx_comp(&self.state.document, 60.0, 5.0);
                                if let Ok(json) = serde_json::to_string_pretty(&comp) {
                                    let _ = std::fs::write(&path, json);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Export Motion Path Keyframes (.json)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Motion Path", &["json"])
                                .save_file()
                            {
                                let mut paths = Vec::new();
                                for (_, obj) in self.state.document.all_objects() {
                                    let kfs = irasu_illustrator::io::vfx::object_to_motion_path_keyframes(obj, 60, 5.0, 60.0);
                                    if !kfs.is_empty() {
                                        paths.push(serde_json::json!({
                                            "name": obj.name,
                                            "keyframes": kfs
                                        }));
                                    }
                                }
                                if let Ok(json) = serde_json::to_string_pretty(&paths) {
                                    let _ = std::fs::write(&path, json);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Export 3D Mesh (.obj)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Wavefront OBJ", &["obj"])
                                .save_file()
                            {
                                let obj_str = irasu_illustrator::io::vfx::export_doc_to_obj(&self.state.document, 20.0, 2.0);
                                let _ = std::fs::write(&path, obj_str);
                            }
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    let can_undo = self.state.undo_manager.can_undo();
                    let can_redo = self.state.undo_manager.can_redo();
                    let undo_name = self.state.undo_manager.undo_name().unwrap_or("—").to_string();
                    let redo_name = self.state.undo_manager.redo_name().unwrap_or("—").to_string();
                    if ui.add_enabled(can_undo, egui::Button::new(format!("Undo ({undo_name})  (Ctrl+Z)"))).clicked() {
                        self.state.undo_manager.undo(&mut self.state.document);
                        ui.close_menu();
                    }
                    if ui.add_enabled(can_redo, egui::Button::new(format!("Redo ({redo_name})  (Ctrl+Y)"))).clicked() {
                        self.state.undo_manager.redo(&mut self.state.document);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Select All  (Ctrl+A)").clicked() {
                        self.state.selected_ids = self.state.document.all_objects()
                            .filter(|(_, o)| o.visible && !o.locked)
                            .map(|(_, o)| o.id.clone())
                            .collect();
                        ui.close_menu();
                    }
                    if ui.button("Deselect All").clicked() {
                        self.state.selected_ids.clear();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Duplicate  (Ctrl+D)").clicked() {
                        let ids: Vec<String> = self.state.selected_ids.clone();
                        let mut new_objs = Vec::new();
                        for id in &ids {
                            if let Some((_, obj)) = self.state.document.all_objects().find(|(_, o)| &o.id == id) {
                                let mut new_obj = obj.clone();
                                new_obj.id = uuid::Uuid::new_v4().to_string();
                                new_obj.name = format!("{} (copy)", obj.name);
                                new_obj.transform.x += 20.0;
                                new_obj.transform.y += 20.0;
                                new_objs.push(new_obj);
                            }
                        }
                        let mut new_ids = Vec::new();
                        for obj in new_objs {
                            let new_id = obj.id.clone();
                            let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(obj));
                            self.state.undo_manager.execute(cmd, &mut self.state.document);
                            new_ids.push(new_id);
                        }
                        self.state.selected_ids = new_ids;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Object", |ui| {
                    let has_sel = !self.state.selected_ids.is_empty();
                    let multi_sel = self.state.selected_ids.len() >= 2;

                    if ui.add_enabled(multi_sel, egui::Button::new("Group  (Ctrl+G)")).clicked() {
                        let mut objs = Vec::new();
                        for id in &self.state.selected_ids {
                            if let Some(o) = self.state.document.remove_object(id) {
                                objs.push(o);
                            }
                        }
                        let group = Object::new_group("Group", objs);
                        let gid = group.id.clone();
                        let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(group));
                        self.state.undo_manager.execute(cmd, &mut self.state.document);
                        self.state.selected_ids = vec![gid];
                        ui.close_menu();
                    }

                    if ui.add_enabled(has_sel, egui::Button::new("Bring to Front")).clicked() {
                        let sel = self.state.selected_ids.clone();
                        for id in &sel {
                            for layer in self.state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    let obj = layer.objects.remove(pos);
                                    layer.objects.push(obj);
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.add_enabled(has_sel, egui::Button::new("Send to Back")).clicked() {
                        let sel = self.state.selected_ids.clone();
                        for id in &sel {
                            for layer in self.state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    let obj = layer.objects.remove(pos);
                                    layer.objects.insert(0, obj);
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Pathfinder", |ui| {
                    let multi = self.state.selected_ids.len() >= 2;
                    let ops = [
                        (BooleanOp::Union, "Unite"),
                        (BooleanOp::Subtract, "Minus Front"),
                        (BooleanOp::Intersect, "Intersect"),
                        (BooleanOp::Exclude, "Exclude"),
                    ];
                    for (op, name) in ops {
                        if ui.add_enabled(multi, egui::Button::new(format!("{} {}", op.icon(), name))).clicked() {
                            let mut selected_objs: Vec<Object> = Vec::new();
                            for id in &self.state.selected_ids {
                                if let Some((_, obj)) = self.state.document.all_objects().find(|(_, o)| &o.id == id) {
                                    selected_objs.push(obj.clone());
                                }
                            }
                            let obj_refs: Vec<&Object> = selected_objs.iter().collect();
                            if let Some(result_obj) = execute_pathfinder(&obj_refs, op) {
                                for id in &self.state.selected_ids {
                                    self.state.document.remove_object(id);
                                }
                                let new_id = result_obj.id.clone();
                                let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(result_obj));
                                self.state.undo_manager.execute(cmd, &mut self.state.document);
                                self.state.selected_ids = vec![new_id];
                            }
                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.state.show_grid, "Show Grid");
                    ui.checkbox(&mut self.state.snap_to_grid, "Snap to Grid");
                    ui.checkbox(&mut self.state.show_rulers, "Show Rulers");
                    ui.checkbox(&mut self.state.show_smart_guides, "Smart Guides");
                    ui.checkbox(&mut self.state.show_timeline, "Show Timeline");
                    ui.add(egui::DragValue::new(&mut self.state.grid_size).speed(10.0).prefix("Grid Size: ").range(5.0..=500.0));
                    ui.separator();
                    if ui.button("Zoom to Fit  (Ctrl+0)").clicked() {
                        zoom_to_fit(&mut self.state);
                        ui.close_menu();
                    }
                    if ui.button("Zoom 100%  (Ctrl+1)").clicked() {
                        self.state.zoom = 1.0;
                        ui.close_menu();
                    }
                });
            });
        });

        // Left Vertical Toolbar
        egui::SidePanel::left("toolbar")
            .resizable(false)
            .default_width(52.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(6.0);

                    let tools = [
                        Tool::Select,
                        Tool::Node,
                        Tool::Pen,
                        Tool::Pencil,
                        Tool::Rectangle,
                        Tool::Ellipse,
                        Tool::Star,
                        Tool::Polygon,
                        Tool::Line,
                        Tool::Text,
                        Tool::Eyedropper,
                        Tool::Hand,
                    ];

                    for tool in tools {
                        let is_active = self.state.current_tool == tool;
                        let btn = if is_active {
                            egui::Button::new(
                                egui::RichText::new(tool.icon()).size(19.0).strong().color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(0, 120, 255))
                            .min_size(Vec2::new(38.0, 34.0))
                        } else {
                            egui::Button::new(egui::RichText::new(tool.icon()).size(19.0))
                                .min_size(Vec2::new(38.0, 34.0))
                        };

                        if ui
                            .add(btn)
                            .on_hover_text(format!("{} ({})", tool.name(), tool.shortcut()))
                            .clicked()
                        {
                            if self.state.current_tool == Tool::Pen && self.canvas.pen_state.is_drawing {
                                if let Some(obj) = self.canvas.pen_state.finish_path(
                                    self.state.fill_color,
                                    self.state.stroke_color,
                                    self.state.stroke_width,
                                ) {
                                    let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(obj));
                                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                                }
                            }
                            self.state.current_tool = tool;
                        }
                    }
                });
            });

        // Right Sidebar Panels
        egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    PropertyPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    PresetPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    FormulaPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    HalftonePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    IsometricPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    SymmetryPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    VoronoiPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    LSystemPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    QrCodePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    DeformPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    FlowFieldPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    ScatterBrushPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    AudioWavePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    MeshWarpPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    GradientMeshPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    AxonometricPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    NeonGlowPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    RevolvePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    EnvelopePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    PolarPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    KnifePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    VfxTrailPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    TracePanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    EffectsPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    OffsetPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    MorphPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    PathfinderPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    AlignPanel::show(ui, &mut self.state);
                    ui.add_space(8.0);
                    ui.separator();
                    LayerPanel::show(ui, &mut self.state);
                });
            });

        // Bottom Timeline Panel
        if self.state.show_timeline {
            egui::TopBottomPanel::bottom("timeline_panel")
                .resizable(true)
                .default_height(70.0)
                .show(ctx, |ui| {
                    TimelineWidget::show(ui, &mut self.state);
                });
        }

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "🔍 Zoom: {:.0}%  |  📏 Grid: {:.0}px  |  📦 Objects: {} (Selected: {})  |  ↩ Undo: {}  |  ↪ Redo: {}",
                    self.state.zoom * 100.0,
                    self.state.grid_size,
                    self.state.document.all_objects().count(),
                    self.state.selected_ids.len(),
                    self.state.undo_manager.undo_depth(),
                    self.state.undo_manager.redo_depth(),
                ));

                if self.canvas.pen_state.is_drawing {
                    ui.separator();
                    ui.label(egui::RichText::new(format!("✒ Pen: {} nodes (Press Enter to finish)", self.canvas.pen_state.points.len())).color(egui::Color32::from_rgb(0, 220, 120)));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Artboard: {} × {} px",
                        self.state.document.width, self.state.document.height
                    ));
                    ui.separator();
                    ui.label(format!("Tool: {} [{}]", self.state.current_tool.name(), self.state.current_tool.shortcut()));
                });
            });
        });

        // Central Drawing Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            self.canvas.show(ui, &mut self.state);
        });
    }
}

fn zoom_to_fit(state: &mut AppState) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for (_, obj) in state.document.all_objects() {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            min_x = min_x.min(bb_min.x);
            min_y = min_y.min(bb_min.y);
            max_x = max_x.max(bb_max.x);
            max_y = max_y.max(bb_max.y);
        }
    }
    if min_x < max_x && min_y < max_y {
        let obj_w = max_x - min_x;
        let obj_h = max_y - min_y;
        let cw = state.canvas_width;
        let ch = state.canvas_height;
        state.zoom = ((cw / obj_w as f32).min(ch / obj_h as f32) * 0.85).clamp(0.01, 100.0);
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        state.pan_x = cw / 2.0 - center_x as f32 * state.zoom;
        state.pan_y = ch / 2.0 - center_y as f32 * state.zoom;
    } else {
        state.zoom = 1.0;
        state.pan_x = 0.0;
        state.pan_y = 0.0;
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match run_cli(cli) {
        Ok(false) => {
            // CLI command ran and finished
            return Ok(());
        }
        Err(err) => {
            eprintln!("❌ Error: {err}");
            std::process::exit(1);
        }
        Ok(true) => {
            // Launch GUI
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("IRASU Illustrator — Pro Vector Studio & VFX Pipeline"),
        ..Default::default()
    };

    eframe::run_native(
        "IRASU Illustrator",
        options,
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = Vec2::new(4.0, 4.0);
            cc.egui_ctx.set_style(style);
            Ok(Box::new(IrasuApp::default()))
        }),
    )
}
