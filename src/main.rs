use clap::Parser;
use eframe::egui;
use egui::Vec2;
use irasu_illustrator::cli::{Cli, run_cli};
use irasu_illustrator::core::boolean::{execute_pathfinder, BooleanOp};
use irasu_illustrator::core::document::{Object, ObjectType};
use irasu_illustrator::core::state::{AppState, Tool};
use irasu_illustrator::ui::canvas::CanvasWidget;
use irasu_illustrator::ui::panels::{
    AlignPanel, AppearancePanel, AudioWavePanel, BlendModePanel, ClippingMaskPanel,
    ColorHarmonyPanel, DeformPanel, EffectsPanel, EnvelopePanel, ExportPanel, FlowFieldPanel,
    FormulaPanel, GradientMeshPanel, GradientPanel, GridRepeatPanel, HalftonePanel, HistoryPanel,
    IsometricPanel, KnifePanel, LSystemPanel, LayerPanel, MeshWarpPanel, MorphPanel, NeonGlowPanel,
    OffsetPanel, PathfinderPanel, PatternPanel, PolarPanel, PresetPanel, PropertyPanel,
    QrCodePanel, RevolvePanel, ScatterBrushPanel, ShortcutsHelpPanel, SmartGuidesPanel,
    StrokePanel, SwatchesPanel, SymbolsPanel, SymmetryPanel, TextPanel, TracePanel,
    TransformPanel, VfxTrailPanel, VoronoiPanel, WidthToolPanel,
};
use irasu_illustrator::ui::timeline_widget::TimelineWidget;

#[derive(Debug, Clone, PartialEq)]
enum ActiveTab {
    Properties,
    Pathfinder,
    ThreeDAndVfx,
    Generative,
    Layers,
    Symbols,
    Export,
    Guides,
}

struct IrasuApp {
    state: AppState,
    canvas: CanvasWidget,
    active_tab: ActiveTab,
}

impl Default for IrasuApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            canvas: CanvasWidget::new(),
            active_tab: ActiveTab::Properties,
        }
    }
}

