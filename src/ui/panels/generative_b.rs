use crate::core::document::Object;
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct FlowFieldPanel;

impl FlowFieldPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌌 Vector Flow Field").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal(|ui| {
            if ui.button("🌀 Vortex").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::Vortex,
                    w,
                    h,
                    60,
                    80,
                    5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🧲 Magnetic").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::MagneticDipole,
                    w,
                    h,
                    60,
                    80,
                    5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("⚡ Cyber").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::CyberChaos,
                    w,
                    h,
                    60,
                    80,
                    5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        });
    }
}

pub struct ScatterBrushPanel;

impl ScatterBrushPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖌️ Scatter & Pattern Brush").strong());
        ui.add_space(4.0);

        let has_sel = state.selected_ids.len() >= 2;

        if ui
            .add_enabled(
                has_sel,
                egui::Button::new("Scatter 1st (Motif) along 2nd (Path)"),
            )
            .clicked()
        {
            let id_motif = state.selected_ids[0].clone();
            let id_path = state.selected_ids[1].clone();

            let target_motif = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id_motif)
                .map(|(_, o)| o.clone());
            let target_path = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id_path)
                .map(|(_, o)| o.clone());

            if let (Some(motif), Some(path_obj)) = (target_motif, target_path) {
                let mut traj = path_obj.to_path_data();
                traj.transform(&path_obj.transform.matrix());

                let clones =
                    crate::core::brush::scatter_brush_along_path(&traj, &motif, 30.0, 0.3, true);
                for clone in clones {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(clone));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        } else if !has_sel {
            ui.label(
                RichText::new("Select 2 objects (Motif + Curve)")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct AudioWavePanel;

impl AudioWavePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎵 Audio Waveform (LogicPro DSP)").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal_wrapped(|ui| {
            if ui.button("〰️ Sine Wave").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(
                    crate::core::audio_curve::WaveformType::Sine,
                    4.0,
                    1,
                    w,
                    h,
                    200,
                );
                let obj = Object::new_path("Audio Sine Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("📐 Sawtooth").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(
                    crate::core::audio_curve::WaveformType::Sawtooth,
                    4.0,
                    1,
                    w,
                    h,
                    200,
                );
                let obj = Object::new_path("Audio Saw Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🎹 Harmonics").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(
                    crate::core::audio_curve::WaveformType::Harmonics,
                    3.0,
                    5,
                    w,
                    h,
                    250,
                );
                let obj = Object::new_path("Audio Harmonics", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("⚡ FM Synth").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(
                    crate::core::audio_curve::WaveformType::FM,
                    3.0,
                    1,
                    w,
                    h,
                    300,
                );
                let obj = Object::new_path("Audio FM Synth Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct MeshWarpPanel;

impl MeshWarpPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🦴 2D Mesh Warp (Live2D FFD)").strong());
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
                    .add_enabled(has_sel, egui::Button::new("⭕ Bulge"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(
                            obj,
                            4,
                            4,
                            crate::core::mesh_warp::WarpPreset::Bulge,
                            1.0,
                        );
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("🌀 Twist"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(
                            obj,
                            4,
                            4,
                            crate::core::mesh_warp::WarpPreset::TwistS,
                            1.0,
                        );
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("🌊 Wave"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(
                            obj,
                            4,
                            4,
                            crate::core::mesh_warp::WarpPreset::WaveWarp,
                            1.0,
                        );
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to warp with FFD lattice")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct GradientMeshPanel;

impl GradientMeshPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌈 Gradient Mesh Generator").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal_wrapped(|ui| {
            if ui.button("🌅 Sunset Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Sunset,
                    w,
                    h,
                    3,
                    3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🌆 Cyber Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Cyberpunk,
                    w,
                    h,
                    3,
                    3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🌌 Aurora Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Aurora,
                    w,
                    h,
                    3,
                    3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        });
    }
}

pub struct AxonometricPanel;

impl AxonometricPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 Axonometric Architectural Projections").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Dimetric"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(
                            obj,
                            crate::core::axonometric::AxonometricMode::Dimetric,
                        );
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Trimetric"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(
                            obj,
                            crate::core::axonometric::AxonometricMode::Trimetric,
                        );
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Cabinet (Oblique)"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(
                            obj,
                            crate::core::axonometric::AxonometricMode::Cabinet,
                        );
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Cavalier"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(
                            obj,
                            crate::core::axonometric::AxonometricMode::Cavalier,
                        );
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object for axonometric projection")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct RevolvePanel;

impl RevolvePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🏺 3D Revolve & Lathe Modeler").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            if ui
                .add_enabled(has_sel, egui::Button::new("Export 3D Revolve OBJ..."))
                .clicked()
            {
                if let Some(obj) = &target_obj {
                    let axis = obj.bounding_box().map(|(min, _)| min.x).unwrap_or(0.0);
                    let obj_data =
                        crate::core::revolve::generate_3d_revolve_obj(obj, axis, 360.0, 32);
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OBJ 3D Model", &["obj"])
                        .save_file()
                    {
                        let _ = std::fs::write(path, obj_data);
                    }
                }
            }
        } else {
            ui.label(
                RichText::new("Select a profile path to revolve in 3D")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct EnvelopePanel;

impl EnvelopePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🚩 Envelope Distort & Shape Mold").strong());
        ui.add_space(4.0);

        let has_sel = state.selected_ids.len() >= 2;

        if ui
            .add_enabled(
                has_sel,
                egui::Button::new("Mold 1st (Art) inside 2nd (Frame)"),
            )
            .clicked()
        {
            let id_art = state.selected_ids[0].clone();
            let id_env = state.selected_ids[1].clone();

            let target_art = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id_art)
                .map(|(_, o)| o.clone());
            let target_env = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id_env)
                .map(|(_, o)| o.clone());

            if let (Some(art), Some(env)) = (target_art, target_env) {
                let warped = crate::core::envelope::apply_envelope_distort(&art, &env);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        } else if !has_sel {
            ui.label(
                RichText::new("Select 2 objects (Art + Envelope Frame)")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct PolarPanel;

impl PolarPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌐 Polar Coordinates & Planet Wrap").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;
        let w = state.document.width;
        let h = state.document.height;

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Rect -> Polar"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let polar = crate::core::polar::apply_polar_transform(
                            obj,
                            cx,
                            cy,
                            w,
                            h,
                            crate::core::polar::PolarMode::RectToPolar,
                        );
                        let new_id = polar.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(polar));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Polar -> Rect"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let rect = crate::core::polar::apply_polar_transform(
                            obj,
                            cx,
                            cy,
                            w,
                            h,
                            crate::core::polar::PolarMode::PolarToRect,
                        );
                        let new_id = rect.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(rect));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to wrap into circular polar coordinates")
                    .weak()
                    .size(11.0),
            );
        }
    }
}
