//! Fetches real split names/comparisons/history for a game+category from
//! therun.gg (an active community site with a LiveSplit auto-upload
//! component) — used as a splits.io replacement since splits.io shut down
//! permanently. Every endpoint here was verified against real live data
//! while planning this (Hollow Knight / Super Mario 64), not assumed from
//! any docs.
//!
//! Chain: `/api/games/<slug>` (find the record holder for a category) ->
//! `/api/users/<username>` (find their upload for that exact category) ->
//! the CloudFront-hosted `history.json` for that run (public, no auth) ->
//! `Vec<Split>`.
//!
//! Known limitation: `total.bestAchievedTime` per segment (used for our
//! "Personal Best" comparison) is therun.gg's best-cumulative-ever at that
//! checkpoint, not guaranteed to come from one single consistent fastest
//! run — the best approximation available from this data, not a promise of
//! exact fidelity to a single PB run (same spirit as the icon-extraction
//! and attempt-count caveats already documented in `formats::lss`).

use serde::Deserialize;

use crate::core::split::{
    COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, ComparisonTime, SegmentHistoryEntry, Split,
};

const API_BASE: &str = "https://therun.gg/api";
const HISTORY_BASE: &str = "https://d1qsrp2avfthuv.cloudfront.net";
const USER_AGENT: &str = "OpenSpeedRun/0.1 (+https://github.com)";

/// Numeric fields in therun.gg's history JSON are inconsistently plain
/// JSON numbers in some places (`"values": [83019, ...]`) and numeric
/// strings in others (`"bestAchievedTime": "79728.012"`) — accept either.
#[derive(Deserialize)]
#[serde(untagged)]
enum FlexNum {
    Num(f64),
    Text(String),
}

impl FlexNum {
    fn as_ms(&self) -> Option<f64> {
        match self {
            FlexNum::Num(n) => Some(*n),
            FlexNum::Text(s) => s.parse::<f64>().ok(),
        }
    }
}

fn ms_to_duration(ms: f64) -> Option<chrono::Duration> {
    if !ms.is_finite() {
        return None;
    }
    Some(chrono::Duration::milliseconds(ms.round() as i64))
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

// --- /api/games/<slug> ---

#[derive(Deserialize)]
struct GameResponse {
    stats: GameStats,
}

#[derive(Deserialize)]
struct GameStats {
    #[serde(rename = "categoryLeaderboards")]
    category_leaderboards: Vec<CategoryLeaderboard>,
}

#[derive(Deserialize)]
struct CategoryLeaderboard {
    #[serde(rename = "categoryNameDisplay")]
    category_name_display: String,
    #[serde(rename = "categoryName")]
    category_name: String,
    #[serde(rename = "pbLeaderboard")]
    pb_leaderboard: Vec<LeaderboardEntry>,
}

#[derive(Deserialize)]
struct LeaderboardEntry {
    username: String,
}

// --- /api/users/<username> ---

#[derive(Deserialize)]
struct UserRun {
    #[serde(rename = "originalRun")]
    original_run: Option<String>,
    #[serde(rename = "historyFilename")]
    history_filename: Option<String>,
    #[serde(rename = "attemptCount", default)]
    attempt_count: u32,
    /// Display name of the game this run belongs to — only needed by
    /// `resolve_game_slug`, to pick the right entry out of a user's full
    /// run list (a search hit can be for a user who also plays other games).
    #[serde(default)]
    game: Option<String>,
}

// --- /api/search?q=... ---

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    runs: Vec<SearchRun>,
}

#[derive(Deserialize)]
struct SearchRun {
    user: String,
    game: String,
}

// --- history.json ---

#[derive(Deserialize)]
struct HistoryFile {
    splits: Vec<HistorySplit>,
}

#[derive(Deserialize)]
struct HistorySplit {
    name: String,
    single: HistoryStat,
    total: HistoryStat,
    values: Vec<Option<FlexNum>>,
}

#[derive(Deserialize)]
struct HistoryStat {
    #[serde(rename = "bestAchievedTime")]
    best_achieved_time: Option<FlexNum>,
}

/// A category as therun.gg tracks it for a game — surfaced to the user so
/// *they* pick which one to pull from, instead of us silently guessing a
/// match against speedrun.com's category name (the two sites don't always
/// agree on naming, and therun.gg simply doesn't track every subcategory
/// speedrun.com does).
pub struct AvailableCategory {
    pub display_name: String,
    /// Internal slug, needed by `fetch_record_splits` — not always equal to
    /// `display_name` lowercased (e.g. variable-suffixed categories).
    pub slug: String,
    /// How many runners have a recorded PB — shown to the user before they
    /// commit to fetching, so they know roughly what they'll get.
    pub runner_count: usize,
}

