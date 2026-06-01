use nih_plug::prelude::*;
use std::sync::Arc;

mod curve;
#[cfg(feature = "gui")]
mod editor;
mod params;
mod processor;

pub use params::FoxshaperParams;
use params::{LfoMode, LoopMode, TriggerMode};
use processor::{
    advance_beats_for_sample, sync_loop_beats, sync_phase_from_beats, sync_rate_hz,
    FoxshaperFrameParams, FoxshaperProcessor,
};

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

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
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
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let transport = context.transport();
        let tempo_bpm = transport.tempo.unwrap_or(120.0) as f32;
        let beats_per_bar = transport.time_sig_numerator.unwrap_or(4).max(1) as f32;
        let loop_beats = sync_loop_beats(
            self.params.sync_length.value(),
            self.params.sync_rhythm.value(),
            beats_per_bar,
        );
        let sync_rate = sync_rate_hz(tempo_bpm, loop_beats);
        let use_beat_sync = self.params.lfo_mode.value() == LfoMode::Beats;
        let trigger_mode = self.params.trigger_mode.value();
        let use_midi_trigger = trigger_mode == TriggerMode::MIDI;
        let use_one_shot = self.params.loop_mode.value() == LoopMode::OneShot;
        let end_marker = self.params.end_marker.value();
        let sample_rate = transport.sample_rate;
        let start_pos_beats = transport.pos_beats;
        let mut next_event = context.next_event();

        for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
            if use_midi_trigger {
                while let Some(event) = next_event {
                    if event.timing() > sample_idx as u32 {
                        break;
                    }

                    if let NoteEvent::NoteOn { velocity, .. } = event {
                        if velocity > 0.0 {
                            self.processor.trigger();
                        }
                    }

                    next_event = context.next_event();
                }
            }

            let rate_hz = if use_beat_sync {
                sync_rate
            } else {
                self.params.rate_hz.smoothed.next()
            };
            let frame_params = FoxshaperFrameParams::from_plain_values(
                rate_hz,
                self.params.depth.smoothed.next(),
                self.params.shape_preset.value(),
                self.params.shape.smoothed.next(),
                self.params.phase_offset.smoothed.next(),
                self.params.smooth.smoothed.next(),
                self.params.mix.smoothed.next(),
                self.params.trim.smoothed.next(),
                self.params.output_gain.smoothed.next(),
                self.params.custom_curve.snapshot(),
            );

            if use_midi_trigger && use_one_shot {
                self.processor.process_frame_one_shot(
                    channel_samples.iter_mut(),
                    frame_params,
                    end_marker,
                );
            } else if use_beat_sync && !use_midi_trigger {
                if let Some(start_pos_beats) = start_pos_beats {
                    let pos_beats = start_pos_beats
                        + advance_beats_for_sample(sample_idx, tempo_bpm, sample_rate);
                    let phase = sync_phase_from_beats(pos_beats, loop_beats);
                    self.processor.process_frame_at_phase(
                        channel_samples.iter_mut(),
                        frame_params,
                        phase,
                    );
                } else {
                    self.processor
                        .process_frame(channel_samples.iter_mut(), frame_params);
                }
            } else {
                self.processor
                    .process_frame(channel_samples.iter_mut(), frame_params);
            }
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
