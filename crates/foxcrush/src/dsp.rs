#[derive(Clone, Copy, Debug)]
pub struct BitcrushChannelState {
    held: f32,
    counter: f32,
    needs_sample: bool,
    input_level: f32,
    crushed_level: f32,
}

impl Default for BitcrushChannelState {
    fn default() -> Self {
        Self {
            held: 0.0,
            counter: 0.0,
            needs_sample: true,
            input_level: 0.0,
            crushed_level: 0.0,
        }
    }
}

impl BitcrushChannelState {
    pub fn reset(&mut self) {
        self.held = 0.0;
        self.counter = 0.0;
        self.needs_sample = true;
        self.input_level = 0.0;
        self.crushed_level = 0.0;
    }
}

#[inline]
pub fn quantization_steps(bit_depth: f32) -> f32 {
    2.0_f32.powf(bit_depth - 1.0)
}

#[inline]
pub fn quantize(input: f32, steps: f32) -> f32 {
    (input * steps).round() / steps
}

#[inline]
fn quantize_nonzero(input: f32, steps: f32) -> f32 {
    let quantized = quantize(input, steps);

    if quantized == 0.0 && input != 0.0 {
        input.signum() * (0.5 / steps)
    } else {
        quantized
    }
}

const LEVEL_COMPENSATION_SMOOTHING: f32 = 0.01;
const LEVEL_COMPENSATION_EPSILON: f32 = 0.000_001;

#[inline]
fn smooth_level(current: f32, target: f32) -> f32 {
    current + (target - current) * LEVEL_COMPENSATION_SMOOTHING
}

#[inline]
fn compensate_upward_level_boost(
    input: f32,
    crushed: f32,
    state: &mut BitcrushChannelState,
) -> f32 {
    state.input_level = smooth_level(state.input_level, input.abs());
    state.crushed_level = smooth_level(state.crushed_level, crushed.abs());

    if state.crushed_level <= LEVEL_COMPENSATION_EPSILON {
        return crushed;
    }

    let gain = (state.input_level / state.crushed_level).min(1.0);
    crushed * gain
}

#[inline]
fn quantize_with_level_compensation(
    input: f32,
    steps: f32,
    state: &mut BitcrushChannelState,
) -> f32 {
    let crushed = quantize_nonzero(input, steps);
    compensate_upward_level_boost(input, crushed, state)
}

/// Process one sample through the sample-and-hold and bit-depth quantizer.
///
/// `downsample` is intentionally continuous. Fractional values create a fractional
/// sample-and-hold phase pattern instead of being rounded to integer factors.
#[inline]
pub fn process_sample(
    input: f32,
    downsample: f32,
    quantization_steps: f32,
    state: &mut BitcrushChannelState,
) -> f32 {
    let downsample = downsample.max(1.0);

    if state.needs_sample {
        state.held = input;
        // Treat the first captured sample as consumed by the sample-and-hold phase.
        // This avoids reset/startup silence while preserving the pre-refactor capture
        // cadence for subsequent samples.
        state.counter = if downsample > 1.0 { 1.0 } else { 0.0 };
        state.needs_sample = false;
    } else {
        state.counter += 1.0;
        if state.counter >= downsample {
            state.counter -= downsample;
            state.held = input;
        }
    }

    quantize_with_level_compensation(state.held, quantization_steps, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_1;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn quantizes_positive_and_negative_samples_at_known_bit_depth() {
        let steps = quantization_steps(3.0);

        assert_close(steps, 4.0);
        assert_close(quantize(0.62, steps), 0.5);
        assert_close(quantize(0.63, steps), 0.75);
        assert_close(quantize(-0.62, steps), -0.5);
        assert_close(quantize(-0.63, steps), -0.75);
    }

    #[test]
    fn continuous_bit_depth_values_are_preserved() {
        let steps = quantization_steps(2.5);
        assert_close(steps, 2.0_f32.powf(1.5));
    }

    #[test]
    fn pure_quantizer_has_expected_zero_deadband() {
        let steps = quantization_steps(5.0);

        assert_close(steps, 16.0);
        assert_close(quantize(0.02, steps), 0.0);
        assert_close(quantize(-0.02, steps), 0.0);
        assert_close(quantize(0.0, steps), 0.0);
    }

    #[test]
    fn level_compensation_controls_boost_without_erasing_crush_shape() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(2.0);
        let mut input_abs_sum = 0.0;
        let mut output_abs_sum = 0.0;
        let mut difference_sum = 0.0;

        for n in 0..1000 {
            let phase = n as f32 * 0.1;
            let input = phase.sin() * 0.02;
            let output = process_sample(input, 1.0, steps, &mut state);

            input_abs_sum += input.abs();
            output_abs_sum += output.abs();
            difference_sum += (output - input).abs();
        }

        assert!(
            output_abs_sum <= input_abs_sum * 1.2,
            "expected compensated output level near input level, got input abs sum {input_abs_sum}, output abs sum {output_abs_sum}"
        );
        assert!(
            difference_sum > input_abs_sum * 0.2,
            "expected audible shape change, got input abs sum {input_abs_sum}, difference sum {difference_sum}"
        );
    }

    #[test]
    fn downsample_one_captures_every_sample() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(16.0);

        assert_close(
            process_sample(0.1, 1.0, steps, &mut state),
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.2, 1.0, steps, &mut state),
            quantize(0.2, steps),
        );
        assert_close(
            process_sample(0.3, 1.0, steps, &mut state),
            quantize(0.3, steps),
        );
    }

    #[test]
    fn downsample_greater_than_one_holds_between_captures() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(16.0);

        assert_close(
            process_sample(0.1, 3.0, steps, &mut state),
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.2, 3.0, steps, &mut state),
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.3, 3.0, steps, &mut state),
            quantize(0.3, steps),
        );
        assert_close(
            process_sample(0.4, 3.0, steps, &mut state),
            quantize(0.3, steps),
        );
    }

    #[test]
    fn fractional_downsample_uses_fractional_phase() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(16.0);

        assert_close(
            process_sample(0.1, 1.5, steps, &mut state),
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.2, 1.5, steps, &mut state),
            quantize(0.2, steps),
        );
        assert_close(
            process_sample(0.3, 1.5, steps, &mut state),
            quantize(0.3, steps),
        );
        assert_close(
            process_sample(0.4, 1.5, steps, &mut state),
            quantize(0.3, steps),
        );
    }

    #[test]
    fn startup_capture_does_not_lock_phase_to_zero_crossings() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(5.0);
        let inputs = [0.0, 0.02, 0.02, 0.7, 0.0, -0.02, -0.02, -0.7];
        let outputs = inputs.map(|input| process_sample(input, 4.0, steps, &mut state));

        assert_close(outputs[0], 0.0);
        assert_close(outputs[1], 0.0);
        assert_close(outputs[2], 0.0);
        assert!(outputs[3].abs() > 0.5);
        assert!(outputs[4].abs() > 0.5);
        assert!(outputs[5].abs() > 0.5);
        assert!(outputs[6].abs() > 0.5);
        assert!(outputs[7] < -0.5);
    }

    #[test]
    fn reset_makes_next_sample_capture_immediately() {
        let mut state = BitcrushChannelState::default();
        let steps = quantization_steps(16.0);

        assert_close(
            process_sample(0.5, 4.0, steps, &mut state),
            quantize(0.5, steps),
        );
        assert_close(
            process_sample(0.1, 4.0, steps, &mut state),
            quantize(0.5, steps),
        );

        state.reset();

        assert_close(
            process_sample(0.25, 4.0, steps, &mut state),
            quantize(0.25, steps),
        );
    }
}
