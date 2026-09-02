use crate::core::document::{ObjectType, Object};
use crate::core::path::{
    AnchorPoint, FillStyle, FillType, LinearGradient, PatternFill, PathData, PathElement,
    RadialGradient, StrokeStyle,
};
use crate::core::state::{AppState, HandleCorner, Tool};
use crate::gpu::GpuRenderer;
use crate::tools::pen::PenState;
use crate::tools::select::SelectState;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::sync::Arc;

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
    MoveNode(usize),
}

struct DragState {
    mode: DragMode,
    start_world: (f64, f64),
    current_world: (f64, f64),
    pencil_points: Vec<AnchorPoint>,
}

impl DragState {
    fn new(mode: DragMode, wx: f64, wy: f64) -> Self {
        Self {
            mode,
            start_world: (wx, wy),
            current_world: (wx, wy),
            pencil_points: vec![AnchorPoint::new(wx, wy)],
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
    selected_anchor_idx: Option<usize>,
    selected_object_id: Option<String>,
}

impl NodeEditState {
    fn new() -> Self {
        Self {
            selected_anchor_idx: None,
            selected_object_id: None,
        }
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

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState, device: Option<Arc<wgpu::Device>>, queue: Option<Arc<wgpu::Queue>>) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height()),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        state.canvas_center_x = rect.center().x;
        state.canvas_center_y = rect.center().y;
        let origin = Pos2::new(
            rect.center().x + state.pan_x,
            rect.center().y + state.pan_y,
        );

        state.canvas_width = rect.width();
        state.canvas_height = rect.height();

        // Dark Pasteboard Canvas Background (#1e1e1e)
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(30, 30, 30));

        // Grid
        if state.show_grid {
            self.draw_grid(&painter, rect, origin, state);
        }

        // Artboard Dimensions & Realistic Drop Shadow
        let artboard = Rect::from_min_size(
            origin,
            Vec2::new(
                state.document.width as f32 * state.zoom,
                state.document.height as f32 * state.zoom,
            ),
        );
        let shadow_rect1 = artboard.translate(Vec2::new(5.0, 5.0));
        painter.rect_filled(shadow_rect1, 0.0_f32, Color32::from_black_alpha(60));
        let shadow_rect2 = artboard.translate(Vec2::new(2.0, 2.0));
        painter.rect_filled(shadow_rect2, 0.0_f32, Color32::from_black_alpha(100));

