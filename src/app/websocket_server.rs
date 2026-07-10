//! Local WebSocket server streaming `app::overlay::OverlaySnapshot` as JSON
//! to every connected client, meant for an OBS browser-source overlay (or
//! any other custom overlay/companion tool). Bound to `127.0.0.1` only —
//! this is meant for the same machine's OBS instance, not the network.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use crate::app::overlay::build_snapshot;
use crate::app::state::AppState;

/// How often a snapshot is pushed to each connected client. Doesn't need to
/// track the app's actual frame rate — this only feeds a browser-source
/// overlay, not the timer's own display.
const BROADCAST_INTERVAL: Duration = Duration::from_millis(33);

/// Binds the listening socket. Split out from `run` (which binds *and*
/// serves forever) so tests can bind to an OS-assigned port (`port: 0`),
/// read back the real port via `TcpListener::local_addr`, and drive `serve`
/// directly against a real client connection.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(format!("127.0.0.1:{port}")).await
}

/// Accepts connections forever, spawning a broadcast task per client.
pub async fn serve(listener: TcpListener, app: Arc<Mutex<AppState>>) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, app).await {
                        eprintln!("Overlay server: connection from {peer} ended: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Overlay server: accept failed: {e}"),
        }
    }
}

pub async fn run(app: Arc<Mutex<AppState>>, port: u16) {
    match bind(port).await {
        Ok(listener) => {
            println!("Overlay server listening on ws://127.0.0.1:{port}");
            serve(listener, app).await;
        }
        Err(e) => eprintln!("Overlay server: failed to bind 127.0.0.1:{port}: {e}"),
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    app: Arc<Mutex<AppState>>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let mut ws = tokio_tungstenite::accept_async(stream).await?;

    loop {
        let json = {
            let app = app.lock().unwrap();
            serde_json::to_string(&build_snapshot(&app)).unwrap_or_default()
        };

        ws.send(Message::Text(json)).await?;
        tokio::time::sleep(BROADCAST_INTERVAL).await;
    }
}
