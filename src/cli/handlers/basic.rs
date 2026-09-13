use super::common::{load_any_document, save_any_document, save_any_document_scaled};
use crate::cli::types::*;
use std::path::PathBuf;

pub fn handle_convert(
    input: &PathBuf,
    output: &PathBuf,
    scale: f32,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🔄 Converting '{:?}' to '{:?}' (scale: {}x)...",
        input, output, scale
    );
    let doc = load_any_document(input)?;
    save_any_document_scaled(&doc, output, scale)?;
    println!("✅ Conversion complete: {:?}", output);
    Ok(false)
}

pub fn handle_export_vfx(
    input: &PathBuf,
    output: &PathBuf,
    fps: f64,
    duration: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🎬 Exporting to AEVFX Studio Comp: '{:?}' ({} fps, {}s)...",
        output, fps, duration
    );
    let doc = load_any_document(input)?;
    let vfx_comp = crate::io::vfx::doc_to_aevfx_comp(&doc, fps, duration);
    let json = serde_json::to_string_pretty(&vfx_comp)?;
    std::fs::write(output, json)?;
    println!(
        "✅ Exported {} VFX layers to {:?}",
        vfx_comp.layers.len(),
        output
    );
    Ok(false)
}

pub fn handle_export_3d(
    input: &PathBuf,
    output: &PathBuf,
    depth: f64,
    bevel: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🧱 Exporting to 3D OBJ Mesh (depth: {}, bevel: {}): '{:?}'...",
        depth, bevel, output
    );
    let doc = load_any_document(input)?;
    let obj_str = crate::io::vfx::export_doc_to_obj(&doc, depth, bevel);
    std::fs::write(output, obj_str)?;
    println!("✅ 3D OBJ Mesh exported to {:?}", output);
    Ok(false)
}

pub fn handle_export_pdf(
    input: &PathBuf,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("📄 Exporting '{:?}' to Pure Vector PDF...", input);
    let doc = load_any_document(input)?;
    let pdf_bytes = crate::io::pdf::export_pdf(&doc);
    std::fs::write(output, pdf_bytes)?;
    println!("✅ Vector PDF exported successfully to {:?}", output);
    Ok(false)
}

pub fn handle_morph(
    input1: &PathBuf,
    input2: &PathBuf,
    t: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🧬 Morphing '{:?}' and '{:?}' at t = {} -> '{:?}'...",
        input1, input2, t, output
    );
    let doc1 = load_any_document(input1)?;
    let doc2 = load_any_document(input2)?;

    let obj1 = doc1
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Input 1 contains no objects")?;
    let obj2 = doc2
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Input 2 contains no objects")?;

    let mut p1 = obj1.to_path_data();
    p1.transform(&obj1.transform.matrix());
    let mut p2 = obj2.to_path_data();
    p2.transform(&obj2.transform.matrix());

    let morphed_path = crate::core::morph::morph_paths(&p1, &p2, t);
    let morphed_obj = crate::core::document::Object::new_path("Morphed Shape", morphed_path);

    let mut out_doc = crate::core::document::Document {
        width: doc1.width.max(doc2.width),
        height: doc1.height.max(doc2.height),
        ..Default::default()
    };
    out_doc.add_object(morphed_obj);
    save_any_document(&out_doc, output)?;
    println!("✅ Morphed shape saved to {:?}", output);
    Ok(false)
}

pub fn handle_offset(
    input: &PathBuf,
    delta: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "📐 Offsetting path in '{:?}' by {}px -> '{:?}'...",
        input, delta, output
    );
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };

    for (_, obj) in doc.all_objects() {
        let path = obj.to_path_data();
        let off_path = crate::core::offset::offset_path(&path, delta);
        let mut new_obj =
            crate::core::document::Object::new_path(&format!("{} (Offset)", obj.name), off_path);
        new_obj.transform = obj.transform.clone();
        out_doc.add_object(new_obj);
    }

    save_any_document(&out_doc, output)?;
    println!("✅ Offset path saved to {:?}", output);
    Ok(false)
}

pub fn handle_outline_stroke(
    input: &PathBuf,
    width: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🖋 Outlining strokes in '{:?}' (width: {}px) -> '{:?}'...",
        input, width, output
    );
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };

    for (_, obj) in doc.all_objects() {
        let path = obj.to_path_data();
        let outlined = crate::core::offset::outline_stroke(&path, width);
        let mut new_obj =
            crate::core::document::Object::new_path(&format!("{} (Outlined)", obj.name), outlined);
        new_obj.transform = obj.transform.clone();
        out_doc.add_object(new_obj);
    }

    save_any_document(&out_doc, output)?;
    println!("✅ Outlined stroke saved to {:?}", output);
    Ok(false)
}

pub fn handle_motion_path(
    input: &PathBuf,
    output: &PathBuf,
    samples: usize,
    duration: f64,
    fps: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🚀 Generating Motion Path Keyframes ({} samples) from '{:?}'...",
        samples, input
    );
    let doc = load_any_document(input)?;
    let mut all_trajectories = Vec::new();

    for layer in &doc.layers {
        for obj in &layer.objects {
            let kfs = crate::io::vfx::object_to_motion_path_keyframes(obj, samples, duration, fps);
            if !kfs.is_empty() {
                all_trajectories.push(serde_json::json!({
                    "object_id": obj.id,
                    "object_name": obj.name,
                    "keyframes": kfs
                }));
            }
        }
    }

    let json = serde_json::to_string_pretty(&all_trajectories)?;
    std::fs::write(output, json)?;
    println!(
        "✅ Generated {} motion paths into {:?}",
        all_trajectories.len(),
        output
    );
    Ok(false)
}

