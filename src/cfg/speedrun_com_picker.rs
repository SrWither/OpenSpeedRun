use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::thread;

use eframe::egui;
use openspeedrun::core::split::Split;
use openspeedrun::speedrun_com::{self, Category, Game, Variable};
use openspeedrun::therun_gg;

use crate::style;

/// Tracks a blocking network call running on a background thread, so the UI
/// thread never blocks on it — every request in this picker used to call
/// into `ureq` directly from a button's `clicked()` handler, freezing the
/// whole app for as long as the request took.
enum AsyncOp<T> {
    Idle,
    Loading(Receiver<Result<T, String>>),
}

impl<T: Send + 'static> AsyncOp<T> {
    fn start(&mut self, work: impl FnOnce() -> Result<T, String> + Send + 'static) {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let _ = tx.send(work());
        });
        *self = AsyncOp::Loading(rx);
    }

    fn is_loading(&self) -> bool {
        matches!(self, AsyncOp::Loading(_))
    }

    /// Non-blocking poll — returns the result exactly once, the first time
    /// this is called after the background thread finishes.
    fn poll(&mut self) -> Option<Result<T, String>> {
        let result = match self {
            AsyncOp::Loading(rx) => match rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("Background request was lost".to_string()))
                }
            },
            AsyncOp::Idle => None,
        };
        if result.is_some() {
            *self = AsyncOp::Idle;
        }
        result
    }
}

/// What the user picked, handed back to the caller (`SplitEditor`) to apply
/// to its own `Run` — this struct doesn't touch `SplitEditor` directly, the
/// same decoupling `formats::lss::import` uses via `ImportResult`.
pub struct PickerResult {
    pub title: String,
    pub category: String,
    pub speedrun_com_game_id: String,
    pub speedrun_com_category_id: String,
    /// (variable name, chosen value label) pairs, ready to become
    /// `RunVariable`s.
    pub variables: Vec<(String, String)>,
    /// Real splits (names + comparisons + history) fetched from therun.gg's
    /// record holder for this category, if the user opted into that.
    pub splits: Option<Vec<Split>>,
}

/// A search → game → category → variables wizard backed by the public
/// speedrun.com API (plus an optional therun.gg step). Every network call
/// runs on a background thread via `AsyncOp`, polled once per frame in
/// `ui()` — the UI stays responsive (with a spinner) while a request is in
/// flight instead of freezing.
pub struct SpeedrunComPicker {
    pub open: bool,
    query: String,
    games: Vec<Game>,
    games_op: AsyncOp<Vec<Game>>,
    selected_game: Option<Game>,
    categories: Vec<Category>,
    categories_op: AsyncOp<Vec<Category>>,
    selected_category: Option<Category>,
    variables: Vec<Variable>,
    variables_op: AsyncOp<Vec<Variable>>,
    /// variable id -> chosen value label (or absent if skipped/not yet set)
    chosen_values: HashMap<String, String>,
    status: Option<String>,
    /// Categories therun.gg actually tracks for the selected game — loaded
    /// on demand so the user can see (and pick from) what's really there
    /// instead of us guessing a name match against speedrun.com.
    therun_categories: Vec<therun_gg::AvailableCategory>,
    therun_categories_op: AsyncOp<(String, Vec<therun_gg::AvailableCategory>)>,
    therun_categories_status: Option<String>,
    /// The therun.gg slug that actually resolved for the selected game
    /// (might differ from `game.abbreviation` — see `therun_gg::list_categories`).
    therun_game_slug: Option<String>,
    /// Real splits fetched from therun.gg for a chosen therun.gg category.
    fetched_splits: Option<Vec<Split>>,
    splits_op: AsyncOp<Vec<Split>>,
    splits_status: Option<String>,
}

impl Default for SpeedrunComPicker {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            games: Vec::new(),
            games_op: AsyncOp::Idle,
            selected_game: None,
            categories: Vec::new(),
            categories_op: AsyncOp::Idle,
            selected_category: None,
            variables: Vec::new(),
            variables_op: AsyncOp::Idle,
            chosen_values: HashMap::new(),
            status: None,
            therun_categories: Vec::new(),
            therun_categories_op: AsyncOp::Idle,
            therun_categories_status: None,
            therun_game_slug: None,
            fetched_splits: None,
            splits_op: AsyncOp::Idle,
            splits_status: None,
        }
    }
}

