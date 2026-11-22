//! # nusick-dsp
//!
//! Pure-Rust DSP building blocks for Nusick plugins, with no plugin-API
//! dependencies, so everything here can be unit-tested and rendered offline.
//!
//! Rules for this crate:
//! - Nothing allocates after construction. Types that need memory (for example
//!   [`DelayLine`]) allocate in `new()`, which runs off the audio thread.
//! - Every processor is deterministic: the same input and settings always give
//!   the same output, noise included.
//! - Nothing panics, allocates without bound, or outputs NaN/inf, whatever it is given. Values
//!   arrive from saved sessions, hosts and command lines, so every public entry point clamps or
//!   rejects them with [`finite`] / [`sane_sample_rate`], and stateful processors recover from a
//!   bad value instead of keeping it forever. `tests/hostile_input.rs` enforces this.
//! - Processors run per sample (`next()` / `process()`) in `f32`. Phase
//!   accumulators use `f64` to avoid drift.

// Per-sample `next()` is the DSP idiom here; these aren't iterators.
#![allow(clippy::should_implement_trait)]

pub mod delay;
pub mod envelope;
pub mod filter;
pub mod kick;
pub mod noise;
pub mod oscillator;
pub mod shaper;
pub mod smoother;
pub mod wav;

pub use delay::DelayLine;
pub use envelope::{AdEnvelope, ExpDecay};
pub use filter::OnePole;
pub use kick::{KickParams, KickVoice};
pub use noise::NoiseBurst;
pub use oscillator::SineOsc;
pub use shaper::{soft_clip, Drive};
pub use smoother::Smoother;

/// Lowest sample rate the DSP accepts, in Hz.
pub const MIN_SAMPLE_RATE: f32 = 8_000.0;
/// Highest sample rate the DSP accepts, in Hz.
pub const MAX_SAMPLE_RATE: f32 = 768_000.0;
/// Rate used when a caller passes one outside the accepted range (or NaN).
pub const DEFAULT_SAMPLE_RATE: f32 = 48_000.0;

/// Returns `value` clamped to `[lo, hi]`, or `default` if it isn't finite.
///
/// Unlike `f32::clamp` this never panics (when `lo > hi` or a bound is NaN) and never passes NaN
/// through, which is how a bad parameter used to get into a processor's state for good.
#[inline]
pub fn finite(value: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.max(lo).min(hi)
    } else {
        default
    }
}

/// Returns `rate` if it is a usable sample rate, otherwise [`DEFAULT_SAMPLE_RATE`].
#[inline]
pub fn sane_sample_rate(rate: f32) -> f32 {
    if rate.is_finite() && (MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&rate) {
        rate
    } else {
        DEFAULT_SAMPLE_RATE
    }
}

/// Convert decibels to linear gain.
#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear gain to decibels. Returns `-inf` for silence.
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.abs().log10()
}
