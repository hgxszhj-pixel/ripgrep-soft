//! UI helper components for professional look

use eframe::egui;
use crate::gui::state::{AppTheme, FileCategory};

/// Display a professional search bar with icon and clear button
pub fn search_bar(
    ui: &mut egui::Ui,
    query: &mut String,
    hint_text: &str,
    icon: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        // Search icon
        ui.label(egui::RichText::new(icon).size(18.0));

        // Search input with frame
        let response = ui.add_sized(
            [ui.available_width(), 32.0],
            egui::TextEdit::singleline(query)
                .hint_text(hint_text)
                .frame(true)
                .desired_width(ui.available_width() - 40.0),
        );

        // Clear button (X) when text exists
        if !query.is_empty()
            && ui
                .add(egui::Button::new(egui::RichText::new("\u{2715}").small())
                    .frame(false))
                .clicked()
        {
            query.clear();
        }

        response
    })
    .inner
}

/// Display a theme toggle button row
pub fn theme_toggle(ui: &mut egui::Ui, current_theme: &mut AppTheme) {
    ui.horizontal(|ui| {
        let themes = [
            (AppTheme::Light, "\u{2600}", "Light"),
            (AppTheme::Dark, "\u{1F319}", "Dark"),
            (AppTheme::Blue, "\u{1F499}", "Blue"),
            (AppTheme::Green, "\u{1F49A}", "Green"),
            (AppTheme::Purple, "\u{1F49C}", "Purple"),
        ];

        for (theme, icon, _name) in themes {
            let is_active = *current_theme == theme;
            let btn = egui::Button::new(egui::RichText::new(icon).size(16.0))
                .frame(false)
                .fill(if is_active {
                    ui.style().visuals.selection.bg_fill
                } else {
                    egui::Color32::TRANSPARENT
                });

            if ui.add(btn).clicked() {
                *current_theme = theme;
            }
        }
    });
}

/// Display a chip/badge for search mode
pub fn mode_chip(ui: &mut egui::Ui, mode: &str, is_active: bool) {
    let color = if is_active {
        egui::Color32::from_rgb(0, 120, 212)
    } else {
        egui::Color32::GRAY
    };

    ui.label(
        egui::RichText::new(format!("[{mode}]"))
            .small()
            .color(egui::Color32::WHITE)
            .background_color(color),
    );
}

/// Display status indicator with icon, color, and text
pub fn status_indicator(
    ui: &mut egui::Ui,
    icon: &str,
    color: egui::Color32,
    text: &str,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(icon).size(14.0).color(color));
        ui.label(egui::RichText::new(text).small().color(color));
    });
}

/// Display file result row with icon, name, and path
pub fn file_result_row(
    ui: &mut egui::Ui,
    is_selected: bool,
    file_name: &str,
    file_path: &str,
    file_size: u64,
    alt_bg: bool,
) {
    let bg_color = if is_selected {
        egui::Color32::from_rgb(0, 120, 212).linear_multiply(0.15)
    } else if alt_bg {
        ui.style().visuals.faint_bg_color
    } else {
        egui::Color32::TRANSPARENT
    };

    let (rect, _response) = ui.allocate_at_least(
        egui::vec2(ui.available_width(), 36.0),
        egui::Sense::click(),
    );

    ui.painter().rect_filled(rect, 4.0, bg_color);

    // File icon based on extension
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let category = FileCategory::from_extension(ext);

    // Draw icon
    let icon_rect = egui::Rect::from_min_size(rect.left_center() - egui::vec2(0.0, 10.0), egui::vec2(20.0, 20.0));
    ui.painter().text(
        icon_rect.center(),
        egui::Align2::CENTER_CENTER,
        category.icon(),
        egui::FontId::proportional(16.0),
        ui.style().visuals.text_color(),
    );

    // File name
    let name_color = if is_selected {
        egui::Color32::from_rgb(0, 100, 200)
    } else {
        ui.style().visuals.text_color()
    };

    ui.allocate_at_least(egui::vec2(200.0, 20.0), egui::Sense::hover());
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(file_name)
                .color(name_color)
                .strong(),
        );
        ui.label(
            egui::RichText::new(format_size(file_size))
                .small()
                .color(egui::Color32::GRAY),
        );
    });

    // File path (truncated)
    if !file_path.is_empty() {
        ui.label(
            egui::RichText::new(truncate_path(file_path, 60))
                .small()
                .color(egui::Color32::GRAY),
        );
    }
}

/// Format file size in human-readable format
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Truncate path for display
pub fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        let start = path.len() - max_len + 3;
        format!("...{}", &path[start..])
    }
}

/// Display a styled button with icon
pub fn icon_button(ui: &mut egui::Ui, icon: &str, _tooltip: &str) -> bool {
    ui.add(egui::Button::new(icon).frame(false))
        .clicked()
}

/// Display loading spinner with message
pub fn loading_spinner(ui: &mut egui::Ui, message: &str) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(message);
    });
}

/// Apply theme to egui context
pub fn apply_theme(ctx: &egui::Context, theme: AppTheme) {
    match theme {
        AppTheme::Light => {
            let mut visuals = egui::Visuals::light();
            visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 212);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 100, 180);
            ctx.set_visuals(visuals);
        }
        AppTheme::Dark => {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 212);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 100, 180);
            ctx.set_visuals(visuals);
        }
        AppTheme::Blue => {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = egui::Color32::from_rgb(86, 86, 156);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(98, 114, 164);
            ctx.set_visuals(visuals);
        }
        AppTheme::Green => {
            let mut visuals = egui::Visuals::light();
            visuals.selection.bg_fill = egui::Color32::from_rgb(76, 175, 80);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(56, 142, 60);
            ctx.set_visuals(visuals);
        }
        AppTheme::Purple => {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = egui::Color32::from_rgb(98, 114, 164);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(139, 92, 246);
            ctx.set_visuals(visuals);
        }
    }
}
