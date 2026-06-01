use crate::curve::{Curve, DEFAULT_VOLUME_CURVE};
use crate::params::{ShapePreset, SyncLength, SyncRhythm};
use foxplugs_dsp::{dry_wet, lerp, lfo, STEREO_CHANNELS};

const DEFAULT_SAMPLE_RATE: f32 = 44_100.0;
const MAX_SMOOTHING_MS: f32 = 50.0;

#[derive(Clone, Copy, Debug)]
pub struct FoxshaperFrameParams {
    pub rate_hz: f32,
    pub depth: f32,
    pub shape_preset: ShapePreset,
    pub shape: f32,
    pub phase_offset: f32,
    pub smooth: f32,
    pub mix: f32,
    pub trim_gain: f32,
    pub output_gain: f32,
    pub custom_curve: Curve,
}

impl FoxshaperFrameParams {
    #[inline]
    pub fn from_plain_values(
        rate_hz: f32,
        depth: f32,
        shape_preset: ShapePreset,
        shape: f32,
        phase_offset: f32,
        smooth: f32,
        mix: f32,
        trim_gain: f32,
        output_gain: f32,
        custom_curve: Curve,
    ) -> Self {
        Self {
            rate_hz,
            depth,
            shape_preset,
            shape,
            phase_offset,
            smooth,
            mix,
            trim_gain,
            output_gain,
            custom_curve,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FoxshaperProcessor {
    sample_rate: f32,
    phase: f32,
    smoothed_gain: f32,
    gain_initialized: bool,
}

impl Default for FoxshaperProcessor {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            phase: 0.0,
            smoothed_gain: 1.0,
            gain_initialized: false,
        }
    }
}

impl FoxshaperProcessor {
    pub const CHANNELS: usize = STEREO_CHANNELS;

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.smoothed_gain = 1.0;
        self.gain_initialized = false;
    }

    #[cfg(test)]
    #[inline]
    fn current_phase(&self) -> f32 {
        self.phase
    }

    #[inline]
    fn target_gain_for_phase(&self, phase: f32, params: FoxshaperFrameParams) -> f32 {
        let shaper_phase = lfo::wrap_unit_phase(phase + params.phase_offset);
        let curve = if params.shape_preset == ShapePreset::Custom {
            params.custom_curve.evaluate(shaper_phase)
        } else {
            evaluate_shape(shaper_phase, params.shape_preset, params.shape)
        };
        let depth_curve = lerp(1.0, curve, params.depth.clamp(0.0, 1.0));
        volume_curve_to_gain(depth_curve)
    }

    #[inline]
    fn next_gain(&mut self, target_gain: f32, smooth: f32) -> f32 {
        let smooth = smooth.clamp(0.0, 1.0);

        if !self.gain_initialized || smooth <= 0.0 {
            self.smoothed_gain = target_gain;
            self.gain_initialized = true;
            return target_gain;
        }

        let smoothing_samples = (smooth * MAX_SMOOTHING_MS * 0.001 * self.sample_rate).max(1.0);
        let coefficient = 1.0 / smoothing_samples;
        self.smoothed_gain += (target_gain - self.smoothed_gain) * coefficient;
        self.smoothed_gain
    }

    #[inline]
    pub fn process_frame<'a>(
        &mut self,
        samples: impl IntoIterator<Item = &'a mut f32>,
        params: FoxshaperFrameParams,
    ) {
        self.process_frame_at_phase(samples, params, self.phase);
        self.phase = lfo::advance_phase(self.phase, params.rate_hz, self.sample_rate);
    }

    #[inline]
    pub fn process_frame_at_phase<'a>(
        &mut self,
        samples: impl IntoIterator<Item = &'a mut f32>,
        params: FoxshaperFrameParams,
        phase: f32,
    ) {
        let target_gain = self.target_gain_for_phase(phase, params);
        let shaped_gain = self.next_gain(target_gain, params.smooth);
        let post_gain = params.trim_gain * params.output_gain;

        for sample in samples.into_iter().take(Self::CHANNELS) {
            let dry = *sample;
            let wet = dry * shaped_gain;
            *sample = dry_wet(dry, wet, params.mix) * post_gain;
        }
    }
}

#[inline]
pub(crate) fn sync_loop_beats(length: SyncLength, rhythm: SyncRhythm, beats_per_bar: f32) -> f32 {
    let beats_per_bar = beats_per_bar.max(1.0);
    (length.beats(beats_per_bar) * rhythm.multiplier()).max(0.001)
}

