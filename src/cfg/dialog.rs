//! `rfd::FileDialog`'s `pick_file`/`save_file`/`pick_folder` block the
//! calling thread until the native dialog closes. Called directly from
//! `egui`'s update loop (as every callsite in this crate used to), that
//! freezes the whole window for as long as the dialog is open — no repaints
//! happen, so window managers (GNOME included) flag it as unresponsive.
//!
//! `PendingDialog` runs the blocking call on a background thread instead,
//! handing the result back over a channel that `poll` checks without
//! blocking. Usage: call `PendingDialog::spawn` from a button's `clicked()`
//! handler (instead of calling `rfd` directly), stash the returned
//! `PendingDialog` in the widget's state, and call `.poll()` once per frame
//! to pick up the result once the user closes the dialog.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

pub struct PendingDialog {
    receiver: Receiver<Option<PathBuf>>,
}

impl PendingDialog {
    /// Runs `open_dialog` (expected to be an `rfd::FileDialog` call) on a
    /// background thread.
    pub fn spawn(open_dialog: impl FnOnce() -> Option<PathBuf> + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // The receiver only ever disappears if the widget holding this
            // `PendingDialog` was dropped before the dialog closed; nothing
            // to do about that but let the send fail silently.
            let _ = tx.send(open_dialog());
        });
        Self { receiver: rx }
    }

    /// `None` while the dialog is still open; `Some(result)` exactly once,
    /// the frame it closes (`result` itself is `None` if the user cancelled).
    pub fn poll(&self) -> Option<Option<PathBuf>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}
