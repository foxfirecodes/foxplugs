use foxplugs_ui::{ParamKnob, ParamKnobOptions, FOXPLUGS_DARK_STYLESHEET};
use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::processor::evaluate_shape;
use crate::FoxshaperParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (720, 520))
}

pub(crate) fn create(
    params: Arc<FoxshaperParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(FOXPLUGS_DARK_STYLESHEET).ok();

        VStack::new(cx, |cx| {
            Label::new(cx, "foxshaper").class("title");

            Label::new(cx, "Volume shaper").class("subtitle");

            WavePreview::new(cx, params.clone());

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.lfo_mode, None);
                    knob_cell(cx, &params.sync_length, None);
                    knob_cell(cx, &params.sync_rhythm, None);
                    knob_cell(cx, &params.rate_hz, Some(0.01));
                    knob_cell(cx, &params.shape_preset, None);
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.depth, Some(0.01));
                    knob_cell(cx, &params.shape, Some(0.01));
                    knob_cell(cx, &params.phase_offset, Some(0.01));
                    knob_cell(cx, &params.smooth, Some(0.01));
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.mix, Some(0.01));
                    // Gain parameters are stored as linear gain, so snapping remains in linear units.
                    knob_cell(cx, &params.trim, Some(0.05));
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
        ParamKnobOptions::default()
            .with_snap_step(snap_step)
            .with_diameter(58.0),
    );
}

struct WavePreview {
    params: Arc<FoxshaperParams>,
}

impl WavePreview {
    fn new(cx: &mut Context, params: Arc<FoxshaperParams>) -> Handle<'_, Self> {
        let shape_preset_signal =
            ParamWidgetBase::new(cx, &params.shape_preset).unmodulated_signal(cx);
        let shape_signal = ParamWidgetBase::new(cx, &params.shape).unmodulated_signal(cx);
        let phase_signal = ParamWidgetBase::new(cx, &params.phase_offset).unmodulated_signal(cx);
        let depth_signal = ParamWidgetBase::new(cx, &params.depth).unmodulated_signal(cx);

        Self { params }
            .build(cx, |_| {})
            .class("wave-preview")
            .width(Pixels(640.0))
            .height(Pixels(120.0))
            .bind(shape_preset_signal, |mut h| h.needs_redraw())
            .bind(shape_signal, |mut h| h.needs_redraw())
            .bind(phase_signal, |mut h| h.needs_redraw())
            .bind(depth_signal, |mut h| h.needs_redraw())
    }
}

impl View for WavePreview {
    fn element(&self) -> Option<&'static str> {
        Some("wave-preview")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }

        let left = bounds.x + 12.0;
        let right = bounds.x + bounds.w - 12.0;
        let top = bounds.y + 12.0;
        let bottom = bounds.y + bounds.h - 12.0;
        let width = (right - left).max(1.0);
        let height = (bottom - top).max(1.0);
        let center_y = top + height * 0.5;

        let mut center_line = vg::PathBuilder::new();
        center_line.move_to((left, center_y));
        center_line.line_to((right, center_y));
        let mut grid_paint = vg::Paint::default();
        grid_paint.set_color(vg::Color::from_argb(255, 42, 38, 56));
        grid_paint.set_stroke_width(1.0);
        grid_paint.set_style(vg::PaintStyle::Stroke);
        canvas.draw_path(&center_line.snapshot(), &grid_paint);

        let preset = self.params.shape_preset.value();
        let shape = self.params.shape.value();
        let phase_offset = self.params.phase_offset.value();
        let depth = self.params.depth.value().clamp(0.0, 1.0);

        let mut curve = vg::PathBuilder::new();
        let segments = 96;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let raw = evaluate_shape(t + phase_offset, preset, shape);
            let value = 1.0 - (1.0 - raw) * depth;
            let x = left + t * width;
            let y = bottom - value * height;
            if i == 0 {
                curve.move_to((x, y));
            } else {
                curve.line_to((x, y));
            }
        }

        let mut curve_paint = vg::Paint::default();
        curve_paint.set_color(vg::Color::from_argb(255, 179, 136, 255));
        curve_paint.set_stroke_width(3.0);
        curve_paint.set_style(vg::PaintStyle::Stroke);
        curve_paint.set_stroke_cap(vg::PaintCap::Round);
        curve_paint.set_stroke_join(vg::PaintJoin::Round);
        curve_paint.set_anti_alias(true);
        canvas.draw_path(&curve.snapshot(), &curve_paint);
    }
}