#[inline]
pub(crate) fn sync_rate_hz(tempo_bpm: f32, loop_beats: f32) -> f32 {
    (tempo_bpm.max(1.0) / 60.0) / loop_beats.max(0.001)
}

#[inline]
pub(crate) fn sync_phase_from_beats(pos_beats: f64, loop_beats: f32) -> f32 {
    lfo::wrap_unit_phase((pos_beats / loop_beats.max(0.001) as f64) as f32)
}

#[inline]
pub(crate) fn advance_beats_for_sample(
    sample_index: usize,
    tempo_bpm: f32,
    sample_rate: f32,
) -> f64 {
    let beats_per_sample = tempo_bpm.max(1.0) as f64 / (60.0 * sample_rate.max(1.0) as f64);
    sample_index as f64 * beats_per_sample
}

#[inline]
pub(crate) fn evaluate_shape(phase: f32, preset: ShapePreset, shape: f32) -> f32 {
    let phase = lfo::wrap_unit_phase(phase);
    let shape = shape.clamp(0.0, 1.0);

    match preset {
        ShapePreset::Sidechain => sidechain_curve(phase, shape),
        ShapePreset::RampUp => phase,
        ShapePreset::RampDown => 1.0 - phase,
        ShapePreset::Gate => gate_curve(phase, shape),
        ShapePreset::Sine => 1.0 - lfo::unipolar_sine(phase),
        ShapePreset::Triangle => 1.0 - lfo::skewed_triangle(phase, skew_to_peak(shape)),
        ShapePreset::Custom => DEFAULT_VOLUME_CURVE.evaluate(phase),
    }
    .clamp(0.0, 1.0)
}

#[inline]
fn sidechain_curve(phase: f32, shape: f32) -> f32 {
    let release_end = lerp(0.08, 0.95, shape);
    if phase >= release_end {
        1.0
    } else {
        smoothstep(phase / release_end)
    }
}

#[inline]
fn gate_curve(phase: f32, shape: f32) -> f32 {
    let duty = lerp(0.05, 0.95, shape);
    if phase < duty {
        1.0
    } else {
        0.0
    }
}

