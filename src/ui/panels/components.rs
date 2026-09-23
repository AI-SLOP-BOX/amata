use crate::core::state::AppState;
use egui::{self, Color32, RichText, Ui, Vec2};

pub struct ComponentPanel;

impl ComponentPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("❖ コンポーネント (Component / Symbol)").strong());
        ui.label(
            RichText::new("SVG <symbol> / <use> によるマスター再利用システム")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        // 1. Create Component from selection
        ui.group(|ui| {
            ui.label(RichText::new("マスター作成").strong().size(11.5));
            ui.label(
                RichText::new("選択した図形をSVG <symbol> コンポーネントとして定義")
                    .weak()
                    .size(10.5),
            );
            ui.add_space(2.0);

            if ui
                .add_enabled(has_sel, egui::Button::new("❖ 選択からコンポーネントを作成"))
                .clicked()
            {
                // Snapshot selected objects first: create_component removes
                // them from the layers, and we need one undo step that
                // restores both the objects and the symbols list.
                let located = crate::core::history::collect_located_objects(
                    &state.document,
                    &state.selected_ids,
                );
                let symbols_before = state.document.symbols.clone();
                if let Some(symbol_id) = state
                    .document
                    .create_component_from_selection(&state.selected_ids)
                {
                    state.undo_manager.execute(
                        Box::new(crate::core::history::ComponentCommand::create(
                            "Create Component",
                            located,
                            symbols_before,
                            state.document.symbols.clone(),
                            state.document.find_object(&symbol_id).cloned(),
                        )),
                        &mut state.document,
                    );
                    state.selected_ids.clear();
                    state.notify_info(format!(
                        "コンポーネント <symbol id=\"{}\"> を作成しました",
                        symbol_id
                    ));
                }
            }
        });

        ui.add_space(6.0);

        // 2. Component library & instantiation
        let symbol_count = state.document.symbols.len();
        ui.label(RichText::new(format!("マスター一覧 ({} 件)", symbol_count)).strong());
        ui.separator();

        if symbol_count == 0 {
            ui.label(RichText::new("定義されたコンポーネントはありません").weak());
            return;
        }

        let mut to_instantiate: Option<String> = None;
        let mut to_remove_id: Option<String> = None;

        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for sym in &state.document.symbols {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::hover());
                            ui.painter()
                                .rect_filled(rect, 3.0, Color32::from_rgb(32, 34, 42));
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "❖",
                                egui::FontId::proportional(12.0),
                                Color32::from_rgb(100, 180, 255),
                            );

                            ui.vertical(|ui| {
                                ui.label(RichText::new(&sym.name).strong().size(11.5));
                                ui.label(
                                    RichText::new(format!("<symbol id=\"{}\">", sym.id))
                                        .weak()
                                        .size(10.0),
                                );
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").on_hover_text("マスター削除").clicked()
                                    {
                                        to_remove_id = Some(sym.id.clone());
                                    }
                                    if ui
                                        .button(RichText::new("＋ インスタンス配置").size(11.0))
                                        .clicked()
                                    {
                                        to_instantiate = Some(sym.id.clone());
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(2.0);
                }
            });

        if let Some(id) = to_remove_id {
            let before = state.document.symbols.clone();
            state.document.symbols.retain(|s| s.id != id);
            let after = state.document.symbols.clone();
            let cmd = Box::new(crate::core::history::SetSymbolsCommand::new(
                "Delete Component Master",
                before,
                after,
            ));
            state.undo_manager.execute(cmd, &mut state.document);
            state.notify_info("コンポーネント定義を削除しました");
        }

        if let Some(id) = to_instantiate {
            let cx = state.document.width * 0.5;
            let cy = state.document.height * 0.5;
            if let Some(instance) = state.document.instantiate_component(&id, cx, cy) {
                let inst_id = instance.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(instance));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![inst_id];
                state.notify_info(format!("<use href=\"#{}\"> インスタンスを配置しました", id));
            }
        }
    }
}
