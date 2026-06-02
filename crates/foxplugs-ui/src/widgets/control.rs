use nih_plug::prelude::{Param, ParamPtr};
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;

#[derive(Clone, Copy, Debug)]
pub struct ParamSliderOptions {
    pub width: f32,
    pub height: f32,
    pub snap_step: Option<f32>,
}

impl Default for ParamSliderOptions {
    fn default() -> Self {
        Self {
            width: 160.0,
            height: 34.0,
            snap_step: None,
        }
    }
}

impl ParamSliderOptions {
    pub fn with_snap_step(mut self, snap_step: Option<f32>) -> Self {
        self.snap_step = snap_step;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

pub struct ParamSlider {
    param_base: ParamWidgetBase,
    value_signal: SyncSignal<f32>,
    param_ptr: ParamPtr,
    options: ParamSliderOptions,
    dragging: bool,
}

impl ParamSlider {
    pub fn new<'c, 'p, P>(
        cx: &'c mut Context,
        param: &'p P,
        options: ParamSliderOptions,
    ) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, param);
        let value_signal = param_base.unmodulated_signal(cx);
        let param_ptr = param_base.param_ptr();
        let display_value = Memo::new(move |_| unsafe {
            param_ptr.normalized_value_to_string(value_signal.get(), true)
        });
        let name = unsafe { param_ptr.name() }.to_string();

        Self {
            param_base,
            value_signal,
            param_ptr,
            options,
            dragging: false,
        }
        .build(cx, move |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, name.clone()).class("param-slider-name");
                    Label::new(cx, display_value).class("param-slider-value");
                })
                .class("param-slider-header");
            })
            .class("param-slider-shell");
        })
        .class("param-slider")
        .width(Pixels(options.width))
        .height(Pixels(options.height))
        .bind(value_signal, |mut h| h.needs_redraw())
    }

    fn normalized_from_x(&self, cx: &EventContext, x: f32) -> f32 {
        let bounds = cx.bounds();
        ((x - bounds.x) / bounds.w.max(1.0)).clamp(0.0, 1.0)
    }

    fn snap_normalized(&self, normalized: f32) -> f32 {
        match self.options.snap_step {
            Some(step) if step > 0.0 => {
                let plain = unsafe { self.param_ptr.preview_plain(normalized) };
                let snapped_plain = (plain / step).round() * step;
                unsafe { self.param_ptr.preview_normalized(snapped_plain) }
            }
            _ => normalized,
        }
    }

    fn set_from_x(&self, cx: &mut EventContext, x: f32) {
        let normalized = self.snap_normalized(self.normalized_from_x(cx, x));
        self.param_base.set_normalized_value(cx, normalized);
    }
}

impl View for ParamSlider {
    fn element(&self) -> Option<&'static str> {
        Some("param-slider")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.dragging = true;
                cx.capture();
                cx.focus();
                cx.set_active(true);
                self.param_base.begin_set_parameter(cx);
                self.set_from_x(cx, cx.mouse().cursor_x);
                meta.consume();
            }
            WindowEvent::MouseMove(x, _) if self.dragging => {
                self.set_from_x(cx, *x);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.dragging => {
                self.dragging = false;
                cx.release();
                cx.set_active(false);
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        let y = bounds.y + bounds.h - 8.0;
        let left = bounds.x + 8.0;
        let right = bounds.x + bounds.w - 8.0;
        let value = self.value_signal.get().clamp(0.0, 1.0);

        let mut track = vg::PathBuilder::new();
        track.move_to((left, y));
        track.line_to((right, y));
        let mut track_paint = vg::Paint::default();
        track_paint.set_color(vg::Color::from_argb(255, 42, 38, 56));
        track_paint.set_stroke_width(5.0);
        track_paint.set_stroke_cap(vg::PaintCap::Round);
        track_paint.set_style(vg::PaintStyle::Stroke);
        track_paint.set_anti_alias(true);
        canvas.draw_path(&track.snapshot(), &track_paint);

        let fill_right = left + (right - left) * value;
        let mut fill = vg::PathBuilder::new();
        fill.move_to((left, y));
        fill.line_to((fill_right, y));
        let mut fill_paint = vg::Paint::default();
        fill_paint.set_color(vg::Color::from_argb(255, 179, 136, 255));
        fill_paint.set_stroke_width(5.0);
        fill_paint.set_stroke_cap(vg::PaintCap::Round);
        fill_paint.set_style(vg::PaintStyle::Stroke);
        fill_paint.set_anti_alias(true);
        canvas.draw_path(&fill.snapshot(), &fill_paint);
    }
}

pub struct ParamToggleGroup;

impl ParamToggleGroup {
    pub fn new<'c, 'p, P>(cx: &'c mut Context, param: &'p P) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, param);
        let param_ptr = param_base.param_ptr();
        let name = unsafe { param_ptr.name() }.to_string();
        let step_count = param_base.step_count().unwrap_or(1);
        let steps = step_count + 1;

        Self.build(cx, move |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, name.clone()).class("toggle-group-name");
                HStack::new(cx, |cx| {
                    for step in 0..steps {
                        let normalized = if step_count == 0 {
                            0.0
                        } else {
                            step as f32 / step_count as f32
                        };
                        let text =
                            unsafe { param_ptr.normalized_value_to_string(normalized, true) };
                        let base = param_base;
                        Label::new(cx, text)
                            .class("toggle-option")
                            .on_press(move |cx| {
                                base.begin_set_parameter(cx);
                                base.set_normalized_value(cx, normalized);
                                base.end_set_parameter(cx);
                            });
                    }
                })
                .class("toggle-row");
            })
            .class("toggle-group-shell");
        })
        .class("toggle-group")
    }
}

impl View for ParamToggleGroup {
    fn element(&self) -> Option<&'static str> {
        Some("param-toggle-group")
    }
}
