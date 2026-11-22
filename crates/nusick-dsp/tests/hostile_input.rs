//! Hostile-input tests: values a saved session, a host or a command line can hand to the DSP.
//!
//! Each test names the finding in docs/SECURITY_REVIEW.md that it pins down. The rule for
//! the whole crate is the one AudioZero's engine follows: **no input, however absurd, may panic,
//! allocate without bound, or leave NaN/inf in the output.**

use nusick_dsp::{
    kick, wav, AdEnvelope, DelayLine, Drive, ExpDecay, KickParams, KickVoice, OnePole, SineOsc,
    Smoother,
};

const BAD_F32: [f32; 9] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MAX,
    f32::MIN,
    1.0e30,
    -1.0e30,
    0.0,
    -0.0,
];

fn all_finite_and_bounded(out: &[f32], bound: f32) {
    for (i, s) in out.iter().enumerate() {
        assert!(s.is_finite(), "sample {i} is {s}");
        assert!(
            s.abs() <= bound,
            "sample {i} is {s}, over the bound {bound}"
        );
    }
}

fn hostile_params() -> Vec<KickParams> {
    let mut all = Vec::new();
    for &v in &BAD_F32 {
        let d = KickParams::default();
        all.push(KickParams { start_hz: v, ..d });
        all.push(KickParams { end_hz: v, ..d });
        all.push(KickParams {
            pitch_decay_ms: v,
            ..d
        });
        all.push(KickParams {
            amp_decay_ms: v,
            ..d
        });
        all.push(KickParams { click: v, ..d });
        all.push(KickParams { drive: v, ..d });
        all.push(KickParams { level_db: v, ..d });
        all.push(KickParams {
            start_hz: v,
            end_hz: v,
            pitch_decay_ms: v,
            amp_decay_ms: v,
            click: v,
            drive: v,
            level_db: v,
        });
    }
    all
}

/// Finding 1 (NaN/inf poisoning; AudioZero's finding 5): a bad parameter must not reach the output, and a voice that
/// saw one must recover once the parameters are sane again.
#[test]
fn bad_params_never_make_a_non_finite_or_loud_output() {
    for p in hostile_params() {
        let mut voice = KickVoice::new(48_000.0, &p);
        voice.trigger();
        let mut out = vec![0.0; 4096];
        voice.render_into(&mut out);
        all_finite_and_bounded(&out, 1.0);

        // Recovery: after the bad value is replaced the voice must play normally again.
        voice.set_params(&KickParams::default());
        voice.trigger();
        let mut out = vec![0.0; 48_000];
        voice.render_into(&mut out);
        all_finite_and_bounded(&out, 1.0);
        let peak = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.1, "voice stayed silent after {p:?} (peak {peak})");
    }
}

/// Finding 4 (unbounded magnitudes; AudioZero's finding 6): the output peak may never exceed the documented level range.
#[test]
fn level_is_capped() {
    let p = KickParams {
        level_db: 1000.0,
        ..KickParams::default()
    };
    let out = kick::render(&p, 48_000.0, 0.5);
    all_finite_and_bounded(&out, 1.0);
}

/// Finding 2 (panic from a bad sample rate): the sample rate comes from the host.
#[test]
fn bad_sample_rates_neither_panic_nor_poison() {
    for &rate in &BAD_F32 {
        let mut voice = KickVoice::new(rate, &KickParams::default());
        voice.trigger();
        let mut out = vec![0.0; 1024];
        voice.render_into(&mut out);
        all_finite_and_bounded(&out, 1.0);
    }
    // `f32::clamp` panics when min > max, which used to happen for rates below about 2 Hz.
    for rate in [0.5, 1.0, 2.0, 100.0] {
        let mut voice = KickVoice::new(rate, &KickParams::default());
        voice.trigger();
        voice.render_into(&mut [0.0; 256]);
    }
}

/// Finding 3 (unbounded allocation; AudioZero's finding 3): an offline render must refuse absurd lengths and rates.
#[test]
fn render_is_bounded() {
    let p = KickParams::default();
    for &seconds in &BAD_F32 {
        let out = kick::render(&p, 48_000.0, seconds);
        assert!(out.len() as f64 <= kick::MAX_RENDER_SECONDS * 768_000.0);
        all_finite_and_bounded(&out, 1.0);
    }
    for &rate in &BAD_F32 {
        let out = kick::render(&p, rate, 1.0);
        assert!(out.len() as f64 <= kick::MAX_RENDER_SECONDS * 768_000.0);
        all_finite_and_bounded(&out, 1.0);
    }
    let out = kick::render(&p, 48_000.0, 1.0e9);
    assert!(out.len() as f64 <= kick::MAX_RENDER_SECONDS * 48_000.0 + 1.0);
}

