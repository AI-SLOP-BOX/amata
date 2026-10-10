use crate::core::state::AppState;
use egui::{self, Color32, RichText, Ui, Vec2};

pub struct ComponentPanel;

impl ComponentPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "components.title")).strong());
        ui.label(
            RichText::new(crate::ui::i18n::text(&locale, "components.description"))
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        // 1. Create Component from selection
        ui.group(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "components.create_master"))
                    .strong()
                    .size(11.5),
            );
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "components.define_selected"))
                    .weak()
                    .size(10.5),
            );
            ui.add_space(2.0);

            if ui
                .add_enabled(
                    has_sel,
                    egui::Button::new(crate::ui::i18n::text(
                        &locale,
                        "components.create_from_selection",
                    )),
                )
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
                    state.notify_info(crate::ui::i18n::format(
                        &locale,
                        "components.created",
                        &[("id", &symbol_id)],
                    ));
                }
            }
        });

        ui.add_space(6.0);

        // 2. Component library & instantiation
        let symbol_count = state.document.symbols.len();
        ui.label(
            RichText::new(crate::ui::i18n::format(
                &locale,
                "components.master_list",
                &[("count", &symbol_count.to_string())],
            ))
            .strong(),
        );
        ui.separator();

        if symbol_count == 0 {
            ui.label(RichText::new(crate::ui::i18n::text(&locale, "components.empty")).weak());
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
                                "◆",
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
                                    if ui
                                        .small_button("×")
                                        .on_hover_text(crate::ui::i18n::text(
                                            &locale,
                                            "components.delete_master",
                                        ))
                                        .clicked()
                                    {
                                        to_remove_id = Some(sym.id.clone());
                                    }
                                    if ui
                                        .button(
                                            RichText::new(crate::ui::i18n::text(
                                                &locale,
                                                "components.place_instance",
                                            ))
                                            .size(11.0),
                                        )
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
            state.notify_info(crate::ui::i18n::text(&locale, "components.deleted").into_owned());
        }

        if let Some(id) = to_instantiate {
            let cx = state.document.width * 0.5;
            let cy = state.document.height * 0.5;
            if let Some(instance) = state.document.instantiate_component(&id, cx, cy) {
                let inst_id = instance.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(instance));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![inst_id];
                state.notify_info(crate::ui::i18n::format(
                    &locale,
                    "components.instance_placed",
                    &[("id", &id)],
                ));
            }
        }
    }
}
