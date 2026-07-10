pub mod app;
pub mod config;
pub mod core;

#[cfg(unix)]
use crate::core::server::UICommand;
#[cfg(windows)]
use crate::core::winserver::UICommand;

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
            start_ipc_listener(app_clone2, tx_clone);
        });
    }

    let layout = app_state.lock().unwrap().layout.clone();
    let titlebar = layout.options.titlebar;
    let window_size = layout.options.window_size;

    if layout.options.enable_overlay_server {
        let app_clone = app_state.clone();
        let port = layout.options.overlay_server_port;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(app::websocket_server::run(app_clone, port));
        });
    }

    let options = NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default()
            .with_decorations(titlebar)
            .with_inner_size(egui::vec2(window_size.0 as f32, window_size.1 as f32)),
        ..Default::default()
    };

    eframe::run_native(
        "OpenSpeedRun",
        options,
        Box::new(move |cc| Ok(Box::new(AppWrapper::new(app_state, rx, cc)))),
    )
}
