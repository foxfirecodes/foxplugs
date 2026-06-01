use foxplugs_ui::{ParamKnob, ParamKnobOptions, FOXPLUGS_DARK_STYLESHEET};
use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::curve::{CurvePoint, PointWeight};
use crate::params::ShapePreset;
use crate::processor::evaluate_shape;
use crate::FoxshaperParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (760, 600))
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
                    knob_cell(cx, &params.trigger_mode, None);
                    knob_cell(cx, &params.loop_mode, None);
                    knob_cell(cx, &params.midi_switch, None);
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.rate_hz, Some(0.01));
                    knob_cell(cx, &params.shape_preset, None);
                    knob_cell(cx, &params.depth, Some(0.01));
                    knob_cell(cx, &params.shape, Some(0.01));
                    knob_cell(cx, &params.end_marker, Some(0.01));
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.audio_sidechain, None);
                    knob_cell(cx, &params.audio_threshold, Some(0.01));
                    knob_cell(cx, &params.audio_low_cut_hz, Some(1.0));
                    knob_cell(cx, &params.audio_high_cut_hz, Some(10.0));
                    knob_cell(cx, &params.audio_detail, Some(0.01));
                })
                .class("knob-row");

                HStack::new(cx, |cx| {
                    knob_cell(cx, &params.phase_offset, Some(0.01));
                    knob_cell(cx, &params.smooth, Some(0.01));
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
    dragging_point: Option<usize>,
}

impl WavePreview {
    fn new(cx: &mut Context, params: Arc<FoxshaperParams>) -> Handle<'_, Self> {
        let shape_preset_signal =
            ParamWidgetBase::new(cx, &params.shape_preset).unmodulated_signal(cx);
        let shape_signal = ParamWidgetBase::new(cx, &params.shape).unmodulated_signal(cx);
        let phase_signal = ParamWidgetBase::new(cx, &params.phase_offset).unmodulated_signal(cx);
        let depth_signal = ParamWidgetBase::new(cx, &params.depth).unmodulated_signal(cx);

        Self {
            params,
            dragging_point: None,
        }
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

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let (phase, value) =
                    mouse_to_curve_position(cx.bounds(), cx.mouse().cursor_x, cx.mouse().cursor_y);
                let curve = self.params.custom_curve.snapshot();
                let nearest = self.params.custom_curve.nearest_point_index(phase, value);
                let selected = nearest.filter(|idx| {
                    let point = curve.points()[*idx];
                    point_distance(point, phase, value) <= 0.01
                });

                self.dragging_point = match selected {
                    Some(index) => Some(index),
                    None => self.params.custom_curve.insert_point(CurvePoint::new(
                        phase,
                        value,
                        PointWeight::Soft,
                    )),
                };

                cx.capture();
                cx.focus();
                cx.set_active(true);
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right) => {
                let (phase, value) =
                    mouse_to_curve_position(cx.bounds(), cx.mouse().cursor_x, cx.mouse().cursor_y);
                if let Some(index) = self.params.custom_curve.nearest_point_index(phase, value) {
                    self.params.custom_curve.remove_point(index);
                    self.dragging_point = None;
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(x, y) => {
                if let Some(index) = self.dragging_point {
                    let (phase, value) = mouse_to_curve_position(cx.bounds(), *x, *y);
                    self.params
                        .custom_curve
                        .set_point(index, CurvePoint::new(phase, value, PointWeight::Soft));
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging_point.is_some() {
                    self.dragging_point = None;
                    cx.release();
                    cx.set_active(false);
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            _ => {}
        });
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
        let custom_curve = self.params.custom_curve.snapshot();

        let mut curve = vg::PathBuilder::new();
        let segments = 96;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let raw = if preset == ShapePreset::Custom {
                custom_curve.evaluate(t + phase_offset)
            } else {
                evaluate_shape(t + phase_offset, preset, shape)
            };
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

        if preset == ShapePreset::Custom {
            let mut point_paint = vg::Paint::default();
            point_paint.set_color(vg::Color::from_argb(255, 110, 168, 254));
            point_paint.set_style(vg::PaintStyle::Fill);
            point_paint.set_anti_alias(true);

            for point in custom_curve.points() {
                let x = left + point.phase * width;
                let y = bottom - point.value * height;
                let mut point_path = vg::PathBuilder::new();
                point_path.add_circle((x, y), 4.5, None);
                canvas.draw_path(&point_path.snapshot(), &point_paint);
            }
        }
    }
}

fn mouse_to_curve_position(bounds: BoundingBox, x: f32, y: f32) -> (f32, f32) {
    let left = bounds.x + 12.0;
    let right = bounds.x + bounds.w - 12.0;
    let top = bounds.y + 12.0;
    let bottom = bounds.y + bounds.h - 12.0;
    let width = (right - left).max(1.0);
    let height = (bottom - top).max(1.0);

    let phase = ((x - left) / width).clamp(0.0, 1.0);
    let value = (1.0 - ((y - top) / height)).clamp(0.0, 1.0);
    (phase, value)
}

fn point_distance(point: CurvePoint, phase: f32, value: f32) -> f32 {
    let phase_distance = (point.phase - phase)
        .abs()
        .min(1.0 - (point.phase - phase).abs());
    let value_distance = (point.value - value).abs();
    phase_distance * phase_distance + value_distance * value_distance
}
