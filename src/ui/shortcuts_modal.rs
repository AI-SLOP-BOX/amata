use crate::ui::panels::ShortcutsHelpPanel;
use eframe::egui::{self, Color32, RichText, Vec2};

/// Modal wrapping [`ShortcutsHelpPanel`] so Cmd+/ and the Help menu can
/// show the full cheat sheet (previously they only opened About, which
/// lists five rows).
#[derive(Default)]
pub struct ShortcutsModal {
    pub is_open: bool,
}

impl ShortcutsModal {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut crate::core::state::AppState) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        let mut close_clicked = false;

        let screen = ctx.screen_rect();
        let (size, min_size, pos) = crate::ui::window_defaults(
            screen,
            Vec2::new(460.0, 560.0),
            Vec2::new(320.0, 300.0),
        );
        egui::Window::new("キーボードショートカット")
            .open(&mut is_open)
            .resizable(true)
            .collapsible(false)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                crate::ui::modal_body(ui, "shortcuts_body", &mut |ui, is_body| {
                    if !is_body {
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("閉じる").strong().color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(96.0, 28.0)),
                                )
                                .clicked()
                            {
                                close_clicked = true;
                            }
                        });
                        return;
                    }
                    ui.set_width(total_w);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ShortcutsHelpPanel::show(ui, state);
                        });
                });
            });

        if close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
    }
}
