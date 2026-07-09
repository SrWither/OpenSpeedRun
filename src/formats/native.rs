use std::fs;
use std::path::{Path, PathBuf};

/// Copies a run's `split.json` plus its `icons/` directory (if any) into
/// `dest_dir`, so the run can be handed to someone else or backed up as a
/// single self-contained folder instead of a loose path inside the config
/// directory.
pub fn export_folder(run_dir: &Path, dest_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest_dir)?;

    let split_json = run_dir.join("split.json");
    if split_json.exists() {
        fs::copy(&split_json, dest_dir.join("split.json"))?;
    }

    let icons_src = run_dir.join("icons");
    if icons_src.is_dir() {
        let icons_dest = dest_dir.join("icons");
        fs::create_dir_all(&icons_dest)?;
        for entry in fs::read_dir(&icons_src)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), icons_dest.join(entry.file_name()))?;
            }
        }
    }

    Ok(())
}

/// Copies a previously-exported run folder (`split.json` + `icons/`) from
/// `src_dir` into `splits_base_dir/name`, refusing to overwrite an existing
/// run with that name. Returns the new run's directory.
pub fn import_folder(
    src_dir: &Path,
    splits_base_dir: &Path,
    name: &str,
) -> std::io::Result<PathBuf> {
    let dest_dir = splits_base_dir.join(name);
    if dest_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("A run named '{name}' already exists"),
        ));
    }

    let src_split_json = src_dir.join("split.json");
    if !src_split_json.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No split.json found in {}", src_dir.display()),
        ));
    }

    export_folder(src_dir, &dest_dir)?;
    Ok(dest_dir)
}
