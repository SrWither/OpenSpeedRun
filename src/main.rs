pub mod config;
pub mod core;
pub mod app;

use crate::core::server::listen_for_commands;
use eframe::NativeOptions;
use egui::ViewportBuilder;
use app::state::{AppState, AppWrapper};
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let app_clone = app_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(listen_for_commands(app_clone));
    });

    let layout = app_state.lock().unwrap().layout.clone();
    let titlebar = layout.titlebar;
    let window_size = layout.window_size;

    let mut options = NativeOptions::default();
    options.viewport = ViewportBuilder::default()
        .with_decorations(titlebar)
        .with_inner_size(egui::vec2(
            window_size.0 as f32,
            window_size.1 as f32,
        ));
    

    eframe::run_native(
        "OpenSpeedRun",
        options,
        Box::new(move |_| Ok(Box::new(AppWrapper { app_state }))),
    )
}
