//! The Infinati Kick voice: a sine with an exponential pitch sweep, an
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

use crate::{db_to_gain, Drive, ExpDecay, NoiseBurst, OnePole, SineOsc, Smoother};

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
    pub fn new(sample_rate: f32, params: &KickParams) -> Self {
        let sr = sample_rate;
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

        drive.process(body + click).clamp(-1.0, 1.0) * level
    }

    /// Fill `out` with samples.
    pub fn render_into(&mut self, out: &mut [f32]) {
        for s in out {
            *s = self.next();
        }
    }
}

/// Offline helper: render one hit of `seconds` length. Allocates, so don't
/// call it on the audio thread.
pub fn render(params: &KickParams, sample_rate: f32, seconds: f32) -> Vec<f32> {
    let mut voice = KickVoice::new(sample_rate, params);
    voice.trigger();
    let mut out = vec![0.0; (seconds * sample_rate).round() as usize];
    voice.render_into(&mut out);
    out
}
