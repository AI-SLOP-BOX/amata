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