impl SpeedrunComPicker {
    fn reset_from_game(&mut self) {
        self.selected_game = None;
        self.categories.clear();
        self.selected_category = None;
        self.variables.clear();
        self.chosen_values.clear();
        self.reset_therun();
    }

    fn reset_from_category(&mut self) {
        self.selected_category = None;
        self.variables.clear();
        self.chosen_values.clear();
        self.reset_therun();
    }

    fn reset_therun(&mut self) {
        self.therun_categories.clear();
        self.therun_categories_status = None;
        self.therun_game_slug = None;
        self.fetched_splits = None;
        self.splits_status = None;
    }

    fn search(&mut self) {
        let query = self.query.clone();
        self.status = None;
        self.games_op
            .start(move || speedrun_com::search_games(&query));
    }

    fn pick_game(&mut self, game: Game) {
        self.reset_from_game();
        self.status = None;
        let game_id = game.id.clone();
        self.selected_game = Some(game);
        self.categories_op
            .start(move || speedrun_com::categories(&game_id));
    }

    fn pick_category(&mut self, category: Category) {
        self.reset_therun();
        self.status = None;
        let category_id = category.id.clone();
        self.selected_category = Some(category);
        self.variables.clear();
        self.variables_op
            .start(move || speedrun_com::variables(&category_id));
    }

    /// Loads what therun.gg actually tracks for this game, so the user can
    /// see and pick from real options instead of us guessing a match
    /// against speedrun.com's category name (the two sites don't always
    /// agree on naming or even track the same subcategories). therun.gg's
    /// slug for the game can also differ from speedrun.com's `abbreviation`
    /// — `list_categories` falls back to a search when the direct guess
    /// doesn't resolve, and hands back whichever slug actually worked.
    fn load_therun_categories(&mut self, game: &Game) {
        self.fetched_splits = None;
        self.splits_status = None;
        self.therun_categories_status = None;
        let abbreviation = game.abbreviation.clone();
        let name = game.name.clone();
        self.therun_categories_op
            .start(move || therun_gg::list_categories(&abbreviation, &name));
    }

    fn fetch_splits(&mut self, category_slug: &str) {
        let Some(game_slug) = self.therun_game_slug.clone() else {
            self.splits_status = Some("No therun.gg game resolved yet.".to_string());
            return;
        };
        let category_slug = category_slug.to_string();
        self.splits_status = None;
        self.splits_op
            .start(move || therun_gg::fetch_record_splits(&game_slug, &category_slug));
    }

    /// Applies results from any background request that finished since the
    /// last frame. Must run before rendering, since it can change what's
    /// selected/available.
    fn poll(&mut self) {
        if let Some(result) = self.games_op.poll() {
            match result {
                Ok(games) => {
                    self.games = games;
                    self.reset_from_game();
                    self.status = None;
                }
                Err(e) => self.status = Some(format!("Search failed: {e}")),
            }
        }

        if let Some(result) = self.categories_op.poll() {
            match result {
                Ok(categories) => {
                    self.categories = categories;
                    self.status = None;
                }
                Err(e) => self.status = Some(format!("Failed to load categories: {e}")),
            }
        }

        if let Some(result) = self.variables_op.poll() {
            match result {
                Ok(variables) => {
                    self.chosen_values.clear();
                    for v in &variables {
                        if let Some(default_id) = &v.default
                            && let Some(entry) = v.values.iter().find(|val| &val.id == default_id)
                        {
                            self.chosen_values.insert(v.id.clone(), entry.label.clone());
                        }
                    }
                    self.variables = variables;
                    self.status = None;
                }
                Err(e) => self.status = Some(format!("Failed to load variables: {e}")),
            }
        }

        if let Some(result) = self.therun_categories_op.poll() {
            match result {
                Ok((_, categories)) if categories.is_empty() => {
                    self.therun_categories.clear();
                    self.therun_game_slug = None;
                    self.therun_categories_status =
                        Some("therun.gg has no tracked categories for this game.".to_string());
                }
                Ok((resolved_slug, categories)) => {
                    self.therun_categories = categories;
                    self.therun_game_slug = Some(resolved_slug);
                    self.therun_categories_status = None;
                }
                Err(e) => {
                    self.therun_categories.clear();
                    self.therun_game_slug = None;
                    self.therun_categories_status =
                        Some(format!("Failed to load therun.gg categories: {e}"));
                }
            }
        }

        if let Some(result) = self.splits_op.poll() {
            match result {
                Ok(splits) => {
                    self.splits_status = Some(format!(
                        "Fetched {} real splits from therun.gg.",
                        splits.len()
                    ));
                    self.fetched_splits = Some(splits);
                }
                Err(e) => {
                    self.splits_status = Some(format!("Could not fetch real splits: {e}"));
                    self.fetched_splits = None;
                }
            }
        }
    }

