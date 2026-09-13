use super::common::{load_any_document, save_any_document};
use crate::cli::types::*;
use std::path::PathBuf;

pub fn handle_voronoi(
    cells: usize,
    padding: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🔷 Generating Voronoi mosaic ({} cells, {}px padding)...",
        cells, padding
    );
    let w = 800.0;
    let h = 600.0;
    let mut seeds = Vec::with_capacity(cells);
    for i in 0..cells {
        let hx = ((i as f64 * 37.123 + 12.34).sin() * 43758.5453)
            .fract()
            .abs();
        let hy = ((i as f64 * 91.567 + 84.12).sin() * 43758.5453)
            .fract()
            .abs();
        seeds.push(crate::core::path::AnchorPoint::new(hx * w, hy * h));
    }
    let cell_objs = crate::core::voronoi::generate_voronoi_cells(w, h, &seeds, padding);
    let mut out_doc = crate::core::document::Document {
        width: w,
        height: h,
        ..Default::default()
    };
    for obj in cell_objs {
        out_doc.add_object(obj);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Voronoi mosaic saved to {:?}", output);
    Ok(false)
}

pub fn handle_lsystem(
    preset: CliLSystemPreset,
    iterations: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🌿 Generating L-System {:?} ({} iterations)...",
        preset, iterations
    );
    let lpreset = match preset {
        CliLSystemPreset::Tree => crate::core::lsystem::LSystemPreset::Tree,
        CliLSystemPreset::Dragon => crate::core::lsystem::LSystemPreset::Dragon,
        CliLSystemPreset::Snowflake => crate::core::lsystem::LSystemPreset::Snowflake,
        CliLSystemPreset::Hilbert => crate::core::lsystem::LSystemPreset::Hilbert,
    };
    let path = crate::core::lsystem::generate_lsystem(lpreset, iterations, 400.0, 400.0, 10.0);
    let mut out_doc = crate::core::document::Document {
        width: 800.0,
        height: 800.0,
        ..Default::default()
    };
    out_doc.add_object(crate::core::document::Object::new_path(
        "L-System Fractal",
        path,
    ));
    save_any_document(&out_doc, output)?;
    println!("✅ L-System fractal saved to {:?}", output);
    Ok(false)
}

pub fn handle_qr(text: &str, output: &PathBuf) -> Result<bool, Box<dyn std::error::Error>> {
    println!("📱 Generating Vector QR Code for '{}'...", text);
    let path = crate::core::barcode::generate_vector_qr(text, 200.0, 200.0, 360.0)?;
    let mut out_doc = crate::core::document::Document {
        width: 400.0,
        height: 400.0,
        ..Default::default()
    };
    out_doc.add_object(crate::core::document::Object::new_path(
        "Vector QR Code",
        path,
    ));
    save_any_document(&out_doc, output)?;
    println!("✅ Vector QR code saved to {:?}", output);
    Ok(false)
}

pub fn handle_deform(
    input: &PathBuf,
    output: &PathBuf,
    deform_type: CliDeformType,
    amplitude: f64,
    frequency: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🌊 Deforming '{:?}' ({:?}, amp: {}, freq: {})...",
        input, deform_type, amplitude, frequency
    );
    let doc = load_any_document(input)?;
    let dtype = match deform_type {
        CliDeformType::Wave => crate::core::noise::DeformType::SineWave,
        CliDeformType::Noise => crate::core::noise::DeformType::TurbulentNoise,
        CliDeformType::Glitch => crate::core::noise::DeformType::JitterGlitch,
    };
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let path = obj.to_path_data();
        let def_path = crate::core::noise::deform_path(&path, dtype, amplitude, frequency, 0.0);
        let mut new_obj =
            crate::core::document::Object::new_path(&format!("{} (Deformed)", obj.name), def_path);
        new_obj.transform = obj.transform.clone();
        out_doc.add_object(new_obj);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Deformed vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_flowfield(
    preset: CliFlowFieldPreset,
    lines: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🌌 Generating Flow Field Streamlines ({:?}, {} lines)...",
        preset, lines
    );
    let fpreset = match preset {
        CliFlowFieldPreset::Vortex => crate::core::flowfield::FlowFieldPreset::Vortex,
        CliFlowFieldPreset::Magnetic => crate::core::flowfield::FlowFieldPreset::MagneticDipole,
        CliFlowFieldPreset::Cyber => crate::core::flowfield::FlowFieldPreset::CyberChaos,
    };
    let mut out_doc = crate::core::document::Document {
        width: 800.0,
        height: 600.0,
        ..Default::default()
    };
    let streamlines = crate::core::flowfield::generate_flowfield_streamlines(
        fpreset, 800.0, 600.0, lines, 80, 5.0,
    );
    for line in streamlines {
        out_doc.add_object(line);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Flow field streamlines saved to {:?}", output);
    Ok(false)
}

