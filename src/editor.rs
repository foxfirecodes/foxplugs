use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::FoxcrushParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (260, 200))
}

pub(crate) fn create(
    params: Arc<FoxcrushParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        VStack::new(cx, |cx| {
            Label::new(cx, "foxcrush")
                .font_size(28.0)
                .height(Pixels(50.0))
                .alignment(Alignment::BottomCenter);

            Label::new(cx, "Gain");
            ParamSlider::new(cx, &params.gain);
        })
        .alignment(Alignment::TopCenter);
    })
}
