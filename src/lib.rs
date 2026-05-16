use nih_plug::prelude::*;
use std::sync::Arc;
use vizia_plug::ViziaState;

mod editor;

pub struct Foxcrush {
    params: Arc<FoxcrushParams>,
}

#[derive(Params)]
pub struct FoxcrushParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for Foxcrush {
    fn default() -> Self {
        Self {
            params: Arc::new(FoxcrushParams::default()),
        }
    }
}

impl Default for FoxcrushParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for Foxcrush {
    const NAME: &'static str = "Foxcrush";
    const VENDOR: &'static str = "foxfire";
    const URL: &'static str = "https://github.com/foxfire/foxcrush";
    const EMAIL: &'static str = "rayzr522@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Foxcrush {
    const CLAP_ID: &'static str = "dev.foxfire.foxcrush";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A bitcrusher audio effect plugin (placeholder gain stage)");
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