        // Crisp White Artboard Paper
        painter.rect_filled(artboard, 0.0_f32, Color32::WHITE);
        painter.rect_stroke(artboard, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(80, 80, 80)), StrokeKind::Inside);

        // Artboard Header Tab Label
        let tab_pos = Pos2::new(artboard.min.x, artboard.min.y - 18.0);
        painter.text(
            tab_pos,
            egui::Align2::LEFT_TOP,
            format!("Artboard 1 ({} × {} px)", state.document.width as i32, state.document.height as i32),
            FontId::proportional(11.0),
            Color32::from_rgb(170, 170, 170),
        );

        // Render Objects (CPU fallback via egui painter)
        for (_, obj) in state.document.all_objects() {
            if !obj.visible {
                continue;
            }
            self.draw_object(&painter, obj, origin, state);
        }

        // GPU-Accelerated Rendering Pass (effects: glow, blur, shadow)
        if let (Some(device), Some(queue)) = (device, queue) {
            self.ensure_gpu_renderer(device.clone(), queue.clone());

            if let Some(ref _gpu) = self.gpu_renderer {
                // Collect objects with GPU-renderable effects
                let mut has_gpu_effects = false;
                for (_, obj) in state.document.all_objects() {
                    if !obj.visible { continue; }
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
                        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba16Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("GPU Effects Encoder"),
                    });

                    // Render objects with GPU effects to texture
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                            if !obj.visible { continue; }

                            let _fill_rgba = obj.fill.as_ref().map(|f| {
                                let c = f.color;
                                [c[0], c[1], c[2], c[3] * obj.opacity]
                            }).unwrap_or([0.0, 0.0, 0.0, 0.0]);

                            let _opacity = obj.opacity;

                            // Shadow: render offset geometry in shadow color
                            if let Some(ref sh) = obj.shadow {
                                let sh_color = [sh.color[0], sh.color[1], sh.color[2], sh.color[3] * sh.opacity * obj.opacity];
                                let poly = obj.to_path_data().to_polygon(16);
                                let sh_pts: Vec<(f32, f32)> = poly.iter().map(|p| {
                                    let (wx, wy) = obj.transform.transform_point(
                                        p.x + sh.offset_x,
                                        p.y + sh.offset_y,
                                    );
                                    (origin.x + wx as f32 * state.zoom,
                                     origin.y + wy as f32 * state.zoom)
                                }).collect();
                                gpu.push_convex_polygon(&sh_pts, sh_color);
                            }

                            // Glow: render with additive blending
                            if let Some(ref gl) = obj.glow {
                                let gl_color = [gl.color[0], gl.color[1], gl.color[2], gl.color[3] * gl.intensity * obj.opacity];
                                let poly = obj.to_path_data().to_polygon(16);
                                let screen_pts: Vec<(f32, f32)> = poly.iter().map(|p| {
                                    let (wx, wy) = obj.transform.transform_point(p.x, p.y);
                                    (origin.x + wx as f32 * state.zoom,
                                     origin.y + wy as f32 * state.zoom)
                                }).collect();
                                // Render glow at larger scale
                                let cx: f32 = screen_pts.iter().map(|p| p.0).sum::<f32>() / screen_pts.len() as f32;
                                let cy: f32 = screen_pts.iter().map(|p| p.1).sum::<f32>() / screen_pts.len() as f32;
                                let glow_pts: Vec<(f32, f32)> = screen_pts.iter().map(|p| {
                                    let dx = p.0 - cx;
                                    let dy = p.1 - cy;
                                    let r = gl.radius as f32;
                                    (p.0 + dx * r * 0.1, p.1 + dy * r * 0.1)
                                }).collect();
                                gpu.push_convex_polygon(&glow_pts, gl_color);
                            }
                        }

                        let resolution = [width as f32, height as f32];
                        gpu.render(&mut render_pass, resolution, state.zoom, [state.pan_x, state.pan_y], 0.0, 0.0, 0.0, 0.0, [0.0, 0.0], 0.0, 0.0, 1.0);
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                }
            }
        }

        // Smart Guides
        if state.show_smart_guides && self.drag.is_some() {
            self.draw_smart_guides(&painter, artboard, rect);
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

        // Selection Bounding Box & Handles
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                self.draw_selection(&painter, obj, origin, state);
            }
        }

        // Node Editing Handles
        if state.current_tool == Tool::Node {
            self.draw_node_edit(&painter, origin, state);
        }

        // Rulers
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
            state.target_zoom = (state.target_zoom * (1.0_f32 + scroll * 0.001_f32)).clamp(0.01_f32, 100.0_f32);
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
        response.context_menu(|ui| {
            let has_sel = !state.selected_ids.is_empty();
            let multi_sel = state.selected_ids.len() >= 2;

            if has_sel {
                ui.label(egui::RichText::new("Selection").weak().size(10.0));
                ui.separator();

                if ui.button("Cut   Cmd+X").clicked() {
                    state.clipboard.clear();
                    let ids = state.selected_ids.clone();
                    for id in &ids {
                        if let Some(obj) = state.document.remove_object(id) {
                            state.clipboard.push(obj);
                        }
                    }
                    state.selected_ids.clear();
                    ui.close_menu();
                }

                if ui.button("Copy   Cmd+C").clicked() {
                    state.clipboard.clear();
                    for id in &state.selected_ids {
                        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                            state.clipboard.push(obj.clone());
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("Paste   Cmd+V").clicked() {
                    state.selected_ids.clear();
                    let mut offset = 0.0;
                    for obj in &state.clipboard {
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} (copy)", obj.name);
                        new_obj.transform.x += 20.0 + offset;
                        new_obj.transform.y += 20.0 + offset;
                        offset += 15.0;
                        let id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids.push(id);
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Duplicate   Cmd+D").clicked() {
                    let ids = state.selected_ids.clone();
                    let mut new_objs = Vec::new();
                    for id in &ids {
                        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                            let mut c = obj.clone();
                            c.id = uuid::Uuid::new_v4().to_string();
                            c.transform.x += 20.0;
                            c.transform.y += 20.0;
                            new_objs.push(c);
                        }
                    }
                    let mut new_ids = Vec::new();
                    for obj in new_objs {
                        new_ids.push(obj.id.clone());
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                    state.selected_ids = new_ids;
                    ui.close_menu();
                }

                if ui.button("Delete   Del").clicked() {
                    for id in state.selected_ids.drain(..) {
                        state.document.remove_object(&id);
                    }
                    ui.close_menu();
                }

                ui.separator();
                ui.menu_button("Arrange", |ui| {
                    if ui.button("Bring to Front   Cmd+Shift+]").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    let obj = layer.objects.remove(pos);
                                    layer.objects.push(obj);
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Bring Forward   Cmd+]").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    if pos + 1 < layer.objects.len() {
                                        layer.objects.swap(pos, pos + 1);
                                    }
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Send Backward   Cmd+[").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    if pos > 0 {
                                        layer.objects.swap(pos, pos - 1);
                                    }
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Send to Back   Cmd+Shift+[").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
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

                if multi_sel {
                    ui.separator();
                    if ui.button("Group   Cmd+G").clicked() {
                        // collect selected objects into a group
                        let sel = state.selected_ids.clone();
                        let mut children = Vec::new();
                        for id in &sel {
                            if let Some(obj) = state.document.remove_object(id) {
                                children.push(obj);
                            }
                        }
                        let grp = crate::core::document::Object::new_group("Group", children);
                        let new_id = grp.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(grp));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                        ui.close_menu();
                    }
                }

                if has_sel {
                    // Check if any selected object is a Group
                    let has_group = state.selected_ids.iter().any(|id| {
                        state.document.all_objects().any(|(_, o)| o.id == *id && matches!(o.object_type, ObjectType::Group(_)))
                    });
                    if has_group && ui.button("Ungroup   Cmd+Shift+G").clicked() {
                        let ids = state.selected_ids.clone();
                        let mut new_ids = Vec::new();
                        for id in &ids {
                            if let Some(obj) = state.document.remove_object(id) {
                                if let ObjectType::Group(children) = obj.object_type {
                                    for child in children {
                                        new_ids.push(child.id.clone());
                                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(child));
                                        state.undo_manager.execute(cmd, &mut state.document);
                                    }
                                } else {
                                    new_ids.push(obj.id.clone());
                                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                                    state.undo_manager.execute(cmd, &mut state.document);
                                }
                            }
                        }
                        state.selected_ids = new_ids;
                        ui.close_menu();
                    }
                }
            } else {
                ui.label(egui::RichText::new("Canvas").weak().size(10.0));
                ui.separator();
                if ui.button("Zoom to Fit   Cmd+0").clicked() {
                    state.start_zoom = state.zoom;
                    state.start_pan_x = state.pan_x;
                    state.start_pan_y = state.pan_y;
                    state.zoom_to_fit();
                    state.zoom_animation_progress = 0.0;
                    ui.close_menu();
                }
                if ui.button("Zoom 100%   Cmd+1").clicked() {
                    state.start_zoom = state.zoom;
                    state.start_pan_x = state.pan_x;
                    state.start_pan_y = state.pan_y;
                    state.target_zoom = 1.0;
                    state.target_pan_x = 0.0;
                    state.target_pan_y = 0.0;
                    state.zoom_animation_progress = 0.0;
                    ui.close_menu();
                }
                ui.separator();
                ui.checkbox(&mut state.show_grid, "Show Grid");
                ui.checkbox(&mut state.show_rulers, "Show Rulers");
                ui.checkbox(&mut state.show_smart_guides, "Smart Guides");
                ui.checkbox(&mut state.snap_to_objects, "Snap to Objects");
            }
        });

        let space_down = ui.input(|i| i.key_down(egui::Key::Space));
        let shift_down = ui.input(|i| i.modifiers.shift);
        let alt_down = ui.input(|i| i.modifiers.alt);


        if (response.secondary_clicked() || (space_down && response.clicked())) && !response.dragged() {
            self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
        }

        // Handle Pointer Interaction
        if let Some(screen_pos) = response.interact_pointer_pos() {
            let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);

            let mut cursor = egui::CursorIcon::Default;
            if space_down {
                cursor = if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab };
            } else if state.current_tool == Tool::Select {
                for id in &state.selected_ids {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                        if let Some(corner) = self.hit_test_handles(obj, screen_pos, origin, state) {
                            cursor = match corner {
                                HandleCorner::TopLeft | HandleCorner::BottomRight => egui::CursorIcon::ResizeNeSw,
                                HandleCorner::TopRight | HandleCorner::BottomLeft => egui::CursorIcon::ResizeNwSe,
                                HandleCorner::Top | HandleCorner::Bottom => egui::CursorIcon::ResizeVertical,
                                HandleCorner::Left | HandleCorner::Right => egui::CursorIcon::ResizeHorizontal,
                                HandleCorner::RotateTopRight => egui::CursorIcon::Crosshair,
                            };
                            break;
                        }
                    }
                }
                if cursor == egui::CursorIcon::Default
                    && self.select_state.hit_test(state, wx, wy).is_some() {
                        cursor = if alt_down { egui::CursorIcon::Copy } else { egui::CursorIcon::Move };
                    }
            } else if state.current_tool == Tool::Hand {
                cursor = if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab };
            } else if matches!(state.current_tool, Tool::Eyedropper | Tool::Brush | Tool::Eraser | Tool::Pen) {
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
        }

        // Drag Start
        if response.drag_started() && self.drag.is_none() {
            if space_down {
                self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
            } else if let Some(screen_pos) = response.interact_pointer_pos() {
                let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                let (wx, wy) = state.snap(wx, wy);

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
                            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                                if let Some(corner) = self.hit_test_handles(obj, screen_pos, origin, state) {
                                    if corner == HandleCorner::RotateTopRight {
                                        self.drag = Some(DragState::new(DragMode::Rotate, wx, wy));
                                    } else {
                                        self.drag = Some(DragState::new(DragMode::Resize(corner), wx, wy));
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
                        if let Some((anchor_idx, obj_id)) = self.hit_test_nodes(state, screen_pos, origin) {
                            self.node_edit_state.selected_anchor_idx = Some(anchor_idx);
                            self.node_edit_state.selected_object_id = Some(obj_id);
                            self.drag = Some(DragState::new(DragMode::MoveNode(anchor_idx), wx, wy));
                        }
                    }
                    Tool::Hand => {
                        self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
                    }
                    _ => {}
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

                    if matches!(drag.mode, DragMode::PencilDraw | DragMode::BrushDraw | DragMode::EraserDrag) {
                        drag.pencil_points.push(AnchorPoint::new(wx, wy));
                    } else if drag.mode == DragMode::MoveObject {
                        self.select_state.update_drag(state, wx, wy);
                    } else if drag.mode == DragMode::Rotate {
                        self.update_rotate(state, wx, wy);
                    } else if let DragMode::MoveNode(idx) = drag.mode {
                        self.move_node(state, idx, wx, wy);
                    } else if let DragMode::Resize(corner) = drag.mode {
                        self.update_resize(state, corner, wx, wy);
                    }
                }
            }
        }

        // Drag Stopped / Completed
        if response.drag_stopped() {
            if let Some(drag) = self.drag.take() {
                match drag.mode {
                    DragMode::CreateRect => {
                        let mut w = drag.current_world.0 - drag.start_world.0;
                        let mut h = drag.current_world.1 - drag.start_world.1;
                        if shift_down {
                            let sz = w.abs().max(h.abs());
                            w = sz * w.signum();
                            h = sz * h.signum();
                        }
                        if w.abs() > 4.0 && h.abs() > 4.0 {
                            let (x, y, w_val, h_val) = if alt_down {
                                (drag.start_world.0 - w.abs(), drag.start_world.1 - h.abs(), w.abs() * 2.0, h.abs() * 2.0)
                            } else {
                                (drag.start_world.0.min(drag.current_world.0), drag.start_world.1.min(drag.current_world.1), w.abs(), h.abs())
                            };
                            let mut obj = Object::new_rect("Rectangle", x, y, w_val, h_val, state.corner_radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreateEllipse => {
                        let mut rx = ((drag.current_world.0 - drag.start_world.0) / 2.0).abs();
                        let mut ry = ((drag.current_world.1 - drag.start_world.1) / 2.0).abs();
                        if shift_down {
                            let r = rx.max(ry);
                            rx = r;
                            ry = r;
                        }
                        if rx > 2.0 && ry > 2.0 {
                            let (cx, cy) = if alt_down {
                                (drag.start_world.0, drag.start_world.1)
                            } else {
                                ((drag.start_world.0 + drag.current_world.0) / 2.0, (drag.start_world.1 + drag.current_world.1) / 2.0)
                            };
                            let mut obj = Object::new_ellipse("Ellipse", cx, cy, rx, ry);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreateStar => {
                        let dx = drag.current_world.0 - drag.start_world.0;
                        let dy = drag.current_world.1 - drag.start_world.1;
                        let outer_radius = (dx * dx + dy * dy).sqrt();
                        if outer_radius > 4.0 {
                            let inner_radius = outer_radius * state.star_inner_ratio;
                            let mut obj = Object::new_star("Star", drag.start_world.0, drag.start_world.1, state.star_points, inner_radius, outer_radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreatePolygon => {
                        let dx = drag.current_world.0 - drag.start_world.0;
                        let dy = drag.current_world.1 - drag.start_world.1;
                        let radius = (dx * dx + dy * dy).sqrt();
                        if radius > 4.0 {
                            let mut obj = Object::new_polygon("Polygon", drag.start_world.0, drag.start_world.1, state.polygon_sides, radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreateLine => {
                        let mut dx = drag.current_world.0 - drag.start_world.0;
                        let mut dy = drag.current_world.1 - drag.start_world.1;
                        if shift_down {
                            let angle = dy.atan2(dx);
                            let snap_step = std::f64::consts::FRAC_PI_4;
                            let snapped = (angle / snap_step).round() * snap_step;
                            let len = (dx * dx + dy * dy).sqrt();
                            dx = len * snapped.cos();
                            dy = len * snapped.sin();
                        }
                        if (dx * dx + dy * dy).sqrt() > 2.0 {
                            let mut obj = Object::new_line("Line", drag.start_world.0, drag.start_world.1, drag.start_world.0 + dx, drag.start_world.1 + dy);
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::MoveObject => {
                        self.select_state.end_drag(state);
                        if alt_down {
                            // Alt+Drag duplicates selected objects!
                            let mut duplicated = Vec::new();
                            for id in &state.selected_ids {
                                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                                    let mut dup = obj.clone();
                                    dup.id = uuid::Uuid::new_v4().to_string();
                                    dup.name = format!("{} Copy", obj.name);
                                    duplicated.push(dup);
                                }
                            }
                            for dup in duplicated {
                                let new_id = dup.id.clone();
                                let cmd = Box::new(crate::core::history::AddObjectCommand::new(dup));
                                state.undo_manager.execute(cmd, &mut state.document);
                                state.selected_ids = vec![new_id];
                            }
                        }
                    }
                    DragMode::PencilDraw => {
                        if drag.pencil_points.len() >= 2 {
                            let mut path = PathData::from_smooth_points(&drag.pencil_points);
                            path.fill = None;
                            path.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let obj = Object::new_path("Pencil Stroke", path);
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::BrushDraw => {
                        if drag.pencil_points.len() >= 2 {
                            let mut path = PathData::from_smooth_points(&drag.pencil_points);
                            path.fill = None;
                            path.stroke = Some(StrokeStyle {
                                color: state.fill_color,
                                width: 4.0,
                                dash_pattern: None,
                                ..StrokeStyle::default()
                            });
                            let obj = Object::new_path("Brush Stroke", path);
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::EraserDrag => {
                        if drag.pencil_points.len() >= 2 {
                            let min_x = drag.pencil_points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                            let max_x = drag.pencil_points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                            let min_y = drag.pencil_points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                            let max_y = drag.pencil_points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

                            let eraser_margin = 10.0;
                            let eraser_rect_min = (min_x - eraser_margin, min_y - eraser_margin);
                            let eraser_rect_max = (max_x + eraser_margin, max_y + eraser_margin);

                            let mut ids_to_remove = Vec::new();
                            for (_, obj) in state.document.all_objects() {
                                if !obj.visible || obj.locked {
                                    continue;
                                }
                                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                                    let intersects = bb_max.x >= eraser_rect_min.0
                                        && bb_min.x <= eraser_rect_max.0
                                        && bb_max.y >= eraser_rect_min.1
                                        && bb_min.y <= eraser_rect_max.1;
                                    if intersects {
                                        ids_to_remove.push(obj.id.clone());
                                    }
                                }
                            }
                            for id in &ids_to_remove {
                                let layer_idx = state
                                    .document
                                    .layers
                                    .iter()
                                    .position(|l| l.objects.iter().any(|o| &o.id == id))
                                    .unwrap_or(0);
                                if let Some(obj) = state.document.remove_object(id) {
                                    let cmd = Box::new(crate::core::history::RemoveObjectCommand::new(
                                        obj, layer_idx, 0,
                                    ));
                                    state.undo_manager.execute(cmd, &mut state.document);
                                }
                            }
                        }
                    }
                    DragMode::Select => {
                        let min_x = drag.start_world.0.min(drag.current_world.0);
                        let max_x = drag.start_world.0.max(drag.current_world.0);
                        let min_y = drag.start_world.1.min(drag.current_world.1);
                        let max_y = drag.start_world.1.max(drag.current_world.1);

                        state.selected_ids.clear();
                        for (_, obj) in state.document.all_objects() {
                            if !obj.visible || obj.locked {
                                continue;
                            }
                            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                                if bb_max.x >= min_x && bb_min.x <= max_x && bb_max.y >= min_y && bb_min.y <= max_y {
                                    state.selected_ids.push(obj.id.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_click(
        &mut self,
        state: &mut AppState,
        wx: f64,
        wy: f64,
        _screen_pos: Pos2,
        _origin: Pos2,
        shift: bool,
    ) {
        match state.current_tool {
            Tool::Pen => {
                if !self.pen_state.is_drawing {
                    self.pen_state.start_path(wx, wy);
                } else {
                    self.pen_state.add_point(wx, wy);
                }
            }
            Tool::Text => {
                let mut obj = Object::new_text("Text", &state.text_input_buf, wx, wy, state.font_size);
                obj.fill = Some(FillStyle::solid(state.fill_color));
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
            Tool::Eyedropper => {
                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                        if let Some(ref fill) = obj.fill {
                            state.fill_color = fill.color;
                        } else if let Some(ref stroke) = obj.stroke {
                            // No fill on object: use stroke color as fill
                            state.fill_color = stroke.color;
                        }
                        if let Some(ref stroke) = obj.stroke {
                            state.stroke_color = stroke.color;
                            state.stroke_width = stroke.width;
                        }
                    }
                }
                let prev = state.previous_tool;
                state.current_tool = prev;
            }
            Tool::Select => {
                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                    if shift {
                        if let Some(pos) = state.selected_ids.iter().position(|x| x == &id) {
                            state.selected_ids.remove(pos);
                        } else {
                            state.selected_ids.push(id);
                        }
                    } else {
                        state.selected_ids = vec![id];
                    }
                } else if !shift {
                    state.selected_ids.clear();
                }
            }
            _ => {}
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect, origin: Pos2, state: &AppState) {
        let grid_px = state.grid_size as f32 * state.zoom;
        if grid_px < 5.0_f32 {
            return;
        }

        let start_x = ((rect.min.x - origin.x) / grid_px).floor() * grid_px + origin.x;
        let start_y = ((rect.min.y - origin.y) / grid_px).floor() * grid_px + origin.y;

        let grid_stroke = Stroke::new(0.5_f32, Color32::from_rgb(50, 52, 58));

        let mut x = start_x;
        while x <= rect.max.x {
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], grid_stroke);
            x += grid_px;
        }

        let mut y = start_y;
        while y <= rect.max.y {
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], grid_stroke);
            y += grid_px;
        }
    }

    fn draw_rulers(&self, painter: &egui::Painter, rect: Rect, origin: Pos2, state: &AppState) {
        let ruler_color = Color32::from_rgb(45, 46, 50);
        let tick_color = Color32::from_rgb(160, 160, 160);
        let font_id = FontId::proportional(9.0_f32);

        // Top ruler bar
        let top_ruler = Rect::from_min_size(rect.min, Vec2::new(rect.width(), RULER_WIDTH));
        painter.rect_filled(top_ruler, 0.0_f32, ruler_color);

        // Left ruler bar
        let left_ruler = Rect::from_min_size(rect.min, Vec2::new(RULER_WIDTH, rect.height()));
        painter.rect_filled(left_ruler, 0.0_f32, ruler_color);

        // Top-left corner box
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(RULER_WIDTH)), 0.0_f32, Color32::from_rgb(38, 39, 42));

        // Horizontal ticks
        let step: f32 = if state.zoom > 2.0_f32 { 50.0_f32 } else if state.zoom < 0.3_f32 { 500.0_f32 } else { 100.0_f32 };
        let step_px = step * state.zoom;
        let start_val = ((rect.min.x - origin.x) / step_px).floor() * step;

        let mut val = start_val;
        while val * state.zoom + origin.x <= rect.max.x {
            let sx = origin.x + (val * state.zoom);
            if sx >= rect.min.x + RULER_WIDTH {
                painter.line_segment([Pos2::new(sx, rect.min.y + 12.0_f32), Pos2::new(sx, rect.min.y + RULER_WIDTH)], Stroke::new(1.0_f32, tick_color));
                painter.text(Pos2::new(sx + 2.0_f32, rect.min.y + 2.0_f32), egui::Align2::LEFT_TOP, format!("{val:.0}"), font_id.clone(), tick_color);
            }
            val += step;
        }

        // Vertical ticks
        let mut val_y = ((rect.min.y - origin.y) / step_px).floor() * step;
        while val_y * state.zoom + origin.y <= rect.max.y {
            let sy = origin.y + (val_y * state.zoom);
            if sy >= rect.min.y + RULER_WIDTH {
                painter.line_segment([Pos2::new(rect.min.x + 12.0_f32, sy), Pos2::new(rect.min.x + RULER_WIDTH, sy)], Stroke::new(1.0_f32, tick_color));
                painter.text(Pos2::new(rect.min.x + 2.0_f32, sy + 2.0_f32), egui::Align2::LEFT_TOP, format!("{val_y:.0}"), font_id.clone(), tick_color);
            }
            val_y += step;
        }
    }

    fn draw_smart_guides(&self, painter: &egui::Painter, artboard: Rect, canvas_rect: Rect) {
        let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 128));
        let center_x = artboard.center().x;
        let center_y = artboard.center().y;

        painter.line_segment([Pos2::new(center_x, canvas_rect.min.y), Pos2::new(center_x, canvas_rect.max.y)], guide_stroke);
        painter.line_segment([Pos2::new(canvas_rect.min.x, center_y), Pos2::new(canvas_rect.max.x, center_y)], guide_stroke);
    }

    fn draw_object(&self, painter: &egui::Painter, obj: &Object, origin: Pos2, state: &AppState) {
        let opacity = obj.opacity.clamp(0.0_f32, 1.0_f32);
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(origin.x + sx as f32 * state.zoom, origin.y + sy as f32 * state.zoom)
        };

        let fill_color = obj.fill.as_ref().and_then(|f| fill_type_color(f, opacity));

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0_f32) as u8,
                (c[1] * 255.0_f32) as u8,
                (c[2] * 255.0_f32) as u8,
                ((c[3] * opacity) * 255.0_f32) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });

        if let Some(ref sh) = obj.shadow {
            let sh_c = Color32::from_rgba_unmultiplied(
                (sh.color[0] * 255.0_f32) as u8,
                (sh.color[1] * 255.0_f32) as u8,
                (sh.color[2] * 255.0_f32) as u8,
                ((sh.color[3] * sh.opacity * opacity) * 255.0_f32) as u8,
            );
            let sh_to_screen = |lx: f64, ly: f64| -> Pos2 {
                let (wx, wy) = obj.transform.transform_point(lx + sh.offset_x, ly + sh.offset_y);
                Pos2::new(
                    origin.x + (wx as f32 * state.zoom),
                    origin.y + (wy as f32 * state.zoom),
                )
            };
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                let sh_pts: Vec<Pos2> = poly.iter().map(|p| sh_to_screen(p.x, p.y)).collect();
                painter.add(egui::epaint::PathShape::convex_polygon(sh_pts, sh_c, Stroke::NONE));
            }
        }

        match &obj.object_type {
            ObjectType::Path(path) => {
                let poly = path.to_polygon(16);
                if poly.len() >= 3 {
                    let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();
                    if let Some(fill) = fill_color {
                        painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                    }
                    if let Some(stroke) = stroke_info {
                        painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
                    }
                }
            }
            ObjectType::Rectangle { width, height, corner_radius } => {
                let path = PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius);
                let screen_pts: Vec<Pos2> = path.to_polygon(12).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Ellipse { rx, ry } => {
                let path = PathData::from_ellipse(0.0, 0.0, *rx, *ry);
                let screen_pts: Vec<Pos2> = path.to_polygon(24).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Star { points, inner_radius, outer_radius } => {
                let path = PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Polygon { sides, radius } => {
                let path = PathData::from_polygon(*sides, *radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Line { x2, y2 } => {
                let p1 = to_screen(0.0, 0.0);
                let p2 = to_screen(*x2, *y2);
                let stroke = stroke_info.unwrap_or_else(|| Stroke::new(2.0_f32, Color32::BLACK));
                painter.line_segment([p1, p2], stroke);
            }
            ObjectType::Text { text, font_size } => {
                let pos = to_screen(0.0, 0.0);
                let font_id = FontId::proportional((font_size * state.zoom as f64) as f32);
                let color = fill_color.unwrap_or(Color32::BLACK);
                painter.text(pos, egui::Align2::LEFT_BOTTOM, text, font_id, color);
            }
            ObjectType::Group(children) => {
                for child in children {
                    self.draw_object(painter, child, origin, state);
                }
            }
            ObjectType::ClippingMask { children } => {
                for child in children {
                    self.draw_object(painter, child, origin, state);
                }
            }
        }

        if let Some(ref fill) = obj.fill {
            match &fill.fill_type {
                FillType::Linear(grad) => {
                    self.draw_linear_gradient(painter, obj, grad, opacity, origin, state);
                }
                FillType::Radial(grad) => {
                    self.draw_radial_gradient(painter, obj, grad, opacity, origin, state);
                }
                FillType::Pattern(pat) => {
                    self.draw_pattern_fill(painter, obj, pat, opacity, origin, state);
                }
                FillType::Solid(_) => {}
            }
        }
    }

    fn draw_linear_gradient(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        grad: &LinearGradient,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(origin.x + sx as f32 * state.zoom, origin.y + sy as f32 * state.zoom)
        };

        let path = obj.to_path_data();
        let poly = path.to_polygon(16);
        if poly.len() < 3 {
            return;
        }
        let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();

        let dx = grad.end_x - grad.start_x;
        let dy = grad.end_y - grad.start_y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }

        let num_bands = 32;
        let min_y = screen_pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = screen_pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let band_height = (max_y - min_y) / num_bands as f32;

        for i in 0..num_bands {
            let y_top = min_y + i as f32 * band_height;
            let y_bot = y_top + band_height;
            let y_mid = (y_top + y_bot) / 2.0;

            let t = ((y_mid - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
            let c = sample_gradient_stops(&grad.stops, t);
            let a = c[3] * opacity;
            if a <= 0.0 {
                continue;
            }
            let band_color = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (a * 255.0) as u8,
            );

            let clipped = clip_polygon_to_y_band(&screen_pts, y_top, y_bot);
            if clipped.len() >= 3 {
                painter.add(egui::epaint::PathShape::convex_polygon(
                    clipped,
                    band_color,
                    Stroke::NONE,
                ));
            }
        }

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                ((c[3] * opacity) * 255.0) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });
        if let Some(stroke) = stroke_info {
            painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
        }
    }

    fn draw_radial_gradient(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        grad: &RadialGradient,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(origin.x + sx as f32 * state.zoom, origin.y + sy as f32 * state.zoom)
        };

        let path = obj.to_path_data();
        let poly = path.to_polygon(16);
        if poly.len() < 3 {
            return;
        }
        let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();

        let min_x = screen_pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = screen_pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = screen_pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = screen_pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;
        let max_r = ((max_x - min_x).max(max_y - min_y)) / 2.0;
        if max_r <= 0.0 {
            return;
        }

        let num_rings = 24;
        for i in (0..num_rings).rev() {
            let t = (i as f32) / (num_rings as f32);
            let inner_r = t * max_r;
            let outer_r = ((i + 1) as f32) / (num_rings as f32) * max_r;

            let c = sample_gradient_stops(&grad.stops, t);
            let a = c[3] * opacity;
            if a <= 0.0 {
                continue;
            }
            let ring_color = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (a * 255.0) as u8,
            );

            let segments = 32;
            let mut ring_pts: Vec<Pos2> = Vec::new();
            for j in 0..=segments {
                let angle = (j as f32 / segments as f32) * std::f32::consts::TAU;
                ring_pts.push(Pos2::new(
                    cx + outer_r * angle.cos(),
                    cy + outer_r * angle.sin(),
                ));
            }
            for j in (0..=segments).rev() {
                let angle = (j as f32 / segments as f32) * std::f32::consts::TAU;
                ring_pts.push(Pos2::new(
                    cx + inner_r * angle.cos(),
                    cy + inner_r * angle.sin(),
                ));
            }

            painter.add(egui::epaint::PathShape::convex_polygon(
                ring_pts,
                ring_color,
                Stroke::NONE,
            ));
        }

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                ((c[3] * opacity) * 255.0) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });
        if let Some(stroke) = stroke_info {
            painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
        }
    }

    fn draw_pattern_fill(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        pat: &PatternFill,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let tile_w = (pat.tile_width * pat.scale) as f32 * state.zoom;
            let tile_h = (pat.tile_height * pat.scale) as f32 * state.zoom;
            if tile_w < 2.0 || tile_h < 2.0 {
                return;
            }

            let base_color = obj.fill.as_ref().map(|f| f.color).unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let a = (base_color[3] * opacity * 255.0) as u8;
            let tile_color = Color32::from_rgba_unmultiplied(
                (base_color[0] * 255.0) as u8,
                (base_color[1] * 255.0) as u8,
                (base_color[2] * 255.0) as u8,
                a,
            );

            let sx = origin.x + bb_min.x as f32 * state.zoom + (pat.offset_x as f32 * state.zoom);
            let sy = origin.y + bb_min.y as f32 * state.zoom + (pat.offset_y as f32 * state.zoom);
            let ex = origin.x + bb_max.x as f32 * state.zoom;
            let ey = origin.y + bb_max.y as f32 * state.zoom;

            let mut y = sy;
            while y < ey {
                let mut x = sx;
                while x < ex {
                    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(tile_w, tile_h));
                    painter.rect_filled(r, 0.0, tile_color);
                    x += tile_w;
                }
                y += tile_h;
            }
        }
    }

    fn draw_selection(&self, painter: &egui::Painter, obj: &Object, origin: Pos2, state: &AppState) {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let min_p = Pos2::new(origin.x + bb_min.x as f32 * state.zoom, origin.y + bb_min.y as f32 * state.zoom);
            let max_p = Pos2::new(origin.x + bb_max.x as f32 * state.zoom, origin.y + bb_max.y as f32 * state.zoom);
            let rect = Rect::from_min_max(min_p, max_p);

            let sel_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
            painter.rect_stroke(rect, 0.0_f32, sel_stroke, StrokeKind::Outside);

            // Center target crosshair (+)
            let cp = rect.center();
            let c_size = 4.0_f32;
            let c_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
            painter.line_segment([Pos2::new(cp.x - c_size, cp.y), Pos2::new(cp.x + c_size, cp.y)], c_stroke);
            painter.line_segment([Pos2::new(cp.x, cp.y - c_size), Pos2::new(cp.x, cp.y + c_size)], c_stroke);

            // 8 handles (Hollow white square with blue border)
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
                rect.center_top(),
                rect.center_bottom(),
                rect.left_center(),
                rect.right_center(),
            ];

            for p in corners {
                let h_rect = Rect::from_center_size(p, Vec2::splat(HANDLE_SIZE));
                painter.rect_filled(h_rect, 0.0_f32, Color32::WHITE);
                painter.rect_stroke(h_rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)), StrokeKind::Outside);
            }

            // Top Rotation handle circle above right_top
            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            painter.circle_filled(rot_p, 4.0_f32, Color32::from_rgb(20, 115, 230));
            painter.circle_stroke(rot_p, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
        }
    }

    fn hit_test_handles(&self, obj: &Object, screen_pos: Pos2, origin: Pos2, state: &AppState) -> Option<HandleCorner> {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let min_p = Pos2::new(origin.x + bb_min.x as f32 * state.zoom, origin.y + bb_min.y as f32 * state.zoom);
            let max_p = Pos2::new(origin.x + bb_max.x as f32 * state.zoom, origin.y + bb_max.y as f32 * state.zoom);
            let rect = Rect::from_min_max(min_p, max_p);

            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            if screen_pos.distance(rot_p) <= HANDLE_HIT_RADIUS {
                return Some(HandleCorner::RotateTopRight);
            }

            let handles = [
                (rect.left_top(), HandleCorner::TopLeft),
                (rect.right_top(), HandleCorner::TopRight),
                (rect.right_bottom(), HandleCorner::BottomRight),
                (rect.left_bottom(), HandleCorner::BottomLeft),
                (rect.center_top(), HandleCorner::Top),
                (rect.center_bottom(), HandleCorner::Bottom),
                (rect.left_center(), HandleCorner::Left),
                (rect.right_center(), HandleCorner::Right),
            ];

            for (p, corner) in handles {
                if screen_pos.distance(p) <= HANDLE_HIT_RADIUS {
                    return Some(corner);
                }
            }
        }
        None
    }

    fn update_rotate(&self, state: &mut AppState, wx: f64, wy: f64) {
        for id in &state.selected_ids {
            for (_, obj) in state.document.all_objects_mut() {
                if &obj.id == id {
                    let cx = obj.transform.x;
                    let cy = obj.transform.y;
                    let angle = (wy - cy).atan2(wx - cx);
                    obj.transform.rotation = angle;
                }
            }
        }
    }

    fn update_resize(&self, state: &mut AppState, corner: HandleCorner, wx: f64, wy: f64) {
        if let Some(ref drag) = self.drag {
            let (start_wx, start_wy) = drag.start_world;
            let id = state.selected_ids[0].clone();

            // Get current bounding box
            let (bb_min, bb_max) = if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                if let Some((min, max)) = obj.bounding_box() {
                    (min, max)
                } else {
                    return;
                }
            } else {
                return;
            };

            let bb_w = bb_max.x - bb_min.x;
            let bb_h = bb_max.y - bb_min.y;
            if bb_w < 1.0 || bb_h < 1.0 { return; }

            let dx = wx - start_wx;
            let dy = wy - start_wy;

            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    match corner {
                        HandleCorner::BottomRight => {
                            let new_w = (bb_w + dx).max(5.0);
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::TopLeft => {
                            let new_w = (bb_w - dx).max(5.0);
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::TopRight => {
                            let new_w = (bb_w + dx).max(5.0);
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::BottomLeft => {
                            let new_w = (bb_w - dx).max(5.0);
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Top => {
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Bottom => {
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Left => {
                            let new_w = (bb_w - dx).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.scale_x = new_w / bb_w;
                        }
                        HandleCorner::Right => {
                            let new_w = (bb_w + dx).max(5.0);
                            obj.transform.scale_x = new_w / bb_w;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn hit_test_nodes(&self, state: &AppState, screen_pos: Pos2, origin: Pos2) -> Option<(usize, String)> {
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                let poly = obj.to_world_polygon();
                for (idx, p) in poly.iter().enumerate() {
                    let sp = Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom);
                    if screen_pos.distance(sp) <= HANDLE_HIT_RADIUS {
                        return Some((idx, id.clone()));
                    }
                }
            }
        }
        None
    }

    fn move_node(&self, state: &mut AppState, node_idx: usize, wx: f64, wy: f64) {
        if let Some(ref obj_id) = self.node_edit_state.selected_object_id {
            for (_, obj) in state.document.all_objects_mut() {
                if &obj.id == obj_id {
                    if let ObjectType::Path(ref mut path) = obj.object_type {
                        if let Some(elem) = path.elements.get_mut(node_idx) {
                            match elem {
                                PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                                    *p = AnchorPoint::new(wx, wy);
                                }
                                PathElement::CurveTo(seg) => {
                                    seg.end = AnchorPoint::new(wx, wy);
                                }
                                PathElement::ClosePath => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_node_edit(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let active_node_idx = self.node_edit_state.selected_anchor_idx;
        let active_obj_id = self.node_edit_state.selected_object_id.as_deref();

        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                let poly = obj.to_world_polygon();
                for (idx, p) in poly.iter().enumerate() {
                    let sp = Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom);
                    let is_active = active_obj_id == Some(id.as_str()) && active_node_idx == Some(idx);

                    let anchor_rect = Rect::from_center_size(sp, Vec2::splat(6.0));
                    if is_active {
                        // Selected anchor: Solid blue square with white outline
                        painter.rect_filled(anchor_rect, 0.0_f32, Color32::from_rgb(0, 140, 255));
                        painter.rect_stroke(anchor_rect, 0.0_f32, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Outside);
                    } else {
                        // Unselected anchor: Hollow white square with blue outline
                        painter.rect_filled(anchor_rect, 0.0_f32, Color32::WHITE);
                        painter.rect_stroke(anchor_rect, 0.0_f32, Stroke::new(1.2_f32, Color32::from_rgb(0, 140, 255)), StrokeKind::Outside);
                    }
                }
            }
        }
    }

    fn draw_rect_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 40));
        painter.rect_stroke(rect, 0.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)), StrokeKind::Outside);
    }

    fn draw_ellipse_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.circle_filled(rect.center(), rect.width() / 2.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 40));
        painter.circle_stroke(rect.center(), rect.width() / 2.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)));
    }

    fn draw_star_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let outer_r = (dx * dx + dy * dy).sqrt();
        let inner_r = outer_r * state.star_inner_ratio;
        let path = PathData::from_star(state.star_points, inner_r, outer_radius_to_f64(outer_r), drag.start_world.0, drag.start_world.1);
        let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
        painter.add(egui::epaint::PathShape::closed_line(screen_pts, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255))));
    }

    fn draw_polygon_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let radius = (dx * dx + dy * dy).sqrt();
        let path = PathData::from_polygon(state.polygon_sides, radius, drag.start_world.0, drag.start_world.1);
        let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
        painter.add(egui::epaint::PathShape::closed_line(screen_pts, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255))));
    }

    fn draw_line_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        painter.line_segment([p1, p2], Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255)));
    }

    fn draw_pencil_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag.pencil_points.iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
            painter.add(egui::epaint::PathShape::line(pts, Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255))));
        }
    }

    fn draw_brush_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag.pencil_points.iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
            let fill_c = state.fill_color;
            let color = Color32::from_rgba_unmultiplied(
                (fill_c[0] * 255.0) as u8,
                (fill_c[1] * 255.0) as u8,
                (fill_c[2] * 255.0) as u8,
                (fill_c[3] * 255.0) as u8,
            );
            painter.add(egui::epaint::PathShape::line(pts, Stroke::new(4.0_f32, color)));
        }
    }

    fn draw_eraser_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag.pencil_points.iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
            painter.add(egui::epaint::PathShape::line(pts, Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(255, 80, 80, 180))));
        }
    }

    fn draw_marquee(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 30));
        painter.rect_stroke(rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)), StrokeKind::Outside);
    }

    fn draw_pen_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let w2s = |x: f64, y: f64| -> Pos2 {
            Pos2::new(origin.x + x as f32 * state.zoom, origin.y + y as f32 * state.zoom)
        };
        let mut screen_pts = Vec::new();

        for p in &self.pen_state.points {
            let sp = w2s(p.anchor.x, p.anchor.y);
            screen_pts.push(sp);

            // Draw bezier handle lines
            if let Some(ho) = p.handle_out {
                let hsp = w2s(ho.x, ho.y);
                painter.line_segment([sp, hsp], Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 240)));
                painter.circle_filled(hsp, 4.0_f32, Color32::from_rgb(20, 115, 230));
                painter.circle_stroke(hsp, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
            }
            if let Some(hi) = p.handle_in {
                let hsp = w2s(hi.x, hi.y);
                painter.line_segment([sp, hsp], Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 240)));
                painter.circle_filled(hsp, 4.0_f32, Color32::from_rgb(20, 115, 230));
                painter.circle_stroke(hsp, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
            }

            // Anchor point square (Illustrator style: white fill, blue border)
            let a_rect = Rect::from_center_size(sp, Vec2::splat(6.0));
            painter.rect_filled(a_rect, 0.0_f32, Color32::WHITE);
            painter.rect_stroke(a_rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)), StrokeKind::Outside);
        }

        // Live Illustrator Rubberband Line from last anchor to mouse hover
        if let Some((hx, hy)) = self.pen_state.hover_pos {
            let hp = w2s(hx, hy);
            if let Some(last_sp) = screen_pts.last() {
                // Dashed line from last anchor to cursor
                painter.line_segment([*last_sp, hp], Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)));
            }
            // Mouse cursor crosshair dot
            painter.circle_filled(hp, 3.0_f32, Color32::from_rgb(20, 115, 230));
            painter.circle_stroke(hp, 3.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
        }

        // Draw the path segments as actual bezier curves
        if screen_pts.len() >= 2 {
            // Simple polyline for straight segments; curves handled by segment drawing
            for i in 1..self.pen_state.points.len() {
                let prev = &self.pen_state.points[i - 1];
                let curr = &self.pen_state.points[i];
                let p0 = w2s(prev.anchor.x, prev.anchor.y);
                let p3 = w2s(curr.anchor.x, curr.anchor.y);
                let c0 = prev.handle_out.map(|h| w2s(h.x, h.y)).unwrap_or(p0);
                let c1 = curr.handle_in.map(|h| w2s(h.x, h.y)).unwrap_or(p3);

                if c0 == p0 && c1 == p3 {
                    painter.line_segment([p0, p3], Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)));
                } else {
                    // Approximate bezier with segments
                    let steps = 32;
                    let mut prev_pt = p0;
                    for step in 1..=steps {
                        let t = step as f32 / steps as f32;
                        let mt = 1.0 - t;
                        let x = mt*mt*mt*p0.x + 3.0*mt*mt*t*c0.x + 3.0*mt*t*t*c1.x + t*t*t*p3.x;
                        let y = mt*mt*mt*p0.y + 3.0*mt*mt*t*c0.y + 3.0*mt*t*t*c1.y + t*t*t*p3.y;
                        let next_pt = Pos2::new(x, y);
                        painter.line_segment([prev_pt, next_pt], Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)));
                        prev_pt = next_pt;
                    }
                }
            }
        }
    }
}

