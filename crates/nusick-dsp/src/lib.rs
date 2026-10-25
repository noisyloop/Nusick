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
