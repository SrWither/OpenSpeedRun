use crate::app::AppState;
use named_pipe::PipeOptions;
use rdev::{Event, EventType, listen};
use std::io::{BufRead, BufReader};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
pub enum UICommand {
    ReloadShader,
}

pub fn listen_for_hotkeys(app: Arc<Mutex<AppState>>) {
    std::thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            handle_event(event, &app);
        }) {
            eprintln!("Error: {:?}", error);
        }
    });
}

fn handle_event(event: Event, app: &Arc<Mutex<AppState>>) {
    if let EventType::KeyPress(key) = event.event_type {
        let mut app = app.lock().unwrap();

        let hotkeys = app.layout.hotkeys.clone();

        macro_rules! check_and_run {
            ($hotkey:expr, $action:block) => {
                if let Some(expected_key) = $hotkey.as_key() {
                    if expected_key == key {
                        $action
                    }
                }
            };
        }

        check_and_run!(&hotkeys.split, {
            app.split();
        });

        check_and_run!(&hotkeys.start, {
            let offset = app.run.start_offset.unwrap_or(0);
            app.timer.start_with_offset(offset);
        });

        check_and_run!(&hotkeys.pause, {
            app.timer.pause();
        });

        check_and_run!(&hotkeys.reset, {
            app.reset_splits();
        });

        check_and_run!(&hotkeys.save_pb, {
            if let Err(e) = app.save_pb() {
                eprintln!("Error saving PB: {}", e);
            }
        });

        check_and_run!(&hotkeys.undo_split, {
            app.undo_split();
        });

        check_and_run!(&hotkeys.undo_pb, {
            app.undo_pb();
        });

        check_and_run!(&hotkeys.next_page, {
            let total_splits = app.run.splits.len();
            let total_pages = (total_splits + app.splits_per_page - 1) / app.splits_per_page;
            if app.current_page + 1 < total_pages {
                app.current_page += 1;
            }
        });

        check_and_run!(&hotkeys.prev_page, {
            if app.current_page > 0 {
                app.current_page -= 1;
            }
        });

        check_and_run!(&hotkeys.toggle_help, {
            app.show_help = !app.show_help;
        });

        check_and_run!(&hotkeys.reload_all, {
            app.reload_all();
        });

        check_and_run!(&hotkeys.reload_run, {
            app.reload_run();
        });

        check_and_run!(&hotkeys.reload_theme, {
            app.reload_theme();
        });
    }
}

pub fn start_ipc_listener(app: Arc<Mutex<AppState>>, tx: Sender<UICommand>) {
    let pipe_name = r"\\.\pipe\openspeedrun";

    thread::spawn(move || {
        println!("IPC listener running on named pipe {}", pipe_name);

        loop {
            let connecting = match PipeOptions::new(pipe_name).single() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("❌ Failed to create pipe server: {}", e);
                    thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };
            let mut server = match connecting.wait() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("❌ Failed to connect to client: {}", e);
                    continue;
                }
            };

            let reader = BufReader::new(&mut server);
            let app = Arc::clone(&app);
            let tx = tx.clone();

            for line in reader.lines() {
                match line {
                    Ok(cmd) => {
                        handle_ipc_command(&app, &tx, cmd.trim());
                    }
                    Err(e) => {
                        eprintln!("⚠️ Error reading from pipe: {}", e);
                        break;
                    }
                }
            }
        }
    });
}

fn handle_ipc_command(app: &Arc<Mutex<AppState>>, tx: &Sender<UICommand>, cmd: &str) {
    match cmd.to_lowercase().as_str() {
        "reloadshader" => {
            println!("Command: reloadshader");
            let _ = tx.send(UICommand::ReloadShader);
        }
        "reloadtheme" => {
            println!("Command: reloadtheme");
            let mut app = app.lock().unwrap();
            app.reload_theme();
        }
        "reloadall" => {
            println!("Command: reloadall");
            let mut app = app.lock().unwrap();
            app.reload_all();
        }
        "reloadrun" => {
            println!("Command: reloadrun");
            let mut app = app.lock().unwrap();
            app.reload_run();
        }
        other => {
            println!("Unknown command received: {}", other);
        }
    }
}
