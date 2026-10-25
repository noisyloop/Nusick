//! Rotary knob bound to a nih-plug parameter.

use crate::theme;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self, pos2, vec2, Align2, FontId, Response, Sense, Shape, Stroke, Ui, Widget,
};
use std::f32::consts::PI;

/// Knob sweep: 270 degrees, starting at the bottom left.
const START_ANGLE: f32 = 0.75 * PI;
const SWEEP: f32 = 1.5 * PI;
/// Pixels of vertical drag for the full range.
const DRAG_PIXELS: f32 = 200.0;
const FINE_FACTOR: f32 = 0.1;

pub struct Knob<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    diameter: f32,
}

impl<'a, P: Param> Knob<'a, P> {
    pub fn new(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,
            diameter: 56.0,
        }
    }

    pub fn diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }
}

impl<P: Param> Widget for Knob<'_, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(self.diameter + 24.0, self.diameter + 40.0);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        // Interaction: gestures are bracketed with begin/end so hosts record
        // clean automation.
        if response.double_clicked() {
            self.setter.begin_set_parameter(self.param);
            self.setter
                .set_parameter(self.param, self.param.default_plain_value());
            self.setter.end_set_parameter(self.param);
            response.mark_changed();
        } else if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }
        if response.dragged() {
            let fine = ui.input(|i| i.modifiers.shift);
            let delta =
                -response.drag_delta().y / DRAG_PIXELS * if fine { FINE_FACTOR } else { 1.0 };
            if delta != 0.0 {
                let value = (self.param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
                self.setter.set_parameter_normalized(self.param, value);
                response.mark_changed();
            }
        }
        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        // Drawing.
        let painter = ui.painter_at(rect);
        let radius = self.diameter * 0.5;
        let center = pos2(rect.center().x, rect.top() + 18.0 + radius);
        let value = self.param.unmodulated_normalized_value();
        let accent = if response.hovered() || response.dragged() {
            theme::ACCENT.gamma_multiply(1.2)
        } else {
            theme::ACCENT
        };

        painter.text(
            pos2(center.x, rect.top() + 2.0),
            Align2::CENTER_TOP,
            self.param.name(),
            FontId::proportional(12.0),
            theme::TEXT_DIM,
        );
        painter.circle_filled(center, radius - 6.0, theme::PANEL);
        painter.add(arc(
            center,
            radius - 2.0,
            0.0,
            1.0,
            Stroke::new(4.0_f32, theme::TRACK),
        ));
        painter.add(arc(
            center,
            radius - 2.0,
            0.0,
            value,
            Stroke::new(4.0_f32, accent),
        ));
        let angle = START_ANGLE + SWEEP * value;
        let dir = vec2(angle.cos(), angle.sin());
        painter.line_segment(
            [center + dir * (radius * 0.2), center + dir * (radius - 8.0)],
            Stroke::new(2.5_f32, theme::TEXT),
        );
        painter.text(
            pos2(center.x, center.y + radius + 6.0),
            Align2::CENTER_TOP,
            self.param.to_string(),
            FontId::proportional(12.0),
            theme::TEXT,
        );

        response.on_hover_text("Drag to change, Shift for fine, double-click to reset")
    }
}

/// Arc from normalized position `from` to `to` (0..1 of the knob sweep).
fn arc(center: egui::Pos2, radius: f32, from: f32, to: f32, stroke: Stroke) -> Shape {
    let steps = ((to - from).abs() * 48.0).ceil().max(1.0) as usize;
    let points = (0..=steps)
        .map(|i| {
            let t = from + (to - from) * i as f32 / steps as f32;
            let a = START_ANGLE + SWEEP * t;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    Shape::line(points, stroke)
}
