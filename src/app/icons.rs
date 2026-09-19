/// icons.rs — Vector icon painters for Amata.
/// Each function draws a crisp, scalable icon using egui's Painter API.
/// Icons are drawn to fit inside `rect` with appropriate padding.
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, Vec2};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn pad(rect: Rect, frac: f32) -> Rect {
    rect.shrink(rect.size().min_elem() * frac)
}

fn center(rect: Rect) -> Pos2 {
    rect.center()
}

// ─── Tool Icons ─────────────────────────────────────────────────────────────

/// Selection arrow (↖)
pub fn icon_select(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.08);
    let tip = Pos2::new(r.min.x + r.width() * 0.15, r.min.y + r.height() * 0.12);
    let pts = vec![
        tip,
        Pos2::new(tip.x, tip.y + r.height() * 0.65),
        Pos2::new(tip.x + r.width() * 0.20, tip.y + r.height() * 0.45),
        Pos2::new(tip.x + r.width() * 0.35, tip.y + r.height() * 0.72),
        Pos2::new(tip.x + r.width() * 0.47, tip.y + r.height() * 0.65),
        Pos2::new(tip.x + r.width() * 0.32, tip.y + r.height() * 0.38),
        Pos2::new(tip.x + r.width() * 0.55, tip.y + r.height() * 0.38),
    ];
    p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

/// Direct Select / Node tool (hollow arrow)
pub fn icon_node(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.08);
    let tip = Pos2::new(r.min.x + r.width() * 0.15, r.min.y + r.height() * 0.12);
    let pts = vec![
        tip,
        Pos2::new(tip.x, tip.y + r.height() * 0.65),
        Pos2::new(tip.x + r.width() * 0.20, tip.y + r.height() * 0.45),
        Pos2::new(tip.x + r.width() * 0.35, tip.y + r.height() * 0.72),
        Pos2::new(tip.x + r.width() * 0.47, tip.y + r.height() * 0.65),
        Pos2::new(tip.x + r.width() * 0.32, tip.y + r.height() * 0.38),
        Pos2::new(tip.x + r.width() * 0.55, tip.y + r.height() * 0.38),
    ];
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.5_f32, color)));
    // Small square at tip to indicate node selection
    let sq = Rect::from_center_size(
        Pos2::new(tip.x + r.width() * 0.55, tip.y + r.height() * 0.15),
        Vec2::splat(4.0),
    );
    p.rect_stroke(
        sq,
        0.0,
        Stroke::new(1.2_f32, color),
        egui::StrokeKind::Middle,
    );
}

/// Pen tool nib
pub fn icon_pen(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let c = center(r);
    let stroke = Stroke::new(1.5_f32, color);
    let top = Pos2::new(c.x, r.min.y);
    let right = Pos2::new(r.max.x, c.y - r.height() * 0.05);
    let bottom = Pos2::new(c.x, r.max.y - r.height() * 0.15);
    let left = Pos2::new(r.min.x, c.y - r.height() * 0.05);
    p.add(egui::Shape::closed_line(
        vec![top, right, bottom, left],
        stroke,
    ));
    p.line_segment(
        [bottom, Pos2::new(c.x - r.width() * 0.08, r.max.y)],
        Stroke::new(1.5_f32, color),
    );
    p.line_segment(
        [bottom, Pos2::new(c.x + r.width() * 0.08, r.max.y)],
        Stroke::new(1.5_f32, color),
    );
    p.circle_filled(c, 2.0, color);
}

/// Pencil icon
pub fn icon_pencil(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let stroke = Stroke::new(1.5_f32, color);
    let body_top_l = Pos2::new(r.min.x + r.width() * 0.25, r.min.y + r.height() * 0.10);
    let body_top_r = Pos2::new(r.min.x + r.width() * 0.65, r.min.y + r.height() * 0.10);
    let body_bot_r = Pos2::new(r.min.x + r.width() * 0.65, r.max.y - r.height() * 0.25);
    let body_bot_l = Pos2::new(r.min.x + r.width() * 0.25, r.max.y - r.height() * 0.25);
    let tip = Pos2::new(r.center().x, r.max.y - r.height() * 0.04);
    p.add(egui::Shape::closed_line(
        vec![body_top_l, body_top_r, body_bot_r, body_bot_l],
        stroke,
    ));
    p.line_segment([body_bot_l, tip], stroke);
    p.line_segment([body_bot_r, tip], stroke);
    let band_y = r.min.y + r.height() * 0.22;
    p.line_segment(
        [
            Pos2::new(body_top_l.x, band_y),
            Pos2::new(body_top_r.x, band_y),
        ],
        stroke,
    );
}

