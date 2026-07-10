use chrono::{DateTime, Duration, Utc};
use openspeedrun::Run;
use openspeedrun::core::split::{AttemptHistoryEntry, SegmentHistoryEntry};
use openspeedrun::formats::csv;

fn ms(n: i64) -> Duration {
    Duration::milliseconds(n)
}

fn fixed_date() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

#[test]
fn attempts_csv_has_a_header_and_one_row_per_attempt() {
    let mut run = Run::new("Test Game", "Any%", &["A"]);
    run.attempt_history = vec![
        AttemptHistoryEntry {
            run_index: 0,
            real_time: Some(ms(125_340)),
            game_time: None,
            ended: true,
            date: Some(fixed_date()),
        },
        AttemptHistoryEntry {
            run_index: 1,
            real_time: Some(ms(9_000)),
            game_time: None,
            ended: false,
            date: None,
        },
    ];
    run.pb_history = vec![AttemptHistoryEntry {
        run_index: 0,
        real_time: Some(ms(125_340)),
        game_time: None,
        ended: true,
        date: Some(fixed_date()),
    }];

    let csv = csv::attempts_csv(&run);
    let mut lines = csv.lines();

    assert_eq!(
        lines.next(),
        Some("run_index,date,real_time_ms,game_time_ms,ended,is_pb")
    );
    assert_eq!(
        lines.next(),
        Some("0,2023-11-14T22:13:20+00:00,125340,,true,true")
    );
    assert_eq!(lines.next(), Some("1,,9000,,false,false"));
    assert_eq!(lines.next(), None);
}

#[test]
fn attempts_csv_of_a_run_with_no_history_is_just_the_header() {
    let run = Run::new("Test Game", "Any%", &["A"]);
    assert_eq!(
        csv::attempts_csv(&run),
        "run_index,date,real_time_ms,game_time_ms,ended,is_pb\n"
    );
}

#[test]
fn segments_csv_has_one_row_per_attempt_and_split_pair() {
    let mut run = Run::new("Test Game", "Any%", &["World 1-1", "Boss"]);
    run.splits[0].segment_history = vec![
        SegmentHistoryEntry {
            run_index: 0,
            real_time: Some(ms(30_120)),
            game_time: None,
        },
        SegmentHistoryEntry {
            run_index: 1,
            real_time: Some(ms(29_800)),
            game_time: None,
        },
    ];
    run.splits[1].segment_history = vec![SegmentHistoryEntry {
        run_index: 0,
        real_time: Some(ms(60_000)),
        game_time: Some(ms(58_500)),
    }];

    let csv = csv::segments_csv(&run);
    let mut lines = csv.lines();

    assert_eq!(
        lines.next(),
        Some("run_index,split_index,split_name,real_time_ms,game_time_ms")
    );
    assert_eq!(lines.next(), Some("0,0,World 1-1,30120,"));
    assert_eq!(lines.next(), Some("1,0,World 1-1,29800,"));
    assert_eq!(lines.next(), Some("0,1,Boss,60000,58500"));
    assert_eq!(lines.next(), None);
}

#[test]
fn segments_csv_quotes_a_split_name_containing_a_comma() {
    let mut run = Run::new("Test Game", "Any%", &["Boss 1, Phase 2"]);
    run.splits[0].segment_history = vec![SegmentHistoryEntry {
        run_index: 0,
        real_time: Some(ms(1_000)),
        game_time: None,
    }];

    let csv = csv::segments_csv(&run);
    assert!(csv.contains("0,0,\"Boss 1, Phase 2\",1000,"));
}

#[test]
fn segments_csv_of_a_run_with_no_history_is_just_the_header() {
    let run = Run::new("Test Game", "Any%", &["A", "B"]);
    assert_eq!(
        csv::segments_csv(&run),
        "run_index,split_index,split_name,real_time_ms,game_time_ms\n"
    );
}
