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
