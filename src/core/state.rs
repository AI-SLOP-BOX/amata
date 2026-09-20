use super::document::{Document, Layer, Object, Transform};
use super::history::{BatchCommand, Command, LayerCommand, ObjectCommand, TransformCommand, UndoManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Node,
    Pen,
    Pencil,
    Rectangle,
    Ellipse,
    Star,
    Polygon,
    Line,
    Text,
    Eyedropper,
    Hand,
    Brush,
    Eraser,
    ShapeBuilder,
    Zoom,
    PixelPencil,
    PixelEraser,
    PixelBucket,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Selection",
            Tool::Node => "Direct Select",
            Tool::Pen => "Pen",
            Tool::Pencil => "Pencil (Smooth)",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Star => "Star",
            Tool::Polygon => "Polygon",
            Tool::Line => "Line Segment",
            Tool::Text => "Type",
            Tool::Eyedropper => "Eyedropper",
            Tool::Hand => "Hand",
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
            Tool::ShapeBuilder => "Shape Builder",
            Tool::Zoom => "Zoom",
            Tool::PixelPencil => "Pixel Pencil",
            Tool::PixelEraser => "Pixel Eraser",
            Tool::PixelBucket => "Pixel Bucket",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tool::Select => "↖",
            Tool::Node => "⇱",
            Tool::Pen => "✒",
            Tool::Pencil => "✎",
            Tool::Rectangle => "▭",
            Tool::Ellipse => "○",
            Tool::Star => "★",
            Tool::Polygon => "⬡",
            Tool::Line => "╱",
            Tool::Text => "T",
            Tool::Eyedropper => "⚗",
            Tool::Hand => "✋",
            Tool::Brush => "∂",
            Tool::Eraser => "⌫",
            Tool::ShapeBuilder => "⊕",
            Tool::Zoom => "⊕",
            Tool::PixelPencil => "▦",
            Tool::PixelEraser => "▧",
            Tool::PixelBucket => "🪣",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Node => "A",
            Tool::Pen => "P",
            Tool::Pencil => "N",
            Tool::Rectangle => "U",
            Tool::Ellipse => "O",
            Tool::Star => "S",
            Tool::Polygon => "G",
            Tool::Line => "L",
            Tool::Text => "T",
            Tool::Eyedropper => "I",
            Tool::Hand => "H",
            Tool::Brush => "B",
            Tool::Eraser => "E",
            Tool::ShapeBuilder => "M",
            Tool::Zoom => "Z",
            Tool::PixelPencil => "X",
            Tool::PixelEraser => "C",
            Tool::PixelBucket => "K",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Top,
    Bottom,
    Left,
    Right,
    RotateTopRight,
}

