use crate::app::icons::{icon_button_labeled, icon_pause, icon_play, icon_stop};
use crate::core::state::AppState;
use crate::core::timeline::{AnimProperty, EaseType};
use egui::{Color32, RichText, Ui, Vec2};

pub struct TimelineWidget;

impl TimelineWidget {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new(crate::ui::i18n::text(&locale, "timeline.title"))
                    .strong()
                    .size(14.0),
            );

            let playing = state.timeline.is_playing;
            let play_tip = crate::ui::i18n::text(
                &locale,
                if playing {
                    "timeline.pause"
                } else {
                    "timeline.play"
                },
            );
            if icon_button_labeled(
                ui,
                Vec2::new(22.0, 20.0),
                if playing { icon_pause } else { icon_play },
                play_tip.clone(),
            )
            .on_hover_text(play_tip)
            .clicked()
            {
                state.timeline.is_playing = !state.timeline.is_playing;
            }

            if icon_button_labeled(
                ui,
                Vec2::new(22.0, 20.0),
                icon_stop,
                crate::ui::i18n::text(&locale, "timeline.stop"),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "timeline.stop"))
            .clicked()
            {
                state.timeline.is_playing = false;
                state.timeline.current_frame = 0;
            }

            ui.separator();

            let fps = state.timeline.fps;
            let cf = state.timeline.current_frame;
            let tf = state.timeline.total_frames;
            let secs = cf as f64 / fps.max(1.0);
            ui.label(
                RichText::new(crate::ui::i18n::format(
                    &locale,
                    "timeline.frame",
                    &[
                        ("current", &format!("{cf:03}")),
                        ("total", &tf.to_string()),
                        ("seconds", &format!("{secs:.2}")),
                    ],
                ))
                .strong()
                .color(Color32::from_rgb(0, 180, 255)),
            );

            if tf > 1 {
                // Scrubbing fights the playback tick (advance_frame +
                // apply_to_document): pause while dragged, resume after.
                let was_playing = state.timeline.is_playing;
                let resp = ui.add_enabled(
                    true,
                    egui::Slider::new(&mut state.timeline.current_frame, 0..=tf - 1)
                        .show_value(false),
                );
                if resp.drag_started() {
                    state.timeline.is_playing = false;
                    state.timeline_was_scrubbing = was_playing;
                }
                if resp.drag_stopped() && state.timeline_was_scrubbing {
                    state.timeline.is_playing = true;
                    state.timeline_was_scrubbing = false;
                }
            } else {
                // Degenerate range (0/1 frames): a slider here divides by
                // zero internally and the thumb sticks.
                ui.label(RichText::new("—").weak());
            }

            ui.separator();
            ui.checkbox(
                &mut state.timeline.loop_playback,
                crate::ui::i18n::text(&locale, "timeline.loop"),
            );

            // Keyframing controls for selected object
            if let Some(sel_id) = state.selected_ids.first().cloned() {
                ui.separator();
                ui.menu_button(
                    crate::ui::i18n::text(&locale, "timeline.add_keyframe"),
                    |ui| {
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

                        if ui
                            .button(crate::ui::i18n::text(&locale, "timeline.position"))
                            .clicked()
                        {
                            let track_x = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::PositionX);
                            track_x.add_keyframe(cf, obj_tx, EaseType::EaseInOut);
                            let track_y = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::PositionY);
                            track_y.add_keyframe(cf, obj_ty, EaseType::EaseInOut);
                            ui.close_menu();
                        }

                        if ui
                            .button(crate::ui::i18n::text(&locale, "timeline.rotation"))
                            .clicked()
                        {
                            let track_rot = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::Rotation);
                            track_rot.add_keyframe(cf, obj_rot, EaseType::EaseInOut);
                            ui.close_menu();
                        }

                        if ui
                            .button(crate::ui::i18n::text(&locale, "timeline.scale"))
                            .clicked()
                        {
                            let track_sx = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::ScaleX);
                            track_sx.add_keyframe(cf, obj_sx, EaseType::EaseInOut);
                            let track_sy = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::ScaleY);
                            track_sy.add_keyframe(cf, obj_sy, EaseType::EaseInOut);
                            ui.close_menu();
                        }

                        if ui
                            .button(crate::ui::i18n::text(&locale, "timeline.opacity"))
                            .clicked()
                        {
                            let track_op = state
                                .timeline
                                .add_or_get_track_mut(&sel_id, AnimProperty::Opacity);
                            track_op.add_keyframe(cf, obj_opac, EaseType::Linear);
                            ui.close_menu();
                        }
                    },
                );
            }
        });

        // Track list display
        if !state.timeline.tracks.is_empty() {
            ui.add_space(2.0);
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for track in &state.timeline.tracks {
                        let prop_name = match track.property {
                            AnimProperty::PositionX => {
                                crate::ui::i18n::text(&locale, "timeline.pos_x")
                            }
                            AnimProperty::PositionY => {
                                crate::ui::i18n::text(&locale, "timeline.pos_y")
                            }
                            AnimProperty::Rotation => {
                                crate::ui::i18n::text(&locale, "timeline.rot")
                            }
                            AnimProperty::ScaleX => {
                                crate::ui::i18n::text(&locale, "timeline.scale_x")
                            }
                            AnimProperty::ScaleY => {
                                crate::ui::i18n::text(&locale, "timeline.scale_y")
                            }
                            AnimProperty::Opacity => {
                                crate::ui::i18n::text(&locale, "timeline.opacity")
                            }
                        };
                        let fallback_name =
                            crate::ui::i18n::text(&locale, "timeline.object").into_owned();
                        let obj_name = state
                            .document
                            .all_objects()
                            .find(|(_, o)| o.id == track.object_id)
                            .map(|(_, o)| o.name.as_str())
                            .unwrap_or(&fallback_name);

                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(140.0, 20.0), egui::Sense::hover());
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
