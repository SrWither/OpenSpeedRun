use crate::app::state::AppState;
use crate::config::layout::LayoutConfig;
use crate::core::split::COMPARISON_BEST_SEGMENTS;
use eframe::egui;
use egui::Color32;
use egui_plot::{HLine, Line, Plot, PlotPoints};

impl AppState {
    pub fn draw_graph(&mut self, ui: &mut egui::Ui, top: bool) {
        let LayoutConfig {
            colors, options, ..
        } = self.layout.clone();

        let elapsed_split_time = self.elapsed_split_time();
        let series = self.delta_series(elapsed_split_time);

        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };

        let gold_positive = Color32::from_hex(&colors.gold_positive).unwrap_or(Color32::GOLD);
        let gold_negative = Color32::from_hex(&colors.gold_negative).unwrap_or(Color32::RED);
        let pb_positive = Color32::from_hex(&colors.pb_positive).unwrap_or(Color32::GREEN);
        let pb_negative = Color32::from_hex(&colors.pb_negative).unwrap_or(Color32::RED);

        // "Best Segments" gets the gold color scheme, everything else (PB,
        // Average, Median, custom) gets the PB one — same split footer.rs
        // makes for the "Prev Segment" line.
        let is_gold_style = self.run.selected_comparison == COMPARISON_BEST_SEGMENTS;
        let (positive_color, negative_color) = if is_gold_style {
            (gold_positive, gold_negative)
        } else {
            (pb_positive, pb_negative)
        };

        let panel = if top {
            egui::Panel::top("graph")
        } else {
            egui::Panel::bottom("graph")
        };

        panel
            .resizable(false)
            .exact_size(76.0)
            .frame(egui::Frame {
                fill: bg_color,
                stroke: egui::Stroke::NONE,
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                Plot::new("delta_graph")
                    .height(72.0)
                    .show_axes(false)
                    .show_grid(false)
                    .show_background(false)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .allow_boxed_zoom(false)
                    .show(ui, |plot_ui| {
                        plot_ui.hline(
                            HLine::new("zero", 0.0)
                                .color(Color32::from_gray(100))
                                .width(1.0_f32),
                        );

                        // Negate so "ahead" (a negative delta) plots upward,
                        // matching LiveSplit's convention.
                        let points: Vec<[f64; 2]> = series
                            .iter()
                            .map(|(i, delta)| [*i as f64, -*delta as f64])
                            .collect();

                        // One line segment per pair of consecutive points,
                        // colored by whether that specific segment gained or
                        // lost time — mirrors LiveSplit's per-segment graph
                        // coloring rather than a single whole-line color.
                        for window in points.windows(2) {
                            let [from, to] = [window[0], window[1]];
                            let color = if to[1] >= from[1] {
                                positive_color
                            } else {
                                negative_color
                            };
                            plot_ui.line(
                                Line::new("delta", PlotPoints::new(vec![from, to]))
                                    .color(color)
                                    .width(2.0_f32),
                            );
                        }
                    });
            });
    }
}
