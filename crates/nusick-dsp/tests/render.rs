//! Offline render tests for the kick voice and the DSP blocks it uses.

use nusick_dsp::{db_to_gain, gain_to_db, kick, DelayLine, KickParams, KickVoice, Smoother};

const SR: f32 = 48_000.0;

fn peak(s: &[f32]) -> f32 {
    s.iter().fold(0.0_f32, |m, x| m.max(x.abs()))
}

/// A spread of settings, including the extremes of every parameter.
fn settings() -> Vec<KickParams> {
    let d = KickParams::default();
    vec![
        d,
        KickParams {
            click: 0.0,
            drive: 0.0,
            ..d
        },
        KickParams {
            click: 1.0,
            drive: 1.0,
            level_db: 0.0,
            ..d
        },
        KickParams {
            start_hz: 1000.0,
            end_hz: 20.0,
            pitch_decay_ms: 1.0,
            ..d
        },
        KickParams {
            start_hz: 40.0,
            end_hz: 200.0,
            pitch_decay_ms: 500.0,
            amp_decay_ms: 10.0,
            ..d
        },
        KickParams {
            amp_decay_ms: 2000.0,
            level_db: -36.0,
            ..d
        },
    ]
}

#[test]
fn output_is_finite() {
    for p in settings() {
        for sr in [44_100.0, 48_000.0, 96_000.0] {
            let out = kick::render(&p, sr, 2.0);
            assert!(
                out.iter().all(|s| s.is_finite()),
                "non-finite sample for {p:?} @ {sr}"
            );
        }
    }
}

#[test]
fn peak_matches_level() {
    for level_db in [-24.0, -6.0, -3.0, 0.0] {
        for (click, drive) in [(0.0, 0.0), (0.2, 0.2), (1.0, 1.0)] {
            let p = KickParams {
                level_db,
                click,
                drive,
                ..KickParams::default()
            };
            let out = kick::render(&p, SR, 1.0);
            let peak_db = gain_to_db(peak(&out));
            assert!(
                peak_db <= level_db + 1e-3,
                "peak {peak_db} dB above level {level_db} for {p:?}"
            );
            assert!(
                peak_db >= level_db - 0.5,
                "peak {peak_db} dB well below level {level_db} for {p:?}"
            );
        }
    }
}

#[test]
fn default_level_is_minus_3_db() {
    let out = kick::render(&KickParams::default(), SR, 1.0);
    assert!((gain_to_db(peak(&out)) + 3.0).abs() < 0.5);
}

#[test]
fn decays_to_silence() {
    for p in settings() {
        // Amp decay is the -60 dB time, so after 2x that the tail sits below
        // -120 dB relative to full scale.
        let len = 2.0 * p.amp_decay_ms / 1000.0 + 0.1;
        let out = kick::render(&p, SR, len);
        let tail = &out[out.len() - (SR * 0.05) as usize..];
        assert!(
            peak(tail) < db_to_gain(-100.0),
            "tail too loud for {p:?}: {}",
            peak(tail)
        );
    }
}

#[test]
fn voice_goes_inactive() {
    let p = KickParams {
        amp_decay_ms: 50.0,
        ..KickParams::default()
    };
    let mut v = KickVoice::new(SR, &p);
    v.trigger();
    let mut buf = vec![0.0; SR as usize];
    v.render_into(&mut buf);
    assert!(!v.is_active());
    assert_eq!(v.next(), 0.0);
}

#[test]
fn output_is_deterministic() {
    for p in settings() {
        assert_eq!(kick::render(&p, SR, 0.5), kick::render(&p, SR, 0.5));
    }
}

#[test]
fn retrigger_restarts_the_hit() {
    let p = KickParams::default();
    let first = kick::render(&p, SR, 0.2);

    let mut v = KickVoice::new(SR, &p);
    v.trigger();
    let mut scratch = vec![0.0; (SR * 0.1) as usize];
    v.render_into(&mut scratch);
    v.trigger();
    let mut second = vec![0.0; first.len()];
    v.render_into(&mut second);
    assert_eq!(first, second, "a retrigger should replay the same hit");
}

#[test]
fn pitch_sweeps_down() {
    // Count zero crossings early vs. late: early should be higher pitched.
    let p = KickParams {
        click: 0.0,
        drive: 0.0,
        amp_decay_ms: 2000.0,
        ..KickParams::default()
    };
    let out = kick::render(&p, SR, 1.0);
    let crossings = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    let early = crossings(&out[..(SR * 0.02) as usize]) as f32 / 0.02;
    let late = crossings(&out[(SR * 0.5) as usize..(SR * 0.9) as usize]) as f32 / 0.4;
    assert!(early > 120.0, "early pitch {early} Hz");
    assert!(
        (late - p.end_hz).abs() < 5.0,
        "late pitch {late} Hz, expected ~{}",
        p.end_hz
    );
}

#[test]
fn smoother_reaches_target_without_jumping() {
    let mut s = Smoother::new(0.0, 20.0, SR);
    s.set_target(1.0);
    let mut prev = 0.0;
    for _ in 0..(SR * 0.05) as usize {
        let v = s.next();
        assert!(v - prev < 0.01, "step too large: {}", v - prev);
        prev = v;
    }
    assert_eq!(s.current(), 1.0);
}

#[test]
fn delay_line_delays() {
    let mut d = DelayLine::new(10);
    let out: Vec<f32> = (0..8)
        .map(|i| d.process(if i == 0 { 1.0 } else { 0.0 }, 3.0))
        .collect();
    assert_eq!(out, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    let mut d = DelayLine::new(10);
    let out: Vec<f32> = (0..4)
        .map(|i| d.process(if i == 0 { 1.0 } else { 0.0 }, 1.5))
        .collect();
    assert_eq!(out, vec![0.0, 0.5, 0.5, 0.0]);
}