/// Rectangle tool
pub fn icon_rectangle(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.18);
    p.rect_stroke(
        r,
        0.0,
        Stroke::new(1.6_f32, color),
        egui::StrokeKind::Middle,
    );
}

/// Ellipse tool
pub fn icon_ellipse(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    p.circle_stroke(
        r.center(),
        r.size().min_elem() * 0.45,
        Stroke::new(1.6_f32, color),
    );
}

/// Star tool (5-pointed)
pub fn icon_star(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.08);
    let c = center(r);
    let outer = r.size().min_elem() * 0.46;
    let inner = outer * 0.42;
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let angle = std::f32::consts::PI * i as f32 / 5.0 - std::f32::consts::PI / 2.0;
            let radius = if i % 2 == 0 { outer } else { inner };
            Pos2::new(c.x + angle.cos() * radius, c.y + angle.sin() * radius)
        })
        .collect();
    p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

/// Polygon tool (hexagon outline)
pub fn icon_polygon(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let c = center(r);
    let radius = r.size().min_elem() * 0.44;
    let pts: Vec<Pos2> = (0..6)
        .map(|i| {
            let angle = std::f32::consts::PI / 3.0 * i as f32 - std::f32::consts::PI / 6.0;
            Pos2::new(c.x + angle.cos() * radius, c.y + angle.sin() * radius)
        })
        .collect();
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.6_f32, color)));
}

/// Line Segment tool
pub fn icon_line(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.15);
    p.line_segment(
        [Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.min.y)],
        Stroke::new(1.8_f32, color),
    );
    p.circle_filled(Pos2::new(r.min.x, r.max.y), 2.0, color);
    p.circle_filled(Pos2::new(r.max.x, r.min.y), 2.0, color);
}

/// Type / Text tool
pub fn icon_text(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let stroke = Stroke::new(1.7_f32, color);
    let top_y = r.min.y + r.height() * 0.08;
    p.line_segment(
        [Pos2::new(r.min.x, top_y), Pos2::new(r.max.x, top_y)],
        stroke,
    );
    p.line_segment(
        [
            Pos2::new(r.center().x, top_y),
            Pos2::new(r.center().x, r.max.y - r.height() * 0.05),
        ],
        stroke,
    );
    let serif_y = r.max.y - r.height() * 0.05;
    let serif_w = r.width() * 0.18;
    p.line_segment(
        [
            Pos2::new(r.center().x - serif_w, serif_y),
            Pos2::new(r.center().x + serif_w, serif_y),
        ],
        stroke,
    );
}

/// Eyedropper tool
pub fn icon_eyedropper(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let body_start = Pos2::new(r.max.x - r.width() * 0.12, r.min.y + r.height() * 0.12);
    let body_end = Pos2::new(r.min.x + r.width() * 0.28, r.max.y - r.height() * 0.30);
    p.line_segment([body_start, body_end], Stroke::new(3.5_f32, color));
    p.circle_filled(
        Pos2::new(r.min.x + r.width() * 0.18, r.max.y - r.height() * 0.18),
        2.5,
        color,
    );
    p.circle_stroke(
        Pos2::new(
            body_start.x - r.width() * 0.05,
            body_start.y + r.height() * 0.05,
        ),
        4.5,
        Stroke::new(1.5_f32, color),
    );
}

/// Hand / Pan tool
pub fn icon_hand(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.08);
    let c = center(r);
    let palm_y = c.y + r.height() * 0.08;
    for i in 0..4usize {
        let x = r.min.x + r.width() * (0.10 + 0.22 * i as f32);
        let finger_top = r.min.y + r.height() * (0.08 + 0.08 * i.min(1) as f32);
        p.line_segment(
            [Pos2::new(x, finger_top), Pos2::new(x, palm_y)],
            Stroke::new(5.5_f32, color),
        );
        p.circle_filled(Pos2::new(x, finger_top), 2.8, color);
    }
    let thumb_x = r.max.x - r.width() * 0.10;
    p.line_segment(
        [
            Pos2::new(thumb_x, c.y - r.height() * 0.05),
            Pos2::new(thumb_x, palm_y),
        ],
        Stroke::new(5.5_f32, color),
    );
    p.circle_filled(Pos2::new(thumb_x, c.y - r.height() * 0.05), 2.8, color);
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(r.min.x + r.width() * 0.06, palm_y),
            Vec2::new(r.width() * 0.88, r.height() * 0.30),
        ),
        CornerRadius::same(3),
        color,
    );
}

