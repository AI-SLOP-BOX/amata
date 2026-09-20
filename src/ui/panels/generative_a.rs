use crate::core::document::Object;
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct TracePanel;

impl TracePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖼️ Live Auto-Trace").strong());
        ui.label(
            RichText::new("Vectorize bitmap into paths")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        if ui.button("Open Image to Trace (PNG/JPG)...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "bmp"])
                .pick_file()
            {
                if let Ok(img) = image::open(&path) {
                    let gray = img.to_luma8();
                    let w = gray.width() as usize;
                    let h = gray.height() as usize;
                    let path_data =
                        crate::core::trace::trace_bitmap_to_path(w, h, gray.as_raw(), 128);
                    let mut obj = Object::new_path(
                        &format!(
                            "Traced {}",
                            path.file_stem().and_then(|s| s.to_str()).unwrap_or("Image")
                        ),
                        path_data,
                    );
                    obj.transform.x = 50.0;
                    obj.transform.y = 50.0;
                    let id = obj.id.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                    state.selected_ids = vec![id];
                }
            }
        }
    }
}

pub struct FormulaPanel;

impl FormulaPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌀 Math & Formula Curves").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui
                .button("🌀 Spiral")
                .on_hover_text("Archimedean Spiral")
                .clicked()
            {
                let path = crate::core::formula::FormulaCurves::spiral(cx, cy, 4.0, 5.0, 3.0, 180);
                let obj = Object::new_path("Spiral", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("〰️ Lissajous")
                .on_hover_text("Oscilloscope Waveform")
                .clicked()
            {
                let path = crate::core::formula::FormulaCurves::lissajous(
                    cx, cy, 3.0, 2.0, 0.5, 200.0, 160.0, 240,
                );
                let obj = Object::new_path("Lissajous", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("💮 Spirograph")
                .on_hover_text("Geometric Spirograph Pattern")
                .clicked()
            {
                let path = crate::core::formula::FormulaCurves::spirograph(
                    cx, cy, 100.0, 42.0, 60.0, 8, 48,
                );
                let obj = Object::new_path("Spirograph", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("🌸 Rose Curve")
                .on_hover_text("Rhodonea Mathematical Flower")
                .clicked()
            {
                let path = crate::core::formula::FormulaCurves::rose_curve(cx, cy, 4.0, 90.0, 200);
                let obj = Object::new_path("Rose Curve", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }
        });
    }
}

pub struct VfxTrailPanel;

impl VfxTrailPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("⚡ VFX Particle Trails").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            if ui
                .add_enabled(
                    has_sel,
                    egui::Button::new("Export Particle Trails (.json)..."),
                )
                .clicked()
            {
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                    let mut path = obj.to_path_data();
                    path.transform(&obj.transform.matrix());
                    let particles =
                        crate::core::vfx_particles::generate_particle_trail(&path, 200, 50.0, 10.0);

                    if let Some(save_path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        if let Ok(json) = serde_json::to_string_pretty(&particles) {
                            let _ = crate::io::atomic::atomic_write_str(&save_path, &json);
                        }
                    }
                }
            }
        } else {
            ui.label(
                RichText::new("Select a path to generate particle trails")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct HalftonePanel;

impl HalftonePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🏁 Halftone & Dot Matrix").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Grid Dots"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let ht = crate::core::halftone::generate_halftone_from_path(
                            &path,
                            10.0,
                            4.5,
                            crate::core::halftone::HalftonePattern::CircularGrid,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Halftone)", obj.name), ht);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Hex Dots"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let ht = crate::core::halftone::generate_halftone_from_path(
                            &path,
                            10.0,
                            4.5,
                            crate::core::halftone::HalftonePattern::HexagonalGrid,
                        );
                        let mut new_obj =
                            Object::new_path(&format!("{} (Hex Halftone)", obj.name), ht);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to generate halftone dots")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct IsometricPanel;

impl IsometricPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 2.5D Isometric Transformer").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Top Plane"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(
                            obj,
                            crate::core::isometric::IsometricPlane::Top,
                        );
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Left Plane"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(
                            obj,
                            crate::core::isometric::IsometricPlane::Left,
                        );
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Right Plane"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(
                            obj,
                            crate::core::isometric::IsometricPlane::Right,
                        );
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to project into isometric plane")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct SymmetryPanel;

impl SymmetryPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("☸️ Radial Symmetry & Mandala").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("4-Fold"))
                    .clicked()
                {
                    Self::apply_sym(state, &id, 4, false);
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("6-Fold"))
                    .clicked()
                {
                    Self::apply_sym(state, &id, 6, false);
                }
                if ui
                    .add_enabled(has_sel, egui::Button::new("8-Fold Mirror"))
                    .clicked()
                {
                    Self::apply_sym(state, &id, 8, true);
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to create symmetry mandala")
                    .weak()
                    .size(11.0),
            );
        }
    }

    fn apply_sym(state: &mut AppState, id: &str, folds: usize, mirror: bool) {
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;
        let target_obj = state
            .document
            .all_objects()
            .find(|(_, o)| o.id == id)
            .map(|(_, o)| o.clone());
        if let Some(obj) = target_obj {
            let clones = crate::core::symmetry::create_radial_symmetry(&obj, cx, cy, folds, mirror);
            for clone in clones {
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(clone));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        }
    }
}

