//! Brush panel: calligraphy / art / pattern / bristle brushes applied as
//! baked outlines (fully undoable). Custom artwork comes from the
//! selection; named brushes persist in a JSON library.

use crate::core::brush::{apply_bristle, builtin_motif, apply_brush, BrushDefinition, BrushKind};
use crate::core::document::{Object, ObjectType};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct BrushPanel;

fn library_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".config").join("amata").join("brushes.json"))
}

fn load_library() -> Vec<BrushDefinition> {
    library_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_library(lib: &[BrushDefinition]) {
    if let Some(p) = library_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(lib) {
            let _ = std::fs::write(p, json);
        }
    }
}

fn artwork_for(state: &AppState) -> Option<crate::core::path::PathData> {
    if state.brush_custom_art {
        if let Some(art) = &state.brush_artwork {
            if !art.elements.is_empty() {
                return Some(art.clone());
            }
        }
    }
    builtin_motif(&state.brush_motif)
}

fn current_def(state: &AppState) -> Option<BrushDefinition> {
    match state.brush_kind_idx {
        0 => Some(BrushDefinition::calligraphy(
            state.brush_angle,
            state.brush_roundness / 100.0,
            state.brush_size,
        )),
        1 => {
            let artwork = artwork_for(state)?;
            Some(BrushDefinition {
                name: if state.brush_custom_art {
                    "Art (custom)".to_string()
                } else {
                    format!("Art ({})", state.brush_motif)
                },
                kind: BrushKind::Art { artwork },
            })
        }
        2 => {
            let artwork = artwork_for(state)?;
            Some(BrushDefinition {
                name: if state.brush_custom_art {
                    "Pattern (custom)".to_string()
                } else {
                    format!("Pattern ({})", state.brush_motif)
                },
                kind: BrushKind::Pattern {
                    artwork,
                    spacing: state.brush_spacing.max(1.0),
                    scale: state.brush_scale.clamp(0.1, 10.0),
                },
            })
        }
        _ => Some(BrushDefinition {
            name: "Bristle".to_string(),
            kind: BrushKind::Bristle {
                count: (state.brush_bristles as usize).clamp(1, 64),
                scatter: state.brush_scatter.clamp(0.0, 1.0),
                size: state.brush_size.max(0.5),
                opacity: state.brush_opacity.clamp(0.05, 1.0),
            },
        }),
    }
}

/// Push a brush definition's params back into panel state (library apply).
fn adopt_def(state: &mut AppState, def: &BrushDefinition) {
    match &def.kind {
        BrushKind::Calligraphy { angle_deg, roundness, size } => {
            state.brush_kind_idx = 0;
            state.brush_angle = *angle_deg;
            state.brush_roundness = roundness * 100.0;
            state.brush_size = *size;
        }
        BrushKind::Art { artwork } => {
            state.brush_kind_idx = 1;
            state.brush_artwork = Some(artwork.clone());
            state.brush_custom_art = true;
        }
        BrushKind::Pattern { artwork, spacing, scale } => {
            state.brush_kind_idx = 2;
            state.brush_artwork = Some(artwork.clone());
            state.brush_custom_art = true;
            state.brush_spacing = *spacing;
            state.brush_scale = *scale;
        }
        BrushKind::Bristle { count, scatter, size, opacity } => {
            state.brush_kind_idx = 3;
            state.brush_bristles = *count as f64;
            state.brush_scatter = *scatter;
            state.brush_size = *size;
            state.brush_opacity = *opacity;
        }
    }
    state.brush_lib_name = def.name.clone();
}

