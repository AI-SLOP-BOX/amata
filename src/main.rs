#![allow(dead_code)]

mod app;
mod cli;
mod core;
mod gpu;
mod io;
mod plugin;
mod tools;
mod ui;

use crate::cli::{run_cli, Cli};
use app::IrasuApp;
use clap::Parser;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match run_cli(cli) {
        Ok(false) => {
            // CLI command ran and finished
            return Ok(());
        }
        Err(err) => {
            eprintln!("❌ Error: {err}");
            std::process::exit(1);
        }
        Ok(true) => {
            // Launch GUI
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Amata (数多) — Pro Vector Graphics Studio"),
        ..Default::default()
    };

    let initial_file = crate::cli::take_initial_file();

    eframe::run_native(
        "Amata Vector Studio",
        options,
        Box::new(move |cc| {
            crate::ui::setup_custom_fonts(&cc.egui_ctx);
            crate::ui::apply_adobe_theme(&cc.egui_ctx);
            Ok(Box::new(IrasuApp::with_file(initial_file)))
        }),
    )
}