pub fn handle_brush_stroke(
    path: &PathBuf,
    motif: &PathBuf,
    spacing: f64,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🖌️ Scattering motif '{:?}' along path '{:?}' (spacing: {})...",
        motif, path, spacing
    );
    let path_doc = load_any_document(path)?;
    let motif_doc = load_any_document(motif)?;

    let path_obj = path_doc
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Path document is empty")?;
    let motif_obj = motif_doc
        .all_objects()
        .next()
        .map(|(_, o)| o)
        .ok_or("Motif document is empty")?;

    let mut traj = path_obj.to_path_data();
    traj.transform(&path_obj.transform.matrix());

    let clones = crate::core::brush::scatter_brush_along_path(&traj, motif_obj, spacing, 0.2, true);
    let mut out_doc = crate::core::document::Document {
        width: path_doc.width.max(motif_doc.width),
        height: path_doc.height.max(motif_doc.height),
        ..Default::default()
    };
    for clone in clones {
        out_doc.add_object(clone);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Scattered brush stroke saved to {:?}", output);
    Ok(false)
}

pub fn handle_blend(
    base: &PathBuf,
    blend: &PathBuf,
    mode: CliBlendMode,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🎨 Blending '{:?}' over '{:?}' ({:?})...",
        blend, base, mode
    );
    let mut base_doc = load_any_document(base)?;
    let blend_doc = load_any_document(blend)?;

    let blend_mode = match mode {
        CliBlendMode::Normal => crate::core::blend::BlendMode::Normal,
        CliBlendMode::Multiply => crate::core::blend::BlendMode::Multiply,
        CliBlendMode::Screen => crate::core::blend::BlendMode::Screen,
        CliBlendMode::Overlay => crate::core::blend::BlendMode::Overlay,
        CliBlendMode::Darken => crate::core::blend::BlendMode::Darken,
        CliBlendMode::Lighten => crate::core::blend::BlendMode::Lighten,
        CliBlendMode::ColorDodge => crate::core::blend::BlendMode::ColorDodge,
        CliBlendMode::ColorBurn => crate::core::blend::BlendMode::ColorBurn,
        CliBlendMode::HardLight => crate::core::blend::BlendMode::HardLight,
        CliBlendMode::SoftLight => crate::core::blend::BlendMode::SoftLight,
        CliBlendMode::Difference => crate::core::blend::BlendMode::Difference,
        CliBlendMode::Exclusion => crate::core::blend::BlendMode::Exclusion,
    };

    for (_, obj) in blend_doc.all_objects() {
        let mut blended_obj = obj.clone();
        if let Some(fill) = &mut blended_obj.fill {
            fill.color =
                crate::core::blend::blend_colors(fill.color, [1.0, 1.0, 1.0, 1.0], blend_mode);
        }
        base_doc.add_object(blended_obj);
    }

    save_any_document(&base_doc, output)?;
    println!("✅ Blended vector artwork saved to {:?}", output);
    Ok(false)
}

