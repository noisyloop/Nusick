//! Sine oscillator with an `f64` phase accumulator.

use std::f64::consts::TAU;

#[derive(Debug, Clone, Default)]
pub struct SineOsc {
    /// Phase in cycles, kept in `[0, 1)`.
    phase: f64,
}

impl SineOsc {
    pub fn new() -> Self {
        Self::default()
    }

    /// Restart at phase 0, so a new note always starts at a zero crossing.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Output the next sample, then advance by `freq_hz`. The frequency can
    /// change every sample; the phase stays continuous.
    #[inline]
    pub fn next(&mut self, freq_hz: f32, sample_rate: f32) -> f32 {
        let out = (self.phase * TAU).sin() as f32;
        self.phase += freq_hz as f64 / sample_rate as f64;
        self.phase -= self.phase.floor();
        out
    }
}