#[inline]
fn skew_to_peak(shape: f32) -> f32 {
    lerp(0.05, 0.95, shape)
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Map VolumeShaper-style normalized volume to linear gain.
///
/// The graph is displayed as a percentage, but the useful audible mapping is in
/// gain: 1.0 is unity, 0.5 is approximately -6 dB, and 0.0 is silence.
#[inline]
pub(crate) fn volume_curve_to_gain(curve: f32) -> f32 {
    curve.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_001;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn params(
        shape_preset: ShapePreset,
        depth: f32,
        phase_offset: f32,
        mix: f32,
    ) -> FoxshaperFrameParams {
        FoxshaperFrameParams::from_plain_values(
            1.0,
            depth,
            shape_preset,
            0.5,
            phase_offset,
            0.0,
            mix,
            1.0,
            1.0,
            DEFAULT_VOLUME_CURVE,
        )
    }

    #[test]
    fn processor_has_stereo_channel_count() {
        assert_eq!(FoxshaperProcessor::CHANNELS, 2);
    }

    #[test]
    fn volume_curve_mapping_matches_volume_shaper_anchor_points() {
        assert_close(volume_curve_to_gain(1.0), 1.0);
        assert_close(volume_curve_to_gain(0.5), 0.5);
        assert_close(volume_curve_to_gain(0.0), 0.0);
        assert_close(volume_curve_to_gain(-1.0), 0.0);
        assert_close(volume_curve_to_gain(2.0), 1.0);
    }

    #[test]
    fn sync_timing_converts_musical_lengths_to_beats_and_hz() {
        let one_bar = sync_loop_beats(SyncLength::OneBar, SyncRhythm::Straight, 4.0);
        let dotted_eighth = sync_loop_beats(SyncLength::Eighth, SyncRhythm::Dotted, 4.0);
        let quarter_triplet = sync_loop_beats(SyncLength::Quarter, SyncRhythm::Triplet, 4.0);

        assert_close(one_bar, 4.0);
        assert_close(dotted_eighth, 0.75);
        assert_close(quarter_triplet, 2.0 / 3.0);
        assert_close(sync_rate_hz(120.0, one_bar), 0.5);
    }

    #[test]
    fn sync_phase_uses_absolute_beat_position() {
        assert_close(sync_phase_from_beats(0.0, 4.0), 0.0);
        assert_close(sync_phase_from_beats(1.0, 4.0), 0.25);
        assert_close(sync_phase_from_beats(4.0, 4.0), 0.0);
        assert_close(sync_phase_from_beats(5.0, 4.0), 0.25);
        assert_close(
            advance_beats_for_sample(24_000, 120.0, 48_000.0) as f32,
            1.0,
        );
    }

    #[test]
    fn process_frame_at_phase_does_not_advance_free_running_phase() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [1.0, 1.0];

        processor.process_frame_at_phase(
            &mut frame,
            params(ShapePreset::Sidechain, 1.0, 0.0, 1.0),
            0.0,
        );

        assert_close(processor.current_phase(), 0.0);
        assert_close(frame[0], 0.0);
    }

    #[test]
    fn evaluates_shape_presets_at_known_phases() {
        assert_close(evaluate_shape(0.0, ShapePreset::Sidechain, 0.5), 0.0);
        assert_close(evaluate_shape(0.75, ShapePreset::Sidechain, 0.5), 1.0);
        assert_close(evaluate_shape(0.25, ShapePreset::RampUp, 0.5), 0.25);
        assert_close(evaluate_shape(0.25, ShapePreset::RampDown, 0.5), 0.75);
        assert_close(evaluate_shape(0.25, ShapePreset::Gate, 0.5), 1.0);
        assert_close(evaluate_shape(0.75, ShapePreset::Gate, 0.5), 0.0);
        assert_close(evaluate_shape(0.0, ShapePreset::Sine, 0.5), 1.0);
        assert_close(evaluate_shape(0.5, ShapePreset::Sine, 0.5), 0.0);
        assert_close(evaluate_shape(0.0, ShapePreset::Triangle, 0.5), 1.0);
        assert_close(evaluate_shape(0.5, ShapePreset::Triangle, 0.5), 0.0);
        assert_close(evaluate_shape(0.0, ShapePreset::Custom, 0.5), 0.0);
        assert_close(evaluate_shape(0.5, ShapePreset::Custom, 0.5), 1.0);
    }

    #[test]
    fn depth_zero_outputs_dry_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 0.0, 0.0, 1.0));

        assert_close(frame[0], 0.5);
        assert_close(frame[1], -0.25);
    }

    #[test]
    fn full_depth_at_curve_bottom_mutes_wet_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 1.0, 0.0, 1.0));

        assert_close(frame[0], 0.0);
        assert_close(frame[1], 0.0);
    }

    #[test]
    fn mix_blends_between_dry_and_shaped_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 1.0, 0.0, 0.5));

        assert_close(frame[0], 0.25);
        assert_close(frame[1], -0.125);
    }

    #[test]
    fn trim_and_output_gain_scale_after_mix() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];
        let mut params = params(ShapePreset::Sidechain, 0.0, 0.0, 1.0);
        params.trim_gain = 2.0;
        params.output_gain = 0.5;

        processor.process_frame(&mut frame, params);

        assert_close(frame[0], 0.5);
        assert_close(frame[1], -0.25);
    }

    #[test]
    fn smoothing_moves_toward_gain_target_after_first_frame() {
        let mut processor = FoxshaperProcessor::default();
        processor.set_sample_rate(1_000.0);
        let mut frame = [1.0, 1.0];
        let mut params = params(ShapePreset::RampDown, 1.0, 0.0, 1.0);
        params.smooth = 1.0;

        processor.process_frame(&mut frame, params);
        assert_close(frame[0], 1.0);

        params.phase_offset = 0.5;
        processor.process_frame(&mut frame, params);

        assert!(frame[0] < 1.0);
        assert!(frame[0] > 0.5);
    }

    #[test]
    fn phase_advances_per_processed_frame() {
        let mut processor = FoxshaperProcessor::default();
        processor.set_sample_rate(4.0);
        let mut frame = [0.0, 0.0];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 0.0, 0.0, 1.0));

        assert_close(processor.current_phase(), 0.25);
    }

    #[test]
    fn reset_restarts_shape_phase_and_gain_smoothing() {
        let mut processor = FoxshaperProcessor::default();
        processor.set_sample_rate(4.0);
        let mut frame = [0.0, 0.0];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 0.0, 0.0, 1.0));
        processor.reset();

        assert_close(processor.current_phase(), 0.0);
        assert!(!processor.gain_initialized);
    }

    #[test]
    fn extra_channels_do_not_get_processed() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [1.0, 1.0, 1.0];

        processor.process_frame(&mut frame, params(ShapePreset::Sidechain, 1.0, 0.0, 1.0));

        assert_close(frame[0], 0.0);
        assert_close(frame[1], 0.0);
        assert_close(frame[2], 1.0);
    }
}
