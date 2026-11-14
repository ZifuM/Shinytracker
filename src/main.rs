// Hide the console window in release builds on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod apps;
mod cards;
mod masonry;
mod battle;
mod breed;
mod breed_plan;
mod calc;
mod canvas;
mod capture;
mod card;
mod config;
mod cosmetics;
mod detect;
mod dexextra;
mod engine;
mod fonts;
mod gamemap;
mod gamelog;
mod bingo;
mod rare_calc;
mod rare_log;
mod replay;
#[cfg(all(windows, feature = "replay-rec"))]
mod replay_win;
mod gotchi;
mod goals;
mod hotkeys;
mod icon;
mod imageproc;
mod moddex;
mod poster;
mod poster_text;
mod ocr;
mod pass;
mod progress;
mod share;
mod sound;
mod species;
mod sprites;
mod safefile;
mod sim;
mod stats;
mod teams;
mod trade;
mod theme;
mod topmost;
mod update;
mod wiki;
mod worker;
mod worldclock;

use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    // Remove the previous version left behind by an auto-update.
    update::cleanup();
    let (icon_rgba, icon_w, icon_h) = icon::rgba();

    // The launcher is the main window; the overlay is a second window that
    // Shiny Hunter opens and closes.
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(topmost::LAUNCHER_TITLE)
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(true)
        .with_min_inner_size([720.0, 600.0])
        .with_icon(Arc::new(egui::IconData {
            rgba: icon_rgba,
            width: icon_w,
            height: icon_h,
        }))
        .with_inner_size(match cfg.settings_size {
            Some([w, h]) => [w.max(720.0), h.max(600.0)],
            None => [1100.0, 740.0],
        })
        .with_maximized(cfg.settings_maximized);
    match cfg.settings_pos {
        Some([x, y]) if topmost::point_on_screen([x + 60.0, y + 16.0], 1.0) => {
            viewport = viewport.with_position([x, y]);
        }
        _ => {}
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