impl BrushPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖌️ Brushes").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for (i, label) in ["Calligraphy", "Art", "Pattern", "Bristle"].iter().enumerate() {
                if ui
                    .selectable_label(state.brush_kind_idx == i, *label)
                    .clicked()
                {
                    state.brush_kind_idx = i;
                }
            }
        });
        ui.add_space(4.0);

        match state.brush_kind_idx {
            0 => {
                ui.horizontal(|ui| {
                    ui.label("Angle:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_angle)
                            .range(-90.0..=90.0)
                            .suffix("°"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Roundness:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_roundness)
                            .range(5.0..=100.0)
                            .suffix("%"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_size)
                            .range(1.0..=200.0)
                            .suffix("pt"),
                    );
                });
            }
            _ if state.brush_kind_idx == 3 => {
                ui.horizontal(|ui| {
                    ui.label("Bristles:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_bristles)
                            .range(1.0..=64.0),
                    );
                    ui.label("Scatter:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_scatter)
                            .range(0.0..=1.0)
                            .speed(0.01),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_size)
                            .range(1.0..=200.0)
                            .suffix("pt"),
                    );
                    ui.label("Opacity:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_opacity)
                            .range(0.05..=1.0)
                            .speed(0.01),
                    );
                });
                ui.label(RichText::new("乾いた筆の筋になります。").weak().size(11.0));
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    if ui
                        .selectable_label(!state.brush_custom_art, "Builtin")
                        .clicked()
                    {
                        state.brush_custom_art = false;
                    }
                    if ui
                        .selectable_label(state.brush_custom_art, "Selection")
                        .on_hover_text("選択中の図形をモチーフに使います")
                        .clicked()
                    {
                        // Capture the selection as artwork (local geometry).
                        if let Some(id) = state.selected_ids.first().cloned() {
                            if let Some(obj) = state.document.find_object(&id).cloned() {
                                let mut art = obj.to_path_data();
                                art.transform(&obj.transform.matrix());
                                if !art.elements.is_empty() {
                                    state.brush_artwork = Some(art);
                                    state.brush_custom_art = true;
                                    state.notify_success("選択をブラシモチーフに登録しました");
                                } else {
                                    state.notify_error("モチーフにできる図形がありません");
                                }
                            }
                        } else {
                            state.notify_error("先にモチーフを選んでください");
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Motif:");
                    for motif in ["arrow", "leaf", "wave"] {
                        if ui
                            .selectable_label(
                                !state.brush_custom_art && state.brush_motif == motif,
                                motif,
                            )
                            .clicked()
                        {
                            state.brush_motif = motif.to_string();
                            state.brush_custom_art = false;
                        }
                    }
                });
                if state.brush_kind_idx == 1 {
                    ui.label(RichText::new("Motif height := path stroke width.").weak().size(11.0));
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Spacing:");
                        ui.add(
                            egui::DragValue::new(&mut state.brush_spacing)
                                .range(1.0..=500.0),
                        );
                        ui.label("Scale:");
                        ui.add(
                            egui::DragValue::new(&mut state.brush_scale)
                                .range(0.1..=10.0),
                        );
                    });
                }
            }
        }
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();
        if ui
            .add_enabled(has_sel, egui::Button::new("Apply Brush to Selection"))
            .clicked()
        {
            if let Some(def) = current_def(state) {
                let targets: Vec<Object> = state
                    .selected_ids
                    .iter()
                    .filter_map(|id| {
                        state
                            .document
                            .all_objects()
                            .find(|(_, o)| &o.id == id)
                            .map(|(_, o)| o.clone())
                    })
                    .collect();
                let mut applied = 0;
                for obj in targets {
                    // Brushing a text/image bbox is never what the user
                    // wants — those need outlining/rasterizing first.
                    if matches!(
                        obj.object_type,
                        ObjectType::Text { .. }
                            | ObjectType::Image { .. }
                            | ObjectType::PixelArt(_)
                    ) {
                        continue;
                    }
                    let spine = obj.to_path_data();
                    if spine.elements.is_empty() {
                        continue;
                    }
                    let stroke_w = obj.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                    let base_fill = obj.fill.clone().or_else(|| {
                        Some(crate::core::path::FillStyle::solid(state.fill_color))
                    });
                    if let BrushKind::Bristle { count, scatter, size, opacity } = def.kind {
                        // One group of translucent streaks per spine.
                        let streaks = apply_bristle(&spine, count, scatter, size, opacity);
                        if streaks.is_empty() {
                            continue;
                        }
                        let mut children = Vec::new();
                        for (i, (mut band, alpha)) in streaks.into_iter().enumerate() {
                            if let Some(mut f) = base_fill.clone() {
                                f.color[3] *= alpha;
                                if let crate::core::path::FillType::Solid(c) = &mut f.fill_type {
                                    c[3] *= alpha;
                                }
                                band.fill = Some(f);
                            }
                            let mut child = Object::new_path(
                                &format!("{} (Bristle {i})", obj.name),
                                band,
                            );
                            child.transform = obj.transform.clone();
                            children.push(child);
                        }
                        let mut group =
                            Object::new_group(&format!("{} (Brush)", obj.name), children);
                        group.transform = crate::core::document::Transform::default();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(group));
                        state.undo_manager.execute(cmd, &mut state.document);
                        applied += 1;
                    } else if let Some(mut brushed) = apply_brush(&spine, &def, stroke_w) {
                        brushed.fill = base_fill;
                        let mut new_obj =
                            Object::new_path(&format!("{} (Brush)", obj.name), brushed);
                        new_obj.transform = obj.transform.clone();
                        let cmd =
                            Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        applied += 1;
                    }
                }
                if applied > 0 {
                    state.notify_success(format!("ブラシを適用しました（{applied}件）"));
                } else {
                    state.notify_error("適用できるパスがありません");
                }
            }
        }
        if !has_sel {
            ui.label(RichText::new("Select a path or shape first.").weak().size(11.0));
        }

        // Brush library (persisted JSON).
        ui.add_space(4.0);
        ui.separator();
        ui.label(RichText::new("ブラシライブラリ").strong());
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.brush_lib_name);
            if ui.button("保存").clicked() {
                if let Some(mut def) = current_def(state) {
                    let name = state.brush_lib_name.trim();
                    def.name = if name.is_empty() { "Brush".to_string() } else { name.to_string() };
                    let mut lib = load_library();
                    if let Some(pos) = lib.iter().position(|b| b.name == def.name) {
                        lib[pos] = def;
                    } else {
                        lib.push(def);
                    }
                    save_library(&lib);
                    state.notify_success("ブラシを保存しました");
                }
            }
        });
        let mut adopt: Option<BrushDefinition> = None;
        let mut remove: Option<String> = None;
        for b in load_library() {
            ui.horizontal(|ui| {
                ui.label(&b.name);
                if ui.small_button("適用").clicked() {
                    adopt = Some(b.clone());
                }
                if ui.small_button("✕").clicked() {
                    remove = Some(b.name.clone());
                }
            });
        }
        if let Some(def) = adopt {
            adopt_def(state, &def);
        }
        if let Some(name) = remove {
            let mut lib = load_library();
            lib.retain(|b| b.name != name);
            save_library(&lib);
        }
    }
}
