use chrono::{Duration, Utc};
use eframe::glow::Context;
use eframe::{egui, glow};
use egui::{FontDefinitions, TextureHandle};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use crate::config::layout::LayoutConfig;
use crate::config::load::{AppConfig, config_base_dir};
use crate::config::shaders::ShaderBackground;
#[cfg(unix)]
use crate::core::server::UICommand;
use crate::core::split::{
    AttemptHistoryEntry, COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, Run,
    SegmentHistoryEntry, Split, TimingMethod,
};
use crate::core::timer::{Timer, TimerState};
#[cfg(windows)]
use crate::core::winserver::UICommand;

pub struct AppState {
    pub timer: Timer,
    /// Independent game-time clock, paused/resumed by `toggle_igt_pause`
    /// (there's no autosplitter, so this is manual — same as LiveSplit's
    /// manual game-time mode).
    pub igt_timer: Timer,
    pub run: Run,
    pub layout: LayoutConfig,
    pub current_split: usize,
    pub textures: HashMap<String, TextureHandle>,
    pub split_base_path: std::path::PathBuf,
    pub current_page: usize,
    pub splits_per_page: usize,
    pub splits_display: Vec<Split>,
    pub splits_backup: Vec<Split>,
    pub show_help: bool,
    pub start_time: std::time::Instant,
    pub last_elapsed: f32,
    pub shader: Option<ShaderBackground>,
    pub gl: Option<Arc<Context>>,
    pub fonts_loaded: bool,
    pub transparent_set: bool,
    pub background_image: Option<TextureHandle>,
    pub background_image_name: Option<String>,
    pub background_gl_texture: Option<glow::NativeTexture>,
    pub loaded_fonts: Option<FontDefinitions>,
    /// Whether the most recently completed split beat its "Best Segments"
    /// comparison. Sticky until the next split (or a reset).
    pub last_segment_is_gold: bool,
    /// Whether the most recently *finished* run beat the existing Personal
    /// Best. Sticky until the next run finishes (or a reset).
    pub last_run_is_pb: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let app_config = AppConfig::load();
        println!("Using config: {:?}", app_config);
        let split_base_path = config_base_dir().join(&app_config.last_split_path);
        let run_path = split_base_path.join("split.json");

        let run = Run::load_from_file(run_path.to_str().unwrap()).unwrap_or_else(|_| {
            Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
        });

        let splits_per_page = run.splits_per_page.unwrap_or(5);
        let layout_path = config_base_dir().join(&app_config.theme);
        let layout = LayoutConfig::load_or_default(layout_path.to_str().unwrap());

        let splits = run.splits.clone();

        Self {
            timer: Timer::new(),
            igt_timer: Timer::new(),
            run,
            layout,
            current_split: 0,
            textures: HashMap::new(),
            split_base_path,
            current_page: 0,
            splits_per_page,
            splits_display: splits.clone(),
            splits_backup: splits,
            show_help: false,
            start_time: std::time::Instant::now(),
            last_elapsed: 0.0,
            shader: None,
            gl: None,
            fonts_loaded: false,
            transparent_set: false,
            background_image: None,
            background_image_name: None,
            background_gl_texture: None,
            loaded_fonts: None,
            last_segment_is_gold: false,
            last_run_is_pb: false,
        }
    }
}

impl AppState {
    /// Builds an `AppState` with empty/zero defaults for every field,
    /// touching neither disk nor `~/.config/openspeedrun` — unlike
    /// `AppState::default()`, which reads (and, on first run, *writes*) real
    /// files there. That makes `default()` unsafe to call from more than one
    /// test at a time: two tests racing "create the default split/theme"
    /// against the same shared config directory can catch each other
    /// mid-write via plain `std::fs::write`'s truncate-then-write (fixed
    /// now with `config::atomic_write`, but real concurrent first-run writes
    /// to the *same* file are still a wasted race, not a feature). Tests
    /// that only care about a handful of fields should build this and set
    /// those directly, rather than pull in `default()`'s I/O for fields they
    /// don't even look at.
    pub fn empty_for_test() -> Self {
        Self {
            timer: Timer::new(),
            igt_timer: Timer::new(),
            run: Run::default(),
            layout: LayoutConfig::default(),
            current_split: 0,
            textures: HashMap::new(),
            split_base_path: std::path::PathBuf::new(),
            current_page: 0,
            splits_per_page: 5,
            splits_display: Vec::new(),
            splits_backup: Vec::new(),
            show_help: false,
            start_time: std::time::Instant::now(),
            last_elapsed: 0.0,
            shader: None,
            gl: None,
            fonts_loaded: false,
            transparent_set: false,
            background_image: None,
            background_image_name: None,
            background_gl_texture: None,
            loaded_fonts: None,
            last_segment_is_gold: false,
            last_run_is_pb: false,
        }
    }
}

