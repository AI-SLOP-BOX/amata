pub mod context_menu;
pub mod drag;
pub mod interaction;
pub mod overlays;
pub mod rendering;

pub use rendering::sample_gradient_stops;

use crate::core::path::AnchorPoint;
use crate::core::state::{AppState, HandleCorner, Tool};
use crate::gpu::GpuRenderer;
use crate::tools::pen::PenState;
use crate::tools::select::SelectState;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::sync::Arc;

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
    MoveObject,
    Resize(HandleCorner),
    Rotate,
    Pan,
    MoveNode(NodeTarget),
    DragGuideHorizontal,
    DragGuideVertical,
}

struct DragState {
    mode: DragMode,
    start_world: (f64, f64),
    current_world: (f64, f64),
    pencil_points: Vec<AnchorPoint>,
    initial_elements: Option<Vec<crate::core::path::PathElement>>,
}

impl DragState {
    fn new(mode: DragMode, wx: f64, wy: f64) -> Self {
        Self {
            mode,
            start_world: (wx, wy),
            current_world: (wx, wy),
            pencil_points: vec![AnchorPoint::new(wx, wy)],
            initial_elements: None,
        }
    }
}

const HANDLE_SIZE: f32 = 8.0_f32;
const HANDLE_HIT_RADIUS: f32 = 12.0_f32;
const RULER_WIDTH: f32 = 20.0_f32;

