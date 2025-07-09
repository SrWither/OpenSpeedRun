pub mod app;
pub mod config;
pub mod core;

use crate::core::server::UICommand;
#[cfg(unix)]
use crate::core::server::listen_for_commands;
#[cfg(windows)]
use crate::core::winserver::{listen_for_hotkeys, start_ipc_listener};
use app::state::{AppState, AppWrapper};
use eframe::NativeOptions;
use egui::ViewportBuilder;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let app_clone = app_state.clone();

    let (tx, rx) = mpsc::channel::<UICommand>();
    let tx_clone = tx.clone();

    #[cfg(unix)]
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(listen_for_commands(app_clone, tx_clone));
    });

    #[cfg(windows)]
    {
        let app_clone1 = app_clone.clone();
        std::thread::spawn(move || {
            listen_for_hotkeys(app_clone1);
        });

        let app_clone2 = app_clone.clone();
        std::thread::spawn(move || {
            start_ipc_listener(app_clone2);
        });
    }

    let layout = app_state.lock().unwrap().layout.clone();
    let titlebar = layout.options.titlebar;
    let window_size = layout.options.window_size;

    let mut options = NativeOptions::default();
    options.viewport = ViewportBuilder::default()
        .with_decorations(titlebar)
        .with_inner_size(egui::vec2(window_size.0 as f32, window_size.1 as f32));

    eframe::run_native(
        "OpenSpeedRun",
        options,
        Box::new(move |cc| Ok(Box::new(AppWrapper::new(app_state, rx, cc)))),
    )
}
