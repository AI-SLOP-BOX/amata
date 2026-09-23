use eframe::egui;

/// Clamp a desired modal size/position so it always fits inside `screen`.
///
/// Returns `(default_size, min_size, default_pos)`. Both sizes are clamped
/// to the available screen area so a small host window can still show the
/// dialog without spilling off-screen.
pub fn window_defaults(
    screen: egui::Rect,
    desired: egui::Vec2,
    min: egui::Vec2,
) -> (egui::Vec2, egui::Vec2, egui::Pos2) {
    let pad = 24.0_f32;
    let avail = egui::vec2(
        (screen.width() - pad).max(120.0),
        (screen.height() - pad).max(120.0),
    );
    let min = min.min(avail);
    let size = desired.min(avail).max(min);
    let pos = screen.center() - size * 0.5;
    (size, min, pos)
}

/// Standard modal body layout: one scrollable column + a fixed footer.
///
/// `f` is called twice per frame: with `is_body = true` inside the
/// ScrollArea, then with `is_body = false` for the footer row (the footer
/// callback runs first so it can be laid out at the bottom). A single
/// `FnMut` keeps one mutable borrow of the modal state (two separate
/// body/footer closures would fight over `&mut self`).
///
/// Layout notes for `egui::Window` vertical resize:
/// - `Window` sizes itself from `content_ui.min_rect()`, not from
///   `Resize::desired_size`. If content is shorter than `desired_size`,
///   the window snaps back after an edge drag.
/// - We claim the full available height with `set_min_size` so the outer
///   rect always tracks `desired_size` while dragging edges.
/// - The footer is placed with a `bottom_up` layout at its *natural*
///   height (buttons may wrap to several rows on narrow windows). The
///   ScrollArea then takes whatever height remains, so body + footer
///   always sum to the window height — no fixed 48px reserve that would
///   clip wrapped footers or feed back into runaway window growth.
pub fn modal_body(ui: &mut egui::Ui, id: &str, f: &mut dyn FnMut(&mut egui::Ui, bool)) {
    let total_w = ui.available_width();
    let total_h = ui.available_height();

    // Force min_rect to the full available size so Window edge-drives
    // stick (desired_size) instead of collapsing to short content.
    ui.set_min_size(egui::vec2(total_w, total_h));

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.set_width(total_w);
        // Footer first → bottom edge of the window, natural height.
        f(ui, false);
        ui.separator();
        let body_h = ui.available_height().max(0.0);
        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .max_height(body_h)
            .show(ui, |ui| {
                ui.set_width(total_w);
                f(ui, true);
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2, Rect};

    #[test]
    fn window_defaults_keeps_desired_when_it_fits() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0));
        let (size, min, pos) = window_defaults(screen, vec2(720.0, 640.0), vec2(320.0, 300.0));
        assert_eq!(size, vec2(720.0, 640.0));
        assert_eq!(min, vec2(320.0, 300.0));
        // Centered on screen.
        assert!((pos.x - (1440.0 - 720.0) * 0.5).abs() < 0.5);
        assert!((pos.y - (900.0 - 640.0) * 0.5).abs() < 0.5);
    }

    #[test]
    fn window_defaults_clamps_to_small_screen() {
        // Minimum host window is 800×600; a 720×640 dialog must not spill.
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let (size, min, pos) = window_defaults(screen, vec2(720.0, 640.0), vec2(320.0, 300.0));
        assert!(size.x <= 800.0 - 24.0, "width {} spills", size.x);
        assert!(size.y <= 600.0 - 24.0, "height {} spills", size.y);
        assert!(size.x >= min.x && size.y >= min.y);
        assert!(pos.x >= 0.0 && pos.y >= 0.0);
        assert!(pos.x + size.x <= screen.width());
        assert!(pos.y + size.y <= screen.height());
    }

    #[test]
    fn window_defaults_min_never_exceeds_available() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 320.0));
        let (_size, min, _pos) = window_defaults(screen, vec2(560.0, 520.0), vec2(300.0, 260.0));
        assert!(min.x <= 400.0 - 24.0);
        assert!(min.y <= 320.0 - 24.0);
    }
}