fn outer_radius_to_f64(r: f64) -> f64 {
    r
}

fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn clip_polygon_to_y_band(poly: &[Pos2], y_top: f32, y_bot: f32) -> Vec<Pos2> {
    if poly.len() < 3 {
        return poly.to_vec();
    }
    let mut result = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let curr_in = curr.y >= y_top && curr.y <= y_bot;
        let next_in = next.y >= y_top && next.y <= y_bot;

        if curr_in && next_in {
            result.push(curr);
        } else if curr_in && !next_in {
            result.push(curr);
            if (next.y - curr.y).abs() > f32::EPSILON {
                let t = if next.y > curr.y {
                    (y_bot - curr.y) / (next.y - curr.y)
                } else {
                    (y_top - curr.y) / (next.y - curr.y)
                };
                let t = t.clamp(0.0, 1.0);
                result.push(Pos2::new(
                    curr.x + t * (next.x - curr.x),
                    curr.y + t * (next.y - curr.y),
                ));
            }
        } else if !curr_in && next_in && (next.y - curr.y).abs() > f32::EPSILON {
            let t = if next.y > curr.y {
                (y_top - curr.y) / (next.y - curr.y)
            } else {
                (y_bot - curr.y) / (next.y - curr.y)
            };
            let t = t.clamp(0.0, 1.0);
            result.push(Pos2::new(
                curr.x + t * (next.x - curr.x),
                curr.y + t * (next.y - curr.y),
            ));
        }
    }
    result
}