pub fn handle_audio_wave(
    wave_type: CliWaveformType,
    freq: f64,
    harmonics: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🎵 Generating Audio Waveform ({:?}, freq: {}, harmonics: {})...",
        wave_type, freq, harmonics
    );
    let wtype = match wave_type {
        CliWaveformType::Sine => crate::core::audio_curve::WaveformType::Sine,
        CliWaveformType::Sawtooth => crate::core::audio_curve::WaveformType::Sawtooth,
        CliWaveformType::Square => crate::core::audio_curve::WaveformType::Square,
        CliWaveformType::Triangle => crate::core::audio_curve::WaveformType::Triangle,
        CliWaveformType::Harmonics => crate::core::audio_curve::WaveformType::Harmonics,
        CliWaveformType::Fm => crate::core::audio_curve::WaveformType::FM,
    };

    let path = crate::core::audio_curve::generate_audio_waveform(
        wtype, freq, harmonics, 800.0, 400.0, 300,
    );
    let mut out_doc = crate::core::document::Document {
        width: 800.0,
        height: 400.0,
        ..Default::default()
    };
    out_doc.add_object(crate::core::document::Object::new_path(
        "Audio Waveform",
        path,
    ));
    save_any_document(&out_doc, output)?;
    println!("✅ Audio waveform vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_warp(
    input: &PathBuf,
    preset: CliWarpPreset,
    grid: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🦴 Warping '{:?}' with Live2D FFD ({:?}, grid: {}x{})...",
        input, preset, grid, grid
    );
    let doc = load_any_document(input)?;
    let wpreset = match preset {
        CliWarpPreset::Bulge => crate::core::mesh_warp::WarpPreset::Bulge,
        CliWarpPreset::Pinch => crate::core::mesh_warp::WarpPreset::Pinch,
        CliWarpPreset::Twist => crate::core::mesh_warp::WarpPreset::TwistS,
        CliWarpPreset::Wave => crate::core::mesh_warp::WarpPreset::WaveWarp,
    };

    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let warped = crate::core::mesh_warp::apply_lattice_warp(obj, grid, grid, wpreset, 1.0);
        out_doc.add_object(warped);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ FFD Mesh warped vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_gradient_mesh(
    preset: CliGradientMeshPreset,
    rows: usize,
    cols: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🌈 Generating Gradient Mesh ({:?}, {}x{})...",
        preset, rows, cols
    );
    let gpreset = match preset {
        CliGradientMeshPreset::Sunset => crate::core::gradient_mesh::GradientMeshPreset::Sunset,
        CliGradientMeshPreset::Cyberpunk => {
            crate::core::gradient_mesh::GradientMeshPreset::Cyberpunk
        }
        CliGradientMeshPreset::Aurora => crate::core::gradient_mesh::GradientMeshPreset::Aurora,
        CliGradientMeshPreset::Gold => crate::core::gradient_mesh::GradientMeshPreset::Gold,
    };

    let patches =
        crate::core::gradient_mesh::generate_gradient_mesh(gpreset, 800.0, 600.0, rows, cols);
    let mut out_doc = crate::core::document::Document {
        width: 800.0,
        height: 600.0,
        ..Default::default()
    };
    for patch in patches {
        out_doc.add_object(patch);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Gradient Mesh saved to {:?}", output);
    Ok(false)
}

pub fn handle_axonometric(
    input: &PathBuf,
    mode: CliAxonometricMode,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "📐 Transforming '{:?}' with Axonometric Projection ({:?})...",
        input, mode
    );
    let doc = load_any_document(input)?;
    let amode = match mode {
        CliAxonometricMode::Isometric => crate::core::axonometric::AxonometricMode::Isometric,
        CliAxonometricMode::Dimetric => crate::core::axonometric::AxonometricMode::Dimetric,
        CliAxonometricMode::Trimetric => crate::core::axonometric::AxonometricMode::Trimetric,
        CliAxonometricMode::Cabinet => crate::core::axonometric::AxonometricMode::Cabinet,
        CliAxonometricMode::Cavalier => crate::core::axonometric::AxonometricMode::Cavalier,
    };

    let mut out_doc = crate::core::document::Document {
        width: doc.width * 1.5,
        height: doc.height * 1.5,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let proj = crate::core::axonometric::apply_axonometric_projection(obj, amode);
        out_doc.add_object(proj);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Axonometric vector saved to {:?}", output);
    Ok(false)
}

pub fn handle_evolve(
    polygons: usize,
    generations: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "🧬 Synthesizing Evolutionary Art ({} polygons, {} gens)...",
        polygons, generations
    );
    let polys =
        crate::core::evolutionary::evolve_vector_composition(800.0, 800.0, polygons, generations);
    let mut out_doc = crate::core::document::Document {
        width: 800.0,
        height: 800.0,
        ..Default::default()
    };
    for poly in polys {
        out_doc.add_object(poly);
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Evolutionary art saved to {:?}", output);
    Ok(false)
}

