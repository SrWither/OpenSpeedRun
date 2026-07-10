//! Builds the JSON snapshot the overlay WebSocket server (see
//! `app::websocket_server`) streams to connected clients — typically an OBS
//! browser-source overlay. Kept pure (just reads `AppState`, no I/O) so it's
//! testable without a live socket; the same numbers this produces are what
//! `app::footer`/`app::splits_panel` already render, just as structured
//! data instead of `egui` widgets.

use serde::Serialize;

use crate::app::state::AppState;
use crate::core::split::{COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, TimingMethod};
use crate::core::timer::TimerState;

#[derive(Debug, Clone, Serialize)]
pub struct OverlaySplit {
    pub name: String,
    pub is_current: bool,
    /// Total time-from-start when this split was hit this attempt, or
    /// `None` if not reached yet.
    pub cumulative_time_ms: Option<i64>,
    /// This split's own segment duration (`cumulative_time_ms` minus the
    /// previous split's), or `None` if not reached yet.
    pub segment_time_ms: Option<i64>,
    /// The selected comparison's segment (not cumulative) time for this
    /// split, if set.
    pub segment_comparison_ms: Option<i64>,
    /// `segment_time_ms - segment_comparison_ms`; only present once both are.
    pub delta_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverlaySnapshot {
    pub title: String,
    pub category: String,
    pub attempts: u32,
    pub timing_method: &'static str,
    pub selected_comparison: String,
    pub timer_state: &'static str,
    /// The clock the run considers authoritative (RTA for a Real Time run,
    /// IGT for a Game Time one) — the same one shown as the big timer.
    pub current_time_ms: i64,
    /// The *other* clock (IGT if `timing_method` is real time, RTA if it's
    /// game time), only present once it's actually been used this attempt.
    pub secondary_label: Option<&'static str>,
    pub secondary_time_ms: Option<i64>,
    pub current_split_index: usize,
    pub total_splits: usize,
    pub sum_of_best_ms: i64,
    pub best_possible_time_ms: i64,
    pub pb_time_ms: Option<i64>,
    /// The just-finished split's delta against the selected comparison —
    /// what the footer's "Prev Segment" line shows. `None` before the first
    /// split of an attempt.
    pub previous_segment_delta_ms: Option<i64>,
    pub splits: Vec<OverlaySplit>,
}

fn ms(d: chrono::Duration) -> i64 {
    d.num_milliseconds()
}

pub fn build_snapshot(app: &AppState) -> OverlaySnapshot {
    let method = app.run.timing_method;
    let selected_comparison = app.run.selected_comparison.clone();

    let timer_state = match app.timer.state {
        TimerState::NotStarted => "not_started",
        TimerState::Running => "running",
        TimerState::Paused => "paused",
        TimerState::Ended => "ended",
    };

    let (current_time_ms, secondary_label, secondary_time_ms) = match method {
        TimingMethod::RealTime => (
            ms(app.timer.current_time()),
            (app.igt_timer.state != TimerState::NotStarted).then_some("IGT"),
            (app.igt_timer.state != TimerState::NotStarted)
                .then(|| ms(app.igt_timer.current_time())),
        ),
        TimingMethod::GameTime => (
            ms(app.igt_timer.current_time()),
            (app.timer.state != TimerState::NotStarted).then_some("RTA"),
            (app.timer.state != TimerState::NotStarted).then(|| ms(app.timer.current_time())),
        ),
    };

    // Always a best-effort partial sum, same as the footer's "Sum of Best" —
    // unlike Best Possible/PB below, a gap here doesn't make the total
    // meaningless, so this never needs to be `None`.
    let sum_of_best_ms: i64 = ms(app
        .run
        .splits
        .iter()
        .filter_map(|s| s.comparison_time(COMPARISON_BEST_SEGMENTS, method))
        .sum());

    let elapsed_so_far = if app.current_split > 0 {
        app.splits_display
            .get(app.current_split - 1)
            .and_then(|s| s.last_time_for(method))
            .unwrap_or_else(chrono::Duration::zero)
    } else {
        chrono::Duration::zero()
    };
    let remaining_best: chrono::Duration = app
        .run
        .splits
        .iter()
        .skip(app.current_split)
        .filter_map(|s| s.comparison_time(COMPARISON_BEST_SEGMENTS, method))
        .sum();
    let best_possible_time_ms = ms(elapsed_so_far + remaining_best);

    let pb_time_ms = app
        .run
        .splits
        .iter()
        .map(|s| s.comparison_time(COMPARISON_PERSONAL_BEST, method))
        .collect::<Option<Vec<_>>>()
        .map(|times| {
            ms(times
                .into_iter()
                .fold(chrono::Duration::zero(), |a, b| a + b))
        });

    let previous_segment_delta_ms = if app.current_split > 0 {
        let previous_split_relative = if app.current_split == 1 {
            app.splits_display
                .first()
                .and_then(|s| s.last_time_for(method))
                .unwrap_or_else(chrono::Duration::zero)
        } else {
            let current = app
                .splits_display
                .get(app.current_split - 1)
                .and_then(|s| s.last_time_for(method))
                .unwrap_or_else(chrono::Duration::zero);
            let previous = app
                .splits_display
                .get(app.current_split - 2)
                .and_then(|s| s.last_time_for(method))
                .unwrap_or_else(chrono::Duration::zero);
            current - previous
        };
        let previous_segment_comparison = app
            .splits_display
            .get(app.current_split - 1)
            .and_then(|s| s.comparison_time(&selected_comparison, method))
            .unwrap_or_else(chrono::Duration::zero);
        Some(ms(previous_split_relative - previous_segment_comparison))
    } else {
        None
    };

    let mut previous_cumulative = chrono::Duration::zero();
    let splits = app
        .splits_display
        .iter()
        .enumerate()
        .map(|(i, split)| {
            let cumulative = split.last_time_for(method);
            let segment_time = cumulative.map(|c| c - previous_cumulative);
            if let Some(c) = cumulative {
                previous_cumulative = c;
            }

            let segment_comparison = app
                .run
                .splits
                .get(i)
                .and_then(|s| s.comparison_time(&selected_comparison, method));

            let delta_ms = match (segment_time, segment_comparison) {
                (Some(t), Some(c)) => Some(ms(t - c)),
                _ => None,
            };

            OverlaySplit {
                name: split.name.clone(),
                is_current: i == app.current_split,
                cumulative_time_ms: cumulative.map(ms),
                segment_time_ms: segment_time.map(ms),
                segment_comparison_ms: segment_comparison.map(ms),
                delta_ms,
            }
        })
        .collect();

    OverlaySnapshot {
        title: app.run.title.clone(),
        category: app.run.category.clone(),
        attempts: app.run.attempts,
        timing_method: match method {
            TimingMethod::RealTime => "real_time",
            TimingMethod::GameTime => "game_time",
        },
        selected_comparison,
        timer_state,
        current_time_ms,
        secondary_label,
        secondary_time_ms,
        current_split_index: app.current_split,
        total_splits: app.splits_display.len(),
        sum_of_best_ms,
        best_possible_time_ms,
        pb_time_ms,
        previous_segment_delta_ms,
        splits,
    }
}
