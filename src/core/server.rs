use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;

use std::sync::mpsc::Sender;

use crate::app::AppState;
use crate::core::socket_path;

#[derive(Debug)]
pub enum UICommand {
    ReloadShader,
}

pub async fn listen_for_commands(app: Arc<Mutex<AppState>>, tx: Sender<UICommand>) {
    let socket_path = socket_path();

    if socket_path.exists() {
        std::fs::remove_file(&socket_path).expect("Failed to remove existing socket file");
    }

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind to socket");

    // Unix sockets otherwise inherit permissions from umask, which can leave
    // them connectable by other local users — restrict to owner-only so a
    // shared-path fallback (see `socket_path`) can't be used to send
    // start/split/reset commands to someone else's timer.
    let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));

    println!("Listening for commands on {}", socket_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let app = app.clone();
                let tx = tx.clone();

                tokio::spawn(async move {
                    let reader = BufReader::new(stream);
                    let mut lines = reader.lines();

                    if let Ok(Some(line)) = lines.next_line().await {
                        let cmd = line.trim();
                        println!("Received command: '{}'", cmd);
                        if cmd.is_empty() {
                            eprintln!("Empty command received.");
                            return;
                        }

                        if cmd == "reloadshader" {
                            let _ = tx.send(UICommand::ReloadShader);
                            return;
                        }

                        let mut app = app.lock().unwrap();
                        match cmd {
                            "split" => app.split(),
                            "start" => app.start_timers(),
                            "pause" => app.pause_timers(),
                            "reset" => app.reset_splits(),
                            "savepb" => {
                                if let Err(e) = app.save_comparisons() {
                                    eprintln!("Error saving comparisons: {}", e);
                                }
                            }
                            "undolastsplit" => app.undo_split(),
                            "loadbackup" => app.undo_pb(),
                            "toggleloading" => app.toggle_igt_pause(),
                            "cyclecomparison" => app.cycle_comparison(),
                            "nextpage" => {
                                let total_splits = app.run.splits.len();
                                let total_pages =
                                    (total_splits + app.splits_per_page - 1) / app.splits_per_page;
                                if app.current_page + 1 < total_pages {
                                    app.current_page += 1;
                                }
                            }
                            "prevpage" => {
                                if app.current_page > 0 {
                                    app.current_page -= 1;
                                }
                            }
                            "togglehelp" => app.show_help = !app.show_help,
                            "reloadall" => app.reload_all(),
                            "reloadrun" => app.reload_run(),
                            "reloadtheme" => app.reload_theme(),
                            other => eprintln!("Unknown command: '{}'", other),
                        }
                    }
                });
            }
            Err(e) => eprintln!("Accept failed: {}", e),
        }
    }
}
