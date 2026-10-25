//! Envelopes: a one-shot exponential decay and an attack/decay envelope.

/// Ratio a "decay time" refers to: the envelope falls by 60 dB (to 1/1000).
const DECAY_RATIO: f32 = 1.0e-3;

/// Per-sample multiplier that decays by 60 dB over `time_ms`.
#[inline]
pub fn decay_coef(time_ms: f32, sample_rate: f32) -> f32 {
    let samples = (time_ms * 0.001 * sample_rate).max(1.0);
    DECAY_RATIO.powf(1.0 / samples)
}

/// Exponential decay from 1 to 0. `decay_ms` is the time it takes to fall by 60 dB.
#[derive(Debug, Clone)]
pub struct ExpDecay {
    value: f32,
    coef: f32,
}

impl ExpDecay {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            coef: 0.0,
        }
    }

    pub fn set_decay(&mut self, decay_ms: f32, sample_rate: f32) {
        self.coef = decay_coef(decay_ms, sample_rate);
    }

    /// Jump to full level.
    pub fn trigger(&mut self) {
        self.value = 1.0;
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    /// Output the current value, then decay.
    #[inline]
    pub fn next(&mut self) -> f32 {
        let out = self.value;
        self.value *= self.coef;
        // Flush denormals and tails below roughly -200 dB to exact zero.
        if self.value < 1.0e-10 {
            self.value = 0.0;
        }
        out
    }
}

impl Default for ExpDecay {
    fn default() -> Self {
        Self::new()
    }
}

/// Linear attack to 1, then exponential decay to 0.
///
/// The attack starts from the current value, so a retrigger doesn't click.
#[derive(Debug, Clone, Default)]
pub struct AdEnvelope {
    value: f32,
    attack_step: f32,
    decay: ExpDecay,
    attacking: bool,
}

impl AdEnvelope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_times(&mut self, attack_ms: f32, decay_ms: f32, sample_rate: f32) {
        let attack_samples = attack_ms * 0.001 * sample_rate;
        self.attack_step = if attack_samples < 1.0 {
            1.0
        } else {
            1.0 / attack_samples
        };
        self.decay.set_decay(decay_ms, sample_rate);
    }

    pub fn trigger(&mut self) {
        self.attacking = true;
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.attacking = false;
        self.decay.reset();
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if self.attacking {
            self.value += self.attack_step;
            if self.value >= 1.0 {
                self.value = 1.0;
                self.attacking = false;
                self.decay.trigger();
            }
            self.value
        } else {
            self.value = self.decay.next();
            self.value
        }
    }
}
