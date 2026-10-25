//! # Kick Me
//!
//! A Rust port of Infinati's JUCE kick synth, the first Nusick plugin. Any MIDI
//! note triggers the kick and a new hit restarts it. All sound comes from
//! [`nusick_dsp::KickVoice`]; this crate only wires that voice to nih-plug
//! (parameters, MIDI, the egui editor) and exports it as CLAP and standalone.
//! VST3 export is opt-in through the `vst3-gpl` feature (see README).

mod editor;
mod params;

use nih_plug::prelude::*;
use nusick_dsp::KickVoice;
use params::KickMeParams;
use std::sync::{atomic::Ordering, Arc};

/// Peak meter falloff: the time for the meter to drop 12 dB in silence.
const METER_DECAY_MS: f32 = 300.0;

pub struct KickMe {
    params: Arc<KickMeParams>,
    voice: KickVoice,
    /// Output peak (linear gain), shared with the editor.
    peak: Arc<AtomicF32>,
    meter_decay: f32,
}

impl Default for KickMe {
    fn default() -> Self {
        let params = Arc::new(KickMeParams::default());
        let voice = KickVoice::new(44_100.0, &params.kick_params());
        Self {
            params,
            voice,
            peak: Arc::new(AtomicF32::new(0.0)),
            meter_decay: 1.0,
        }
    }
}

impl Plugin for KickMe {
    const NAME: &'static str = "Kick Me";
    const VENDOR: &'static str = "Infinati";
    const URL: &'static str = env!("CARGO_PKG_REPOSITORY");
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.peak.clone())
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Allocation is allowed here; it runs off the audio thread.
        self.voice = KickVoice::new(config.sample_rate, &self.params.kick_params());
        self.meter_decay = 0.25_f32.powf(1.0 / (config.sample_rate * METER_DECAY_MS / 1000.0));
        true
    }

    fn reset(&mut self) {
        self.voice.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Parameter targets are read once per block; the voice smooths them.
        self.voice.set_params(&self.params.kick_params());

        let mut next_event = context.next_event();
        let mut peak = self.peak.load(Ordering::Relaxed);
        for (offset, channels) in buffer.iter_samples().enumerate() {
            // Sample-accurate note triggering.
            while let Some(event) = next_event {
                if event.timing() > offset as u32 {
                    break;
                }
                if let NoteEvent::NoteOn { .. } = event {
                    self.voice.trigger();
                }
                next_event = context.next_event();
            }

            let sample = self.voice.next();
            for out in channels {
                *out = sample;
            }
            peak = (peak * self.meter_decay).max(sample.abs());
        }
        if self.params.editor_state.is_open() {
            self.peak.store(peak, Ordering::Relaxed);
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for KickMe {
    const CLAP_ID: &'static str = "com.infinati.kick-me";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Sine kick synth with pitch sweep, click and drive");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Drum,
        ClapFeature::Mono,
        ClapFeature::Stereo,
    ];
}

nih_export_clap!(KickMe);

#[cfg(feature = "vst3-gpl")]
mod vst3 {
    use super::KickMe;
    use nih_plug::prelude::*;

    impl Vst3Plugin for KickMe {
        const VST3_CLASS_ID: [u8; 16] = *b"InfinatiKickMe01";
        const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
            Vst3SubCategory::Instrument,
            Vst3SubCategory::Drum,
            Vst3SubCategory::Synth,
        ];
    }

    nih_export_vst3!(KickMe);
}
