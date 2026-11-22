//! The Kick Me voice: a sine with an exponential pitch sweep, an
//! exponential amp decay, a noise click, tanh drive and an output level.
//!
//! The kick lives here, not in the plugin crate, so it can be tested and
//! rendered offline with no host.
//!
//! Signal flow:
//! ```text
//! sine(end + (start - end) * pitch_env) * amp_env ─┐
//!                                                  + ─> Drive ─> clamp ±1 ─> × level
//! highpassed noise burst * click ──────────────────┘
//! ```
//! Drive is gain-normalized and the sum is clamped to ±1, so the output peak
//! never goes above `level_db`.

use crate::{
    db_to_gain, finite, sane_sample_rate, Drive, ExpDecay, NoiseBurst, OnePole, SineOsc, Smoother,
};

/// Longest offline render, in seconds.
pub const MAX_RENDER_SECONDS: f64 = 60.0;

/// Fixed voicing constants.
const CLICK_DECAY_MS: f32 = 6.0;
const CLICK_HIGHPASS_HZ: f32 = 1500.0;
const SMOOTHING_MS: f32 = 20.0;

/// User-facing kick settings, in the units the plugin shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KickParams {
    /// Pitch at the start of the hit, in Hz.
    pub start_hz: f32,
    /// Pitch the sweep settles at, in Hz.
    pub end_hz: f32,
    /// Time for the pitch sweep to fall by 60 dB towards `end_hz`, in ms.
    pub pitch_decay_ms: f32,
    /// Time for the amplitude to fall by 60 dB, in ms.
    pub amp_decay_ms: f32,
    /// Click (noise burst) amount, 0 to 1.
    pub click: f32,
    /// Drive amount, 0 to 1.
    pub drive: f32,
    /// Output peak level in dBFS.
    pub level_db: f32,
}

impl KickParams {
    /// Allowed ranges, `(min, max)`. These are the single source of truth: the plugin's
    /// parameters use them too, so the DSP and the plugin can't drift apart. Anything outside is
    /// clamped, and anything that isn't finite falls back to the default.
    pub const START_HZ: (f32, f32) = (40.0, 1000.0);
    pub const END_HZ: (f32, f32) = (20.0, 200.0);
    pub const PITCH_DECAY_MS: (f32, f32) = (1.0, 500.0);
    pub const AMP_DECAY_MS: (f32, f32) = (10.0, 2000.0);
    pub const CLICK: (f32, f32) = (0.0, 1.0);
    pub const DRIVE: (f32, f32) = (0.0, 1.0);
    /// The ceiling is 0 dB so a bad level can never push the output above full scale.
    pub const LEVEL_DB: (f32, f32) = (-36.0, 0.0);

    /// A copy with every field clamped into its range (non-finite values become the default).
    pub fn sanitized(&self) -> Self {
        let d = Self::default();
        Self {
            start_hz: finite(
                self.start_hz,
                Self::START_HZ.0,
                Self::START_HZ.1,
                d.start_hz,
            ),
            end_hz: finite(self.end_hz, Self::END_HZ.0, Self::END_HZ.1, d.end_hz),
            pitch_decay_ms: finite(
                self.pitch_decay_ms,
                Self::PITCH_DECAY_MS.0,
                Self::PITCH_DECAY_MS.1,
                d.pitch_decay_ms,
            ),
            amp_decay_ms: finite(
                self.amp_decay_ms,
                Self::AMP_DECAY_MS.0,
                Self::AMP_DECAY_MS.1,
                d.amp_decay_ms,
            ),
            click: finite(self.click, Self::CLICK.0, Self::CLICK.1, d.click),
            drive: finite(self.drive, Self::DRIVE.0, Self::DRIVE.1, d.drive),
            level_db: finite(
                self.level_db,
                Self::LEVEL_DB.0,
                Self::LEVEL_DB.1,
                d.level_db,
            ),
        }
    }
}

impl Default for KickParams {
    fn default() -> Self {
        Self {
            start_hz: 200.0,
            end_hz: 50.0,
            pitch_decay_ms: 60.0,
            amp_decay_ms: 450.0,
            click: 0.2,
            drive: 0.2,
            level_db: -3.0,
        }
    }
}

/// One monophonic kick voice. A new trigger restarts the hit.
#[derive(Debug, Clone)]
pub struct KickVoice {
    sample_rate: f32,
    osc: SineOsc,
    pitch_env: ExpDecay,
    amp_env: ExpDecay,
    click: NoiseBurst,
    click_filter: OnePole,
    // Smoothed controls: anything that would zipper if it jumped.
    start_hz: Smoother,
    end_hz: Smoother,
    click_amount: Smoother,
    drive_amount: Smoother,
    level_gain: Smoother,
}

