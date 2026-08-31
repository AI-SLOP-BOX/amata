use crate::core::state::AppState;
use crate::core::timeline::{AnimProperty, EaseType};
use egui::{Color32, RichText, Ui, Vec2};

pub struct TimelineWidget;

impl TimelineWidget {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("⏱️ Timeline & Animation").strong().size(14.0));

            let play_text = if state.timeline.is_playing { "⏸ Pause" } else { "▶ Play" };
            if ui.button(play_text).clicked() {
                state.timeline.is_playing = !state.timeline.is_playing;
            }

            if ui.button("⏹ Stop").clicked() {
                state.timeline.is_playing = false;
                state.timeline.current_frame = 0;
            }

            ui.separator();

            let fps = state.timeline.fps;
            let cf = state.timeline.current_frame;
            let tf = state.timeline.total_frames;
            let secs = cf as f64 / fps.max(1.0);
            ui.label(RichText::new(format!("Frame: {:03}/{} ({:.2}s)", cf, tf, secs)).strong().color(Color32::from_rgb(0, 180, 255)));

            ui.add(egui::Slider::new(&mut state.timeline.current_frame, 0..=tf.saturating_sub(1)).show_value(false));

            ui.separator();
            ui.checkbox(&mut state.timeline.loop_playback, "Loop");

            // Keyframing controls for selected object
            if let Some(sel_id) = state.selected_ids.first().cloned() {
                ui.separator();
                ui.menu_button("➕ Add Keyframe", |ui| {
                    let mut obj_tx = 0.0;
                    let mut obj_ty = 0.0;
                    let mut obj_rot = 0.0;
                    let mut obj_sx = 1.0;
                    let mut obj_sy = 1.0;
                    let mut obj_opac = 1.0;

                    for (_, obj) in state.document.all_objects() {
                        if obj.id == sel_id {
                            obj_tx = obj.transform.x;
                            obj_ty = obj.transform.y;
                            obj_rot = obj.transform.rotation.to_degrees();
                            obj_sx = obj.transform.scale_x;
                            obj_sy = obj.transform.scale_y;
                            obj_opac = obj.opacity as f64;
                            break;
                        }
                    }

                    if ui.button("📍 Position (X, Y)").clicked() {
                        let track_x = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::PositionX);
                        track_x.add_keyframe(cf, obj_tx, EaseType::EaseInOut);
                        let track_y = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::PositionY);
                        track_y.add_keyframe(cf, obj_ty, EaseType::EaseInOut);
                        ui.close_menu();
                    }

                    if ui.button("🔄 Rotation").clicked() {
                        let track_rot = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::Rotation);
                        track_rot.add_keyframe(cf, obj_rot, EaseType::EaseInOut);
                        ui.close_menu();
                    }

                    if ui.button("⇲ Scale (X, Y)").clicked() {
                        let track_sx = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::ScaleX);
                        track_sx.add_keyframe(cf, obj_sx, EaseType::EaseInOut);
                        let track_sy = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::ScaleY);
                        track_sy.add_keyframe(cf, obj_sy, EaseType::EaseInOut);
                        ui.close_menu();
                    }

                    if ui.button("👁 Opacity").clicked() {
                        let track_op = state.timeline.add_or_get_track_mut(&sel_id, AnimProperty::Opacity);
                        track_op.add_keyframe(cf, obj_opac, EaseType::Linear);
                        ui.close_menu();
                    }
                });
            }
        });

        // Track list display
        if !state.timeline.tracks.is_empty() {
            ui.add_space(2.0);
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for track in &state.timeline.tracks {
                        let prop_name = match track.property {
                            AnimProperty::PositionX => "Pos X",
                            AnimProperty::PositionY => "Pos Y",
                            AnimProperty::Rotation => "Rot",
                            AnimProperty::ScaleX => "Scale X",
                            AnimProperty::ScaleY => "Scale Y",
                            AnimProperty::Opacity => "Opacity",
                        };
                        let obj_name = state.document.all_objects()
                            .find(|(_, o)| o.id == track.object_id)
                            .map(|(_, o)| o.name.as_str())
                            .unwrap_or("Object");

                        let (rect, _) = ui.allocate_exact_size(Vec2::new(140.0, 20.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, Color32::from_gray(40));
                        ui.painter().text(
                            rect.left_center() + Vec2::new(4.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{}: {} ({})", obj_name, prop_name, track.keyframes.len()),
                            egui::FontId::proportional(11.0),
                            Color32::from_gray(220),
                        );
                    }
                });
            });
        }
    }
}
