use super::common::{load_any_document, save_any_document};
use crate::cli::types::*;
use std::path::{Path, PathBuf};

pub fn handle_revolve(
    input: &Path,
    angle: f64,
    segments: usize,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🏺 Revolving '{:?}' in 3D (angle: {} deg, segs: {})...",
        input, angle, segments
    );
    let doc = load_any_document(input)?;
    let profile_obj = doc
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Profile document is empty")?;
    let axis_x = profile_obj
        .bounding_box()
        .map(|(min, _)| min.x)
        .unwrap_or(0.0);
    let obj_data =
        crate::core::revolve::generate_3d_revolve_obj(profile_obj, axis_x, angle, segments);
    std::fs::write(output, obj_data)?;
    println!("✅ 3D Revolved OBJ mesh saved to {:?}", output);
    Ok(false)
}

pub fn handle_envelope(
    art: &Path,
    envelope: &Path,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🚩 Molding '{:?}' into Envelope frame '{:?}'...",
        art, envelope
    );
    let art_doc = load_any_document(art)?;
    let env_doc = load_any_document(envelope)?;

    let art_obj = art_doc
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Art document is empty")?;
    let env_obj = env_doc
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Envelope document is empty")?;

    let warped = crate::core::envelope::apply_envelope_distort(art_obj, env_obj);
    let mut out_doc = crate::core::document::Document {
        width: art_doc.width.max(env_doc.width),
        height: art_doc.height.max(env_doc.height),
        ..Default::default()
    };
    out_doc.add_object(warped);
    save_any_document(&out_doc, output)?;
    println!("✅ Envelope distorted vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_polar(input: &Path, output: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🌐 Transforming '{:?}' to Polar Coordinates...", input);
    let doc = load_any_document(input)?;
    let cx = doc.width * 0.5;
    let cy = doc.height * 0.5;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let polar = crate::core::polar::apply_polar_transform(
            obj,
            cx,
            cy,
            doc.width,
            doc.height,
            crate::core::polar::PolarMode::RectToPolar,
        );
        out_doc.add_object(polar);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Polar coordinates vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_slice(input: &Path, output: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    println!("✂ Slicing '{:?}' in half...", input);
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        if let Some((min, max)) = obj.bounding_box() {
            let mid_y = (min.y + max.y) * 0.5;
            let p1 = crate::core::path::AnchorPoint::new(min.x - 10.0, mid_y);
            let p2 = crate::core::path::AnchorPoint::new(max.x + 10.0, mid_y);
            if let Some((pa, pb)) = crate::core::knife::slice_object_with_line(obj, p1, p2) {
                out_doc.add_object(pa);
                out_doc.add_object(pb);
            } else {
                out_doc.add_object(obj.clone());
            }
        } else {
            out_doc.add_object(obj.clone());
        }
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Sliced vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_outline(
    input: &Path,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🔤 Creating Outlines from text in '{:?}'...", input);
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        if let Some(outlined) = crate::core::text_path::create_text_outlines(obj) {
            out_doc.add_object(outlined);
        } else {
            out_doc.add_object(obj.clone());
        }
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Outlined vector artwork saved to {:?}", output);
    Ok(false)
}

pub fn handle_text_path(
    path: &Path,
    text: &str,
    font_size: f64,
    offset: f64,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("〰 Placing text '{}' along path '{:?}'...", text, path);
    let pdoc = load_any_document(path)?;
    let objs: Vec<&crate::core::document::Object> = pdoc.all_objects().map(|(_, o)| o).collect();
    if objs.is_empty() {
        return Err("Trajectory file contains no vector objects".into());
    }
    let traj_path = objs[0].to_path_data();
    let outlined_path =
        crate::core::text_path::text_on_path_to_outlines(&traj_path, text, font_size, offset);
    let mut out_doc = crate::core::document::Document {
        width: pdoc.width,
        height: pdoc.height,
        ..Default::default()
    };
    let mut out_obj =
        crate::core::document::Object::new_path(&format!("TextOnPath_{}", text), outlined_path);
    out_obj.fill = Some(crate::core::path::FillStyle::solid([0.1, 0.1, 0.1, 1.0]));
    out_doc.add_object(out_obj);
    save_any_document(&out_doc, output)?;
    println!("✅ Text on path vector saved to {:?}", output);
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
pub fn handle_effect(
    input: &Path,
    shadow: bool,
    shadow_x: f64,
    shadow_y: f64,
    shadow_blur: f64,
    glow: bool,
    glow_radius: f64,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("✨ Applying vector appearance effects to '{:?}'...", input);
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let mut modified = obj.clone();
        if shadow {
            modified.shadow = Some(crate::core::effects::DropShadow {
                offset_x: shadow_x,
                offset_y: shadow_y,
                blur_radius: shadow_blur,
                color: [0.0, 0.0, 0.0, 1.0],
                opacity: 0.5,
            });
        }
        if glow {
            modified.glow = Some(crate::core::effects::GlowEffect {
                radius: glow_radius,
                color: [0.0, 0.8, 1.0, 1.0],
                intensity: 0.7,
            });
        }
        out_doc.add_object(modified);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Vector artwork with effects saved to {:?}", output);
    Ok(false)
}

pub fn handle_shape_build(
    input: &Path,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "⯐ Decomposing overlapping shapes into fragments from '{:?}'...",
        input
    );
    let doc = load_any_document(input)?;
    let objs: Vec<&crate::core::document::Object> = doc.all_objects().map(|(_, o)| o).collect();
    if objs.is_empty() {
        return Err("Input file contains no vector objects".into());
    }

    let fragments = crate::core::shape_builder::decompose_shapes_into_fragments(&objs);
    println!("   Generated {} atomic shape fragments", fragments.len());

    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };

    for (i, frag) in fragments.iter().enumerate() {
        let base_idx = frag.original_object_indices.first().copied().unwrap_or(0);
        let base_obj = objs[base_idx];
        let mut frag_obj = crate::core::shape_builder::fragment_to_object(frag, base_obj);
        frag_obj.name = format!("Fragment_{}", i + 1);
        out_doc.add_object(frag_obj);
    }

    save_any_document(&out_doc, output)?;
    println!(
        "✅ Decomposed shape builder fragments saved to {:?}",
        output
    );
    Ok(false)
}