impl AppState {
    pub fn split(&mut self) {
        match self.timer.state {
            TimerState::NotStarted => self.start_run(),
            TimerState::Running => self.record_split(),
            _ => {}
        }
    }

    /// Starts (or resumes, if paused) both the RTA and IGT clocks together
    /// — the only place that should ever call `Timer::start_with_offset`,
    /// so the two clocks can't drift out of sync by one being started
    /// without the other. Safe to call from any timer state.
    pub fn start_timers(&mut self) {
        let offset = self.run.start_offset.unwrap_or(0);
        self.timer.start_with_offset(offset);
        self.igt_timer.start_with_offset(offset);
    }

    fn start_run(&mut self) {
        self.splits_backup = self.run.splits.clone();
        self.start_timers();
        self.current_split = 0;
    }

    /// Pauses both clocks together (the whole run is on hold, as opposed to
    /// `toggle_igt_pause`, which only stops the IGT clock for a load).
    pub fn pause_timers(&mut self) {
        self.timer.pause();
        self.igt_timer.pause();
    }

    /// Freezes both clocks for good — unlike `pause_timers`, this can't be
    /// undone by the "start" action, since a finished run shouldn't start
    /// ticking again. Only `reset_splits` can bring the timers back.
    fn end_run(&mut self) {
        self.timer.end();
        self.igt_timer.end();
    }

    fn record_split(&mut self) {
        // Belt-and-suspenders: the run already finished, and `end_run`
        // below should mean `self.timer.state` is `Ended` (not resumable)
        // by the time this could be called again. Kept as an explicit
        // guard anyway since `current_split` indexing below relies on it.
        if self.current_split >= self.splits_display.len() {
            return;
        }

        let now = self.timer.current_time();
        if now < Duration::zero() {
            return;
        }
        let now_game = self.igt_timer.current_time();

        if let Some(split) = self.splits_display.get_mut(self.current_split) {
            split.last_time = Some(now);
            split.last_time_game = Some(now_game);
        }

        self.current_split += 1;

        if self.current_split >= self.splits_display.len() {
            self.end_run();
        }

        self.update_page();
        self.save_history();
        self.update_comparisons();
    }

    /// Toggles the IGT clock only, representing "a load is happening" —
    /// there's no autosplitter, so this is driven by a hotkey.
    pub fn toggle_igt_pause(&mut self) {
        if self.timer.state != TimerState::Running {
            return;
        }
        if self.igt_timer.is_running() {
            self.igt_timer.pause();
        } else if self.igt_timer.is_paused() {
            self.igt_timer.start_with_offset(0);
        }
    }

    /// Switches to the next available comparison (Personal Best -> Best
    /// Segments -> Average Segments -> Median Segments -> any custom ones
    /// -> back to Personal Best), wrapping around. Reachable from the timer
    /// itself (click on the delta label, or a hotkey) instead of only from
    /// the config app's split editor.
    pub fn cycle_comparison(&mut self) {
        let names = self.run.comparison_names();
        let Some(current_index) = names
            .iter()
            .position(|n| n == &self.run.selected_comparison)
        else {
            return;
        };
        let next_index = (current_index + 1) % names.len();
        self.run.selected_comparison = names[next_index].clone();

        if let Err(e) = self.save() {
            eprintln!("Error saving selected comparison: {}", e);
        }
    }

    fn save_history(&mut self) {
        if self.current_split >= self.splits_display.len() {
            self.run.attempts += 1;

            let real_time = self.splits_display.last().and_then(|s| s.last_time);
            let game_time = self.splits_display.last().and_then(|s| s.last_time_game);

            self.run.attempt_history.push(AttemptHistoryEntry {
                run_index: self.run.attempts,
                real_time,
                game_time,
                ended: true,
                date: Some(Utc::now()),
            });

            let method = self.run.timing_method;
            let pb_total_time = self.run.comparison_total(COMPARISON_PERSONAL_BEST, method);

            let current_total = match method {
                TimingMethod::RealTime => real_time,
                TimingMethod::GameTime => game_time,
            };

            let is_new_pb = match (current_total, pb_total_time) {
                (Some(current), Some(existing)) => current < existing,
                (Some(_), None) => true,
                (None, _) => false,
            };

            if is_new_pb {
                self.run.pb_history.push(AttemptHistoryEntry {
                    run_index: self.run.attempts,
                    real_time,
                    game_time,
                    ended: true,
                    date: Some(Utc::now()),
                });
            }

            if let Err(e) = self.save() {
                eprintln!("Error saving run history: {}", e);
            }
        }
    }

