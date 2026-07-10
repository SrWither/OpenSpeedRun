use openspeedrun::autosplitter::config::{
    Action, AutosplitterConfig, Condition, Endian, ValueType, Watch,
};
#[cfg(target_os = "linux")]
use openspeedrun::autosplitter::process_memory::ProcessMemoryReader;
use openspeedrun::autosplitter::process_memory::{find_module_base, resolve_pointer_chain};
use openspeedrun::autosplitter::retroarch::parse_read_memory_response;

#[test]
fn condition_never_fires_on_the_first_sample() {
    // No `previous` yet, so there's nothing to transition from.
    assert!(!Condition::Equals { value: 5 }.triggered(None, 5));
    assert!(!Condition::Changed.triggered(None, 5));
    assert!(!Condition::Increased.triggered(None, 5));
}

#[test]
fn equals_fires_only_on_the_transition_into_the_value() {
    let cond = Condition::Equals { value: 2 };
    assert!(cond.triggered(Some(1), 2));
    // Already at 2 on the previous sample: must not re-fire every tick.
    assert!(!cond.triggered(Some(2), 2));
    assert!(!cond.triggered(Some(1), 3));
}

#[test]
fn not_equals_fires_only_on_leaving_the_value() {
    let cond = Condition::NotEquals { value: 2 };
    assert!(cond.triggered(Some(2), 3));
    assert!(!cond.triggered(Some(3), 4));
    assert!(!cond.triggered(Some(2), 2));
}

#[test]
fn greater_than_fires_only_on_crossing_the_threshold_upward() {
    let cond = Condition::GreaterThan { value: 10 };
    assert!(cond.triggered(Some(10), 11));
    // Still above threshold next tick: no re-fire.
    assert!(!cond.triggered(Some(11), 12));
    assert!(!cond.triggered(Some(5), 5));
}

#[test]
fn less_than_fires_only_on_crossing_the_threshold_downward() {
    let cond = Condition::LessThan { value: 10 };
    assert!(cond.triggered(Some(10), 9));
    assert!(!cond.triggered(Some(9), 8));
}

#[test]
fn increased_decreased_and_changed() {
    assert!(Condition::Increased.triggered(Some(1), 2));
    assert!(!Condition::Increased.triggered(Some(2), 2));
    assert!(Condition::Decreased.triggered(Some(2), 1));
    assert!(!Condition::Decreased.triggered(Some(1), 1));
    assert!(Condition::Changed.triggered(Some(1), 2));
    assert!(!Condition::Changed.triggered(Some(2), 2));
}

#[test]
fn value_type_decodes_signed_and_unsigned_widths_in_both_endians() {
    assert_eq!(ValueType::U8.decode(&[0xFF], Endian::Little), Some(255));
    assert_eq!(ValueType::I8.decode(&[0xFF], Endian::Little), Some(-1));

    assert_eq!(
        ValueType::U16.decode(&[0x01, 0x00], Endian::Little),
        Some(1)
    );
    assert_eq!(ValueType::U16.decode(&[0x00, 0x01], Endian::Big), Some(1));

    assert_eq!(
        ValueType::I32.decode(&[0xFF, 0xFF, 0xFF, 0xFF], Endian::Little),
        Some(-1)
    );
    assert_eq!(
        ValueType::U32.decode(&[0xFF, 0xFF, 0xFF, 0xFF], Endian::Little),
        Some(u32::MAX as i128)
    );

    assert_eq!(
        ValueType::U64.decode(&[1, 0, 0, 0, 0, 0, 0, 0], Endian::Little),
        Some(1)
    );
}

#[test]
fn value_type_decode_returns_none_on_a_short_buffer() {
    assert_eq!(ValueType::U32.decode(&[1, 2], Endian::Little), None);
}

#[test]
fn watch_address_parses_with_and_without_0x_prefix() {
    let mk = |addr: &str| Watch {
        name: "w".to_string(),
        address: addr.to_string(),
        module: None,
        pointer_path: Vec::new(),
        value_type: ValueType::U8,
        endian: Endian::Little,
        condition: Condition::Changed,
        action: Action::Split,
    };

    assert_eq!(mk("0x1A").address(), Ok(0x1A));
    assert_eq!(mk("1A").address(), Ok(0x1A));
    assert!(mk("not hex").address().is_err());
}

#[test]
fn action_maps_to_the_same_command_vocabulary_the_control_socket_expects() {
    assert_eq!(Action::Start.as_command(), "start");
    assert_eq!(Action::Split.as_command(), "split");
    assert_eq!(Action::Reset.as_command(), "reset");
    assert_eq!(Action::Pause.as_command(), "pause");
}

#[test]
fn parses_a_successful_read_core_memory_response() {
    let response = "READ_CORE_MEMORY 7e0020 01 02 ff";
    assert_eq!(
        parse_read_memory_response(response),
        Some(vec![0x01, 0x02, 0xFF])
    );
}