pub fn handle_compound(
    input: &Path,
    output: &Path,
    release: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "⛓ Processing Compound Path operation on '{:?}' (release: {})...",
        input, release
    );
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };

    if release {
        for (_, obj) in doc.all_objects() {
            let parts = obj.release_compound_path();
            for p in parts {
                out_doc.add_object(p);
            }
        }
        println!("   Released compound paths into independent elements");
    } else {
        let objs: Vec<crate::core::document::Object> =
            doc.all_objects().map(|(_, o)| o.clone()).collect();
        if objs.is_empty() {
            return Err("Input file contains no objects to combine into compound path".into());
        }
        if let Some(compound) = crate::core::document::Object::make_compound_path(&objs) {
            out_doc.add_object(compound);
            println!(
                "   Combined {} objects into a single Compound Path with holes",
                objs.len()
            );
        } else {
            return Err("Failed to create compound path".into());
        }
    }

    save_any_document(&out_doc, output)?;
    println!("✅ Compound path result saved to {:?}", output);
    Ok(false)
}

pub fn handle_boolean(
    input1: &Path,
    input2: &Path,
    op: CliBooleanOp,
    output: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "✂ Running Pathfinder ({:?}) on '{:?}' and '{:?}'...",
        op, input1, input2
    );
    let doc1 = load_any_document(input1)?;
    let doc2 = load_any_document(input2)?;

    let objs1: Vec<&crate::core::document::Object> = doc1.all_objects().map(|(_, o)| o).collect();
    let objs2: Vec<&crate::core::document::Object> = doc2.all_objects().map(|(_, o)| o).collect();

    if objs1.is_empty() || objs2.is_empty() {
        return Err("Both input documents must contain at least one vector object".into());
    }

    let bool_op = match op {
        CliBooleanOp::Union => crate::core::boolean::BooleanOp::Union,
        CliBooleanOp::Subtract => crate::core::boolean::BooleanOp::Subtract,
        CliBooleanOp::Intersect => crate::core::boolean::BooleanOp::Intersect,
        CliBooleanOp::Exclude => crate::core::boolean::BooleanOp::Exclude,
    };

    let combined_refs: Vec<&crate::core::document::Object> = vec![objs1[0], objs2[0]];
    if let Some(res_obj) = crate::core::boolean::execute_pathfinder(&combined_refs, bool_op) {
        let mut out_doc = crate::core::document::Document {
            width: doc1.width.max(doc2.width),
            height: doc1.height.max(doc2.height),
            ..Default::default()
        };
        out_doc.add_object(res_obj);
        save_any_document(&out_doc, output)?;
        println!("✅ Pathfinder result saved to {:?}", output);
    } else {
        println!("⚠️ Pathfinder produced empty geometry.");
    }
    Ok(false)
}