    fn update_page(&mut self) {
        let next_page = self.current_split / self.splits_per_page;
        let max_page = (self.splits_display.len().saturating_sub(1)) / self.splits_per_page;
        self.current_page = next_page.min(max_page);
    }

    /// Records this attempt's segment times into history, updates the
    /// "Best Segments" comparison (always tracked, regardless of what's
    /// currently selected for display), and — on a finished run that beats
    /// the current "Personal Best" — updates that comparison too.
    fn update_comparisons(&mut self) {
        let method = self.run.timing_method;

        // Only the split that was *just* recorded gets a new history entry
        // — every earlier split in this attempt still has `last_time` set
        // too (it's only cleared once the whole run finishes), so looping
        // over all of them here would re-log each one on every subsequent
        // split.
        if let Some(current) = self
            .current_split
            .checked_sub(1)
            .and_then(|i| self.splits_display.get(i))
        {
            let i = self.current_split - 1;
            if let Some(current_time) = current.last_time {
                let prev_time = if i == 0 {
                    Duration::zero()
                } else {
                    self.splits_display[i - 1]
                        .last_time
                        .unwrap_or(Duration::zero())
                };
                let relative_real = current_time - prev_time;

                let relative_game = current.last_time_game.map(|current_game| {
                    let prev_game = if i == 0 {
                        Duration::zero()
                    } else {
                        self.splits_display[i - 1]
                            .last_time_game
                            .unwrap_or(Duration::zero())
                    };
                    current_game - prev_game
                });

                let target = &mut self.run.splits[i];

                target.segment_history.push(SegmentHistoryEntry {
                    run_index: self.run.attempts,
                    real_time: Some(relative_real),
                    game_time: relative_game,
                });

                let best = target
                    .comparisons
                    .entry(COMPARISON_BEST_SEGMENTS.to_string())
                    .or_default();
                let prev_best = best.get(method);

                self.last_segment_is_gold = match method {
                    TimingMethod::RealTime => prev_best.is_none_or(|gold| relative_real < gold),
                    TimingMethod::GameTime => {
                        relative_game.is_some_and(|rg| prev_best.is_none_or(|gold| rg < gold))
                    }
                };

                if best.real_time.is_none_or(|gold| relative_real < gold) {
                    best.real_time = Some(relative_real);
                }
                if let Some(relative_game) = relative_game
                    && best.game_time.is_none_or(|gold| relative_game < gold)
                {
                    best.game_time = Some(relative_game);
                }
            }
        }

        if self.current_split == self.run.splits.len() {
            let current_total = match method {
                TimingMethod::RealTime => self.splits_display.last().and_then(|s| s.last_time),
                TimingMethod::GameTime => self.splits_display.last().and_then(|s| s.last_time_game),
            };

            let existing_pb_total = self.run.comparison_total(COMPARISON_PERSONAL_BEST, method);

            let is_new_pb = match (current_total, existing_pb_total) {
                (Some(current), Some(existing)) => current < existing,
                (Some(_), None) => true,
                (None, _) => false,
            };
            self.last_run_is_pb = is_new_pb;

            if is_new_pb && self.run.auto_update_pb {
                for i in 0..self.splits_display.len() {
                    let current = &self.splits_display[i];
                    let Some(current_time) = current.last_time else {
                        continue;
                    };

                    let prev_time = if i == 0 {
                        Duration::zero()
                    } else {
                        self.splits_display[i - 1]
                            .last_time
                            .unwrap_or(Duration::zero())
                    };
                    let relative_real = current_time - prev_time;

                    let relative_game = current.last_time_game.map(|current_game| {
                        let prev_game = if i == 0 {
                            Duration::zero()
                        } else {
                            self.splits_display[i - 1]
                                .last_time_game
                                .unwrap_or(Duration::zero())
                        };
                        current_game - prev_game
                    });

                    let split = &mut self.run.splits[i];
                    let pb = split
                        .comparisons
                        .entry(COMPARISON_PERSONAL_BEST.to_string())
                        .or_default();
                    pb.real_time = Some(relative_real);
                    pb.game_time = relative_game;
                }
            }

            for split in self.run.splits.iter_mut() {
                split.last_time = None;
                split.last_time_game = None;
            }
        }

        if let Err(e) = self.save_comparisons() {
            eprintln!("Error saving comparisons: {}", e);
        }
    }

