use nih_plug::prelude::*;
use std::sync::Arc;

mod dsp;
#[cfg(feature = "gui")]
mod editor;
mod params;
mod processor;

pub use params::FoxcrushParams;
use processor::{Bitcrusher, BitcrusherFrameParams};

#[derive(Default)]
pub struct Foxcrush {
    params: Arc<FoxcrushParams>,
    processor: Bitcrusher,
}

impl Plugin for Foxcrush {
    const NAME: &'static str = "Foxcrush";
    const VENDOR: &'static str = "foxfire";
    const URL: &'static str = "https://github.com/foxfire/foxplugs";
    const EMAIL: &'static str = "rayzr522@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(Bitcrusher::CHANNELS as u32),
        main_output_channels: NonZeroU32::new(Bitcrusher::CHANNELS as u32),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    #[cfg(feature = "gui")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    #[cfg(not(feature = "gui"))]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn reset(&mut self) {
        self.processor.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for mut channel_samples in buffer.iter_samples() {
            let frame_params = BitcrusherFrameParams::from_plain_values(
                self.params.bit_depth.smoothed.next(),
                self.params.downsample.smoothed.next(),
                self.params.mix.smoothed.next(),
                self.params.output_gain.smoothed.next(),
            );

            self.processor
                .process_frame(channel_samples.iter_mut(), frame_params);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Foxcrush {
    const CLAP_ID: &'static str = "dev.foxfire.foxcrush";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A bitcrusher audio effect inspired by Ableton Redux");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for Foxcrush {
    const VST3_CLASS_ID: [u8; 16] = *b"FoxcrushBitCrush";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nih_export_clap!(Foxcrush);
nih_export_vst3!(Foxcrush);
