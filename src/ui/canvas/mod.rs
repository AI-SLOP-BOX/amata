pub mod context_menu;
pub mod drag;
pub mod interaction;
pub mod overlays;
pub mod pixel;
pub mod rendering;
pub mod stroke_paint;

pub use rendering::sample_gradient_stops;

use crate::core::document::{Object, ObjectType};
use crate::core::path::AnchorPoint;
use crate::core::state::{AppState, HandleCorner, Tool};

use crate::tools::pen::PenState;
use crate::tools::pixel::PixelStroke;
use crate::tools::select::SelectState;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTarget {
    Anchor(usize),
    Control1(usize),
    Control2(usize),
}

impl NodeTarget {
    pub fn elem_idx(&self) -> usize {
        match self {
            NodeTarget::Anchor(i) | NodeTarget::Control1(i) | NodeTarget::Control2(i) => *i,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    Select,
    CreateRect,
    CreateEllipse,
    CreateStar,
    CreatePolygon,
    CreateLine,
    PencilDraw,
    BrushDraw,
    EraserDrag,
    /// Text tool drag: creates Illustrator-style area text on release.
    CreateTextArea,
    MoveObject,
    Resize(HandleCorner),
    Rotate,
    Pan,
    MoveNode(NodeTarget),
    /// Dragging the Text-on-Path start handle (arc-length slide).
    SlideTextOnPath,
    /// Dragging a live corner-radius widget on a selected rectangle.
    AdjustCorner(HandleCorner),
    DragGuideHorizontal,
    DragGuideVertical,
}

struct DragState {
    mode: DragMode,
    /// Target object for id-carrying drags (`SlideTextOnPath`).
    object_id: Option<String>,
    start_world: (f64, f64),
    current_world: (f64, f64),
    pencil_points: Vec<AnchorPoint>,
    initial_elements: Option<Vec<crate::core::path::PathElement>>,
}

impl DragState {
    fn new(mode: DragMode, wx: f64, wy: f64) -> Self {
        Self {
            mode,
            object_id: None,
            start_world: (wx, wy),
            current_world: (wx, wy),
            pencil_points: vec![AnchorPoint::new(wx, wy)],
            initial_elements: None,
        }
    }
}

const HANDLE_HIT_RADIUS: f32 = 12.0_f32;
const RULER_WIDTH: f32 = 20.0_f32;

pub struct CanvasWidget {
    pub pen_state: PenState,
    pub pixel_stroke: Option<PixelStroke>,
    pub select_state: SelectState,
    drag: Option<DragState>,
    node_edit_state: NodeEditState,
    image_textures: std::collections::HashMap<String, egui::TextureHandle>,
    pixel_textures: std::collections::HashMap<String, (egui::TextureHandle, u64)>,
    /// Baked text outlines: object id → (shape key, per-line meshes).
    /// `None` meshes mean "no real face" (legacy egui-font path draws).
    text_meshes:
        std::collections::HashMap<String, (u64, Option<rendering::CachedText>)>,
}

struct NodeEditState {
    selected_target: Option<NodeTarget>,
    selected_object_id: Option<String>,
}

impl NodeEditState {
    fn new() -> Self {
        Self {
            selected_target: None,
            selected_object_id: None,
        }
    }

    #[allow(dead_code)]
    fn selected_anchor_idx(&self) -> Option<usize> {
        self.selected_target.map(|t| t.elem_idx())
    }
}

impl Default for CanvasWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasWidget {
    pub fn new() -> Self {
        Self {
            pen_state: PenState::new(),
            pixel_stroke: None,
            select_state: SelectState::new(),
            drag: None,
            node_edit_state: NodeEditState::new(),
            image_textures: std::collections::HashMap::new(),
            pixel_textures: std::collections::HashMap::new(),
            text_meshes: std::collections::HashMap::new(),
        }
    }

    /// Decode a placed-image object into an egui texture.
    fn decode_image_texture(ctx: &egui::Context, obj: &Object) -> Option<egui::TextureHandle> {
        let png_bytes = match &obj.object_type {
            ObjectType::Image { png_bytes, .. } => png_bytes,
            _ => return None,
        };
        let img = image::load_from_memory(png_bytes).ok()?.to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        if w == 0 || h == 0 || w * h > 16_777_216 {
            return None;
        }
        let pixels = img
            .pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        Some(ctx.load_texture(
            &obj.id,
            egui::ColorImage {
                size: [w, h],
                pixels,
            },
            egui::TextureOptions::LINEAR,
        ))
    }

    fn ensure_image_textures(&mut self, ctx: &egui::Context, state: &AppState) {
        use std::collections::HashSet;
        let mut live: HashSet<String> = HashSet::new();
        // Top-level images plus group children.
        let mut stack: Vec<&crate::core::document::Object> = state
            .document
            .layers
            .iter()
            .flat_map(|l| l.objects.iter())
            .collect();
        while let Some(obj) = stack.pop() {
            match &obj.object_type {
                crate::core::document::ObjectType::Image { .. } => {
                    live.insert(obj.id.clone());
                    if !self.image_textures.contains_key(&obj.id) {
                        if let Some(tex) = Self::decode_image_texture(ctx, obj) {
                            self.image_textures.insert(obj.id.clone(), tex);
                        }
                    }
                }
                crate::core::document::ObjectType::Group(children)
                | crate::core::document::ObjectType::ClippingMask { children } => {
                    stack.extend(children.iter());
                }
                _ => {}
            }
        }
        self.image_textures.retain(|id, _| live.contains(id));
    }

    /// Decode a pixel-art object into an egui texture (NEAREST so dots stay
    /// crisp). Re-uploads only when the grid checksum changed, so drawing
    /// strokes don't pay per-frame upload costs.
    fn ensure_pixel_textures(&mut self, ctx: &egui::Context, state: &AppState) {
        use std::collections::HashSet;
        let mut live: HashSet<String> = HashSet::new();
        let mut stack: Vec<&crate::core::document::Object> = state
            .document
            .layers
            .iter()
            .flat_map(|l| l.objects.iter())
            .collect();
        while let Some(obj) = stack.pop() {
            match &obj.object_type {
                crate::core::document::ObjectType::PixelArt(p) => {
                    live.insert(obj.id.clone());
                    let sum = p.checksum();
                    let stale = self
                        .pixel_textures
                        .get(&obj.id)
                        .map(|(_, s)| *s != sum)
                        .unwrap_or(true);
                    if stale {
                        let raw = p.to_rgba8();
                        let (w, h) = (p.width as usize, p.height as usize);
                        if w > 0 && h > 0 && raw.len() == w * h * 4 {
                            let (chunks, _) = raw.as_chunks::<4>();
                            let pixels = chunks
                                .iter()
                                .map(|px| {
                                    egui::Color32::from_rgba_unmultiplied(
                                        px[0], px[1], px[2], px[3],
                                    )
                                })
                                .collect();
                            let tex = ctx.load_texture(
                                format!("{}#{}", obj.id, sum),
                                egui::ColorImage {
                                    size: [w, h],
                                    pixels,
                                },
                                egui::TextureOptions::NEAREST,
                            );
                            self.pixel_textures.insert(obj.id.clone(), (tex, sum));
                        }
                    }
                }
                crate::core::document::ObjectType::Group(children)
                | crate::core::document::ObjectType::ClippingMask { children } => {
                    stack.extend(children.iter());
                }
                _ => {}
            }
        }
        self.pixel_textures.retain(|id, _| live.contains(id));
    }

    /// Bake text outlines into local-space triangle meshes (real typeface
    /// on canvas, including kerning and faux-italic). Re-bakes only when
    /// the shape key changes; per-frame drawing just transforms the cached
    /// vertices. Objects without a resolvable face cache `None` and keep
    /// the legacy egui-font path.
    fn ensure_text_meshes(&mut self, state: &AppState) {
        use std::collections::HashSet;
        let mut live: HashSet<String> = HashSet::new();
        let mut stack: Vec<&crate::core::document::Object> = state
            .document
            .layers
            .iter()
            .flat_map(|l| l.objects.iter())
            .collect();
        while let Some(obj) = stack.pop() {
            match &obj.object_type {
                crate::core::document::ObjectType::Text {
                    text, style, area, ..
                } => {
                    live.insert(obj.id.clone());
                    let key = rendering::text_shape_key(text, style, *area);
                    let stale = self
                        .text_meshes
                        .get(&obj.id)
                        .map(|(k, _)| *k != key)
                        .unwrap_or(true);
                    if !stale {
                        continue;
                    }
                    let layout = crate::core::document::layout_text(text, style, *area);
                    let mut real = true;
                    let mut lines = Vec::new();
                    for line in rendering::text_draw_lines(text, style, *area) {
                        match crate::core::text_path::try_text_to_outline_path_with_style(
                            &line, style,
                        ) {
                            Some(ol) => {
                                let width = ol
                                    .bounding_box()
                                    .map(|(mn, mx)| mx.x - mn.x)
                                    .unwrap_or(0.0);
                                // Anchor shift in local coords, baked once:
                                // point text anchors on the origin, area
                                // text on the box edges/center.
                                let ox = layout.origin.0;
                                let x_off = match style.text_anchor {
                                    crate::core::document::TextAnchor::Start => 0.0,
                                    crate::core::document::TextAnchor::Middle => {
                                        area.map(|a| a.x + a.width / 2.0).unwrap_or(ox)
                                            - (ox + width / 2.0)
                                    }
                                    crate::core::document::TextAnchor::End => {
                                        area.map(|a| a.x + a.width).unwrap_or(ox)
                                            - (ox + width)
                                    }
                                };
                                lines.push(rendering::CachedTextLine {
                                    tris: ol.to_triangles(12),
                                    width,
                                    x_off,
                                });
                            }
                            None => {
                                real = false;
                                break;
                            }
                        }
                    }
                    let cached = real.then_some(rendering::CachedText {
                        lines,
                        origin: layout.origin,
                        visible: layout.visible,
                    });
                    self.text_meshes.insert(obj.id.clone(), (key, cached));
                }
                crate::core::document::ObjectType::Group(children)
                | crate::core::document::ObjectType::ClippingMask { children } => {
                    stack.extend(children.iter());
                }
                _ => {}
            }
        }
        self.text_meshes.retain(|id, _| live.contains(id));
    }

    /// Place raster bytes on the canvas centred at world `(cx, cy)`.
    pub fn place_image_bytes(
        &mut self,
        state: &mut AppState,
        bytes: &[u8],
        name: &str,
        cx: f64,
        cy: f64,
    ) {
        match crate::io::raster::decode_placed_image(bytes) {
            Err(e) => state.notify_error(format!("画像の配置に失敗しました: {e}")),
            Ok((w, h, png)) => {
                let obj = Object::new_image(name, cx - w / 2.0, cy - h / 2.0, w, h, png);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
                state.notify_success("画像を配置しました");
            }
        }
    }

    fn place_dropped_images(
        &mut self,
        state: &mut AppState,
        _origin: Pos2,
        dropped: &[egui::DroppedFile],
    ) {
        // View-centre world coordinates for the drop target.
        let zoom = state.zoom as f64;
        let (cx, cy) = (
            -state.pan_x as f64 / zoom,
            -state.pan_y as f64 / zoom,
        );
        for f in dropped {
            let ext = f
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let is_raster = matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "bmp"
            ) || f.bytes.is_some()
                && ext.is_empty();
            if !is_raster {
                continue;
            }
            let name = f
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if f.name.is_empty() {
                        "Image".to_string()
                    } else {
                        f.name.clone()
                    }
                });
            let bytes: Option<Vec<u8>> = f
                .path
                .as_ref()
                .and_then(|p| std::fs::read(p).ok())
                .or_else(|| f.bytes.as_ref().map(|b| b.to_vec()));
            if let Some(bytes) = bytes {
                self.place_image_bytes(state, &bytes, &name, cx, cy);
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
    ) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height()),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        state.canvas_center_x = rect.center().x;
        state.canvas_center_y = rect.center().y;
        let origin = Pos2::new(rect.center().x + state.pan_x, rect.center().y + state.pan_y);

