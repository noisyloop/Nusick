//! Render one Kick Me hit to a WAV file for quick listening.
//!
//! ```text
//! cargo run -p nusick-dsp --bin render-kick -- kick.wav --start 220 --end 45 --drive 0.4
//! ```

use nusick_dsp::{kick, wav, KickParams};
use std::{env, fs::File, io::BufWriter, process::ExitCode};

const USAGE: &str = "usage: render-kick <out.wav> [--start HZ] [--end HZ] [--pitch-decay MS] \
[--amp-decay MS] [--click 0..1] [--drive 0..1] [--level DB] [--length S] [--rate HZ]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or("missing output path")?;
    if path == "-h" || path == "--help" {
        println!("{USAGE}");
        return Ok(());
    }

    let mut p = KickParams::default();
    let mut length = 1.0_f32;
    let mut rate = 48_000_u32;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let v: f32 = value
            .parse()
            .map_err(|_| format!("bad number for {flag}: {value}"))?;
        match flag.as_str() {
            "--start" => p.start_hz = v,
            "--end" => p.end_hz = v,
            "--pitch-decay" => p.pitch_decay_ms = v,
            "--amp-decay" => p.amp_decay_ms = v,
            "--click" => p.click = v,
            "--drive" => p.drive = v,
            "--level" => p.level_db = v,
            "--length" => length = v,
            "--rate" => rate = v as u32,
            _ => return Err(format!("unknown flag {flag}")),
        }
    }

    let samples = kick::render(&p, rate as f32, length);
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let file = File::create(&path).map_err(|e| format!("{path}: {e}"))?;
    wav::write_wav_f32(BufWriter::new(file), &samples, rate).map_err(|e| e.to_string())?;
    println!(
        "wrote {path}: {length:.2} s at {rate} Hz, peak {:.2} dBFS, {p:?}",
        nusick_dsp::gain_to_db(peak)
    );
    Ok(())
}