/// Brush tool
pub fn icon_brush(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    p.line_segment(
        [
            Pos2::new(r.max.x - r.width() * 0.05, r.min.y + r.height() * 0.05),
            Pos2::new(r.min.x + r.width() * 0.35, r.max.y - r.height() * 0.30),
        ],
        Stroke::new(2.2_f32, color),
    );
    let band_pos = Pos2::new(r.min.x + r.width() * 0.38, r.max.y - r.height() * 0.33);
    p.circle_filled(band_pos, 3.0, Color32::from_gray(120));
    let tip = Pos2::new(r.min.x + r.width() * 0.12, r.max.y - r.height() * 0.06);
    p.line_segment([band_pos, tip], Stroke::new(3.5_f32, color));
    p.circle_filled(tip, 2.0, color);
}

/// Eraser tool
pub fn icon_eraser(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let body = Rect::from_min_size(
        Pos2::new(r.min.x + r.width() * 0.08, r.center().y - r.height() * 0.18),
        Vec2::new(r.width() * 0.84, r.height() * 0.40),
    );
    p.rect_stroke(
        body,
        CornerRadius::same(2),
        Stroke::new(1.6_f32, color),
        egui::StrokeKind::Middle,
    );
    let cap = Rect::from_min_size(body.min, Vec2::new(r.width() * 0.28, body.height()));
    p.rect_filled(
        cap,
        CornerRadius {
            nw: 2,
            ne: 0,
            sw: 2,
            se: 0,
        },
        Color32::from_rgb(235, 120, 140),
    );
    p.line_segment(
        [
            Pos2::new(body.min.x + body.width() * 0.35, body.max.y + 3.0),
            Pos2::new(body.max.x, body.max.y + 3.0),
        ],
        Stroke::new(1.2_f32, color),
    );
}

/// Shape Builder tool (two overlapping circles + arrow)
pub fn icon_shape_builder(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let stroke = Stroke::new(1.4_f32, color);
    let r1 = r.size().min_elem() * 0.28;
    let c1 = Pos2::new(r.center().x - r1 * 0.5, r.center().y);
    let c2 = Pos2::new(r.center().x + r1 * 0.5, r.center().y);
    p.circle_stroke(c1, r1, stroke);
    p.circle_stroke(c2, r1, stroke);
    let ax = r.max.x - r.width() * 0.10;
    let ay = r.min.y + r.height() * 0.22;
    p.line_segment(
        [Pos2::new(ax - r.width() * 0.18, ay), Pos2::new(ax, ay)],
        Stroke::new(1.3_f32, color),
    );
    p.line_segment(
        [
            Pos2::new(ax - r.width() * 0.07, ay - r.height() * 0.07),
            Pos2::new(ax, ay),
        ],
        Stroke::new(1.3_f32, color),
    );
    p.line_segment(
        [
            Pos2::new(ax - r.width() * 0.07, ay + r.height() * 0.07),
            Pos2::new(ax, ay),
        ],
        Stroke::new(1.3_f32, color),
    );
}

/// Zoom / Magnifier tool
pub fn icon_zoom(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.1);
    let glass_c = Pos2::new(
        r.center().x - r.width() * 0.08,
        r.center().y - r.height() * 0.08,
    );
    let glass_r = r.size().min_elem() * 0.30;
    p.circle_stroke(glass_c, glass_r, Stroke::new(1.8_f32, color));
    let handle_start = Pos2::new(glass_c.x + glass_r * 0.70, glass_c.y + glass_r * 0.70);
    let handle_end = Pos2::new(r.max.x - r.width() * 0.08, r.max.y - r.height() * 0.08);
    p.line_segment([handle_start, handle_end], Stroke::new(2.0_f32, color));
    p.line_segment(
        [
            Pos2::new(glass_c.x - glass_r * 0.45, glass_c.y),
            Pos2::new(glass_c.x + glass_r * 0.45, glass_c.y),
        ],
        Stroke::new(1.2_f32, color),
    );
    p.line_segment(
        [
            Pos2::new(glass_c.x, glass_c.y - glass_r * 0.45),
            Pos2::new(glass_c.x, glass_c.y + glass_r * 0.45),
        ],
        Stroke::new(1.2_f32, color),
    );
}

