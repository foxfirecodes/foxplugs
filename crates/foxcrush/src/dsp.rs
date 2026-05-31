#[derive(Default, Clone)]
pub struct BitcrushChannelState {
    held: f32,
    counter: f32,
}

impl BitcrushChannelState {
    pub fn reset(&mut self) {
        self.held = 0.0;
        self.counter = 0.0;
    }
}

pub fn process_sample(
    input: f32,
    bit_depth: f32,
    downsample: f32,
    state: &mut BitcrushChannelState,
) -> f32 {
    state.counter += 1.0;
    if state.counter >= downsample {
        state.counter -= downsample;
        state.held = input;
    }

    let steps = 2.0_f32.powf(bit_depth - 1.0);
    (state.held * steps).round() / steps
}
