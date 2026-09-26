use super::document::{Document, Layer, Object};
use super::history::{BatchCommand, Command, LayerCommand, ObjectCommand, TransformCommand, UndoManager};
use super::prefs::Prefs;

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
    /// User preferences (環境設定) — edited live by the preferences dialog
    /// and read by the renderer, the theme and the startup path.
    pub prefs: Prefs,
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
    /// Round the final snapped position to integer document units (crisp
    /// 1px strokes for logos / pixel-aligned UI work).
    pub snap_to_pixels: bool,
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
    /// Property panel: show the artboard editor (size / position / name).
    pub artboard_edit_open: bool,
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
    // Brush panel settings (calligraphy / art / pattern).
    pub brush_kind_idx: usize,
    pub brush_angle: f64,
    pub brush_roundness: f64,
    pub brush_size: f64,
    pub brush_motif: String,
    pub brush_spacing: f64,
    pub brush_scale: f64,
    // Bristle brush params + custom artwork + library name buffer.
    pub brush_bristles: f64,
    pub brush_scatter: f64,
    pub brush_opacity: f64,
    pub brush_artwork: Option<crate::core::path::PathData>,
    pub brush_custom_art: bool,
    pub brush_lib_name: String,
    // Print export switches.
    pub print_marks: bool,
    pub print_pdfx: bool,
    // In-progress transform gesture: (object id, object as it was at gesture
    // start). Committed as one undo step when the gesture ends.
    pub pending_transforms: Vec<(String, Object)>,
    // Same mechanism for whole-object panel edits (opacity, stroke, fill…).
    pub pending_objects: Vec<(String, Object)>,
    // Same for whole-layer edits (opacity).
    pub pending_layers: Vec<(String, Layer)>,
    /// Artboard edits (property panel): the artboard list as it was before the
    /// edit started, so a drag collapses into one undo step.
    pub pending_artboards: Option<Vec<crate::core::document::Artboard>>,
    /// Layer-tree collapsed group ids (open when absent from this set).
    pub tree_collapsed: std::collections::HashSet<String>,
    /// Inline rename in the layer tree: (object id, text buffer).
    pub tree_rename: Option<(String, String)>,
    pub tree_rename_focused: bool,
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
            prefs: Prefs::default(),
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
            snap_to_pixels: false,
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
            artboard_edit_open: false,
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
            brush_kind_idx: 0,
            brush_angle: 30.0,
            brush_roundness: 50.0,
            brush_size: 10.0,
            brush_motif: "arrow".to_string(),
            brush_spacing: 60.0,
            brush_scale: 1.0,
            brush_bristles: 12.0,
            brush_scatter: 0.4,
            brush_opacity: 0.8,
            brush_artwork: None,
            brush_custom_art: false,
            brush_lib_name: "My Brush".to_string(),
            print_marks: true,
            print_pdfx: true,
            pending_transforms: Vec::new(),
            pending_objects: Vec::new(),
            pending_layers: Vec::new(),
            pending_artboards: None,
            tree_collapsed: std::collections::HashSet::new(),
            tree_rename: None,
            tree_rename_focused: false,
        }
    }
}

impl AppState {
    /// Snapshot the object at the start of a transform gesture.
    ///
    /// The whole object, not just the transform: a resize with 「角を拡大・
    /// 縮小」/「線幅と効果を拡大・縮小」off also rewrites corner radii,
    /// stroke widths and effect sizes (see `Object::apply_scale_change`),
    /// and undo has to bring those back together with the transform.
    /// Called before every mutation; keeps the first snapshot only.
    pub fn ensure_transform_snapshot(&mut self, id: &str) {
        if !self.pending_transforms.iter().any(|(pid, _)| pid == id) {
            if let Some(obj) = self.document.find_object(id) {
                self.pending_transforms.push((id.to_string(), obj.clone()));
            }
        }
    }