pub struct AppState {
    pub document: Document,
    pub undo_manager: UndoManager,
    pub current_tool: Tool,
    pub previous_tool: Tool,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub target_zoom: f32,
    pub target_pan_x: f32,
    pub target_pan_y: f32,
    pub zoom_animation_progress: f32,
    pub start_zoom: f32,
    pub start_pan_x: f32,
    pub start_pan_y: f32,
    pub selected_ids: Vec<String>,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub show_grid: bool,
    pub grid_size: f64,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    pub snap_to_guides: bool,
    pub snap_to_points: bool,
    pub show_rulers: bool,
    pub show_smart_guides: bool,
    pub fill_color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f64,
    pub opacity: f32,
    pub corner_radius: f64,
    pub star_points: usize,
    pub star_inner_ratio: f64,
    pub polygon_sides: usize,
    pub font_size: f64,
    pub text_input_buf: String,
    pub is_panning: bool,
    pub drag_start: Option<(f32, f32)>,
    pub clipboard: Vec<crate::core::document::Object>,
    pub timeline: super::timeline::Timeline,
    pub show_timeline: bool,
    // Artboard navigation
    pub active_artboard_idx: usize,
    // Symbols
    pub symbols: Vec<crate::core::document::Symbol>,
    // Guides
    pub guides: Vec<Guide>,
    // Export
    pub export_format: String,
    pub export_width: f64,
    pub export_height: f64,
    pub export_scale: f32,
    pub export_transparent: bool,
    pub export_svg_viewbox: bool,
    pub export_svg_embed_fonts: bool,
    pub export_scope: String,
    pub export_path: Option<String>,
    pub pending_export: bool,
    // Repeat
    pub repeat_cols: usize,
    pub repeat_rows: usize,
    pub repeat_h_gap: f64,
    pub repeat_v_gap: f64,
    pub repeat_radial_count: usize,
    pub repeat_radial_radius: f64,
    pub repeat_start_angle: f64,
    // Canvas center (set each frame)
    pub canvas_center_x: f32,
    pub canvas_center_y: f32,
    // Live cursor position in world coordinates (updated each frame by canvas)
    pub cursor_world: Option<(f64, f64)>,
    // Floating toast notification feedback
    pub toast: Option<ToastNotification>,
    // Active visual diff overlay on canvas
    pub active_diff: Option<crate::core::diff::SemanticDiff>,
    pub is_comparing_diff: bool,
    // Group isolation editing (Illustrator-style double-click into group):
    // id of the top-level group whose children are directly editable.
    pub isolated_group_id: Option<String>,
    // Timeline playback bookkeeping: previous frame's playing flag plus
    // object snapshots taken when playback started, committed as one undo
    // step when playback stops (otherwise played values stick forever
    // with no undo and no dirty flag).
    pub timeline_was_playing: bool,
    // Pathfinder simplify tolerance (squared px area) remembered by UI.
    pub simplify_tolerance: f64,
    // Pixel-art (dot絵) editing: selected palette index for the pencil /
    // bucket, and whether the per-cell grid overlay is drawn.
    pub pixel_palette_index: usize,
    pub pixel_show_grid: bool,
    /// Edge length for newly created square pixel canvases.
    pub pixel_new_size: u32,
    // In-progress panel transform gesture: (object id, transform at gesture
    // start). Committed as one undo step when the gesture ends.
    pub pending_transforms: Vec<(String, Transform)>,
    // Same mechanism for whole-object panel edits (opacity, stroke, fill…).
    pub pending_objects: Vec<(String, Object)>,
    // Same for whole-layer edits (opacity).
    pub pending_layers: Vec<(String, Layer)>,
}

#[derive(Debug, Clone)]
pub struct ToastNotification {
    pub message: String,
    pub is_error: bool,
    pub created_at: std::time::Instant,
}

pub use super::document::{Guide, GuideOrientation};

