use chrono::Duration;
use openspeedrun::core::split::{
    AttemptHistoryEntry, COMPARISON_AVERAGE_SEGMENTS, COMPARISON_BEST_SEGMENTS,
    COMPARISON_MEDIAN_SEGMENTS, COMPARISON_PERSONAL_BEST, SegmentHistoryEntry, TimingMethod,
};
use openspeedrun::{Run, Split};

fn ms(n: i64) -> Duration {
    Duration::milliseconds(n)
}

#[test]
fn new_run_renames_trailing_empty_split_to_final_boss() {
    let run = Run::new("Game", "Any%", &["Intro", ""]);
    assert_eq!(run.splits.last().unwrap().name, "Final Boss");
}

#[test]
fn default_split_has_builtin_comparison_entries() {
    let split = Split::default();
    assert!(split.comparisons.contains_key(COMPARISON_PERSONAL_BEST));
    assert!(split.comparisons.contains_key(COMPARISON_BEST_SEGMENTS));
    assert_eq!(
        split.comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        None
    );
}

#[test]
fn average_and_median_segments_are_computed_from_history() {
    let split = Split {
        segment_history: vec![
            SegmentHistoryEntry {
                run_index: 0,
                real_time: Some(ms(1000)),
                game_time: None,
            },
            SegmentHistoryEntry {
                run_index: 1,
                real_time: Some(ms(2000)),
                game_time: None,
            },
            SegmentHistoryEntry {
                run_index: 2,
                real_time: Some(ms(3000)),
                game_time: None,
            },
        ],
        ..Default::default()
    };

    assert_eq!(
        split.comparison_time(COMPARISON_AVERAGE_SEGMENTS, TimingMethod::RealTime),
        Some(ms(2000))
    );
    assert_eq!(
        split.comparison_time(COMPARISON_MEDIAN_SEGMENTS, TimingMethod::RealTime),
        Some(ms(2000))
    );
}

#[test]
fn median_of_an_even_length_history_averages_the_middle_two() {
    let split = Split {
        segment_history: vec![
            SegmentHistoryEntry {
                run_index: 0,
                real_time: Some(ms(1000)),
                game_time: None,
            },
            SegmentHistoryEntry {
                run_index: 1,
                real_time: Some(ms(2000)),
                game_time: None,
            },
        ],
        ..Default::default()
    };

    assert_eq!(
        split.comparison_time(COMPARISON_MEDIAN_SEGMENTS, TimingMethod::RealTime),
        Some(ms(1500))
    );
}

#[test]
fn empty_segment_history_has_no_average_or_median() {
    let split = Split::default();
    assert_eq!(
        split.comparison_time(COMPARISON_AVERAGE_SEGMENTS, TimingMethod::RealTime),
        None
    );
    assert_eq!(
        split.comparison_time(COMPARISON_MEDIAN_SEGMENTS, TimingMethod::RealTime),
        None
    );
}

#[test]
fn recompute_best_segment_picks_the_minimum_and_drops_removed_records() {
    let mut split = Split {
        segment_history: vec![
            SegmentHistoryEntry {
                run_index: 0,
                real_time: Some(ms(5000)),
                game_time: None,
            },
            SegmentHistoryEntry {
                run_index: 1,
                real_time: Some(ms(3000)),
                game_time: None,
            },
        ],
        ..Default::default()
    };
    split.recompute_best_segment();
    assert_eq!(
        split.comparison_time(COMPARISON_BEST_SEGMENTS, TimingMethod::RealTime),
        Some(ms(3000))
    );

    // Simulate deleting the record-breaking attempt (e.g. an erroneous
    // segment) and recomputing.
    split.segment_history.retain(|e| e.run_index != 1);
    split.recompute_best_segment();
    assert_eq!(
        split.comparison_time(COMPARISON_BEST_SEGMENTS, TimingMethod::RealTime),
        Some(ms(5000))
    );
}

