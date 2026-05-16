use nih_plug::prelude::{Param, ParamPtr};
use std::f32::consts::PI;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::widgets::util::ModifiersExt;

const KNOB_DIAMETER: f32 = 64.0;
const TRACK_THICKNESS: f32 = 5.0;
const INDICATOR_THICKNESS: f32 = 3.0;
const INDICATOR_LENGTH_RATIO: f32 = 0.55;

const ARC_START_DEG: f32 = 135.0;
const ARC_SWEEP_DEG: f32 = 270.0;

const DRAG_PIXELS_PER_FULL_RANGE: f32 = 200.0;
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.2;

pub struct ParamKnob {
    param_base: ParamWidgetBase,
    bipolar: SyncSignal<bool>,
    text_input_active: SyncSignal<bool>,

    drag_active: bool,
    drag_start_y: f32,
    drag_start_value: f32,
    scrolled_lines: f32,
}

enum ParamKnobEvent {
    StartTextInput,
    SubmitTextInput(String),
    CancelTextInput,
}

impl ParamKnob {
    pub fn new<'c, 'p, P>(cx: &'c mut Context, param: &'p P) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, param);
        let bipolar = SyncSignal::new(false);
        let text_input_active = SyncSignal::new(false);

        let unmodulated = param_base.unmodulated_signal(cx);
        let param_ptr = param_base.param_ptr();

        let display_value: Memo<String> = Memo::new(move |_| {
            let current = unmodulated.get();
            unsafe { param_ptr.normalized_value_to_string(current, true) }
        });

        let name_text: String = unsafe { param_ptr.name() }.to_string();

        Self {
            param_base,
            bipolar,
            text_input_active,
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            scrolled_lines: 0.0,
        }
        .build(cx, move |cx| {
            VStack::new(cx, |cx| {
                KnobDial {
                    value_signal: unmodulated,
                    bipolar_signal: bipolar,
                    param_ptr,
                }
                .build(cx, |_| {})
                .class("knob-dial")
                .width(Pixels(KNOB_DIAMETER))
                .height(Pixels(KNOB_DIAMETER))
                .bind(unmodulated, |mut h| h.needs_redraw())
                .bind(bipolar, |mut h| h.needs_redraw())
                .hoverable(false);

                Label::new(cx, name_text.clone()).class("knob-name");

                Binding::new(cx, text_input_active, move |cx| {
                    if text_input_active.get() {
                        Textbox::new(cx, display_value)
                            .class("knob-value-input")
                            .on_submit(|cx, s, success| {
                                if success {
                                    cx.emit(ParamKnobEvent::SubmitTextInput(s));
                                } else {
                                    cx.emit(ParamKnobEvent::CancelTextInput);
                                }
                            })
                            .on_cancel(|cx| cx.emit(ParamKnobEvent::CancelTextInput))
                            .on_build(|cx| {
                                cx.emit(TextEvent::StartEdit);
                                cx.emit(TextEvent::SelectAll);
                            })
                            .width(Pixels(KNOB_DIAMETER + 20.0))
                            .alignment(Alignment::Center);
                    } else {
                        Label::new(cx, display_value)
                            .class("knob-value")
                            .on_press(|cx| cx.emit(ParamKnobEvent::StartTextInput));
                    }
                });
            })
            .class("knob-cell")
            .alignment(Alignment::TopCenter);
        })
    }

    fn handle_drag(&mut self, cx: &mut EventContext, current_y: f32) {
        let multiplier = if cx.modifiers().shift() {
            GRANULAR_DRAG_MULTIPLIER
        } else {
            1.0
        };
        let delta_pixels = self.drag_start_y - current_y;
        let delta_normalized =
            (delta_pixels / DRAG_PIXELS_PER_FULL_RANGE) * multiplier * cx.scale_factor();
        let new_value = (self.drag_start_value + delta_normalized).clamp(0.0, 1.0);
        self.param_base.set_normalized_value(cx, new_value);
    }
}

impl View for ParamKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|knob_event, meta| match knob_event {
            ParamKnobEvent::StartTextInput => {
                self.text_input_active.set(true);
                cx.set_active(true);
                meta.consume();
            }
            ParamKnobEvent::SubmitTextInput(s) => {
                if let Some(normalized) = self.param_base.string_to_normalized_value(s) {
                    self.param_base.begin_set_parameter(cx);
                    self.param_base
                        .set_normalized_value(cx, normalized.clamp(0.0, 1.0));
                    self.param_base.end_set_parameter(cx);
                }
                self.text_input_active.set(false);
                cx.set_active(false);
                meta.consume();
            }
            ParamKnobEvent::CancelTextInput => {
                self.text_input_active.set(false);
                cx.set_active(false);
                meta.consume();
            }
        });

        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                if self.text_input_active.get() {
                    return;
                }

                if cx.modifiers().alt() {
                    self.text_input_active.set(true);
                    cx.set_active(true);
                    meta.consume();
                    return;
                }

                if cx.modifiers().command() {
                    self.param_base.begin_set_parameter(cx);
                    self.param_base
                        .set_normalized_value(cx, self.param_base.default_normalized_value());
                    self.param_base.end_set_parameter(cx);
                    meta.consume();
                    return;
                }

                self.drag_active = true;
                self.drag_start_y = cx.mouse().cursor_y;
                self.drag_start_value = self.param_base.unmodulated_normalized_value();
                cx.capture();
                cx.focus();
                cx.set_active(true);
                self.param_base.begin_set_parameter(cx);
                meta.consume();
            }

            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right)
            | WindowEvent::MouseDoubleClick(MouseButton::Right) => {
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }

            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    cx.release();
                    cx.set_active(false);
                    self.param_base.end_set_parameter(cx);
                    meta.consume();
                }
            }

            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    self.handle_drag(cx, *y);
                }
            }

            WindowEvent::MouseScroll(_sx, sy) => {
                self.scrolled_lines += sy;
                if self.scrolled_lines.abs() >= 1.0 {
                    let finer = cx.modifiers().shift();
                    if !self.drag_active {
                        self.param_base.begin_set_parameter(cx);
                    }
                    let mut current = self.param_base.unmodulated_normalized_value();
                    while self.scrolled_lines >= 1.0 {
                        current = self.param_base.next_normalized_step(current, finer);
                        self.param_base.set_normalized_value(cx, current);
                        self.scrolled_lines -= 1.0;
                    }
                    while self.scrolled_lines <= -1.0 {
                        current = self.param_base.previous_normalized_step(current, finer);
                        self.param_base.set_normalized_value(cx, current);
                        self.scrolled_lines += 1.0;
                    }
                    if !self.drag_active {
                        self.param_base.end_set_parameter(cx);
                    }
                    meta.consume();
                }
            }

            _ => {}
        });
    }
}

