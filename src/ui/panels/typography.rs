use crate::core::document::ObjectType;
use crate::core::path::{FillStyle, FillType};
use crate::core::state::AppState;
use egui::{Color32, RichText, Stroke, Ui, Vec2};

pub struct TextPanel;

impl TextPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("𝐓 Text").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select a text object").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut text = String::new();
        let mut font_size = 32.0;
        let mut font_family = String::from("Sans-Serif");
        let mut align = TextAlignment::Left;
        let mut line_height = 1.2f64;
        let mut letter_spacing = 0.0f64;
        let mut word_spacing = 0.0f64;
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                if let ObjectType::Text {
                    text: t,
                    font_size: fs,
                } = &obj.object_type
                {
                    text = t.clone();
                    font_size = *fs;
                    found = true;
                }
                break;
            }
        }

        if !found {
            ui.label(RichText::new("Not a text object").weak());
            return;
        }

        // Text content
        ui.label("Content:");
        ui.text_edit_multiline(&mut text);

        ui.add_space(4.0);

        // Font size
        ui.horizontal(|ui| {
            ui.label("Size:");
            let mut fs = font_size;
            if ui
                .add(
                    egui::DragValue::new(&mut fs)
                        .speed(1.0)
                        .range(6.0..=500.0)
                        .suffix("pt"),
                )
                .changed()
            {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let ObjectType::Text {
                            font_size: ref mut s,
                            ..
                        } = obj.object_type
                        {
                            *s = fs;
                        }
                    }
                }
            }
        });

        // Font family — Professional Adobe Typography Font Menu
        ui.horizontal(|ui| {
            ui.label("フォント:");
            egui::ComboBox::from_id_salt("font_family")
                .selected_text(&font_family)
                .width(160.0)
                .show_ui(ui, |ui| {
                    let font_choices = [
                        ("Noto Sans JP (ゴシック)", "Noto Sans JP"),
                        ("Hiragino Sans (ヒラギノ角ゴ)", "Hiragino Sans"),
                        ("Inter (モダン欧文サンセリフ)", "Inter"),
                        ("Yu Mincho (游明朝 / セリフ)", "Yu Mincho"),
                        ("Monospace (等幅コード)", "Monospace"),
                        ("Cursive (筆記体)", "Cursive"),
                    ];
                    for (display_name, val) in font_choices {
                        if ui
                            .selectable_label(font_family == val, display_name)
                            .clicked()
                        {
                            font_family = val.into();
                        }
                    }
                });
        });

        ui.add_space(4.0);

        // Alignment
        ui.label("行揃え:");
        ui.horizontal(|ui| {
            for (a, label) in [
                (TextAlignment::Left, "左揃え"),
                (TextAlignment::Center, "中央揃え"),
                (TextAlignment::Right, "右揃え"),
                (TextAlignment::Justify, "両端揃え"),
            ] {
                if ui.selectable_label(align == a, label).clicked() {
                    align = a;
                }
            }
        });

        ui.add_space(4.0);

        // Line height
        ui.horizontal(|ui| {
            ui.label("Line Height:");
            let mut lh = line_height;
            if ui
                .add(
                    egui::Slider::new(&mut lh, 0.5..=3.0)
                        .show_value(true)
                        .step_by(0.1),
                )
                .changed()
            {
                line_height = lh;
            }
        });

        // Letter spacing
        ui.horizontal(|ui| {
            ui.label("Letter Spacing:");
            let mut ls = letter_spacing;
            if ui
                .add(egui::DragValue::new(&mut ls).speed(0.5).range(-10.0..=50.0))
                .changed()
            {
                letter_spacing = ls;
            }
        });

        // Word spacing
        ui.horizontal(|ui| {
            ui.label("Word Spacing:");
            let mut ws = word_spacing;
            if ui
                .add(
                    egui::DragValue::new(&mut ws)
                        .speed(1.0)
                        .range(-10.0..=100.0),
                )
                .changed()
            {
                word_spacing = ws;
            }
        });

        ui.add_space(4.0);

        // Quick sizes
        ui.label("Quick Sizes:");
        ui.horizontal_wrapped(|ui| {
            for size in [
                9.0, 10.0, 11.0, 12.0, 14.0, 18.0, 24.0, 36.0, 48.0, 60.0, 72.0, 96.0,
            ] {
                if ui
                    .selectable_label(font_size == size, format!("{}", size))
                    .clicked()
                {
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            if let ObjectType::Text {
                                font_size: ref mut s,
                                ..
                            } = obj.object_type
                            {
                                *s = size;
                            }
                        }
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justify,
}

// ═══════════════════════════════════════════════════════════════════
// SmartGuidesPanel: Snapping and guides
// ═══════════════════════════════════════════════════════════════════

pub struct SymbolsPanel;

impl SymbolsPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("⭐ Symbols").strong());
        ui.add_space(4.0);

        // Save selected as symbol
        let has_sel = !state.selected_ids.is_empty();
        if ui
            .add_enabled(has_sel, egui::Button::new("Save Selection as Symbol"))
            .clicked()
        {
            if let Some(id) = state.selected_ids.first() {
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                    let sym = crate::core::document::Symbol::new(
                        &format!("Symbol {}", state.symbols.len() + 1),
                        obj.clone(),
                    );
                    state.symbols.push(sym);
                }
            }
        }

        ui.add_space(4.0);
        ui.separator();

        // Symbol library
        if state.symbols.is_empty() {
            ui.label(RichText::new("登録されたアセット・シンボルはありません").weak());
            return;
        }

        ui.label(
            RichText::new(format!(
                "アセットライブラリ ({} 個のプリセット)",
                state.symbols.len()
            ))
            .strong()
            .color(Color32::WHITE),
        );
        ui.add_space(4.0);

        let mut to_remove = None;
        for (i, sym) in state.symbols.iter().enumerate() {
            ui.horizontal(|ui| {
                // Color preview indicator thumbnail
                let col = sym
                    .object
                    .fill
                    .as_ref()
                    .map(|f| f.color)
                    .unwrap_or([0.2, 0.6, 0.9, 1.0]);
                let c32 = Color32::from_rgba_unmultiplied(
                    (col[0] * 255.0) as u8,
                    (col[1] * 255.0) as u8,
                    (col[2] * 255.0) as u8,
                    255,
                );
                let (rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 3.0, Color32::from_rgb(32, 32, 34));
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 65)),
                    egui::StrokeKind::Inside,
                );
                ui.painter().circle_filled(rect.center(), 7.0, c32);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&sym.name)
                            .strong()
                            .size(11.0)
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        RichText::new(format!("配置回数: {} 回", sym.use_count))
                            .weak()
                            .size(9.5),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").on_hover_text("削除").clicked() {
                        to_remove = Some(i);
                    }
                    if ui
                        .button(RichText::new("＋ 配置").size(10.5))
                        .on_hover_text("キャンバスに配置")
                        .clicked()
                    {
                        let mut new_obj = sym.object.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} (Instance)", sym.name);
                        new_obj.transform.x = state.document.width * 0.5;
                        new_obj.transform.y = state.document.height * 0.5;
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                });
            });
            ui.add_space(2.0);
        }

        if let Some(idx) = to_remove {
            state.symbols.remove(idx);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// WidthToolPanel: Variable stroke width
// ═══════════════════════════════════════════════════════════════════

pub struct WidthToolPanel;

impl WidthToolPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("〰 Width Tool").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select a stroked object").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut width_profile = crate::core::document::WidthProfile::default();
        let mut has_stroke = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id && obj.stroke.is_some() {
                has_stroke = true;
                width_profile = state.width_profiles.get(&id).cloned().unwrap_or_default();
                break;
            }
        }

        if !has_stroke {
            ui.label(RichText::new("Object has no stroke").weak());
            return;
        }

        ui.label("Add width points along the stroke path:");
        ui.add_space(4.0);

        let mut to_remove = None;
        let point_count = width_profile.points.len();
        for i in 0..point_count {
            let mut pos = width_profile.points[i].position;
            let mut width = width_profile.points[i].width;
            ui.horizontal(|ui| {
                ui.label(format!("{:.0}%", pos * 100.0));
                if ui
                    .add(
                        egui::Slider::new(&mut pos, 0.0..=1.0)
                            .show_value(false)
                            .step_by(0.01),
                    )
                    .changed()
                {
                    width_profile.points[i].position = pos;
                }
                if ui
                    .add(
                        egui::DragValue::new(&mut width)
                            .speed(0.1)
                            .range(0.01..=10.0)
                            .suffix("x"),
                    )
                    .changed()
                {
                    width_profile.points[i].width = width;
                }
                if point_count > 2 && ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(idx) = to_remove {
            width_profile.points.remove(idx);
        }

        if ui.button("+ Add Width Point").clicked() {
            let last_pos = width_profile
                .points
                .last()
                .map(|p| p.position)
                .unwrap_or(0.5);
            width_profile
                .points
                .push(crate::core::document::WidthPoint {
                    position: (last_pos + 0.5).min(1.0),
                    width: 1.0,
                    side: crate::core::document::WidthSide::Both,
                });
            width_profile
                .points
                .sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
        }

        // Reset profile
        if ui.button("Reset Profile").clicked() {
            width_profile = crate::core::document::WidthProfile::default();
        }

        state.width_profiles.insert(id, width_profile);
    }
}

// ═══════════════════════════════════════════════════════════════════
// PatternPanel: Repeating pattern fills
// ═══════════════════════════════════════════════════════════════════

pub struct PatternPanel;

impl PatternPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🔲 Pattern Fill").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to apply pattern").weak());
            return;
        }

        let id = state.selected_ids[0].clone();

        // Pattern type selector
        ui.label("Pattern Type:");
        let mut pattern = crate::core::path::PatternFill::default();

        ui.horizontal_wrapped(|ui| {
            for (ptype, label) in [
                (crate::core::path::PatternType::Grid, "Grid"),
                (crate::core::path::PatternType::Hex, "Hex"),
                (crate::core::path::PatternType::Brick, "Brick"),
                (crate::core::path::PatternType::Dots, "Dots"),
            ] {
                if ui
                    .selectable_label(pattern.pattern_type == ptype, label)
                    .clicked()
                {
                    pattern.pattern_type = ptype;
                }
            }
        });

        ui.add_space(4.0);

        // Tile size
        ui.horizontal(|ui| {
            ui.label("Tile W:");
            ui.add(
                egui::DragValue::new(&mut pattern.tile_width)
                    .speed(1.0)
                    .range(5.0..=500.0),
            );
            ui.label("H:");
            ui.add(
                egui::DragValue::new(&mut pattern.tile_height)
                    .speed(1.0)
                    .range(5.0..=500.0),
            );
        });

        // Scale
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(egui::Slider::new(&mut pattern.scale, 0.1..=5.0).show_value(true));
        });

        // Rotation
        ui.horizontal(|ui| {
            ui.label("Rotation:");
            ui.add(
                egui::DragValue::new(&mut pattern.rotation)
                    .speed(1.0)
                    .range(-180.0..=180.0)
                    .suffix("°"),
            );
        });

        ui.add_space(4.0);

        // Apply pattern
        if ui.button("Apply Pattern").clicked() {
            let fill = FillStyle {
                color: state.fill_color,
                fill_type: FillType::Pattern(pattern.clone()),
                rule: crate::core::path::FillRule::NonZero,
            };
            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    obj.fill = Some(fill.clone());
                }
            }
        }

        // Preset patterns
        ui.separator();
        ui.label(RichText::new("Presets").strong().size(11.0));
        ui.horizontal_wrapped(|ui| {
            for (name, pw, ph, ptype) in [
                (
                    "Small Grid",
                    20.0,
                    20.0,
                    crate::core::path::PatternType::Grid,
                ),
                (
                    "Large Grid",
                    60.0,
                    60.0,
                    crate::core::path::PatternType::Grid,
                ),
                ("Hex Small", 25.0, 25.0, crate::core::path::PatternType::Hex),
                (
                    "Dots Small",
                    30.0,
                    30.0,
                    crate::core::path::PatternType::Dots,
                ),
                ("Brick", 50.0, 25.0, crate::core::path::PatternType::Brick),
            ] {
                if ui.selectable_label(false, name).clicked() {
                    let p = crate::core::path::PatternFill {
                        pattern_type: ptype,
                        tile_width: pw,
                        tile_height: ph,
                        scale: 1.0,
                        ..Default::default()
                    };
                    let fill = FillStyle {
                        color: state.fill_color,
                        fill_type: FillType::Pattern(p),
                        rule: crate::core::path::FillRule::NonZero,
                    };
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            obj.fill = Some(fill.clone());
                        }
                    }
                }
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// TextPanel: Font/Size/Alignment/Spacing
// ═══════════════════════════════════════════════════════════════════
