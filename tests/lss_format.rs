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

    lss::export(&run, &lss_path).expect("export failed");
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
