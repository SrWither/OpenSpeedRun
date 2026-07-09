#[cfg(unix)]
pub mod server;
pub mod split;
pub mod timer;
#[cfg(windows)]
pub mod winserver;

/// Where the control socket lives. Prefers the per-user, already
/// `0700`-restricted `XDG_RUNTIME_DIR` (e.g. `/run/user/1000`); falls back to
/// a username-suffixed path in the shared temp dir on platforms without one
/// (e.g. macOS) so two local users don't collide on the same filename. The
/// socket file itself is also chmod'd to `0600` after bind (see
/// `core::server::listen_for_commands`), since that — not the path — is what
/// actually stops another local user from connecting and issuing commands.
#[cfg(unix)]
pub fn socket_path() -> std::path::PathBuf {
    if let Some(dir) = dirs::runtime_dir() {
        dir.join("openspeedrun.sock")
    } else {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        std::env::temp_dir().join(format!("openspeedrun-{user}.sock"))
    }
}
