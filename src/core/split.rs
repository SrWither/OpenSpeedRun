use chrono::{Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Split {
    pub name: String,
    #[serde(with = "crate::core::split::duration_millis")]
    pub pb_time: Option<Duration>,
    #[serde(with = "crate::core::split::duration_millis")]
    pub last_time: Option<Duration>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub title: String,
    pub category: String,
    pub splits: Vec<Split>,
    #[serde(default)]
    pub start_offset: Option<i64>,
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
            splits,
            start_offset: None,
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
