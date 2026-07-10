#[cfg(windows)]
pub mod keys;
pub mod layout;
pub mod load;
pub mod shaders;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static ATOMIC_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `contents` to `path` without ever leaving a truncated/empty file
/// visible to a concurrent reader: writes to a sibling temp file first, then
/// renames it into place (atomic on the same filesystem), instead of
/// `std::fs::write`'s truncate-then-write (which a reader can catch
/// mid-write). The temp file name includes both the PID and a per-call
/// counter, since two racing writers can be two *threads* in the same
/// process (same PID) — e.g. two parallel tests both triggering "create the
/// default split/theme on first run" against the same config directory.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    let n = ATOMIC_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut tmp_path = path.as_os_str().to_owned();
    tmp_path.push(format!(".tmp-{}-{n}", std::process::id()));
    let tmp_path = std::path::PathBuf::from(tmp_path);

    std::fs::write(&tmp_path, contents)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