/// therun.gg's own slug for a game doesn't always match speedrun.com's
/// `abbreviation` — confirmed: speedrun.com calls the original "Super Mario
/// Bros." `smb1`, but therun.gg's real, richly-populated entry for it
/// (159 runners on Any%) lives at `supermariobros.` (with a trailing
/// period); the bare `smb1` guess 500s, and even a same-named-looking
/// `supermariobros` slug exists but is a near-empty duplicate. Falls back
/// to therun.gg's own `/api/search` (used by their site's search box) to
/// find a real user with a run for this exact game display name, then
/// reads the correct slug off their `originalRun`.
fn resolve_game_slug(game_display_name: &str) -> Result<String, String> {
    let search: SearchResponse = get_json(&format!(
        "{API_BASE}/search?q={}",
        urlencode(game_display_name)
    ))?;

    let matching_user = search
        .runs
        .iter()
        .find(|r| r.game.eq_ignore_ascii_case(game_display_name))
        .ok_or_else(|| format!("therun.gg has no data for \"{game_display_name}\""))?;

    let user_runs: Vec<UserRun> = get_json(&format!("{API_BASE}/users/{}", matching_user.user))?;

    user_runs
        .iter()
        .find(|r| {
            r.game
                .as_deref()
                .is_some_and(|g| g.eq_ignore_ascii_case(game_display_name))
        })
        .and_then(|r| r.original_run.as_deref())
        .and_then(|orig| orig.split('#').next())
        .map(str::to_string)
        .ok_or_else(|| {
            format!("therun.gg's data for \"{game_display_name}\" has no usable identifier")
        })
}

