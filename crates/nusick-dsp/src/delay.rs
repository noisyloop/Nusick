//! Fixed-capacity delay line. Memory is allocated once in [`DelayLine::new`].

#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Box<[f32]>,
    write: usize,
}

impl DelayLine {
    /// Largest capacity [`DelayLine::new`] will allocate (16M samples, 64 MB).
    pub const MAX_SAMPLES: usize = 1 << 24;
    /// Samples are clamped to this magnitude on the way in.
    pub const MAX_SAMPLE_VALUE: f32 = 1.0e9;

    /// Allocates room for `max_delay_samples` (capped at [`DelayLine::MAX_SAMPLES`]). Call this
    /// off the audio thread.
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples.clamp(1, Self::MAX_SAMPLES) + 1].into_boxed_slice(),
            write: 0,
        }
    }

    pub fn max_delay(&self) -> usize {
        self.buffer.len() - 1
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write = 0;
    }

    /// Write `x` and return the sample from `delay` samples ago (clamped to
    /// capacity). Fractional delays are linearly interpolated.
    #[inline]
    pub fn process(&mut self, x: f32, delay: f32) -> f32 {
        // Bounded as well as finite: interpolating between two huge samples would overflow.
        self.buffer[self.write] =
            crate::finite(x, -Self::MAX_SAMPLE_VALUE, Self::MAX_SAMPLE_VALUE, 0.0);
        let out = self.read(delay);
        self.write = (self.write + 1) % self.buffer.len();
        out
    }

    /// Read `delay` samples behind the most recent write.
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        let len = self.buffer.len();
        let delay = crate::finite(delay, 0.0, self.max_delay() as f32, 0.0);
        let whole = delay.floor() as usize;
        let frac = delay - whole as f32;
        let i0 = (self.write + len - whole) % len;
        let i1 = (i0 + len - 1) % len;
        self.buffer[i0] + (self.buffer[i1] - self.buffer[i0]) * frac
    }
}
