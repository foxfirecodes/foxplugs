//! Shared plugin-integration helpers for Foxplugs plugins.
//!
//! Keep real-time DSP out of this crate unless it is a zero-cost helper that
//! belongs at the plugin/host boundary. Prefer small, explicit building blocks
//! over large plugin frameworks.

pub mod metadata {
    pub const VENDOR: &str = "foxfire";
    pub const URL: &str = "https://github.com/foxfire/foxplugs";
    pub const EMAIL: &str = "rayzr522@gmail.com";
}

pub mod params {
    use nih_plug::prelude::*;
    use std::sync::Arc;

    pub const CONTROL_SMOOTHING_MS: f32 = 20.0;
    pub const OUTPUT_GAIN_SMOOTHING_MS: f32 = 50.0;
    pub const OUTPUT_GAIN_MIN_DB: f32 = -12.0;
    pub const OUTPUT_GAIN_MAX_DB: f32 = 12.0;

    pub fn percentage_param(name: &'static str, default: f32) -> FloatParam {
        FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
            .with_smoother(SmoothingStyle::Linear(CONTROL_SMOOTHING_MS))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
    }

    pub fn mix_param(default: f32) -> FloatParam {
        percentage_param("Mix", default)
    }

    pub fn output_gain_param(default_db: f32) -> FloatParam {
        output_gain_param_with_range(default_db, OUTPUT_GAIN_MIN_DB, OUTPUT_GAIN_MAX_DB)
    }

    pub fn output_gain_param_with_range(default_db: f32, min_db: f32, max_db: f32) -> FloatParam {
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

    pub fn hz_param(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
        FloatParam::new(
            name,
            default,
            FloatRange::Skewed {
                min,
                max,
                factor: FloatRange::skew_factor(-1.0),
            },
        )
        .with_smoother(SmoothingStyle::Linear(CONTROL_SMOOTHING_MS))
        .with_unit(" Hz")
        .with_value_to_string(smart_frequency_formatter())
    }

    fn smart_frequency_formatter() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
        Arc::new(|v: f32| {
            if v >= 10.0 {
                format!("{v:.1}")
            } else {
                format!("{v:.2}")
            }
        })
    }
}
