#[derive(Clone, Copy, Debug)]
pub struct BitcrushChannelState {
    held: f32,
    counter: f32,
    needs_sample: bool,
}

impl Default for BitcrushChannelState {
    fn default() -> Self {
        Self {
            held: 0.0,
            counter: 0.0,
            needs_sample: true,
        }
    }
}

impl BitcrushChannelState {
    pub fn reset(&mut self) {
        self.held = 0.0;
        self.counter = 0.0;
        self.needs_sample = true;
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
        state.counter = 0.0;
        state.needs_sample = false;
    } else {
        state.counter += 1.0;
        if state.counter >= downsample {
            state.counter -= downsample;
            state.held = input;
        }
    }

    quantize(state.held, quantization_steps)
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
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.4, 3.0, steps, &mut state),
            quantize(0.4, steps),
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
            quantize(0.1, steps),
        );
        assert_close(
            process_sample(0.3, 1.5, steps, &mut state),
            quantize(0.3, steps),
        );
        assert_close(
            process_sample(0.4, 1.5, steps, &mut state),
            quantize(0.4, steps),
        );
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