impl Default for AppState {
    fn default() -> Self {
        Self {
            document: Document::default(),
            undo_manager: UndoManager::new(),
            current_tool: Tool::Select,
            previous_tool: Tool::Select,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            target_zoom: 1.0,
            target_pan_x: 0.0,
            target_pan_y: 0.0,
            zoom_animation_progress: 1.0,
            start_zoom: 1.0,
            start_pan_x: 0.0,
            start_pan_y: 0.0,
            selected_ids: Vec::new(),
            canvas_width: 0.0,
            canvas_height: 0.0,
            show_grid: true,
            grid_size: 50.0,
            snap_to_grid: false,
            snap_to_objects: true,
            snap_to_guides: true,
            snap_to_points: true,
            show_rulers: true,
            show_smart_guides: true,
            fill_color: [0.2, 0.5, 0.8, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
            opacity: 1.0,
            corner_radius: 0.0,
            star_points: 5,
            star_inner_ratio: 0.4,
            polygon_sides: 6,
            font_size: 32.0,
            text_input_buf: "Hello Illustrator".to_string(),
            is_panning: false,
            drag_start: None,
            clipboard: Vec::new(),
            timeline: super::timeline::Timeline::default(),
            show_timeline: true,
            active_artboard_idx: 0,
            symbols: vec![
                crate::core::document::Symbol::new(
                    "ハート (Heart)",
                    crate::core::presets::PresetLibrary::heart("Heart", 0.0, 0.0, 80.0),
                ),
                crate::core::document::Symbol::new(
                    "矢印 (Arrow)",
                    crate::core::presets::PresetLibrary::arrow("Arrow", 0.0, 0.0, 100.0, 30.0),
                ),
                crate::core::document::Symbol::new(
                    "ギア (Gear)",
                    crate::core::presets::PresetLibrary::gear("Gear", 0.0, 0.0, 8, 25.0, 45.0),
                ),
                crate::core::document::Symbol::new(
                    "吹き出し (Bubble)",
                    crate::core::presets::PresetLibrary::speech_bubble(
                        "Bubble", 0.0, 0.0, 100.0, 60.0,
                    ),
                ),
                crate::core::document::Symbol::new(
                    "ポータル (Hex Ring)",
                    crate::core::presets::PresetLibrary::vfx_portal("Portal", 0.0, 0.0, 50.0),
                ),
                crate::core::document::Symbol::new(
                    "検索アイコン (Search)",
                    crate::core::presets::PresetLibrary::search_icon("Search", 0.0, 0.0, 40.0),
                ),
                crate::core::document::Symbol::new(
                    "ユーザー (User)",
                    crate::core::presets::PresetLibrary::user_avatar("User", 0.0, 0.0, 50.0),
                ),
                crate::core::document::Symbol::new(
                    "クラウド (Cloud)",
                    crate::core::presets::PresetLibrary::cloud("Cloud", 0.0, 0.0, 90.0),
                ),
                crate::core::document::Symbol::new(
                    "カート (Shopping Cart)",
                    crate::core::presets::PresetLibrary::shopping_cart("Cart", 0.0, 0.0, 60.0),
                ),
                crate::core::document::Symbol::new(
                    "リボンバッジ (Ribbon)",
                    crate::core::presets::PresetLibrary::ribbon_badge(
                        "Ribbon", 0.0, 0.0, 110.0, 45.0,
                    ),
                ),
            ],
            guides: Vec::new(),
            export_format: "SVG".into(),
            export_width: 1920.0,
            export_height: 1080.0,
            export_scale: 1.0,
            export_transparent: false,
            export_svg_viewbox: true,
            export_svg_embed_fonts: true,
            export_scope: "All".into(),
            export_path: None,
            pending_export: false,
            repeat_cols: 3,
            repeat_rows: 3,
            repeat_h_gap: 50.0,
            repeat_v_gap: 50.0,
            repeat_radial_count: 8,
            repeat_radial_radius: 200.0,
            repeat_start_angle: 0.0,
            canvas_center_x: 0.0,
            canvas_center_y: 0.0,
            cursor_world: None,
            toast: None,
            active_diff: None,
            is_comparing_diff: false,
            isolated_group_id: None,
            timeline_was_playing: false,
            simplify_tolerance: 5.0,
            pixel_palette_index: 0,
            pixel_show_grid: true,
            pixel_new_size: 32,
            pending_transforms: Vec::new(),
            pending_objects: Vec::new(),
            pending_layers: Vec::new(),
        }
    }
}

impl AppState {
    /// Snapshot the object's transform at the start of a panel gesture.
    /// Called before every mutation; keeps the first snapshot only.
    pub fn ensure_transform_snapshot(&mut self, id: &str) {
        if !self.pending_transforms.iter().any(|(pid, _)| pid == id) {
            if let Some(t) = self.document.find_object(id).map(|o| o.transform.clone()) {
                self.pending_transforms.push((id.to_string(), t));
            }
        }
    }

    /// Record the gesture from the snapshots as one undo step (no-op when
    /// nothing actually changed, e.g. a drag that returned to start).
    pub fn commit_transform_edits(&mut self, label: &str) {
        let pending = std::mem::take(&mut self.pending_transforms);
        let mut cmds: Vec<(String, Transform, Transform)> = Vec::new();
        for (id, old) in pending {
            if let Some(obj) = self.document.find_object(&id) {
                if obj.transform != old {
                    cmds.push((id, old, obj.transform.clone()));
                }
            }
        }
        if cmds.len() == 1 {
            let (id, old, new) = cmds.into_iter().next().unwrap();
            self.undo_manager.execute(
                Box::new(TransformCommand {
                    object_id: id,
                    old_t: old,
                    new_t: new,
                }),
                &mut self.document,
            );
        } else if !cmds.is_empty() {
            let batch: Vec<Box<dyn Command>> = cmds
                .into_iter()
                .map(|(id, old, new)| {
                    Box::new(TransformCommand {
                        object_id: id,
                        old_t: old,
                        new_t: new,
                    }) as Box<dyn Command>
                })
                .collect();
            self.undo_manager.execute(
                Box::new(BatchCommand::new(label, batch)),
                &mut self.document,
            );
        }
    }

    /// The isolated group object, if isolation is active and the group
    /// still exists as a top-level group (auto-invalidated otherwise).
    pub fn isolated_group(&self) -> Option<&crate::core::document::Object> {
        let gid = self.isolated_group_id.as_ref()?;
        let (_, obj) = self.document.all_objects().find(|(_, o)| &o.id == gid)?;
        if matches!(
            obj.object_type,
            crate::core::document::ObjectType::Group(_)
        ) {
            Some(obj)
        } else {
            None
        }
    }

