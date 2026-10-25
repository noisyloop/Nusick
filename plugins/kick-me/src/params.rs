//! Kick Me parameters. IDs are stable: changing one breaks saved sessions.

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use nusick_dsp::KickParams;
use std::sync::Arc;

#[derive(Params)]
pub struct KickMeParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "start"]
    pub start_pitch: FloatParam,
    #[id = "end"]
    pub end_pitch: FloatParam,
    #[id = "pdecay"]
    pub pitch_decay: FloatParam,
    #[id = "adecay"]
    pub amp_decay: FloatParam,
    #[id = "click"]
    pub click: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "level"]
    pub level: FloatParam,
}

fn hz(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_unit(" Hz")
    .with_value_to_string(formatters::v2s_f32_rounded(1))
}

fn ms(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_unit(" ms")
    .with_value_to_string(formatters::v2s_f32_rounded(1))
}

fn percent(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

impl Default for KickMeParams {
    fn default() -> Self {
        let d = KickParams::default();
        Self {
            editor_state: EguiState::from_size(560, 200),
            start_pitch: hz("Start Pitch", d.start_hz, 40.0, 1000.0),
            end_pitch: hz("End Pitch", d.end_hz, 20.0, 200.0),
            pitch_decay: ms("Pitch Decay", d.pitch_decay_ms, 1.0, 500.0),
            amp_decay: ms("Amp Decay", d.amp_decay_ms, 10.0, 2000.0),
            click: percent("Click", d.click),
            drive: percent("Drive", d.drive),
            level: FloatParam::new(
                "Level",
                d.level_db,
                FloatRange::Linear {
                    min: -36.0,
                    max: 0.0,
                },
            )
            .with_step_size(0.1)
            .with_unit(" dB"),
        }
    }
}

impl KickMeParams {
    /// Snapshot the current (unsmoothed) values. Smoothing happens in the DSP
    /// voice so offline renders and the plugin sound the same.
    pub fn kick_params(&self) -> KickParams {
        KickParams {
            start_hz: self.start_pitch.value(),
            end_hz: self.end_pitch.value(),
            pitch_decay_ms: self.pitch_decay.value(),
            amp_decay_ms: self.amp_decay.value(),
            click: self.click.value(),
            drive: self.drive.value(),
            level_db: self.level.value(),
        }
    }
}
