#![cfg_attr(not(feature = "gui"), allow(dead_code))]

use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

pub const MAX_CURVE_POINTS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointWeight {
    Hard,
    Medium,
    Soft,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvePoint {
    pub phase: f32,
    pub value: f32,
    pub weight: PointWeight,
}

impl CurvePoint {
    pub const fn new(phase: f32, value: f32, weight: PointWeight) -> Self {
        Self {
            phase,
            value,
            weight,
        }
    }
}

const EMPTY_POINT: CurvePoint = CurvePoint::new(0.0, 1.0, PointWeight::Hard);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    points: [CurvePoint; MAX_CURVE_POINTS],
    len: usize,
}

impl Curve {
    pub const fn new(points: [CurvePoint; MAX_CURVE_POINTS], len: usize) -> Self {
        Self { points, len }
    }

    pub fn len(&self) -> usize {
        self.len.min(MAX_CURVE_POINTS)
    }

    pub fn points(&self) -> &[CurvePoint] {
        &self.points[..self.len()]
    }

    pub fn evaluate(&self, phase: f32) -> f32 {
        let len = self.len();
        if len == 0 {
            return 1.0;
        }
        if len == 1 {
            return self.points[0].value.clamp(0.0, 1.0);
        }

        let phase = wrap_unit(phase);
        let first = self.points[0];

        if phase < first.phase {
            let left = self.points[len - 1];
            return interpolate_segment(phase + 1.0, left.phase, left, first.phase + 1.0, first);
        }

        for pair in self.points()[1..].iter().copied() {
            if phase <= pair.phase {
                let left = previous_point(self.points(), pair);
                return interpolate_segment(phase, left.phase, left, pair.phase, pair);
            }
        }

        let left = self.points[len - 1];
        interpolate_segment(phase, left.phase, left, first.phase + 1.0, first)
    }
}

#[derive(Debug)]
pub struct CurveState {
    phases: [AtomicU32; MAX_CURVE_POINTS],
    values: [AtomicU32; MAX_CURVE_POINTS],
    weights: [AtomicU8; MAX_CURVE_POINTS],
    len: AtomicUsize,
}

impl Default for CurveState {
    fn default() -> Self {
        let state = Self {
            phases: std::array::from_fn(|_| AtomicU32::new(0.0f32.to_bits())),
            values: std::array::from_fn(|_| AtomicU32::new(1.0f32.to_bits())),
            weights: std::array::from_fn(|_| AtomicU8::new(encode_weight(PointWeight::Hard))),
            len: AtomicUsize::new(0),
        };
        state.store_curve(DEFAULT_VOLUME_CURVE);
        state
    }
}

impl CurveState {
    pub fn snapshot(&self) -> Curve {
        let len = self.len.load(Ordering::Acquire).min(MAX_CURVE_POINTS);
        let mut points = [EMPTY_POINT; MAX_CURVE_POINTS];

        for (idx, point) in points.iter_mut().take(len).enumerate() {
            *point = CurvePoint::new(
                f32::from_bits(self.phases[idx].load(Ordering::Relaxed)).clamp(0.0, 1.0),
                f32::from_bits(self.values[idx].load(Ordering::Relaxed)).clamp(0.0, 1.0),
                decode_weight(self.weights[idx].load(Ordering::Relaxed)),
            );
        }

        sort_points(&mut points[..len]);
        Curve::new(points, len)
    }

    pub fn store_curve(&self, curve: Curve) {
        let len = curve.len();
        for (idx, point) in curve.points().iter().copied().enumerate() {
            self.store_point(idx, point);
        }
        self.len.store(len, Ordering::Release);
    }

    pub fn set_point(&self, index: usize, point: CurvePoint) {
        let mut curve = self.snapshot();
        let len = curve.len();
        if index >= len {
            return;
        }
        curve.points[index] = sanitize_point(point);
        sort_points(&mut curve.points[..len]);
        self.store_curve(curve);
    }

    pub fn insert_point(&self, point: CurvePoint) -> Option<usize> {
        let mut curve = self.snapshot();
        let len = curve.len();
        if len >= MAX_CURVE_POINTS {
            return None;
        }
        curve.points[len] = sanitize_point(point);
        curve.len = len + 1;
        sort_points(&mut curve.points[..curve.len]);
        let inserted = nearest_point_index(&curve, point.phase, point.value).unwrap_or(0);
        self.store_curve(curve);
        Some(inserted)
    }

    pub fn remove_point(&self, index: usize) -> bool {
        let mut curve = self.snapshot();
        let len = curve.len();
        if len <= 2 || index >= len {
            return false;
        }
        for idx in index..(len - 1) {
            curve.points[idx] = curve.points[idx + 1];
        }
        curve.points[len - 1] = EMPTY_POINT;
        curve.len = len - 1;
        self.store_curve(curve);
        true
    }

    pub fn nearest_point_index(&self, phase: f32, value: f32) -> Option<usize> {
        nearest_point_index(&self.snapshot(), phase, value)
    }

    fn store_point(&self, index: usize, point: CurvePoint) {
        let point = sanitize_point(point);
        self.phases[index].store(point.phase.to_bits(), Ordering::Relaxed);
        self.values[index].store(point.value.to_bits(), Ordering::Relaxed);
        self.weights[index].store(encode_weight(point.weight), Ordering::Relaxed);
    }
}