    pub fn exit_isolation(&mut self) {
        self.isolated_group_id = None;
        self.selected_ids.clear();
    }

    /// Push live timeline/guides into the document before any
    /// save/export/checkpoint/autosave so they persist.
    pub fn sync_doc_extras(&mut self) {
        self.document.timeline = self.timeline.clone();
        self.document.guides = self.guides.clone();
    }

    /// Pull timeline/guides from a freshly loaded document into live state.
    pub fn adopt_doc_extras(&mut self) {
        self.timeline = self.document.timeline.clone();
        self.guides = self.document.guides.clone();
    }

    /// Snapshot a whole object before a non-transform panel edit.
    pub fn ensure_object_snapshot(&mut self, id: &str) {
        if !self.pending_objects.iter().any(|(pid, _)| pid == id) {
            if let Some(o) = self.document.find_object(id) {
                self.pending_objects.push((id.to_string(), o.clone()));
            }
        }
    }

    /// Record pending whole-object gestures as one undo step (or a batch).
    pub fn commit_object_edits(&mut self, label: &str) {
        let pending = std::mem::take(&mut self.pending_objects);
        let mut cmds: Vec<(String, Object, Object)> = Vec::new();
        for (id, old) in pending {
            if let Some(obj) = self.document.find_object(&id) {
                if obj != &old {
                    cmds.push((id, old, obj.clone()));
                }
            }
        }
        if cmds.len() == 1 {
            let (id, old, new) = cmds.into_iter().next().unwrap();
            self.undo_manager.execute(
                Box::new(ObjectCommand {
                    object_id: id,
                    old_obj: old,
                    new_obj: new,
                }),
                &mut self.document,
            );
        } else if !cmds.is_empty() {
            let batch: Vec<Box<dyn Command>> = cmds
                .into_iter()
                .map(|(id, old, new)| {
                    Box::new(ObjectCommand {
                        object_id: id,
                        old_obj: old,
                        new_obj: new,
                    }) as Box<dyn Command>
                })
                .collect();
            self.undo_manager.execute(
                Box::new(BatchCommand::new(label, batch)),
                &mut self.document,
            );
        }
    }

    /// Panel-widget edit of one object with drag coalescing.
    /// Mutate via `f`, then the gesture commits as one undo step
    /// (immediately for keyboard/click edits, on drag-stop for drags).
    /// Callers must additionally call `commit_object_edits` when
    /// `resp.drag_stopped()` fires outside a `changed()` frame.
    pub fn object_edit(
        &mut self,
        id: &str,
        resp: &egui::Response,
        f: impl FnOnce(&mut Object),
    ) {
        self.ensure_object_snapshot(id);
        if let Some(o) = self.document.find_object_mut(id) {
            f(o);
        }
        if !resp.dragged() {
            self.commit_object_edits("Edit Object");
        }
    }

    /// Multi-selection variant: one undo step total, never one per object.
    pub fn objects_edit(
        &mut self,
        ids: &[String],
        resp: &egui::Response,
        mut f: impl FnMut(&mut Object),
    ) {
        for id in ids {
            self.ensure_object_snapshot(id);
        }
        for id in ids {
            if let Some(o) = self.document.find_object_mut(id) {
                f(o);
            }
        }
        if !resp.dragged() {
            self.commit_object_edits("Edit Object");
        }
    }

    /// Reorder objects (z-order ops) as one undo step. The mutation runs
    /// inside `f`; per-object absolute positions are diffed, so sibling
    /// shifts cannot corrupt the restore.
    pub fn reorder_objects_undoable(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut crate::core::document::Document),
    ) {
        let mut before: Vec<(String, usize, usize)> = Vec::new();
        for (li, layer) in self.document.layers.iter().enumerate() {
            for (pos, obj) in layer.objects.iter().enumerate() {
                before.push((obj.id.clone(), li, pos));
            }
        }
        f(&mut self.document);
        let mut cmds: Vec<Box<dyn Command>> = Vec::new();
        for (id, li, old_pos) in before {
            if let Some(layer) = self.document.layers.get(li) {
                if let Some(new_pos) = layer.objects.iter().position(|o| o.id == id) {
                    if new_pos != old_pos {
                        cmds.push(Box::new(
                            crate::core::history::ReorderObjectCommand {
                                object_id: id,
                                layer_idx: li,
                                old_position: old_pos,
                                new_position: new_pos,
                            },
                        )
                            as Box<dyn Command>);
                    }
                }
            }
        }
        if cmds.len() == 1 {
            let cmd = cmds.pop().unwrap();
            self.undo_manager.execute(cmd, &mut self.document);
        } else if !cmds.is_empty() {
            self.undo_manager.execute(
                Box::new(BatchCommand::new(label, cmds)),
                &mut self.document,
            );
        }
    }

