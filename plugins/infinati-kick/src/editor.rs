//! The Infinati Kick editor: seven knobs in a row and an output meter.

use crate::params::InfinatiKickParams;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui};
use nusick_ui::{theme, Knob, Meter};
use std::sync::{atomic::Ordering, Arc};

pub fn create(params: Arc<InfinatiKickParams>, peak: Arc<AtomicF32>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| theme::apply(ctx),
        move |ctx, setter, _| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("INFINATI KICK")
                            .color(theme::ACCENT)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("nusick").color(theme::TEXT_DIM).small());
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let p = &params;
                    ui.add(Knob::new(&p.start_pitch, setter));
                    ui.add(Knob::new(&p.end_pitch, setter));
                    ui.add(Knob::new(&p.pitch_decay, setter));
                    ui.add(Knob::new(&p.amp_decay, setter));
                    ui.add(Knob::new(&p.click, setter));
                    ui.add(Knob::new(&p.drive, setter));
                    ui.add(Knob::new(&p.level, setter));
                    ui.add_space(8.0);
                    let db = util::gain_to_db(peak.load(Ordering::Relaxed));
                    ui.add(Meter::new(db));
                });
            });
            // Keep the meter moving while the window is open.
            ctx.request_repaint();
        },
    )
}
