//! Minimal mono 32-bit float WAV writer for offline renders. Not for the audio thread.

use std::io::{self, Write};

/// Write `samples` as a mono IEEE-float WAV file.
pub fn write_wav_f32(mut w: impl Write, samples: &[f32], sample_rate: u32) -> io::Result<()> {
    let data_len = (samples.len() * 4) as u32;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    w.write_all(&3u16.to_le_bytes())?; // format: IEEE float
    w.write_all(&1u16.to_le_bytes())?; // channels
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&(sample_rate * 4).to_le_bytes())?; // byte rate
    w.write_all(&4u16.to_le_bytes())?; // block align
    w.write_all(&32u16.to_le_bytes())?; // bits per sample
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}
