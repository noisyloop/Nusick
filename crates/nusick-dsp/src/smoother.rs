//! One-pole parameter smoother, to remove zipper noise on control changes.

#[derive(Debug, Clone)]
pub struct Smoother {
    current: f32,
    target: f32,
    coef: f32,
}

impl Smoother {
    /// `time_ms` is roughly how long it takes to cover 99.9% of a jump.
    pub fn new(initial: f32, time_ms: f32, sample_rate: f32) -> Self {
        let mut s = Self {
            current: initial,
            target: initial,
            coef: 0.0,
        };
        s.set_time(time_ms, sample_rate);
        s
    }

    pub fn set_time(&mut self, time_ms: f32, sample_rate: f32) {
        // Time to reach 1/1000 of the remaining distance.
        self.coef = crate::envelope::decay_coef(time_ms, sample_rate);
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Jump straight to `value` with no smoothing.
    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Jump to the current target.
    pub fn snap(&mut self) {
        self.current = self.target;
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.current = self.target + (self.current - self.target) * self.coef;
        // Snap when close: f32 rounding can otherwise stall just short of the target.
        if (self.current - self.target).abs() <= 1.0e-5 * self.target.abs().max(1.0) {
            self.current = self.target;
        }
        self.current
    }
}