/// Finding 3: the WAV header holds 32-bit sizes, so oversized input is an error, not a
/// corrupt file (or an overflow panic in a debug build).
#[test]
fn wav_header_sizes_are_checked() {
    let mut buf = Vec::new();
    assert!(wav::write_wav_f32(&mut buf, &[0.0; 8], 48_000).is_ok());
    assert_eq!(buf.len(), 44 + 8 * 4);

    // A rate whose byte rate overflows u32.
    let mut buf = Vec::new();
    assert!(wav::write_wav_f32(&mut buf, &[0.0; 8], u32::MAX).is_err());
    // A rate of zero is not a valid file either.
    let mut buf = Vec::new();
    assert!(wav::write_wav_f32(&mut buf, &[0.0; 8], 0).is_err());
}

/// Finding 1: every stateful processor must shrug off non-finite input and recover.
#[test]
fn processors_recover_from_non_finite_input() {
    // Oscillator: a NaN frequency used to make the phase NaN for good.
    let mut osc = SineOsc::new();
    for &f in &BAD_F32 {
        osc.next(f, 48_000.0);
    }
    for _ in 0..8 {
        assert!(osc.next(100.0, 48_000.0).is_finite());
    }
    let mut osc = SineOsc::new();
    for &sr in &BAD_F32 {
        osc.next(100.0, sr);
    }
    for _ in 0..8 {
        assert!(osc.next(100.0, 48_000.0).is_finite());
    }

    // Smoother: a NaN target used to leave `current` NaN forever.
    let mut s = Smoother::new(1.0, 20.0, 48_000.0);
    for &v in &BAD_F32 {
        s.set_target(v);
        for _ in 0..4 {
            s.next();
        }
    }
    s.set_target(2.0);
    let last = (0..96_000).map(|_| s.next()).last().unwrap();
    assert!((last - 2.0).abs() < 1.0e-3, "smoother stuck at {last}");
    for &v in &BAD_F32 {
        let s = Smoother::new(v, v, v);
        assert!(s.current().is_finite());
    }

    // One-pole filter: NaN cutoff / input.
    for &v in &BAD_F32 {
        let mut f = OnePole::new(v, 48_000.0);
        f.set_cutoff(v, v);
        f.lowpass(v);
        f.highpass(v);
        f.set_cutoff(1500.0, 48_000.0);
        let mut last = 0.0;
        for _ in 0..8 {
            last = f.lowpass(0.5);
        }
        assert!(last.is_finite(), "one-pole stuck at {last} after {v}");
    }

    // Drive: NaN amount.
    for &v in &BAD_F32 {
        let d = Drive::new(v);
        for &x in &BAD_F32 {
            let y = d.process(x);
            assert!(y.is_finite(), "drive({v})({x}) = {y}");
        }
    }

    // Envelopes: NaN times.
    for &v in &BAD_F32 {
        let mut env = ExpDecay::new();
        env.set_decay(v, v);
        env.trigger();
        assert!(env.next().is_finite());
        let mut ad = AdEnvelope::new();
        ad.set_times(v, v, v);
        ad.trigger();
        for _ in 0..16 {
            assert!(ad.next().is_finite());
        }
        ad.set_times(5.0, 50.0, 48_000.0);
        ad.reset();
        ad.trigger();
        let peak = (0..4096).map(|_| ad.next()).fold(0.0_f32, f32::max);
        assert!(peak > 0.9, "envelope stuck after {v}");
    }
}

/// Finding 3: the delay line's size is caller-chosen; it must be capped, and a NaN delay time
/// must not produce NaN.
#[test]
fn delay_line_is_bounded_and_finite() {
    let d = DelayLine::new(usize::MAX);
    assert!(d.max_delay() <= DelayLine::MAX_SAMPLES);

    let mut d = DelayLine::new(100);
    for &t in &BAD_F32 {
        let y = d.process(0.5, t);
        assert!(y.is_finite(), "delay({t}) = {y}");
    }
    for &x in &BAD_F32 {
        d.process(x, 10.0);
    }
    for _ in 0..512 {
        assert!(
            d.process(0.0, 10.0).is_finite(),
            "delay buffer stayed poisoned"
        );
    }
}
