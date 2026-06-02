use foxplugs_ui::{ParamSlider, ParamSliderOptions, ParamToggleGroup, FOXPLUGS_DARK_STYLESHEET};
use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::curve::{CurvePoint, PointWeight};
use crate::params::{LoopMode, ShapePreset};
use crate::processor::evaluate_shape;
use crate::FoxshaperParams;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (860, 620))
}

pub(crate) fn create(
    params: Arc<FoxshaperParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(FOXPLUGS_DARK_STYLESHEET).ok();

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, "foxshaper").class("title");
                    Label::new(cx, "Volume shaper").class("subtitle");
                })
                .class("brand-block");

                Label::new(cx, "Phase-synced volume movement").class("top-hint");
            })
            .class("top-bar");

            WavePreview::new(cx, params.clone());

            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, "Shape").class("section-title");
                    HStack::new(cx, |cx| {
                        toggle_cell(cx, &params.shape_preset);
                        slider_cell(cx, &params.shape, Some(0.01));
                        slider_cell(cx, &params.depth, Some(0.01));
                    })
                    .class("compact-knob-row");

                    Label::new(cx, "Custom curves: select Custom, click/drag the graph, right-click to remove points.")
                        .class("panel-help");
                })
                .class("control-panel");

                VStack::new(cx, |cx| {
                    Label::new(cx, "Timing").class("section-title");
                    HStack::new(cx, |cx| {
                        toggle_cell(cx, &params.lfo_mode);
                        toggle_cell(cx, &params.sync_length);
                        toggle_cell(cx, &params.sync_rhythm);
                        slider_cell(cx, &params.rate_hz, Some(0.01));
                        slider_cell(cx, &params.phase_offset, Some(0.01));
                    })
                    .class("compact-knob-row");
                })
                .class("control-panel");
            })
            .class("panel-row");

            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, "Trigger").class("section-title");
                    HStack::new(cx, |cx| {
                        toggle_cell(cx, &params.trigger_mode);
                        toggle_cell(cx, &params.loop_mode);
                        slider_cell(cx, &params.end_marker, Some(0.01));
                        toggle_cell(cx, &params.midi_switch);
                    })
                    .class("compact-knob-row");
                    Label::new(cx, "MIDI: notes retrigger; MIDI Switch maps C to trigger and C#–A to wave slots.")
                        .class("panel-help");
                })
                .class("control-panel");

                VStack::new(cx, |cx| {
                    Label::new(cx, "Audio Detector").class("section-title");
                    HStack::new(cx, |cx| {
                        toggle_cell(cx, &params.audio_sidechain);
                        slider_cell(cx, &params.audio_threshold, Some(0.01));
                        slider_cell(cx, &params.audio_detail, Some(0.01));
                        slider_cell(cx, &params.audio_low_cut_hz, Some(1.0));
                        slider_cell(cx, &params.audio_high_cut_hz, Some(10.0));
                    })
                    .class("compact-knob-row");
                })
                .class("control-panel");
            })
            .class("panel-row");

            HStack::new(cx, |cx| {
                Label::new(cx, "Output").class("section-title");
                slider_cell(cx, &params.smooth, Some(0.01));
                slider_cell(cx, &params.mix, Some(0.01));
                // Gain parameters are stored as linear gain, so snapping remains in linear units.
                slider_cell(cx, &params.trim, Some(0.05));
                slider_cell(cx, &params.output_gain, Some(0.05));
            })
            .class("mix-strip");
        })
        .class("foxshaper-root")
        .alignment(Alignment::TopCenter);
    })
}

fn slider_cell<P: nih_plug::params::Param + 'static>(
    cx: &mut Context,
    param: &P,
    snap_step: Option<f32>,
) {
    ParamSlider::new(
        cx,
        param,
        ParamSliderOptions::default()
            .with_snap_step(snap_step)
            .with_width(132.0),
    );
}

fn toggle_cell<P: nih_plug::params::Param + 'static>(cx: &mut Context, param: &P) {
    ParamToggleGroup::new(cx, param);
}

struct WavePreview {
    params: Arc<FoxshaperParams>,
    end_marker_base: ParamWidgetBase,
    dragging_point: Option<usize>,
    selected_point: Option<usize>,
    dragging_end_marker: bool,
}

