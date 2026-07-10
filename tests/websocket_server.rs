use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use openspeedrun::app::websocket_server;
use openspeedrun::{AppState, Run};

/// Drives the real bind/accept/handshake/send path against an actual
/// WebSocket client — the pure `overlay::build_snapshot` tests already
/// cover the JSON contents, this covers the networking plumbing around it.
#[tokio::test]
async fn overlay_server_streams_a_json_snapshot_to_a_connecting_client() {
    let run = Run::new("Test Game", "Any%", &["A", "B"]);
    let app = Arc::new(Mutex::new(AppState {
        splits_display: run.splits.clone(),
        run,
        ..AppState::empty_for_test()
    }));

    // Port 0: let the OS pick a free one, so this doesn't collide with a
    // real openspeedrun instance (or another test run) on a fixed port.
    let listener = websocket_server::bind(0)
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(websocket_server::serve(listener, app));

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
        .await
        .expect("client should connect and complete the WS handshake");

    let message = ws
        .next()
        .await
        .expect("stream shouldn't end")
        .expect("frame shouldn't be an error");
    let text = message.into_text().expect("should be a text frame");

    let json: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(json["title"], "Test Game");
    assert_eq!(json["category"], "Any%");
    assert_eq!(json["total_splits"], 2);
    assert_eq!(json["timer_state"], "not_started");
}

#[tokio::test]
async fn overlay_server_keeps_streaming_updated_snapshots() {
    let run = Run::new("Test Game", "Any%", &["A"]);
    let app = Arc::new(Mutex::new(AppState {
        splits_display: run.splits.clone(),
        run,
        ..AppState::empty_for_test()
    }));

    let listener = websocket_server::bind(0)
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(websocket_server::serve(listener, app.clone()));

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
        .await
        .expect("connect");

    // First snapshot, before anything changed.
    let first = ws.next().await.unwrap().unwrap().into_text().unwrap();
    let first: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["attempts"], 0);

    // Mutate the shared state the way a real attempt-completion would, and
    // confirm the *next* broadcast reflects it rather than being stuck on
    // the value from when the connection was accepted.
    app.lock().unwrap().run.attempts = 1;

    let updated = loop {
        let text = ws.next().await.unwrap().unwrap().into_text().unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        if value["attempts"] == 1 {
            break value;
        }
    };
    assert_eq!(updated["attempts"], 1);
}
