use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever the on-disk shape of `Run`/`Split` changes in a way that
/// needs a migration. Files written before this field existed deserialize
/// with `format_version == 0` (see the `#[serde(default)]` override below).
pub const CURRENT_FORMAT_VERSION: u32 = 1;

pub const COMPARISON_PERSONAL_BEST: &str = "Personal Best";
pub const COMPARISON_BEST_SEGMENTS: &str = "Best Segments";
pub const COMPARISON_AVERAGE_SEGMENTS: &str = "Average Segments";
pub const COMPARISON_MEDIAN_SEGMENTS: &str = "Median Segments";

/// Comparisons every split always has an entry for. `"Average"`/`"Median"`
/// are computed on the fly from `segment_history` instead of being stored.
pub const BUILTIN_COMPARISONS: &[&str] = &[
    COMPARISON_PERSONAL_BEST,
    COMPARISON_BEST_SEGMENTS,
    COMPARISON_AVERAGE_SEGMENTS,
    COMPARISON_MEDIAN_SEGMENTS,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimingMethod {
    #[default]
    RealTime,
    GameTime,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ComparisonTime {
    #[serde(with = "crate::core::split::duration_millis")]
    pub real_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub game_time: Option<Duration>,
}

impl ComparisonTime {
    pub fn get(&self, method: TimingMethod) -> Option<Duration> {
        match method {
            TimingMethod::RealTime => self.real_time,
            TimingMethod::GameTime => self.game_time,
        }
    }

    pub fn set(&mut self, method: TimingMethod, value: Option<Duration>) {
        match method {
            TimingMethod::RealTime => self.real_time = value,
            TimingMethod::GameTime => self.game_time = value,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunVariable {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RunMetadata {
    pub platform: Option<String>,
    pub region: Option<String>,
    pub variables: Vec<RunVariable>,
    pub speedrun_com_game_id: Option<String>,
    pub speedrun_com_category_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Split {
    pub name: String,
    #[serde(with = "crate::core::split::duration_millis")]
    pub last_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub last_time_game: Option<Duration>,
    pub icon_path: Option<String>,
    /// Always has `"Personal Best"` and `"Best Segments"` entries; may also
    /// hold custom-named comparisons (e.g. imported from a `.lss` file).
    pub comparisons: BTreeMap<String, ComparisonTime>,
    /// Segment time for every attempt that reached this split (not just
    /// record-breaking ones) — the source data for "Average"/"Median".
    pub segment_history: Vec<SegmentHistoryEntry>,
}

impl Split {
    /// This attempt's time at this split so far, for whichever clock is
    /// authoritative (`Run::timing_method`).
    pub fn last_time_for(&self, method: TimingMethod) -> Option<Duration> {
        match method {
            TimingMethod::RealTime => self.last_time,
            TimingMethod::GameTime => self.last_time_game,
        }
    }

    /// Looks up a comparison by name. `"Average Segments"`/`"Median
    /// Segments"` are computed from `segment_history`; anything else is a
    /// direct lookup in `comparisons` (built-in or custom).
    pub fn comparison_time(&self, name: &str, method: TimingMethod) -> Option<Duration> {
        match name {
            COMPARISON_AVERAGE_SEGMENTS => Self::segment_stat(&self.segment_history, method, false),
            COMPARISON_MEDIAN_SEGMENTS => Self::segment_stat(&self.segment_history, method, true),
            _ => self.comparisons.get(name).and_then(|c| c.get(method)),
        }
    }

    /// Recomputes the "Best Segments" comparison from whatever remains in
    /// `segment_history` — used after deleting an erroneous entry so a
    /// removed record doesn't linger in `comparisons`.
    pub fn recompute_best_segment(&mut self) {
        let best_real = self
            .segment_history
            .iter()
            .filter_map(|e| e.real_time)
            .min();
        let best_game = self
            .segment_history
            .iter()
            .filter_map(|e| e.game_time)
            .min();

        let best = self
            .comparisons
            .entry(COMPARISON_BEST_SEGMENTS.to_string())
            .or_default();
        best.real_time = best_real;
        best.game_time = best_game;
    }

    fn segment_stat(
        history: &[SegmentHistoryEntry],
        method: TimingMethod,
        median: bool,
    ) -> Option<Duration> {
        let mut millis: Vec<i64> = history
            .iter()
            .filter_map(|e| e.get(method))
            .map(|d| d.num_milliseconds())
            .collect();

        if millis.is_empty() {
            return None;
        }

        let result = if median {
            millis.sort_unstable();
            let mid = millis.len() / 2;
            if millis.len().is_multiple_of(2) {
                (millis[mid - 1] + millis[mid]) / 2
            } else {
                millis[mid]
            }
        } else {
            millis.iter().sum::<i64>() / millis.len() as i64
        };

        Some(Duration::milliseconds(result))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    /// Missing on every file written before this field existed, which
    /// deserializes to `0` here (field-level default overrides the
    /// struct-level one, which would otherwise pull `CURRENT_FORMAT_VERSION`
    /// from `Run::default()`). `Run::load_from_file` migrates on read.
    #[serde(default)]
    pub format_version: u32,
    pub title: String,
    pub category: String,
    pub attempts: u32,
    pub splits: Vec<Split>,
    #[serde(default)]
    pub start_offset: Option<i64>,
    pub splits_per_page: Option<usize>,
    pub auto_update_pb: bool,
    pub timing_method: TimingMethod,
    pub selected_comparison: String,
    pub attempt_history: Vec<AttemptHistoryEntry>,
    pub pb_history: Vec<AttemptHistoryEntry>,
    pub metadata: RunMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SegmentHistoryEntry {
    pub run_index: u32,
    #[serde(with = "crate::core::split::duration_millis")]
    pub real_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub game_time: Option<Duration>,
}

impl SegmentHistoryEntry {
    pub fn get(&self, method: TimingMethod) -> Option<Duration> {
        match method {
            TimingMethod::RealTime => self.real_time,
            TimingMethod::GameTime => self.game_time,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptHistoryEntry {
    pub run_index: u32,
    #[serde(with = "crate::core::split::duration_millis")]
    pub real_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub game_time: Option<Duration>,
    pub ended: bool,
    pub date: Option<DateTime<Utc>>,
}

impl Run {
    pub fn new(title: &str, category: &str, names: &[&str]) -> Self {
        let mut splits: Vec<Split> = names
            .iter()
            .map(|name| Split {
                name: name.to_string(),
                ..Split::default()
            })
            .collect();

        if let Some(last) = splits.last()
            && last.name.trim().is_empty()
        {
            splits.last_mut().unwrap().name = "Final Boss".to_string();
        }

        Self {
            format_version: CURRENT_FORMAT_VERSION,
            title: title.to_string(),
            category: category.to_string(),
            attempts: 0,
            splits,
            start_offset: None,
            splits_per_page: Some(5),
            auto_update_pb: true,
            timing_method: TimingMethod::RealTime,
            selected_comparison: COMPARISON_PERSONAL_BEST.to_string(),
            attempt_history: Vec::new(),
            pb_history: Vec::new(),
            metadata: RunMetadata::default(),
        }
    }

    /// Sum of a comparison's segment times across every split, or `None` if
    /// any split is missing that comparison (e.g. no Personal Best set yet).
    pub fn comparison_total(&self, name: &str, method: TimingMethod) -> Option<Duration> {
        self.splits
            .iter()
            .map(|s| s.comparison_time(name, method))
            .collect::<Option<Vec<_>>>()
            .map(|times| times.into_iter().fold(Duration::zero(), |a, b| a + b))
    }

    /// Recomputes the "Personal Best" comparison on every split from
    /// `attempt_history` + each split's `segment_history` — used after
    /// deleting an erroneous attempt so a removed PB doesn't linger in
    /// `comparisons`. Only attempts marked `ended` are eligible, and only
    /// if every split has a recorded segment for that attempt.
    pub fn recompute_personal_best(&mut self) {
        let method = self.timing_method;

        let best_run_index = self
            .attempt_history
            .iter()
            .filter(|a| a.ended)
            .filter_map(|a| {
                let mut total = Duration::zero();
                for split in &self.splits {
                    let segment = split
                        .segment_history
                        .iter()
                        .find(|e| e.run_index == a.run_index)
                        .and_then(|e| e.get(method))?;
                    total += segment;
                }
                Some((a.run_index, total))
            })
            .min_by_key(|(_, total)| *total)
            .map(|(run_index, _)| run_index);

        for split in &mut self.splits {
            let pb = split
                .comparisons
                .entry(COMPARISON_PERSONAL_BEST.to_string())
                .or_default();

            match best_run_index
                .and_then(|idx| split.segment_history.iter().find(|e| e.run_index == idx))
            {
                Some(entry) => {
                    pb.real_time = entry.real_time;
                    pb.game_time = entry.game_time;
                }
                None => *pb = ComparisonTime::default(),
            }
        }
    }

    /// Every comparison name currently in use: the built-ins plus any
    /// custom ones present on at least one split.
    pub fn comparison_names(&self) -> Vec<String> {
        let mut names: Vec<String> = BUILTIN_COMPARISONS.iter().map(|s| s.to_string()).collect();
        for split in &self.splits {
            for key in split.comparisons.keys() {
                if !names.contains(key) {
                    names.push(key.clone());
                }
            }
        }
        names
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        let mut run: Self = serde_json::from_str(&file).expect("Invalid JSON");

        if run.format_version < CURRENT_FORMAT_VERSION {
            if let Ok(legacy_run) = serde_json::from_str::<legacy::LegacyRun>(&file) {
                run.migrate_from_legacy(&legacy_run);
            }
            run.format_version = CURRENT_FORMAT_VERSION;

            // Keep the pre-migration file around in case something about
            // the conversion was wrong.
            let backup_path = format!("{path}.bak-v0");
            if !std::path::Path::new(&backup_path).exists()
                && let Err(e) = std::fs::copy(path, &backup_path)
            {
                eprintln!("⚠ Could not back up pre-migration file '{path}': {e}");
            }
        }

        Ok(run)
    }

    fn migrate_from_legacy(&mut self, legacy: &legacy::LegacyRun) {
        for (split, legacy_split) in self.splits.iter_mut().zip(legacy.splits.iter()) {
            let mut comparisons = BTreeMap::new();
            comparisons.insert(
                COMPARISON_PERSONAL_BEST.to_string(),
                ComparisonTime {
                    real_time: legacy_split.pb_time,
                    game_time: None,
                },
            );
            comparisons.insert(
                COMPARISON_BEST_SEGMENTS.to_string(),
                ComparisonTime {
                    real_time: legacy_split.gold_time,
                    game_time: None,
                },
            );
            split.comparisons = comparisons;

            // gold_history/pb_history could both hold an entry for the same
            // run_index; the segment time was the same value either way, so
            // just dedupe by run_index.
            let mut by_run_index: BTreeMap<u32, Duration> = BTreeMap::new();
            for entry in legacy_split
                .gold_history
                .iter()
                .chain(legacy_split.pb_history.iter())
            {
                if let Some(time) = entry.time {
                    by_run_index.insert(entry.run_index, time);
                }
            }

            split.segment_history = by_run_index
                .into_iter()
                .map(|(run_index, real_time)| SegmentHistoryEntry {
                    run_index,
                    real_time: Some(real_time),
                    game_time: None,
                })
                .collect();
        }

        self.selected_comparison = if legacy.gold_split {
            COMPARISON_BEST_SEGMENTS.to_string()
        } else {
            COMPARISON_PERSONAL_BEST.to_string()
        };

        let convert = |entries: &[legacy::LegacyAttemptHistoryEntry]| -> Vec<AttemptHistoryEntry> {
            entries
                .iter()
                .map(|e| AttemptHistoryEntry {
                    run_index: e.run_index,
                    real_time: e.total_time,
                    // No historical IGT data ever existed (the old
                    // `ingame_time` field was just a copy of total_time).
                    game_time: None,
                    ended: e.ended,
                    date: e.date,
                })
                .collect()
        };

        self.attempt_history = convert(&legacy.attempt_history);
        self.pb_history = convert(&legacy.pb_history);
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        crate::config::atomic_write(std::path::Path::new(path), &json)?;
        Ok(())
    }
}

impl Default for Run {
    fn default() -> Self {
        Self::new("New Run", "Category", &["Split 1", "Split 2"])
    }
}

impl Default for Split {
    fn default() -> Self {
        let mut comparisons = BTreeMap::new();
        comparisons.insert(
            COMPARISON_PERSONAL_BEST.to_string(),
            ComparisonTime::default(),
        );
        comparisons.insert(
            COMPARISON_BEST_SEGMENTS.to_string(),
            ComparisonTime::default(),
        );

        Self {
            name: "New Split".to_string(),
            last_time: None,
            last_time_game: None,
            icon_path: None,
            comparisons,
            segment_history: Vec::new(),
        }
    }
}

/// Structs mirroring the on-disk shape of `Run`/`Split` before
/// `CURRENT_FORMAT_VERSION` existed (single PB/gold fields, no timing
/// method, no metadata). Used only by `Run::migrate_from_legacy` to recover
/// data that would otherwise be silently dropped by the new field names.
mod legacy {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(default)]
    #[derive(Default)]
    pub struct LegacySplit {
        #[serde(with = "crate::core::split::duration_millis")]
        pub pb_time: Option<Duration>,
        #[serde(with = "crate::core::split::duration_millis")]
        pub gold_time: Option<Duration>,
        pub gold_history: Vec<LegacySegmentHistoryEntry>,
        pub pb_history: Vec<LegacySegmentHistoryEntry>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(default)]
    pub struct LegacySegmentHistoryEntry {
        pub run_index: u32,
        #[serde(with = "crate::core::split::duration_millis")]
        pub time: Option<Duration>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(default)]
    pub struct LegacyAttemptHistoryEntry {
        pub run_index: u32,
        #[serde(with = "crate::core::split::duration_millis")]
        pub total_time: Option<Duration>,
        pub ended: bool,
        pub date: Option<DateTime<Utc>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(default)]
    pub struct LegacyRun {
        pub splits: Vec<LegacySplit>,
        pub gold_split: bool,
        pub attempt_history: Vec<LegacyAttemptHistoryEntry>,
        pub pb_history: Vec<LegacyAttemptHistoryEntry>,
    }

    impl Default for LegacyRun {
        fn default() -> Self {
            Self {
                splits: Vec::new(),
                gold_split: true,
                attempt_history: Vec::new(),
                pb_history: Vec::new(),
            }
        }
    }
}

pub mod duration_millis {
    use chrono::Duration;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(dur: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match dur {
            Some(d) => serializer.serialize_i64(d.num_milliseconds()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<i64>::deserialize(deserializer)?;
        Ok(opt.map(Duration::milliseconds))
    }
}