    /// Snapshot a layer before a panel edit (see object variant).
    pub fn ensure_layer_snapshot(&mut self, id: &str) {
        if !self.pending_layers.iter().any(|(lid, _)| lid == id) {
            if let Some(l) = self.document.layers.iter().find(|l| l.id == id) {
                self.pending_layers.push((id.to_string(), l.clone()));
            }
        }
    }

    /// Record pending layer gestures as one undo step (or a batch).
    pub fn commit_layer_edits(&mut self, label: &str) {
        let pending = std::mem::take(&mut self.pending_layers);
        let mut cmds: Vec<Box<dyn Command>> = Vec::new();
        for (id, old) in pending {
            if let Some(layer) = self.document.layers.iter().find(|l| l.id == id) {
                if layer != &old {
                    cmds.push(Box::new(LayerCommand {
                        layer_id: id,
                        old_layer: old,
                        new_layer: layer.clone(),
                    }) as Box<dyn Command>);
                }
            }
        }
        if cmds.len() == 1 {
            let cmd = cmds.pop().unwrap();
            self.undo_manager.execute(cmd, &mut self.document);
        } else if !cmds.is_empty() {
            self.undo_manager.execute(
                Box::new(BatchCommand::new(label, cmds)),
                &mut self.document,
            );
        }
    }

    /// Replace the current selection with computed results as ONE undo
    /// step. Snapshots are taken before anything runs; when `build`
    /// returns None (e.g. a failed compound) the document is untouched.
    pub fn replace_selected(
        &mut self,
        label: &str,
        build: impl FnOnce(Vec<Object>) -> Option<(Vec<Object>, Vec<String>)>,
    ) {
        let removed = crate::core::history::collect_located_objects(
            &self.document,
            &self.selected_ids,
        );
        if removed.is_empty() {
            return;
        }
        let objects: Vec<Object> = removed.iter().map(|item| item.object.clone()).collect();
        if let Some((added, new_ids)) = build(objects) {
            let cmd = Box::new(crate::core::history::ReplaceObjectsCommand::new(
                label, removed, added,
            ));
            self.undo_manager.execute(cmd, &mut self.document);
            self.selected_ids = new_ids;
        }
    }

