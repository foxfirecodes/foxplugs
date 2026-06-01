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
    advance_beats_for_sample, midi_note_is_trigger, midi_note_to_wave_slot, sync_loop_beats,
    sync_phase_from_beats, sync_rate_hz, AudioTriggerDetector, FoxshaperFrameParams,
    FoxshaperProcessor,
};

#[derive(Default)]
pub struct Foxshaper {
    params: Arc<FoxshaperParams>,
    processor: FoxshaperProcessor,
    audio_trigger: AudioTriggerDetector,
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
        aux_input_ports: &[new_nonzero_u32(FoxshaperProcessor::CHANNELS as u32)],
        names: PortNames {
            layout: Some("Stereo with Sidechain"),
            main_input: Some("Input"),
            main_output: Some("Output"),
            aux_inputs: &["Sidechain"],
            aux_outputs: &[],
        },
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
        self.audio_trigger.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
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
        let use_audio_trigger = trigger_mode == TriggerMode::Audio;
        let use_one_shot = self.params.loop_mode.value() == LoopMode::OneShot;
        let midi_switch = self.params.midi_switch.value();
        let end_marker = self.params.end_marker.value();
        let audio_sidechain = self.params.audio_sidechain.value();
        let sample_rate = transport.sample_rate;
        let start_pos_beats = transport.pos_beats;
        let sidechain_inputs = if audio_sidechain {
            aux.inputs.first().map(|input| input.as_slice_immutable())
        } else {
            None
        };
        let mut next_event = if use_midi_trigger {
            context.next_event()
        } else {
            None
        };

        for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
            if use_midi_trigger {
                while let Some(event) = next_event {
                    if event.timing() > sample_idx as u32 {
                        break;
                    }

                    if let NoteEvent::NoteOn { note, velocity, .. } = event {
                        if velocity > 0.0 {
                            if midi_switch {
                                if let Some(slot) = midi_note_to_wave_slot(note) {
                                    self.processor.set_wave_slot(slot);
                                } else if midi_note_is_trigger(note) {
                                    self.processor.trigger();
                                }
                            } else {
                                self.processor.trigger();
                            }
                        }
                    }

                    next_event = context.next_event();
                }
            }

            if use_audio_trigger {
                let detector_input = match sidechain_inputs {
                    Some(sidechain_inputs) => {
                        sidechain_inputs
                            .iter()
                            .take(FoxshaperProcessor::CHANNELS)
                            .filter_map(|channel| channel.get(sample_idx).copied())
                            .sum::<f32>()
                            / FoxshaperProcessor::CHANNELS as f32
                    }
                    None => {
                        channel_samples
                            .iter_mut()
                            .take(FoxshaperProcessor::CHANNELS)
                            .map(|sample| *sample)
                            .sum::<f32>()
                            / FoxshaperProcessor::CHANNELS as f32
                    }
                };
                if self.audio_trigger.detect_filtered(
                    detector_input,
                    self.params.audio_threshold.smoothed.next(),
                    sample_rate,
                    self.params.audio_low_cut_hz.smoothed.next(),
                    self.params.audio_high_cut_hz.smoothed.next(),
                    self.params.audio_detail.smoothed.next(),
                ) {
                    self.processor.trigger();
                }
            }

            let rate_hz = if use_beat_sync {
                sync_rate
            } else {
                self.params.rate_hz.smoothed.next()
            };
            let shape_preset = if midi_switch {
                self.processor.active_wave_shape()
            } else {
                self.params.shape_preset.value()
            };
            let frame_params = FoxshaperFrameParams::from_plain_values(
                rate_hz,
                self.params.depth.smoothed.next(),
                shape_preset,
                self.params.shape.smoothed.next(),
                self.params.phase_offset.smoothed.next(),
                self.params.smooth.smoothed.next(),
                self.params.mix.smoothed.next(),
                self.params.trim.smoothed.next(),
                self.params.output_gain.smoothed.next(),
                self.params.custom_curve.snapshot(),
            );

            if (use_midi_trigger || use_audio_trigger) && use_one_shot {
                self.processor.process_frame_one_shot(
                    channel_samples.iter_mut(),
                    frame_params,
                    end_marker,
                );
            } else if use_beat_sync && !use_midi_trigger && !use_audio_trigger {
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
