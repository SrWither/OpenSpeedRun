pub mod config;
pub mod core;
pub mod gui;
pub mod hotkeys;

use eframe::NativeOptions;

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native("OpenSpeedRun", options, Box::new(|_cc| Ok(Box::new(gui::AppState::default()))))
}