pub fn handle_info(input: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let doc = load_any_document(input)?;
    println!("📊 === IRASU Illustrator Document Info ===");
    println!("  Name: {}", doc.name);
    println!("  Canvas Size: {} × {} px", doc.width, doc.height);
    println!("  Layers ({}):", doc.layers.len());
    for (i, layer) in doc.layers.iter().enumerate() {
        println!(
            "    [{}] Layer '{}' (visible: {}, locked: {}, opacity: {:.2}) - {} objects",
            i,
            layer.name,
            layer.visible,
            layer.locked,
            layer.opacity,
            layer.objects.len()
        );
        for (j, obj) in layer.objects.iter().enumerate() {
            let bb_str = obj
                .bounding_box()
                .map(|(min, max)| {
                    format!(
                        "bounds: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                        min.x, min.y, max.x, max.y
                    )
                })
                .unwrap_or_else(|| "no bounds".to_string());
            println!(
                "      - ({}) '{}' [{:?}] (opacity: {:.2}, {})",
                j,
                obj.name,
                std::mem::discriminant(&obj.object_type),
                obj.opacity,
                bb_str
            );
        }
    }
    Ok(false)
}

pub fn handle_script(
    script: &Path,
    output: Option<PathBuf>,
    input: Option<PathBuf>,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("📜 Executing Rhai script '{:?}'...", script);
    let mut doc = match input {
        Some(path) => load_any_document(&path)?,
        None => crate::core::document::Document::default(),
    };
    let mut state = crate::core::state::AppState::default();
    let engine = crate::plugin::script::ScriptEngine::new();
    engine.run_script(
        &std::fs::read_to_string(script).map_err(|e| format!("Failed to read script: {}", e))?,
        &mut doc,
        &mut state,
    )?;
    if let Some(out_path) = output {
        save_any_document(&doc, &out_path)?;
        println!("✅ Script result saved to {:?}", out_path);
    } else {
        let tmp = std::env::temp_dir().join("irasu_script_output.json");
        save_any_document(&doc, &tmp)?;
        println!("📄 Script result saved to temp: {:?}", tmp);
        println!("   Run with --output to specify output path, or open in GUI.");
    }
    Ok(false)
}

pub fn handle_plugins(info: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🧩 IRASU Plugin System");
    println!("   Plugins are loaded from ~/.irasu/plugins/ or ./plugins/");
    if let Some(plugin_id) = info {
        println!("   Plugin info for '{}':", plugin_id);
        println!("   (Plugin discovery not yet implemented — use the GUI Plugin Manager)");
    } else {
        println!("   No plugins loaded (headless mode). Use the GUI to manage plugins.");
        println!("   Plugin API: implement the `Plugin` trait from irasu_illustrator::plugin::api");
    }
    Ok(false)
}

pub fn handle_serve(port: u16, input: Option<PathBuf>) -> Result<bool, Box<dyn std::error::Error>> {
    println!("🌐 IRASU API Server starting on port {}...", port);
    let doc = match input {
        Some(path) => load_any_document(&path)?,
        None => crate::core::document::Document::default(),
    };
    println!(
        "   Loaded document: '{}' ({} × {} px)",
        doc.name, doc.width, doc.height
    );
    println!("   API endpoints:");
    println!("     GET  /api/document       — Get document info");
    println!("     GET  /api/objects        — List all objects");
    println!("     POST /api/objects/rect   — Create rectangle");
    println!("     POST /api/objects/ellipse — Create ellipse");
    println!("     POST /api/objects/path   — Create path");
    println!("     POST /api/script          — Execute Rhai script");
    println!("     POST /api/export/svg      — Export to SVG");
    println!("   (HTTP server requires `tiny_http` or `axum` dependency — currently stub)");
    println!("   For full API server, add `axum` to Cargo.toml and implement routes.");
    Ok(false)
}