    fn any_loading(&self) -> bool {
        self.games_op.is_loading()
            || self.categories_op.is_loading()
            || self.variables_op.is_loading()
            || self.therun_categories_op.is_loading()
            || self.splits_op.is_loading()
    }

    /// Draws the picker window (a no-op if `self.open` is `false`), and
    /// returns the user's choice once they click "Use this".
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<PickerResult> {
        if !self.open {
            return None;
        }

        self.poll();

        let mut result = None;
        let mut still_open = true;

        egui::Window::new("Fill from speedrun.com")
            .open(&mut still_open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                style::section_card(ui, "Search", egui_phosphor::regular::MAGNIFYING_GLASS, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Game:");
                        let response =
                            ui.add_enabled(!self.games_op.is_loading(), egui::TextEdit::singleline(&mut self.query));
                        let submitted =
                            response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui
                            .add_enabled(!self.games_op.is_loading(), egui::Button::new("Search"))
                            .clicked()
                            || submitted
                        {
                            self.search();
                        }
                        if self.games_op.is_loading() {
                            ui.spinner();
                        }
                    });
                    ui.label(
                        egui::RichText::new(
                            "Tip: speedrun.com's search is picky about punctuation — try the exact \
                             title (e.g. \"Super Mario Bros. 3\", with the period) if nothing shows up.",
                        )
                        .small()
                        .weak(),
                    );

                    if !self.games.is_empty() && self.selected_game.is_none() {
                        ui.add_space(style::SPACE_SM);
                        ui.label("Pick a game:");
                        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                            for game in self.games.clone() {
                                let label = format!("{} ({})", game.name, game.abbreviation);
                                if ui.selectable_label(false, label).clicked() {
                                    self.pick_game(game);
                                }
                            }
                        });
                    }
                });

                if let Some(game) = self.selected_game.clone() {
                    ui.add_space(style::SPACE_SM);
                    ui.horizontal(|ui| {
                        ui.label(format!("Game: {}", game.name));
                        if ui.small_button("change").clicked() {
                            self.reset_from_game();
                        }
                    });

                    if self.categories_op.is_loading() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading categories…");
                        });
                    } else if !self.categories.is_empty() && self.selected_category.is_none() {
                        ui.label("Pick a category:");
                        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                            for category in self.categories.clone() {
                                if ui.selectable_label(false, &category.name).clicked() {
                                    self.pick_category(category);
                                }
                            }
                        });
                    }

                    if let Some(category) = self.selected_category.clone() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Category: {}", category.name));
                            if ui.small_button("change").clicked() {
                                self.reset_from_category();
                            }
                        });

                        ui.add_space(style::SPACE_SM);
                        style::section_card(ui, "Real splits (therun.gg)", egui_phosphor::regular::GLOBE, |ui| {
                            ui.label(
                                egui::RichText::new(
                                    "Optional: therun.gg tracks real splits (names, PB, best segments, \
                                     history) uploaded by runners via LiveSplit. speedrun.com and \
                                     therun.gg don't always agree on category names, so pick from what's \
                                     actually there instead of guessing.",
                                )
                                .small()
                                .weak(),
                            );
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        !self.therun_categories_op.is_loading(),
                                        egui::Button::new("Check therun.gg for this game"),
                                    )
                                    .clicked()
                                {
                                    self.load_therun_categories(&game);
                                }
                                if self.therun_categories_op.is_loading() {
                                    ui.spinner();
                                }
                            });
                            if let Some(status) = &self.therun_categories_status {
                                ui.label(status);
                            }

                            if !self.therun_categories.is_empty() {
                                let mut clicked_slug = None;
                                ui.add_enabled_ui(!self.splits_op.is_loading(), |ui| {
                                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                                        for cat in &self.therun_categories {
                                            let label = format!(
                                                "{}  —  {} runner{} tracked",
                                                cat.display_name,
                                                cat.runner_count,
                                                if cat.runner_count == 1 { "" } else { "s" }
                                            );
                                            if ui.button(label).clicked() {
                                                clicked_slug = Some(cat.slug.clone());
                                            }
                                        }
                                    });
                                });
                                if let Some(slug) = clicked_slug {
                                    self.fetch_splits(&slug);
                                }
                            }

                            if self.splits_op.is_loading() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Fetching real splits from therun.gg…");
                                });
                            } else if let Some(splits_status) = &self.splits_status {
                                ui.label(splits_status);
                            }
                        });

                        if self.variables_op.is_loading() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading variables…");
                            });
                        } else if !self.variables.is_empty() {
                            ui.add_space(style::SPACE_SM);
                            style::section_card(ui, "Variables", egui_phosphor::regular::SLIDERS, |ui| {
                                for variable in &self.variables {
                                    ui.horizontal(|ui| {
                                        let label = if variable.mandatory {
                                            variable.name.clone()
                                        } else {
                                            format!("{} (optional)", variable.name)
                                        };
                                        ui.label(label);

                                        let current =
                                            self.chosen_values.get(&variable.id).cloned().unwrap_or_default();
                                        egui::ComboBox::from_id_salt(("srcom_var", &variable.id))
                                            .selected_text(if current.is_empty() { "(none)" } else { &current })
                                            .show_ui(ui, |ui| {
                                                if !variable.mandatory
                                                    && ui.selectable_label(current.is_empty(), "(none)").clicked()
                                                {
                                                    self.chosen_values.remove(&variable.id);
                                                }
                                                for value in &variable.values {
                                                    if ui
                                                        .selectable_label(current == value.label, &value.label)
                                                        .clicked()
                                                    {
                                                        self.chosen_values
                                                            .insert(variable.id.clone(), value.label.clone());
                                                    }
                                                }
                                            });
                                    });
                                }
                            });
                        }

                        ui.add_space(style::SPACE_MD);
                        let use_button = egui::Button::new(format!(
                            "{} Use this",
                            egui_phosphor::regular::CHECK_CIRCLE
                        ))
                        .fill(style::ACCENT_BG)
                        .stroke(egui::Stroke::new(1.0, style::ACCENT));
                        if ui.add(use_button).clicked() {
                            result = Some(PickerResult {
                                title: game.name.clone(),
                                category: category.name.clone(),
                                speedrun_com_game_id: game.id.clone(),
                                speedrun_com_category_id: category.id.clone(),
                                variables: self
                                    .variables
                                    .iter()
                                    .filter_map(|v| {
                                        self.chosen_values.get(&v.id).map(|val| (v.name.clone(), val.clone()))
                                    })
                                    .collect(),
                                splits: self.fetched_splits.clone(),
                            });
                            self.open = false;
                        }
                    }
                }

                if let Some(status) = &self.status {
                    ui.colored_label(style::ERROR, status);
                }
            });

        self.open &= still_open;

        // Keep repainting while something's in flight so the spinner
        // animates and the result gets picked up as soon as it arrives,
        // instead of waiting for the next unrelated input event.
        if self.any_loading() {
            ctx.request_repaint();
        }

        result
    }
}