        state.canvas_width = rect.width();
        state.canvas_height = rect.height();

        // Dark Pasteboard Canvas Background (#1e1e1e)
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(30, 30, 30));

        // Grid
        if state.show_grid {
            self.draw_grid(&painter, rect, origin, state);
            self.draw_perspective(&painter, rect, origin, state);
        }

        // Artboard(s): draw all artboards owned by the document.
        // The active artboard (or the only one) gets a header label.
        let artboards = state.document.effective_artboards();
        for ab in artboards.iter() {
            let ab_rect = Rect::from_min_size(
                Pos2::new(
                    origin.x + ab.x as f32 * state.zoom,
                    origin.y + ab.y as f32 * state.zoom,
                ),
                Vec2::new(
                    ab.width as f32 * state.zoom,
                    ab.height as f32 * state.zoom,
                ),
            );
            // Soft outer diffuse shadow
            let shadow_rect1 = ab_rect.translate(Vec2::new(4.0, 4.0));
            painter.rect_filled(shadow_rect1, 0.0_f32, Color32::from_black_alpha(35));
            let shadow_rect2 = ab_rect.translate(Vec2::new(2.0, 2.0));
            painter.rect_filled(shadow_rect2, 0.0_f32, Color32::from_black_alpha(70));
            let shadow_rect3 = ab_rect.translate(Vec2::new(1.0, 1.0));
            painter.rect_filled(shadow_rect3, 0.0_f32, Color32::from_black_alpha(90));

            // Artboard background: opaque white or the transparency
            // checkerboard (`artboard_bg_mode`).
            if state.prefs.artboard_is_white() {
                painter.rect_filled(ab_rect, 0.0_f32, Color32::WHITE);
            } else {
                // Transparency checkerboard (6px squares at base zoom)
                let check_size = 6.0 * state.zoom;
                if check_size >= 2.0 {
                    let cols = (ab_rect.width() / check_size).ceil() as i32;
                    let rows = (ab_rect.height() / check_size).ceil() as i32;
                    let c_light = Color32::from_rgb(240, 240, 240);
                    let c_dark = Color32::from_rgb(204, 204, 204);
                    for row in 0..rows {
                        for col in 0..cols {
                            let x = ab_rect.min.x + col as f32 * check_size;
                            let y = ab_rect.min.y + row as f32 * check_size;
                            let r = Rect::from_min_size(
                                Pos2::new(x, y),
                                Vec2::new(check_size + 0.5, check_size + 0.5),
                            ).intersect(ab_rect);
                            let c = if (row + col) % 2 == 0 { c_light } else { c_dark };
                            painter.rect_filled(r, 0.0, c);
                        }
                    }
                } else {
                    // Too zoomed out: just fill white
                    painter.rect_filled(ab_rect, 0.0_f32, Color32::WHITE);
                }
            }

            if state.prefs.show_boundary_lines {
                painter.rect_stroke(
                    ab_rect,
                    0.0_f32,
                    Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60)),
                    StrokeKind::Outside,
                );
            }

            // Artboard Header Tab Label (the size part is the dimension label)
            let tab_pos = Pos2::new(ab_rect.min.x, ab_rect.min.y - 18.0);
            let label = if state.prefs.show_dimension_labels {
                let unit = state.prefs.ruler_unit;
                format!(
                    "{} ({} × {} {})",
                    ab.name,
                    unit.format(ab.width),
                    unit.format(ab.height),
                    unit.suffix()
                )
            } else {
                ab.name.clone()
            };
            painter.text(
                tab_pos,
                egui::Align2::LEFT_TOP,
                label,
                FontId::proportional(11.0),
                Color32::from_rgb(170, 170, 170),
            );
        }

        // Compute the active artboard rect for isolation overlay / smart guides
        let active_ab = artboards.get(state.active_artboard_idx).cloned().unwrap_or_else(|| {
            crate::core::document::Artboard::new("Artboard 1", 0.0, 0.0, state.document.width, state.document.height)
        });
        let artboard = Rect::from_min_size(
            Pos2::new(
                origin.x + active_ab.x as f32 * state.zoom,
                origin.y + active_ab.y as f32 * state.zoom,
            ),
            Vec2::new(
                active_ab.width as f32 * state.zoom,
                active_ab.height as f32 * state.zoom,
            ),
        );

        // Decode placed images ahead of drawing (texture cache).
        self.ensure_image_textures(ui.ctx(), state);
        self.ensure_pixel_textures(ui.ctx(), state);
        self.ensure_text_meshes(state);

        // Handle files dropped onto the canvas: raster images are placed,
        // documents are ignored here (use File > Open).
        let dropped: Vec<egui::DroppedFile> =
            ui.ctx().input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            self.place_dropped_images(state, origin, &dropped);
        }

        // Render Objects (CPU fallback via egui painter).
        // Viewport culling above a size threshold: egui already clips
        // rasterization, but shape construction/tessellation dominates, so
        // fully offscreen objects are skipped via their world bbox.
        let total_objs = state.document.all_objects().count();
        let cull_enabled = total_objs > 200;
        let (vw0, vh0, vw1, vh1) = (
            ((rect.min.x - origin.x) / state.zoom) as f64 - 50.0,
            ((rect.min.y - origin.y) / state.zoom) as f64 - 50.0,
            ((rect.max.x - origin.x) / state.zoom) as f64 + 50.0,
            ((rect.max.y - origin.y) / state.zoom) as f64 + 50.0,
        );
        for (layer_idx, obj) in state.document.all_objects() {
            if !obj.visible {
                continue;
            }
            // Hidden layers must not render (opacity was honored before,
            // visibility was not).
            if state
                .document
                .layers
                .get(layer_idx)
                .is_some_and(|l| !l.visible)
            {
                continue;
            }
            if cull_enabled {
                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                    if bb_max.x < vw0 || bb_min.x > vw1 || bb_max.y < vh0 || bb_min.y > vh1 {
                        continue;
                    }
                }
            }
            let layer_op = state
                .document
                .layers
                .get(layer_idx)
                .map(|l| l.opacity)
                .unwrap_or(1.0);
            self.draw_object(
                &painter,
                obj,
                origin,
                state,
                &crate::ui::canvas::rendering::IDENTITY_AFFINE,
                layer_op,
            );
        }

        // Isolation overlay: dim everything, then redraw the isolated
        // group on top so its children stay fully editable.
        if let Some(group) = state.isolated_group() {
            let gid = group.id.clone();
            let layer_op = state
                .document
                .layers
                .iter()
                .find(|l| l.objects.iter().any(|o| o.id == gid))
                .map(|l| l.opacity)
                .unwrap_or(1.0);
            painter.rect_filled(artboard, 0.0_f32, Color32::from_black_alpha(110));
            self.draw_object(
                &painter,
                group,
                origin,
                state,
                &crate::ui::canvas::rendering::IDENTITY_AFFINE,
                layer_op,
            );
        }

        // Smart Guides
        if state.show_smart_guides {
            self.draw_smart_guides(&painter, artboard, rect, origin, state);
        }

        // Drag Previews
        if let Some(ref drag) = self.drag {
            match drag.mode {
                DragMode::CreateRect => self.draw_rect_preview(&painter, origin, state, drag),
                // Text area box preview reuses the rect rubber-band.
                DragMode::CreateTextArea => self.draw_rect_preview(&painter, origin, state, drag),
                DragMode::CreateEllipse => self.draw_ellipse_preview(&painter, origin, state, drag),
                DragMode::CreateStar => self.draw_star_preview(&painter, origin, state, drag),
                DragMode::CreatePolygon => self.draw_polygon_preview(&painter, origin, state, drag),
                DragMode::CreateLine => self.draw_line_preview(&painter, origin, state, drag),
                DragMode::PencilDraw => self.draw_pencil_preview(&painter, origin, state, drag),
                DragMode::BrushDraw => self.draw_brush_preview(&painter, origin, state, drag),
                DragMode::EraserDrag => self.draw_eraser_preview(&painter, origin, state, drag),
                DragMode::Select => self.draw_marquee(&painter, origin, state, drag),
                _ => {}
            }
        }

        // Pen Preview (show even before first anchor when hovering)
        if state.current_tool == Tool::Pen {
            self.draw_pen_preview(&painter, origin, state);
        }

        // User cyan guidelines
        self.draw_user_guides(&painter, rect, origin, state);

        // Pixel-art cell grid + hover cell (dot絵)
        self.draw_pixel_grid(&painter, origin, state);

        // Active dragged guide preview
        if let Some(ref drag) = self.drag {
            match drag.mode {
                DragMode::DragGuideHorizontal => {
                    let sy = origin.y + (drag.current_world.1 as f32 * state.zoom);
                    painter.line_segment(
                        [Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)],
                        Stroke::new(1.0_f32, Color32::from_rgb(0, 220, 255)),
                    );
                }
                DragMode::DragGuideVertical => {
                    let sx = origin.x + (drag.current_world.0 as f32 * state.zoom);
                    painter.line_segment(
                        [Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)],
                        Stroke::new(1.0_f32, Color32::from_rgb(0, 220, 255)),
                    );
                }
                _ => {}
            }
        }

        // Selection Bounding Box & Handles
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                self.draw_selection(&painter, obj, origin, state);
            }
        }

        // Text-on-Path slide handle (single selection): circle at the arc start.
        if state.current_tool == Tool::Select && state.selected_ids.len() == 1 {
            let top_id = state.selected_ids[0].clone();
            let handle = state.document.find_object(&top_id).and_then(|obj| {
                if let ObjectType::TextOnPath {
                    path: base_path,
                    start_offset,
                    ..
                } = &obj.object_type
                {
                    let lp = crate::core::text_path::arc_point_at(base_path, *start_offset)?;
                    let m = obj.transform.matrix();
                    let hx = m[0] * lp.x + m[2] * lp.y + m[4];
                    let hy = m[1] * lp.x + m[3] * lp.y + m[5];
                    let (sx, sy) = state.world_to_screen(hx, hy);
                    Some(Pos2::new(sx, sy))
                } else {
                    None
                }
            });
            if let Some(sp) = handle {
                painter.circle_filled(sp, 6.0, Color32::from_rgb(20, 115, 230));
                painter.circle_stroke(sp, 6.0, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }

        // Visual Diff Highlights on Canvas
        if state.is_comparing_diff {
            if let Some(ref diff) = state.active_diff {
                self.draw_diff_overlays(&painter, diff, origin, state);
            }
        }

        // Node Editing Handles
        if state.current_tool == Tool::Node {
            self.draw_node_edit(&painter, origin, state);
        }

        // Smart Guides (Magenta alignment overlays)
        self.draw_smart_guides(&painter, artboard, rect, origin, state);

        // Figma-style measurements while Alt is held (must read input here:
        // the later alt_down at the interaction phase comes after drawing).
        let alt_down = ui.input(|i| i.modifiers.alt);
        self.draw_measurements(&painter, rect, origin, state, alt_down);

        // Layout grid (columns/rows/cell) on artboards that have one.
        self.draw_layout_grid(&painter, origin, state);

        // Rulers (drawn above artboard and guidelines)
        if state.show_rulers {
            self.draw_rulers(&painter, rect, origin, state);
        }

        // Smooth Zoom Animation (lerp toward target)
        if state.zoom_animation_progress < 1.0 {
            state.zoom_animation_progress = (state.zoom_animation_progress + 0.15_f32).min(1.0);
            let t = smooth_step(state.zoom_animation_progress);
            state.zoom = state.start_zoom + (state.target_zoom - state.start_zoom) * t;
            state.pan_x = state.start_pan_x + (state.target_pan_x - state.start_pan_x) * t;
            state.pan_y = state.start_pan_y + (state.target_pan_y - state.start_pan_y) * t;
            ui.ctx().request_repaint();
        } else {
            state.zoom = state.target_zoom;
            state.pan_x = state.target_pan_x;
            state.pan_y = state.target_pan_y;
        }

        // Mouse Wheel Zoom (Ctrl+scroll or trackpad pinch)
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0_f32 {
            let old_zoom = state.target_zoom;
            state.target_zoom =
                (state.target_zoom * (1.0_f32 + scroll * 0.001_f32)).clamp(0.01_f32, 100.0_f32);
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let zoom_ratio = state.target_zoom / old_zoom;
                let dx = (screen_pos.x - origin.x) * (zoom_ratio - 1.0_f32);
                let dy = (screen_pos.y - origin.y) * (zoom_ratio - 1.0_f32);
                state.target_pan_x -= dx;
                state.target_pan_y -= dy;
            }
            state.start_zoom = state.zoom;
            state.start_pan_x = state.pan_x;
            state.start_pan_y = state.pan_y;
            state.zoom_animation_progress = 0.0;
        }

        // Right-Click Context Menu (Illustrator style)
        self.show_context_menu(ui, state, &response, origin);
        // Track pointer position in world coordinates for status bar & tools
        if let Some(hover_pos) = response.hover_pos() {
            let (wx, wy) = state.screen_to_world(hover_pos.x, hover_pos.y);
            state.cursor_world = Some((wx, wy));
        } else {
            state.cursor_world = None;
        }

        let space_down = ui.input(|i| i.key_down(egui::Key::Space));
        let shift_down = ui.input(|i| i.modifiers.shift);
        let alt_down = ui.input(|i| i.modifiers.alt);

        if (response.secondary_clicked() || (space_down && response.clicked()))
            && !response.dragged()
        {
            self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
        }

        // Handle Pointer Interaction
        if let Some(screen_pos) = response.interact_pointer_pos() {
            let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
            state.cursor_world = Some((wx, wy));

            let mut cursor = egui::CursorIcon::Default;
            if space_down {
                cursor = if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                };
            } else if state.current_tool == Tool::Select {
                let mut corner_widget = false;
                for id in &state.selected_ids {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id)
                    {
                        if let Some(corner) =
                            self.hit_test_corner_widgets(obj, screen_pos, origin, state)
                        {
                            cursor = match corner {
                                HandleCorner::TopRight | HandleCorner::BottomLeft => {
                                    egui::CursorIcon::ResizeNwSe
                                }
                                _ => egui::CursorIcon::ResizeNeSw,
                            };
                            corner_widget = true;
                            break;
                        }
                        if let Some(corner) = self.hit_test_handles(obj, screen_pos, origin, state)
                        {
                            cursor = match corner {
                                HandleCorner::TopLeft | HandleCorner::BottomRight => {
                                    egui::CursorIcon::ResizeNeSw
                                }
                                HandleCorner::TopRight | HandleCorner::BottomLeft => {
                                    egui::CursorIcon::ResizeNwSe
                                }
                                HandleCorner::Top | HandleCorner::Bottom => {
                                    egui::CursorIcon::ResizeVertical
                                }
                                HandleCorner::Left | HandleCorner::Right => {
                                    egui::CursorIcon::ResizeHorizontal
                                }
                                HandleCorner::RotateTopRight => egui::CursorIcon::Crosshair,
                            };
                            break;
                        }
                    }
                }
                if !corner_widget
                    && cursor == egui::CursorIcon::Default
                    && self.select_state.hit_test(state, wx, wy).is_some()
                {
                    cursor = if alt_down {
                        egui::CursorIcon::Copy
                    } else {
                        egui::CursorIcon::Move
                    };
                }
            } else if state.current_tool == Tool::Hand {
                cursor = if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                };
            } else if state.current_tool == Tool::Zoom {
                cursor = if shift_down || alt_down {
                    egui::CursorIcon::ZoomOut
                } else {
                    egui::CursorIcon::ZoomIn
                };
            } else if matches!(
                state.current_tool,
                Tool::Eyedropper
                    | Tool::Brush
                    | Tool::Eraser
                    | Tool::Pen
                    | Tool::PixelPencil
                    | Tool::PixelEraser
                    | Tool::PixelBucket
            ) {
                cursor = egui::CursorIcon::Crosshair;
            } else if state.current_tool == Tool::Text {
                cursor = egui::CursorIcon::Text;
            } else if state.current_tool == Tool::Node {
                if self.hit_test_nodes(state, screen_pos, origin).is_some() {
                    cursor = egui::CursorIcon::Move;
                } else {
                    cursor = egui::CursorIcon::Crosshair;
                }
            } else {
                cursor = egui::CursorIcon::Crosshair;
            }
            painter.ctx().set_cursor_icon(cursor);

            if state.current_tool == Tool::Pen && self.pen_state.is_drawing {
                self.pen_state.update_hover(wx, wy);
            }

            if response.clicked() && !space_down {
                let shift = painter.ctx().input(|i| i.modifiers.shift);
                self.handle_click(state, wx, wy, screen_pos, origin, shift);
            }
            // Double-click a top-level group enters isolation editing
            // (Illustrator-style); double-click empty space exits.
            if response.double_clicked()
                && !space_down
                && state.current_tool == Tool::Select
            {
                if state.isolated_group_id.is_none() {
                    if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                        let is_group = state
                            .document
                            .all_objects()
                            .find(|(_, o)| o.id == id)
                            .map(|(_, o)| {
                                matches!(
                                    o.object_type,
                                    crate::core::document::ObjectType::Group(_)
                                )
                            })
                            .unwrap_or(false);
                        if is_group {
                            state.isolated_group_id = Some(id);
                            state.selected_ids.clear();
                        }
                    }
                } else if self.select_state.hit_test(state, wx, wy).is_none() {
                    state.exit_isolation();
                }
            }
            // Isolation breadcrumb + exit affordance.
            if let Some(group) = state.isolated_group() {
                let name = group.name.clone();
                egui::Area::new(egui::Id::new("isolation_breadcrumb"))
                    .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 30.0))
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("⧉ {name}"))
                                    .strong()
                                    .color(egui::Color32::from_rgb(100, 190, 255)),
                            );
                            if ui.small_button("‹ Exit (Esc)").clicked() {
                                state.exit_isolation();
                            }
                        });
                    });
            }
        }

        // Drag Start
        if response.drag_started() && self.drag.is_none() {
            if space_down {
                self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
            } else if let Some(screen_pos) = response.interact_pointer_pos() {
                let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                let (wx, wy) = state.snap(wx, wy);

                // Illustrator CC: Pulling guides from top or left rulers
                if state.show_rulers
                    && screen_pos.y <= rect.min.y + RULER_WIDTH
                    && screen_pos.x >= rect.min.x + RULER_WIDTH
                {
                    self.drag = Some(DragState::new(DragMode::DragGuideHorizontal, wx, wy));
                } else if state.show_rulers
                    && screen_pos.x <= rect.min.x + RULER_WIDTH
                    && screen_pos.y >= rect.min.y + RULER_WIDTH
                {
                    self.drag = Some(DragState::new(DragMode::DragGuideVertical, wx, wy));
                } else {
                    match state.current_tool {
                        Tool::Rectangle => {
                            self.drag = Some(DragState::new(DragMode::CreateRect, wx, wy));
                        }
                        Tool::Ellipse => {
                            self.drag = Some(DragState::new(DragMode::CreateEllipse, wx, wy));
                        }
                        Tool::Star => {
                            self.drag = Some(DragState::new(DragMode::CreateStar, wx, wy));
                        }
                        Tool::Polygon => {
                            self.drag = Some(DragState::new(DragMode::CreatePolygon, wx, wy));
                        }
                        Tool::Line => {
                            self.drag = Some(DragState::new(DragMode::CreateLine, wx, wy));
                        }
                        Tool::Pencil => {
                            self.drag = Some(DragState::new(DragMode::PencilDraw, wx, wy));
                        }
                        Tool::Brush => {
                            self.drag = Some(DragState::new(DragMode::BrushDraw, wx, wy));
                        }
                        Tool::PixelPencil | Tool::PixelEraser | Tool::PixelBucket => {
                            // Dots live on exact integer cells: never snap.
                            let (rx, ry) = state.screen_to_world(screen_pos.x, screen_pos.y);
                            match state.current_tool {
                                Tool::PixelBucket => self.pixel_bucket(state, rx, ry),
                                Tool::PixelPencil => {
                                    self.pixel_stroke_begin(state, rx, ry, false);
                                }
                                _ => {
                                    self.pixel_stroke_begin(state, rx, ry, true);
                                }
                            }
                        }
                        Tool::Eraser => {
                            self.drag = Some(DragState::new(DragMode::EraserDrag, wx, wy));
                        }
                        // Text tool: dragging out a box creates area text;
                        // a plain click (no drag) falls through to
                        // `handle_click`, which creates point text.
                        Tool::Text => {
                            self.drag = Some(DragState::new(DragMode::CreateTextArea, wx, wy));
                        }
                        Tool::Pen => {
                            // If drawing and drag starts, we're pulling bezier handles
                            if self.pen_state.is_drawing && self.pen_state.dragging_handle {
                                // dragging handle is already set, just let drag update handle
                            } else if !self.pen_state.is_drawing {
                                self.pen_state.start_path(wx, wy);
                            } else {
                                self.pen_state.add_point(wx, wy);
                            }
                        }
                        Tool::Select => {
                            let mut handled = false;
                            // Text-on-Path slide handle takes priority over the
                            // resize handles when it is under the cursor.
                            if state.selected_ids.len() == 1 {
                                let sid = state.selected_ids[0].clone();
                                let handle = state.document.find_object(&sid).and_then(|obj| {
                                    if let ObjectType::TextOnPath {
                                        path: base_path,
                                        start_offset,
                                        ..
                                    } = &obj.object_type
                                    {
                                        let lp = crate::core::text_path::arc_point_at(
                                            base_path,
                                            *start_offset,
                                        )?;
                                        let m = obj.transform.matrix();
                                        let hx = m[0] * lp.x + m[2] * lp.y + m[4];
                                        let hy = m[1] * lp.x + m[3] * lp.y + m[5];
                                        let (sx, sy) = state.world_to_screen(hx, hy);
                                        Some(Pos2::new(sx, sy))
                                    } else {
                                        None
                                    }
                                });
                                if let Some(sp) = handle {
                                    if screen_pos.distance(sp) <= HANDLE_HIT_RADIUS {
                                        state.ensure_object_snapshot(&sid);
                                        let mut d =
                                            DragState::new(DragMode::SlideTextOnPath, wx, wy);
                                        d.object_id = Some(sid);
                                        self.drag = Some(d);
                                        handled = true;
                                    }
                                }
                            }
                            // Live corner-radius widgets take priority over
                            // the resize handles (Rectangle only).
                            if !handled {
                                let selected = state.selected_ids.clone();
                                let mut hit_corner = None;
                                let mut hit_id = None;
                                for id in &selected {
                                    if let Some((_, obj)) = state
                                        .document
                                        .all_objects()
                                        .find(|(_, o)| &o.id == id)
                                    {
                                        if let Some(corner) = self.hit_test_corner_widgets(
                                            obj, screen_pos, origin, state,
                                        ) {
                                            hit_corner = Some(corner);
                                            hit_id = Some(id.clone());
                                            break;
                                        }
                                    }
                                }
                                if let (Some(corner), Some(id)) = (hit_corner, hit_id) {
                                    state.ensure_object_snapshot(&id);
                                    let mut d =
                                        DragState::new(DragMode::AdjustCorner(corner), wx, wy);
                                    d.object_id = Some(id);
                                    self.drag = Some(d);
                                    handled = true;
                                }
                            }
                            for id in &state.selected_ids {
                                if let Some((_, obj)) =
                                    state.document.all_objects().find(|(_, o)| &o.id == id)
                                {
                                    if let Some(corner) =
                                        self.hit_test_handles(obj, screen_pos, origin, state)
                                    {
                                        if corner == HandleCorner::RotateTopRight {
                                            self.drag =
                                                Some(DragState::new(DragMode::Rotate, wx, wy));
                                        } else {
                                            self.drag = Some(DragState::new(
                                                DragMode::Resize(corner),
                                                wx,
                                                wy,
                                            ));
                                        }
                                        handled = true;
                                        break;
                                    }
                                }
                            }
                            if !handled {
                                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                                    if !state.selected_ids.contains(&id) {
                                        state.selected_ids = vec![id];
                                    }
                                    self.select_state.start_drag(state, wx, wy);
                                    self.drag = Some(DragState::new(DragMode::MoveObject, wx, wy));
                                } else {
                                    state.selected_ids.clear();
                                    self.drag = Some(DragState::new(DragMode::Select, wx, wy));
                                }
                            }
                        }
                        Tool::Node => {
                            if let Some((target, obj_id)) =
                                self.hit_test_nodes(state, screen_pos, origin)
                            {
                                if !state.selected_ids.contains(&obj_id) {
                                    state.selected_ids = vec![obj_id.clone()];
                                }
                                self.node_edit_state.selected_target = Some(target);
                                self.node_edit_state.selected_object_id = Some(obj_id.clone());

                                // Alt+click on an anchor deletes it (one undo).
                                let delete_idx = if alt_down {
                                    match target {
                                        NodeTarget::Anchor(idx) => Some(idx),
                                        _ => None,
                                    }
                                } else {
                                    None
                                };
                                if let Some(idx) = delete_idx {
                                    self.delete_anchor_at(state, &obj_id, idx);
                                    self.node_edit_state.selected_target = None;
                                } else {
                                    // Shape-to-path conversion is destructive:
                                    // snapshot first so one Undo restores the
                                    // original shape.
                                    let needs_convert = state
                                        .document
                                        .find_object(&obj_id)
                                        .map(|o| {
                                            !matches!(
                                                o.object_type,
                                                crate::core::document::ObjectType::Path(_)
                                            )
                                        })
                                        .unwrap_or(false);
                                    if needs_convert {
                                        state.ensure_object_snapshot(&obj_id);
                                    }
                                    let initial_elements = if let Some(obj) =
                                        state.document.find_object_mut(&obj_id)
                                    {
                                        if needs_convert {
                                            let p = obj.to_path_data();
                                            obj.object_type =
                                                crate::core::document::ObjectType::Path(p);
                                        }
                                        if let crate::core::document::ObjectType::Path(ref p) =
                                            obj.object_type
                                        {
                                            Some(p.elements.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    if needs_convert {
                                        state.commit_object_edits("Convert to Path");
                                    }

                                    let mut d =
                                        DragState::new(DragMode::MoveNode(target), wx, wy);
                                    d.initial_elements = initial_elements;
                                    self.drag = Some(d);
                                }
                            } else if let Some(obj_id) =
                                self.insert_anchor_near(state, screen_pos, origin)
                            {
                                // Click on a segment: add an anchor there.
                                if !state.selected_ids.contains(&obj_id) {
                                    state.selected_ids = vec![obj_id.clone()];
                                }
                                self.node_edit_state.selected_object_id = Some(obj_id);
                                self.node_edit_state.selected_target = None;
                            } else if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                                state.selected_ids = vec![id.clone()];
                                self.node_edit_state.selected_object_id = Some(id);
                                self.node_edit_state.selected_target = None;
                            }
                        }
                        Tool::Hand => {
                            self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Dragging
        if response.dragged() {
            // Pen tool handle dragging (no DragState needed)
            if state.current_tool == Tool::Pen && self.pen_state.dragging_handle {
                if let Some(screen_pos) = response.interact_pointer_pos() {
                    let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                    self.pen_state.drag_handle(wx, wy);
                }
            }

            // Active pixel stroke follows the raw pointer (never snapped).
            if self.pixel_stroke.is_some() {
                if let Some(screen_pos) = response.interact_pointer_pos() {
                    let (rx, ry) = state.screen_to_world(screen_pos.x, screen_pos.y);
                    self.pixel_stroke_extend(state, rx, ry);
                }
            }

            if let Some(ref mut drag) = self.drag {
                let delta = response.drag_delta();
                if drag.mode == DragMode::Pan {
                    state.pan_x += delta.x;
                    state.pan_y += delta.y;
                } else if let Some(screen_pos) = response.interact_pointer_pos() {
                    let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                    let (wx, wy) = state.snap(wx, wy);
                    drag.current_world = (wx, wy);

                    if matches!(
                        drag.mode,
                        DragMode::PencilDraw | DragMode::BrushDraw | DragMode::EraserDrag
                    ) {
                        drag.pencil_points.push(AnchorPoint::new(wx, wy));
                    } else if drag.mode == DragMode::MoveObject {
                        self.select_state.update_drag(state, wx, wy);
                    } else if drag.mode == DragMode::Rotate {
                        self.update_rotate(state, wx, wy, shift_down);
                    } else if let DragMode::MoveNode(target) = drag.mode {
                        self.move_node(state, target, wx, wy);
                    } else if let DragMode::Resize(corner) = drag.mode {
                        self.update_resize(state, corner, wx, wy);
                    } else if drag.mode == DragMode::SlideTextOnPath {
                        if let Some(obj_id) = drag.object_id.clone() {
                            let projected =
                                state.document.find_object(&obj_id).and_then(|obj| {
                                    if let ObjectType::TextOnPath { path: bp, .. } =
                                        &obj.object_type
                                    {
                                        let (lx, ly) =
                                            obj.transform.inverse_transform_point(wx, wy);
                                        crate::core::text_path::project_to_arc_length(bp, lx, ly)
                                    } else {
                                        None
                                    }
                                });
                            if let Some(new_off) = projected {
                                if let Some(o) = state.document.find_object_mut(&obj_id) {
                                    if let ObjectType::TextOnPath {
                                        path,
                                        start_offset,
                                        ..
                                    } = &mut o.object_type
                                    {
                                        let total =
                                            crate::core::text_path::path_total_length(path);
                                        *start_offset = new_off.clamp(0.0, total.max(0.0));
                                    }
                                }
                            }
                        }
                    } else if let DragMode::AdjustCorner(corner) = drag.mode {
                        if let Some(obj_id) = drag.object_id.clone() {
                            self.update_corner_radius(state, &obj_id, corner, wx, wy);
                        }
                    }
                }
            }
        }

        // Drag Stopped / Completed
        if response.drag_stopped() {
            self.handle_drag_stopped(state, shift_down, alt_down);
        }
    }
}

fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