    pub fn reset_splits(&mut self) {
        // `sync_splits` below reloads `splits_display` from disk wholesale,
        // so there's nothing to selectively preserve here — just clear the
        // in-progress attempt times.
        for snapshot in &mut self.splits_display {
            snapshot.last_time = None;
            snapshot.last_time_game = None;
        }

        self.current_split = 0;
        self.timer.reset();
        self.igt_timer.reset();
        self.last_segment_is_gold = false;
        self.last_run_is_pb = false;
        self.sync_splits();
    }

    pub fn undo_split(&mut self) {
        if self.current_split > 0 {
            self.current_split -= 1;
            self.last_segment_is_gold = false;
            self.last_run_is_pb = false;

            if let Some(backup) = self.splits_backup.get(self.current_split) {
                if let Some(display_split) = self.splits_display.get_mut(self.current_split) {
                    display_split.last_time = None;
                    display_split.last_time_game = None;
                    display_split.comparisons = backup.comparisons.clone();
                }

                if let Some(run_split) = self.run.splits.get_mut(self.current_split) {
                    run_split.last_time = None;
                    run_split.last_time_game = None;
                    run_split.comparisons = backup.comparisons.clone();
                }
            }

            self.update_page();

            let path = self.split_base_path.join("split.json");
            let mut saved_run = self.run.clone();

            for split in saved_run.splits.iter_mut() {
                split.last_time = None;
                split.last_time_game = None;
            }

            if let Err(e) = saved_run.save_to_file(path.to_str().unwrap()) {
                eprintln!("Error saving after undo split: {}", e);
            }
        }
    }

    fn sync_splits(&mut self) {
        let run =
            Run::load_from_file(self.split_base_path.join("split.json").to_str().unwrap()).unwrap();
        self.run = run.clone();
        self.splits_display = run.splits.clone();
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        let path = self.split_base_path.join("split.json");
        self.run.save_to_file(path.to_str().unwrap())
    }

    /// Merges this attempt's comparisons/segment history into the on-disk
    /// run (a read-modify-write against the file, since other fields there
    /// — attempt history, metadata, etc. — may have moved on independently).
    pub fn save_comparisons(&mut self) -> std::io::Result<()> {
        let path = self.split_base_path.join("split.json");
        let mut saved_run = Run::load_from_file(path.to_str().unwrap())?;

        for (saved_split, current_split) in saved_run.splits.iter_mut().zip(self.run.splits.iter())
        {
            saved_split.comparisons = current_split.comparisons.clone();
            saved_split.segment_history = current_split.segment_history.clone();
        }

        saved_run.save_to_file(path.to_str().unwrap())
    }

    pub fn undo_pb(&mut self) {
        self.run.splits = self.splits_backup.clone();
        self.save().unwrap_or_else(|e| {
            eprintln!("Error saving PB after undo: {}", e);
        });
        self.reset_splits();
    }

    /// Signed seconds ahead (negative) or behind (positive) `selected_comparison`,
    /// summed over every completed split plus the segment currently in
    /// progress — mirrors LiveSplit's live-updating delta. `0.0` wherever a
    /// split has no comparison time to measure against yet.
    pub fn live_delta(&self, elapsed_split_time: f32) -> f32 {
        let method = self.run.timing_method;
        let comparison = self.run.selected_comparison.as_str();
        let mut delta = Duration::zero();

        for i in 0..self.current_split.min(self.splits_display.len()) {
            let Some(actual_total) = self.splits_display[i].last_time_for(method) else {
                continue;
            };
            let prev_actual = if i == 0 {
                Duration::zero()
            } else {
                self.splits_display[i - 1]
                    .last_time_for(method)
                    .unwrap_or(Duration::zero())
            };

            if let Some(cmp_segment) = self.run.splits[i].comparison_time(comparison, method) {
                delta += (actual_total - prev_actual) - cmp_segment;
            }
        }

        if let Some(split) = self.run.splits.get(self.current_split)
            && let Some(cmp_segment) = split.comparison_time(comparison, method)
        {
            let live_segment = Duration::milliseconds((elapsed_split_time * 1000.0) as i64);
            delta += live_segment - cmp_segment;
        }

        delta.as_seconds_f32()
    }