pub fn handle_trace(
    input: &PathBuf,
    threshold: u8,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🖼️ Auto-tracing image '{:?}' (threshold: {}) -> '{:?}'...",
        input, threshold, output
    );
    let img = image::open(input)?;
    let gray = img.to_luma8();
    let w = gray.width() as usize;
    let h = gray.height() as usize;
    let path_data = crate::core::trace::trace_bitmap_to_path(w, h, gray.as_raw(), threshold);
    let mut doc = crate::core::document::Document {
        width: w as f64,
        height: h as f64,
        ..Default::default()
    };
    doc.add_object(crate::core::document::Object::new_path(
        "Traced Image",
        path_data,
    ));
    save_any_document(&doc, output)?;
    println!("✅ Auto-traced vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_animate(
    input: &PathBuf,
    output: &PathBuf,
    fps: f64,
    duration: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🎬 Generating animated AEVFX Studio Comp from '{:?}' ({} fps, {}s)...",
        input, fps, duration
    );
    let doc = load_any_document(input)?;
    let vfx_comp = crate::io::vfx::doc_to_aevfx_comp(&doc, fps, duration);
    let json = serde_json::to_string_pretty(&vfx_comp)?;
    std::fs::write(output, json)?;
    println!("✅ Animated composition saved to {:?}", output);
    Ok(false)
}

pub fn handle_formula(
    curve_type: CliCurveType,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🌀 Generating mathematical curve ({:?}) -> '{:?}'...",
        curve_type, output
    );
    let cx = 400.0;
    let cy = 300.0;
    let path = match curve_type {
        CliCurveType::Spiral => {
            crate::core::formula::FormulaCurves::spiral(cx, cy, 4.0, 10.0, 4.0, 200)
        }
        CliCurveType::Lissajous => {
            crate::core::formula::FormulaCurves::lissajous(cx, cy, 3.0, 2.0, 0.5, 300.0, 200.0, 240)
        }
        CliCurveType::Spirograph => {
            crate::core::formula::FormulaCurves::spirograph(cx, cy, 140.0, 60.0, 80.0, 8, 48)
        }
        CliCurveType::Rose => {
            crate::core::formula::FormulaCurves::rose_curve(cx, cy, 4.0, 120.0, 200)
        }
    };
    let mut doc = crate::core::document::Document {
        width: 800.0,
        height: 600.0,
        ..Default::default()
    };
    doc.add_object(crate::core::document::Object::new_path(
        "Formula Curve",
        path,
    ));
    save_any_document(&doc, output)?;
    println!("✅ Formula curve saved to {:?}", output);
    Ok(false)
}

pub fn handle_vfx_trail(
    input: &PathBuf,
    count: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "⚡ Generating {} VFX particle trails from '{:?}'...",
        count, input
    );
    let doc = load_any_document(input)?;
    let mut all_particles = Vec::new();
    for (_, obj) in doc.all_objects() {
        let mut path = obj.to_path_data();
        path.transform(&obj.transform.matrix());
        let p = crate::core::vfx_particles::generate_particle_trail(&path, count, 60.0, 12.0);
        all_particles.extend(p);
    }
    let json = serde_json::to_string_pretty(&all_particles)?;
    std::fs::write(output, json)?;
    println!(
        "✅ Exported {} VFX particles to {:?}",
        all_particles.len(),
        output
    );
    Ok(false)
}

pub fn handle_halftone(
    input: &PathBuf,
    spacing: f64,
    radius: f64,
    pattern: CliHalftonePattern,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🏁 Generating halftone dots from '{:?}' (spacing: {}, radius: {})...",
        input, spacing, radius
    );
    let doc = load_any_document(input)?;
    let ht_pat = match pattern {
        CliHalftonePattern::Circular => crate::core::halftone::HalftonePattern::CircularGrid,
        CliHalftonePattern::Hex => crate::core::halftone::HalftonePattern::HexagonalGrid,
        CliHalftonePattern::Scanline => crate::core::halftone::HalftonePattern::ScanlineMatrix,
    };
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let mut path = obj.to_path_data();
        path.transform(&obj.transform.matrix());
        let ht_path =
            crate::core::halftone::generate_halftone_from_path(&path, spacing, radius, ht_pat);
        out_doc.add_object(crate::core::document::Object::new_path(
            &format!("{} (Halftone)", obj.name),
            ht_path,
        ));
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Halftone vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_simplify(
    input: &PathBuf,
    tolerance: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🪄 Simplifying paths in '{:?}' (tolerance: {})...",
        input, tolerance
    );
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let path = obj.to_path_data();
        let simplified = crate::core::simplify::simplify_path_visvalingam(&path, tolerance);
        let mut new_obj = crate::core::document::Object::new_path(
            &format!("{} (Simplified)", obj.name),
            simplified,
        );
        new_obj.transform = obj.transform.clone();
        out_doc.add_object(new_obj);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Simplified vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_isometric(
    input: &PathBuf,
    plane: CliIsoPlane,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("📐 Projecting '{:?}' to Isometric {:?}...", input, plane);
    let doc = load_any_document(input)?;
    let iso_plane = match plane {
        CliIsoPlane::Top => crate::core::isometric::IsometricPlane::Top,
        CliIsoPlane::Left => crate::core::isometric::IsometricPlane::Left,
        CliIsoPlane::Right => crate::core::isometric::IsometricPlane::Right,
    };
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let iso_obj = crate::core::isometric::apply_isometric_transform(obj, iso_plane);
        out_doc.add_object(iso_obj);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Isometric vector saved to {:?}", output);
    Ok(false)
}
