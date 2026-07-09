use serde::{Deserialize, Serialize};

/// On-disk shape of `autosplitter.json`, sitting next to `split.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosplitterConfig {
    pub target: Target,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    pub watches: Vec<Watch>,
}

fn default_poll_interval_ms() -> u64 {
    50
}

impl AutosplitterConfig {
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("Invalid autosplitter config in {path}: {e}"))
    }
}

/// What to read memory from. `Retroarch` needs no elevated privileges at all
/// (see module docs). `ProcessMemory` is opt-in only — nothing here
/// defaults to it, you must explicitly write `"kind": "process_memory"` —
/// because it requires the target process to be ptrace-readable, which on
/// Linux means either the system-wide Yama restriction relaxed
/// (`ptrace_scope`) or `CAP_SYS_PTRACE` granted to
/// `openspeedrun-autosplitter` itself. That's a real reduction in process
/// isolation, not a formality; see module docs and the README's
/// "Native games" section before reaching for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    /// `host`/`port` default to RetroArch's own defaults, so a minimal
    /// config only needs `{"kind": "retroarch"}`.
    Retroarch {
        #[serde(default = "default_retroarch_host")]
        host: String,
        #[serde(default = "default_retroarch_port")]
        port: u16,
    },
    /// Reads directly from a native process's `/proc/<pid>/mem`. The
    /// process is located by matching `process_name` against both
    /// `/proc/<pid>/comm` (note: the kernel truncates this to 15 bytes) and
    /// the `/proc/<pid>/exe` symlink's file name, whichever is readable.
    ProcessMemory { process_name: String },
}

fn default_retroarch_host() -> String {
    "127.0.0.1".to_string()
}

fn default_retroarch_port() -> u16 {
    55355
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Endian {
    #[default]
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}

impl ValueType {
    pub fn size(self) -> usize {
        match self {
            ValueType::U8 | ValueType::I8 => 1,
            ValueType::U16 | ValueType::I16 => 2,
            ValueType::U32 | ValueType::I32 => 4,
            ValueType::U64 | ValueType::I64 => 8,
        }
    }

    /// Decodes the first `self.size()` bytes of `bytes` as this type, widened
    /// to `i128` so every variant (signed or not, up to 64 bits) fits in one
    /// return type. Returns `None` if `bytes` is shorter than `self.size()`.
    pub fn decode(self, bytes: &[u8], endian: Endian) -> Option<i128> {
        let n = self.size();
        if bytes.len() < n {
            return None;
        }
        let b = &bytes[..n];

        Some(match (self, endian) {
            (ValueType::U8, _) => b[0] as i128,
            (ValueType::I8, _) => (b[0] as i8) as i128,
            (ValueType::U16, Endian::Little) => u16::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::U16, Endian::Big) => u16::from_be_bytes(b.try_into().ok()?) as i128,
            (ValueType::I16, Endian::Little) => i16::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::I16, Endian::Big) => i16::from_be_bytes(b.try_into().ok()?) as i128,
            (ValueType::U32, Endian::Little) => u32::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::U32, Endian::Big) => u32::from_be_bytes(b.try_into().ok()?) as i128,
            (ValueType::I32, Endian::Little) => i32::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::I32, Endian::Big) => i32::from_be_bytes(b.try_into().ok()?) as i128,
            (ValueType::U64, Endian::Little) => u64::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::U64, Endian::Big) => u64::from_be_bytes(b.try_into().ok()?) as i128,
            (ValueType::I64, Endian::Little) => i64::from_le_bytes(b.try_into().ok()?) as i128,
            (ValueType::I64, Endian::Big) => i64::from_be_bytes(b.try_into().ok()?) as i128,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watch {
    /// Just a label for logging/error messages and to key the "previous
    /// value" table watch-to-watch; not sent to the emulator/process.
    pub name: String,
    /// Hex address, with or without a leading `0x` (e.g. `"0x7E0020"` or
    /// `"7E0020"`). For `Target::Retroarch`, this is the address as-is in
    /// whatever address space `READ_CORE_MEMORY` expects for the loaded
    /// core. For `Target::ProcessMemory`, this is an offset from `module`'s
    /// base address if `module` is set, or an absolute address otherwise.
    pub address: String,
    /// `ProcessMemory`-only: resolve this address relative to the named
    /// module's (executable or shared library's) load base, read from
    /// `/proc/<pid>/maps`, instead of treating `address` as absolute.
    /// Ignored for `Target::Retroarch`.
    #[serde(default)]
    pub module: Option<String>,
    /// `ProcessMemory`-only: multi-level pointer chase, ASL-style — each
    /// entry is a hex offset. `address` (relative to `module` if set) is
    /// read as a 64-bit pointer, `pointer_path[0]` is added to it and *that*
    /// is read as a pointer, and so on; the watch's actual value is read at
    /// the address formed by the last offset. Empty means `address` already
    /// points straight at the value. Ignored for `Target::Retroarch`.
    #[serde(default)]
    pub pointer_path: Vec<String>,
    pub value_type: ValueType,
    #[serde(default)]
    pub endian: Endian,
    pub condition: Condition,
    pub action: Action,
}

impl Watch {
    pub fn address(&self) -> Result<u64, String> {
        parse_hex_u64(&self.address)
    }

    pub fn pointer_path_values(&self) -> Result<Vec<u64>, String> {
        self.pointer_path.iter().map(|s| parse_hex_u64(s)).collect()
    }
}

fn parse_hex_u64(raw: &str) -> Result<u64, String> {
    let s = raw.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u64::from_str_radix(s, 16).map_err(|e| format!("invalid hex value '{raw}': {e}"))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    Equals { value: i128 },
    NotEquals { value: i128 },
    GreaterThan { value: i128 },
    LessThan { value: i128 },
    Increased,
    Decreased,
    Changed,
}

impl Condition {
    /// Edge-triggered: fires only on the sample where the condition
    /// transitions from not-holding to holding, never on every sample while
    /// it continues to hold (e.g. `Equals` on a room ID that stays constant
    /// for the next 500 polls must not re-split each time). Never fires on
    /// the very first sample — with no `previous` to compare against, there's
    /// no way to tell a genuine transition from "this is just where the
    /// value happened to be when we attached".
    pub fn triggered(&self, previous: Option<i128>, current: i128) -> bool {
        let Some(previous) = previous else {
            return false;
        };
        match *self {
            Condition::Equals { value } => previous != value && current == value,
            Condition::NotEquals { value } => previous == value && current != value,
            Condition::GreaterThan { value } => previous <= value && current > value,
            Condition::LessThan { value } => previous >= value && current < value,
            Condition::Increased => current > previous,
            Condition::Decreased => current < previous,
            Condition::Changed => current != previous,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Start,
    Split,
    Reset,
    Pause,
}

impl Action {
    /// The exact string `core::server::listen_for_commands` matches on —
    /// see `cli/main.rs` for the same vocabulary.
    pub fn as_command(self) -> &'static str {
        match self {
            Action::Start => "start",
            Action::Split => "split",
            Action::Reset => "reset",
            Action::Pause => "pause",
        }
    }
}
