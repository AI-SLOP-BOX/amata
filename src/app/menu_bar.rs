use super::{zoom_to_fit, ActiveTab, IrasuApp};
use crate::app::icons::{icon_bell, icon_button, icon_search};
use crate::core::boolean::{execute_pathfinder, BooleanOp};
use crate::core::document::Object;
use egui::{self, Color32, Pos2, RichText, Vec2};
impl IrasuApp {
    pub(super) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        // Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // macOS Window Control Dots (Red, Yellow, Green)
                let (dots_rect, _) =
                    ui.allocate_exact_size(Vec2::new(52.0, 18.0), egui::Sense::hover());
                // Amata Logo & Home Button
                let (logo_rect, logo_resp) =
                    ui.allocate_exact_size(Vec2::new(26.0, 22.0), egui::Sense::click());
                let p = ui.painter();
                let cy = dots_rect.center().y;
                p.circle_filled(
                    Pos2::new(dots_rect.min.x + 6.0, cy),
                    5.5,
                    Color32::from_rgb(255, 95, 87),
                );
                p.circle_filled(
                    Pos2::new(dots_rect.min.x + 22.0, cy),
                    5.5,
                    Color32::from_rgb(254, 188, 46),
                );
                p.circle_filled(
                    Pos2::new(dots_rect.min.x + 38.0, cy),
                    5.5,
                    Color32::from_rgb(40, 200, 64),
                );

                // Draw Amata Official Vector Logo
                let hover = logo_resp.hovered();
                let bg_color = if hover {
                    Color32::from_rgb(38, 38, 46)
                } else {
                    Color32::from_rgb(24, 24, 30)
                };
                p.rect_filled(logo_rect, 4.0, bg_color);
                let border_color = if hover {
                    Color32::from_rgb(79, 70, 229)
                } else {
                    Color32::from_rgb(46, 46, 56)
                };
                p.rect_stroke(
                    logo_rect,
                    4.0,
                    egui::Stroke::new(1.0_f32, border_color),
                    egui::StrokeKind::Outside,
                );
                crate::app::icons::icon_amata_logo(p, logo_rect.shrink(1.5));

                if logo_resp
                    .on_hover_text("Amata (数多) ホーム画面へ戻る")
                    .clicked()
                {
                    self.home_view.is_open = !self.home_view.is_open;
                }
                ui.add_space(8.0);

