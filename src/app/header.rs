use crate::{app::state::AppState, config::layout::LayoutConfig};
use eframe::egui::{self, Color32, RichText};
use egui::{Sense, ViewportCommand};

impl AppState {
    pub fn draw_header(&self, ctx: &egui::Context) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
            #[cfg(windows)]
                hotkeys: _,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK);
        let title_color = Color32::from_hex(&colors.title).unwrap_or(Color32::WHITE);
        let category_color = Color32::from_hex(&colors.category).unwrap_or(Color32::WHITE);
        let timer_color = Color32::from_hex(&colors.timer).unwrap_or(Color32::RED);

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::default().fill(bg_color))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                if !options.titlebar {
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

                            let icon_response =
                                ui.add(icon_label).interact(Sense::click_and_drag());

                            if icon_response.hovered()
                                && ui.input(|i| i.pointer.primary_down())
                                && ctx.dragged_id().is_none()
                            {
                                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                        });
                    });
                }

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

                    let elapsed = self.timer.current_time();
                    let time_str = self.format_duration(elapsed, 1);

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(time_str)
                            .size(font_sizes.timer)
                            .color(timer_color)
                            .strong(),
                    );
                    ui.add_space(10.0);
                });
            });
    }
}