// ─── UI Icons ───────────────────────────────────────────────────────────────

/// Bell notification icon
pub fn icon_bell(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let c = center(r);
    let stroke = Stroke::new(1.4_f32, color);
    let dome_top = Pos2::new(c.x, r.min.y + r.height() * 0.10);
    let dome_pts = vec![
        Pos2::new(c.x - r.width() * 0.36, c.y + r.height() * 0.18),
        Pos2::new(c.x - r.width() * 0.42, c.y + r.height() * 0.30),
        Pos2::new(c.x + r.width() * 0.42, c.y + r.height() * 0.30),
        Pos2::new(c.x + r.width() * 0.36, c.y + r.height() * 0.18),
        Pos2::new(c.x + r.width() * 0.28, r.min.y + r.height() * 0.20),
        dome_top,
        Pos2::new(c.x - r.width() * 0.28, r.min.y + r.height() * 0.20),
    ];
    p.add(egui::Shape::closed_line(dome_pts, stroke));
    p.circle_stroke(
        Pos2::new(c.x, r.max.y - r.height() * 0.12),
        r.size().min_elem() * 0.09,
        stroke,
    );
    p.line_segment(
        [dome_top, Pos2::new(c.x, r.min.y + r.height() * 0.03)],
        Stroke::new(1.2_f32, color),
    );
}

/// Search magnifying glass icon
pub fn icon_search(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.15);
    let c = Pos2::new(
        r.center().x - r.width() * 0.08,
        r.center().y - r.height() * 0.08,
    );
    let rad = r.size().min_elem() * 0.32;
    p.circle_stroke(c, rad, Stroke::new(1.4_f32, color));
    let start = Pos2::new(c.x + rad * 0.707, c.y + rad * 0.707);
    let end = Pos2::new(r.max.x - 1.0, r.max.y - 1.0);
    p.line_segment([start, end], Stroke::new(1.8_f32, color));
}

/// Layers icon (three offset horizontal bars)
pub fn icon_layers(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let stroke = Stroke::new(1.4_f32, color);
    for i in 0..3usize {
        let y = r.min.y + r.height() * (0.20 + 0.30 * i as f32);
        let indent = r.width() * 0.12 * i as f32;
        p.line_segment(
            [
                Pos2::new(r.min.x + indent, y),
                Pos2::new(r.max.x - indent, y),
            ],
            stroke,
        );
    }
}

/// Properties / Sliders icon
pub fn icon_properties(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let stroke = Stroke::new(1.3_f32, color);
    let thumb_xs: [f32; 3] = [0.65, 0.35, 0.55];
    for (i, &thumb_xs_i) in thumb_xs.iter().enumerate() {
        let y = r.min.y + r.height() * (0.20 + 0.30 * i as f32);
        p.line_segment([Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)], stroke);
        let thumb_x = r.min.x + r.width() * thumb_xs_i;
        p.circle_filled(Pos2::new(thumb_x, y), 3.0, color);
    }
}

/// Libraries icon (2×2 grid of squares)
pub fn icon_libraries(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let stroke = Stroke::new(1.3_f32, color);
    for row in 0..2i32 {
        for col in 0..2i32 {
            let x = r.min.x + r.width() * (0.10 + 0.50 * col as f32);
            let y = r.min.y + r.height() * (0.10 + 0.50 * row as f32);
            let cell =
                Rect::from_min_size(Pos2::new(x, y), Vec2::splat(r.size().min_elem() * 0.38));
            p.rect_stroke(cell, 1.0, stroke, egui::StrokeKind::Middle);
        }
    }
}

