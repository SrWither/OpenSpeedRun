use crate::core::split::TimingMethod;
use crate::core::timer::TimerState;
use crate::{app::state::AppState, config::layout::LayoutConfig};
use eframe::egui::{self, Color32, RichText};
use egui::{Sense, ViewportCommand};

impl AppState {
    pub fn draw_drag_handle(&self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let options = self.layout.options.clone();

        if options.titlebar {
            return;
        }

        let colors = self.layout.colors.clone();
        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };

        egui::Panel::top("drag_handle")
            .frame(egui::Frame::default().fill(bg_color))
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.style_mut().interaction.selectable_labels = false;

                        ui.add_space(ui.available_width() - 24.0);

                        let icon_label = egui::Label::new(
                            RichText::new(egui_phosphor::regular::DOTS_SIX_VERTICAL)
                                .italics()
                                .size(14.0)
                                .color(egui::Color32::GRAY),
                        );

                        let icon_response = ui.add(icon_label).interact(Sense::click_and_drag());

                        if icon_response.hovered()
                            && ui.input(|i| i.pointer.primary_down())
                            && ctx.dragged_id().is_none()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                    });
                });
            });
    }

    pub fn draw_title(&self, ui: &mut egui::Ui, top: bool) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
            #[cfg(windows)]
                hotkeys: _,
        } = self.layout.clone();

        if !options.show_title && !options.show_category {
            return;
        }

        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };
        let title_color = Color32::from_hex(&colors.title).unwrap_or(Color32::WHITE);
        let category_color = Color32::from_hex(&colors.category).unwrap_or(Color32::WHITE);

        let frame = egui::Frame::default().fill(bg_color);
        let panel = if top {
            egui::Panel::top("title")
        } else {
            egui::Panel::bottom("title")
        };

        panel.frame(frame).show_inside(ui, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if options.show_title {
                    ui.label(
                        RichText::new(&self.run.title)
                            .color(title_color)
                            .size(font_sizes.title),
                    );
                }
                if options.show_category {
                    ui.label(
                        RichText::new(&self.run.category)
                            .color(category_color)
                            .size(font_sizes.category),
                    );
                }
            });
            ui.add_space(10.0);
        });
    }

    pub fn draw_timer(&self, ui: &mut egui::Ui, top: bool) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
            #[cfg(windows)]
                hotkeys: _,
        } = self.layout.clone();

        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };
        let timer_color = Color32::from_hex(&colors.timer).unwrap_or(Color32::RED);
        let category_color = Color32::from_hex(&colors.category).unwrap_or(Color32::WHITE);

        // The big timer follows whichever clock the run
        // considers authoritative — for a Game Time category,
        // showing RTA (which includes loads) as the prominent
        // number is backwards. The other clock, if it's been
        // used this run, shows small right underneath.
        let method = self.run.timing_method;
        let (primary_time, secondary_label, secondary_time, secondary_active) = match method {
            TimingMethod::RealTime => (
                self.timer.current_time(),
                "IGT",
                self.igt_timer.current_time(),
                self.igt_timer.state != TimerState::NotStarted,
            ),
            TimingMethod::GameTime => (
                self.igt_timer.current_time(),
                "RTA",
                self.timer.current_time(),
                self.timer.state != TimerState::NotStarted,
            ),
        };

        let frame = egui::Frame::default().fill(bg_color);
        let panel = if top {
            egui::Panel::top("timer")
        } else {
            egui::Panel::bottom("timer")
        };

        panel.frame(frame).show_inside(ui, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(self.format_duration(primary_time, 1))
                        .size(font_sizes.timer)
                        .color(timer_color)
                        .strong(),
                );

                if secondary_active {
                    let loading_suffix =
                        if method == TimingMethod::RealTime && self.igt_timer.is_paused() {
                            format!(" {}", egui_phosphor::regular::HOURGLASS)
                        } else {
                            String::new()
                        };
                    ui.label(
                        RichText::new(format!(
                            "{secondary_label}: {}{loading_suffix}",
                            self.format_duration(secondary_time, 1)
                        ))
                        .size(font_sizes.category)
                        .color(category_color),
                    );
                }
            });
            ui.add_space(10.0);
        });
    }
}
