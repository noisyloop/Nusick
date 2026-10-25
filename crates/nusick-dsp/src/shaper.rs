//! tanh soft clipping.

/// Plain tanh soft clip.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    x.tanh()
}

/// Gain-normalized tanh drive: `tanh(g·x) / tanh(g)`.
///
/// Full-scale input (±1) always maps to ±1, so drive changes the tone without
/// changing the peak level. At `amount = 0` the stage is linear.
#[derive(Debug, Clone, Copy)]
pub struct Drive {
    gain: f32,
    norm: f32,
}

impl Drive {
    /// Pre-gain at maximum drive.
    pub const MAX_GAIN: f32 = 10.0;

    /// `amount` runs from 0 (clean) to 1 (heavy).
    pub fn new(amount: f32) -> Self {
        let gain = amount.clamp(0.0, 1.0) * Self::MAX_GAIN;
        let norm = if gain < 1.0e-3 {
            1.0
        } else {
            1.0 / gain.tanh()
        };
        Self { gain, norm }
    }

    #[inline]
    pub fn process(&self, x: f32) -> f32 {
        if self.gain < 1.0e-3 {
            x
        } else {
            (self.gain * x).tanh() * self.norm
        }
    }
}
