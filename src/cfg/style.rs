use eframe::egui;
use egui::{Color32, RichText, Stroke};

// === Palette ===
// Blue/light-blue accent, dark neutral surfaces. Chosen deliberately to not
// clash with the timer's own user-configurable gold/PB colors, which live
// in a completely different part of the app.
pub const ACCENT: Color32 = Color32::from_rgb(74, 158, 255);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(110, 180, 255);
pub const ACCENT_BG: Color32 = Color32::from_rgb(24, 44, 66);

pub const BG_BASE: Color32 = Color32::from_rgb(17, 18, 21);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(27, 28, 33);
pub const BG_SUNKEN: Color32 = Color32::from_rgb(12, 13, 15);

pub const BORDER: Color32 = Color32::from_rgb(56, 59, 68);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 40, 47);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 231, 235);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(150, 152, 162);

pub const SUCCESS: Color32 = Color32::from_rgb(80, 200, 130);
pub const ERROR: Color32 = Color32::from_rgb(235, 95, 95);
pub const WARNING: Color32 = Color32::from_rgb(230, 180, 80);

// === Spacing ===
pub const SPACE_SM: f32 = 6.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 20.0;

/// A rounded, bordered card with an icon + title heading. Replaces the
/// ad-hoc `ui.group()` + manual heading label pattern used throughout the
/// cfg screens before this redesign.
pub fn section_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    icon: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(SPACE_MD)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(ACCENT).size(16.0));
                ui.label(RichText::new(title).strong().size(15.0));
            });
            ui.add_space(SPACE_SM);
            add_contents(ui)
        })
        .inner
}

/// A pill-shaped selectable item (theme/split pickers): icon + label, with
/// an accent border/fill and a trailing checkmark when selected.
pub fn selectable_chip(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
) -> egui::Response {
    let text = if selected {
        format!("{icon}  {label}   {}", egui_phosphor::regular::CHECK)
    } else {
        format!("{icon}  {label}")
    };

    let button = egui::Button::new(RichText::new(text).size(14.0))
        .corner_radius(18)
        .min_size(egui::vec2(0.0, 34.0))
        .fill(if selected { ACCENT_BG } else { BG_SUNKEN })
        .stroke(Stroke::new(
            if selected { 1.5 } else { 1.0 },
            if selected { ACCENT } else { BORDER },
        ));

    ui.add(button)
}

/// Applies the global visual theme once, at app startup.
pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    let visuals = &mut style.visuals;

    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT_BG;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.window_fill = BG_BASE;
    visuals.panel_fill = BG_BASE;
    visuals.extreme_bg_color = BG_SUNKEN;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.code_bg_color = BG_SUNKEN;

    visuals.window_corner_radius = 8.into();
    visuals.menu_corner_radius = 6.into();
    visuals.window_stroke = Stroke::new(1.0, BORDER);

    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = ERROR;

    // Sliders (and checkbox/radio boxes) fill their track with
    // `widgets.inactive.bg_fill`, painted directly on whatever's behind them
    // (usually a `section_card`, which is `BG_ELEVATED`) — so it needs to be
    // visibly darker than that, or the track disappears entirely.
    visuals.slider_trailing_fill = true;

    let w = &mut visuals.widgets;

    w.noninteractive.bg_fill = BG_BASE;
    w.noninteractive.weak_bg_fill = BG_ELEVATED;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    w.noninteractive.corner_radius = 6.into();

    w.inactive.bg_fill = BG_SUNKEN;
    w.inactive.weak_bg_fill = BG_ELEVATED;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    w.inactive.corner_radius = 6.into();

    w.hovered.bg_fill = Color32::from_rgb(38, 40, 48);
    w.hovered.weak_bg_fill = Color32::from_rgb(38, 40, 48);
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT_HOVER);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    w.hovered.corner_radius = 6.into();

    w.active.bg_fill = ACCENT_BG;
    w.active.weak_bg_fill = ACCENT_BG;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    w.active.corner_radius = 6.into();

    w.open.bg_fill = ACCENT_BG;
    w.open.weak_bg_fill = ACCENT_BG;
    w.open.bg_stroke = Stroke::new(1.0, ACCENT);
    w.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    w.open.corner_radius = 6.into();

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(SPACE_MD as i8);

    ctx.set_global_style(style);
}
