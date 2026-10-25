//! Standalone Infinati Kick. Run with `--help` to see audio/MIDI backend options.

fn main() {
    nih_plug::prelude::nih_export_standalone::<infinati_kick::InfinatiKick>();
}