pub fn handle_neon(
    input: &PathBuf,
    radius: f64,
    layers: usize,
    output: &PathBuf,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "✨ Generating Vector Neon Bloom on '{:?}' (radius: {}, layers: {})...",
        input, radius, layers
    );
    let doc = load_any_document(input)?;
    let mut out_doc = crate::core::document::Document {
        width: doc.width,
        height: doc.height,
        ..Default::default()
    };
    for (_, obj) in doc.all_objects() {
        let path = obj.to_path_data();
        let neon_layers =
            crate::core::neon_glow::generate_neon_glow(&path, [0.0, 1.0, 0.9, 1.0], radius, layers);
        for layer in neon_layers {
            out_doc.add_object(layer);
        }
    }
    save_any_document(&out_doc, output)?;
    println!("✅ Vector neon artwork saved to {:?}", output);
    Ok(false)
}

pub fn handle_logo(
    output: &PathBuf,
    variant: &str,
    size: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!(
        "✨ Generating Amata Vector SVG Logo (variant: '{}', size: {}x{})...",
        variant, size, size
    );

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let svg_content = match variant.to_lowercase().as_str() {
        "prism" | "layers" => generate_logo_prism(size),
        "minimal" => generate_logo_minimal(size),
        _ => generate_logo_vector(size),
    };

    std::fs::write(output, svg_content)?;
    println!("🎨 Logo successfully forged!");
    println!("   Output: {:?}", output);
    println!("   Format: Scalable Vector Graphics (SVG)");
    println!("   Variant: {}", variant);
    println!("💡 Tip: Open this in Amata or any browser to inspect pure vector paths.");
    Ok(false)
}

fn generate_logo_vector(size: f64) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="{size}" height="{size}">
  <defs>
    <!-- Left Spine Gradient: Vibrant Electric Cyan to Royal Blue -->
    <linearGradient id="leftSpineGrad" x1="0%" y1="100%" x2="80%" y2="0%">
      <stop offset="0%" stop-color="#00D2D8" />
      <stop offset="35%" stop-color="#14B8E4" />
      <stop offset="70%" stop-color="#0088F0" />
      <stop offset="100%" stop-color="#0066E0" />
    </linearGradient>

    <!-- Crossbar Arch Gradient: Bright Sky Blue to Rich Indigo-Purple -->
    <linearGradient id="archGrad" x1="5%" y1="90%" x2="95%" y2="20%">
      <stop offset="0%" stop-color="#00AEEF" />
      <stop offset="30%" stop-color="#0072CE" />
      <stop offset="65%" stop-color="#4F46E5" />
      <stop offset="100%" stop-color="#7C3AED" />
    </linearGradient>

    <!-- Descending Right Ribbon Gradient: Indigo -> Fuchsia -> Coral -> Tangerine Orange -->
    <linearGradient id="ribbonGrad" x1="15%" y1="0%" x2="85%" y2="100%">
      <stop offset="0%" stop-color="#4F46E5" />
      <stop offset="24%" stop-color="#7C3AED" />
      <stop offset="48%" stop-color="#C026D3" />
      <stop offset="70%" stop-color="#E11D48" />
      <stop offset="88%" stop-color="#F97316" />
      <stop offset="100%" stop-color="#FB923C" />
    </linearGradient>

    <!-- Modern Clean Flat Anchor Nodes (Vector Anchor Points) -->
    <linearGradient id="nodeTop" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#2563EB" />
      <stop offset="100%" stop-color="#1D4ED8" />
    </linearGradient>

    <linearGradient id="nodeBL" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#06B6D4" />
      <stop offset="100%" stop-color="#0891B2" />
    </linearGradient>

    <linearGradient id="nodeBR" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FB923C" />
      <stop offset="100%" stop-color="#EA580C" />
    </linearGradient>
  </defs>

  <g id="amata-emblem">
    <!-- 1. Left Outer Curved Spine -->
    <path d="M 99 364 C 104 290, 168 180, 264 113" 
          fill="none" 
          stroke="url(#leftSpineGrad)" 
          stroke-width="14" 
          stroke-linecap="round" />

    <!-- 2. Central Vector Crossbar Arch (Clean crisp path without muddy dark layers) -->
    <path d="M 114 342 
             C 136 298, 185 252, 244 244 
             C 280 239, 318 252, 348 274 
             C 328 295, 298 294, 268 282 
             C 218 264, 165 295, 114 342 
             Z" 
          fill="url(#archGrad)" />

    <!-- 3. Broad Descending Right Ribbon (Slimmed 12% for elegant balanced weight) -->
    <path d="M 261 115 
             C 265 158, 275 212, 287 258 
             C 300 302, 330 345, 374 372 
             C 390 380, 406 378, 419 373 
             C 405 352, 376 325, 351 287 
             C 326 244, 301 188, 280 115 
             Z" 
          fill="url(#ribbonGrad)" 
          opacity="0.96" />

    <!-- 4. Three Modern Vector Anchor Nodes (Flat, Crisp & Clean) -->
    <!-- Top Apex Node -->
    <circle cx="264" cy="113" r="25" fill="url(#nodeTop)" />

    <!-- Bottom Left Node -->
    <circle cx="99" cy="364" r="24" fill="url(#nodeBL)" />

    <!-- Bottom Right Node -->
    <circle cx="419" cy="373" r="23" fill="url(#nodeBR)" />
  </g>