fn sample_gradient_stops(stops: &[crate::core::path::GradientStop], t: f32) -> [f32; 4] {
    if stops.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }
    if stops.len() == 1 {
        return stops[0].color;
    }
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        if t >= w[0].offset && t <= w[1].offset {
            let span = w[1].offset - w[0].offset;
            let local_t = if span > 0.0 {
                (t - w[0].offset) / span
            } else {
                0.0
            };
            return [
                w[0].color[0] + (w[1].color[0] - w[0].color[0]) * local_t,
                w[0].color[1] + (w[1].color[1] - w[0].color[1]) * local_t,
                w[0].color[2] + (w[1].color[2] - w[0].color[2]) * local_t,
                w[0].color[3] + (w[1].color[3] - w[0].color[3]) * local_t,
            ];
        }
    }
    stops.last().unwrap().color
}

fn fill_type_color(fill: &FillStyle, opacity: f32) -> Option<Color32> {
    let c = match &fill.fill_type {
        FillType::Solid(color) => *color,
        FillType::Linear(_) | FillType::Radial(_) | FillType::Pattern(_) => return None,
    };
    let a = c[3] * opacity;
    if a <= 0.0 {
        return None;
    }
    Some(Color32::from_rgba_unmultiplied(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (a * 255.0) as u8,
    ))
}
