//! Shared conventional 1 Euro filter and display-only linear predictor.
//! Equations: https://gery.casiez.net/1euro/ (CHI 2012).
use vectorcraft_geom::{Point, Vec2};

#[derive(Default)]
pub struct Tracker {
    raw: Option<Point>,
    filtered: Option<Point>,
    derivative: Vec2,
    velocity: Vec2,
    raw_velocity: Vec2,
    time: Option<f64>,
    age: f64,
}

fn alpha(cutoff: f64, dt: f64) -> f64 {
    let r = std::f64::consts::TAU * cutoff * dt;
    r / (1.0 + r)
}

impl Tracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// A reversed clock or long pause restarts. Legacy callers use 120 Hz, not measured time.
    pub fn sample(&mut self, raw: Point, time: Option<f64>, stabilize: bool, min_cutoff: f64, beta: f64) -> Point {
        if !raw.x.is_finite() || !raw.y.is_finite() {
            return self.filtered.unwrap_or(Point::ZERO);
        }
        let time = time.filter(|t| t.is_finite() && *t >= 0.0);
        let dt = self.time.zip(time).map_or(1.0 / 120.0, |(a, b)| b - a);
        if self.raw.is_none() || !(0.0..=0.1).contains(&dt) || dt == 0.0 {
            self.raw = Some(raw);
            self.filtered = Some(raw);
            self.time = time;
            self.derivative = Vec2::ZERO;
            self.velocity = Vec2::ZERO;
            self.raw_velocity = Vec2::ZERO;
            self.age = 0.0;
            return raw;
        }
        let previous_raw = self.raw.unwrap_or(raw);
        let previous = self.filtered.unwrap_or(raw);
        let speed = (raw - previous_raw) / dt;
        let da = alpha(1.0, dt);
        self.derivative = self.derivative * (1.0 - da) + speed * da;
        let cutoff = min_cutoff.clamp(0.1, 60.0) + beta.clamp(0.0, 2.0) * self.derivative.hypot();
        let filtered = if stabilize { previous.lerp(raw, alpha(cutoff, dt)) } else { raw };
        let velocity = (filtered - previous) / dt;
        let predictable = velocity.hypot() > 1.0
            && self.velocity.hypot() > 1.0
            && velocity.dot(self.velocity) > 0.8 * velocity.hypot() * self.velocity.hypot()
            && speed.dot(self.raw_velocity) > 0.8 * speed.hypot() * self.raw_velocity.hypot();
        self.velocity = velocity;
        self.raw_velocity = speed;
        self.age = if predictable { 0.0 } else { 1.0 };
        self.raw = Some(raw);
        self.filtered = Some(filtered);
        self.time = time;
        filtered
    }
    pub fn tick(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 {
            self.age += dt;
        }
    }
    pub fn prediction(&self, ms: f64, zoom: f64) -> Option<(Point, Point)> {
        if self.age > 0.025 || self.velocity.hypot() < 1.0 || ms <= 0.0 {
            return None;
        }
        let p = self.filtered?;
        let delta = self.velocity * (ms.clamp(0.0, 16.0) / 1000.0);
        let max = 24.0 / zoom.max(1e-9);
        Some((p, p + delta * (max / delta.hypot().max(max))))
    }
}

/// 1D 1 Euro filter for scalar values (e.g. pressure, tilt angle, azimuth).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OneEuro1D {
    raw: Option<f64>,
    filtered: Option<f64>,
    derivative: f64,
    time: Option<f64>,
}

impl OneEuro1D {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn filter(&mut self, val: f64, t: f64, min_cutoff: f64, beta: f64) -> f64 {
        if !val.is_finite() || !t.is_finite() {
            return self.filtered.unwrap_or(0.0);
        }
        let dt = self.time.map_or(1.0 / 120.0, |prev| (t - prev).clamp(1e-4, 0.1));
        if self.raw.is_none() || !(0.0..=0.1).contains(&dt) || dt == 0.0 {
            self.raw = Some(val);
            self.filtered = Some(val);
            self.time = Some(t);
            self.derivative = 0.0;
            return val;
        }
        let previous_raw = self.raw.unwrap_or(val);
        let previous = self.filtered.unwrap_or(val);
        let speed = (val - previous_raw) / dt;
        let da = alpha(1.0, dt);
        self.derivative = self.derivative * (1.0 - da) + speed * da;
        let cutoff = min_cutoff.max(0.1) + beta.max(0.0) * self.derivative.abs();
        let a = alpha(cutoff, dt);
        let filtered = previous + a * (val - previous);
        self.raw = Some(val);
        self.filtered = Some(filtered);
        self.time = Some(t);
        filtered
    }
}
