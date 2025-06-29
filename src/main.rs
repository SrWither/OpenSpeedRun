pub mod config;
pub mod core;
pub mod gui;

use crate::core::server::listen_for_commands;
use eframe::NativeOptions;
use gui::AppState;
use gui::AppWrapper;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let app_clone = app_state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(listen_for_commands(app_clone));
    });

    let options = NativeOptions::default();
    
    eframe::run_native(
        "OpenSpeedRun",
        options,
        Box::new(move |_| Ok(Box::new(AppWrapper { app_state }))),
    )
}

