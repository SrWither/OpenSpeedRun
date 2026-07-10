use chrono::Duration;
use openspeedrun::core::split::COMPARISON_PERSONAL_BEST;
use openspeedrun::{AppState, Run};

fn ms(n: i64) -> Duration {
    Duration::milliseconds(n)
}

#[test]
fn delta_series_reports_cumulative_delta_at_each_reached_split() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(10_000));
    app.run.splits[1]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(20_000));

    app.splits_display = app.run.splits.clone();
    // A took 12s (2s behind its 10s comparison segment).
    app.splits_display[0].last_time = Some(ms(12_000));
    // B's segment (29s - 12s = 17s) beats its 20s comparison by 3s, so the
    // running total flips from +2s behind to -1s (net ahead).
    app.splits_display[1].last_time = Some(ms(29_000));
    app.current_split = 2;

    let series = app.delta_series(0.0);

    assert_eq!(series, vec![(0, 2.0), (1, -1.0)]);
}

#[test]
fn live_delta_matches_the_last_point_of_delta_series() {
    let mut app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    app.run.splits[0]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(10_000));
    app.run.splits[1]
        .comparisons
        .get_mut(COMPARISON_PERSONAL_BEST)
        .unwrap()
        .real_time = Some(ms(20_000));

    app.splits_display = app.run.splits.clone();
    app.splits_display[0].last_time = Some(ms(12_000));
    app.current_split = 1;

    // 5s into the in-progress B segment, compared against its 20s
    // comparison segment.
    let elapsed_split_time = 5.0;

    let series = app.delta_series(elapsed_split_time);
    let live_delta = app.live_delta(elapsed_split_time);

    assert_eq!(series.last().copied(), Some((1, live_delta)));
    // +2s from A, then (5s - 20s) for the partial B segment in progress.
    assert_eq!(live_delta, -13.0);
}

#[test]
fn delta_series_is_empty_and_live_delta_is_zero_before_any_comparison_exists() {
    let app = AppState {
        run: Run::new("Game", "Any%", &["A", "B"]),
        ..AppState::empty_for_test()
    };
    // A fresh run has no Personal Best set yet, so there is nothing to
    // compare against — this must not panic, and the graph should simply
    // render nothing rather than a misleading all-zero line.
    assert_eq!(app.delta_series(0.0), Vec::new());
    assert_eq!(app.live_delta(0.0), 0.0);
}