/// Tries `game_slug_guess` (normally speedrun.com's `abbreviation`) first,
/// falling back to `resolve_game_slug` if that doesn't resolve. Returns the
/// slug that actually worked, so the caller can reuse it directly for
/// `fetch_record_splits` without re-resolving.
fn fetch_game(
    game_slug_guess: &str,
    game_display_name: &str,
) -> Result<(GameResponse, String), String> {
    match get_json::<GameResponse>(&format!("{API_BASE}/games/{game_slug_guess}")) {
        Ok(game) => Ok((game, game_slug_guess.to_string())),
        Err(guess_error) => {
            let resolved_slug = resolve_game_slug(game_display_name).map_err(|resolve_error| {
                format!(
                    "Guessed slug \"{game_slug_guess}\" failed ({guess_error}); {resolve_error}"
                )
            })?;
            let game: GameResponse = get_json(&format!("{API_BASE}/games/{resolved_slug}"))?;
            Ok((game, resolved_slug))
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Lists the categories therun.gg actually has data for, for the game with
/// `game_display_name` (using `game_slug_guess` — normally speedrun.com's
/// `abbreviation` — as a first guess before falling back to search). Empty
/// (not an error) if the game exists there but has no tracked categories.
/// Returns the slug that actually resolved, for reuse with
/// `fetch_record_splits`.
pub fn list_categories(
    game_slug_guess: &str,
    game_display_name: &str,
) -> Result<(String, Vec<AvailableCategory>), String> {
    let (game, resolved_slug) = fetch_game(game_slug_guess, game_display_name)?;

    let categories = game
        .stats
        .category_leaderboards
        .into_iter()
        .map(|c| AvailableCategory {
            display_name: c.category_name_display,
            slug: c.category_name,
            runner_count: c.pb_leaderboard.len(),
        })
        .collect();

    Ok((resolved_slug, categories))
}

/// Fetches the record holder's real splits for `category_slug` (from
/// `list_categories`) of the game identified by `game_slug` (also from
/// `list_categories`'s returned resolved slug — no fallback resolution
/// needed here since the caller already has the right one).
pub fn fetch_record_splits(game_slug: &str, category_slug: &str) -> Result<Vec<Split>, String> {
    let game: GameResponse = get_json(&format!("{API_BASE}/games/{game_slug}"))?;

    let category = game
        .stats
        .category_leaderboards
        .iter()
        .find(|c| c.category_name == category_slug)
        .ok_or_else(|| format!("Category \"{category_slug}\" no longer found on therun.gg"))?;

    if category.pb_leaderboard.is_empty() {
        return Err(format!(
            "No runs recorded for \"{category_slug}\" on therun.gg"
        ));
    }

    let original_run = format!("{game_slug}#{}", category.category_name);
    let mut last_error = String::new();
    let mut best: Option<Vec<Split>> = None;

    // `pbLeaderboard` order doesn't necessarily start at the actual #1 (an
    // entry can be present with `"placing": 2` at index 0 — some leaderboard
    // slots are apparently skipped), and a leaderboard entry's therun.gg
    // upload for this category can be a thin practice/test file rather than
    // their real category run. Try several entries and keep whichever
    // yields the richest data (most total segment history), instead of
    // stopping at the first one that merely doesn't error.
    let richness = |splits: &[Split]| -> (usize, usize) {
        (
            splits.len(),
            splits.iter().map(|s| s.segment_history.len()).sum(),
        )
    };

    for entry in category.pb_leaderboard.iter().take(5) {
        match try_fetch_from_user(&entry.username, &original_run) {
            Ok(splits) => {
                let candidate_score = richness(&splits);
                let current_best_score = best.as_deref().map(richness).unwrap_or((0, 0));
                if candidate_score > current_best_score {
                    best = Some(splits);
                }
            }
            Err(e) => last_error = e,
        }
    }

    best.ok_or_else(|| format!("Could not fetch splits from any leaderboard entry: {last_error}"))
}

fn try_fetch_from_user(username: &str, original_run_prefix: &str) -> Result<Vec<Split>, String> {
    let runs: Vec<UserRun> = get_json(&format!("{API_BASE}/users/{username}"))?;

    // Categories with mandatory variables (e.g. Hollow Knight's "Any%
    // Glitch") never appear bare — therun.gg always suffixes `originalRun`
    // with `#variables:...` for them (confirmed: "hollowknight#any%" never
    // occurs by itself, only "hollowknight#any%#variables:any%glitch=...").
    // Match the bare form OR that prefix followed by a `#`, so a category
    // with a longer name that happens to share a prefix (e.g. "any%" vs
    // "any%onehanded") isn't matched by mistake.
    //
    // A user can have several uploads matching the same prefix (different
    // variable combos, test/practice files, etc). Prefer the one with the
    // most attempts — confirmed necessary: the *first* prefix match for a
    // real leaderboard-topping player turned out to be a near-empty
    // 1-split practice file, not their real category run.
    let suffixed_prefix = format!("{original_run_prefix}#");
    let run = runs
        .iter()
        .filter(|r| {
            r.original_run.as_deref().is_some_and(|actual| {
                actual == original_run_prefix || actual.starts_with(&suffixed_prefix)
            })
        })
        .max_by_key(|r| r.attempt_count)
        .ok_or_else(|| format!("{username} has no upload matching {original_run_prefix}"))?;

    let history_filename = run
        .history_filename
        .as_ref()
        .ok_or_else(|| format!("{username}'s run for {original_run_prefix} has no history file"))?;

    let history: HistoryFile = get_json(&format!("{HISTORY_BASE}/{history_filename}"))?;

    if history.splits.is_empty() {
        return Err(format!(
            "{username}'s history file for {original_run_prefix} has no splits"
        ));
    }

    // `total.bestAchievedTime` is cumulative time-from-start (same shape as
    // LiveSplit's `SplitTimes`, see `formats::lss`), so it needs the same
    // cumulative -> relative conversion to become our `"Personal Best"`
    // (a segment delta). `single.bestAchievedTime` is already a delta.
    let mut prev_cumulative_pb = chrono::Duration::zero();

    Ok(history
        .splits
        .into_iter()
        .map(|s| {
            let mut comparisons = std::collections::BTreeMap::new();

            let best_segment = s
                .single
                .best_achieved_time
                .as_ref()
                .and_then(FlexNum::as_ms)
                .and_then(ms_to_duration);
            comparisons.insert(
                COMPARISON_BEST_SEGMENTS.to_string(),
                ComparisonTime {
                    real_time: best_segment,
                    game_time: None,
                },
            );

            let cumulative_pb = s
                .total
                .best_achieved_time
                .as_ref()
                .and_then(FlexNum::as_ms)
                .and_then(ms_to_duration);
            let relative_pb = cumulative_pb.map(|cum| cum - prev_cumulative_pb);
            if let Some(cum) = cumulative_pb {
                prev_cumulative_pb = cum;
            }
            comparisons.insert(
                COMPARISON_PERSONAL_BEST.to_string(),
                ComparisonTime {
                    real_time: relative_pb,
                    game_time: None,
                },
            );

            let segment_history = s
                .values
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    let real_time = v
                        .as_ref()
                        .and_then(FlexNum::as_ms)
                        .and_then(ms_to_duration)?;
                    Some(SegmentHistoryEntry {
                        run_index: i as u32,
                        real_time: Some(real_time),
                        game_time: None,
                    })
                })
                .collect();

            Split {
                name: s.name,
                comparisons,
                segment_history,
                ..Split::default()
            }
        })
        .collect())
}