/// Document paper icon with folded corner
pub fn icon_document(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.16);
    let fold = r.width() * 0.32;
    let stroke = Stroke::new(1.4_f32, color);
    let pts = vec![
        Pos2::new(r.min.x, r.min.y),
        Pos2::new(r.max.x - fold, r.min.y),
        Pos2::new(r.max.x, r.min.y + fold),
        Pos2::new(r.max.x, r.max.y),
        Pos2::new(r.min.x, r.max.y),
    ];
    p.add(egui::Shape::closed_line(pts, stroke));
    // Fold crease
    p.line_segment(
        [
            Pos2::new(r.max.x - fold, r.min.y),
            Pos2::new(r.max.x - fold, r.min.y + fold),
        ],
        stroke,
    );
    p.line_segment(
        [
            Pos2::new(r.max.x - fold, r.min.y + fold),
            Pos2::new(r.max.x, r.min.y + fold),
        ],
        stroke,
    );
}

/// Desktop browser monitor window icon
pub fn icon_desktop(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.16);
    let stroke = Stroke::new(1.4_f32, color);
    let screen = Rect::from_min_max(r.min, Pos2::new(r.max.x, r.max.y - r.height() * 0.25));
    p.rect_stroke(screen, 2.0, stroke, egui::StrokeKind::Middle);
    // Browser title bar line
    p.line_segment(
        [
            Pos2::new(screen.min.x, screen.min.y + 6.0),
            Pos2::new(screen.max.x, screen.min.y + 6.0),
        ],
        Stroke::new(1.0_f32, color),
    );
    // Dots
    p.circle_filled(
        Pos2::new(screen.min.x + 5.0, screen.min.y + 3.5),
        1.0,
        color,
    );
    p.circle_filled(
        Pos2::new(screen.min.x + 9.0, screen.min.y + 3.5),
        1.0,
        color,
    );
    p.circle_filled(
        Pos2::new(screen.min.x + 13.0, screen.min.y + 3.5),
        1.0,
        color,
    );
}

/// Mobile smartphone icon
pub fn icon_phone(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.16);
    let stroke = Stroke::new(1.4_f32, color);
    let phone = Rect::from_center_size(r.center(), Vec2::new(r.width() * 0.58, r.height()));
    p.rect_stroke(phone, 3.0, stroke, egui::StrokeKind::Middle);
    // Speaker slit
    p.line_segment(
        [
            Pos2::new(phone.center().x - 4.0, phone.min.y + 4.0),
            Pos2::new(phone.center().x + 4.0, phone.min.y + 4.0),
        ],
        Stroke::new(1.0_f32, color),
    );
    // Home indicator
    p.line_segment(
        [
            Pos2::new(phone.center().x - 6.0, phone.max.y - 4.0),
            Pos2::new(phone.center().x + 6.0, phone.max.y - 4.0),
        ],
        Stroke::new(1.2_f32, color),
    );
}

/// Camera / Instagram icon
pub fn icon_camera(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.16);
    let stroke = Stroke::new(1.4_f32, color);
    let body = Rect::from_center_size(r.center(), Vec2::new(r.width() * 0.85, r.height() * 0.78));
    p.rect_stroke(body, 3.0, stroke, egui::StrokeKind::Middle);
    p.circle_stroke(body.center(), body.height() * 0.28, stroke);
    p.circle_filled(Pos2::new(body.max.x - 5.0, body.min.y + 5.0), 1.5, color);
}

/// Lock icon (shackle + body)
pub fn icon_lock(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.16);
    let stroke = Stroke::new(1.3_f32, color);
    let body_h = r.height() * 0.55;
    let body = Rect::from_min_max(Pos2::new(r.min.x, r.max.y - body_h), r.max);
    p.rect_filled(body, CornerRadius::same(2), color);

    // Shackle
    let shackle_w = r.width() * 0.52;
    let shackle_left = r.center().x - shackle_w * 0.5;
    let shackle_right = r.center().x + shackle_w * 0.5;
    let shackle_top = r.min.y + 1.0;
    let shackle_pts = vec![
        Pos2::new(shackle_left, body.min.y),
        Pos2::new(shackle_left, shackle_top + 3.0),
        Pos2::new(r.center().x, shackle_top),
        Pos2::new(shackle_right, shackle_top + 3.0),
        Pos2::new(shackle_right, body.min.y),
    ];
    p.add(egui::Shape::line(shackle_pts, stroke));
}