    /// Record the gesture from the snapshots as one undo step (no-op when
    /// nothing actually changed, e.g. a drag that returned to start).
    ///
    /// A gesture that only moved or resized keeps the cheap
    /// [`TransformCommand`]; one that also counter-scaled absolute-valued
    /// attributes commits the whole object instead, so undo restores them
    /// along with the transform rather than leaving a half-scaled object.
    pub fn commit_transform_edits(&mut self, label: &str) {
        let pending = std::mem::take(&mut self.pending_transforms);
        let mut cmds: Vec<Box<dyn Command>> = Vec::new();
        for (id, old) in pending {
            let Some(obj) = self.document.find_object(&id) else {
                continue;
            };
            if obj.transform == old.transform {
                continue;
            }
            // Did anything besides the transform move?
            let mut probe = obj.clone();
            probe.transform = old.transform.clone();
            if probe == old {
                cmds.push(Box::new(TransformCommand {
                    object_id: id.clone(),
                    old_t: old.transform.clone(),
                    new_t: obj.transform.clone(),
                }));
            } else {
                cmds.push(Box::new(ObjectCommand {
                    object_id: id,
                    old_obj: old,
                    new_obj: obj.clone(),
                }));
            }
        }
        if cmds.len() == 1 {
            let cmd = cmds.into_iter().next().unwrap();
            self.undo_manager.execute(cmd, &mut self.document);
        } else if !cmds.is_empty() {
            self.undo_manager
                .execute(Box::new(BatchCommand::new(label, cmds)), &mut self.document);
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

    /// Snapshot objects before a non-drag document mutation (batch flips,
    /// stroke presets, etc.) then record as one undo step.
    pub fn undoable_snapshot(&mut self, label: &str, ids: &[String], f: impl FnOnce(&mut Document)) {
        for id in ids {
            self.ensure_object_snapshot(id);
        }
        f(&mut self.document);
        self.commit_object_edits(label);
    }

    /// Record a full artboards list replacement as one undo step.
    pub fn push_artboards_undo(
        &mut self,
        label: &str,
        before: Vec<crate::core::document::Artboard>,
        after: Vec<crate::core::document::Artboard>,
    ) {
        if before == after {
            return;
        }
        let cmd = Box::new(crate::core::history::SetArtboardsCommand::new(
            label, before, after,
        ));
        self.undo_manager.execute(cmd, &mut self.document);
    }

    /// Snapshot the artboard list before an artboard edit (see the object
    /// variant).  A drag therefore becomes a single undo step instead of one
    /// step per frame.
    pub fn ensure_artboard_snapshot(&mut self) {
        if self.pending_artboards.is_none() {
            self.pending_artboards = Some(self.document.artboards.clone());
        }
    }

    /// Apply an artboard edit.  Changes commit immediately, except while the
    /// widget is mid-drag, where [`Self::commit_artboard_edits`] closes the
    /// gesture out (call it from `drag_stopped`).
    pub fn artboard_edit(
        &mut self,
        resp: &egui::Response,
        label: &str,
        f: impl FnOnce(&mut Vec<crate::core::document::Artboard>),
    ) {
        self.ensure_artboard_snapshot();
        f(&mut self.document.artboards);
        if !resp.dragged() {
            self.commit_artboard_edits(label);
        }
    }

    /// Push the pending artboard snapshot as one undo step.  No-op when the
    /// list did not actually change.
    pub fn commit_artboard_edits(&mut self, label: &str) {
        let Some(before) = self.pending_artboards.take() else {
            return;
        };
        self.push_artboards_undo(label, before, self.document.artboards.clone());
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

    /// Snap a world-space point against every enabled snap target and return
    /// the snapped position.
    ///
    /// Targets compete on their distance from `(x, y)`: the closest one wins,
    /// so a guide1px away beats a grid line 30px away. Guides (like the grid)
    /// pin a single axis, objects / anchor points / the perspective grid snap
    /// both axes at once.
    pub fn snap(&self, x: f64, y: f64) -> (f64, f64) {
        /// Reach of the point-like targets — the window
        /// `snap_to_object_edges` has always used.
        const THRESHOLD: f64 = 5.0;

        let (mut sx, mut sy) = (x, y);
        // Distance of each axis' current value from the raw point. An axis
        // that has not snapped yet carries no distance at all (`snapped_* =
        // false`), which is what lets the first candidate win instead of
        // being rejected against a distance of 0.
        let mut x_snapped = false;
        let mut y_snapped = false;

        // Grid: no threshold — it always lands on the nearest line.
        if self.snap_to_grid && self.grid_size > 0.0 {
            sx = (x / self.grid_size).round() * self.grid_size;
            sy = (y / self.grid_size).round() * self.grid_size;
            x_snapped = true;
            y_snapped = true;
        }

        // Guides: a horizontal guide pins Y, a vertical one pins X.
        if self.snap_to_guides {
            for guide in &self.guides {
                match guide.orientation {
                    GuideOrientation::Horizontal => {
                        let d = (guide.position - y).abs();
                        if d <= THRESHOLD && (!y_snapped || d < (sy - y).abs()) {
                            sy = guide.position;
                            y_snapped = true;
                        }
                    }
                    GuideOrientation::Vertical => {
                        let d = (guide.position - x).abs();
                        if d <= THRESHOLD && (!x_snapped || d < (sx - x).abs()) {
                            sx = guide.position;
                            x_snapped = true;
                        }
                    }
                }
            }
        }

        // Distance of the whole current candidate; infinite while no axis
        // snapped, so whole-point targets are always considered first.
        let mut cur_dist = if x_snapped || y_snapped {
            ((sx - x).powi(2) + (sy - y).powi(2)).sqrt()
        } else {
            f64::INFINITY
        };
        let mut consider = |px: f64, py: f64| {
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d <= THRESHOLD && d < cur_dist {
                sx = px;
                sy = py;
                cur_dist = d;
            }
        };

        // Object edges / centers and anchor points share one pass over the
        // document (snap runs on every mouse move).
        if self.snap_to_objects || self.snap_to_points {
            let objects: Vec<&Object> = self
                .document
                .all_objects()
                .filter(|(_, o)| o.visible && !o.locked)
                .map(|(_, o)| o)
                .collect();

            if self.snap_to_objects {
                if let Some((ox, oy)) = self.snap_to_object_edges(x, y, &objects) {
                    consider(ox, oy);
                }
            }

            if self.snap_to_points {
                for obj in &objects {
                    // Only an object whose box reaches the point can hold an
                    // anchor this close — skips building the path for the
                    // rest of the document.
                    match obj.bounding_box() {
                        Some((bb_min, bb_max))
                            if x >= bb_min.x - THRESHOLD
                                && x <= bb_max.x + THRESHOLD
                                && y >= bb_min.y - THRESHOLD
                                && y <= bb_max.y + THRESHOLD => {}
                        _ => continue,
                    }
                    let path = obj.to_path_data();
                    for el in &path.elements {
                        let pts = match el {
                            crate::core::path::PathElement::MoveTo(a)
                            | crate::core::path::PathElement::LineTo(a) => [*a, *a],
                            crate::core::path::PathElement::CurveTo(seg) => [seg.start, seg.end],
                            crate::core::path::PathElement::ClosePath => continue,
                        };
                        for p in pts {
                            let (wx, wy) = obj.transform.transform_point(p.x, p.y);
                            consider(wx, wy);
                        }
                    }
                }
            }
        }

        if let Some(grid) = self.document.perspective.as_ref() {
            if grid.snap {
                let canvas = (0.0, 0.0, self.document.width, self.document.height);
                if let Some((px, py)) = grid.snap_point(x, y, canvas, THRESHOLD) {
                    consider(px, py);
                }
            }
        }

        if self.snap_to_pixels {
            sx = sx.round();
            sy = sy.round();
        }

        (sx, sy)
    }

    /// Snaps to the nearest object edge or center within a threshold (5px in world space).
    /// Returns `None` when nothing is close enough, so a miss cannot be told
    /// apart from an exact hit (and cannot cancel a stronger snap target).
    pub fn snap_to_object_edges(
        &self,
        x: f64,
        y: f64,
        objects: &[&crate::core::document::Object],
    ) -> Option<(f64, f64)> {
        let threshold = 5.0;
        let mut best: Option<(f64, f64, f64)> = None;

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
                    if dist < threshold && best.is_none_or(|(best_dist, _, _)| dist < best_dist) {
                        best = Some((dist, ex, ey));
                    }
                }
            }
        }

        best.map(|(_, ex, ey)| (ex, ey))
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
