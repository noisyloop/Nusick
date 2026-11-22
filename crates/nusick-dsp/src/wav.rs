//! Minimal mono 32-bit float WAV writer for offline renders. Not for the audio thread.

use std::io::{self, Write};

/// Write `samples` as a mono IEEE-float WAV file.
///
/// The header stores sizes as 32-bit values, so input that doesn't fit (or a sample rate of zero,
/// or one whose byte rate overflows) is an `InvalidInput` error rather than a corrupt file.
pub fn write_wav_f32(mut w: impl Write, samples: &[f32], sample_rate: u32) -> io::Result<()> {
    let too_big = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "audio too large for a WAV file",
        )
    };
    if sample_rate == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "sample rate must not be zero",
        ));
    }
    let data_len = samples
        .len()
        .checked_mul(4)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(too_big)?;
    let riff_len = data_len.checked_add(36).ok_or_else(too_big)?;
    let byte_rate = sample_rate.checked_mul(4).ok_or_else(too_big)?;
    w.write_all(b"RIFF")?;
    w.write_all(&riff_len.to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    w.write_all(&3u16.to_le_bytes())?; // format: IEEE float
    w.write_all(&1u16.to_le_bytes())?; // channels
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&byte_rate.to_le_bytes())?; // byte rate
    w.write_all(&4u16.to_le_bytes())?; // block align
    w.write_all(&32u16.to_le_bytes())?; // bits per sample
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}