/// Lightbulb / Idea hint icon
pub fn icon_bulb(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.15);
    let c = Pos2::new(r.center().x, r.min.y + r.height() * 0.38);
    let rad = r.size().min_elem() * 0.34;
    p.circle_stroke(c, rad, Stroke::new(1.3_f32, color));

    // Base screw lines
    let base_y1 = c.y + rad * 0.75;
    let base_y2 = base_y1 + 4.0;
    let base_w = rad * 0.65;
    p.line_segment(
        [
            Pos2::new(c.x - base_w, base_y1),
            Pos2::new(c.x + base_w, base_y1),
        ],
        Stroke::new(1.3_f32, color),
    );
    p.line_segment(
        [
            Pos2::new(c.x - base_w * 0.7, base_y2),
            Pos2::new(c.x + base_w * 0.7, base_y2),
        ],
        Stroke::new(1.3_f32, color),
    );
}

/// More / Ellipsis icon
pub fn icon_more_dots(p: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = 2.0;
    p.circle_filled(Pos2::new(c.x - 8.0, c.y), r, color);
    p.circle_filled(Pos2::new(c.x, c.y), r, color);
    p.circle_filled(Pos2::new(c.x + 8.0, c.y), r, color);
}

// ─── Render Helpers ──────────────────────────────────────────────────────────

/// Paint a tool's vector icon into the given rect.
pub fn paint_tool_icon(p: &Painter, tool: crate::core::state::Tool, rect: Rect, color: Color32) {
    use crate::core::state::Tool;
    match tool {
        Tool::Select => icon_select(p, rect, color),
        Tool::Node => icon_node(p, rect, color),
        Tool::Pen => icon_pen(p, rect, color),
        Tool::Pencil => icon_pencil(p, rect, color),
        Tool::Rectangle => icon_rectangle(p, rect, color),
        Tool::Ellipse => icon_ellipse(p, rect, color),
        Tool::Star => icon_star(p, rect, color),
        Tool::Polygon => icon_polygon(p, rect, color),
        Tool::Line => icon_line(p, rect, color),
        Tool::Text => icon_text(p, rect, color),
        Tool::Eyedropper => icon_eyedropper(p, rect, color),
        Tool::Hand => icon_hand(p, rect, color),
        Tool::Brush => icon_brush(p, rect, color),
        Tool::Eraser => icon_eraser(p, rect, color),
        Tool::ShapeBuilder => icon_shape_builder(p, rect, color),
        Tool::Zoom => icon_zoom(p, rect, color),
    }
}

/// Allocate a square region and paint a tool icon button.
/// Returns the egui Response for click/hover detection.
pub fn tool_icon_button(
    ui: &mut egui::Ui,
    tool: crate::core::state::Tool,
    is_active: bool,
    size: Vec2,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    let bg_color = if is_active {
        Color32::from_rgb(20, 115, 230)
    } else if response.hovered() {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::TRANSPARENT
    };

    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(3), bg_color);
        let icon_color = if is_active {
            Color32::WHITE
        } else {
            Color32::from_gray(200)
        };
        paint_tool_icon(ui.painter(), tool, rect, icon_color);

        // Adobe CC signature tiny bottom-right triangle showing sub-tools available
        let tri_pts = vec![
            Pos2::new(rect.max.x - 5.0, rect.max.y - 1.5),
            Pos2::new(rect.max.x - 1.5, rect.max.y - 1.5),
            Pos2::new(rect.max.x - 1.5, rect.max.y - 5.0),
        ];
        let tri_color = if is_active {
            Color32::WHITE
        } else {
            Color32::from_gray(140)
        };
        ui.painter().add(egui::Shape::convex_polygon(
            tri_pts,
            tri_color,
            Stroke::NONE,
        ));
    }

    response
}

/// Small icon-only button (non-tool) with hover highlight.
pub fn icon_button(
    ui: &mut egui::Ui,
    size: Vec2,
    painter_fn: impl FnOnce(&Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if response.hovered() {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::TRANSPARENT
    };
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(3), bg);
        let color = if response.hovered() {
            Color32::WHITE
        } else {
            Color32::from_gray(180)
        };
        painter_fn(ui.painter(), rect, color);
    }
    response
}