pub struct CanvasWidget {
    pub pen_state: PenState,
    pub select_state: SelectState,
    drag: Option<DragState>,
    node_edit_state: NodeEditState,
    pub gpu_renderer: Option<GpuRenderer>,
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
            select_state: SelectState::new(),
            drag: None,
            node_edit_state: NodeEditState::new(),
            gpu_renderer: None,
        }
    }

    /// Lazily initialize the GPU renderer from eframe's wgpu context
    pub fn ensure_gpu_renderer(&mut self, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) {
        if self.gpu_renderer.is_some() {
            return;
        }
        self.gpu_renderer = Some(GpuRenderer::new(device, queue));
        log::info!("GPU renderer initialized via eframe wgpu backend");
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        device: Option<Arc<wgpu::Device>>,
        queue: Option<Arc<wgpu::Queue>>,
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
        }

        // Artboard Dimensions & Realistic Multi-Tier Soft Drop Shadow (Illustrator CC signature canvas)
        let artboard = Rect::from_min_size(
            origin,
            Vec2::new(
                state.document.width as f32 * state.zoom,
                state.document.height as f32 * state.zoom,
            ),
        );
        // Soft outer diffuse shadow
        let shadow_rect1 = artboard.translate(Vec2::new(4.0, 4.0));
        painter.rect_filled(shadow_rect1, 0.0_f32, Color32::from_black_alpha(35));
        let shadow_rect2 = artboard.translate(Vec2::new(2.0, 2.0));
        painter.rect_filled(shadow_rect2, 0.0_f32, Color32::from_black_alpha(70));
        let shadow_rect3 = artboard.translate(Vec2::new(1.0, 1.0));
        painter.rect_filled(shadow_rect3, 0.0_f32, Color32::from_black_alpha(90));

        // Crisp White Artboard Paper
        painter.rect_filled(artboard, 0.0_f32, Color32::WHITE);
        painter.rect_stroke(
            artboard,
            0.0_f32,
            Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60)),
            StrokeKind::Outside,
        );

        // Artboard Header Tab Label
        let tab_pos = Pos2::new(artboard.min.x, artboard.min.y - 18.0);
        painter.text(
            tab_pos,
            egui::Align2::LEFT_TOP,
            format!(
                "Artboard 1 ({} × {} px)",
                state.document.width as i32, state.document.height as i32
            ),
            FontId::proportional(11.0),
            Color32::from_rgb(170, 170, 170),
        );

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
        for (_, obj) in state.document.all_objects() {
            if !obj.visible {
                continue;
            }
            if cull_enabled {
                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                    if bb_max.x < vw0 || bb_min.x > vw1 || bb_max.y < vh0 || bb_min.y > vh1 {
                        continue;
                    }
                }
            }
            self.draw_object(
                &painter,
                obj,
                origin,
                state,
                &crate::ui::canvas::rendering::IDENTITY_AFFINE,
            );
        }

        // Isolation overlay: dim everything, then redraw the isolated
        // group on top so its children stay fully editable.
        if let Some(group) = state.isolated_group() {
            painter.rect_filled(artboard, 0.0_f32, Color32::from_black_alpha(110));
            self.draw_object(
                &painter,
                group,
                origin,
                state,
                &crate::ui::canvas::rendering::IDENTITY_AFFINE,
            );
        }

        // GPU-Accelerated Rendering Pass (effects: glow, blur, shadow)
        if let (Some(device), Some(queue)) = (device, queue) {
            self.ensure_gpu_renderer(device.clone(), queue.clone());

            if let Some(ref _gpu) = self.gpu_renderer {
                // Collect objects with GPU-renderable effects
                let mut has_gpu_effects = false;
                for (_, obj) in state.document.all_objects() {
                    if !obj.visible {
                        continue;
                    }
                    if obj.shadow.is_some() || obj.glow.is_some() {
                        has_gpu_effects = true;
                        break;
                    }
                }

                if has_gpu_effects {
                    let screen_rect = ui.ctx().input(|i| i.screen_rect);
                    let width = screen_rect.width() as u32;
                    let height = screen_rect.height() as u32;

                    // Create render target texture for GPU effects
                    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("GPU Effects Texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba16Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    let output_view =
                        output_texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("GPU Effects Encoder"),
                        });

                    // Render objects with GPU effects to texture
                    {
                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("GPU Effects Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &output_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        let mut gpu = GpuRenderer::new(device.clone(), queue.clone());
                        gpu.begin_frame();

                        // Collect geometry with effects
                        for (_, obj) in state.document.all_objects() {
                            if !obj.visible {
                                continue;
                            }

                            let _fill_rgba = obj
                                .fill
                                .as_ref()
                                .map(|f| {
                                    let c = f.color;
                                    [c[0], c[1], c[2], c[3] * obj.opacity]
                                })
                                .unwrap_or([0.0, 0.0, 0.0, 0.0]);

                            let _opacity = obj.opacity;

                            // Shadow: render offset geometry in shadow color
                            if let Some(ref sh) = obj.shadow {
                                let sh_color = [
                                    sh.color[0],
                                    sh.color[1],
                                    sh.color[2],
                                    sh.color[3] * sh.opacity * obj.opacity,
                                ];
                                let poly = obj.to_path_data().to_polygon(16);
                                let sh_pts: Vec<(f32, f32)> = poly
                                    .iter()
                                    .map(|p| {
                                        let (wx, wy) = obj
                                            .transform
                                            .transform_point(p.x + sh.offset_x, p.y + sh.offset_y);
                                        (
                                            origin.x + wx as f32 * state.zoom,
                                            origin.y + wy as f32 * state.zoom,
                                        )
                                    })
                                    .collect();
                                gpu.push_convex_polygon(&sh_pts, sh_color);
                            }

                            // Glow: render with additive blending
                            if let Some(ref gl) = obj.glow {
                                let gl_color = [
                                    gl.color[0],
                                    gl.color[1],
                                    gl.color[2],
                                    gl.color[3] * gl.intensity * obj.opacity,
                                ];
                                let poly = obj.to_path_data().to_polygon(16);
                                let screen_pts: Vec<(f32, f32)> = poly
                                    .iter()
                                    .map(|p| {
                                        let (wx, wy) = obj.transform.transform_point(p.x, p.y);
                                        (
                                            origin.x + wx as f32 * state.zoom,
                                            origin.y + wy as f32 * state.zoom,
                                        )
                                    })
                                    .collect();
                                // Render glow at larger scale
                                let cx: f32 = screen_pts.iter().map(|p| p.0).sum::<f32>()
                                    / screen_pts.len() as f32;
                                let cy: f32 = screen_pts.iter().map(|p| p.1).sum::<f32>()
                                    / screen_pts.len() as f32;
                                let glow_pts: Vec<(f32, f32)> = screen_pts
                                    .iter()
                                    .map(|p| {
                                        let dx = p.0 - cx;
                                        let dy = p.1 - cy;
                                        let r = gl.radius as f32;
                                        (p.0 + dx * r * 0.1, p.1 + dy * r * 0.1)
                                    })
                                    .collect();
                                gpu.push_convex_polygon(&glow_pts, gl_color);
                            }
                        }

                        let resolution = [width as f32, height as f32];
                        gpu.render(
                            &mut render_pass,
                            resolution,
                            state.zoom,
                            [state.pan_x, state.pan_y],
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            [0.0, 0.0],
                            0.0,
                            0.0,
                            1.0,
                        );
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                }
            }
        }

        // Smart Guides
        if state.show_smart_guides {
            self.draw_smart_guides(&painter, artboard, rect, origin, state);
        }

        // Drag Previews
        if let Some(ref drag) = self.drag {
            match drag.mode {
                DragMode::CreateRect => self.draw_rect_preview(&painter, origin, state, drag),
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
                for id in &state.selected_ids {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id)
                    {
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
                if cursor == egui::CursorIcon::Default
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
                Tool::Eyedropper | Tool::Brush | Tool::Eraser | Tool::Pen
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
                        Tool::Eraser => {
                            self.drag = Some(DragState::new(DragMode::EraserDrag, wx, wy));
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
                                self.node_edit_state.selected_target = Some(target);
                                self.node_edit_state.selected_object_id = Some(obj_id.clone());

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

                                let mut d = DragState::new(DragMode::MoveNode(target), wx, wy);
                                d.initial_elements = initial_elements;
                                self.drag = Some(d);
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