impl KickVoice {
    /// `sample_rate` outside [`crate::MIN_SAMPLE_RATE`]..=[`crate::MAX_SAMPLE_RATE`] (or NaN)
    /// is replaced by [`crate::DEFAULT_SAMPLE_RATE`]; `params` are clamped.
    pub fn new(sample_rate: f32, params: &KickParams) -> Self {
        let sr = sane_sample_rate(sample_rate);
        let params = &params.sanitized();
        let mut voice = Self {
            sample_rate: sr,
            osc: SineOsc::new(),
            pitch_env: ExpDecay::new(),
            amp_env: ExpDecay::new(),
            click: NoiseBurst::new(CLICK_DECAY_MS, sr),
            click_filter: OnePole::new(CLICK_HIGHPASS_HZ, sr),
            start_hz: Smoother::new(params.start_hz, SMOOTHING_MS, sr),
            end_hz: Smoother::new(params.end_hz, SMOOTHING_MS, sr),
            click_amount: Smoother::new(params.click, SMOOTHING_MS, sr),
            drive_amount: Smoother::new(params.drive, SMOOTHING_MS, sr),
            level_gain: Smoother::new(db_to_gain(params.level_db), SMOOTHING_MS, sr),
        };
        voice.set_params(params);
        voice
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Set new targets. Cheap enough to call once per block or per sample.
    /// Pitch, click, drive and level glide over ~20 ms. Decay times change
    /// the envelope rate straight away, which can't cause a step.
    pub fn set_params(&mut self, p: &KickParams) {
        let p = &p.sanitized();
        let sr = self.sample_rate;
        self.start_hz.set_target(p.start_hz);
        self.end_hz.set_target(p.end_hz);
        self.click_amount.set_target(p.click.clamp(0.0, 1.0));
        self.drive_amount.set_target(p.drive.clamp(0.0, 1.0));
        self.level_gain.set_target(db_to_gain(p.level_db));
        self.pitch_env.set_decay(p.pitch_decay_ms, sr);
        self.amp_env.set_decay(p.amp_decay_ms, sr);
    }

    /// Start (or restart) a hit.
    pub fn trigger(&mut self) {
        self.osc.reset();
        self.pitch_env.trigger();
        self.amp_env.trigger();
        self.click.trigger();
        self.click_filter.reset();
    }

    /// Silence the voice and snap all smoothers to their targets.
    pub fn reset(&mut self) {
        self.osc.reset();
        self.pitch_env.reset();
        self.amp_env.reset();
        self.click.reset();
        self.click_filter.reset();
        for s in [
            &mut self.start_hz,
            &mut self.end_hz,
            &mut self.click_amount,
            &mut self.drive_amount,
            &mut self.level_gain,
        ] {
            s.snap();
        }
    }

    /// True while the amp envelope is still sounding.
    pub fn is_active(&self) -> bool {
        self.amp_env.value() > 0.0
    }

    /// Render one sample.
    #[inline]
    pub fn next(&mut self) -> f32 {
        let start = self.start_hz.next();
        let end = self.end_hz.next();
        let click_amount = self.click_amount.next();
        let drive = Drive::new(self.drive_amount.next());
        let level = self.level_gain.next();

        if !self.is_active() {
            return 0.0;
        }

        let freq = end + (start - end) * self.pitch_env.next();
        let body = self.osc.next(freq, self.sample_rate) * self.amp_env.next();
        let click = self.click_filter.highpass(self.click.next()) * click_amount;

        let out = drive.process(body + click).clamp(-1.0, 1.0) * level;
        // Last line of defence: nothing but a finite sample ever leaves the voice.
        if out.is_finite() {
            out
        } else {
            0.0
        }
    }

    /// Fill `out` with samples.
    pub fn render_into(&mut self, out: &mut [f32]) {
        for s in out {
            *s = self.next();
        }
    }
}

/// Offline helper: render one hit of `seconds` length (clamped to
/// [`MAX_RENDER_SECONDS`]). Allocates, so don't call it on the audio thread.
pub fn render(params: &KickParams, sample_rate: f32, seconds: f32) -> Vec<f32> {
    let mut voice = KickVoice::new(sample_rate, params);
    voice.trigger();
    let seconds = finite(seconds, 0.0, MAX_RENDER_SECONDS as f32, 0.0);
    let frames = (f64::from(seconds) * f64::from(voice.sample_rate())).round() as usize;
    let mut out = vec![0.0; frames];
    voice.render_into(&mut out);
    out
}
