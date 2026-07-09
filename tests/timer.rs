use chrono::Duration;
use openspeedrun::{Timer, TimerState};

#[test]
fn new_timer_is_not_started_at_zero() {
    let timer = Timer::new();
    assert_eq!(timer.state, TimerState::NotStarted);
    assert_eq!(timer.current_time(), Duration::zero());
}

#[test]
fn start_runs_and_advances_current_time() {
    let mut timer = Timer::new();
    timer.start();
    assert!(timer.is_running());

    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(timer.current_time() >= Duration::milliseconds(20));
}

#[test]
fn pause_freezes_elapsed_and_resume_continues_from_it() {
    let mut timer = Timer::new();
    timer.start();
    std::thread::sleep(std::time::Duration::from_millis(20));
    timer.pause();
    assert!(timer.is_paused());

    let frozen = timer.current_time();
    assert!(frozen >= Duration::milliseconds(20));

    // Time must not advance while paused.
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert_eq!(timer.current_time(), frozen);

    // start_with_offset() on a Paused timer resumes rather than restarting
    // from zero.
    timer.start_with_offset(0);
    assert!(timer.is_running());
    assert!(timer.current_time() >= frozen);
}

#[test]
fn end_freezes_permanently_and_blocks_restart() {
    let mut timer = Timer::new();
    timer.start();
    timer.end();
    assert!(timer.is_ended());

    let frozen = timer.current_time();
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!(timer.current_time(), frozen);

    // An ended run shouldn't start ticking again.
    timer.start_with_offset(0);
    assert!(timer.is_ended());
    assert_eq!(timer.current_time(), frozen);
}

#[test]
fn reset_clears_state_back_to_not_started() {
    let mut timer = Timer::new();
    timer.start();
    timer.pause();
    timer.reset();

    assert_eq!(timer.state, TimerState::NotStarted);
    assert_eq!(timer.current_time(), Duration::zero());
}

#[test]
fn start_with_offset_counts_down_before_the_run_officially_starts() {
    let mut timer = Timer::new();
    timer.start_with_offset(5);
    assert!(timer.is_running());
    // start_time is 5s in the future, so current_time should read as a
    // negative countdown rather than a positive elapsed time.
    assert!(timer.current_time() < Duration::zero());
    assert!(timer.current_time() > Duration::seconds(-6));
}
