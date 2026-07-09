//! Minimal read-only client for the public speedrun.com API (v1), used to
//! prefill a new run's title/category/metadata from a real game+category on
//! the site. No authentication needed — every endpoint used here is public.
//!
//! Verified against the live API while planning this (not assumed from
//! memory): `GET /games?name=...`, `GET /games/:id/categories`,
//! `GET /categories/:id/variables` all confirmed working with real data.

use serde::Deserialize;

const BASE_URL: &str = "https://www.speedrun.com/api/v1";
const USER_AGENT: &str = "OpenSpeedRun/0.1 (+https://github.com)";

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub abbreviation: String,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub category_type: String,
}

#[derive(Debug, Clone)]
pub struct VariableValue {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub id: String,
    pub name: String,
    /// `None` means the variable applies globally (every category), not
    /// just the one it was fetched for.
    pub category: Option<String>,
    pub mandatory: bool,
    pub values: Vec<VariableValue>,
    pub default: Option<String>,
}

// --- Raw API response shapes (kept private; mapped to the simpler public
// structs above right after deserializing) ---

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct RawGame {
    id: String,
    names: RawGameNames,
    abbreviation: String,
}

#[derive(Deserialize)]
struct RawGameNames {
    international: String,
}

#[derive(Deserialize)]
struct RawCategory {
    id: String,
    name: String,
    #[serde(rename = "type")]
    category_type: String,
}

#[derive(Deserialize)]
struct RawVariable {
    id: String,
    name: String,
    category: Option<String>,
    mandatory: bool,
    values: RawVariableValues,
}

#[derive(Deserialize)]
struct RawVariableValues {
    values: std::collections::BTreeMap<String, RawVariableValueEntry>,
    default: Option<String>,
}

#[derive(Deserialize)]
struct RawVariableValueEntry {
    label: String,
}

fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Request to {url} failed: {e}"))?
        .body_mut()
        .read_json::<T>()
        .map_err(|e| format!("Failed to parse response from {url}: {e}"))
}

pub fn search_games(query: &str) -> Result<Vec<Game>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let url = format!(
        "{BASE_URL}/games?name={}&max=20",
        urlencode(query.trim())
    );
    let envelope: Envelope<Vec<RawGame>> = get_json(&url)?;
    Ok(envelope
        .data
        .into_iter()
        .map(|g| Game {
            id: g.id,
            name: g.names.international,
            abbreviation: g.abbreviation,
        })
        .collect())
}

pub fn categories(game_id: &str) -> Result<Vec<Category>, String> {
    let url = format!("{BASE_URL}/games/{}/categories", urlencode(game_id));
    let envelope: Envelope<Vec<RawCategory>> = get_json(&url)?;
    Ok(envelope
        .data
        .into_iter()
        .map(|c| Category {
            id: c.id,
            name: c.name,
            category_type: c.category_type,
        })
        .collect())
}

pub fn variables(category_id: &str) -> Result<Vec<Variable>, String> {
    let url = format!("{BASE_URL}/categories/{}/variables", urlencode(category_id));
    let envelope: Envelope<Vec<RawVariable>> = get_json(&url)?;
    Ok(envelope
        .data
        .into_iter()
        .map(|v| Variable {
            id: v.id,
            name: v.name,
            category: v.category,
            mandatory: v.mandatory,
            default: v.values.default.clone(),
            values: v
                .values
                .values
                .into_iter()
                .map(|(id, entry)| VariableValue { id, label: entry.label })
                .collect(),
        })
        .collect())
}

/// Minimal query-string escaping (avoids pulling in a whole `url` crate for
/// one function — the values here are always simple search terms/ids).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
