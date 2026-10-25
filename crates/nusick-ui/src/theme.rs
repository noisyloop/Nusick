//! Nusick's dark theme.

use nih_plug_egui::egui::{self, Color32, CornerRadius, Stroke};

pub const BACKGROUND: Color32 = Color32::from_rgb(0x12, 0x12, 0x16);
pub const PANEL: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x22);
pub const TRACK: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x38);
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0x4d, 0x6d);
pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe6, 0xeb);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x96);

/// Apply the Nusick theme to an egui context. Cheap enough to call every frame.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, TRACK);
    for w in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        w.corner_radius = CornerRadius::same(4);
    }
    ctx.set_visuals(visuals);
}
