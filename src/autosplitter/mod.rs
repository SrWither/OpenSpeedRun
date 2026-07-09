//! Autosplitting support. Two targets exist, with very different privilege
//! requirements:
//!
//! - `Target::Retroarch` reads emulated RAM over RetroArch's plaintext UDP
//!   "Network Command Interface" — opt-in on the emulator's side, no
//!   elevated privileges needed at all. This is the target to prefer.
//! - `Target::ProcessMemory` reads a native process's memory directly via
//!   `/proc/<pid>/mem`, which on Linux requires ptrace access to that
//!   process — either the system-wide Yama restriction relaxed
//!   (`ptrace_scope`) or `CAP_SYS_PTRACE` granted to
//!   `openspeedrun-autosplitter` itself. That's a real reduction in the
//!   OS's process-isolation guarantees, not a rubber-stamp permission, which
//!   is why nothing in the config format defaults to it — see
//!   `config::Target::ProcessMemory`'s docs and the README's "Native games"
//!   section before reaching for it.
//!
//! Either way, this binary only ever *reads* memory and only ever turns a
//! configured transition into one of the same `start`/`split`/`reset`/
//! `pause` commands `openspeedrun-cli` sends — see `core::socket_path` for
//! the shared control socket. If neither target fits your case (a game with
//! its own scripting/mod support, say), nothing stops you from writing your
//! own watcher that talks to that same socket directly; the socket protocol
//! is the integration point, not this module.

pub mod config;
pub mod process_memory;
pub mod retroarch;
