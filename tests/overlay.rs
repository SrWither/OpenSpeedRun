use chrono::Duration;
use openspeedrun::app::overlay::build_snapshot;
use openspeedrun::core::split::{COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, TimingMethod};
use openspeedrun::core::timer::TimerState;
use openspeedrun::{AppState, Run};

fn ms(n: i64) -> Duration {
    Duration::milliseconds(n)
}

#[test]
fn fresh_run_reports_not_started_and_nothing_reached() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.splits_display = app.run.splits.clone();
    app.current_split = 0;

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.timer_state, "not_started");
    assert_eq!(snapshot.current_time_ms, 0);
    assert_eq!(snapshot.current_split_index, 0);
    assert_eq!(snapshot.total_splits, 2);
    assert_eq!(snapshot.previous_segment_delta_ms, None);
    assert!(
        snapshot
            .splits
            .iter()
            .all(|s| s.cumulative_time_ms.is_none())
    );
    assert!(snapshot.splits[0].is_current);
}

#[test]
fn reached_split_reports_segment_time_and_delta_against_comparison() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(28_000));

    app.splits_display = app.run.splits.clone();
    app.splits_display[0].last_time = Some(ms(30_000));
    app.current_split = 1;
    app.timer.state = TimerState::Running;

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.splits[0].cumulative_time_ms, Some(30_000));
    assert_eq!(snapshot.splits[0].segment_time_ms, Some(30_000));
    assert_eq!(snapshot.splits[0].segment_comparison_ms, Some(28_000));
    assert_eq!(snapshot.splits[0].delta_ms, Some(2_000));
    assert!(!snapshot.splits[0].is_current);
    assert!(snapshot.splits[1].is_current);
    assert_eq!(snapshot.previous_segment_delta_ms, Some(2_000));
}

#[test]
fn second_split_segment_time_excludes_the_first_splits_cumulative_time() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.splits_display = app.run.splits.clone();
    app.splits_display[0].last_time = Some(ms(30_000));
    app.splits_display[1].last_time = Some(ms(50_000));
    app.current_split = 2;

    let snapshot = build_snapshot(&app);

    // 50s cumulative minus 30s at the previous split = 20s segment, not 50s.
    assert_eq!(snapshot.splits[1].segment_time_ms, Some(20_000));
}

#[test]
fn game_time_run_uses_igt_as_primary_and_exposes_rta_as_secondary() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A"]),
        ..AppState::empty_for_test()
    };
    app.run.timing_method = TimingMethod::GameTime;
    app.splits_display = app.run.splits.clone();

    app.igt_timer.state = TimerState::Running;
    app.igt_timer.elapsed = ms(12_000);
    app.timer.state = TimerState::Running;
    app.timer.elapsed = ms(15_000);

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.timing_method, "game_time");
    assert_eq!(snapshot.current_time_ms, 12_000);
    assert_eq!(snapshot.secondary_label, Some("RTA"));
    assert_eq!(snapshot.secondary_time_ms, Some(15_000));
}

#[test]
fn secondary_clock_is_absent_until_it_has_actually_been_used() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A"]),
        ..AppState::empty_for_test()
    };
    app.splits_display = app.run.splits.clone();
    app.timer.state = TimerState::Running;
    // igt_timer stays NotStarted (game-time mode never toggled on).

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.secondary_label, None);
    assert_eq!(snapshot.secondary_time_ms, None);
}

#[test]
fn best_possible_and_pb_totals_sum_across_splits() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    for (i, (best, pb)) in [(10_000, 12_000), (20_000, 25_000)].into_iter().enumerate() {
        let split = &mut app.run.splits[i];
        split
            .comparisons
            .get_mut(COMPARISON_BEST_SEGMENTS)
            .unwrap()
            .real_time = Some(ms(best));
        split
            .comparisons
            .get_mut(COMPARISON_PERSONAL_BEST)
            .unwrap()
            .real_time = Some(ms(pb));
    }
    app.splits_display = app.run.splits.clone();
    app.current_split = 0;

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.sum_of_best_ms, 30_000);
    assert_eq!(snapshot.best_possible_time_ms, 30_000);
    assert_eq!(snapshot.pb_time_ms, Some(37_000));
}

#[test]
fn pb_time_is_none_when_any_split_is_missing_a_personal_best() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(12_000));
    // Split "B" never got a Personal Best.
    app.splits_display = app.run.splits.clone();

    let snapshot = build_snapshot(&app);

    assert_eq!(snapshot.pb_time_ms, None);
}

#[test]
fn best_possible_time_accounts_for_progress_already_made() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.run.splits[1]
        .comparisons
        .get_mut(COMPARISON_BEST_SEGMENTS)
        .unwrap()
        .real_time = Some(ms(20_000));

    app.splits_display = app.run.splits.clone();
    app.splits_display[0].last_time = Some(ms(25_000)); // already ahead/behind, doesn't matter which
    app.current_split = 1;

    let snapshot = build_snapshot(&app);

    // Already-elapsed time (25s) plus the remaining split's best segment (20s).
    assert_eq!(snapshot.best_possible_time_ms, 45_000);
}

#[test]
fn snapshot_serializes_to_the_documented_json_field_names() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A"]),
        ..AppState::empty_for_test()
    };
    app.splits_display = app.run.splits.clone();

    let json = serde_json::to_value(build_snapshot(&app)).unwrap();
    for field in [
        "title",
        "category",
        "attempts",
        "timing_method",
        "selected_comparison",
        "timer_state",
        "current_time_ms",
        "secondary_label",
        "secondary_time_ms",
        "current_split_index",
        "total_splits",
        "sum_of_best_ms",
        "best_possible_time_ms",
        "pb_time_ms",
        "previous_segment_delta_ms",
        "splits",
    ] {
        assert!(
            json.get(field).is_some(),
            "missing field '{field}' in serialized snapshot"
        );
    }
}