/// Official Amata 3-Node Vector Emblem Painter
/// Renders the polished pure vector logo scaled to any given bounding `rect`.
pub fn icon_amata_logo(p: &Painter, rect: Rect) {
    let r = pad(rect, 0.06);
    let w = r.width();
    let h = r.height();
    let ox = r.min.x;
    let oy = r.min.y;

    // Helper mapping 0..512 logo coordinates to the given target rect
    let pt = |x: f32, y: f32| -> Pos2 { Pos2::new(ox + (x / 512.0) * w, oy + (y / 512.0) * h) };

    let p_top = pt(264.0, 113.0);
    let p_bl = pt(99.0, 364.0);
    let p_br = pt(419.0, 373.0);

    // 1. Left Curved Spine (Electric Cyan -> Cobalt Blue)
    let spine_steps = 14;
    for i in 0..spine_steps {
        let t0 = i as f32 / spine_steps as f32;
        let t1 = (i + 1) as f32 / spine_steps as f32;

        // Quadratic Bezier approx: M 99 364 C 104 290, 168 180, 264 113
        let eval = |t: f32| -> Pos2 {
            let omt = 1.0 - t;
            let omt2 = omt * omt;
            let omt3 = omt2 * omt;
            let t2 = t * t;
            let t3 = t2 * t;
            let x = omt3 * 99.0 + 3.0 * omt2 * t * 104.0 + 3.0 * omt * t2 * 168.0 + t3 * 264.0;
            let y = omt3 * 364.0 + 3.0 * omt2 * t * 290.0 + 3.0 * omt * t2 * 180.0 + t3 * 113.0;
            pt(x, y)
        };

        let stroke_col = Color32::from_rgb(
            (0.0 * (1.0 - t0) + 0.0 * t0) as u8,
            (208.0 * (1.0 - t0) + 102.0 * t0) as u8,
            (214.0 * (1.0 - t0) + 224.0 * t0) as u8,
        );
        let stroke_w = (14.0 / 512.0 * w).max(1.8);
        p.line_segment([eval(t0), eval(t1)], Stroke::new(stroke_w, stroke_col));
    }

    // 2. Central Vector Crossbar Arch (Cyan-Sky -> Indigo-Purple)
    let arch_pts = [
        pt(114.0, 342.0),
        pt(136.0, 298.0),
        pt(185.0, 252.0),
        pt(244.0, 244.0),
        pt(280.0, 239.0),
        pt(318.0, 252.0),
        pt(348.0, 274.0),
        pt(328.0, 295.0),
        pt(298.0, 294.0),
        pt(268.0, 282.0),
        pt(218.0, 264.0),
        pt(165.0, 295.0),
    ];
    p.add(egui::Shape::convex_polygon(
        arch_pts.to_vec(),
        Color32::from_rgb(79, 70, 229),
        Stroke::NONE,
    ));

    // 3. Descending Right Ribbon (Indigo -> Fuchsia -> Coral -> Tangerine)
    let ribbon_pts = [
        pt(261.0, 115.0),
        pt(265.0, 158.0),
        pt(275.0, 212.0),
        pt(287.0, 258.0),
        pt(300.0, 302.0),
        pt(330.0, 345.0),
        pt(374.0, 372.0),
        pt(419.0, 373.0),
        pt(405.0, 352.0),
        pt(376.0, 325.0),
        pt(351.0, 287.0),
        pt(326.0, 244.0),
        pt(301.0, 188.0),
        pt(280.0, 115.0),
    ];
    p.add(egui::Shape::convex_polygon(
        ribbon_pts.to_vec(),
        Color32::from_rgb(225, 29, 72),
        Stroke::NONE,
    ));

    // 4. Three Modern Vector Anchor Nodes
    let r_top = (25.0 / 512.0 * w).max(3.0);
    let r_bl = (24.0 / 512.0 * w).max(2.8);
    let r_br = (23.0 / 512.0 * w).max(2.6);

    // Top Node (Royal Blue)
    p.circle_filled(p_top, r_top, Color32::from_rgb(37, 99, 235));
    // BL Node (Electric Cyan)
    p.circle_filled(p_bl, r_bl, Color32::from_rgb(6, 182, 212));
    // BR Node (Tangerine Orange)
    p.circle_filled(p_br, r_br, Color32::from_rgb(251, 146, 60));
}
