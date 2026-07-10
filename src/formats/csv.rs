//! CSV export of a `Run`'s attempt/segment history, for analysis in
//! whatever external tool the user prefers (a spreadsheet, pandas, R,
//! Grafana, ...). Deliberately minimal — this exposes the raw data rather
//! than computing metrics itself, the same "expose data, let other tools do
//! the specialized work" pattern as the control socket (for autosplitters)
//! and the overlay WebSocket server (for OBS).

use crate::core::split::Run;

/// One row per recorded attempt: when it happened, how long it took, and
/// whether it became the new Personal Best.
pub fn attempts_csv(run: &Run) -> String {
    let mut out = String::from("run_index,date,real_time_ms,game_time_ms,ended,is_pb\n");
    for attempt in &run.attempt_history {
        let is_pb = run
            .pb_history
            .iter()
            .any(|pb| pb.run_index == attempt.run_index);
        let date = attempt.date.map(|d| d.to_rfc3339()).unwrap_or_default();
        let real_time_ms = ms_field(attempt.real_time);
        let game_time_ms = ms_field(attempt.game_time);
        out.push_str(&format!(
            "{},{date},{real_time_ms},{game_time_ms},{},{is_pb}\n",
            attempt.run_index, attempt.ended,
        ));
    }
    out
}

/// One row per (attempt, split) pair: every segment time ever recorded for
/// every split, not just record-breaking ones — the same source data
/// `Split::segment_history` keeps for computing Average/Median Segments.
pub fn segments_csv(run: &Run) -> String {
    let mut out = String::from("run_index,split_index,split_name,real_time_ms,game_time_ms\n");
    for (split_index, split) in run.splits.iter().enumerate() {
        for entry in &split.segment_history {
            out.push_str(&format!(
                "{},{split_index},{},{},{}\n",
                entry.run_index,
                csv_escape(&split.name),
                ms_field(entry.real_time),
                ms_field(entry.game_time),
            ));
        }
    }
    out
}

fn ms_field(duration: Option<chrono::Duration>) -> String {
    duration
        .map(|d| d.num_milliseconds().to_string())
        .unwrap_or_default()
}

/// Quotes a field only if it needs it (contains a comma, quote, or newline),
/// doubling any internal quotes — split names are the only genuinely
/// free-text column here, everything else is a number/bool/RFC3339 date
/// that can never contain a delimiter.
fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