impl WavePreview {
    fn new(cx: &mut Context, params: Arc<FoxshaperParams>) -> Handle<'_, Self> {
        let shape_preset_signal =
            ParamWidgetBase::new(cx, &params.shape_preset).unmodulated_signal(cx);
        let shape_signal = ParamWidgetBase::new(cx, &params.shape).unmodulated_signal(cx);
        let phase_signal = ParamWidgetBase::new(cx, &params.phase_offset).unmodulated_signal(cx);
        let depth_signal = ParamWidgetBase::new(cx, &params.depth).unmodulated_signal(cx);
        let end_marker_base = ParamWidgetBase::new(cx, &params.end_marker);
        let end_marker_signal = end_marker_base.unmodulated_signal(cx);
        let loop_mode_signal = ParamWidgetBase::new(cx, &params.loop_mode).unmodulated_signal(cx);

        Self {
            params,
            end_marker_base,
            dragging_point: None,
            selected_point: None,
            dragging_end_marker: false,
        }
        .build(cx, |_| {})
        .class("wave-preview")
        .width(Stretch(1.0))
        .height(Pixels(190.0))
        .bind(shape_preset_signal, |mut h| h.needs_redraw())
        .bind(shape_signal, |mut h| h.needs_redraw())
        .bind(phase_signal, |mut h| h.needs_redraw())
        .bind(depth_signal, |mut h| h.needs_redraw())
        .bind(end_marker_signal, |mut h| h.needs_redraw())
        .bind(loop_mode_signal, |mut h| h.needs_redraw())
    }
}

impl View for WavePreview {
    fn element(&self) -> Option<&'static str> {
        Some("wave-preview")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                if self.params.loop_mode.value() == LoopMode::OneShot
                    && end_marker_hit_test(
                        cx.bounds(),
                        cx.mouse().cursor_x,
                        self.params.end_marker.value(),
                    )
                {
                    self.dragging_end_marker = true;
                    self.end_marker_base.begin_set_parameter(cx);
                    set_end_marker_from_x(self.end_marker_base, cx, cx.mouse().cursor_x);
                    cx.capture();
                    cx.focus();
                    cx.set_active(true);
                    cx.needs_redraw();
                    meta.consume();
                    return;
                }

                let snap = cx.modifiers().shift();
                let (phase, value) = mouse_to_curve_position(
                    cx.bounds(),
                    cx.mouse().cursor_x,
                    cx.mouse().cursor_y,
                    snap,
                );
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
                self.selected_point = self.dragging_point;