                ui.menu_button("ファイル (F)", |ui| {
                    if ui.button("新規ドキュメント... (Cmd+N)").clicked() {
                        self.new_doc_modal.is_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Open SVG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    self.state.document =
                                        crate::io::svg::parse_svg_document(&content);
                                    let obj_count = self.state.document.all_objects().count();
                                    self.state.document.name = path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("Untitled")
                                        .to_string();
                                    self.state.undo_manager.clear();
                                    self.state.selected_ids.clear();
                                    let mut watcher =
                                        crate::core::watcher::FileWatcher::new(path.clone());
                                    watcher.mark_saved(&content);
                                    self.file_watcher = Some(watcher);
                                    self.version_history_panel.refresh_history(&path);
                                    crate::io::recent::push_recent(
                                        &path,
                                        self.state.document.width,
                                        self.state.document.height,
                                    );
                                    if obj_count > 0 {
                                        self.state.notify_info(format!(
                                            "SVGをインポートしました ({} 個のオブジェクト)",
                                            obj_count
                                        ));
                                    } else {
                                        self.state
                                            .notify_info("SVGを読み込みました (オブジェクトなし)");
                                    }
                                }
                                Err(e) => {
                                    self.state
                                        .notify_error(format!("SVGの読み込みに失敗しました: {e}"));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("画像を配置... (Place Image)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
                            .pick_file()
                        {
                            let name = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Image")
                                .to_string();
                            match std::fs::read(&path) {
                                Err(e) => self.state.notify_error(format!(
                                    "画像の読み込みに失敗しました: {e}"
                                )),
                                Ok(bytes) => {
                                    // View-centre world coordinates.
                                    let zoom = self.state.zoom as f64;
                                    let (cx, cy) = (
                                        -self.state.pan_x as f64 / zoom,
                                        -self.state.pan_y as f64 / zoom,
                                    );
                                    self.canvas.place_image_bytes(
                                        &mut self.state,
                                        &bytes,
                                        &name,
                                        cx,
                                        cy,
                                    );
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save (Cmd+S)").clicked() {
                        if let Some(ref mut watcher) = self.file_watcher {
                            match crate::cli::handlers::common::save_any_document(
                                &self.state.document,
                                &watcher.file_path,
                            ) {
                                Err(e) => {
                                    self.state.notify_error(format!("保存に失敗しました: {e}"));
                                }
                                Ok(_) => {
                                    if let Ok(content) =
                                        std::fs::read_to_string(&watcher.file_path)
                                    {
                                        watcher.mark_saved(&content);
                                    } else {
                                        watcher.update_timestamp();
                                    }
                                    self.state.undo_manager.mark_saved();
                                    self.version_history_panel
                                        .refresh_history(&watcher.file_path);
                                    crate::io::recent::push_recent(
                                        &watcher.file_path,
                                        self.state.document.width,
                                        self.state.document.height,
                                    );
                                    crate::io::project::clear_recovery();
                                    self.state.notify_success("ファイルを上書き保存しました");
                                }
                            }
                        } else if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .save_file()
                        {
                            let svg = crate::io::svg::export_svg(&self.state.document);
                            if let Err(e) = std::fs::write(&path, &svg) {
                                self.state.notify_error(format!("保存に失敗しました: {e}"));
                            } else {
                                let mut watcher =
                                    crate::core::watcher::FileWatcher::new(path.clone());
                                watcher.mark_saved(&svg);
                                self.file_watcher = Some(watcher);
                                self.state.undo_manager.mark_saved();
                                self.version_history_panel.refresh_history(&path);
                                crate::io::recent::push_recent(
                                    &path,
                                    self.state.document.width,
                                    self.state.document.height,
                                );
                                crate::io::project::clear_recovery();
                                self.state.notify_success("SVGを保存しました");
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save Project (.amata / .json)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Amata Project", &["amata", "json"])
                            .save_file()
                        {
                            match crate::io::project::save_project(&self.state.document, &path) {
                                Ok(_) => {
                                    crate::io::recent::push_recent(
                                        &path,
                                        self.state.document.width,
                                        self.state.document.height,
                                    );
                                    crate::io::project::clear_recovery();
                                    self.state.notify_info("プロジェクトを保存しました");
                                }
                                Err(e) => {
                                    self.state.notify_error(format!("保存に失敗しました: {e}"));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Load Project (.amata / .json)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Amata Project", &["amata", "json"])
                            .pick_file()
                        {
                            match crate::io::project::load_project(&path) {
                                Ok(doc) => {
                                    self.state.document = doc;
                                    self.state.undo_manager.clear();
                                    self.state.selected_ids.clear();
                                    // Rebind save destination + watcher to the loaded project.
                                    // Otherwise Cmd+S would silently overwrite the previously
                                    // opened SVG with this project's content.
                                    let mut watcher =
                                        crate::core::watcher::FileWatcher::new(path.clone());
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        watcher.mark_saved(&content);
                                    }
                                    self.file_watcher = Some(watcher);
                                    self.version_history_panel.refresh_history(&path);
                                    crate::io::recent::push_recent(
                                        &path,
                                        self.state.document.width,
                                        self.state.document.height,
                                    );
                                    self.state.notify_info("プロジェクトを読み込みました");
                                }
                                Err(e) => {
                                    self.state
                                        .notify_error(format!("読み込みに失敗しました: {e}"));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export...  (Ctrl+Shift+E)").clicked() {
                        self.export_modal.is_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Export SVG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .save_file()
                        {
                            let svg = crate::io::svg::export_svg(&self.state.document);
                            match std::fs::write(&path, svg) {
                                Ok(_) => {
                                    self.state.notify_info("SVGを書き出しました");
                                }
                                Err(e) => {
                                    self.state
                                        .notify_error(format!("SVG書き出しに失敗しました: {e}"));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("🎬 VFX Pipeline", |ui| {
                        if ui
                            .button("Export for AEVFX Studio Comp (.json)...")
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("AEVFX Comp", &["json", "aevfx"])
                                .save_file()
                            {
                                let comp = crate::io::vfx::doc_to_aevfx_comp(
                                    &self.state.document,
                                    60.0,
                                    5.0,
                                );
                                match serde_json::to_string_pretty(&comp) {
                                    Ok(json) => match std::fs::write(&path, json) {
                                        Ok(_) => self
                                            .state
                                            .notify_info("AEVFXコンポジションを書き出しました"),
                                        Err(e) => self
                                            .state
                                            .notify_error(format!("書き出しに失敗しました: {e}")),
                                    },
                                    Err(e) => {
                                        self.state.notify_error(format!("変換に失敗しました: {e}"))
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button("Export Motion Path Keyframes (.json)...")
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Motion Path", &["json"])
                                .save_file()
                            {
                                let mut paths = Vec::new();
                                for (_, obj) in self.state.document.all_objects() {
                                    let kfs = crate::io::vfx::object_to_motion_path_keyframes(
                                        obj, 60, 5.0, 60.0,
                                    );
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
                                let obj_str = crate::io::vfx::export_doc_to_obj(
                                    &self.state.document,
                                    20.0,
                                    2.0,
                                );
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
                    let undo_name = self
                        .state
                        .undo_manager
                        .undo_name()
                        .unwrap_or("—")
                        .to_string();
                    let redo_name = self
                        .state
                        .undo_manager
                        .redo_name()
                        .unwrap_or("—")
                        .to_string();
                    if ui
                        .add_enabled(
                            can_undo,
                            egui::Button::new(format!("Undo ({undo_name})  (Ctrl+Z)")),
                        )
                        .clicked()
                    {
                        self.state.undo_manager.undo(&mut self.state.document);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            can_redo,
                            egui::Button::new(format!("Redo ({redo_name})  (Ctrl+Y)")),
                        )
                        .clicked()
                    {
                        self.state.undo_manager.redo(&mut self.state.document);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Select All  (Ctrl+A)").clicked() {
                        self.state.selected_ids = self
                            .state
                            .document
                            .all_objects()
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
                            if let Some((_, obj)) =
                                self.state.document.all_objects().find(|(_, o)| &o.id == id)
                            {
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
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            self.state
                                .undo_manager
                                .execute(cmd, &mut self.state.document);
                            new_ids.push(new_id);
                        }
                        self.state.selected_ids = new_ids;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Preferences...  (Ctrl+K)").clicked() {
                        self.preferences_dialog.is_open = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Object", |ui| {
                    let has_sel = !self.state.selected_ids.is_empty();
                    let multi_sel = self.state.selected_ids.len() >= 2;

                    if ui
                        .add_enabled(multi_sel, egui::Button::new("Group  (Ctrl+G)"))
                        .clicked()
                    {
                        let mut objs = Vec::new();
                        for id in &self.state.selected_ids {
                            if let Some(o) = self.state.document.remove_object(id) {
                                objs.push(o);
                            }
                        }
                        let group = Object::new_group("Group", objs);
                        let gid = group.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(group));
                        self.state
                            .undo_manager
                            .execute(cmd, &mut self.state.document);
                        self.state.selected_ids = vec![gid];
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(has_sel, egui::Button::new("Bring to Front"))
                        .clicked()
                    {
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

                    if ui
                        .add_enabled(has_sel, egui::Button::new("Send to Back"))
                        .clicked()
                    {
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

                    ui.separator();

                    if ui
                        .add_enabled(multi_sel, egui::Button::new("Make Compound Path  (Ctrl+8)"))
                        .clicked()
                    {
                        let sel = self.state.selected_ids.clone();
                        let mut objs = Vec::new();
                        for id in &sel {
                            if let Some(obj) = self.state.document.remove_object(id) {
                                objs.push(obj);
                            }
                        }
                        if let Some(compound) = Object::make_compound_path(&objs) {
                            let nid = compound.id.clone();
                            let cmd =
                                Box::new(crate::core::history::AddObjectCommand::new(compound));
                            self.state
                                .undo_manager
                                .execute(cmd, &mut self.state.document);
                            self.state.selected_ids = vec![nid];
                        }
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            has_sel,
                            egui::Button::new("Release Compound Path  (Ctrl+Alt+Shift+8)"),
                        )
                        .clicked()
                    {
                        let sel = self.state.selected_ids.clone();
                        let mut new_ids = Vec::new();
                        for id in &sel {
                            if let Some(obj) = self.state.document.remove_object(id) {
                                let released = obj.release_compound_path();
                                for r in released {
                                    let nid = r.id.clone();
                                    new_ids.push(nid);
                                    let cmd =
                                        Box::new(crate::core::history::AddObjectCommand::new(r));
                                    self.state
                                        .undo_manager
                                        .execute(cmd, &mut self.state.document);
                                }
                            }
                        }
                        self.state.selected_ids = new_ids;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Type", |ui| {
                    let has_sel = !self.state.selected_ids.is_empty();
                    if ui
                        .add_enabled(
                            has_sel,
                            egui::Button::new("Create Outlines  (Ctrl+Shift+O)"),
                        )
                        .clicked()
                    {
                        let sel = self.state.selected_ids.clone();
                        let mut new_ids = Vec::new();
                        for id in &sel {
                            if let Some(obj) = self.state.document.remove_object(id) {
                                if let Some(outlined) =
                                    crate::core::text_path::create_text_outlines(&obj)
                                {
                                    let nid = outlined.id.clone();
                                    new_ids.push(nid);
                                    let cmd = Box::new(
                                        crate::core::history::AddObjectCommand::new(outlined),
                                    );
                                    self.state
                                        .undo_manager
                                        .execute(cmd, &mut self.state.document);
                                } else {
                                    new_ids.push(obj.id.clone());
                                    let cmd =
                                        Box::new(crate::core::history::AddObjectCommand::new(obj));
                                    self.state
                                        .undo_manager
                                        .execute(cmd, &mut self.state.document);
                                }
                            }
                        }
                        self.state.selected_ids = new_ids;
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
                        if ui
                            .add_enabled(
                                multi,
                                egui::Button::new(format!("{} {}", op.icon(), name)),
                            )
                            .clicked()
                        {
                            let mut selected_objs: Vec<Object> = Vec::new();
                            for id in &self.state.selected_ids {
                                if let Some((_, obj)) =
                                    self.state.document.all_objects().find(|(_, o)| &o.id == id)
                                {
                                    selected_objs.push(obj.clone());
                                }
                            }
                            let obj_refs: Vec<&Object> = selected_objs.iter().collect();
                            if let Some(result_obj) = execute_pathfinder(&obj_refs, op) {
                                for id in &self.state.selected_ids {
                                    self.state.document.remove_object(id);
                                }
                                let new_id = result_obj.id.clone();
                                let cmd = Box::new(crate::core::history::AddObjectCommand::new(
                                    result_obj,
                                ));
                                self.state
                                    .undo_manager
                                    .execute(cmd, &mut self.state.document);
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
                    ui.add(
                        egui::DragValue::new(&mut self.state.grid_size)
                            .speed(10.0)
                            .prefix("Grid Size: ")
                            .range(5.0..=500.0),
                    );
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

                ui.menu_button("ヘルプ (H)", |ui| {
                    if ui.button("Amata について (About)...").clicked() {
                        self.about_modal.is_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("クイックツアーを開始...").clicked() {
                        self.onboarding_tour.is_active = true;
                        self.onboarding_tour.is_panel_open = true;
                        ui.close_menu();
                    }
                    if ui.button("ホーム画面を開く").clicked() {
                        self.home_view.is_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("キーボードショートカット (Cmd+/)").clicked() {
                        self.about_modal.is_open = true;
                        ui.close_menu();
                    }
                });

                // Right-aligned Utilities: Search bar, Share button, Bell, Profile
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Profile avatar circle
                    let (ava_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        ava_rect.center(),
                        9.0,
                        Color32::from_rgb(90, 90, 90),
                    );
                    ui.painter().text(
                        ava_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "K",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );

                    ui.add_space(4.0);

                    // Notifications Bell — painted vector icon
                    let _ = icon_button(ui, egui::Vec2::splat(20.0), |p, r, col| {
                        icon_bell(p, r, col);
                    })
                    .on_hover_text("通知");

                    ui.add_space(4.0);

                    // Pill-shaped Blue "共有" (Share) button
                    let share_btn = egui::Button::new(
                        RichText::new("共有")
                            .size(11.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(20, 115, 230))
                    .corner_radius(12)
                    .min_size(Vec2::new(56.0, 22.0));
                    if ui
                        .add(share_btn)
                        .on_hover_text("プロジェクトを共有または書き出し")
                        .clicked()
                    {
                        self.export_modal.is_open = true;
                    }

                    ui.add_space(6.0);

                    // Search help input with vector search icon
                    ui.horizontal(|ui| {
                        let (s_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        icon_search(ui.painter(), s_rect, Color32::from_gray(160));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("ヘルプを検索...")
                                .desired_width(110.0),
                        );
                    });

                    ui.add_space(4.0);

                    // Illustrator Signature Workspace Preset Switcher (Image 1 & 3 top right)
                    egui::ComboBox::from_id_salt("workspace_preset_switcher")
                        .selected_text(match self.active_tab {
                            ActiveTab::Properties => "初期設定",
                            ActiveTab::Layers => "レイヤー",
                            ActiveTab::Pathfinder => "パスファインダー",
                            ActiveTab::ThreeDAndVfx => "3D & VFX",
                            ActiveTab::Generative => "ジェネレーティブ",
                            ActiveTab::Symbols => "ライブラリ",
                            ActiveTab::Components => "コンポーネント",
                            ActiveTab::VersionHistory => "バージョン履歴",
                            ActiveTab::Export => "Web・書き出し",
                            ActiveTab::Guides => "ガイド・配置",
                        })
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::Properties,
                                    "初期設定 (プロパティ)",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Properties;
                            }
                            if ui
                                .selectable_label(self.active_tab == ActiveTab::Layers, "レイヤー")
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Layers;
                            }
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::Pathfinder,
                                    "パス編集",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Pathfinder;
                            }
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::ThreeDAndVfx,
                                    "3D とマテリアル",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::ThreeDAndVfx;
                            }
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::Generative,
                                    "ジェネレーティブデザイン",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Generative;
                            }
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::Symbols,
                                    "グラフィックライブラリ",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Symbols;
                            }
                            if ui
                                .selectable_label(
                                    self.active_tab == ActiveTab::Export,
                                    "Web & アセット書き出し",
                                )
                                .clicked()
                            {
                                self.active_tab = ActiveTab::Export;
                            }
                        });
                });
            });
        });
    }
}
