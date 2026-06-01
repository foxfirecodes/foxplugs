use foxplugs_plugin::params;
use nih_plug::prelude::*;

#[cfg(feature = "gui")]
use std::sync::Arc;
#[cfg(feature = "gui")]
use vizia_plug::ViziaState;

const RATE_MIN_HZ: f32 = 0.05;
const RATE_MAX_HZ: f32 = 20.0;

#[derive(Params)]
pub struct FoxshaperParams {
    #[cfg(feature = "gui")]
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,

    #[id = "rate_hz"]
    pub rate_hz: FloatParam,

    #[id = "depth"]
    pub depth: FloatParam,

    #[id = "shape"]
    pub shape: FloatParam,

    #[id = "phase_offset"]
    pub phase_offset: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,
}

impl Default for FoxshaperParams {
    fn default() -> Self {
        Self {
            #[cfg(feature = "gui")]
            editor_state: crate::editor::default_state(),

            rate_hz: params::hz_param("Rate", 1.0, RATE_MIN_HZ, RATE_MAX_HZ),
            depth: params::percentage_param("Depth", 1.0),
            shape: params::percentage_param("Shape", 0.5),
            phase_offset: params::percentage_param("Phase", 0.0),
            mix: params::mix_param(1.0),
            output_gain: params::output_gain_param(0.0),
        }
    }
}
