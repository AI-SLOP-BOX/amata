use clap::Subcommand;
use std::path::PathBuf;

use super::types::*;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the interactive Graphical User Interface (Default)
    Gui,

    /// Open an SVG or Amata document in the GUI editor
    Open {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
    },

    /// Directly render an SVG or Amata document to PNG, JPEG, WebP, or AVIF with high fidelity
    Render {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        scale: Option<f32>,
        #[arg(short = 'W', long)]
        width: Option<u32>,
        #[arg(short = 'H', long)]
        height: Option<u32>,
        #[arg(short, long)]
        background: Option<String>,
    },

    /// Inspect document hierarchy, objects, resources, and warnings
    Inspect {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Validate SVG syntax, references, resources, and export-readiness (exit 0/1 for CI)
    Validate {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        #[arg(long)]
        strict: bool,
    },

    /// Safely optimize an SVG by pruning unused defs, empty groups, and redundant data
    Optimize {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 2)]
        precision: usize,
    },

    /// Convert between SVG, PNG, JPEG, WebP, AVIF, PDF, and Amata Project formats
    Convert {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 1.0)]
        scale: f32,
    },

    /// Export vector artwork to AEVFX Studio Composition (.json / .aevfx)
    ExportVfx {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 60.0)]
        fps: f64,
        #[arg(long, default_value_t = 5.0)]
        duration: f64,
    },

    /// Export vector artwork into a 3D Wavefront OBJ mesh
    Export3d {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 20.0)]
        depth: f64,
        #[arg(short, long, default_value_t = 2.0)]
        bevel: f64,
    },

    /// Morph / interpolate between two vector shapes
    Morph {
        #[arg(short = '1', long)]
        input1: PathBuf,
        #[arg(short = '2', long)]
        input2: PathBuf,
        #[arg(short, long, default_value_t = 0.5)]
        t: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Offset path outward or inward
    Offset {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 10.0)]
        delta: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Outline stroked paths into filled ribbon polygons
    OutlineStroke {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 4.0)]
        width: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract vector curves as 3D Camera / Particle Motion Path Keyframes
    MotionPath {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 60)]
        samples: usize,
        #[arg(long, default_value_t = 5.0)]
        duration: f64,
        #[arg(long, default_value_t = 60.0)]
        fps: f64,
    },

    /// Vectorize / auto-trace a bitmap image
    Trace {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 128)]
        threshold: u8,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export keyframed animation as AEVFX Studio Comp
    Animate {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 60.0)]
        fps: f64,
        #[arg(long, default_value_t = 3.0)]
        duration: f64,
    },

    /// Generate a mathematical or parametric vector curve
    Formula {
        #[arg(value_enum, short = 't', long)]
        curve_type: CliCurveType,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate VFX particle bursts along a vector curve
    VfxTrail {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 200)]
        count: usize,
    },

    /// Convert vector artwork into vector halftone dots
    Halftone {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 10.0)]
        spacing: f64,
        #[arg(short, long, default_value_t = 4.5)]
        radius: f64,
        #[arg(value_enum, short, long, default_value_t = CliHalftonePattern::Circular)]
        pattern: CliHalftonePattern,
    },

    /// Simplify and smooth vector paths using Visvalingam-Whyatt algorithm
    Simplify {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 4.0)]
        tolerance: f64,
    },

    /// Project 2D vector artwork into 2.5D Isometric space
    Isometric {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(value_enum, short, long, default_value_t = CliIsoPlane::Top)]
        plane: CliIsoPlane,
    },

    /// Generate procedural Voronoi diagram mosaic cells
    Voronoi {
        #[arg(short, long, default_value_t = 40)]
        cells: usize,
        #[arg(short, long, default_value_t = 2.5)]
        padding: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate organic L-System fractal curves
    Lsystem {
        #[arg(value_enum, short, long, default_value_t = CliLSystemPreset::Tree)]
        preset: CliLSystemPreset,
        #[arg(short, long, default_value_t = 4)]
        iterations: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate pure scalable vector QR Code
    Qr {
        #[arg(short, long)]
        text: String,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Deform vector paths using procedural wave, noise, or glitch
    Deform {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(value_enum, short, long, default_value_t = CliDeformType::Noise)]
        deform_type: CliDeformType,
        #[arg(short, long, default_value_t = 12.0)]
        amplitude: f64,
        #[arg(short, long, default_value_t = 0.05)]
        frequency: f64,
    },

    /// Generate vector streamlines from a 2D vector flow field
    Flowfield {
        #[arg(value_enum, short, long, default_value_t = CliFlowFieldPreset::Vortex)]
        preset: CliFlowFieldPreset,
        #[arg(short, long, default_value_t = 60)]
        lines: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Scatter a motif object along a trajectory path
    BrushStroke {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        motif: PathBuf,
        #[arg(short, long, default_value_t = 25.0)]
        spacing: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Apply Photoshop blend mode to composite two vector artworks
    Blend {
        #[arg(short = '1', long)]
        base: PathBuf,
        #[arg(short = '2', long)]
        blend: PathBuf,
        #[arg(value_enum, short, long, default_value_t = CliBlendMode::Multiply)]
        mode: CliBlendMode,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate audio waveform or synthesizer curves
    AudioWave {
        #[arg(value_enum, short, long, default_value_t = CliWaveformType::Sine)]
        wave_type: CliWaveformType,
        #[arg(short, long, default_value_t = 4.0)]
        freq: f64,
        #[arg(short = 'H', long, default_value_t = 5)]
        harmonics: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Apply Live2D-style Free-Form Deformation (FFD) Lattice Mesh Warp
    Warp {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(value_enum, short, long, default_value_t = CliWarpPreset::Bulge)]
        preset: CliWarpPreset,
        #[arg(short, long, default_value_t = 4)]
        grid: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Export vector artwork to pure standards-compliant Vector PDF
    ExportPdf {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate multi-point Gradient Mesh color patches
    GradientMesh {
        #[arg(value_enum, short, long, default_value_t = CliGradientMeshPreset::Sunset)]
        preset: CliGradientMeshPreset,
        #[arg(short, long, default_value_t = 3)]
        rows: usize,
        #[arg(short, long, default_value_t = 3)]
        cols: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Transform artwork with axonometric projection
    Axonometric {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(value_enum, short, long, default_value_t = CliAxonometricMode::Dimetric)]
        mode: CliAxonometricMode,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Synthesize evolutionary computational vector art
    Evolve {
        #[arg(short, long, default_value_t = 40)]
        polygons: usize,
        #[arg(short, long, default_value_t = 50)]
        generations: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate multi-tiered glowing vector neon halos and laser blooms
    Neon {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 20.0)]
        radius: f64,
        #[arg(short, long, default_value_t = 6)]
        layers: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate 3D Revolve / Lathe OBJ mesh by spinning a 2D profile
    Revolve {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value_t = 360.0)]
        angle: f64,
        #[arg(short, long, default_value_t = 32)]
        segments: usize,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Mold and distort art artwork into an envelope frame polygon
    Envelope {
        #[arg(short, long)]
        art: PathBuf,
        #[arg(short, long)]
        envelope: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Transform artwork between Cartesian and Polar coordinates
    Polar {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Slice and bisect vector artwork with a cutting line
    Slice {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Convert text elements into editable vector paths
    Outline {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Flow text along an arbitrary vector curve and generate outlines
    TextPath {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        text: String,
        #[arg(long, default_value_t = 24.0)]
        font_size: f64,
        #[arg(long, default_value_t = 0.0)]
        offset: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Apply non-destructive appearance effects
    Effect {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long)]
        shadow: bool,
        #[arg(long, default_value_t = 8.0)]
        shadow_x: f64,
        #[arg(long, default_value_t = 8.0)]
        shadow_y: f64,
        #[arg(long, default_value_t = 10.0)]
        shadow_blur: f64,
        #[arg(long)]
        glow: bool,
        #[arg(long, default_value_t = 15.0)]
        glow_radius: f64,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Decompose overlapping vector objects into atomic disjoint fragments
    ShapeBuild {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Combine multiple vector objects into a Compound Path or release it
    Compound {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        release: bool,
    },

    /// Execute headless Pathfinder (Boolean Operations) on two vector files
    Boolean {
        #[arg(short = '1', long)]
        input1: PathBuf,
        #[arg(short = '2', long)]
        input2: PathBuf,
        #[arg(value_enum, short, long)]
        op: CliBooleanOp,
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Inspect document hierarchy, layers, and bounding box info
    Info {
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Execute a Rhai script to generate/transform vector art
    Script {
        #[arg(short, long)]
        script: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        input: Option<PathBuf>,
    },

    /// List installed plugins or get plugin info
    Plugins {
        #[arg(long)]
        info: Option<String>,
    },

    /// Start a local HTTP API server for external tool integration
    Serve {
        #[arg(short, long, default_value_t = 9260)]
        port: u16,
        #[arg(short, long)]
        input: Option<PathBuf>,
    },

    /// Generate the official Amata vector SVG logo
    Logo {
        #[arg(short, long, default_value = "assets/logo.svg")]
        output: PathBuf,
        #[arg(short, long, default_value = "vector")]
        variant: String,
        #[arg(short, long, default_value_t = 512.0)]
        size: f64,
    },

    /// Semantic SVG diff comparing two files or Git revisions
    Diff {
        /// First file or git revision (e.g. old.svg, HEAD, HEAD~1)
        #[arg(value_name = "TARGET_A")]
        target_a: String,
        /// Second file or target file when comparing git revisions (e.g. new.svg, poster.svg)
        #[arg(value_name = "TARGET_B")]
        target_b: Option<String>,
        /// Filter diff to a specific file when comparing revisions
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Output structured JSON representation
        #[arg(long)]
        json: bool,
    },

    /// Show Git-backed version history timeline for an SVG file
    History {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        /// Maximum number of revisions to display
        #[arg(short, long, default_value_t = 20)]
        max: usize,
        /// Output structured JSON representation
        #[arg(long)]
        json: bool,
    },

    /// Create a milestone checkpoint (Git commit) for an SVG file
    Checkpoint {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        /// Milestone message
        #[arg(short, long, default_value = "Milestone checkpoint")]
        message: String,
    },

    /// Restore an SVG file to a previous revision
    Restore {
        #[arg(value_name = "INPUT")]
        input: PathBuf,
        /// Revision hash or ref (e.g. HEAD~1, commit hash)
        #[arg(value_name = "REVISION")]
        revision: String,
    },
}
