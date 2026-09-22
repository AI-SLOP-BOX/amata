use crate::core::document::{FontStyle, ObjectType, TextAnchor, TextArea, TextStyle};
use crate::core::font::FontRegistry;
use crate::core::history::ModifyTextCommand;
use crate::core::path::{FillStyle, FillType};
use crate::core::state::AppState;
use egui::{Color32, RichText, Stroke, Ui, Vec2};

pub struct TextPanel;

impl TextPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("𝐓 Typography & Text").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("テキストオブジェクトを選択してください").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut current_text = String::new();
        let mut current_style = TextStyle::default();
        let mut current_area: Option<TextArea> = None;
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                if let ObjectType::Text {
                    text: t,
                    style: s,
                    area: a,
                    ..
                } = &obj.object_type
                {
                    current_text = t.clone();
                    current_style = s.clone();
                    current_area = *a;
                    found = true;
                }
                break;
            }
        }

        if !found {
            ui.label(RichText::new("選択中の要素はテキストではありません").weak());
            return;
        }

        let registry = FontRegistry::global();
        let is_avail = registry.is_any_family_available(&current_style.font_family);

        // Missing font warning banner
        if !is_avail {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "⚠️ 未インストール: \"{}\"",
                        current_style.font_family
                    ))
                    .color(Color32::from_rgb(255, 160, 40))
                    .small(),
                );
            });
            ui.label(
                RichText::new("（キャンバスでは代替フォントで描画中。保存・出力時は元のフォント名を保持します）")
                    .weak()
                    .small(),
            );
            ui.add_space(2.0);
        }

        // 1. Text Content
        ui.label("テキスト内容:");
        let mut text_edit = current_text.clone();
        let text_response = ui.text_edit_multiline(&mut text_edit);
        if text_response.lost_focus() && text_edit != current_text {
            let cmd = Box::new(ModifyTextCommand::new(
                id.clone(),
                current_text.clone(),
                current_style.clone(),
                text_edit.clone(),
                current_style.clone(),
            ));
            state.undo_manager.execute(cmd, &mut state.document);
            current_text = text_edit;
        }

        ui.add_space(6.0);

        // 2. Font Family Picker
        ui.label("フォントファミリー:");
        let mut selected_family = current_style.font_family.clone();
        let display_family = if is_avail {
            selected_family.clone()
        } else {
            format!("⚠️ [Missing] {}", selected_family)
        };

        egui::ComboBox::from_id_salt("typography_font_family_combo")
            .selected_text(&display_family)
            .width(220.0)
            .show_ui(ui, |ui| {
                // Popular curated families at the top
                let curated = [
                    "Inter, sans-serif",
                    "LINE Seed JP",
                    "Noto Sans JP",
                    "Hiragino Sans",
                    "Yu Gothic",
                    "Yu Mincho",
                    "Monospace",
                ];
                ui.label(RichText::new("--- おすすめ ---").weak().small());
                for fam in curated {
                    if ui.selectable_label(selected_family == fam, fam).clicked() {
                        selected_family = fam.to_string();
                    }
                }

                ui.separator();
                ui.label(
                    RichText::new("--- システム / 同梱フォント ---")
                        .weak()
                        .small(),
                );

                // Scroll area for all detected system & bundled fonts
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for fam in registry.list_families() {
                            if ui.selectable_label(selected_family == *fam, fam).clicked() {
                                selected_family = fam.clone();
                            }
                        }
                    });
            });

        if selected_family != current_style.font_family {
            let mut new_style = current_style.clone();
            new_style.font_family = selected_family;
            let cmd = Box::new(ModifyTextCommand::new(
                id.clone(),
                current_text.clone(),
                current_style.clone(),
                current_text.clone(),
                new_style.clone(),
            ));
            state.undo_manager.execute(cmd, &mut state.document);
            current_style = new_style;
        }

        ui.add_space(4.0);

        // 3. Font Size & Quick Sizes (drag coalesced to one undo step;
        // previously every drag frame pushed its own ModifyTextCommand).
        ui.horizontal(|ui| {
            ui.label("サイズ:");
            let mut fs = current_style.font_size;
            let fs_resp = ui.add(
                egui::DragValue::new(&mut fs)
                    .speed(1.0)
                    .range(4.0..=500.0)
                    .suffix("pt"),
            );
            if fs_resp.changed() {
                state.object_edit(&id, &fs_resp, |o| {
                    if let ObjectType::Text {
                        style, font_size, ..
                    } = &mut o.object_type
                    {
                        style.font_size = fs;
                        *font_size = fs;
                    }
                });
                current_style.font_size = fs;
            }
            if fs_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        });

        // Quick sizes
        ui.horizontal_wrapped(|ui| {
            for size in [9.0, 11.0, 14.0, 18.0, 24.0, 36.0, 48.0, 72.0, 96.0] {
                let size_resp =
                    ui.selectable_label(current_style.font_size == size, format!("{size}"));
                if size_resp.clicked() {
                    state.object_edit(&id, &size_resp, |o| {
                        if let ObjectType::Text {
                            style, font_size, ..
                        } = &mut o.object_type
                        {
                            style.font_size = size;
                            *font_size = size;
                        }
                    });
                    current_style.font_size = size;
                }
            }
        });

        ui.add_space(6.0);

        // 4. Font Weight & Font Style
        ui.horizontal(|ui| {
            ui.label("ウェイト:");
            let weights = [
                (100, "Thin (100)"),
                (300, "Light (300)"),
                (400, "Regular (400)"),
                (500, "Medium (500)"),
                (600, "SemiBold (600)"),
                (700, "Bold (700)"),
                (800, "ExtraBold (800)"),
                (900, "Black (900)"),
            ];
            let current_weight_label = weights
                .iter()
                .find(|(w, _)| *w == current_style.font_weight)
                .map(|(_, l)| *l)
                .unwrap_or("Custom");

            egui::ComboBox::from_id_salt("typography_font_weight_combo")
                .selected_text(current_weight_label)
                .width(130.0)
                .show_ui(ui, |ui| {
                    for (w, label) in weights {
                        if ui
                            .selectable_label(current_style.font_weight == w, label)
                            .clicked()
                        {
                            let mut new_style = current_style.clone();
                            new_style.font_weight = w;
                            let cmd = Box::new(ModifyTextCommand::new(
                                id.clone(),
                                current_text.clone(),
                                current_style.clone(),
                                current_text.clone(),
                                new_style.clone(),
                            ));
                            state.undo_manager.execute(cmd, &mut state.document);
                            current_style = new_style;
                        }
                    }
                });

            // Italic toggle
            let is_italic = current_style.font_style == FontStyle::Italic;
            if ui.selectable_label(is_italic, "斜体 (I)").clicked() {
                let next_style = if is_italic {
                    FontStyle::Normal
                } else {
                    FontStyle::Italic
                };
                let mut new_style = current_style.clone();
                new_style.font_style = next_style;
                let cmd = Box::new(ModifyTextCommand::new(
                    id.clone(),
                    current_text.clone(),
                    current_style.clone(),
                    current_text.clone(),
                    new_style.clone(),
                ));
                state.undo_manager.execute(cmd, &mut state.document);
                current_style = new_style;
            }
        });

        ui.add_space(6.0);

        // 4b. Variable-font axes (only when the resolved face has an fvar table).
        let axes = registry.variation_axes(
            &current_style.font_family,
            current_style.font_weight,
            current_style.font_style,
        );
        if !axes.is_empty() {
            ui.label(RichText::new("可変フォント軸").strong().size(11.0));
            for axis in axes {
                let current = current_style
                    .variation(&axis.tag)
                    .unwrap_or(axis.default as f64);
                ui.horizontal(|ui| {
                    ui.label(axis.label());
                    let mut val = current;
                    let resp = ui.add(
                        egui::DragValue::new(&mut val)
                            .speed(0.5)
                            .range(axis.min as f64..=axis.max as f64),
                    );
                    if resp.changed() {
                        state.object_edit(&id, &resp, |o| {
                            if let ObjectType::Text { style, .. } = &mut o.object_type {
                                // Reset-to-default drops the entry so documents
                                // stay clean and match the face's own default.
                                if (val - axis.default as f64).abs() < 1e-4 {
                                    style.clear_variation(&axis.tag);
                                } else {
                                    style.set_variation(&axis.tag, val);
                                }
                            }
                        });
                        if (val - axis.default as f64).abs() < 1e-4 {
                            current_style.clear_variation(&axis.tag);
                        } else {
                            current_style.set_variation(&axis.tag, val);
                        }
                    }
                    if resp.drag_stopped() {
                        state.commit_object_edits("Edit Variation Axis");
                    }
                });
            }
            ui.add_space(4.0);
        }

        ui.add_space(6.0);

        // 5. Alignment (TextAnchor)
        ui.label("行揃え (Text Anchor):");
        ui.horizontal(|ui| {
            for (anchor, label) in [
                (TextAnchor::Start, "左揃え (Start)"),
                (TextAnchor::Middle, "中央 (Middle)"),
                (TextAnchor::End, "右揃え (End)"),
            ] {
                if ui
                    .selectable_label(current_style.text_anchor == anchor, label)
                    .clicked()
                {
                    let mut new_style = current_style.clone();
                    new_style.text_anchor = anchor;
                    let cmd = Box::new(ModifyTextCommand::new(
                        id.clone(),
                        current_text.clone(),
                        current_style.clone(),
                        current_text.clone(),
                        new_style.clone(),
                    ));
                    state.undo_manager.execute(cmd, &mut state.document);
                    current_style = new_style;
                }
            }
        });

        ui.add_space(6.0);

        // Writing direction (横/縦) + ligatures
        ui.horizontal(|ui| {
            ui.label("組み:");
            let vert = current_style.vertical;
            if ui.selectable_label(!vert, "横組み").clicked() && vert {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.vertical = false;
                    }
                }
                state.commit_object_edits("Horizontal Text");
                current_style.vertical = false;
            }
            if ui.selectable_label(vert, "縦組み").clicked() && !vert {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.vertical = true;
                    }
                }
                state.commit_object_edits("Vertical Text");
                current_style.vertical = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("合字 (liga):");
            let lig = current_style.ligatures;
            if ui.selectable_label(lig, "ON").clicked() && !lig {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.ligatures = true;
                    }
                }
                state.commit_object_edits("Enable Ligatures");
                current_style.ligatures = true;
            }
            if ui.selectable_label(!lig, "OFF").clicked() && lig {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.ligatures = false;
                    }
                }
                state.commit_object_edits("Disable Ligatures");
                current_style.ligatures = false;
            }
        });

        ui.add_space(6.0);

        // 6. Letter Spacing (字間)
        ui.horizontal(|ui| {
            ui.label("字間 (Letter Spacing):");
            let mut ls = current_style.letter_spacing;
            let ls_resp = ui.add(
                egui::DragValue::new(&mut ls)
                    .speed(0.2)
                    .range(-10.0..=100.0)
                    .suffix("px"),
            );
            if ls_resp.changed() {
                state.object_edit(&id, &ls_resp, |o| {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.letter_spacing = ls;
                    }
                });
                current_style.letter_spacing = ls;
            }
            if ls_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        });

        ui.add_space(6.0);

        // 7. Text Layout (Line Height / Wrap / Max Width)
        ui.label(RichText::new("レイアウト").strong().size(11.0));

        // Line Height
        ui.horizontal(|ui| {
            ui.label("行間:");
            let mut lh = current_style
                .line_height
                .unwrap_or(1.2_f64);
            let lh_resp = ui.add(
                egui::DragValue::new(&mut lh)
                    .speed(0.1)
                    .range(0.5..=5.0)
                    .suffix("em"),
            );
            if lh_resp.changed() {
                state.object_edit(&id, &lh_resp, |o| {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.line_height = if (lh - 1.2).abs() < 0.001 {
                            None
                        } else {
                            Some(lh)
                        };
                    }
                });
            }
            if lh_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        });

        // Word Wrap + Max Width
        let wrap_on = current_style.word_wrap;
        if ui
            .selectable_label(wrap_on, "折り返し (W)")
            .clicked()
        {
            state.ensure_object_snapshot(&id);
            if let Some(o) = state.document.find_object_mut(&id) {
                if let ObjectType::Text { style, .. } = &mut o.object_type {
                    style.word_wrap = !style.word_wrap;
                }
            }
            state.commit_object_edits("Toggle Wrap");
        }

        if current_style.word_wrap {
            let mut mw = current_style.max_width.unwrap_or(0.0);
            let mw_resp = ui.add(
                egui::DragValue::new(&mut mw)
                    .speed(5.0)
                    .range(10.0..=2000.0)
                    .suffix("px"),
            );
            if mw_resp.changed() {
                state.object_edit(&id, &mw_resp, |o| {
                    if let ObjectType::Text { style, .. } = &mut o.object_type {
                        style.max_width = if mw <= 0.0 { None } else { Some(mw) };
                    }
                });
            }
            if mw_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        }

        // Area text (rect container with overflow).
        ui.add_space(4.0);
        ui.separator();
        ui.label(RichText::new("エリアテキスト").strong());
        match current_area {
            None => {
                if ui.button("ポイントテキストをエリア化").clicked() {
                    state.ensure_object_snapshot(&id);
                    if let Some(o) = state.document.find_object_mut(&id) {
                        if let ObjectType::Text {
                            text, style, area, ..
                        } = &mut o.object_type
                        {
                            // Box the measured block so nothing visibly moves:
                            // first baseline stays at y=0.
                            let (w, h) =
                                crate::core::document::object::text_block_size_with_style(
                                    text, style,
                                );
                            *area = Some(TextArea::new(
                                0.0,
                                -style.font_size,
                                w.max(20.0),
                                h.max(style.font_size),
                            ));
                        }
                    }
                    state.commit_object_edits("Convert to Area Text");
                }
            }
            Some(a) => {
                let overflow = crate::core::document::layout_text(
                    &current_text,
                    &current_style,
                    Some(a),
                )
                .overflow();
                if overflow > 0 {
                    ui.label(
                        RichText::new(format!("⚠ {overflow}行あふれています"))
                            .color(Color32::from_rgb(255, 160, 40)),
                    );
                } else {
                    ui.label(RichText::new("ボックスに収まっています").weak().size(11.0));
                }
                let mut box_vals = [a.x, a.y, a.width, a.height];
                let labels = ["X", "Y", "W", "H"];
                let mut stopped = false;
                ui.horizontal(|ui| {
                    for (i, lab) in labels.iter().enumerate() {
                        ui.label(*lab);
                        let resp = ui.add(
                            egui::DragValue::new(&mut box_vals[i])
                                .speed(1.0)
                                .range(if i < 2 { -5000.0..=5000.0 } else { 1.0..=5000.0 }),
                        );
                        if resp.changed() {
                            state.object_edit(&id, &resp, |o| {
                                if let ObjectType::Text { area, .. } = &mut o.object_type {
                                    *area = Some(TextArea::new(
                                        box_vals[0],
                                        box_vals[1],
                                        box_vals[2],
                                        box_vals[3],
                                    ));
                                }
                            });
                        }
                        stopped |= resp.drag_stopped();
                    }
                });
                if stopped {
                    // Drags coalesce into one undo step on release.
                    state.commit_object_edits("Edit Text Area");
                }
                if ui.button("エリアを解除（ポイントに戻す）").clicked() {
                    state.ensure_object_snapshot(&id);
                    if let Some(o) = state.document.find_object_mut(&id) {
                        if let ObjectType::Text { area, .. } = &mut o.object_type {
                            *area = None;
                        }
                    }
                    state.commit_object_edits("Release Area Text");
                }
                // Thread (linked frames): select exactly two area texts, link
                // the first's overflow to the second.
                if state.selected_ids.len() >= 2 {
                    let a_id = state.selected_ids[0].clone();
                    let b_id = state.selected_ids[1].clone();
                    if ui.button("次のエリアにテキストを流し込む（スレッド）").clicked() {
                        let mut ok = false;
                        if let (Some(oa), Some(ob)) = (
                            state.document.find_object(&a_id),
                            state.document.find_object(&b_id),
                        ) {
                            let is_area = |o: &crate::core::document::Object| {
                                matches!(
                                    o.object_type,
                                    ObjectType::Text {
                                        area: Some(_),
                                        ..
                                    }
                                )
                            };
                            ok = is_area(oa) && is_area(ob) && oa.id != ob.id;
                        }
                        if ok {
                            state.ensure_object_snapshot(&a_id);
                            if let Some(o) = state.document.find_object_mut(&a_id) {
                                if let ObjectType::Text { next_frame, .. } = &mut o.object_type {
                                    *next_frame = Some(b_id.clone());
                                }
                            }
                            state.commit_object_edits("Link Text Thread");
                        }
                    }
                    if ui.button("スレッド解除").clicked() {
                        state.ensure_object_snapshot(&a_id);
                        if let Some(o) = state.document.find_object_mut(&a_id) {
                            if let ObjectType::Text { next_frame, .. } = &mut o.object_type {
                                *next_frame = None;
                            }
                        }
                        state.commit_object_edits("Unlink Text Thread");
                    }
                }
            }
        }
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
        // Profiles live on the object now (persisted + undoable), not in a
        // throwaway session map.
        let mut width_profile = state
            .document
            .find_object(&id)
            .and_then(|o| {
                if o.stroke.is_some() {
                    Some(
                        o.width_profile
                            .clone()
                            .unwrap_or_default(),
                    )
                } else {
                    None
                }
            });
        let Some(ref mut profile) = width_profile else {
            ui.label(RichText::new("Object has no stroke").weak());
            return;
        };

        ui.label("Add width points along the stroke path:");
        ui.add_space(4.0);

        // Any widget interaction snapshots once; drags commit on stop so a
        // single gesture is a single undo step.
        let mut prof_changed = false;
        let mut prof_dragging = false;
        let mut prof_stopped = false;
        let mut to_remove = None;
        let point_count = profile.points.len();
        for i in 0..point_count {
            let mut pos = profile.points[i].position;
            let mut width = profile.points[i].width;
            ui.horizontal(|ui| {
                ui.label(format!("{:.0}%", pos * 100.0));
                let pos_resp = ui.add(
                    egui::Slider::new(&mut pos, 0.0..=1.0)
                        .show_value(false)
                        .step_by(0.01),
                );
                if pos_resp.changed() {
                    profile.points[i].position = pos;
                    prof_changed = true;
                }
                if pos_resp.dragged() {
                    prof_dragging = true;
                }
                if pos_resp.drag_stopped() {
                    prof_stopped = true;
                }
                let w_resp = ui.add(
                    egui::DragValue::new(&mut width)
                        .speed(0.1)
                        .range(0.01..=10.0)
                        .suffix("x"),
                );
                if w_resp.changed() {
                    profile.points[i].width = width;
                    prof_changed = true;
                }
                if w_resp.dragged() {
                    prof_dragging = true;
                }
                if w_resp.drag_stopped() {
                    prof_stopped = true;
                }
                if point_count > 2 && ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(idx) = to_remove {
            profile.points.remove(idx);
            prof_changed = true;
        }

        if ui.button("+ Add Width Point").clicked() {
            let last_pos = profile.points.last().map(|p| p.position).unwrap_or(0.5);
            profile.points.push(crate::core::document::WidthPoint {
                position: (last_pos + 0.5).min(1.0),
                width: 1.0,
                side: crate::core::document::WidthSide::Both,
            });
            profile.points.sort_by(|a, b| {
                // total_cmp never panics on NaN positions.
                a.position.total_cmp(&b.position)
            });
            prof_changed = true;
        }

        // Reset profile
        if ui.button("Reset Profile").clicked() {
            *profile = crate::core::document::WidthProfile::default();
            prof_changed = true;
        }

        if prof_changed {
            state.ensure_object_snapshot(&id);
            if let Some(o) = state.document.find_object_mut(&id) {
                o.width_profile = Some(profile.clone());
            }
            if !prof_dragging {
                state.commit_object_edits("Edit Width Profile");
            }
        }
        if prof_stopped {
            state.commit_object_edits("Edit Width Profile");
        }
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
                overprint: false,
                spot: None,
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
                        overprint: false,
                        spot: None,
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