    /// Like [`Self::replace_selected`], but only selected objects passing
    /// `keep` participate — the rest stay untouched with no undo step.
    /// (Used when an operation is a no-op for some objects, e.g. releasing
    /// a non-compound path, where a pass-through would pointlessly move
    /// them and pollute history.)
    pub fn replace_selected_where(
        &mut self,
        label: &str,
        mut keep: impl FnMut(&Object) -> bool,
        build: impl FnOnce(Vec<Object>) -> Option<(Vec<Object>, Vec<String>)>,
    ) {
        let kept_ids: Vec<String> = self
            .selected_ids
            .iter()
            .filter(|id| {
                self.document
                    .find_object(id)
                    .map(&mut keep)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if kept_ids.is_empty() {
            return;
        }
        let removed = crate::core::history::collect_located_objects(&self.document, &kept_ids);
        if removed.is_empty() {
            return;
        }
        let objects: Vec<Object> = removed.iter().map(|item| item.object.clone()).collect();
        if let Some((added, new_ids)) = build(objects) {
            let cmd = Box::new(crate::core::history::ReplaceObjectsCommand::new(
                label, removed, added,
            ));
            self.undo_manager.execute(cmd, &mut self.document);
            self.selected_ids = new_ids;
        }
    }

    pub fn notify_info(&mut self, message: impl Into<String>) {
        self.toast = Some(ToastNotification {
            message: message.into(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
    }

    pub fn notify_success(&mut self, message: impl Into<String>) {
        self.toast = Some(ToastNotification {
            message: message.into(),
            is_error: false,
            created_at: std::time::Instant::now(),
        });
    }

    pub fn notify_error(&mut self, message: impl Into<String>) {
        self.toast = Some(ToastNotification {
            message: message.into(),
            is_error: true,
            created_at: std::time::Instant::now(),
        });
    }

    pub fn clear_toast_if_expired(&mut self) {
        if let Some(toast) = &self.toast {
            if toast.created_at.elapsed().as_secs_f32() > 3.5 {
                self.toast = None;
            }
        }
    }

    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f64, f64) {
        let wx = (sx - self.canvas_center_x - self.pan_x) as f64 / self.zoom as f64;
        let wy = (sy - self.canvas_center_y - self.pan_y) as f64 / self.zoom as f64;
        (wx, wy)
    }

    pub fn world_to_screen(&self, wx: f64, wy: f64) -> (f32, f32) {
        let sx = wx as f32 * self.zoom + self.pan_x + self.canvas_center_x;
        let sy = wy as f32 * self.zoom + self.pan_y + self.canvas_center_y;
        (sx, sy)
    }

    pub fn snap(&self, x: f64, y: f64) -> (f64, f64) {
        let (mut sx, mut sy) = if self.snap_to_grid && self.grid_size > 0.0 {
            (
                (x / self.grid_size).round() * self.grid_size,
                (y / self.grid_size).round() * self.grid_size,
            )
        } else {
            (x, y)
        };

        if self.snap_to_objects {
            let objects: Vec<&crate::core::document::Object> = self
                .document
                .all_objects()
                .filter(|(_, o)| o.visible && !o.locked)
                .map(|(_, o)| o)
                .collect();
            let (osx, osy) = self.snap_to_object_edges(x, y, &objects);
            let grid_dist = ((sx - x).powi(2) + (sy - y).powi(2)).sqrt();
            let obj_dist = ((osx - x).powi(2) + (osy - y).powi(2)).sqrt();
            if obj_dist < grid_dist {
                sx = osx;
                sy = osy;
            }
        }

        (sx, sy)
    }

    /// Snaps to the nearest object edge or center within a threshold (5px in world space).
    /// Returns the snapped position, or the original position if nothing is close enough.
    pub fn snap_to_object_edges(
        &self,
        x: f64,
        y: f64,
        objects: &[&crate::core::document::Object],
    ) -> (f64, f64) {
        let threshold = 5.0;
        let mut best_x = x;
        let mut best_y = y;
        let mut best_dist = f64::MAX;

        for obj in objects {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                let cx = (bb_min.x + bb_max.x) / 2.0;
                let cy = (bb_min.y + bb_max.y) / 2.0;

                let edges = [
                    (bb_min.x, cy),       // left edge center
                    (bb_max.x, cy),       // right edge center
                    (cx, bb_min.y),       // top edge center
                    (cx, bb_max.y),       // bottom edge center
                    (bb_min.x, bb_min.y), // top-left corner
                    (bb_max.x, bb_min.y), // top-right corner
                    (bb_min.x, bb_max.y), // bottom-left corner
                    (bb_max.x, bb_max.y), // bottom-right corner
                    (cx, cy),             // center
                ];

                for (ex, ey) in edges {
                    let dist = ((ex - x).powi(2) + (ey - y).powi(2)).sqrt();
                    if dist < threshold && dist < best_dist {
                        best_dist = dist;
                        best_x = ex;
                        best_y = ey;
                    }
                }
            }
        }

        (best_x, best_y)
    }

    /// Calculate and apply zoom-to-fit, setting animation targets
    pub fn zoom_to_fit(&mut self) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for (_, obj) in self.document.all_objects() {
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
            let cw = self.canvas_width;
            let ch = self.canvas_height;
            let new_zoom = ((cw / obj_w as f32).min(ch / obj_h as f32) * 0.85).clamp(0.01, 100.0);
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;
            self.target_zoom = new_zoom;
            self.target_pan_x = -center_x as f32 * new_zoom;
            self.target_pan_y = -center_y as f32 * new_zoom;
        } else {
            self.target_zoom = 1.0;
            self.target_pan_x = 0.0;
            self.target_pan_y = 0.0;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.undo_manager.is_dirty()
    }

    pub fn mark_saved(&mut self) {
        self.undo_manager.mark_saved();
    }
}
