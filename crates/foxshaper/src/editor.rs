use foxplugs_ui::{ParamKnob, ParamKnobOptions, FOXPLUGS_DARK_STYLESHEET};
use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::FoxshaperParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (520, 360))
}

pub(crate) fn create(
    params: Arc<FoxshaperParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(FOXPLUGS_DARK_STYLESHEET).ok();

        VStack::new(cx, |cx| {
            Label::new(cx, "foxshaper").class("title");

            Label::new(cx, "Volume shaper scaffold").class("subtitle");

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.rate_hz, Some(0.01));
                    knob_cell(cx, &params.depth, Some(0.01));
                    knob_cell(cx, &params.shape, Some(0.01));
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.phase_offset, Some(0.01));
                    knob_cell(cx, &params.mix, Some(0.01));
                    // Output gain is stored as linear gain, so snapping remains in linear units.
                    knob_cell(cx, &params.output_gain, Some(0.05));
                })
                .class("knob-row");
            })
            .class("knob-grid");
        })
        .alignment(Alignment::TopCenter);
    })
}

fn knob_cell<P: nih_plug::params::Param + 'static>(
    cx: &mut Context,
    param: &P,
    snap_step: Option<f32>,
) {
    ParamKnob::new(
        cx,
        param,
        ParamKnobOptions::default().with_snap_step(snap_step),
    );
}