                cx.capture();
                cx.focus();
                cx.set_active(true);
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right) => {
                let (phase, value) = mouse_to_curve_position(
                    cx.bounds(),
                    cx.mouse().cursor_x,
                    cx.mouse().cursor_y,
                    false,
                );
                if let Some(index) = self.params.custom_curve.nearest_point_index(phase, value) {
                    self.params.custom_curve.remove_point(index);
                    self.dragging_point = None;
                    self.selected_point = None;
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(x, y) => {
                if self.dragging_end_marker {
                    set_end_marker_from_x(self.end_marker_base, cx, *x);
                    cx.needs_redraw();
                    meta.consume();
                } else if let Some(index) = self.dragging_point {
                    let (phase, value) =
                        mouse_to_curve_position(cx.bounds(), *x, *y, cx.modifiers().shift());
                    self.params
                        .custom_curve
                        .set_point(index, CurvePoint::new(phase, value, PointWeight::Soft));
                    self.dragging_point =
                        self.params.custom_curve.nearest_point_index(phase, value);
                    self.selected_point = self.dragging_point;
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging_point.is_some() || self.dragging_end_marker {
                    self.dragging_point = None;
                    if self.dragging_end_marker {
                        self.dragging_end_marker = false;
                        self.end_marker_base.end_set_parameter(cx);
                    }
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

        let left = bounds.x + 18.0;
        let right = bounds.x + bounds.w - 18.0;
        let top = bounds.y + 16.0;
        let bottom = bounds.y + bounds.h - 16.0;
        let width = (right - left).max(1.0);
        let height = (bottom - top).max(1.0);
        let center_y = top + height * 0.5;

        let mut grid_paint = vg::Paint::default();
        grid_paint.set_color(vg::Color::from_argb(255, 42, 38, 56));
        grid_paint.set_stroke_width(1.0);
        grid_paint.set_style(vg::PaintStyle::Stroke);

        for division in 0..=16 {
            let x = left + width * division as f32 / 16.0;
            let mut line = vg::PathBuilder::new();
            line.move_to((x, top));
            line.line_to((x, bottom));
            canvas.draw_path(&line.snapshot(), &grid_paint);
        }

        for value in [0.0, 0.5, 1.0] {
            let y = bottom - height * value;
            let mut line = vg::PathBuilder::new();
            line.move_to((left, y));
            line.line_to((right, y));
            canvas.draw_path(&line.snapshot(), &grid_paint);
        }

        let mut center_line = vg::PathBuilder::new();
        center_line.move_to((left, center_y));
        center_line.line_to((right, center_y));
        canvas.draw_path(&center_line.snapshot(), &grid_paint);

        let preset = self.params.shape_preset.value();
        let shape = self.params.shape.value();
        let phase_offset = self.params.phase_offset.value();
        let depth = self.params.depth.value().clamp(0.0, 1.0);
        let custom_curve = self.params.custom_curve.snapshot();

        let mut curve = vg::PathBuilder::new();
        let segments = 128;
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

        if self.params.loop_mode.value() == LoopMode::OneShot {
            let marker_x = left + self.params.end_marker.value().clamp(0.0, 1.0) * width;
            let mut marker = vg::PathBuilder::new();
            marker.move_to((marker_x, top));
            marker.line_to((marker_x, bottom));
            let mut marker_paint = vg::Paint::default();
            marker_paint.set_color(vg::Color::from_argb(255, 255, 183, 77));
            marker_paint.set_stroke_width(2.0);
            marker_paint.set_style(vg::PaintStyle::Stroke);
            marker_paint.set_anti_alias(true);
            canvas.draw_path(&marker.snapshot(), &marker_paint);
        }

        if preset == ShapePreset::Custom {
            let mut point_paint = vg::Paint::default();
            point_paint.set_color(vg::Color::from_argb(255, 110, 168, 254));
            point_paint.set_style(vg::PaintStyle::Fill);
            point_paint.set_anti_alias(true);

            let mut selected_paint = vg::Paint::default();
            selected_paint.set_color(vg::Color::from_argb(255, 255, 183, 77));
            selected_paint.set_style(vg::PaintStyle::Fill);
            selected_paint.set_anti_alias(true);

            for (index, point) in custom_curve.points().iter().enumerate() {
                let x = left + point.phase * width;
                let y = bottom - point.value * height;
                let mut point_path = vg::PathBuilder::new();
                let selected = self.selected_point == Some(index);
                point_path.add_circle((x, y), if selected { 6.0 } else { 4.5 }, None);
                canvas.draw_path(
                    &point_path.snapshot(),
                    if selected {
                        &selected_paint
                    } else {
                        &point_paint
                    },
                );
            }
        }
    }
}

fn mouse_to_curve_position(bounds: BoundingBox, x: f32, y: f32, snap: bool) -> (f32, f32) {
    let left = bounds.x + 18.0;
    let right = bounds.x + bounds.w - 18.0;
    let top = bounds.y + 16.0;
    let bottom = bounds.y + bounds.h - 16.0;
    let width = (right - left).max(1.0);
    let height = (bottom - top).max(1.0);

    let mut phase = ((x - left) / width).clamp(0.0, 1.0);
    let mut value = (1.0 - ((y - top) / height)).clamp(0.0, 1.0);
    if snap {
        phase = (phase * 16.0).round() / 16.0;
        value = (value * 12.0).round() / 12.0;
    }
    (phase, value)
}

fn set_end_marker_from_x(base: ParamWidgetBase, cx: &mut EventContext, x: f32) {
    let bounds = cx.bounds();
    let left = bounds.x + 18.0;
    let right = bounds.x + bounds.w - 18.0;
    let normalized = ((x - left) / (right - left).max(1.0)).clamp(0.0, 1.0);
    base.set_normalized_value(cx, normalized);
}

fn end_marker_hit_test(bounds: BoundingBox, x: f32, end_marker: f32) -> bool {
    let left = bounds.x + 18.0;
    let right = bounds.x + bounds.w - 18.0;
    let marker_x = left + end_marker.clamp(0.0, 1.0) * (right - left).max(1.0);
    (x - marker_x).abs() <= 8.0
}

fn point_distance(point: CurvePoint, phase: f32, value: f32) -> f32 {
    let phase_distance = (point.phase - phase)
        .abs()
        .min(1.0 - (point.phase - phase).abs());
    let value_distance = (point.value - value).abs();
    phase_distance * phase_distance + value_distance * value_distance
}