    /// Sum of the "Best Segments" comparison across every split (the
    /// theoretical best possible time), or `0.0` if any split is missing one.
    pub fn best_possible_time(&self) -> f32 {
        self.run
            .comparison_total(COMPARISON_BEST_SEGMENTS, self.run.timing_method)
            .map(|d| d.as_seconds_f32())
            .unwrap_or(0.0)
    }

    /// Total Personal Best time, or `0.0` if no PB has been set yet.
    pub fn pb_time(&self) -> f32 {
        self.run
            .comparison_total(COMPARISON_PERSONAL_BEST, self.run.timing_method)
            .map(|d| d.as_seconds_f32())
            .unwrap_or(0.0)
    }

    pub fn reload_theme(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            let app_config = AppConfig::load();
            let layout_path = config_base_dir().join(&app_config.theme);
            self.layout = LayoutConfig::load_or_default(layout_path.to_str().unwrap());
            self.fonts_loaded = false;
        }
    }

    pub fn reload_run(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            let app_config = AppConfig::load();
            let split_base_path = config_base_dir().join(&app_config.last_split_path);
            self.split_base_path = split_base_path.clone();

            let run_path = split_base_path.join("split.json");
            self.run = Run::load_from_file(run_path.to_str().unwrap()).unwrap_or_else(|_| {
                Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
            });

            self.splits_display = self.run.splits.clone();
            self.splits_backup = self.run.splits.clone();
            self.current_split = 0;
            self.current_page = 0;
            self.splits_per_page = self.run.splits_per_page.unwrap_or(5);
            self.update_page();

            // Icon textures are cached by `icon_path` string (see
            // `get_or_load_texture`), and re-importing a `.lss` in the
            // config editor reuses the same "icons/imported_N.png" paths
            // for different underlying files — without clearing the cache
            // here, a reloaded run showing the same paths as the previous
            // one would keep displaying the old textures.
            self.textures.clear();
        }
    }

    pub fn reload_all(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            self.reload_theme();
            self.reload_run();
            self.sync_splits();
        }
    }

    pub fn format_duration(&self, dur: Duration, sign_mode: u8) -> String {
        let sign = match sign_mode {
            1 if dur < Duration::zero() => "-",
            2 => {
                if dur < Duration::zero() {
                    "-"
                } else {
                    "+"
                }
            }
            _ => "",
        };

        let dur_abs = dur.abs();
        let minutes = dur_abs.num_minutes();
        let seconds = dur_abs.num_seconds() % 60;
        let millis = dur_abs.num_milliseconds() % 1000;

        if dur_abs.num_hours() > 0 {
            format!(
                "{}{}:{:02}:{:02}.{:03}",
                sign,
                dur_abs.num_hours(),
                minutes % 60,
                seconds,
                millis
            )
        } else if minutes > 0 {
            format!("{}{:02}:{:02}.{:03}", sign, minutes, seconds, millis)
        } else {
            format!("{}{:02}.{:03}", sign, seconds, millis)
        }
    }
}

pub struct AppWrapper {
    pub app_state: Arc<Mutex<AppState>>,
    pub command_rx: Receiver<UICommand>,
}

impl AppWrapper {
    pub fn new(
        app_state: Arc<Mutex<AppState>>,
        command_rx: Receiver<UICommand>,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        // Asignar shader
        let gl = cc.gl.as_ref().unwrap().clone();
        let shader_path = config_base_dir()
            .join("shaders")
            .join(&app_state.lock().unwrap().layout.colors.shader_path)
            .to_string_lossy()
            .to_string();
        let vertex_shader_path = config_base_dir()
            .join("shaders")
            .join(format!(
                "{}.vert",
                app_state.lock().unwrap().layout.colors.shader_path
            ))
            .to_string_lossy()
            .to_string();

        {
            let mut state = app_state.lock().unwrap();
            state.gl = Some(gl.clone());
            state.shader = ShaderBackground::new(gl.clone(), shader_path, vertex_shader_path);
        }

        Self {
            app_state,
            command_rx,
        }
    }
}
