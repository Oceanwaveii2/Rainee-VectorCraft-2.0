//! Measured pen channels -> editable stroke appearance. No prediction or learned appearance.
use super::{Result, bad};
use serde_json::Value;
use vectorcraft_doc::{NibProfile, StrokeLayer, WidthProfile};
use vectorcraft_geom::Point;

pub(super) fn apply(st: &mut StrokeLayer, points: &[Point], p: &Value) -> Result<()> {
    const C: &str = "path.freehand";
    let Some(options) = p.get("expressive") else { return Ok(()) };
    if !options.is_object() {
        return Err(bad(C, "expressive must be an object"));
    }
    let flag = |key| -> Result<bool> {
        match options.get(key) {
            None => Ok(false),
            Some(Value::Bool(b)) => Ok(*b),
            _ => Err(bad(C, format!("{key} must be a boolean"))),
        }
    };
    let pressure = flag("pressureWidth")?;
    let tilt = flag("tiltNib")?;
    if !pressure && !tilt {
        return Ok(());
    }
    if st.dash.as_ref().is_some_and(|d| d.is_dashed())
        || st.start_arrow.is_some()
        || st.end_arrow.is_some()
        || st.align != vectorcraft_doc::StrokeAlign::Center
    {
        return Err(bad(
            C,
            "Expressive Pen currently needs a solid centered stroke without arrowheads; disable expressive input to retain those features",
        ));
    }
    let number = |key, default, min, max| -> Result<f64> {
        match options.get(key) {
            None => Ok(default),
            Some(v) => v.as_f64().filter(|n| n.is_finite() && *n >= min && *n <= max).ok_or_else(|| bad(C, format!("{key} must be in {min}..{max}"))),
        }
    };
    let gamma = number("pressureGamma", 1.0, 0.1, 4.0)?;
    let floor = number("pressureMin", 0.1, 0.01, 1.0)?;
    let roundness = number("nibRoundness", 0.2, 0.01, 1.0)?;
    let fallback_angle = number("nibAngle", 0.0, -180.0, 180.0)?;
    let samples = p
        .get("penSamples")
        .and_then(Value::as_array)
        .filter(|a| a.len() == points.len() && a.len() <= 16384)
        .ok_or_else(|| bad(C, "penSamples must match valid points, at most 16384"))?;
    let mut cum = vec![0.0];
    for w in points.windows(2) {
        cum.push(cum.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
    }
    let total = cum.last().copied().unwrap_or(0.0);
    if total <= 1e-9 || !total.is_finite() {
        return Err(bad(C, "expressive stroke needs finite nonzero length"));
    }
    let mut widths = WidthProfile::default();
    let mut nibs = NibProfile::default();
    for (i, sample) in samples.iter().enumerate() {
        if !sample.is_object() {
            return Err(bad(C, "pen sample must be an object"));
        }
        let measured = match sample.get("pressure") {
            None | Some(Value::Null) => 1.0,
            Some(v) => {
                v.as_f64().filter(|n| n.is_finite() && (0.0..=1.0).contains(n)).ok_or_else(|| bad(C, "pressure must be 0..1, null or absent"))?
            }
        };
        let (angle, ratio) = match sample.get("tilt") {
            None | Some(Value::Null) => (fallback_angle, roundness),
            Some(v) => {
                let a = v.as_array().filter(|a| a.len() == 2).ok_or_else(|| bad(C, "tilt must be [x,y] degrees"))?;
                let component =
                    |v: &Value| v.as_f64().filter(|n| n.is_finite() && n.abs() <= 90.0).ok_or_else(|| bad(C, "tilt components must be -90..90"));
                let x = component(&a[0])?.clamp(-89.9, 89.9).to_radians().tan();
                let y = component(&a[1])?.clamp(-89.9, 89.9).to_radians().tan();
                let length = x.hypot(y);
                // Upright -> round, tilted -> ellipse. Azimuth is NOT barrel rotation.
                let angle = if length < 1e-6 { fallback_angle } else { -y.atan2(x).to_degrees() };
                (angle, (1.0 / (1.0 + length * length).sqrt()).max(roundness))
            }
        };
        let t = cum[i] / total;
        if pressure {
            let width = floor + (1.0 - floor) * measured.powf(gamma);
            widths.points.push((t, width, width));
        }
        if tilt {
            nibs.points.push((t, angle, ratio));
        }
    }
    st.profile = pressure.then_some(widths);
    st.nib = tilt.then_some(nibs);
    st.brush = Some(vectorcraft_brush::EXPRESSIVE_PEN.into());
    Ok(())
}
