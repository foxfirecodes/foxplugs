use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::FoxcrushParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (320, 360))
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

            labeled_slider(cx, "Bit Depth", &params.bit_depth);
            labeled_slider(cx, "Downsample", &params.downsample);
            labeled_slider(cx, "Mix", &params.mix);
            labeled_slider(cx, "Output", &params.output_gain);
        })
        .alignment(Alignment::TopCenter);
    })
}

fn labeled_slider<P: nih_plug::params::Param + 'static>(
    cx: &mut Context,
    name: &'static str,
    param: &P,
) {
    VStack::new(cx, |cx| {
        Label::new(cx, name).top(Pixels(8.0));
        ParamSlider::new(cx, param);
    });
}
