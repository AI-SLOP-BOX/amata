use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "irasu")]
#[command(author = "IRASU Illustrator Team")]
#[command(version = "0.1.0")]
#[command(about = "IRASU Illustrator — Pro Vector Studio & VFX Pipeline Bridge", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the interactive Graphical User Interface (Default)
    Gui,

    /// Convert between SVG and IRASU Project formats
    Convert {
        /// Input file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output file (.svg or .json)
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export vector artwork to AEVFX Studio Composition (.json / .aevfx)
    ExportVfx {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output AEVFX composition file (.json / .aevfx)
        #[arg(short, long)]
        output: PathBuf,

        /// Composition frame rate (default: 60.0)
        #[arg(long, default_value_t = 60.0)]
        fps: f64,

        /// Composition duration in seconds (default: 5.0)
        #[arg(long, default_value_t = 5.0)]
        duration: f64,
    },

    /// Export vector artwork into a 3D Wavefront OBJ mesh (for VFX / Blender / Cinema 4D)
    Export3d {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output 3D mesh file (.obj)
        #[arg(short, long)]
        output: PathBuf,

        /// 3D Extrusion depth (default: 20.0)
        #[arg(short, long, default_value_t = 20.0)]
        depth: f64,

        /// Bevel radius (default: 2.0)
        #[arg(short, long, default_value_t = 2.0)]
        bevel: f64,
    },

    /// Morph / interpolate between two vector shapes at factor t (0.0 to 1.0)
    Morph {
        /// First vector shape file (t = 0.0)
        #[arg(short = '1', long)]
        input1: PathBuf,

        /// Second vector shape file (t = 1.0)
        #[arg(short = '2', long)]
        input2: PathBuf,

        /// Interpolation factor (0.0 to 1.0)
        #[arg(short, long, default_value_t = 0.5)]
        t: f64,

        /// Output morphed SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Offset path outward (positive delta) or inward (negative delta)
    Offset {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Offset distance in pixels (e.g. 10.0 or -5.0)
        #[arg(short, long, default_value_t = 10.0)]
        delta: f64,

        /// Output offset SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Outline stroked paths into filled ribbon polygons
    OutlineStroke {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Stroke width to expand (default: 4.0)
        #[arg(short, long, default_value_t = 4.0)]
        width: f64,

        /// Output outlined SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract vector curves as 3D Camera / Particle Motion Path Keyframes
    MotionPath {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output keyframe trajectory file (.json)
        #[arg(short, long)]
        output: PathBuf,

        /// Number of keyframe trajectory samples (default: 60)
        #[arg(short, long, default_value_t = 60)]
        samples: usize,

        /// Trajectory duration in seconds (default: 5.0)
        #[arg(long, default_value_t = 5.0)]
        duration: f64,

        /// Target frame rate (default: 60.0)
        #[arg(long, default_value_t = 60.0)]
        fps: f64,
    },

    /// Vectorize / auto-trace a bitmap image into clean vector paths
    Trace {
        /// Input image file (PNG/JPG/BMP)
        #[arg(short, long)]
        input: PathBuf,

        /// Grayscale binarization threshold (0..=255, default: 128)
        #[arg(short, long, default_value_t = 128)]
        threshold: u8,

        /// Output SVG vector file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export keyframed animation as AEVFX Studio Comp with animated transforms
    Animate {
        /// Input vector project file (.json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output animated AEVFX Comp file (.aevfx / .json)
        #[arg(short, long)]
        output: PathBuf,

        /// Frame rate (default: 60.0)
        #[arg(long, default_value_t = 60.0)]
        fps: f64,

        /// Duration in seconds (default: 3.0)
        #[arg(long, default_value_t = 3.0)]
        duration: f64,
    },

    /// Generate a mathematical or parametric vector curve (Spiral, Lissajous, Spirograph, Rose)
    Formula {
        /// Curve type (spiral, lissajous, spirograph, rose)
        #[arg(value_enum, short = 't', long)]
        curve_type: CliCurveType,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate VFX particle bursts along a vector curve for AEVFX Studio
    VfxTrail {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output particle payload file (.json)
        #[arg(short, long)]
        output: PathBuf,

        /// Particle count (default: 200)
        #[arg(short, long, default_value_t = 200)]
        count: usize,
    },

    /// Convert vector artwork into vector halftone dots (Circular, Hexagonal, Scanline)
    Halftone {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,

        /// Dot spacing in pixels (default: 10.0)
        #[arg(short, long, default_value_t = 10.0)]
        spacing: f64,

        /// Maximum dot radius (default: 4.5)
        #[arg(short, long, default_value_t = 4.5)]
        radius: f64,

        /// Pattern type (circular, hex, scanline)
        #[arg(value_enum, short, long, default_value_t = CliHalftonePattern::Circular)]
        pattern: CliHalftonePattern,
    },

    /// Simplify and smooth vector paths using Visvalingam-Whyatt algorithm
    Simplify {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,

        /// Minimum effective area tolerance (default: 4.0)
        #[arg(short, long, default_value_t = 4.0)]
        tolerance: f64,
    },

    /// Project 2D vector artwork into 2.5D Isometric space (Top, Left, Right)
    Isometric {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,

        /// Isometric plane (top, left, right)
        #[arg(value_enum, short, long, default_value_t = CliIsoPlane::Top)]
        plane: CliIsoPlane,
    },

    /// Generate procedural Voronoi diagram mosaic cells
    Voronoi {
        /// Number of seed cells (default: 40)
        #[arg(short, long, default_value_t = 40)]
        cells: usize,

        /// Cell margin padding (default: 2.5)
        #[arg(short, long, default_value_t = 2.5)]
        padding: f64,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate organic L-System fractal curves (Tree, Dragon, Snowflake, Hilbert)
    Lsystem {
        /// Fractal preset (tree, dragon, snowflake, hilbert)
        #[arg(value_enum, short, long, default_value_t = CliLSystemPreset::Tree)]
        preset: CliLSystemPreset,

        /// Iterations (default: 4)
        #[arg(short, long, default_value_t = 4)]
        iterations: usize,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate pure scalable vector QR Code
    Qr {
        /// Text or URL to encode
        #[arg(short, long)]
        text: String,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Deform vector paths using procedural wave, noise, or glitch
    Deform {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,

        /// Deformation type (wave, noise, glitch)
        #[arg(value_enum, short, long, default_value_t = CliDeformType::Noise)]
        deform_type: CliDeformType,

        /// Deformation amplitude in pixels (default: 12.0)
        #[arg(short, long, default_value_t = 12.0)]
        amplitude: f64,

        /// Frequency scale (default: 0.05)
        #[arg(short, long, default_value_t = 0.05)]
        frequency: f64,
    },

    /// Generate vector streamlines from a 2D vector flow field
    Flowfield {
        /// Flow field preset (vortex, magnetic, cyber)
        #[arg(value_enum, short, long, default_value_t = CliFlowFieldPreset::Vortex)]
        preset: CliFlowFieldPreset,

        /// Number of streamlines (default: 60)
        #[arg(short, long, default_value_t = 60)]
        lines: usize,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Scatter a motif object along a trajectory path
    BrushStroke {
        /// Trajectory vector file (.svg or .json)
        #[arg(short, long)]
        path: PathBuf,

        /// Motif vector file (.svg or .json)
        #[arg(short, long)]
        motif: PathBuf,

        /// Spacing between instances (default: 25.0)
        #[arg(short, long, default_value_t = 25.0)]
        spacing: f64,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Apply Photoshop blend mode to composite two vector artworks
    Blend {
        /// Base vector file
        #[arg(short = '1', long)]
        base: PathBuf,

        /// Blend vector file
        #[arg(short = '2', long)]
        blend: PathBuf,

        /// Blend mode (normal, multiply, screen, overlay, color-dodge, difference, etc.)
        #[arg(value_enum, short, long, default_value_t = CliBlendMode::Multiply)]
        mode: CliBlendMode,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate audio waveform or synthesizer curves (LogicPro DSP)
    AudioWave {
        /// Waveform type (sine, sawtooth, square, triangle, harmonics, fm)
        #[arg(value_enum, short, long, default_value_t = CliWaveformType::Sine)]
        wave_type: CliWaveformType,

        /// Frequency / cycles (default: 4.0)
        #[arg(short, long, default_value_t = 4.0)]
        freq: f64,

        /// Harmonic count (default: 5)
        #[arg(short = 'H', long, default_value_t = 5)]
        harmonics: usize,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Apply Live2D-style Free-Form Deformation (FFD) Lattice Mesh Warp
    Warp {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Warp preset (bulge, pinch, twist, wave)
        #[arg(value_enum, short, long, default_value_t = CliWarpPreset::Bulge)]
        preset: CliWarpPreset,

        /// Grid divisions (default: 4)
        #[arg(short, long, default_value_t = 4)]
        grid: usize,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export vector artwork to pure standards-compliant Vector PDF
    ExportPdf {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Output PDF file (.pdf)
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Execute headless Pathfinder (Boolean Operations) on two vector files
    Boolean {
        /// First vector file (Subject)
        #[arg(short = '1', long)]
        input1: PathBuf,

        /// Second vector file (Clip)
        #[arg(short = '2', long)]
        input2: PathBuf,

        /// Boolean operation to perform
        #[arg(value_enum, short, long)]
        op: CliBooleanOp,

        /// Output SVG file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Inspect document hierarchy, layers, and bounding box info
    Info {
        /// Input vector file (.svg or .json)
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliBooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliCurveType {
    Spiral,
    Lissajous,
    Spirograph,
    Rose,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliHalftonePattern {
    Circular,
    Hex,
    Scanline,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliIsoPlane {
    Top,
    Left,
    Right,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliLSystemPreset {
    Tree,
    Dragon,
    Snowflake,
    Hilbert,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliDeformType {
    Wave,
    Noise,
    Glitch,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliFlowFieldPreset {
    Vortex,
    Magnetic,
    Cyber,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliWaveformType {
    Sine,
    Sawtooth,
    Square,
    Triangle,
    Harmonics,
    Fm,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliWarpPreset {
    Bulge,
    Pinch,
    Twist,
    Wave,
}

pub fn run_cli(cli: Cli) -> Result<bool, Box<dyn std::error::Error>> {
    match cli.command {
        None | Some(Commands::Gui) => {
            // Signal caller to launch GUI
            Ok(true)
        }
        Some(Commands::Convert { input, output }) => {
            println!("🔄 Converting '{:?}' to '{:?}'...", input, output);
            let doc = load_any_document(&input)?;
            save_any_document(&doc, &output)?;
            println!("✅ Conversion complete: {:?}", output);
            Ok(false)
        }
        Some(Commands::ExportVfx { input, output, fps, duration }) => {
            println!("🎬 Exporting to AEVFX Studio Comp: '{:?}' ({} fps, {}s)...", output, fps, duration);
            let doc = load_any_document(&input)?;
            let vfx_comp = crate::io::vfx::doc_to_aevfx_comp(&doc, fps, duration);
            let json = serde_json::to_string_pretty(&vfx_comp)?;
            std::fs::write(&output, json)?;
            println!("✅ Exported {} VFX layers to {:?}", vfx_comp.layers.len(), output);
            Ok(false)
        }
        Some(Commands::Export3d { input, output, depth, bevel }) => {
            println!("🧱 Exporting to 3D OBJ Mesh (depth: {}, bevel: {}): '{:?}'...", depth, bevel, output);
            let doc = load_any_document(&input)?;
            let obj_str = crate::io::vfx::export_doc_to_obj(&doc, depth, bevel);
            std::fs::write(&output, obj_str)?;
            println!("✅ 3D OBJ Mesh exported to {:?}", output);
            Ok(false)
        }
        Some(Commands::Morph { input1, input2, t, output }) => {
            println!("🧬 Morphing '{:?}' and '{:?}' at t = {} -> '{:?}'...", input1, input2, t, output);
            let doc1 = load_any_document(&input1)?;
            let doc2 = load_any_document(&input2)?;

            let obj1 = doc1.all_objects().next().map(|(_, o)| o).ok_or("Input 1 contains no objects")?;
            let obj2 = doc2.all_objects().next().map(|(_, o)| o).ok_or("Input 2 contains no objects")?;

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
            save_any_document(&out_doc, &output)?;
            println!("✅ Morphed shape saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Offset { input, delta, output }) => {
            println!("📐 Offsetting path in '{:?}' by {}px -> '{:?}'...", input, delta, output);
            let doc = load_any_document(&input)?;
            let mut out_doc = crate::core::document::Document {
                width: doc.width,
                height: doc.height,
                ..Default::default()
            };

            for (_, obj) in doc.all_objects() {
                let path = obj.to_path_data();
                let off_path = crate::core::offset::offset_path(&path, delta);
                let mut new_obj = crate::core::document::Object::new_path(&format!("{} (Offset)", obj.name), off_path);
                new_obj.transform = obj.transform.clone();
                out_doc.add_object(new_obj);
            }

            save_any_document(&out_doc, &output)?;
            println!("✅ Offset path saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::OutlineStroke { input, width, output }) => {
            println!("🖋 Outlining strokes in '{:?}' (width: {}px) -> '{:?}'...", input, width, output);
            let doc = load_any_document(&input)?;
            let mut out_doc = crate::core::document::Document {
                width: doc.width,
                height: doc.height,
                ..Default::default()
            };

            for (_, obj) in doc.all_objects() {
                let path = obj.to_path_data();
                let outlined = crate::core::offset::outline_stroke(&path, width);
                let mut new_obj = crate::core::document::Object::new_path(&format!("{} (Outlined)", obj.name), outlined);
                new_obj.transform = obj.transform.clone();
                out_doc.add_object(new_obj);
            }

            save_any_document(&out_doc, &output)?;
            println!("✅ Outlined stroke saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Trace { input, threshold, output }) => {
            println!("🖼️ Auto-tracing image '{:?}' (threshold: {}) -> '{:?}'...", input, threshold, output);
            let img = image::open(&input)?;
            let gray = img.to_luma8();
            let w = gray.width() as usize;
            let h = gray.height() as usize;
            let path_data = crate::core::trace::trace_bitmap_to_path(w, h, gray.as_raw(), threshold);
            let mut doc = crate::core::document::Document {
                width: w as f64,
                height: h as f64,
                ..Default::default()
            };
            doc.add_object(crate::core::document::Object::new_path("Traced Image", path_data));
            save_any_document(&doc, &output)?;
            println!("✅ Auto-traced vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Animate { input, output, fps, duration }) => {
            println!("🎬 Generating animated AEVFX Studio Comp from '{:?}' ({} fps, {}s)...", input, fps, duration);
            let doc = load_any_document(&input)?;
            let vfx_comp = crate::io::vfx::doc_to_aevfx_comp(&doc, fps, duration);
            let json = serde_json::to_string_pretty(&vfx_comp)?;
            std::fs::write(&output, json)?;
            println!("✅ Animated composition saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Formula { curve_type, output }) => {
            println!("🌀 Generating mathematical curve ({:?}) -> '{:?}'...", curve_type, output);
            let cx = 400.0;
            let cy = 300.0;
            let path = match curve_type {
                CliCurveType::Spiral => crate::core::formula::FormulaCurves::spiral(cx, cy, 4.0, 10.0, 4.0, 200),
                CliCurveType::Lissajous => crate::core::formula::FormulaCurves::lissajous(cx, cy, 3.0, 2.0, 0.5, 300.0, 200.0, 240),
                CliCurveType::Spirograph => crate::core::formula::FormulaCurves::spirograph(cx, cy, 140.0, 60.0, 80.0, 8, 48),
                CliCurveType::Rose => crate::core::formula::FormulaCurves::rose_curve(cx, cy, 4.0, 120.0, 200),
            };
            let mut doc = crate::core::document::Document {
                width: 800.0,
                height: 600.0,
                ..Default::default()
            };
            doc.add_object(crate::core::document::Object::new_path("Formula Curve", path));
            save_any_document(&doc, &output)?;
            println!("✅ Formula curve saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::VfxTrail { input, output, count }) => {
            println!("⚡ Generating {} VFX particle trails from '{:?}'...", count, input);
            let doc = load_any_document(&input)?;
            let mut all_particles = Vec::new();
            for (_, obj) in doc.all_objects() {
                let mut path = obj.to_path_data();
                path.transform(&obj.transform.matrix());
                let p = crate::core::vfx_particles::generate_particle_trail(&path, count, 60.0, 12.0);
                all_particles.extend(p);
            }
            let json = serde_json::to_string_pretty(&all_particles)?;
            std::fs::write(&output, json)?;
            println!("✅ Exported {} VFX particles to {:?}", all_particles.len(), output);
            Ok(false)
        }
        Some(Commands::Halftone { input, output, spacing, radius, pattern }) => {
            println!("🏁 Generating halftone dots from '{:?}' (spacing: {}, radius: {})...", input, spacing, radius);
            let doc = load_any_document(&input)?;
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
                let ht_path = crate::core::halftone::generate_halftone_from_path(&path, spacing, radius, ht_pat);
                out_doc.add_object(crate::core::document::Object::new_path(&format!("{} (Halftone)", obj.name), ht_path));
            }
            save_any_document(&out_doc, &output)?;
            println!("✅ Halftone vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Simplify { input, output, tolerance }) => {
            println!("🪄 Simplifying paths in '{:?}' (tolerance: {})...", input, tolerance);
            let doc = load_any_document(&input)?;
            let mut out_doc = crate::core::document::Document {
                width: doc.width,
                height: doc.height,
                ..Default::default()
            };
            for (_, obj) in doc.all_objects() {
                let path = obj.to_path_data();
                let simplified = crate::core::simplify::simplify_path_visvalingam(&path, tolerance);
                let mut new_obj = crate::core::document::Object::new_path(&format!("{} (Simplified)", obj.name), simplified);
                new_obj.transform = obj.transform.clone();
                out_doc.add_object(new_obj);
            }
            save_any_document(&out_doc, &output)?;
            println!("✅ Simplified vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Isometric { input, output, plane }) => {
            println!("📐 Projecting '{:?}' to Isometric {:?}...", input, plane);
            let doc = load_any_document(&input)?;
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
            save_any_document(&out_doc, &output)?;
            println!("✅ Isometric vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Voronoi { cells, padding, output }) => {
            println!("🔷 Generating Voronoi mosaic ({} cells, {}px padding)...", cells, padding);
            let w = 800.0;
            let h = 600.0;
            let mut seeds = Vec::with_capacity(cells);
            for i in 0..cells {
                let hx = ((i as f64 * 37.123 + 12.34).sin() * 43758.5453).fract().abs();
                let hy = ((i as f64 * 91.567 + 84.12).sin() * 43758.5453).fract().abs();
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
            save_any_document(&out_doc, &output)?;
            println!("✅ Voronoi mosaic saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Lsystem { preset, iterations, output }) => {
            println!("🌿 Generating L-System {:?} ({} iterations)...", preset, iterations);
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
            out_doc.add_object(crate::core::document::Object::new_path("L-System Fractal", path));
            save_any_document(&out_doc, &output)?;
            println!("✅ L-System fractal saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Qr { text, output }) => {
            println!("📱 Generating Vector QR Code for '{}'...", text);
            let path = crate::core::barcode::generate_vector_qr(&text, 200.0, 200.0, 360.0)?;
            let mut out_doc = crate::core::document::Document {
                width: 400.0,
                height: 400.0,
                ..Default::default()
            };
            out_doc.add_object(crate::core::document::Object::new_path("Vector QR Code", path));
            save_any_document(&out_doc, &output)?;
            println!("✅ Vector QR code saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Deform { input, output, deform_type, amplitude, frequency }) => {
            println!("🌊 Deforming '{:?}' ({:?}, amp: {}, freq: {})...", input, deform_type, amplitude, frequency);
            let doc = load_any_document(&input)?;
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
                let mut new_obj = crate::core::document::Object::new_path(&format!("{} (Deformed)", obj.name), def_path);
                new_obj.transform = obj.transform.clone();
                out_doc.add_object(new_obj);
            }
            save_any_document(&out_doc, &output)?;
            println!("✅ Deformed vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Flowfield { preset, lines, output }) => {
            println!("🌌 Generating Flow Field Streamlines ({:?}, {} lines)...", preset, lines);
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
            let streamlines = crate::core::flowfield::generate_flowfield_streamlines(fpreset, 800.0, 600.0, lines, 80, 5.0);
            for line in streamlines {
                out_doc.add_object(line);
            }
            save_any_document(&out_doc, &output)?;
            println!("✅ Flow field streamlines saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::BrushStroke { path, motif, spacing, output }) => {
            println!("🖌️ Scattering motif '{:?}' along path '{:?}' (spacing: {})...", motif, path, spacing);
            let path_doc = load_any_document(&path)?;
            let motif_doc = load_any_document(&motif)?;

            let path_obj = path_doc.all_objects().next().map(|(_, o)| o).ok_or("Path document is empty")?;
            let motif_obj = motif_doc.all_objects().next().map(|(_, o)| o).ok_or("Motif document is empty")?;

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
            save_any_document(&out_doc, &output)?;
            println!("✅ Scattered brush stroke saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Blend { base, blend, mode, output }) => {
            println!("🎨 Blending '{:?}' over '{:?}' ({:?})...", blend, base, mode);
            let mut base_doc = load_any_document(&base)?;
            let blend_doc = load_any_document(&blend)?;

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
                    fill.color = crate::core::blend::blend_colors(fill.color, [1.0, 1.0, 1.0, 1.0], blend_mode);
                }
                base_doc.add_object(blended_obj);
            }

            save_any_document(&base_doc, &output)?;
            println!("✅ Blended vector artwork saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::AudioWave { wave_type, freq, harmonics, output }) => {
            println!("🎵 Generating Audio Waveform ({:?}, freq: {}, harmonics: {})...", wave_type, freq, harmonics);
            let wtype = match wave_type {
                CliWaveformType::Sine => crate::core::audio_curve::WaveformType::Sine,
                CliWaveformType::Sawtooth => crate::core::audio_curve::WaveformType::Sawtooth,
                CliWaveformType::Square => crate::core::audio_curve::WaveformType::Square,
                CliWaveformType::Triangle => crate::core::audio_curve::WaveformType::Triangle,
                CliWaveformType::Harmonics => crate::core::audio_curve::WaveformType::Harmonics,
                CliWaveformType::Fm => crate::core::audio_curve::WaveformType::FM,
            };

            let path = crate::core::audio_curve::generate_audio_waveform(wtype, freq, harmonics, 800.0, 400.0, 300);
            let mut out_doc = crate::core::document::Document {
                width: 800.0,
                height: 400.0,
                ..Default::default()
            };
            out_doc.add_object(crate::core::document::Object::new_path("Audio Waveform", path));
            save_any_document(&out_doc, &output)?;
            println!("✅ Audio waveform vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::Warp { input, preset, grid, output }) => {
            println!("🦴 Warping '{:?}' with Live2D FFD ({:?}, grid: {}x{})...", input, preset, grid, grid);
            let doc = load_any_document(&input)?;
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
            save_any_document(&out_doc, &output)?;
            println!("✅ FFD Mesh warped vector saved to {:?}", output);
            Ok(false)
        }
        Some(Commands::ExportPdf { input, output }) => {
            println!("📄 Exporting '{:?}' to Pure Vector PDF...", input);
            let doc = load_any_document(&input)?;
            let pdf_bytes = crate::io::pdf::export_pdf(&doc);
            std::fs::write(&output, pdf_bytes)?;
            println!("✅ Vector PDF exported successfully to {:?}", output);
            Ok(false)
        }
        Some(Commands::MotionPath { input, output, samples, duration, fps }) => {
            println!("🚀 Generating Motion Path Keyframes ({} samples) from '{:?}'...", samples, input);
            let doc = load_any_document(&input)?;
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
            std::fs::write(&output, json)?;
            println!("✅ Generated {} motion paths into {:?}", all_trajectories.len(), output);
            Ok(false)
        }
        Some(Commands::Boolean { input1, input2, op, output }) => {
            println!("✂ Running Pathfinder ({:?}) on '{:?}' and '{:?}'...", op, input1, input2);
            let doc1 = load_any_document(&input1)?;
            let doc2 = load_any_document(&input2)?;

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
                save_any_document(&out_doc, &output)?;
                println!("✅ Pathfinder result saved to {:?}", output);
            } else {
                println!("⚠️ Pathfinder produced empty geometry.");
            }
            Ok(false)
        }
        Some(Commands::Info { input }) => {
            let doc = load_any_document(&input)?;
            println!("📊 === IRASU Illustrator Document Info ===");
            println!("  Name: {}", doc.name);
            println!("  Canvas Size: {} × {} px", doc.width, doc.height);
            println!("  Layers ({}):", doc.layers.len());
            for (i, layer) in doc.layers.iter().enumerate() {
                println!("    [{}] Layer '{}' (visible: {}, locked: {}, opacity: {:.2}) - {} objects",
                    i, layer.name, layer.visible, layer.locked, layer.opacity, layer.objects.len());
                for (j, obj) in layer.objects.iter().enumerate() {
                    let bb_str = obj.bounding_box()
                        .map(|(min, max)| format!("bounds: ({:.1}, {:.1}) -> ({:.1}, {:.1})", min.x, min.y, max.x, max.y))
                        .unwrap_or_else(|| "no bounds".to_string());
                    println!("      - ({}) '{}' [{:?}] (opacity: {:.2}, {})",
                        j, obj.name, std::mem::discriminant(&obj.object_type), obj.opacity, bb_str);
                }
            }
            Ok(false)
        }
    }
}

fn load_any_document(path: &PathBuf) -> Result<crate::core::document::Document, Box<dyn std::error::Error>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    if ext == "svg" {
        let content = std::fs::read_to_string(path)?;
        Ok(crate::io::svg::parse_svg_document(&content))
    } else {
        crate::io::project::load_project(path).map_err(|e| e.into())
    }
}

fn save_any_document(doc: &crate::core::document::Document, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    if ext == "svg" {
        let svg = crate::io::svg::export_svg(doc);
        std::fs::write(path, svg)?;
        Ok(())
    } else {
        crate::io::project::save_project(doc, path).map_err(|e| e.into())
    }
}
