//! Standalone Kick Me. Run with `--help` to see audio/MIDI backend options.

fn main() {
    nih_plug::prelude::nih_export_standalone::<kick_me::KickMe>();
}
