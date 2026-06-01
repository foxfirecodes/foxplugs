use crate::dsp::{self, BitcrushChannelState};

const STEREO_CHANNELS: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct BitcrusherFrameParams {
    pub bit_depth: f32,
    pub downsample: f32,
    pub mix: f32,
    pub output_gain: f32,
}

impl BitcrusherFrameParams {
    #[inline]
    pub fn from_plain_values(bit_depth: f32, downsample: f32, mix: f32, output_gain: f32) -> Self {
        Self {
            bit_depth,
            downsample,
            mix,
            output_gain,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct QuantizationCache {
    bit_depth: f32,
    steps: f32,
}

impl Default for QuantizationCache {
    fn default() -> Self {
        Self {
            bit_depth: f32::NAN,
            steps: 0.0,
        }
    }
}

impl QuantizationCache {
    #[inline]
    fn steps_for(&mut self, bit_depth: f32) -> f32 {
        if bit_depth != self.bit_depth {
            self.bit_depth = bit_depth;
            self.steps = dsp::quantization_steps(bit_depth);
        }

        self.steps
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bitcrusher {
    channel_states: [BitcrushChannelState; STEREO_CHANNELS],
    quantization_cache: QuantizationCache,
}

impl Bitcrusher {
    pub const CHANNELS: usize = STEREO_CHANNELS;

    pub fn reset(&mut self) {
        for state in &mut self.channel_states {
            state.reset();
        }
        self.quantization_cache = QuantizationCache::default();
    }

    #[inline]
    pub fn process_frame<'a>(
        &mut self,
        samples: impl IntoIterator<Item = &'a mut f32>,
        params: BitcrusherFrameParams,
    ) {
        let quantization_steps = self.quantization_cache.steps_for(params.bit_depth);

        for (sample, state) in samples.into_iter().zip(self.channel_states.iter_mut()) {
            let dry = *sample;
            let crushed = dsp::process_sample(dry, params.downsample, quantization_steps, state);
            *sample = (dry * (1.0 - params.mix) + crushed * params.mix) * params.output_gain;
        }
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

    #[test]
    fn stereo_processor_has_fixed_channel_count() {
        assert_eq!(Bitcrusher::CHANNELS, 2);
        assert_eq!(Bitcrusher::default().channel_states.len(), 2);
    }

    #[test]
    fn silence_stays_silent() {
        let mut processor = Bitcrusher::default();
        let params = BitcrusherFrameParams::from_plain_values(8.0, 4.0, 1.0, 1.0);
        let mut frame = [0.0, 0.0];

        processor.process_frame(&mut frame, params);

        assert_close(frame[0], 0.0);
        assert_close(frame[1], 0.0);
    }

    #[test]
    fn mix_zero_outputs_dry_signal_with_output_gain() {
        let mut processor = Bitcrusher::default();
        let params = BitcrusherFrameParams::from_plain_values(2.0, 8.0, 0.0, 0.5);
        let mut frame = [0.25, -0.5];

        processor.process_frame(&mut frame, params);

        assert_close(frame[0], 0.125);
        assert_close(frame[1], -0.25);
    }

    #[test]
    fn low_bit_depth_level_compensation_controls_boost_but_keeps_shape_change() {
        let mut processor = Bitcrusher::default();
        let params = BitcrusherFrameParams::from_plain_values(2.0, 1.0, 1.0, 1.0);
        let mut input_abs_sum = 0.0;
        let mut output_abs_sum = 0.0;
        let mut difference_sum = 0.0;

        for n in 0..1000 {
            let input = (n as f32 * 0.1).sin() * 0.02;
            let mut frame = [input, -input];
            processor.process_frame(&mut frame, params);

            input_abs_sum += input.abs() * 2.0;
            output_abs_sum += frame[0].abs() + frame[1].abs();
            difference_sum += (frame[0] - input).abs() + (frame[1] + input).abs();
        }

        assert!(output_abs_sum <= input_abs_sum * 1.2);
        assert!(difference_sum > input_abs_sum * 0.2);
    }

    #[test]
    fn reset_captures_first_frame_after_reset() {
        let mut processor = Bitcrusher::default();
        let params = BitcrusherFrameParams::from_plain_values(16.0, 4.0, 1.0, 1.0);
        let expected_steps = dsp::quantization_steps(params.bit_depth);
        let mut frame = [0.5, -0.5];

        processor.process_frame(&mut frame, params);
        assert_close(frame[0], dsp::quantize(0.5, expected_steps));
        assert_close(frame[1], dsp::quantize(-0.5, expected_steps));

        processor.reset();
        let mut frame = [0.25, -0.25];
        processor.process_frame(&mut frame, params);

        assert_close(frame[0], dsp::quantize(0.25, expected_steps));
        assert_close(frame[1], dsp::quantize(-0.25, expected_steps));
    }

    #[test]
    fn quantization_cache_reuses_steps_until_bit_depth_changes() {
        let mut cache = QuantizationCache::default();

        let first = cache.steps_for(8.0);
        let cached = cache.steps_for(8.0);
        assert_close(first, dsp::quantization_steps(8.0));
        assert_close(cached, first);
        assert_eq!(cache.bit_depth, 8.0);

        let updated = cache.steps_for(8.5);
        assert_close(updated, dsp::quantization_steps(8.5));
        assert_eq!(cache.bit_depth, 8.5);
    }

    #[test]
    fn extra_channels_do_not_panic_or_get_processed() {
        let mut processor = Bitcrusher::default();
        let params = BitcrusherFrameParams::from_plain_values(2.0, 1.0, 1.0, 1.0);
        let mut frame = [0.3, 0.6, 0.9];

        processor.process_frame(&mut frame, params);

        assert_close(frame[0], 0.3);
        assert_close(frame[1], 0.5);
        assert_close(frame[2], 0.9);
    }
}