impl eframe::App for IrasuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply Authentic Adobe CC Charcoal Theme
        irasu_illustrator::ui::apply_adobe_theme(ctx);

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
                if i.key_pressed(egui::Key::I) {
                    self.state.previous_tool = self.state.current_tool;
                    self.state.current_tool = Tool::Eyedropper;
                }
                if i.key_pressed(egui::Key::H) { self.state.current_tool = Tool::Hand; }
                if i.key_pressed(egui::Key::B) { self.state.current_tool = Tool::Brush; }
                if i.key_pressed(egui::Key::E) { self.state.current_tool = Tool::Eraser; }
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

            // Ungroup (Ctrl+Shift+G)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::G)
            {
                let ids = self.state.selected_ids.clone();
                let mut new_ids = Vec::new();
                for id in &ids {
                    if let Some(obj) = self.state.document.remove_object(id) {
                        if let ObjectType::Group(children) = obj.object_type {
                            for child in children {
                                new_ids.push(child.id.clone());
                                let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(child));
                                self.state.undo_manager.execute(cmd, &mut self.state.document);
                            }
                        } else {
                            new_ids.push(obj.id.clone());
                            let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(obj));
                            self.state.undo_manager.execute(cmd, &mut self.state.document);
                        }
                    }
                }
                self.state.selected_ids = new_ids;
            }

            // Zoom to fit all (Ctrl+0)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num0) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                zoom_to_fit(&mut self.state);
                self.state.zoom_animation_progress = 0.0;
            }

            // Zoom to 100% (Ctrl+1)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num1) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = 1.0;
                self.state.target_pan_x = 0.0;
                self.state.target_pan_y = 0.0;
                self.state.zoom_animation_progress = 0.0;
            }

            // Clipping Mask (Ctrl+7)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num7) {
                let sel = self.state.selected_ids.clone();
                if sel.len() >= 2 {
                    let mask_id = sel[0].clone();
                    let content_ids: Vec<String> = sel[1..].to_vec();
                    let mut mask_obj = None;
                    let mut content_objs = Vec::new();
                    let mut ids_to_remove = Vec::new();
                    for (_, obj) in self.state.document.all_objects() {
                        if obj.id == mask_id {
                            mask_obj = Some(obj.clone());
                        } else if content_ids.contains(&obj.id) {
                            content_objs.push(obj.clone());
                        }
                    }
                    if let Some(mask) = mask_obj {
                        ids_to_remove.push(mask_id);
                        ids_to_remove.extend(content_ids);
                        let mask_path = mask.to_path_data();
                        let mut children = vec![Object::new_path("Mask", mask_path)];
                        children.extend(content_objs);
                        let clipping = Object {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: "Clipping Mask".into(),
                            object_type: ObjectType::ClippingMask { children },
                            ..Object::new_rect("Clipping Mask", 0.0, 0.0, 100.0, 100.0, 0.0)
                        };
                        for remove_id in &ids_to_remove {
                            self.state.document.remove_object(remove_id);
                        }
                        let cmd = Box::new(irasu_illustrator::core::history::AddObjectCommand::new(clipping));
                        self.state.undo_manager.execute(cmd, &mut self.state.document);
                    }
                }
            }

            // Arrow key nudging: Arrow=1px, Shift+Arrow=10px, Ctrl+Arrow=10px, Ctrl+Shift+Arrow=100px
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;
            let nudge = if ctrl && shift { 100.0 } else if ctrl || shift { 10.0 } else { 1.0 };
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
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = (self.state.target_zoom * 1.25).clamp(0.01, 100.0);
                self.state.zoom_animation_progress = 0.0;
            }

            // Zoom Out (Ctrl + Minus)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Minus) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = (self.state.target_zoom / 1.25).clamp(0.01, 100.0);
                self.state.zoom_animation_progress = 0.0;
            }

            // Object ordering: detect Ctrl+]/Ctrl+[ via text events
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;
            for event in &i.events {
                if let egui::Event::Text(text) = event {
                    if ctrl && text == "]" {
                        let sel = self.state.selected_ids.clone();
                        if shift {
                            // Bring to Front (Ctrl+Shift+])
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                        let obj = layer.objects.remove(pos);
                                        layer.objects.push(obj);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Bring Forward (Ctrl+])
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                        if pos + 1 < layer.objects.len() {
                                            layer.objects.swap(pos, pos + 1);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    } else if ctrl && text == "[" {
                        let sel = self.state.selected_ids.clone();
                        if shift {
                            // Send to Back (Ctrl+Shift+[)
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                        let obj = layer.objects.remove(pos);
                                        layer.objects.insert(0, obj);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Send Backward (Ctrl+[)
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                        if pos > 0 {
                                            layer.objects.swap(pos, pos - 1);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
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
                    ui.checkbox(&mut self.state.snap_to_objects, "Snap to Objects");
                    ui.checkbox(&mut self.state.show_rulers, "Show Rulers");
                    ui.checkbox(&mut self.state.show_smart_guides, "Smart Guides");
                    ui.checkbox(&mut self.state.show_timeline, "Show Timeline");
                    ui.add(egui::DragValue::new(&mut self.state.grid_size).speed(10.0).prefix("Grid Size: ").range(5.0..=500.0));
                    ui.separator();
                    if ui.button("Zoom to Fit  (Ctrl+0)").clicked() {
                        self.state.start_zoom = self.state.zoom;
                        self.state.start_pan_x = self.state.pan_x;
                        self.state.start_pan_y = self.state.pan_y;
                        zoom_to_fit(&mut self.state);
                        self.state.zoom_animation_progress = 0.0;
                        ui.close_menu();
                    }
                    if ui.button("Zoom 100%  (Ctrl+1)").clicked() {
                        self.state.start_zoom = self.state.zoom;
                        self.state.start_pan_x = self.state.pan_x;
                        self.state.start_pan_y = self.state.pan_y;
                        self.state.target_zoom = 1.0;
                        self.state.target_pan_x = 0.0;
                        self.state.target_pan_y = 0.0;
                        self.state.zoom_animation_progress = 0.0;
                        ui.close_menu();
                    }
                });
            });
        });

        // Top Horizontal Control / Options Bar (Illustrator Signature Bar)
        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let has_sel = !self.state.selected_ids.is_empty();
                    if has_sel {
                        // Read live values from first selected object
                        let first_id = self.state.selected_ids.first().cloned();
                        if let Some(ref fid) = first_id {
                            #[allow(clippy::type_complexity)]
                            let snap: Option<([f32;4],[f32;4],f64,f64,f64,f64,f64)> =
                                self.state.document.all_objects()
                                    .find(|(_, o)| &o.id == fid)
                                    .map(|(_, obj)| {
                                        let fill   = obj.fill.as_ref().map(|f| f.color).unwrap_or([0.0,0.0,0.0,0.0]);
                                        let stroke = obj.stroke.as_ref().map(|s| s.color).unwrap_or([0.0,0.0,0.0,1.0]);
                                        let sw     = obj.stroke.as_ref().map(|s| s.width).unwrap_or(1.0);
                                        let (bw, bh) = obj.bounding_box()
                                            .map(|(mn, mx)| (mx.x - mn.x, mx.y - mn.y))
                                            .unwrap_or((0.0, 0.0));
                                        (fill, stroke, sw, obj.transform.x, obj.transform.y, bw, bh)
                                    });

                            if let Some((obj_fill, obj_stroke, obj_sw, obj_tx, obj_ty, obj_bw, obj_bh)) = snap {
                                // X position
                                let mut tx = obj_tx;
                                ui.label(egui::RichText::new("X").weak().size(11.0));
                                if ui.add(egui::DragValue::new(&mut tx).speed(0.5).max_decimals(1).prefix("")).changed() {
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id { o.transform.x = tx; }
                                        }
                                    }
                                }
                                // Y position
                                let mut ty = obj_ty;
                                ui.label(egui::RichText::new("Y").weak().size(11.0));
                                if ui.add(egui::DragValue::new(&mut ty).speed(0.5).max_decimals(1)).changed() {
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id { o.transform.y = ty; }
                                        }
                                    }
                                }
                                ui.separator();
                                // Width
                                let mut bw = obj_bw;
                                ui.label(egui::RichText::new("W").weak().size(11.0));
                                if ui.add(egui::DragValue::new(&mut bw).speed(0.5).max_decimals(1).range(0.1..=99999.0)).changed() && obj_bw > 0.0 {
                                    let scale = bw / obj_bw;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id { o.transform.scale_x *= scale; }
                                        }
                                    }
                                }
                                // Height
                                let mut bh = obj_bh;
                                ui.label(egui::RichText::new("H").weak().size(11.0));
                                if ui.add(egui::DragValue::new(&mut bh).speed(0.5).max_decimals(1).range(0.1..=99999.0)).changed() && obj_bh > 0.0 {
                                    let scale = bh / obj_bh;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id { o.transform.scale_y *= scale; }
                                        }
                                    }
                                }
                                ui.separator();

                                // Fill Color — read from object, write back on change
                                ui.label(egui::RichText::new("Fill:").size(11.0));
                                let mut fill_c = [
                                    (obj_fill[0] * 255.0) as u8,
                                    (obj_fill[1] * 255.0) as u8,
                                    (obj_fill[2] * 255.0) as u8,
                                    (obj_fill[3] * 255.0) as u8,
                                ];
                                if ui.color_edit_button_srgba_unmultiplied(&mut fill_c).changed() {
                                    let new_fill = [fill_c[0] as f32/255.0, fill_c[1] as f32/255.0, fill_c[2] as f32/255.0, fill_c[3] as f32/255.0];
                                    self.state.fill_color = new_fill;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                obj.fill = Some(irasu_illustrator::core::path::FillStyle::solid(new_fill));
                                            }
                                        }
                                    }
                                }

                                // Stroke Color
                                ui.label(egui::RichText::new("Stroke:").size(11.0));
                                let mut stroke_c = [
                                    (obj_stroke[0] * 255.0) as u8,
                                    (obj_stroke[1] * 255.0) as u8,
                                    (obj_stroke[2] * 255.0) as u8,
                                    (obj_stroke[3] * 255.0) as u8,
                                ];
                                if ui.color_edit_button_srgba_unmultiplied(&mut stroke_c).changed() {
                                    let new_sc = [stroke_c[0] as f32/255.0, stroke_c[1] as f32/255.0, stroke_c[2] as f32/255.0, stroke_c[3] as f32/255.0];
                                    self.state.stroke_color = new_sc;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                match obj.stroke.as_mut() {
                                                    Some(s) => s.color = new_sc,
                                                    None => obj.stroke = Some(irasu_illustrator::core::path::StrokeStyle { color: new_sc, width: obj_sw, dash_pattern: None, ..Default::default() }),
                                                }
                                            }
                                        }
                                    }
                                }

                                // Stroke Width
                                let mut sw = obj_sw;
                                if ui.add(egui::DragValue::new(&mut sw).speed(0.1).range(0.0..=200.0).suffix(" pt")).changed() {
                                    self.state.stroke_width = sw;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                if let Some(ref mut s) = obj.stroke { s.width = sw; }
                                            }
                                        }
                                    }
                                }

                                // Opacity
                                ui.separator();
                                let mut op = self.state.opacity;
                                ui.label(egui::RichText::new("Opacity:").size(11.0));
                                if ui.add(egui::Slider::new(&mut op, 0.0..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                                    self.state.opacity = op;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id { obj.opacity = op; }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("No Selection").weak());
                        ui.separator();
                        ui.label(format!("Doc: {} × {} px", self.state.document.width as i32, self.state.document.height as i32));
                        ui.separator();
                        ui.label(format!("Zoom: {:.0}%", self.state.zoom * 100.0));
                        ui.separator();
                        ui.checkbox(&mut self.state.show_grid, "Grid");
                        ui.checkbox(&mut self.state.snap_to_grid, "Snap");
                        ui.checkbox(&mut self.state.show_rulers, "Rulers");
                    }
                });
            });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Zoom Presets
                    egui::ComboBox::from_id_salt("zoom_select")
                        .selected_text(format!("{:.0}%", self.state.zoom * 100.0))
                        .show_ui(ui, |ui| {
                            let set_zoom = |state: &mut AppState, z: f32| {
                                state.start_zoom = state.zoom;
                                state.start_pan_x = state.pan_x;
                                state.start_pan_y = state.pan_y;
                                state.target_zoom = z;
                                state.target_pan_x = state.pan_x;
                                state.target_pan_y = state.pan_y;
                                state.zoom_animation_progress = 0.0;
                            };
                            if ui.selectable_label(self.state.zoom == 0.25, "25%").clicked() { set_zoom(&mut self.state, 0.25); }
                            if ui.selectable_label(self.state.zoom == 0.5, "50%").clicked() { set_zoom(&mut self.state, 0.5); }
                            if ui.selectable_label(self.state.zoom == 1.0, "100%").clicked() { set_zoom(&mut self.state, 1.0); }
                            if ui.selectable_label(self.state.zoom == 2.0, "200%").clicked() { set_zoom(&mut self.state, 2.0); }
                            if ui.selectable_label(self.state.zoom == 4.0, "400%").clicked() { set_zoom(&mut self.state, 4.0); }
                            if ui.button("Fit on Screen (Cmd+0)").clicked() {
                                self.state.start_zoom = self.state.zoom;
                                self.state.start_pan_x = self.state.pan_x;
                                self.state.start_pan_y = self.state.pan_y;
                                zoom_to_fit(&mut self.state);
                                self.state.zoom_animation_progress = 0.0;
                            }
                        });

                    ui.separator();
                    ui.label(egui::RichText::new(format!("Tool: {} ({})", self.state.current_tool.name(), self.state.current_tool.shortcut())).strong());

                    ui.separator();
                    let hint = match self.state.current_tool {
                        Tool::Select => "Click to select. Drag to move. Alt+Drag to duplicate.",
                        Tool::Node => "Click or drag anchor points to edit shape contour.",
                        Tool::Pen => "Click to add corner points. Drag to pull bezier curve handles.",
                        Tool::Rectangle => "Drag to create. Shift for square. Alt from center.",
                        Tool::Ellipse => "Drag to create. Shift for circle. Alt from center.",
                        Tool::Line => "Drag to draw line. Shift snaps to 45° increments.",
                        Tool::Hand => "Drag to pan the artboard canvas viewport.",
                        Tool::Brush => "Drag to paint smooth brush strokes with bezier curves.",
                        Tool::Eraser => "Drag to erase objects that intersect the eraser path.",
                        _ => "Click or drag on canvas to use tool.",
                    };
                    ui.label(egui::RichText::new(hint).weak().size(11.0));
                });
            });

        // Left Vertical Toolbar (Classic 2-Column Illustrator Grid)
        egui::SidePanel::left("toolbar")
            .resizable(false)
            .default_width(74.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);

                    let tool_pairs = [
                        (Tool::Select, Tool::Node),
                        (Tool::Pen, Tool::Pencil),
                        (Tool::Rectangle, Tool::Ellipse),
                        (Tool::Star, Tool::Polygon),
                        (Tool::Line, Tool::Text),
                        (Tool::Eyedropper, Tool::Hand),
                        (Tool::Brush, Tool::Eraser),
                    ];

                    for (t1, t2) in tool_pairs {
                        ui.horizontal(|ui| {
                            for tool in [t1, t2] {
                                let is_active = self.state.current_tool == tool;
                                let btn = if is_active {
                                    egui::Button::new(
                                        egui::RichText::new(tool.icon()).size(17.0).strong().color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(32.0, 30.0))
                                } else {
                                    egui::Button::new(egui::RichText::new(tool.icon()).size(17.0))
                                        .min_size(Vec2::new(32.0, 30.0))
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
                                    self.state.previous_tool = self.state.current_tool;
                                    self.state.current_tool = tool;
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Overlapping Fill & Stroke Swatches (Illustrator Signature Widget)
                    ui.label(egui::RichText::new("Color").size(10.0).weak());

                    let mut fill_c = [
                        (self.state.fill_color[0] * 255.0) as u8,
                        (self.state.fill_color[1] * 255.0) as u8,
                        (self.state.fill_color[2] * 255.0) as u8,
                        (self.state.fill_color[3] * 255.0) as u8,
                    ];
                    let mut stroke_c = [
                        (self.state.stroke_color[0] * 255.0) as u8,
                        (self.state.stroke_color[1] * 255.0) as u8,
                        (self.state.stroke_color[2] * 255.0) as u8,
                        (self.state.stroke_color[3] * 255.0) as u8,
                    ];

                    ui.horizontal(|ui| {
                        ui.label("Fill");
                        if ui.color_edit_button_srgba_unmultiplied(&mut fill_c).changed() {
                            self.state.fill_color = [fill_c[0] as f32 / 255.0, fill_c[1] as f32 / 255.0, fill_c[2] as f32 / 255.0, fill_c[3] as f32 / 255.0];
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Line");
                        if ui.color_edit_button_srgba_unmultiplied(&mut stroke_c).changed() {
                            self.state.stroke_color = [stroke_c[0] as f32 / 255.0, stroke_c[1] as f32 / 255.0, stroke_c[2] as f32 / 255.0, stroke_c[3] as f32 / 255.0];
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.small_button("⇄").on_hover_text("Swap Fill and Stroke (Shift+X)").clicked() {
                            std::mem::swap(&mut self.state.fill_color, &mut self.state.stroke_color);
                        }
                        if ui.small_button("◻").on_hover_text("Default Colors (D)").clicked() {
                            self.state.fill_color = [1.0, 1.0, 1.0, 1.0];
                            self.state.stroke_color = [0.0, 0.0, 0.0, 1.0];
                            self.state.stroke_width = 1.0;
                        }
                        if ui.small_button("⊘").on_hover_text("None / Transparent (/)").clicked() {
                            self.state.fill_color = [0.0, 0.0, 0.0, 0.0];
                        }
                    });
                });
            });

        // Right Tabbed Sidebar Panels
        egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                // Adobe CC Tab Headers
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(self.active_tab == ActiveTab::Properties, "🎨 Properties").clicked() {
                        self.active_tab = ActiveTab::Properties;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Pathfinder, "✂ Pathfinder").clicked() {
                        self.active_tab = ActiveTab::Pathfinder;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::ThreeDAndVfx, "🏺 3D & VFX").clicked() {
                        self.active_tab = ActiveTab::ThreeDAndVfx;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Generative, "🌈 Generative").clicked() {
                        self.active_tab = ActiveTab::Generative;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Layers, "📄 Layers").clicked() {
                        self.active_tab = ActiveTab::Layers;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Symbols, "⭐ Symbols").clicked() {
                        self.active_tab = ActiveTab::Symbols;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Export, "📤 Export").clicked() {
                        self.active_tab = ActiveTab::Export;
                    }
                    if ui.selectable_label(self.active_tab == ActiveTab::Guides, "📏 Guides").clicked() {
                        self.active_tab = ActiveTab::Guides;
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        ActiveTab::Properties => {
                            PropertyPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            AlignPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            StrokePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            GradientPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            BlendModePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            TransformPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            AppearancePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            TextPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            WidthToolPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            PatternPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            PresetPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            ColorHarmonyPanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Pathfinder => {
                            PathfinderPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            ClippingMaskPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            KnifePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            EnvelopePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            OffsetPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            MorphPanel::show(ui, &mut self.state);
                        }
                        ActiveTab::ThreeDAndVfx => {
                            RevolvePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            NeonGlowPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            VfxTrailPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            EffectsPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            TracePanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Generative => {
                            SwatchesPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            GradientMeshPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            PolarPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            AudioWavePanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            FlowFieldPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            DeformPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            ScatterBrushPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            MeshWarpPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            VoronoiPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            LSystemPanel::show(ui, &mut self.state);
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
                            FormulaPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            QrCodePanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Layers => {
                            LayerPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            HistoryPanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Symbols => {
                            SymbolsPanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Export => {
                            ExportPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            GridRepeatPanel::show(ui, &mut self.state);
                        }
                        ActiveTab::Guides => {
                            SmartGuidesPanel::show(ui, &mut self.state);
                            ui.add_space(8.0);
                            ui.separator();
                            ShortcutsHelpPanel::show(ui, &mut self.state);
                        }
                    }
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
            let device_queue = _frame.wgpu_render_state().map(|s| (std::sync::Arc::new(s.device.clone()), std::sync::Arc::new(s.queue.clone())));
            let (device, queue) = match device_queue {
                Some((d, q)) => (Some(d), Some(q)),
                None => (None, None),
            };
            self.canvas.show(ui, &mut self.state, device, queue);
        });
    }
}

fn zoom_to_fit(state: &mut AppState) {
    state.zoom_to_fit();
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
