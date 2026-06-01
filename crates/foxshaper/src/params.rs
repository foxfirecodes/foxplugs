use crate::curve::CurveState;
use foxplugs_plugin::params;
use nih_plug::prelude::*;
use std::sync::Arc;

#[cfg(feature = "gui")]
use vizia_plug::ViziaState;

const RATE_MIN_HZ: f32 = 0.05;
const RATE_MAX_HZ: f32 = 20.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum LfoMode {
    #[id = "hz"]
    Hz,
    #[id = "beats"]
    Beats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum SyncLength {
    #[id = "1-16"]
    #[name = "1/16"]
    Sixteenth,
    #[id = "1-8"]
    #[name = "1/8"]
    Eighth,
    #[id = "1-4"]
    #[name = "1/4"]
    Quarter,
    #[id = "1-2"]
    #[name = "1/2"]
    Half,
    #[id = "1-bar"]
    #[name = "1 Bar"]
    OneBar,
    #[id = "2-bars"]
    #[name = "2 Bars"]
    TwoBars,
    #[id = "4-bars"]
    #[name = "4 Bars"]
    FourBars,
}

impl SyncLength {
    pub(crate) fn beats(self, beats_per_bar: f32) -> f32 {
        match self {
            Self::Sixteenth => 0.25,
            Self::Eighth => 0.5,
            Self::Quarter => 1.0,
            Self::Half => 2.0,
            Self::OneBar => beats_per_bar,
            Self::TwoBars => beats_per_bar * 2.0,
            Self::FourBars => beats_per_bar * 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum SyncRhythm {
    #[id = "straight"]
    Straight,
    #[id = "dotted"]
    Dotted,
    #[id = "triplet"]
    Triplet,
}

impl SyncRhythm {
    pub(crate) fn multiplier(self) -> f32 {
        match self {
            Self::Straight => 1.0,
            Self::Dotted => 1.5,
            Self::Triplet => 2.0 / 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum TriggerMode {
    #[id = "sync"]
    Sync,
    #[id = "midi"]
    MIDI,
    #[id = "audio"]
    Audio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum LoopMode {
    #[id = "loop"]
    Loop,
    #[id = "one-shot"]
    #[name = "1-Shot"]
    OneShot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum ShapePreset {
    #[id = "sidechain"]
    Sidechain,
    #[id = "ramp-up"]
    #[name = "Ramp Up"]
    RampUp,
    #[id = "ramp-down"]
    #[name = "Ramp Down"]
    RampDown,
    #[id = "gate"]
    Gate,
    #[id = "sine"]
    Sine,
    #[id = "triangle"]
    Triangle,
    #[id = "custom"]
    Custom,
}

#[derive(Params)]
pub struct FoxshaperParams {
    #[cfg(feature = "gui")]
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,

    #[id = "lfo_mode"]
    pub lfo_mode: EnumParam<LfoMode>,

    #[id = "rate_hz"]
    pub rate_hz: FloatParam,

    #[id = "sync_length"]
    pub sync_length: EnumParam<SyncLength>,

    #[id = "sync_rhythm"]
    pub sync_rhythm: EnumParam<SyncRhythm>,

    #[id = "trigger_mode"]
    pub trigger_mode: EnumParam<TriggerMode>,

    #[id = "loop_mode"]
    pub loop_mode: EnumParam<LoopMode>,

    #[id = "midi_switch"]
    pub midi_switch: BoolParam,

    #[id = "end_marker"]
    pub end_marker: FloatParam,

    #[id = "audio_threshold"]
    pub audio_threshold: FloatParam,

    #[id = "depth"]
    pub depth: FloatParam,

    #[id = "shape_preset"]
    pub shape_preset: EnumParam<ShapePreset>,

    #[id = "shape"]
    pub shape: FloatParam,

    #[id = "phase_offset"]
    pub phase_offset: FloatParam,

    #[id = "smooth"]
    pub smooth: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "trim"]
    pub trim: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,

    pub(crate) custom_curve: Arc<CurveState>,
}

impl Default for FoxshaperParams {
    fn default() -> Self {
        Self {
            #[cfg(feature = "gui")]
            editor_state: crate::editor::default_state(),

            lfo_mode: EnumParam::new("Mode", LfoMode::Beats),
            rate_hz: params::hz_param("Rate", 1.0, RATE_MIN_HZ, RATE_MAX_HZ),
            sync_length: EnumParam::new("Length", SyncLength::OneBar),
            sync_rhythm: EnumParam::new("Feel", SyncRhythm::Straight),
            trigger_mode: EnumParam::new("Trigger", TriggerMode::Sync),
            loop_mode: EnumParam::new("Loop", LoopMode::Loop),
            midi_switch: BoolParam::new("MIDI Switch", false),
            end_marker: params::percentage_param("End", 1.0),
            audio_threshold: params::gain_db_param("Audio Thresh", -12.0, -60.0, 0.0),
            depth: params::percentage_param("Depth", 1.0),
            shape_preset: EnumParam::new("Wave", ShapePreset::Sidechain),
            shape: params::percentage_param("Shape", 0.5),
            phase_offset: params::percentage_param("Phase", 0.0),
            smooth: params::percentage_param("Smooth", 0.05),
            mix: params::mix_param(1.0),
            trim: params::gain_db_param("Trim", 0.0, -24.0, 24.0),
            output_gain: params::output_gain_param(0.0),
            custom_curve: Arc::new(CurveState::default()),
        }
    }
}
