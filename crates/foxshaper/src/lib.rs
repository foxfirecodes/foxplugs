use nih_plug::prelude::*;
use std::sync::Arc;

#[cfg(feature = "gui")]
mod editor;
mod params;
mod processor;

pub use params::FoxshaperParams;
use processor::{FoxshaperFrameParams, FoxshaperProcessor};

#[derive(Default)]
pub struct Foxshaper {
    params: Arc<FoxshaperParams>,
    processor: FoxshaperProcessor,
}

impl Plugin for Foxshaper {
    const NAME: &'static str = "Foxshaper";
    const VENDOR: &'static str = foxplugs_plugin::metadata::VENDOR;
    const URL: &'static str = foxplugs_plugin::metadata::URL;
    const EMAIL: &'static str = foxplugs_plugin::metadata::EMAIL;
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(FoxshaperProcessor::CHANNELS as u32),
        main_output_channels: NonZeroU32::new(FoxshaperProcessor::CHANNELS as u32),
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

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.processor.set_sample_rate(buffer_config.sample_rate);
        true
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
            let frame_params = FoxshaperFrameParams::from_plain_values(
                self.params.rate_hz.smoothed.next(),
                self.params.depth.smoothed.next(),
                self.params.shape_preset.value(),
                self.params.shape.smoothed.next(),
                self.params.phase_offset.smoothed.next(),
                self.params.smooth.smoothed.next(),
                self.params.mix.smoothed.next(),
                self.params.trim.smoothed.next(),
                self.params.output_gain.smoothed.next(),
            );

            self.processor
                .process_frame(channel_samples.iter_mut(), frame_params);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Foxshaper {
    const CLAP_ID: &'static str = "dev.foxfire.foxshaper";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A Foxplugs multi-shaper scaffold with tempo-style volume shaping as its first lane");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Tremolo,
        ClapFeature::MultiEffects,
    ];
}

impl Vst3Plugin for Foxshaper {
    const VST3_CLASS_ID: [u8; 16] = *b"FoxshaperPlugin!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Foxshaper);
nih_export_vst3!(Foxshaper);