#[test]
fn comparison_total_sums_across_splits_or_is_none_if_any_split_is_missing_it() {
    let mut run = Run::new("Game", "Any%", &["A", "B"]);
    run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(1000));
    run.splits[1]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(2000));

    assert_eq!(
        run.comparison_total(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        Some(ms(3000))
    );

    // A brand-new comparison name isn't set on every split yet.
    run.splits[0]
        .comparisons
        .entry("Some Guy".to_string())
        .or_default()
        .real_time = Some(ms(500));
    assert_eq!(
        run.comparison_total("Some Guy", TimingMethod::RealTime),
        None
    );
}

#[test]
fn recompute_personal_best_picks_the_fastest_ended_attempt() {
    let mut run = Run::new("Game", "Any%", &["A", "B"]);

    run.attempt_history = vec![
        AttemptHistoryEntry {
            run_index: 0,
            real_time: Some(ms(9000)),
            game_time: None,
            ended: true,
            date: None,
        },
        AttemptHistoryEntry {
            run_index: 1,
            real_time: Some(ms(5000)),
            game_time: None,
            ended: true,
            date: None,
        },
        // A faster-looking but unfinished attempt must not be picked.
        AttemptHistoryEntry {
            run_index: 2,
            real_time: Some(ms(1000)),
            game_time: None,
            ended: false,
            date: None,
        },
    ];

    run.splits[0].segment_history = vec![
        SegmentHistoryEntry {
            run_index: 0,
            real_time: Some(ms(4000)),
            game_time: None,
        },
        SegmentHistoryEntry {
            run_index: 1,
            real_time: Some(ms(2000)),
            game_time: None,
        },
        SegmentHistoryEntry {
            run_index: 2,
            real_time: Some(ms(400)),
            game_time: None,
        },
    ];
    run.splits[1].segment_history = vec![
        SegmentHistoryEntry {
            run_index: 0,
            real_time: Some(ms(5000)),
            game_time: None,
        },
        SegmentHistoryEntry {
            run_index: 1,
            real_time: Some(ms(3000)),
            game_time: None,
        },
        SegmentHistoryEntry {
            run_index: 2,
            real_time: Some(ms(600)),
            game_time: None,
        },
    ];

    run.recompute_personal_best();

    // run_index 1 (5000+3000=8000) beats run_index 0 (4000+5000=9000), and
    // the unfinished run_index 2 is ineligible despite being fastest.
    assert_eq!(
        run.splits[0].comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        Some(ms(2000))
    );
    assert_eq!(
        run.splits[1].comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        Some(ms(3000))
    );
}

#[test]
fn recompute_personal_best_clears_pb_when_no_attempt_is_eligible() {
    let mut run = Run::new("Game", "Any%", &["A"]);
    run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(1234));

    // No attempt_history / segment_history at all.
    run.recompute_personal_best();

    assert_eq!(
        run.splits[0].comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        None
    );
}

#[test]
fn comparison_names_includes_builtins_and_custom_names_without_duplicates() {
    let mut run = Run::new("Game", "Any%", &["A"]);
    run.splits[0]
        .comparisons
        .entry("Some Guy".to_string())
        .or_default();

    let names = run.comparison_names();
    assert!(names.contains(&COMPARISON_PERSONAL_BEST.to_string()));
    assert!(names.contains(&COMPARISON_BEST_SEGMENTS.to_string()));
    assert!(names.contains(&"Some Guy".to_string()));
    assert_eq!(names.iter().filter(|n| n.as_str() == "Some Guy").count(), 1);
}

#[test]
fn run_save_and_load_round_trips_through_json() {
    let dir = std::env::temp_dir().join(format!("osr_split_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("split.json");

    let mut run = Run::new("Game", "Any%", &["A", "B"]);
    run.attempts = 3;
    run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(1500));

    run.save_to_file(path.to_str().unwrap()).unwrap();
    let loaded = Run::load_from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(loaded.title, run.title);
    assert_eq!(loaded.attempts, run.attempts);
    assert_eq!(
        loaded.splits[0].comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime),
        Some(ms(1500))
    );

    std::fs::remove_dir_all(&dir).ok();
}
