//! One-pole (6 dB/oct) low-pass filter with a matching high-pass output.

use std::f32::consts::TAU;

#[derive(Debug, Clone, Default)]
pub struct OnePole {
    a: f32,
    state: f32,
}

impl OnePole {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let mut f = Self::default();
        f.set_cutoff(cutoff_hz, sample_rate);
        f
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        // `f32::clamp` panics when min > max, which a tiny or NaN sample rate used to cause.
        let sample_rate = crate::sane_sample_rate(sample_rate);
        let fc = crate::finite(cutoff_hz, 1.0, sample_rate * 0.49, 1.0);
        self.a = 1.0 - (-TAU * fc / sample_rate).exp();
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    #[inline]
    pub fn lowpass(&mut self, x: f32) -> f32 {
        let x = if x.is_finite() { x } else { 0.0 };
        self.state += self.a * (x - self.state);
        if !self.state.is_finite() {
            self.state = 0.0;
        }
        self.state
    }

    /// High-pass output: the input minus the low-pass.
    #[inline]
    pub fn highpass(&mut self, x: f32) -> f32 {
        x - self.lowpass(x)
    }
}