</svg>
"##
    )
}

fn generate_logo_minimal(size: f64) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="{size}" height="{size}">
  <defs>
    <linearGradient id="minBg" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#24242A" />
      <stop offset="100%" stop-color="#18181D" />
    </linearGradient>
    <linearGradient id="minA" x1="0%" y1="100%" x2="0%" y2="0%">
      <stop offset="0%" stop-color="#FF7A33" />
      <stop offset="100%" stop-color="#FFA834" />
    </linearGradient>
  </defs>
  <rect x="24" y="24" width="464" height="464" rx="96" fill="url(#minBg)" stroke="#33333E" stroke-width="2.5" />
  
  <!-- Minimal Geometric A -->
  <path d="M 160 376 L 256 136 L 352 376" fill="none" stroke="url(#minA)" stroke-width="28" stroke-linecap="round" stroke-linejoin="round" />
  <!-- Center Accent Node -->
  <circle cx="256" cy="288" r="14" fill="#4E9FFF" stroke="#FFFFFF" stroke-width="3" />
  <text x="256" y="436" text-anchor="middle" font-family="system-ui, sans-serif" font-size="24" font-weight="700" letter-spacing="10" fill="#FFFFFF">AMATA</text>
</svg>
"##
    )
}

fn generate_logo_prism(size: f64) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="{size}" height="{size}">
  <defs>
    <linearGradient id="p1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FF6B4A" />
      <stop offset="100%" stop-color="#8B5CF6" />
    </linearGradient>
    <linearGradient id="p2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#38BDF8" />
      <stop offset="100%" stop-color="#2563EB" />
    </linearGradient>
    <linearGradient id="p3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#06B6D4" />
      <stop offset="100%" stop-color="#3B82F6" />
    </linearGradient>
    <linearGradient id="p4" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#F59E0B" />
      <stop offset="100%" stop-color="#F43F5E" />
    </linearGradient>
  </defs>
  <rect x="24" y="24" width="464" height="464" rx="96" fill="#1C1C22" stroke="#2E2E38" stroke-width="2.5" />
  
  <!-- Facets (数多 - Amata Facets) -->
  <polygon points="256,124 176,288 256,260" fill="url(#p1)" opacity="0.9" />
  <polygon points="256,124 336,288 256,260" fill="url(#p2)" opacity="0.9" />
  <polygon points="176,288 140,368 224,368 256,260" fill="url(#p4)" opacity="0.9" />
  <polygon points="336,288 372,368 288,368 256,260" fill="url(#p3)" opacity="0.9" />
  
  <text x="256" y="432" text-anchor="middle" font-family="system-ui, sans-serif" font-size="24" font-weight="700" letter-spacing="10" fill="#FFFFFF">AMATA</text>
  <text x="256" y="454" text-anchor="middle" font-family="system-ui, sans-serif" font-size="10" font-weight="600" letter-spacing="3.5" fill="#8E8E9A">FACETS EDITION</text>
</svg>
"##
    )
}
