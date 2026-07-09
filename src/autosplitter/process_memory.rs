//! Reads a native process's memory directly via `/proc/<pid>/mem`. Opt-in
//! and deliberately not the default target — see `Target::ProcessMemory`'s
//! docs and the crate-level module docs for why this needs the target
//! process to be ptrace-readable, a real security trade-off rather than a
//! permission to just paper over.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

pub struct ProcessMemoryReader {
    mem: File,
}

impl ProcessMemoryReader {
    /// Opens `/proc/<pid>/mem` for reading. On `EPERM`/`EACCES` this returns
    /// an error whose message explains *why* (Yama `ptrace_scope` or
    /// missing `CAP_SYS_PTRACE`) rather than just "permission denied", since
    /// that's the single most common reason this fails and the fix isn't
    /// obvious from the raw OS error.
    pub fn open(pid: u32) -> io::Result<Self> {
        let path = format!("/proc/{pid}/mem");
        match File::open(&path) {
            Ok(mem) => Ok(Self { mem }),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "cannot open {path}: permission denied. Reading another process's memory on \
                     Linux requires ptrace access to it, which is restricted by default (Yama \
                     `ptrace_scope`). Either run: `echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope` \
                     (session-only, resets on reboot, and relaxes this for every process, not just \
                     this one), or grant just this binary the capability instead: `sudo setcap \
                     cap_sys_ptrace=ep $(command -v openspeedrun-autosplitter)`."
                ),
            )),
            Err(e) => Err(e),
        }
    }

    pub fn read_at(&self, addr: u64, size: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        // Cloning the handle (rather than taking &mut self) so callers don't
        // need a mutable reference just to read; pread-style seek+read on an
        // independent fd offset would be nicer but isn't in std, and this
        // reader is only ever driven from a single-threaded poll loop.
        let mut mem = self.mem.try_clone()?;
        mem.seek(SeekFrom::Start(addr))?;
        mem.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_u64(&self, addr: u64) -> io::Result<u64> {
        let bytes = self.read_at(addr, 8)?;
        Ok(u64::from_ne_bytes(bytes.try_into().unwrap()))
    }
}

/// Scans `/proc` for a process whose name matches `process_name`, returning
/// its PID. Locating a process by name never itself requires ptrace
/// permission (only the later `/proc/<pid>/mem` open does), so this works
/// even before any elevated access has been granted.
pub fn find_pid_by_name(process_name: &str) -> io::Result<Option<u32>> {
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };

        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let exe_basename = std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));

        if matches_process_name(&comm, exe_basename.as_deref(), process_name) {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Pure matching logic behind `find_pid_by_name`, split out so it's
/// testable without real `/proc` entries. `comm` is compared as-is (the
/// kernel truncates it to 15 bytes, so a config targeting a long executable
/// name may need to use the truncated form); `exe_basename` is the
/// untruncated fallback when `/proc/<pid>/exe` was readable.
pub fn matches_process_name(comm: &str, exe_basename: Option<&str>, target: &str) -> bool {
    comm == target || exe_basename == Some(target)
}

/// Parses `/proc/<pid>/maps`-format text to find `module_name`'s load base
/// address — the start of the first mapping whose path's file name matches
/// exactly (not a substring match, so a query for `"GL.so"` doesn't
/// accidentally match `"libGL.so"`). Pure/testable: takes the maps text
/// directly rather than reading `/proc` itself.
pub fn find_module_base(maps_text: &str, module_name: &str) -> Option<u64> {
    for line in maps_text.lines() {
        let Some(range) = line.split_whitespace().next() else {
            continue;
        };
        let Some((_, rest)) = line.split_once('/') else {
            continue;
        };
        let path = format!("/{rest}");

        let file_name = path.rsplit('/').next().unwrap_or(&path);
        if file_name != module_name {
            continue;
        }

        let start_hex = range.split('-').next()?;
        return u64::from_str_radix(start_hex, 16).ok();
    }
    None
}

/// Chases an ASL-style pointer path: `base` is read as a pointer, each
/// `pointer_path` offset (except the last) is added and re-read as a
/// pointer, and the final offset is added *without* a further read — that
/// last address is where the watch's actual value lives. Pure: takes a
/// `read_u64` callback instead of a live reader, so the chain-walking logic
/// is testable with a fake/mocked memory image.
pub fn resolve_pointer_chain(
    read_u64: impl Fn(u64) -> Option<u64>,
    base: u64,
    pointer_path: &[u64],
) -> Option<u64> {
    if pointer_path.is_empty() {
        return Some(base);
    }

    let mut ptr = read_u64(base)?;
    for offset in &pointer_path[..pointer_path.len() - 1] {
        ptr = read_u64(ptr + offset)?;
    }
    Some(ptr + pointer_path[pointer_path.len() - 1])
}
