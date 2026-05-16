use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::widgets::{ParamKnob, ParamKnobExt};
use crate::FoxcrushParams;

const STYLESHEET: &str = r#"
* {
    background-color: #0d0c14;
    color: #d8d6e2;
    font-size: 13;
}

label {
    background-color: transparent;
    color: #d8d6e2;
}

label.title {
    font-size: 26;
    color: #b388ff;
    height: 48px;
    alignment: bottom-center;
}

vstack.knob-grid {
    row-between: 14px;
    child-space: 16px;
    width: 1s;
}

hstack.knob-row {
    col-between: 18px;
    width: 1s;
    alignment: center;
}

vstack.knob-cell {
    width: 1s;
    row-between: 6px;
    alignment: top-center;
}

label.knob-name {
    color: #6ea8fe;
    font-size: 11;
    alignment: center;
}

label.knob-value {
    color: #d8d6e2;
    font-size: 12;
    alignment: center;
}

label.knob-value:hover {
    color: #b388ff;
}

textbox.knob-value-input {
    background-color: #1a1825;
    color: #d8d6e2;
    border-color: #b388ff;
    border-width: 1px;
    corner-radius: 3px;
    padding: 2px 4px;
    font-size: 12;
}

knob-dial {
    background-color: transparent;
}
"#;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (340, 360))
}

pub(crate) fn create(
    params: Arc<FoxcrushParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(STYLESHEET).ok();

        VStack::new(cx, |cx| {
            Label::new(cx, "foxcrush").class("title");

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.bit_depth, false);
                    knob_cell(cx, &params.downsample, false);
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.mix, false);
                    knob_cell(cx, &params.output_gain, true);
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
    bipolar: bool,
) {
    ParamKnob::new(cx, param).bipolar(bipolar);
}
