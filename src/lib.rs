use nih_plug::prelude::*;
use std::sync::Arc;
use vizia_plug::ViziaState;

mod dsp;
mod editor;
mod widgets;

use dsp::BitcrushChannelState;

pub struct Foxcrush {
    params: Arc<FoxcrushParams>,
    channel_states: Vec<BitcrushChannelState>,
}

#[derive(Params)]
pub struct FoxcrushParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "bit_depth"]
    pub bit_depth: FloatParam,

    #[id = "downsample"]
    pub downsample: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,
}

impl Default for Foxcrush {
    fn default() -> Self {
        Self {
            params: Arc::new(FoxcrushParams::default()),
            channel_states: Vec::new(),
        }
    }
}

impl Default for FoxcrushParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            bit_depth: FloatParam::new(
                "Bit Depth",
                16.0,
                FloatRange::Linear {
                    min: 2.0,
                    max: 16.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit(" bits")
            .with_value_to_string(smart_rounded()),

            downsample: FloatParam::new(
                "Downsample",
                1.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 50.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("x")
            .with_value_to_string(smart_rounded()),

            mix: FloatParam::new(
                "Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            output_gain: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-12.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-12.0, 12.0),
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

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let channels = audio_io_layout
            .main_output_channels
            .map(|n| n.get() as usize)
            .unwrap_or(2);
        self.channel_states = vec![BitcrushChannelState::default(); channels];
        true
    }

    fn reset(&mut self) {
        for state in &mut self.channel_states {
            state.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for mut channel_samples in buffer.iter_samples() {
            let bit_depth = self.params.bit_depth.smoothed.next();
            let downsample = self.params.downsample.smoothed.next();
            let mix = self.params.mix.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();

            for (ch_idx, sample) in channel_samples.iter_mut().enumerate() {
                let dry = *sample;
                let crushed = dsp::process_sample(
                    dry,
                    bit_depth,
                    downsample,
                    &mut self.channel_states[ch_idx],
                );
                *sample = (dry * (1.0 - mix) + crushed * mix) * output_gain;
            }
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

fn smart_rounded() -> std::sync::Arc<dyn Fn(f32) -> String + Send + Sync> {
    std::sync::Arc::new(|v: f32| {
        if (v - v.round()).abs() < 0.001 {
            format!("{}", v.round() as i32)
        } else {
            format!("{:.2}", v)
        }
    })
}

nih_export_clap!(Foxcrush);
nih_export_vst3!(Foxcrush);