pub trait ParamKnobExt {
    fn bipolar(self, on: bool) -> Self;
}

impl ParamKnobExt for Handle<'_, ParamKnob> {
    fn bipolar(self, on: bool) -> Self {
        self.modify(|k: &mut ParamKnob| k.bipolar.set(on))
    }
}

struct KnobDial {
    value_signal: SyncSignal<f32>,
    bipolar_signal: SyncSignal<bool>,
    param_ptr: ParamPtr,
}

impl View for KnobDial {
    fn element(&self) -> Option<&'static str> {
        Some("knob-dial")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }

        let cx_px = bounds.x + bounds.w / 2.0;
        let cy_px = bounds.y + bounds.h / 2.0;
        let radius = (bounds.w.min(bounds.h) / 2.0) - TRACK_THICKNESS;

        let value = self.value_signal.get().clamp(0.0, 1.0);
        let bipolar = self.bipolar_signal.get();
        let default_normalized = unsafe { self.param_ptr.default_normalized_value() };

        let track_color = vg::Color::from_argb(255, 42, 38, 56);
        let fill_color = vg::Color::from_argb(255, 179, 136, 255);
        let indicator_color = vg::Color::from_argb(255, 230, 226, 240);

        let start_rad = ARC_START_DEG.to_radians();
        let sweep_rad = ARC_SWEEP_DEG.to_radians();

        draw_arc(
            canvas,
            cx_px,
            cy_px,
            radius,
            start_rad,
            sweep_rad,
            TRACK_THICKNESS,
            track_color,
        );

        if bipolar {
            let center_t = default_normalized;
            let delta_t = value - center_t;
            if delta_t.abs() > 1e-4 {
                let fill_start_t = center_t.min(value);
                let fill_sweep_t = delta_t.abs();
                draw_arc(
                    canvas,
                    cx_px,
                    cy_px,
                    radius,
                    start_rad + fill_start_t * sweep_rad,
                    fill_sweep_t * sweep_rad,
                    TRACK_THICKNESS,
                    fill_color,
                );
            }
        } else if value > 1e-4 {
            draw_arc(
                canvas,
                cx_px,
                cy_px,
                radius,
                start_rad,
                value * sweep_rad,
                TRACK_THICKNESS,
                fill_color,
            );
        }

        let angle = start_rad + value * sweep_rad;
        let inner_r = (radius * (1.0 - INDICATOR_LENGTH_RATIO)).max(2.0);
        let outer_r = (radius - TRACK_THICKNESS * 1.5).max(inner_r + 1.0);

        let ix1 = cx_px + angle.cos() * inner_r;
        let iy1 = cy_px + angle.sin() * inner_r;
        let ix2 = cx_px + angle.cos() * outer_r;
        let iy2 = cy_px + angle.sin() * outer_r;

        let mut indicator = vg::PathBuilder::new();
        indicator.move_to((ix1, iy1));
        indicator.line_to((ix2, iy2));

        let mut indicator_paint = vg::Paint::default();
        indicator_paint.set_color(indicator_color);
        indicator_paint.set_stroke_width(INDICATOR_THICKNESS);
        indicator_paint.set_style(vg::PaintStyle::Stroke);
        indicator_paint.set_anti_alias(true);
        canvas.draw_path(&indicator.snapshot(), &indicator_paint);
    }
}

fn draw_arc(
    canvas: &Canvas,
    cx_px: f32,
    cy_px: f32,
    radius: f32,
    start_rad: f32,
    sweep_rad: f32,
    thickness: f32,
    color: vg::Color,
) {
    let segments = ((sweep_rad.abs() / (PI / 32.0)).ceil() as usize).max(2);
    let mut path = vg::PathBuilder::new();
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = start_rad + sweep_rad * t;
        let x = cx_px + a.cos() * radius;
        let y = cy_px + a.sin() * radius;
        if i == 0 {
            path.move_to((x, y));
        } else {
            path.line_to((x, y));
        }
    }
    let mut paint = vg::Paint::default();
    paint.set_color(color);
    paint.set_stroke_width(thickness);
    paint.set_style(vg::PaintStyle::Stroke);
    paint.set_stroke_cap(vg::PaintCap::Round);
    paint.set_anti_alias(true);
    canvas.draw_path(&path.snapshot(), &paint);
}
