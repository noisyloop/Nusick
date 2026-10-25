//! Vertical peak meter.

use crate::theme;
use nih_plug_egui::egui::{vec2, CornerRadius, Rect, Response, Sense, Ui, Widget};

/// Shows a level in dBFS from `floor_db` up to 0 dB.
pub struct Meter {
    level_db: f32,
    floor_db: f32,
    size: (f32, f32),
}

impl Meter {
    pub fn new(level_db: f32) -> Self {
        Self {
            level_db,
            floor_db: -60.0,
            size: (10.0, 96.0),
        }
    }

    pub fn floor_db(mut self, floor_db: f32) -> Self {
        self.floor_db = floor_db;
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = (width, height);
        self
    }
}

impl Widget for Meter {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(self.size.0, self.size.1), Sense::hover());
        let fill = ((self.level_db - self.floor_db) / -self.floor_db).clamp(0.0, 1.0);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(2), theme::TRACK);
        let bar = Rect::from_min_max(
            rect.left_bottom() - vec2(0.0, rect.height() * fill),
            rect.right_bottom(),
        );
        painter.rect_filled(bar, CornerRadius::same(2), theme::ACCENT);
        let text = if self.level_db.is_finite() {
            format!("{:.1} dBFS", self.level_db)
        } else {
            "-inf dBFS".to_owned()
        };
        response.on_hover_text(text)
    }
}
