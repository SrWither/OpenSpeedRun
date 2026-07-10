use chrono::{DateTime, Duration, Utc};
use openspeedrun::Run;
use openspeedrun::core::split::{
    AttemptHistoryEntry, COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, RunVariable,
    SegmentHistoryEntry, TimingMethod,
};
use openspeedrun::formats::lss;

fn ms(n: i64) -> Duration {
    Duration::milliseconds(n)
}

/// `.lss` attempt dates only survive whole-second precision (see
/// `lss::build_xml`'s `%m/%d/%Y %H:%M:%S` format), so build one already
/// truncated to avoid a round-trip mismatch from sub-second jitter.
fn whole_second_now() -> DateTime<Utc> {
    DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap()
}

fn sample_run() -> Run {
    let mut run = Run::new("Test Game", "Any%", &["Intro", "Boss"]);
    run.attempts = 2;
    run.metadata.platform = Some("PC".to_string());
    run.metadata.region = Some("NTSC".to_string());
    run.metadata.variables.push(RunVariable {
        name: "Ruleset".to_string(),
        value: "Glitchless".to_string(),
    });

    run.attempt_history.push(AttemptHistoryEntry {
        run_index: 0,
        real_time: Some(Duration::seconds(120)),
        game_time: None,
        ended: true,
        date: Some(whole_second_now()),
    });

    run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(30_000));
    run.splits[0]
        .comparisons
        .get_mut(COMPARISON_BEST_SEGMENTS)
        .unwrap()
        .real_time = Some(ms(28_000));
    run.splits[0].segment_history.push(SegmentHistoryEntry {
        run_index: 0,
        real_time: Some(ms(30_000)),
        game_time: None,
    });

    run.splits[1]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(90_000));
    run.splits[1]
        .comparisons
        .get_mut(COMPARISON_BEST_SEGMENTS)
        .unwrap()
        .real_time = Some(ms(58_000));
    run.splits[1].segment_history.push(SegmentHistoryEntry {
        run_index: 0,
        real_time: Some(ms(60_000)),
        game_time: None,
    });

    run
}

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("osr_lss_test_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn export_then_import_round_trips_core_fields() {
    let run = sample_run();
    let dir = scratch_dir("roundtrip");
    let lss_path = dir.join("test.lss");
    let icons_dir = dir.join("icons");

    lss::export(&run, &lss_path, &dir).expect("export failed");
    let result = lss::import(&lss_path, &icons_dir).expect("import failed");
    let imported = result.run;

    assert_eq!(imported.title, run.title);
    assert_eq!(imported.category, run.category);
    assert_eq!(imported.attempts, run.attempts);
    assert_eq!(imported.metadata.platform, run.metadata.platform);
    assert_eq!(imported.metadata.region, run.metadata.region);
    assert_eq!(imported.metadata.variables.len(), 1);
    assert_eq!(imported.metadata.variables[0].name, "Ruleset");
    assert_eq!(imported.metadata.variables[0].value, "Glitchless");

    assert_eq!(imported.attempt_history.len(), 1);
    assert_eq!(imported.attempt_history[0].run_index, 0);
    assert!(imported.attempt_history[0].ended);
    assert_eq!(
        imported.attempt_history[0].real_time,
        run.attempt_history[0].real_time
    );
    assert_eq!(
        imported.attempt_history[0].date,
        run.attempt_history[0].date
    );

    assert_eq!(imported.splits.len(), run.splits.len());
    for (original, round_tripped) in run.splits.iter().zip(imported.splits.iter()) {
        assert_eq!(round_tripped.name, original.name);
        assert_eq!(
            round_tripped.comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
            original.comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
            "Personal Best mismatch for split {}",
            original.name
        );
        assert_eq!(
            round_tripped.comparison_time(COMPARISON_BEST_SEGMENTS, TimingMethod::RealTime),
            original.comparison_time(COMPARISON_BEST_SEGMENTS, TimingMethod::RealTime),
            "Best Segments mismatch for split {}",
            original.name
        );
        assert_eq!(
            round_tripped.segment_history.len(),
            original.segment_history.len()
        );
        assert_eq!(
            round_tripped.segment_history[0].real_time,
            original.segment_history[0].real_time
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_then_import_round_trips_icon_bytes() {
    let dir = scratch_dir("icons");

    // icon_path is always relative to the directory holding split.json (see
    // cfg/split_editor.rs), so exercise that same layout on both sides:
    // export reads from `<base>/icons/...`, import writes to a fresh
    // `<base>/icons/...` of its own.
    let export_base = dir.join("export_side");
    let export_icons_dir = export_base.join("icons");
    std::fs::create_dir_all(&export_icons_dir).unwrap();
    let icon_path = export_icons_dir.join("split0.png");
    image::RgbaImage::new(2, 2)
        .save(&icon_path)
        .expect("failed to write test png");
    let original_bytes = std::fs::read(&icon_path).unwrap();

    let mut run = Run::new("Test Game", "Any%", &["Intro"]);
    run.splits[0].icon_path = Some("icons/split0.png".to_string());

    let lss_path = dir.join("test.lss");
    lss::export(&run, &lss_path, &export_base).expect("export failed");

    let import_base = dir.join("import_side");
    let result = lss::import(&lss_path, &import_base.join("icons")).expect("import failed");

    let imported_icon_path = result.run.splits[0]
        .icon_path
        .as_ref()
        .expect("icon should round-trip");
    let imported_bytes = std::fs::read(import_base.join(imported_icon_path)).unwrap();

    assert_eq!(imported_bytes, original_bytes);

    std::fs::remove_dir_all(&dir).ok();
}

/// Standard-alphabet base64 decoder, only for this test to check the raw
/// bytes `lss::export` writes — not a claim that the crate needs a real
/// base64 dependency (see `lss.rs`'s own private encoder/decoder, which
/// exist for exactly the same one-off reason).
fn base64_decode(input: &str) -> Vec<u8> {
    fn value(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("unexpected base64 byte: {c}"),
        }
    }
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in cleaned.chunks(4) {
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = if b == b'=' { 0 } else { value(b) };
        }
        let n = ((buf[0] as u32) << 18)
            | ((buf[1] as u32) << 12)
            | ((buf[2] as u32) << 6)
            | (buf[3] as u32);
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    out
}

/// Real bytes captured from LiveSplit itself (installed under Wine): a
/// `<Icon>` value it wrote for a genuine 32x32 PNG, base64-decoded straight
/// out of a `.lss` file LiveSplit saved. This is the exact `.NET
/// BinaryFormatter`-wrapped `System.Drawing.Bitmap` framing real LiveSplit
/// expects — not a spec-only reconstruction.
#[rustfmt::skip]
const LIVESPLIT_REAL_ICON_BYTES: &[u8] = b"\
\x00\x01\x00\x00\x00\xff\xff\xff\xff\x01\x00\x00\x00\x00\x00\x00\
\x00\x0c\x02\x00\x00\x00\x51\x53\x79\x73\x74\x65\x6d\x2e\x44\x72\
\x61\x77\x69\x6e\x67\x2c\x20\x56\x65\x72\x73\x69\x6f\x6e\x3d\x34\
\x2e\x30\x2e\x30\x2e\x30\x2c\x20\x43\x75\x6c\x74\x75\x72\x65\x3d\
\x6e\x65\x75\x74\x72\x61\x6c\x2c\x20\x50\x75\x62\x6c\x69\x63\x4b\
\x65\x79\x54\x6f\x6b\x65\x6e\x3d\x62\x30\x33\x66\x35\x66\x37\x66\
\x31\x31\x64\x35\x30\x61\x33\x61\x05\x01\x00\x00\x00\x15\x53\x79\
\x73\x74\x65\x6d\x2e\x44\x72\x61\x77\x69\x6e\x67\x2e\x42\x69\x74\
\x6d\x61\x70\x01\x00\x00\x00\x04\x44\x61\x74\x61\x07\x02\x02\x00\
\x00\x00\x09\x03\x00\x00\x00\x0f\x03\x00\x00\x00\x7d\x00\x00\x00\
\x02\x89\x50\x4e\x47\x0d\x0a\x1a\x0a\x00\x00\x00\x0d\x49\x48\x44\
\x52\x00\x00\x00\x20\x00\x00\x00\x20\x08\x06\x00\x00\x00\x73\x7a\
\x7a\xf4\x00\x00\x00\x09\x70\x48\x59\x73\x00\x00\x0e\xc4\x00\x00\
\x0e\xc4\x01\x95\x2b\x0e\x1b\x00\x00\x00\x2f\x49\x44\x41\x54\x58\
\x85\xed\xce\x21\x01\x00\x00\x08\x03\xb0\xc7\x79\xff\x3c\x74\x81\
\x18\x98\x89\xf9\x65\xda\xfd\x14\x01\x01\x01\x01\x01\x01\x01\x01\
\x01\x01\x01\x01\x01\x01\x81\xef\xc0\x01\x17\x12\xac\x79\xf7\xbb\
\xe8\x36\x00\x00\x00\x00\x49\x45\x4e\x44\xae\x42\x60\x82\x0b";

#[test]
fn export_wraps_icons_byte_for_byte_like_real_livesplit() {
    // The real capture's own PNG payload starts right after the fixed NRBF
    // prefix (161 bytes in) and ends one byte before the end (the trailing
    // byte is LiveSplit's MessageEnd record, not part of the PNG).
    let png_bytes = &LIVESPLIT_REAL_ICON_BYTES[161..LIVESPLIT_REAL_ICON_BYTES.len() - 1];
    assert_eq!(
        &png_bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "sanity check: this should be a PNG"
    );

    let dir = scratch_dir("real_icon");
    let icons_dir = dir.join("icons");
    std::fs::create_dir_all(&icons_dir).unwrap();
    std::fs::write(icons_dir.join("split0.png"), png_bytes).unwrap();

    let mut run = Run::new("Test Game", "Any%", &["A"]);
    run.splits[0].icon_path = Some("icons/split0.png".to_string());

    let lss_path = dir.join("test.lss");
    lss::export(&run, &lss_path, &dir).expect("export failed");

    let xml = std::fs::read_to_string(&lss_path).unwrap();
    let start = xml
        .find("<Icon>")
        .expect("should have written a non-empty Icon tag")
        + "<Icon>".len();
    let end = xml[start..].find("</Icon>").unwrap() + start;
    let exported_bytes = base64_decode(&xml[start..end]);

    assert_eq!(
        exported_bytes, LIVESPLIT_REAL_ICON_BYTES,
        "exported icon framing doesn't match what real LiveSplit wrote byte-for-byte"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_without_an_icon_path_writes_a_self_closing_tag() {
    let dir = scratch_dir("noicon");
    let run = Run::new("Test Game", "Any%", &["Intro"]);
    let lss_path = dir.join("test.lss");

    lss::export(&run, &lss_path, &dir).expect("export failed");
    let xml = std::fs::read_to_string(&lss_path).unwrap();
    assert!(xml.contains("<Icon />"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn import_rejects_a_non_run_root_element() {
    let dir = scratch_dir("badroot");
    let path = dir.join("bad.lss");
    std::fs::write(&path, "<NotARun></NotARun>").unwrap();

    let err = match lss::import(&path, &dir.join("icons")) {
        Ok(_) => panic!("expected import to fail on a non-<Run> root"),
        Err(e) => e,
    };
    assert!(err.contains("Run"), "unexpected error message: {err}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn import_rejects_a_run_with_no_segments() {
    let dir = scratch_dir("nosegments");
    let path = dir.join("empty.lss");
    std::fs::write(&path, "<Run version=\"1.7.0\"><GameName>G</GameName></Run>").unwrap();

    let err = match lss::import(&path, &dir.join("icons")) {
        Ok(_) => panic!("expected import to fail on a <Run> with no segments"),
        Err(e) => e,
    };
    assert!(err.contains("segments"), "unexpected error message: {err}");

    std::fs::remove_dir_all(&dir).ok();
}
