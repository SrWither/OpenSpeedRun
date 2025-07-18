use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Split {
    pub name: String,
    #[serde(with = "crate::core::split::duration_millis")]
    pub pb_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub last_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub gold_time: Option<Duration>,
    pub icon_path: Option<String>,
    pub gold_history: Vec<SegmentHistoryEntry>,
    pub pb_history: Vec<SegmentHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    pub title: String,
    pub category: String,
    pub attempts: u32,
    pub splits: Vec<Split>,
    #[serde(default)]
    pub start_offset: Option<i64>,
    pub splits_per_page: Option<usize>,
    pub auto_update_pb: bool,
    #[serde(default)]
    pub gold_split: bool,
    pub attempt_history: Vec<AttemptHistoryEntry>,
    pub pb_history: Vec<AttemptHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentHistoryEntry {
    pub run_index: u32,
    #[serde(with = "crate::core::split::duration_millis")]
    pub time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptHistoryEntry {
    pub run_index: u32,
    #[serde(with = "crate::core::split::duration_millis")]
    pub total_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub ingame_time: Option<Duration>,
    pub ended: bool,
    pub date: Option<DateTime<Utc>>,
}

impl Run {
    pub fn new(title: &str, category: &str, names: &[&str]) -> Self {
        let mut splits: Vec<Split> = names
            .iter()
            .map(|name| Split {
                name: name.to_string(),
                pb_time: None,
                last_time: None,
                icon_path: None,
                gold_time: None,
                gold_history: Vec::new(),
                pb_history: Vec::new(),
            })
            .collect();

        if let Some(last) = splits.last() {
            if last.name.trim().is_empty() {
                splits.last_mut().unwrap().name = "Final Boss".to_string();
            }
        }

        Self {
            title: title.to_string(),
            category: category.to_string(),
            attempts: 0,
            splits,
            start_offset: None,
            splits_per_page: Some(5),
            auto_update_pb: true,
            gold_split: true,
            attempt_history: Vec::new(),
            pb_history: Vec::new(),
        }
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        let run: Self = serde_json::from_str(&file).expect("Invalid JSON");
        Ok(run)
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, json)?;
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
        Self {
            name: "New Split".to_string(),
            pb_time: None,
            last_time: None,
            icon_path: None,
            gold_time: None,
            gold_history: Vec::new(),
            pb_history: Vec::new(),
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