fn nearest_point_index(curve: &Curve, phase: f32, value: f32) -> Option<usize> {
    let mut nearest = None;
    let mut nearest_distance = f32::INFINITY;

    for (idx, point) in curve.points().iter().copied().enumerate() {
        let phase_distance = (point.phase - phase)
            .abs()
            .min(1.0 - (point.phase - phase).abs());
        let value_distance = (point.value - value).abs();
        let distance = phase_distance * phase_distance + value_distance * value_distance;
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = Some(idx);
        }
    }

    nearest
}

fn sanitize_point(point: CurvePoint) -> CurvePoint {
    CurvePoint::new(
        point.phase.clamp(0.0, 1.0),
        point.value.clamp(0.0, 1.0),
        point.weight,
    )
}

fn sort_points(points: &mut [CurvePoint]) {
    points.sort_by(|a, b| a.phase.total_cmp(&b.phase));
}

const fn encode_weight(weight: PointWeight) -> u8 {
    match weight {
        PointWeight::Hard => 0,
        PointWeight::Medium => 1,
        PointWeight::Soft => 2,
    }
}

const fn decode_weight(weight: u8) -> PointWeight {
    match weight {
        1 => PointWeight::Medium,
        2 => PointWeight::Soft,
        _ => PointWeight::Hard,
    }
}

fn previous_point(points: &[CurvePoint], current: CurvePoint) -> CurvePoint {
    let mut previous = points[0];
    for point in points.iter().copied().skip(1) {
        if point == current {
            return previous;
        }
        previous = point;
    }
    previous
}

#[inline]
fn interpolate_segment(
    phase: f32,
    left_phase: f32,
    left: CurvePoint,
    right_phase: f32,
    right: CurvePoint,
) -> f32 {
    let span = (right_phase - left_phase).max(0.000_001);
    let t = ((phase - left_phase) / span).clamp(0.0, 1.0);
    let shaped_t = shape_t(t, right.weight);
    left.value + (right.value - left.value) * shaped_t
}

#[inline]
fn shape_t(t: f32, weight: PointWeight) -> f32 {
    match weight {
        PointWeight::Hard => t,
        PointWeight::Medium => t * t * (3.0 - 2.0 * t),
        PointWeight::Soft => t * t * t * (t * (t * 6.0 - 15.0) + 10.0),
    }
}

#[inline]
fn wrap_unit(phase: f32) -> f32 {
    phase - phase.floor()
}

pub const DEFAULT_VOLUME_CURVE: Curve = Curve::new(
    [
        CurvePoint::new(0.0, 0.0, PointWeight::Hard),
        CurvePoint::new(0.08, 0.0, PointWeight::Medium),
        CurvePoint::new(0.5, 1.0, PointWeight::Soft),
        CurvePoint::new(1.0, 1.0, PointWeight::Hard),
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
        EMPTY_POINT,
    ],
    4,
);

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_001;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn empty_and_single_point_curves_are_safe() {
        let empty = Curve::new([EMPTY_POINT; MAX_CURVE_POINTS], 0);
        let one = Curve::new(
            [CurvePoint::new(0.5, 0.25, PointWeight::Hard); MAX_CURVE_POINTS],
            1,
        );

        assert_close(empty.evaluate(0.5), 1.0);
        assert_close(one.evaluate(0.0), 0.25);
    }

    #[test]
    fn hard_points_interpolate_linearly_and_wrap() {
        let curve = Curve::new(
            [
                CurvePoint::new(0.25, 0.0, PointWeight::Hard),
                CurvePoint::new(0.75, 1.0, PointWeight::Hard),
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
            ],
            2,
        );

        assert_close(curve.evaluate(0.25), 0.0);
        assert_close(curve.evaluate(0.5), 0.5);
        assert_close(curve.evaluate(0.75), 1.0);
        assert_close(curve.evaluate(0.0), 0.5);
    }

    #[test]
    fn soft_point_eases_toward_target() {
        let curve = Curve::new(
            [
                CurvePoint::new(0.0, 0.0, PointWeight::Hard),
                CurvePoint::new(1.0, 1.0, PointWeight::Soft),
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
                EMPTY_POINT,
            ],
            2,
        );

        assert!(curve.evaluate(0.25) < 0.25);
        assert_close(curve.evaluate(0.5), 0.5);
        assert!(curve.evaluate(0.75) > 0.75);
    }

    #[test]
    fn curve_state_stores_inserts_moves_and_removes_points() {
        let state = CurveState::default();
        let inserted = state
            .insert_point(CurvePoint::new(0.25, 0.5, PointWeight::Soft))
            .expect("point should insert");
        let nearest = state.nearest_point_index(0.25, 0.5);
        assert_eq!(nearest, Some(inserted));

        state.set_point(inserted, CurvePoint::new(0.3, 0.6, PointWeight::Medium));
        let moved = state.snapshot();
        assert!(moved.points().iter().any(
            |point| (point.phase - 0.3).abs() < EPSILON && (point.value - 0.6).abs() < EPSILON
        ));

        assert!(state.remove_point(inserted));
        assert_eq!(state.snapshot().len(), DEFAULT_VOLUME_CURVE.len());
    }

    #[test]
    fn default_volume_curve_starts_ducked_and_recovers() {
        assert_close(DEFAULT_VOLUME_CURVE.evaluate(0.0), 0.0);
        assert_close(DEFAULT_VOLUME_CURVE.evaluate(0.08), 0.0);
        assert_close(DEFAULT_VOLUME_CURVE.evaluate(0.5), 1.0);
        assert_close(DEFAULT_VOLUME_CURVE.evaluate(0.9), 1.0);
    }
}
