//! Deterministic white noise and a short decaying noise burst.

/// Seed used on every trigger, so each hit sounds identical.
const SEED: u32 = 0x9E37_79B9;

/// White noise from a 32-bit xorshift generator, in `[-1, 1)`.
#[derive(Debug, Clone)]
pub struct WhiteNoise {
    state: u32,
}

impl WhiteNoise {
    pub fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn reseed(&mut self, seed: u32) {
        self.state = seed.max(1);
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Noise burst: white noise under a fast exponential decay.
#[derive(Debug, Clone)]
pub struct NoiseBurst {
    noise: WhiteNoise,
    env: crate::ExpDecay,
}

impl NoiseBurst {
    pub fn new(decay_ms: f32, sample_rate: f32) -> Self {
        let mut env = crate::ExpDecay::new();
        env.set_decay(decay_ms, sample_rate);
        Self {
            noise: WhiteNoise::new(SEED),
            env,
        }
    }

    pub fn set_decay(&mut self, decay_ms: f32, sample_rate: f32) {
        self.env.set_decay(decay_ms, sample_rate);
    }

    pub fn trigger(&mut self) {
        self.noise.reseed(SEED);
        self.env.trigger();
    }

    pub fn reset(&mut self) {
        self.env.reset();
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let env = self.env.next();
        if env == 0.0 {
            return 0.0;
        }
        self.noise.next() * env
    }
}
