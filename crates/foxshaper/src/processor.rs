use foxplugs_dsp::{dry_wet, lfo, STEREO_CHANNELS};

const DEFAULT_SAMPLE_RATE: f32 = 44_100.0;

#[derive(Clone, Copy, Debug)]
pub struct FoxshaperFrameParams {
    pub rate_hz: f32,
    pub depth: f32,
    pub shape: f32,
    pub phase_offset: f32,
    pub mix: f32,
    pub output_gain: f32,
}

impl FoxshaperFrameParams {
    #[inline]
    pub fn from_plain_values(
        rate_hz: f32,
        depth: f32,
        shape: f32,
        phase_offset: f32,
        mix: f32,
        output_gain: f32,
    ) -> Self {
        Self {
            rate_hz,
            depth,
            shape,
            phase_offset,
            mix,
            output_gain,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FoxshaperProcessor {
    sample_rate: f32,
    phase: f32,
}

impl Default for FoxshaperProcessor {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            phase: 0.0,
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
    }

    #[cfg(test)]
    #[inline]
    fn current_phase(&self) -> f32 {
        self.phase
    }

    #[inline]
    fn gain_for_phase(&self, params: FoxshaperFrameParams) -> f32 {
        let shaper_phase = lfo::wrap_unit_phase(self.phase + params.phase_offset);
        let envelope = lfo::skewed_triangle(shaper_phase, params.shape);
        1.0 - params.depth.clamp(0.0, 1.0) * envelope
    }

    #[inline]
    pub fn process_frame<'a>(
        &mut self,
        samples: impl IntoIterator<Item = &'a mut f32>,
        params: FoxshaperFrameParams,
    ) {
        let gain = self.gain_for_phase(params);

        for sample in samples.into_iter().take(Self::CHANNELS) {
            let dry = *sample;
            let wet = dry * gain;
            *sample = dry_wet(dry, wet, params.mix) * params.output_gain;
        }

        self.phase = lfo::advance_phase(self.phase, params.rate_hz, self.sample_rate);
    }
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

    fn params(depth: f32, phase_offset: f32, mix: f32) -> FoxshaperFrameParams {
        FoxshaperFrameParams::from_plain_values(1.0, depth, 0.5, phase_offset, mix, 1.0)
    }

    #[test]
    fn processor_has_stereo_channel_count() {
        assert_eq!(FoxshaperProcessor::CHANNELS, 2);
    }

    #[test]
    fn depth_zero_outputs_dry_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(0.0, 0.5, 1.0));

        assert_close(frame[0], 0.5);
        assert_close(frame[1], -0.25);
    }

    #[test]
    fn full_depth_at_envelope_peak_mutes_wet_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(1.0, 0.5, 1.0));

        assert_close(frame[0], 0.0);
        assert_close(frame[1], 0.0);
    }

    #[test]
    fn mix_blends_between_dry_and_shaped_signal() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];

        processor.process_frame(&mut frame, params(1.0, 0.5, 0.5));

        assert_close(frame[0], 0.25);
        assert_close(frame[1], -0.125);
    }

    #[test]
    fn output_gain_scales_after_mix() {
        let mut processor = FoxshaperProcessor::default();
        let mut frame = [0.5, -0.25];
        let mut params = params(0.0, 0.0, 1.0);
        params.output_gain = 2.0;

        processor.process_frame(&mut frame, params);

        assert_close(frame[0], 1.0);
        assert_close(frame[1], -0.5);
    }

    #[test]
    fn phase_advances_per_processed_frame() {
        let mut processor = FoxshaperProcessor::default();
        processor.set_sample_rate(4.0);
        let mut frame = [0.0, 0.0];

        processor.process_frame(&mut frame, params(0.0, 0.0, 1.0));

        assert_close(processor.current_phase(), 0.25);
    }

    #[test]
    fn reset_restarts_shape_phase() {
        let mut processor = FoxshaperProcessor::default();
        processor.set_sample_rate(4.0);
        let mut frame = [0.0, 0.0];

        processor.process_frame(&mut frame, params(0.0, 0.0, 1.0));
        processor.reset();

        assert_close(processor.current_phase(), 0.0);
    }
}
