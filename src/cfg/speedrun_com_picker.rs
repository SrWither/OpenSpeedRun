use std::collections::HashMap;

use eframe::egui;
use openspeedrun::speedrun_com::{self, Category, Game, Variable};

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
}

/// A search → game → category → variables wizard backed by the public
/// speedrun.com API. All requests are synchronous (same blocking pattern
/// `rfd::FileDialog` already uses elsewhere in this app), triggered only on
/// clicks, not every frame.
pub struct SpeedrunComPicker {
    pub open: bool,
    query: String,
    games: Vec<Game>,
    selected_game: Option<Game>,
    categories: Vec<Category>,
    selected_category: Option<Category>,
    variables: Vec<Variable>,
    /// variable id -> chosen value label (or absent if skipped/not yet set)
    chosen_values: HashMap<String, String>,
    status: Option<String>,
}

impl Default for SpeedrunComPicker {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            games: Vec::new(),
            selected_game: None,
            categories: Vec::new(),
            selected_category: None,
            variables: Vec::new(),
            chosen_values: HashMap::new(),
            status: None,
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
    }

    fn reset_from_category(&mut self) {
        self.selected_category = None;
        self.variables.clear();
        self.chosen_values.clear();
    }

    fn search(&mut self) {
        match speedrun_com::search_games(&self.query) {
            Ok(games) => {
                self.games = games;
                self.reset_from_game();
                self.status = None;
            }
            Err(e) => self.status = Some(format!("Search failed: {e}")),
        }
    }

    fn pick_game(&mut self, game: Game) {
        match speedrun_com::categories(&game.id) {
            Ok(categories) => {
                self.categories = categories;
                self.selected_category = None;
                self.variables.clear();
                self.chosen_values.clear();
                self.selected_game = Some(game);
                self.status = None;
            }
            Err(e) => self.status = Some(format!("Failed to load categories: {e}")),
        }
    }

    fn pick_category(&mut self, category: Category) {
        match speedrun_com::variables(&category.id) {
            Ok(variables) => {
                self.chosen_values.clear();
                for v in &variables {
                    if let Some(default_id) = &v.default {
                        if let Some(entry) = v.values.iter().find(|val| &val.id == default_id) {
                            self.chosen_values.insert(v.id.clone(), entry.label.clone());
                        }
                    }
                }
                self.variables = variables;
                self.selected_category = Some(category);
                self.status = None;
            }
            Err(e) => self.status = Some(format!("Failed to load variables: {e}")),
        }
    }

    /// Draws the picker window (a no-op if `self.open` is `false`), and
    /// returns the user's choice once they click "Use this".
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<PickerResult> {
        if !self.open {
            return None;
        }

        let mut result = None;
        let mut still_open = true;

        egui::Window::new("Fill from speedrun.com")
            .open(&mut still_open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Game:");
                    let response = ui.text_edit_singleline(&mut self.query);
                    let submitted =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Search").clicked() || submitted {
                        self.search();
                    }
                });

                if !self.games.is_empty() && self.selected_game.is_none() {
                    ui.separator();
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

                if let Some(game) = self.selected_game.clone() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Game: {}", game.name));
                        if ui.small_button("change").clicked() {
                            self.reset_from_game();
                        }
                    });

                    if !self.categories.is_empty() && self.selected_category.is_none() {
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

                        if !self.variables.is_empty() {
                            ui.separator();
                            ui.label("Variables:");
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
                        }

                        ui.separator();
                        if ui.button("Use this").clicked() {
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
                            });
                            self.open = false;
                        }
                    }
                }

                if let Some(status) = &self.status {
                    ui.colored_label(egui::Color32::RED, status);
                }
            });

        self.open &= still_open;

        result
    }
}