#[test]
fn parses_a_failed_read_core_memory_response_as_none() {
    assert_eq!(
        parse_read_memory_response("READ_CORE_MEMORY 7e0020 -1"),
        None
    );
}

#[test]
fn rejects_a_response_from_an_unexpected_command() {
    assert_eq!(parse_read_memory_response("VERSION 1.9.0"), None);
}

#[test]
fn loads_a_minimal_config_relying_on_retroarch_defaults() {
    let dir = std::env::temp_dir().join(format!("osr_autosplitter_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("autosplitter.json");
    std::fs::write(
        &path,
        r#"{
            "target": { "kind": "retroarch" },
            "watches": [
                {
                    "name": "room_id",
                    "address": "0x7E0020",
                    "value_type": "u8",
                    "condition": { "kind": "changed" },
                    "action": "split"
                }
            ]
        }"#,
    )
    .unwrap();

    let config = AutosplitterConfig::load_from_file(path.to_str().unwrap()).expect("should parse");
    assert_eq!(config.poll_interval_ms, 50);
    assert_eq!(config.watches.len(), 1);
    assert_eq!(config.watches[0].address(), Ok(0x7E0020));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loads_a_process_memory_config_with_module_and_pointer_path() {
    let dir = std::env::temp_dir().join(format!("osr_autosplitter_pm_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("autosplitter.json");
    std::fs::write(
        &path,
        r#"{
            "target": { "kind": "process_memory", "process_name": "game.bin" },
            "watches": [
                {
                    "name": "room_id",
                    "address": "0x4A9F00",
                    "module": "game.bin",
                    "pointer_path": ["0x18", "0x10"],
                    "value_type": "u16",
                    "condition": { "kind": "changed" },
                    "action": "split"
                }
            ]
        }"#,
    )
    .unwrap();

    let config = AutosplitterConfig::load_from_file(path.to_str().unwrap()).expect("should parse");
    assert_eq!(config.watches[0].module.as_deref(), Some("game.bin"));
    assert_eq!(
        config.watches[0].pointer_path_values(),
        Ok(vec![0x18, 0x10])
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
#[cfg(target_os = "linux")]
fn process_memory_reader_reads_a_known_value_from_its_own_process() {
    // A process can always read its own /proc/self/mem regardless of
    // ptrace_scope (self-access isn't restricted), so this exercises the
    // real read path without needing a second process or elevated
    // permissions — only cross-process attachment needs those, and that
    // part has to be tested by hand against a real target. `/proc` itself
    // is Linux-only (see `process_memory`'s module docs), hence the cfg.
    let value: u32 = 0xDEADBEEF;
    let addr = &value as *const u32 as u64;

    let reader = ProcessMemoryReader::open(std::process::id()).expect("should open /proc/self/mem");
    let bytes = reader.read_at(addr, 4).expect("should read our own stack");
    assert_eq!(u32::from_ne_bytes(bytes.try_into().unwrap()), value);
}

#[test]
fn find_module_base_matches_the_exact_file_name_not_a_substring() {
    let maps = "\
55a1b2c00000-55a1b2c04000 r-xp 00000000 08:01 123456 /usr/bin/game.bin
55a1b2e00000-55a1b2e10000 rw-p 00000000 00:00 0
7f9a00000000-7f9a00021000 r-xp 00000000 08:01 654321 /usr/lib/libGL.so
7f9a00100000-7f9a00110000 r--p 00000000 08:01 654322 /usr/lib/libGL.so.1
";

    assert_eq!(find_module_base(maps, "game.bin"), Some(0x55a1b2c00000));
    assert_eq!(find_module_base(maps, "libGL.so"), Some(0x7f9a00000000));
    // Must not match "libGL.so" against "libGL.so.1"'s mapping.
    assert_ne!(find_module_base(maps, "libGL.so"), Some(0x7f9a00100000));
    assert_eq!(find_module_base(maps, "nonexistent.so"), None);
}

#[test]
fn resolve_pointer_chain_with_no_offsets_returns_the_base_untouched() {
    let read = |_addr: u64| -> Option<u64> { panic!("shouldn't read anything") };
    assert_eq!(resolve_pointer_chain(read, 0x1000, &[]), Some(0x1000));
}

#[test]
fn resolve_pointer_chain_walks_a_multi_level_pointer() {
    use std::collections::HashMap;

    // base(0x1000) -> pointer 0x2000; (0x2000 + 0x18) -> pointer 0x3000;
    // final address = 0x3000 + 0x10.
    let mut memory: HashMap<u64, u64> = HashMap::new();
    memory.insert(0x1000, 0x2000);
    memory.insert(0x2018, 0x3000);

    let read = |addr: u64| memory.get(&addr).copied();
    assert_eq!(
        resolve_pointer_chain(read, 0x1000, &[0x18, 0x10]),
        Some(0x3010)
    );
}

#[test]
fn resolve_pointer_chain_fails_if_a_dereference_misses() {
    let read = |_addr: u64| -> Option<u64> { None };
    assert_eq!(resolve_pointer_chain(read, 0x1000, &[0x18]), None);
}
