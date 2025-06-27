use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::gui::AppState;

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
                            "start" => app.timer.start(),
                            "pause" => app.timer.pause(),
                            "reset" => app.reset_splits(),
                            other => eprintln!("Unknown command: '{}'", other),
                        }
                    }
                });
            }
            Err(e) => eprintln!("Accept failed: {}", e),
        }
    }
}
