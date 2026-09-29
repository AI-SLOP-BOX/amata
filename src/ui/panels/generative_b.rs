use crate::core::document::Object;
use crate::core::state::AppState;
use crate::ui::panels::color_utils::color_edit_srgba_u8;
use egui::{RichText, Ui};

pub struct FlowFieldPanel;

impl FlowFieldPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("ベクターフローフィールド").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal(|ui| {
            if ui.button("渦巻き").clicked() {
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

            if ui.button("マグネット").clicked() {
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

            if ui.button("サイバー").clicked() {
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
        ui.heading(RichText::new("スキャター・パターンブラシ").strong());
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
                RichText::new("モチーフとカーブの2つを選択してください")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct AudioWavePanel;

impl AudioWavePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("オーディオ波形（LogicPro DSP）").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal_wrapped(|ui| {
            if ui.button("正弦波").clicked() {
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

            if ui.button("ノコギリ波").clicked() {
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

            if ui.button("倍音").clicked() {
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

            if ui.button("FMシンセ").clicked() {
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
        ui.heading(RichText::new("2D Mesh Warp (Live2D FFD)").strong());
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
                    .add_enabled(has_sel, egui::Button::new("膨らみ"))
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
                    .add_enabled(has_sel, egui::Button::new("ねじれ"))
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

                if ui.add_enabled(has_sel, egui::Button::new("波形")).clicked() {
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
                RichText::new("FFD格子で歪めるオブジェクトを選択してください")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct GradientMeshPanel;

fn preset_corners(preset: crate::core::gradient_mesh::GradientMeshPreset) -> [[f32; 4]; 4] {
    match preset {
        crate::core::gradient_mesh::GradientMeshPreset::Sunset => [
            [1.0, 0.2, 0.4, 0.95],
            [1.0, 0.6, 0.1, 0.95],
            [0.4, 0.1, 0.6, 0.95],
            [0.1, 0.05, 0.3, 0.95],
        ],
        crate::core::gradient_mesh::GradientMeshPreset::Cyberpunk => [
            [0.0, 0.9, 1.0, 0.95],
            [1.0, 0.1, 0.6, 0.95],
            [0.1, 0.0, 0.4, 0.95],
            [0.0, 1.0, 0.5, 0.95],
        ],
        crate::core::gradient_mesh::GradientMeshPreset::Aurora => [
            [0.1, 0.9, 0.5, 0.95],
            [0.1, 0.5, 0.9, 0.95],
            [0.5, 0.1, 0.8, 0.95],
            [0.05, 0.2, 0.4, 0.95],
        ],
        crate::core::gradient_mesh::GradientMeshPreset::Gold => [
            [1.0, 0.9, 0.5, 0.95],
            [0.9, 0.6, 0.2, 0.95],
            [0.6, 0.4, 0.1, 0.95],
            [0.3, 0.2, 0.05, 0.95],
        ],
    }
}

fn selected_mesh_id(state: &AppState) -> Option<String> {
    for id in &state.selected_ids {
        if let Some(obj) = state.document.find_object(id) {
            if matches!(
                obj.object_type,
                crate::core::document::ObjectType::GradientMesh(_)
            ) {
                return Some(id.clone());
            }
        }
    }
    state
        .document
        .all_objects()
        .find(|(_, o)| {
            matches!(o.object_type, crate::core::document::ObjectType::GradientMesh(_))
        })
        .map(|(_, o)| o.id.clone())
}

impl GradientMeshPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("グラデーションメッシュ").strong());
        ui.add_space(4.0);

        // Create a new editable mesh from a preset.
        ui.label(RichText::new("新規メッシュ").weak());
        ui.horizontal_wrapped(|ui| {
            for (label, preset) in [
                ("サンセット", crate::core::gradient_mesh::GradientMeshPreset::Sunset),
                ("サイバー", crate::core::gradient_mesh::GradientMeshPreset::Cyberpunk),
                ("オーロラ", crate::core::gradient_mesh::GradientMeshPreset::Aurora),
                ("ゴールド", crate::core::gradient_mesh::GradientMeshPreset::Gold),
            ] {
                if ui.button(label).clicked() {
                    let size = state.document.width.min(state.document.height) / 2.0;
                    let mesh = crate::core::gradient_mesh::MeshGradient::new_rect(
                        0.0,
                        0.0,
                        size,
                        size,
                        4,
                        4,
                        preset_corners(preset),
                    );
                    let obj = crate::core::document::Object::new_mesh(
                        "グラデーションメッシュ",
                        (state.document.width - size) / 2.0,
                        (state.document.height - size) / 2.0,
                        mesh,
                    );
                    let id = obj.id.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                    state.selected_ids = vec![id];
                    state.notify_success("グラデーションメッシュを作成しました");
                }
            }
        });
        ui.add_space(4.0);

        // Edit the targeted mesh node by node.
        let Some(id) = selected_mesh_id(state) else {
            ui.label(
                RichText::new("メッシュを選択するとノード編集できます。ノードはキャンバス上で■表示されます。")
                    .weak()
                    .size(11.0),
            );
            return;
        };
        let (rows, cols) = match state.document.find_object(&id) {
            Some(obj) => {
                if let crate::core::document::ObjectType::GradientMesh(m) = &obj.object_type {
                    (m.rows, m.cols)
                } else {
                    return;
                }
            }
            None => return,
        };
        ui.label(format!("編集中: {rows}×{cols} ノード"));
        // Node picker.
        let mut node_rc: Option<(usize, usize)> = None;
        ui.horizontal(|ui| {
            ui.label("ノード:");
            // Persist picker in egui memory (panel is stateless).
            let mem_id = egui::Id::new(("mesh_node", id.clone()));
            let (mut r, mut c): (usize, usize) = ui.memory_mut(|m| *m.data.get_temp_mut_or_default(mem_id));
            r = r.min(rows - 1);
            c = c.min(cols - 1);
            egui::ComboBox::from_id_salt(("mesh_row", id.clone()))
                .selected_text(format!("row {r}"))
                .width(70.0)
                .show_ui(ui, |ui| {
                    for i in 0..rows {
                        if ui.selectable_label(r == i, format!("row {i}")).clicked() {
                            r = i;
                        }
                    }
                });
            egui::ComboBox::from_id_salt(("mesh_col", id.clone()))
                .selected_text(format!("col {c}"))
                .width(70.0)
                .show_ui(ui, |ui| {
                    for i in 0..cols {
                        if ui.selectable_label(c == i, format!("col {i}")).clicked() {
                            c = i;
                        }
                    }
                });
            ui.memory_mut(|m| m.data.insert_temp(mem_id, (r, c)));
            node_rc = Some((r, c));
        });
        if let Some((r, c)) = node_rc {
            let (mut nx, mut ny, ncol) = match state.document.find_object(&id) {
                Some(obj) => {
                    if let crate::core::document::ObjectType::GradientMesh(m) = &obj.object_type {
                        let n = m.node(r, c);
                        (n.x, n.y, n.color)
                    } else {
                        return;
                    }
                }
                None => return,
            };
            let mut ncolor = [
                (ncol[0] * 255.0) as u8,
                (ncol[1] * 255.0) as u8,
                (ncol[2] * 255.0) as u8,
                (ncol[3] * 255.0) as u8,
            ];
            ui.horizontal(|ui| {
                ui.label("X:");
                let x_resp = ui.add(egui::DragValue::new(&mut nx).speed(1.0));
                ui.label("Y:");
                let y_resp = ui.add(egui::DragValue::new(&mut ny).speed(1.0));
                if x_resp.changed() || y_resp.changed() {
                    state.ensure_object_snapshot(&id);
                    if let Some(o) = state.document.find_object_mut(&id) {
                        if let crate::core::document::ObjectType::GradientMesh(m) = &mut o.object_type {
                            let n = m.node_mut(r, c);
                            n.x = nx;
                            n.y = ny;
                        }
                    }
                    if !x_resp.dragged() && !y_resp.dragged() {
                        state.commit_object_edits("Edit Mesh Node");
                    }
                }
                if x_resp.drag_stopped() || y_resp.drag_stopped() {
                    state.commit_object_edits("Edit Mesh Node");
                }
            });
            ui.horizontal(|ui| {
                ui.label("カラー:");
                if color_edit_srgba_u8(ui, &mut ncolor)
                    .changed()
                {
                    let col = [
                        ncolor[0] as f32 / 255.0,
                        ncolor[1] as f32 / 255.0,
                        ncolor[2] as f32 / 255.0,
                        ncolor[3] as f32 / 255.0,
                    ];
                    state.ensure_object_snapshot(&id);
                    if let Some(o) = state.document.find_object_mut(&id) {
                        if let crate::core::document::ObjectType::GradientMesh(m) = &mut o.object_type {
                            m.node_mut(r, c).color = col;
                        }
                    }
                    state.commit_object_edits("Edit Mesh Color");
                }
            });
        }
    }
}

pub struct AxonometricPanel;

impl AxonometricPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("軸測投影（アーキテクチャ）").strong());
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
                RichText::new("軸測投影するオブジェクトを選択してください")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct RevolvePanel;

impl RevolvePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("3D Revolve & Lathe Modeler").strong());
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
                        let _ = crate::io::atomic::atomic_write_str(&path, &obj_data);
                    }
                }
            }
        } else {
            ui.label(
                RichText::new("3D回転させるプロファイルパスを選択してください")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

pub struct EnvelopePanel;

impl EnvelopePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("エンベロープ歪曲・シェイプモールド").strong());
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
                RichText::new("アートとエンベロープ枠の2つを選択してください")
                    .weak()
                    .size(11.0),
            );
        }

        ui.add_space(4.0);
        ui.separator();
        ui.label(RichText::new("ライブワープ（非破壊）").strong());

        // If a live envelope is selected: edit kind/amount or release.
        let live_id = state.selected_ids.iter().find_map(|id| {
            state.document.find_object(id).and_then(|o| match &o.object_type {
                crate::core::document::ObjectType::Envelope { .. } => Some(id.clone()),
                _ => None,
            })
        });
        if let Some(id) = live_id {
            let (kind, amount) = match state.document.find_object(&id) {
                Some(o) => match &o.object_type {
                    crate::core::document::ObjectType::Envelope { kind, amount, .. } => (*kind, *amount),
                    _ => return,
                },
                None => return,
            };
            ui.horizontal(|ui| {
                for k in [
                    crate::core::envelope::EnvelopeKind::Bulge,
                    crate::core::envelope::EnvelopeKind::Pinch,
                    crate::core::envelope::EnvelopeKind::Twist,
                    crate::core::envelope::EnvelopeKind::Wave,
                ] {
                    if ui.selectable_label(kind == k, k.name()).clicked() {
                        state.ensure_object_snapshot(&id);
                        if let Some(o) = state.document.find_object_mut(&id) {
                            if let crate::core::document::ObjectType::Envelope { kind: kk, .. } =
                                &mut o.object_type
                            {
                                *kk = k;
                            }
                        }
                        state.commit_object_edits("Edit Envelope");
                    }
                }
            });
            let mut amt = amount;
            let amt_resp = ui.add(egui::Slider::new(&mut amt, -1.0..=1.0).text("Amount"));
            if amt_resp.changed() {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    if let crate::core::document::ObjectType::Envelope { amount: aa, .. } =
                        &mut o.object_type
                    {
                        *aa = amt.clamp(-1.0, 1.0);
                    }
                }
                if !amt_resp.dragged() {
                    state.commit_object_edits("Edit Envelope");
                }
            }
            if amt_resp.drag_stopped() {
                state.commit_object_edits("Edit Envelope");
            }
            if ui.button("リリース（ソースパスを復元）").clicked() {
                let source = state.document.find_object(&id).and_then(|o| match &o.object_type {
                    crate::core::document::ObjectType::Envelope { source, .. } => {
                        Some(source.as_ref().clone())
                    }
                    _ => None,
                });
                if let Some(mut src) = source {
                    state.ensure_object_snapshot(&id);
                    // Swap in place: keep id/position, drop the deform.
                    if let Some(o) = state.document.find_object_mut(&id) {
                        src.id.clone_from(&o.id);
                        src.name = format!("{} (Released)", o.name);
                        *o = src;
                    }
                    state.commit_object_edits("Release Envelope");
                }
            }
        } else {
            // Wrap the selection in a new live envelope.
            let wrappable = state.selected_ids.iter().any(|id| {
                state.document.find_object(id).map(|o| {
                    matches!(
                        o.object_type,
                        crate::core::document::ObjectType::Path(_)
                            | crate::core::document::ObjectType::Rectangle { .. }
                            | crate::core::document::ObjectType::Ellipse { .. }
                            | crate::core::document::ObjectType::Star { .. }
                            | crate::core::document::ObjectType::Polygon { .. }
                            | crate::core::document::ObjectType::Line { .. }
                    )
                }).unwrap_or(false)
            });
            if ui
                .add_enabled(wrappable, egui::Button::new("Wrap Selection in Live Warp"))
                .clicked()
            {
                let ids: Vec<String> = state.selected_ids.clone();
                for sid in ids {
                    let src = state.document.find_object(&sid).cloned();
                    if let Some(src) = src {
                        if let Some(mut env) = crate::core::document::Object::wrap_envelope(
                            &format!("{} (Warp)", src.name),
                            &src,
                            crate::core::envelope::EnvelopeKind::Bulge,
                            0.5,
                        ) {
                            // Replace in place (same layer/index) so z-order
                            // survives; one undo step restores the source.
                            state.ensure_object_snapshot(&sid);
                            if let Some(o) = state.document.find_object_mut(&sid) {
                                env.id.clone_from(&o.id);
                                *o = env;
                            }
                            state.commit_object_edits("Wrap Envelope");
                        }
                    }
                }
            }
            if !wrappable {
                ui.label(
                    RichText::new(
                        "ライブワープするパス／図形を選択してください（テキスト・画像は対象外）",
                    )
                    .weak()
                    .size(11.0),
                );
            }
        }
    }
}

pub struct PolarPanel;

impl PolarPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("極座標・プラネットラップ").strong());
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
                RichText::new("極座標に巻きつけるオブジェクトを選択してください")
                    .weak()
                    .size(11.0),
            );
        }
    }
}
