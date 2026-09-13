pub mod commands;
pub mod handlers;
pub mod types;

use clap::Parser;
use std::path::PathBuf;
use std::sync::Mutex;

pub use commands::Commands;
#[allow(unused_imports)]
pub use types::*;

use handlers::basic::*;
use handlers::diff_history::*;
use handlers::generative::*;
use handlers::geometry::*;
use handlers::svg_pipeline::*;

static INITIAL_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_initial_file(path: PathBuf) {
    if let Ok(mut lock) = INITIAL_FILE.lock() {
        *lock = Some(path);
    }
}

pub fn take_initial_file() -> Option<PathBuf> {
    INITIAL_FILE.lock().ok().and_then(|mut lock| lock.take())
}

#[derive(Parser, Debug)]
#[command(name = "amata")]
#[command(author = "Amata Team")]
#[command(version = "0.1.0")]
#[command(about = "Amata — Pro Vector Studio & Generative Graphics Pipeline", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

pub fn run_cli(cli: Cli) -> Result<bool, Box<dyn std::error::Error>> {
    match cli.command {
        None | Some(Commands::Gui) => Ok(true),
        Some(Commands::Open { input }) => {
            set_initial_file(input);
            Ok(true)
        }
        Some(Commands::Render {
            input,
            output,
            scale,
            width,
            height,
            background,
        }) => handle_render(&input, &output, scale, width, height, background.as_deref()),
        Some(Commands::Inspect { input, json }) => handle_inspect(&input, json),
        Some(Commands::Validate { input, strict }) => handle_validate(&input, strict),
        Some(Commands::Optimize {
            input,
            output,
            precision,
        }) => handle_optimize(&input, &output, precision),
        Some(Commands::Convert {
            input,
            output,
            scale,
        }) => handle_convert(&input, &output, scale),
        Some(Commands::ExportVfx {
            input,
            output,
            fps,
            duration,
        }) => handle_export_vfx(&input, &output, fps, duration),
        Some(Commands::Export3d {
            input,
            output,
            depth,
            bevel,
        }) => handle_export_3d(&input, &output, depth, bevel),
        Some(Commands::ExportPdf { input, output }) => handle_export_pdf(&input, &output),
        Some(Commands::Morph {
            input1,
            input2,
            t,
            output,
        }) => handle_morph(&input1, &input2, t, &output),
        Some(Commands::Offset {
            input,
            delta,
            output,
        }) => handle_offset(&input, delta, &output),
        Some(Commands::OutlineStroke {
            input,
            width,
            output,
        }) => handle_outline_stroke(&input, width, &output),
        Some(Commands::MotionPath {
            input,
            output,
            samples,
            duration,
            fps,
        }) => handle_motion_path(&input, &output, samples, duration, fps),
        Some(Commands::Trace {
            input,
            threshold,
            output,
        }) => handle_trace(&input, threshold, &output),
        Some(Commands::Animate {
            input,
            output,
            fps,
            duration,
        }) => handle_animate(&input, &output, fps, duration),
        Some(Commands::Formula { curve_type, output }) => handle_formula(curve_type, &output),
        Some(Commands::VfxTrail {
            input,
            output,
            count,
        }) => handle_vfx_trail(&input, count, &output),
        Some(Commands::Halftone {
            input,
            output,
            spacing,
            radius,
            pattern,
        }) => handle_halftone(&input, spacing, radius, pattern, &output),
        Some(Commands::Simplify {
            input,
            output,
            tolerance,
        }) => handle_simplify(&input, tolerance, &output),
        Some(Commands::Isometric {
            input,
            output,
            plane,
        }) => handle_isometric(&input, plane, &output),
        Some(Commands::Voronoi {
            cells,
            padding,
            output,
        }) => handle_voronoi(cells, padding, &output),
        Some(Commands::Lsystem {
            preset,
            iterations,
            output,
        }) => handle_lsystem(preset, iterations, &output),
        Some(Commands::Qr { text, output }) => handle_qr(&text, &output),
        Some(Commands::Deform {
            input,
            output,
            deform_type,
            amplitude,
            frequency,
        }) => handle_deform(&input, &output, deform_type, amplitude, frequency),
        Some(Commands::Flowfield {
            preset,
            lines,
            output,
        }) => handle_flowfield(preset, lines, &output),
        Some(Commands::BrushStroke {
            path,
            motif,
            spacing,
            output,
        }) => handle_brush_stroke(&path, &motif, spacing, &output),
        Some(Commands::Blend {
            base,
            blend,
            mode,
            output,
        }) => handle_blend(&base, &blend, mode, &output),
        Some(Commands::AudioWave {
            wave_type,
            freq,
            harmonics,
            output,
        }) => handle_audio_wave(wave_type, freq, harmonics, &output),
        Some(Commands::Warp {
            input,
            preset,
            grid,
            output,
        }) => handle_warp(&input, preset, grid, &output),
        Some(Commands::GradientMesh {
            preset,
            rows,
            cols,
            output,
        }) => handle_gradient_mesh(preset, rows, cols, &output),
        Some(Commands::Axonometric {
            input,
            mode,
            output,
        }) => handle_axonometric(&input, mode, &output),
        Some(Commands::Evolve {
            polygons,
            generations,
            output,
        }) => handle_evolve(polygons, generations, &output),
        Some(Commands::Neon {
            input,
            radius,
            layers,
            output,
        }) => handle_neon(&input, radius, layers, &output),
        Some(Commands::Revolve {
            input,
            angle,
            segments,
            output,
        }) => handle_revolve(&input, angle, segments, &output),
        Some(Commands::Envelope {
            art,
            envelope,
            output,
        }) => handle_envelope(&art, &envelope, &output),
        Some(Commands::Polar { input, output }) => handle_polar(&input, &output),
        Some(Commands::Slice { input, output }) => handle_slice(&input, &output),
        Some(Commands::Outline { input, output }) => handle_outline(&input, &output),
        Some(Commands::TextPath {
            path,
            text,
            font_size,
            offset,
            output,
        }) => handle_text_path(&path, &text, font_size, offset, &output),
        Some(Commands::Effect {
            input,
            shadow,
            shadow_x,
            shadow_y,
            shadow_blur,
            glow,
            glow_radius,
            output,
        }) => handle_effect(
            &input,
            shadow,
            shadow_x,
            shadow_y,
            shadow_blur,
            glow,
            glow_radius,
            &output,
        ),
        Some(Commands::ShapeBuild { input, output }) => handle_shape_build(&input, &output),
        Some(Commands::Compound {
            input,
            output,
            release,
        }) => handle_compound(&input, &output, release),
        Some(Commands::Boolean {
            input1,
            input2,
            op,
            output,
        }) => handle_boolean(&input1, &input2, op, &output),
        Some(Commands::Info { input }) => handle_info(&input),
        Some(Commands::Script {
            script,
            output,
            input,
        }) => handle_script(&script, output, input),
        Some(Commands::Plugins { info }) => handle_plugins(info),
        Some(Commands::Serve { port, input }) => handle_serve(port, input),
        Some(Commands::Logo {
            output,
            variant,
            size,
        }) => handle_logo(&output, &variant, size),
        Some(Commands::Diff {
            target_a,
            target_b,
            file,
            json,
        }) => handle_diff(&target_a, target_b.as_deref(), file.as_deref(), json)
            .map_err(|e| e.into())
            .map(|_| false),
        Some(Commands::History { input, max, json }) => handle_history(&input, max, json)
            .map_err(|e| e.into())
            .map(|_| false),
        Some(Commands::Checkpoint { input, message }) => handle_checkpoint(&input, &message)
            .map_err(|e| e.into())
            .map(|_| false),
        Some(Commands::Restore { input, revision }) => handle_restore(&input, &revision)
            .map_err(|e| e.into())
            .map(|_| false),
    }
}
