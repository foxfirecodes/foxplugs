use nih_plug::prelude::*;
use std::sync::Arc;

#[cfg(feature = "gui")]
use vizia_plug::ViziaState;

const CONTROL_SMOOTHING_MS: f32 = 20.0;
const OUTPUT_GAIN_SMOOTHING_MS: f32 = 50.0;
const OUTPUT_GAIN_MIN_DB: f32 = -12.0;
const OUTPUT_GAIN_MAX_DB: f32 = 12.0;

#[derive(Params)]
pub struct FoxcrushParams {
    #[cfg(feature = "gui")]
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,

    #[id = "bit_depth"]
    pub bit_depth: FloatParam,

    #[id = "downsample"]
    pub downsample: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,
}

impl Default for FoxcrushParams {
    fn default() -> Self {
        Self {
            #[cfg(feature = "gui")]
            editor_state: crate::editor::default_state(),

            bit_depth: continuous_bit_depth_param(),
            downsample: continuous_downsample_param(),
            mix: mix_param(1.0),
            output_gain: output_gain_param(0.0, OUTPUT_GAIN_MIN_DB, OUTPUT_GAIN_MAX_DB),
        }
    }
}

fn continuous_bit_depth_param() -> FloatParam {
    FloatParam::new(
        "Bit Depth",
        16.0,
        FloatRange::Linear {
            min: 2.0,
            max: 16.0,
        },
    )
    .with_smoother(SmoothingStyle::Linear(CONTROL_SMOOTHING_MS))
    .with_unit(" bits")
    .with_value_to_string(smart_rounded_formatter())
}

fn continuous_downsample_param() -> FloatParam {
    FloatParam::new(
        "Downsample",
        1.0,
        FloatRange::Skewed {
            min: 1.0,
            max: 50.0,
            factor: FloatRange::skew_factor(-1.0),
        },
    )
    .with_smoother(SmoothingStyle::Linear(CONTROL_SMOOTHING_MS))
    .with_unit("x")
    .with_value_to_string(smart_rounded_formatter())
}

fn mix_param(default: f32) -> FloatParam {
    FloatParam::new("Mix", default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(CONTROL_SMOOTHING_MS))
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

fn output_gain_param(default_db: f32, min_db: f32, max_db: f32) -> FloatParam {
    FloatParam::new(
        "Output",
        util::db_to_gain(default_db),
        FloatRange::Skewed {
            min: util::db_to_gain(min_db),
            max: util::db_to_gain(max_db),
            factor: FloatRange::gain_skew_factor(min_db, max_db),
        },
    )
    .with_smoother(SmoothingStyle::Logarithmic(OUTPUT_GAIN_SMOOTHING_MS))
    .with_unit(" dB")
    .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
    .with_string_to_value(formatters::s2v_f32_gain_to_db())
}

fn smart_rounded_formatter() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
    Arc::new(|v: f32| {
        if (v - v.round()).abs() < 0.001 {
            format!("{}", v.round() as i32)
        } else {
            format!("{v:.2}")
        }
    })
}