pub struct VoronoiPanel;

impl VoronoiPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🔷 Voronoi & Mosaic Shatter").strong());
        ui.add_space(4.0);

        if ui.button("Generate Voronoi Mosaic (40 Cells)").clicked() {
            let w = state.document.width;
            let h = state.document.height;
            let mut seeds = Vec::with_capacity(40);
            for i in 0..40 {
                let hx = ((i as f64 * 37.123 + 12.34).sin() * 43758.5453)
                    .fract()
                    .abs();
                let hy = ((i as f64 * 91.567 + 84.12).sin() * 43758.5453)
                    .fract()
                    .abs();
                seeds.push(crate::core::path::AnchorPoint::new(hx * w, hy * h));
            }
            let cells = crate::core::voronoi::generate_voronoi_cells(w, h, &seeds, 2.5);
            for cell in cells {
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(cell));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        }
    }
}

pub struct LSystemPanel;

impl LSystemPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌿 L-System Fractals").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui.button("🌲 Tree").clicked() {
                let path = crate::core::lsystem::generate_lsystem(
                    crate::core::lsystem::LSystemPreset::Tree,
                    4,
                    cx,
                    cy + 150.0,
                    12.0,
                );
                let obj = Object::new_path("Fractal Tree", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🐉 Dragon").clicked() {
                let path = crate::core::lsystem::generate_lsystem(
                    crate::core::lsystem::LSystemPreset::Dragon,
                    10,
                    cx - 100.0,
                    cy,
                    6.0,
                );
                let obj = Object::new_path("Dragon Curve", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("❄️ Snowflake").clicked() {
                let path = crate::core::lsystem::generate_lsystem(
                    crate::core::lsystem::LSystemPreset::Snowflake,
                    3,
                    cx - 100.0,
                    cy - 50.0,
                    5.0,
                );
                let obj = Object::new_path("Koch Snowflake", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🔲 Hilbert").clicked() {
                let path = crate::core::lsystem::generate_lsystem(
                    crate::core::lsystem::LSystemPreset::Hilbert,
                    4,
                    cx - 100.0,
                    cy - 100.0,
                    14.0,
                );
                let obj = Object::new_path("Hilbert Curve", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct QrCodePanel;

impl QrCodePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📱 Vector QR & Barcode").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal(|ui| {
            if ui.button("Generate QR Code...").clicked() {
                if let Ok(path) = crate::core::barcode::generate_vector_qr(
                    "https://github.com/AI-SLOP-BOX/amata",
                    cx,
                    cy,
                    160.0,
                ) {
                    let obj = Object::new_path("Vector QR Code", path);
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("Barcode (Code-128)").clicked() {
                let path = crate::core::barcode::generate_vector_barcode(
                    "IRASU-AEVFX-2026",
                    cx,
                    cy,
                    200.0,
                    60.0,
                );
                let obj = Object::new_path("Vector Barcode", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct DeformPanel;

impl DeformPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌊 Noise & Wave Deformer").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("🌊 Wave"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(
                            &path,
                            crate::core::noise::DeformType::SineWave,
                            10.0,
                            0.08,
                            0.0,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Wave)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("🌪️ Noise"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(
                            &path,
                            crate::core::noise::DeformType::TurbulentNoise,
                            12.0,
                            0.05,
                            1.23,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Noise)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("⚡ Glitch"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(
                            &path,
                            crate::core::noise::DeformType::JitterGlitch,
                            8.0,
                            0.2,
                            5.67,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Glitch)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to deform")
                    .weak()
                    .size(11.0),
            );
        }
    }
}
