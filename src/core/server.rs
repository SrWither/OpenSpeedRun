use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;

use crate::app::AppState;

pub async fn listen_for_commands(app: Arc<Mutex<AppState>>) {
    let socket_path = "/tmp/openspeedrun.sock";

    if Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path).expect("Failed to remove existing socket file");
    }

    let listener = UnixListener::bind(socket_path).expect("Failed to bind to socket");

    println!("Listening for commands on {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let app = app.clone();

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

                        let mut app = app.lock().unwrap();
                        match cmd {
                            "split" => app.split(),
                            "start" => {
                                let offset = app.run.start_offset.unwrap_or(0);
                                app.timer.start_with_offset(offset);
                            }
                            "pause" => app.timer.pause(),
                            "reset" => app.reset_splits(),
                            "savepb" => {
                                if let Err(e) = app.save_pb() {
                                    eprintln!("Error saving PB: {}", e);
                                }
                            }
                            "undolastsplit" => app.undo_split(),
                            "loadbackup" => app.undo_pb(),
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
