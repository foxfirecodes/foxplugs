//! Shared DSP building blocks for Foxplugs plugins.
//!
//! Keep this crate free of plugin-framework and GUI dependencies. Helpers here
//! should be allocation-free, lock-free, and suitable for real-time audio paths.

pub const STEREO_CHANNELS: usize = 2;

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn dry_wet(dry: f32, wet: f32, mix: f32) -> f32 {
    lerp(dry, wet, mix.clamp(0.0, 1.0))
}

pub mod lfo {
    use std::f32::consts::TAU;

    #[inline]
    pub fn wrap_unit_phase(phase: f32) -> f32 {
        phase - phase.floor()
    }

    #[inline]
    pub fn advance_phase(phase: f32, frequency_hz: f32, sample_rate: f32) -> f32 {
        let increment = if sample_rate > 0.0 {
            frequency_hz.max(0.0) / sample_rate
        } else {
            0.0
        };

        wrap_unit_phase(phase + increment)
    }

    /// A unipolar sine ramp that starts at 0, reaches 1 halfway through the
    /// cycle, and returns to 0 at the end of the cycle.
    #[inline]
    pub fn unipolar_sine(phase: f32) -> f32 {
        (1.0 - (wrap_unit_phase(phase) * TAU).cos()) * 0.5
    }

    /// A unipolar triangle with a movable peak position.
    #[inline]
    pub fn skewed_triangle(phase: f32, peak_phase: f32) -> f32 {
        let phase = wrap_unit_phase(phase);
        let peak_phase = peak_phase.clamp(0.05, 0.95);

        if phase <= peak_phase {
            phase / peak_phase
        } else {
            (1.0 - phase) / (1.0 - peak_phase)
        }
        .clamp(0.0, 1.0)
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
    fn dry_wet_clamps_mix() {
        assert_close(dry_wet(1.0, 0.0, -1.0), 1.0);
        assert_close(dry_wet(1.0, 0.0, 0.25), 0.75);
        assert_close(dry_wet(1.0, 0.0, 2.0), 0.0);
    }

    #[test]
    fn lfo_phase_wraps_forward_and_backward() {
        assert_close(lfo::wrap_unit_phase(1.25), 0.25);
        assert_close(lfo::wrap_unit_phase(-0.25), 0.75);
    }

    #[test]
    fn lfo_advance_uses_sample_rate() {
        assert_close(lfo::advance_phase(0.0, 2.0, 8.0), 0.25);
        assert_close(lfo::advance_phase(0.9, 2.0, 10.0), 0.1);
    }

    #[test]
    fn skewed_triangle_places_peak_at_shape_position() {
        assert_close(lfo::skewed_triangle(0.25, 0.25), 1.0);
        assert_close(lfo::skewed_triangle(0.0, 0.25), 0.0);
        assert_close(lfo::skewed_triangle(1.0, 0.25), 0.0);
    }
}
