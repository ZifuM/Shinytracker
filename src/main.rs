// Hide the console window in release builds on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod canvas;
mod capture;
mod card;
mod config;
mod detect;
mod fonts;
mod gamelog;
mod goals;
mod hotkeys;
mod icon;
mod imageproc;
mod moddex;
mod ocr;
mod progress;
mod share;
mod sound;
mod species;
mod sprites;
mod stats;
mod theme;
mod topmost;
mod update;
mod worker;

use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    // Remove the previous version left behind by an auto-update.
    update::cleanup();
    let (icon_rgba, icon_w, icon_h) = icon::rgba();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Shiny Tracker")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_icon(Arc::new(egui::IconData {
            rgba: icon_rgba,
            width: icon_w,
            height: icon_h,
        }))
        .with_inner_size(if cfg.mini {
            app::MINI_SIZE
        } else {
            app::FULL_SIZE
        });
    if let Some([x, y]) = cfg.window_pos {
        viewport = viewport.with_position([x, y]);
    } else {
        viewport = viewport.with_position([24.0, 120.0]);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Shiny Tracker",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, cfg)))),
    )
}
