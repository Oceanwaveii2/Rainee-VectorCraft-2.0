//! Original procedural pencil and dry-ink nib fibers over an editable centerline.
//!
//! `BrushKind::Natural` routes through this module; callers retain the source path and
//! regenerate after edits through the existing render/export brush pipeline.
//! Each fiber is a filled path of disconnected, tapered contact patches, not a raster asset.

use serde::{Deserialize, Serialize};
use vectorcraft_color::Paint;
use vectorcraft_doc::{Node, StrokeLayer};
use vectorcraft_geom::{BezPath, ParamCurve, PathData, PathEl, PathSeg, Point, SubPath, Vec2};

use crate::track::{Rng, normal};

/// Maximum accepted input elements. Oversized paths return an error rather than a partial stroke.
pub const MAX_INPUT_ELEMENTS: usize = 256;
/// Maximum output nodes; the requested fiber count is clamped to 1..=16.
pub const MAX_FIBERS: usize = 16;
/// Global subdivision budget, shared across all input segments (including closing segments).
pub const MAX_CHORDS: usize = 512;
/// Maximum total closed quadrilaterals across all output nodes.
pub const MAX_PATCHES: usize = MAX_CHORDS * MAX_FIBERS;
/// Maximum supported nib width in document points.
pub const MAX_WIDTH: f64 = 4096.0;
/// Maximum absolute input coordinate, keeping distance and offset arithmetic finite.
pub const MAX_COORDINATE: f64 = 1.0e9;
/// Maximum input samples in each width or nib profile, matching the expressive sample cap.
/// Derived knots still share at most 256 insertions within the 512-chord geometry budget.
pub const MAX_PROFILE_POINTS: usize = 16_384;
/// Maximum accepted width multiplier after clamping (per side).
pub const MAX_PROFILE_FACTOR: f64 = 16.0;
/// Ordinary calligraphic fallback budgets (profile/geometry failures only).
pub const MAX_FALLBACK_POINTS: usize = 256;
pub const MAX_FALLBACK_SUBPATHS: usize = 32;
/// Conservative bound for corner fan points plus open caps in the fallback output.
pub const MAX_FALLBACK_ANCHORS: usize = 40 * MAX_FALLBACK_POINTS + 32 * MAX_FALLBACK_SUBPATHS;

/// Procedural contact model. Both modes use the supplied paint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NaturalMedia {
    /// Thin, near-continuous contacts with fine edge breaks and translucent deposition.
    #[default]
    Pencil,
    /// Wider, joined contacts with fine edge breaks and denser deposition.
    DryInk,
}

/// Geometry parameters; width is the whole nib envelope, not an individual fiber's width.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NaturalBrush {
    pub media: NaturalMedia,
    /// Finite width in (0, MAX_WIDTH]. Direct geometry uses points; BrushKind multiplies it by
    /// StrokeLayer::width, as for the existing calligraphic/bristle definitions.
    pub width: f64,
    /// Persistent caller-owned seed. Path edits do not reseed the fibers.
    pub seed: u64,
    /// Requested fiber count, clamped to 1..=MAX_FIBERS.
    pub fibers: usize,
    /// Zero retains the original fiber renderer. Positive values enable bounded granular
    /// deposition with document-space paper modulation. This is not wet-media simulation.
    #[serde(skip_serializing_if = "no_grain")]
    pub grain: f64,
    /// Coverage multiplier, not a measured pigment quantity. Neutral is one.
    #[serde(rename = "inkLoad", skip_serializing_if = "one")]
    pub ink_load: f64,
    /// Procedural edge darkening inside the nib envelope, not fluid transport.
    #[serde(rename = "edgePooling", skip_serializing_if = "no_grain")]
    pub edge_pooling: f64,
    /// Seeded bristle contact dropout. Zero keeps legacy contact coverage.
    #[serde(skip_serializing_if = "no_grain")]
    pub breakup: f64,
    /// Document-space paper wavelength multiplier. It does not follow viewport zoom.
    #[serde(rename = "paperScale", skip_serializing_if = "one")]
    pub paper_scale: f64,
}

fn one(value: &f64) -> bool {
    *value == 1.0
}

fn no_grain(value: &f64) -> bool {
    *value == 0.0
}

impl Default for NaturalBrush {
    fn default() -> Self {
        Self {
            media: NaturalMedia::Pencil,
            width: 3.0,
            seed: 0,
            fibers: 10,
            grain: 0.0,
            ink_load: 1.0,
            edge_pooling: 0.0,
            breakup: 0.0,
            paper_scale: 1.0,
        }
    }
}

/// Invalid input is reported without emitting truncated or non-finite geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NaturalError {
    InvalidWidth,
    TooManyElements,
    InvalidCoordinate,
    InvalidPath,
    InvalidProfile,
}

impl std::fmt::Display for NaturalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidWidth => "natural brush width must be finite and in (0, 4096]",
            Self::TooManyElements => "natural brush centerline exceeds 256 path elements",
            Self::InvalidCoordinate => "natural brush coordinates must be finite and within +/-1e9",
            Self::InvalidPath => "natural brush drawing elements require an open subpath",
            Self::InvalidProfile => {
                "natural brush profiles require at most 16384 sorted finite samples in [0,1], nonnegative widths and nib ratios in [0.01,1]"
            }
        })
    }
}

impl std::error::Error for NaturalError {}

fn valid_point(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= MAX_COORDINATE && p.y.abs() <= MAX_COORDINATE
}

fn validate(path: &BezPath, width: f64) -> Result<(), NaturalError> {
    if !width.is_finite() || width <= 0.0 || width > MAX_WIDTH {
        return Err(NaturalError::InvalidWidth);
    }
    if path.elements().len() > MAX_INPUT_ELEMENTS {
        return Err(NaturalError::TooManyElements);
    }
    validate_coordinates(path)
}

fn validate_coordinates(path: &BezPath) -> Result<(), NaturalError> {
    let mut open = false;
    for el in path.elements() {
        let finite = match *el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => valid_point(p),
            PathEl::QuadTo(a, b) => valid_point(a) && valid_point(b),
            PathEl::CurveTo(a, b, c) => valid_point(a) && valid_point(b) && valid_point(c),
            PathEl::ClosePath => true,
        };
        if !finite {
            return Err(NaturalError::InvalidCoordinate);
        }
        match el {
            PathEl::MoveTo(_) => open = true,
            _ if !open => return Err(NaturalError::InvalidPath),
            PathEl::ClosePath => open = false,
            _ => {}
        }
    }
    Ok(())
}

fn subpaths(path: &BezPath) -> impl Iterator<Item = &[PathEl]> {
    let mut start = 0;
    std::iter::from_fn(move || {
        let remaining = path.elements().get(start..)?;
        if remaining.is_empty() {
            return None;
        }
        let end = remaining.iter().enumerate().skip(1).find(|(_, el)| matches!(el, PathEl::MoveTo(_))).map_or(remaining.len(), |(i, _)| i);
        start += end;
        remaining.get(..end)
    })
}

/// Bounded ordinary round calligraphic fallback. It samples the source curves into at most
/// 256 polyline vertices. If there are more than 32 drawable subpaths, 32 are distributed
/// across the source including its first and last; intermediate subpaths are omitted. Profiles
/// are not used in this fallback. Finite oversized widths clamp to 4096, but invalid widths,
/// coordinates and path structure never enter the ordinary calligraphic implementation.
pub(crate) fn fallback_geometry(path: &BezPath, brush: &NaturalBrush, stroke: &StrokeLayer) -> Vec<Node> {
    let width = brush.width * stroke.width;
    if !brush.width.is_finite()
        || brush.width <= 0.0
        || !stroke.width.is_finite()
        || stroke.width <= 0.0
        || !width.is_finite()
        || validate_coordinates(path).is_err()
    {
        return Vec::new();
    }
    let drawable = || subpaths(path).filter(|els| kurbo::segments(els.iter().copied()).next().is_some());
    let count = drawable().count();
    if count == 0 {
        return Vec::new();
    }
    let kept = count.min(MAX_FALLBACK_SUBPATHS);
    let quota = MAX_FALLBACK_POINTS / kept;
    let mut simplified = BezPath::new();
    let mut selected: usize = 0;
    for (group, els) in drawable().enumerate() {
        let wanted = if kept == 1 { 0 } else { selected.saturating_mul(count - 1) / (kept - 1) };
        if selected >= kept || group != wanted {
            continue;
        }
        selected += 1;
        let segments = kurbo::segments(els.iter().copied());
        let segment_count = segments.clone().count();
        let Some(first) = segments.clone().next() else { continue };
        simplified.move_to(first.start());
        let room = quota - 1;
        if segment_count <= room {
            let steps = (room / segment_count).clamp(1, 16);
            for seg in segments {
                for step in 1..=steps {
                    simplified.line_to(seg.eval(step as f64 / steps as f64));
                }
            }
        } else {
            for (i, seg) in segments.enumerate() {
                if (i + 1).saturating_mul(room) / segment_count > i.saturating_mul(room) / segment_count {
                    simplified.line_to(seg.end());
                }
            }
        }
        if matches!(els.last(), Some(PathEl::ClosePath)) {
            simplified.close_path();
        }
    }
    let plain = StrokeLayer::new(stroke.paint.clone(), 1.0);
    let nib = crate::Calligraphic { size: width.clamp(0.01, MAX_WIDTH), ..Default::default() };
    crate::calli::calligraphic(&nib, &simplified, 1.0, &stroke.paint, "Natural bounded fallback", &plain)
}

fn control_length(seg: &PathSeg) -> f64 {
    match seg {
        PathSeg::Line(s) => s.p0.distance(s.p1),
        PathSeg::Quad(s) => s.p0.distance(s.p1) + s.p1.distance(s.p2),
        PathSeg::Cubic(s) => s.p0.distance(s.p1) + s.p1.distance(s.p2) + s.p2.distance(s.p3),
    }
}

fn validate_profiles(stroke: &StrokeLayer) -> Result<(), NaturalError> {
    for (points, nib) in [(stroke.profile.as_ref().map(|p| p.points.as_slice()), false), (stroke.nib.as_ref().map(|p| p.points.as_slice()), true)] {
        let Some(points) = points else { continue };
        if points.len() > MAX_PROFILE_POINTS {
            return Err(NaturalError::InvalidProfile);
        }
        let mut previous = 0.0;
        for &(t, a, b) in points {
            if !t.is_finite()
                || !(previous..=1.0).contains(&t)
                || !a.is_finite()
                || !b.is_finite()
                || if nib { a.abs() > MAX_COORDINATE || !(0.01..=1.0).contains(&b) } else { a < 0.0 || b < 0.0 }
            {
                return Err(NaturalError::InvalidProfile);
            }
            previous = t;
        }
    }
    Ok(())
}

/// Like [`geometry`], with base width multiplied by `stroke.width` and editable profiles.
///
/// Width and nib positions are fractions of each subpath's sampled arc length. Width factors
/// are asymmetric, clamped to 0..16, with each side capped at MAX_WIDTH / 2. Zero stroke width
/// emits no art; negative/non-finite widths and invalid/oversized profiles return an error.
/// The nib angle/ratio contributes the ellipse's transverse support only: no tangential caps,
/// tilt-directed fiber rotation, or physics. Up to 256 profile knots are inserted across all
/// subpaths, within the 512-chord budget; dense profiles/curves are approximated. Width steps
/// use before/after values at sampled knots, but unsampled steps interpolate across a chord.
/// Dashes, arrowheads and non-center stroke alignment are not implemented by this contact model.
pub fn stroke_geometry(path: &BezPath, brush: &NaturalBrush, stroke: &StrokeLayer) -> Result<Vec<Node>, NaturalError> {
    validate(path, brush.width)?;
    validate_profiles(stroke)?;
    if !stroke.width.is_finite() || stroke.width < 0.0 {
        return Err(NaturalError::InvalidWidth);
    }
    if stroke.width == 0.0 {
        return Ok(Vec::new());
    }
    let scaled = NaturalBrush { width: brush.width * stroke.width, ..*brush };
    validate(path, scaled.width)?;
    geometry_impl(path, &scaled, &stroke.paint, Some(stroke))
}

#[derive(Clone, Copy)]
struct Chord {
    a: Point,
    b: Point,
    t0: f64,
    t1: f64,
    distance: f64,
}

fn chords(path: &BezPath, width: f64, stroke: Option<&StrokeLayer>) -> Vec<Chord> {
    let mut groups = Vec::<Vec<PathSeg>>::new();
    let mut current = BezPath::new();
    let flush = |current: &mut BezPath, groups: &mut Vec<Vec<PathSeg>>| {
        let segments: Vec<_> = current.segments().collect();
        if !segments.is_empty() {
            groups.push(segments);
        }
        *current = BezPath::new();
    };
    for &el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) {
            flush(&mut current, &mut groups);
        }
        current.push(el);
        if matches!(el, PathEl::ClosePath) {
            flush(&mut current, &mut groups);
        }
    }
    flush(&mut current, &mut groups);
    let count: usize = groups.iter().map(Vec::len).sum();
    if count == 0 {
        return Vec::new();
    }
    let mut knots: Vec<_> = stroke
        .into_iter()
        .flat_map(|s| s.profile.iter().flat_map(|p| p.points.iter().map(|p| p.0)).chain(s.nib.iter().flat_map(|p| p.points.iter().map(|p| p.0))))
        .filter(|t| *t > 0.0 && *t < 1.0)
        .collect();
    knots.sort_by(f64::total_cmp);
    knots.dedup();
    let quota = (MAX_CHORDS / 2) / groups.len().max(1);
    // Spread a limited knot quota across the profile rather than dropping the later portion.
    let used = knots.len().min(quota);
    let knots: Vec<_> = (0..used).filter_map(|i| knots.get(i * knots.len() / used).copied()).collect();
    // Reserve only the knots we actually insert; ordinary lens profiles should not halve
    // sampling density. Fine contacts keep their density when the stroke is widened.
    let budget = MAX_CHORDS - used * groups.len();
    let per_segment = (budget / count).clamp(1, 64);
    let mut out = Vec::new();
    let mut global_distance = 0.0;
    for group in groups {
        let mut base = Vec::new();
        let mut total = 0.0;
        for seg in group {
            let steps = (control_length(&seg) / (width * 0.15).clamp(0.01, 1.0)).ceil().clamp(1.0, per_segment as f64) as usize;
            for step in 0..steps {
                let a = seg.eval(step as f64 / steps as f64);
                let b = seg.eval((step + 1) as f64 / steps as f64);
                let length = a.distance(b);
                if length > 1e-9 {
                    base.push((a, b, total, length));
                    total += length;
                }
            }
        }
        for (a, b, at, length) in base {
            let t0 = at / total;
            let t1 = (at + length) / total;
            if t1 <= t0 {
                // A short chord after a huge distance can lose its fractional interval to
                // rounding. Preserve its finite geometry without dividing by that interval.
                out.push(Chord { a, b, t0, t1, distance: global_distance + at });
                continue;
            }
            let mut from = t0;
            for to in knots.iter().copied().filter(|t| *t > t0 && *t < t1).chain(std::iter::once(t1)) {
                let u0 = ((from - t0) / (t1 - t0)).clamp(0.0, 1.0);
                let u1 = ((to - t0) / (t1 - t0)).clamp(0.0, 1.0);
                out.push(Chord { a: a.lerp(b, u0), b: a.lerp(b, u1), t0: from, t1: to, distance: global_distance + at + u0 * length });
                from = to;
            }
        }
        global_distance += total;
    }
    out
}

fn side_factors(stroke: Option<&StrokeLayer>, t: f64, before: bool, n: Vec2, width: f64) -> (f64, f64) {
    let Some(stroke) = stroke else { return (1.0, 1.0) };
    let (left, right) = stroke.profile.as_ref().map_or((1.0, 1.0), |p| {
        let (a, b) = p.around(t);
        if before { a } else { b }
    });
    let nib = stroke.nib.as_ref().map_or(1.0, |p| {
        let (angle, ratio) = p.at(t);
        let a = -angle.to_radians();
        let projected = n.dot(Vec2::new(a.cos(), a.sin()));
        (projected.powi(2) + ratio.powi(2) * (1.0 - projected.powi(2)).max(0.0)).sqrt().clamp(0.01, 1.0)
    });
    let limit = (MAX_WIDTH / width).min(MAX_PROFILE_FACTOR);
    (left.min(limit) * nib, right.min(limit) * nib)
}

/// Generate filled path nodes in document space, with id 0 and no recursive brush appearance.
///
/// The path and paint are borrowed and never modified. Empty, move-only, and zero-length paths
/// produce `Ok(vec![])`. Invalid inputs return [`NaturalError`]. At most [`MAX_FIBERS`] nodes
/// contain [`MAX_PATCHES`] closed four-anchor patches in total. Every point lies within
/// `width / 2` of a sampled chord; therefore bounds stay inside the source control-point bounds
/// inflated by `width / 2`. There are no miter spikes or joins across separate subpaths.
///
/// Curves use width-dependent uniform parameter subdivisions capped at 64 per segment and a
/// share of [`MAX_CHORDS`]. Long/complex paths become coarser, but all segments are processed.
/// This is a bounded approximation, not a curve-flatness or natural-media fidelity guarantee.
/// Identical inputs reproduce identical geometry on the same floating-point platform.
pub fn geometry(path: &BezPath, brush: &NaturalBrush, paint: &Paint) -> Result<Vec<Node>, NaturalError> {
    validate(path, brush.width)?;
    geometry_impl(path, brush, paint, None)
}

fn geometry_impl(path: &BezPath, brush: &NaturalBrush, paint: &Paint, stroke: Option<&StrokeLayer>) -> Result<Vec<Node>, NaturalError> {
    if !brush.grain.is_finite()
        || !brush.ink_load.is_finite()
        || !(0.0..=2.0).contains(&brush.ink_load)
        || !brush.edge_pooling.is_finite()
        || !(0.0..=1.0).contains(&brush.edge_pooling)
        || !brush.breakup.is_finite()
        || !(0.0..=1.0).contains(&brush.breakup)
        || !brush.paper_scale.is_finite()
        || !(0.125..=128.0).contains(&brush.paper_scale)
    {
        return Err(NaturalError::InvalidProfile);
    }
    let samples = chords(path, brush.width, stroke);
    if samples.is_empty() {
        return Ok(Vec::new());
    }
    if brush.grain > 0.0 || brush.ink_load != 1.0 || brush.edge_pooling > 0.0 || brush.breakup > 0.0 || brush.paper_scale != 1.0 {
        return Ok(deposition(&samples, brush, paint, stroke));
    }
    let fibers = brush.fibers.clamp(1, MAX_FIBERS);
    let mut rng = Rng::new(brush.seed);
    let mut out = Vec::with_capacity(fibers);
    for fiber in 0..fibers {
        // Stratified, irregular lanes retain coverage without equally spaced parallel rails.
        let lane = (fiber as f64 + 0.5 + rng.range(-0.32, 0.32)) / fibers as f64 * 2.0 - 1.0;
        let phase = rng.range(0.0, std::f64::consts::TAU);
        let grain_phase = rng.range(0.0, std::f64::consts::TAU);
        let (thickness, opacity, gap_size) = match brush.media {
            NaturalMedia::Pencil => (rng.range(0.40, 0.66), rng.range(0.28, 0.65), 0.015),
            NaturalMedia::DryInk => (rng.range(0.70, 0.96), rng.range(0.65, 0.95), 0.008),
        };
        let half = brush.width * thickness / (2.0 * fibers as f64);
        // Envelope room is reserved for both the fiber's half-width and its wandering center.
        let travel = (brush.width * 0.5 - half).max(0.0);
        let mut subs = Vec::new();
        for (index, chord) in samples.iter().enumerate() {
            let (a, b) = (chord.a, chord.b);
            let length = a.distance(b);
            if length <= 1e-9 {
                continue;
            }
            let n = normal((b - a) / length);
            let join_normal = |neighbor: Option<&Chord>, before: bool| {
                let Some(other) = neighbor else { return n };
                // Do not join across a MoveTo/erased gap, or across a subpath's reset fraction.
                let joined = if before {
                    other.b.distance(a) <= 1e-8 && (other.t1 - chord.t0).abs() <= 1e-12
                } else {
                    b.distance(other.a) <= 1e-8 && (chord.t1 - other.t0).abs() <= 1e-12
                };
                if !joined {
                    return n;
                }
                let delta = other.b - other.a;
                let neighbor_length = delta.hypot();
                if neighbor_length <= 1e-9 {
                    return n;
                }
                let blended = n + normal(delta / neighbor_length);
                let magnitude = blended.hypot();
                if magnitude <= 1e-9 { n } else { blended / magnitude }
            };
            let n0 = join_normal(index.checked_sub(1).and_then(|i| samples.get(i)), true);
            let n1 = join_normal(samples.get(index + 1), false);
            // Keep the frequency finite even for a positive subnormal width.
            let scale = brush.width.max(0.01);
            let wobble = |s: f64| {
                let roughness = 0.035 + 0.08 * lane.abs();
                ((phase + s / scale * 0.8).sin() + (grain_phase + s / scale * 3.7).sin() * 0.35) * roughness
            };
            let offset0 = (lane * 0.92 + wobble(chord.distance)).clamp(-1.0, 1.0) * travel;
            let offset1 = (lane * 0.92 + wobble(chord.distance + length)).clamp(-1.0, 1.0) * travel;
            // Core fibers join every chord. Only outer fibers get rare, independently seeded
            // micro-breaks, capped in absolute width units AND at 4% of the chord length.
            // No whole-chord dropout and no all-fiber seam at common sample boundaries.
            let gap = if lane.abs() > 0.65 && rng.unit() < 0.12 { (brush.width * gap_size).min(length * 0.04) / length } else { 0.0 };
            let start = rng.range(0.0, gap);
            let end = 1.0 - (gap - start);
            let factors0 = side_factors(stroke, chord.t0, false, n0, brush.width);
            let factors1 = side_factors(stroke, chord.t1, true, n1, brush.width);
            let edge = |u: f64, d: f64| {
                let factor0 = if d >= 0.0 { factors0.0 } else { factors0.1 };
                let factor1 = if d >= 0.0 { factors1.0 } else { factors1.1 };
                a.lerp(b, u) + n0 * (d * factor0 * (1.0 - u)) + n1 * (d * factor1 * u)
            };
            let off0 = offset0 + (offset1 - offset0) * start;
            let off1 = offset0 + (offset1 - offset0) * end;
            // Continuous grain gives adjacent contacts the same thickness at a shared end.
            let grain = |s: f64| 0.72 + 0.18 * (grain_phase + s / scale * 2.3).sin() + 0.10 * (phase + s / scale * 6.1).sin();
            let h0 = half * grain(chord.distance);
            let h1 = half * grain(chord.distance + length);
            subs.push(SubPath::polyline(&[edge(start, off0 + h0), edge(end, off1 + h1), edge(end, off1 - h1), edge(start, off0 - h0)], true));
        }
        if !subs.is_empty() {
            let mut node = crate::filled(PathData::new(subs), paint);
            node.opacity = opacity as f32;
            out.push(node);
        }
    }
    Ok(out)
}

/// Paper modulation is anchored in document coordinates, not the viewport. The generated
/// product consists of bounded filled vector cells; source paths/profiles remain authoritative.
fn paper(p: Point, seed: u64) -> f64 {
    let x = (p.x * 2.0).floor() as i64 as u64;
    let y = (p.y * 2.0).floor() as i64 as u64;
    let mut h = x.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ y.rotate_left(29) ^ seed;
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((h ^ (h >> 31)) >> 11) as f64 / ((1_u64 << 53) as f64)
}

fn deposition(samples: &[Chord], brush: &NaturalBrush, paint: &Paint, stroke: Option<&StrokeLayer>) -> Vec<Node> {
    let grain = brush.grain.clamp(0.0, 1.0);
    let lanes = brush.fibers.clamp(1, MAX_FIBERS);
    let mut bins: Vec<Vec<SubPath>> = (0..MAX_FIBERS).map(|_| Vec::new()).collect();
    let mut rng = Rng::new(brush.seed);
    let per_chord = (MAX_PATCHES / samples.len().max(1) / lanes).max(1);
    for chord in samples {
        let length = chord.a.distance(chord.b);
        if length <= 1e-9 {
            continue;
        }
        let direction = (chord.b - chord.a) / length;
        let n = normal(direction);
        let steps = ((length / 0.65).ceil() as usize).clamp(1, per_chord.min(64));
        for step in 0..steps {
            let u = (step as f64 + 0.5) / steps as f64;
            let t = chord.t0 + (chord.t1 - chord.t0) * u;
            let factors = side_factors(stroke, t, false, n, brush.width);
            let half_left = brush.width * 0.5 * factors.0;
            let half_right = brush.width * 0.5 * factors.1;
            let envelope = half_left + half_right;
            if envelope <= 1e-9 {
                continue;
            }
            let cell_width = envelope / lanes as f64;
            for lane in 0..lanes {
                let transverse = (lane as f64 + rng.range(0.15, 0.85)) / lanes as f64;
                let center = chord.a.lerp(chord.b, u) + n * (-half_right + envelope * transverse);
                let tooth = paper(Point::new(center.x / brush.paper_scale, center.y / brush.paper_scale), brush.seed);
                // Removing contact is opt-in and does not consume random values for legacy brushes.
                if brush.breakup > 0.0 && rng.unit() < brush.breakup * (0.15 + 0.65 * tooth) {
                    continue;
                }
                // Width profiles act as a deposition proxy, not invented raw sensor pressure.
                let load = ((factors.0 + factors.1) * 0.5).clamp(0.0, 1.0);
                let base = match brush.media {
                    NaturalMedia::Pencil => 0.28,
                    NaturalMedia::DryInk => 0.8,
                };
                let edge = (2.0 * transverse - 1.0).abs().powi(3);
                let alpha =
                    (base * (0.3 + 0.7 * load) * brush.ink_load * (1.0 - grain * tooth * 0.9) * (1.0 + brush.edge_pooling * edge)).clamp(0.0, 1.0);
                if alpha <= 0.0 {
                    continue;
                }
                let bin = ((alpha * MAX_FIBERS as f64).ceil() as usize).clamp(1, MAX_FIBERS) - 1;
                let cross_half = cell_width * rng.range(0.40, 0.70);
                let lo = (-half_right).max(-half_right + envelope * transverse - cross_half);
                let hi = half_left.min(-half_right + envelope * transverse + cross_half);
                let shifted = ((step as f64 + rng.range(0.28, 0.72)) / steps as f64).clamp(0.0, 1.0);
                let mid = chord.a.lerp(chord.b, shifted);
                let along = (length / steps as f64 * rng.range(0.45, 0.75)).min(length * shifted).min(length * (1.0 - shifted));
                let a = mid - direction * along;
                let b = mid + direction * along;
                if let Some(bucket) = bins.get_mut(bin) {
                    // Rounded, irregular footprints avoid aligned rectangular "barcode" cells.
                    let middle = (hi + lo) * 0.5;
                    let shoulder = along * rng.range(0.35, 0.65);
                    bucket.push(SubPath::polyline(
                        &[
                            a + n * middle,
                            mid - direction * shoulder + n * hi,
                            mid + direction * shoulder + n * hi,
                            b + n * middle,
                            mid + direction * shoulder + n * lo,
                            mid - direction * shoulder + n * lo,
                        ],
                        true,
                    ));
                }
            }
        }
    }
    bins.into_iter()
        .enumerate()
        .filter_map(|(i, subs)| {
            if subs.is_empty() {
                return None;
            }
            let mut node = crate::filled(PathData::new(subs), paint);
            node.opacity = (i + 1) as f32 / MAX_FIBERS as f32;
            Some(node)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_color::Color;
    use vectorcraft_geom::{Rect, Shape};

    fn line() -> BezPath {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((100.0, 0.0));
        p
    }

    #[test]
    fn granular_deposition_is_deterministic_bounded_and_distinct() {
        let b = NaturalBrush { grain: 0.9, fibers: MAX_FIBERS, ..Default::default() };
        let out = generate(&line(), b);
        assert_eq!(signature(&out), signature(&generate(&line(), b)));
        assert_ne!(signature(&out), signature(&generate(&line(), NaturalBrush::default())));
        assert!(out.len() <= MAX_FIBERS);
        let patches: usize = out.iter().map(|n| n.path_data().unwrap().subpaths.len()).sum();
        assert!(patches <= MAX_PATCHES);
        assert!(out.len() > 1, "paper tooth should vary deposition opacity");
        assert!(bounds(&out).y0 >= -b.width * 0.5 && bounds(&out).y1 <= b.width * 0.5);
        assert_ne!(signature(&out), signature(&generate(&line(), NaturalBrush { grain: 0.2, ..b })));
    }

    #[test]
    fn editable_ink_controls_are_deterministic_bounded_and_independent() {
        let b = NaturalBrush { media: NaturalMedia::DryInk, grain: 0.5, width: 12.0, ..Default::default() };
        let original = generate(&line(), b);
        for change in [
            NaturalBrush { ink_load: 1.6, ..b },
            NaturalBrush { edge_pooling: 1.0, ..b },
            NaturalBrush { breakup: 0.8, ..b },
            NaturalBrush { paper_scale: 4.0, ..b },
        ] {
            let first = generate(&line(), change);
            assert_ne!(signature(&original), signature(&first));
            assert_eq!(signature(&first), signature(&generate(&line(), change)));
            assert!(first.len() <= MAX_FIBERS);
            let patches: usize = first.iter().map(|n| n.path_data().unwrap().subpaths.len()).sum();
            assert!(patches <= MAX_PATCHES);
        }
        assert!(generate(&line(), NaturalBrush { ink_load: 0.0, ..b }).is_empty());
        for bad in [
            NaturalBrush { ink_load: 3.0, ..b },
            NaturalBrush { edge_pooling: f64::NAN, ..b },
            NaturalBrush { breakup: -1.0, ..b },
            NaturalBrush { paper_scale: 0.0, ..b },
        ] {
            assert!(geometry(&line(), &bad, &Paint::solid(Color::BLACK)).is_err());
        }
    }

    #[test]
    fn paper_and_legacy_defaults_are_stable() {
        let p = Point::new(-20.2, 50.3);
        assert_eq!(paper(p, 42), paper(p, 42));
        assert_ne!(paper(p, 42), paper(p, 43));
        let old: NaturalBrush = serde_json::from_str(r#"{"media":"pencil","width":3,"seed":0,"fibers":10}"#).unwrap();
        assert_eq!(old.grain, 0.0);
        assert_eq!(signature(&generate(&line(), old)), signature(&generate(&line(), NaturalBrush::default())));
        assert!(geometry(&line(), &NaturalBrush { grain: f64::NAN, ..old }, &Paint::solid(Color::BLACK)).is_err());
    }

    #[test]
    fn granular_profiles_change_deposition_and_keep_erased_subpaths_separate() {
        use vectorcraft_doc::WidthProfile;
        let b = NaturalBrush { grain: 0.9, width: 8.0, ..Default::default() };
        let mut s = StrokeLayer::new(Paint::solid(Color::BLACK), 1.0);
        s.profile = Some(WidthProfile { points: vec![(0.0, 0.2, 0.2), (1.0, 0.2, 0.2)] });
        let light = stroke_geometry(&line(), &b, &s).unwrap();
        s.profile = Some(WidthProfile { points: vec![(0.0, 1.0, 1.0), (1.0, 1.0, 1.0)] });
        let heavy = stroke_geometry(&line(), &b, &s).unwrap();
        let mean = |nodes: &[Node]| nodes.iter().map(|n| n.opacity as f64).sum::<f64>() / nodes.len().max(1) as f64;
        assert!(mean(&heavy) > mean(&light));
        assert!(bounds(&heavy).height() > bounds(&light).height() * 4.0);
        let mut split = BezPath::new();
        split.move_to((0.0, 0.0));
        split.line_to((40.0, 0.0));
        split.move_to((60.0, 0.0));
        split.line_to((100.0, 0.0));
        for node in stroke_geometry(&split, &b, &s).unwrap() {
            for patch in &node.path_data().unwrap().subpaths {
                assert!(patch.anchors.iter().all(|a| a.p.x <= 40.0) || patch.anchors.iter().all(|a| a.p.x >= 60.0));
            }
        }
    }

    fn generate(p: &BezPath, b: NaturalBrush) -> Vec<Node> {
        geometry(p, &b, &Paint::solid(Color::rgb(0.1, 0.1, 0.1))).unwrap()
    }

    fn signature(nodes: &[Node]) -> Vec<(BezPath, f32)> {
        nodes.iter().map(|n| (n.path_data().unwrap().to_bezpath(), n.opacity)).collect()
    }

    fn bounds(nodes: &[Node]) -> Rect {
        nodes.iter().map(|n| n.path_data().unwrap().to_bezpath().bounding_box()).reduce(|a, b| a.union(b)).unwrap()
    }

    #[test]
    fn deterministic_seed_and_distinct_media() {
        for media in [NaturalMedia::Pencil, NaturalMedia::DryInk] {
            let b = NaturalBrush { media, seed: 42, ..Default::default() };
            let first = signature(&generate(&line(), b));
            assert!(!first.is_empty());
            assert_eq!(first, signature(&generate(&line(), b)));
            assert_ne!(first, signature(&generate(&line(), NaturalBrush { seed: 43, ..b })));
        }
        assert_ne!(
            signature(&generate(&line(), NaturalBrush::default())),
            signature(&generate(&line(), NaturalBrush { media: NaturalMedia::DryInk, ..Default::default() }))
        );
    }

    #[test]
    fn width_changes_geometry_and_envelope() {
        for media in [NaturalMedia::Pencil, NaturalMedia::DryInk] {
            let narrow = generate(&line(), NaturalBrush { media, width: 2.0, ..Default::default() });
            let wide = generate(&line(), NaturalBrush { media, width: 12.0, ..Default::default() });
            assert_ne!(signature(&narrow), signature(&wide));
            assert!(bounds(&wide).height() > bounds(&narrow).height() * 4.0);
            assert!(bounds(&wide).y0 >= -6.0 && bounds(&wide).y1 <= 6.0);
        }
    }

    #[test]
    fn path_edits_recompute_without_mutation() {
        let original = line();
        let before = original.clone();
        let mut edited = BezPath::new();
        edited.move_to((0.0, 0.0));
        edited.curve_to((20.0, 80.0), (70.0, -20.0), (100.0, 20.0));
        assert_ne!(signature(&generate(&original, NaturalBrush::default())), signature(&generate(&edited, NaturalBrush::default())));
        assert_eq!(original, before);
    }

    #[test]
    fn bounded_complex_geometry_and_no_subpath_bridges() {
        let mut p = BezPath::new();
        for i in 0..85 {
            let x = i as f64 * 10000.0;
            p.move_to((x, 0.0));
            p.quad_to((x + 5.0, 40.0), (x + 10.0, 0.0));
            p.close_path();
        }
        for media in [NaturalMedia::Pencil, NaturalMedia::DryInk] {
            let nodes = generate(&p, NaturalBrush { media, width: 8.0, fibers: usize::MAX, ..Default::default() });
            assert!(nodes.len() <= MAX_FIBERS);
            let mut patches = 0;
            let mut saw_last_subpath = false;
            for n in nodes {
                assert_eq!(n.id, vectorcraft_doc::NodeId(0));
                assert!((0.0..=1.0).contains(&n.opacity));
                let bp = n.path_data().unwrap().to_bezpath();
                patches += bp.elements().iter().filter(|e| matches!(e, PathEl::ClosePath)).count();
                for seg in bp.segments() {
                    // A patch cannot bridge the 10,000-point gap between source subpaths.
                    // Full contacts can span the source's 10x40 control box plus the 8-point
                    // nib envelope; the former shortened patches happened to stay below 20.
                    assert!(seg.start().distance(seg.end()) <= 50.0);
                    assert!(valid_point(seg.start()));
                    assert!((-4.0..=44.0).contains(&seg.start().y));
                    saw_last_subpath |= seg.start().x > 839990.0;
                }
            }
            assert!(patches > 0 && patches <= MAX_PATCHES);
            assert!(saw_last_subpath, "budget must cover the whole centerline");
        }
    }

    #[test]
    fn empty_degenerate_and_invalid_inputs() {
        let paint = Paint::solid(Color::rgb(0.0, 0.0, 0.0));
        let b = NaturalBrush::default();
        assert!(generate(&BezPath::new(), b).is_empty());
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        assert!(generate(&p, b).is_empty());
        p.line_to((0.0, 0.0));
        assert!(generate(&p, b).is_empty());
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY, MAX_WIDTH + 1.0] {
            assert!(matches!(geometry(&line(), &NaturalBrush { width, ..b }, &paint), Err(NaturalError::InvalidWidth)));
        }
        for x in [f64::NAN, f64::INFINITY, MAX_COORDINATE + 1.0] {
            let mut bad = BezPath::new();
            bad.move_to((x, 0.0));
            assert!(matches!(geometry(&bad, &b, &paint), Err(NaturalError::InvalidCoordinate)));
        }
        let bad = BezPath::from_vec(vec![PathEl::MoveTo(Point::ZERO), PathEl::ClosePath, PathEl::LineTo(Point::new(1.0, 1.0))]);
        assert!(matches!(geometry(&bad, &b, &paint), Err(NaturalError::InvalidPath)));
        let mut large = line();
        for i in 0..MAX_INPUT_ELEMENTS {
            large.line_to((i as f64, 1.0));
        }
        assert!(matches!(geometry(&large, &b, &paint), Err(NaturalError::TooManyElements)));
        assert!(!generate(&line(), NaturalBrush { fibers: 0, ..b }).is_empty());
    }

    #[test]
    fn extreme_valid_widths_and_coordinates_stay_finite() {
        let mut p = BezPath::new();
        p.move_to((-MAX_COORDINATE, -MAX_COORDINATE));
        p.curve_to((MAX_COORDINATE, -MAX_COORDINATE), (-MAX_COORDINATE, MAX_COORDINATE), (MAX_COORDINATE, MAX_COORDINATE));
        for width in [f64::from_bits(1), 0.01, MAX_WIDTH] {
            let nodes = generate(&p, NaturalBrush { width, ..Default::default() });
            assert!(!nodes.is_empty());
            for n in nodes {
                for (_, _, a) in n.path_data().unwrap().anchors() {
                    assert!(a.p.x.is_finite() && a.p.y.is_finite());
                    assert!(a.p.x.abs() <= MAX_COORDINATE + width * 0.5);
                    assert!(a.p.y.abs() <= MAX_COORDINATE + width * 0.5);
                }
            }
        }
    }

    #[test]
    fn natural_defaults_and_public_api_persist_original_json_fixtures() {
        use crate::{Brush, BrushKind, defaults, find, library, store};
        use vectorcraft_doc::Document;
        let fixture = serde_json::json!({"name":"fixture", "type":"natural", "media":"dryInk", "width":2.5, "seed":123456, "fibers":7});
        let brush: Brush = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(
            brush.kind,
            BrushKind::Natural(NaturalBrush { media: NaturalMedia::DryInk, width: 2.5, seed: 123456, fibers: 7, ..Default::default() })
        );
        assert_eq!(serde_json::to_value(&brush).unwrap(), fixture);
        assert_eq!(brush.kind.label(), "Natural");
        assert_eq!(brush.kind.type_id(), "natural");
        let partial: Brush = serde_json::from_value(serde_json::json!({"name":"partial", "type":"natural"})).unwrap();
        assert_eq!(partial.kind, BrushKind::Natural(NaturalBrush::default()));
        let mut doc = Document::new(100.0, 100.0);
        let originals: Vec<_> = defaults().iter().filter(|b| matches!(b.kind, BrushKind::Natural(_))).cloned().collect();
        assert_eq!(originals.len(), 5);
        assert_eq!(originals[0].name, "Rainee Pencil");
        assert_eq!(originals[1].name, "Rainee Dry Ink");
        let mut lib = library(&doc);
        lib.push(brush.clone());
        store(&mut doc, &lib);
        let saved = serde_json::to_string(&doc).unwrap();
        let reopened: Document = serde_json::from_str(&saved).unwrap();
        assert_eq!(library(&reopened), library(&doc));
        assert_eq!(find(&reopened, "fixture"), Some(brush));
        for b in originals {
            assert_eq!(find(&reopened, &b.name), Some(b.clone()));
        }
    }

    #[test]
    fn routed_stroke_respects_weight_pressure_asymmetry_and_tilt() {
        use crate::{Brush, BrushKind, stroke_pieces};
        use vectorcraft_doc::{NibProfile, WidthProfile};
        let brush = Brush { name: "test".into(), kind: BrushKind::Natural(NaturalBrush { width: 4.0, ..Default::default() }) };
        let mut stroke = StrokeLayer::new(Paint::solid(Color::BLACK), 1.0);
        let plain = stroke_pieces(&brush, &line(), &stroke);
        stroke.width = 2.0;
        let weighted = stroke_pieces(&brush, &line(), &stroke);
        assert!(bounds(&weighted).height() > bounds(&plain).height() * 1.7);
        stroke.profile = Some(WidthProfile { points: vec![(0.0, 0.0, 0.0), (0.5, 2.0, 0.3), (1.0, 0.0, 0.0)] });
        let pressure = stroke_pieces(&brush, &line(), &stroke);
        assert_ne!(signature(&weighted), signature(&pressure));
        let bb = bounds(&pressure);
        assert!(bb.y1 > -bb.y0 * 3.0);
        let middle = pressure
            .iter()
            .flat_map(|n| n.path_data().unwrap().anchors().map(|(_, _, a)| a.p))
            .filter(|p| (p.x - 50.0).abs() < 5.0)
            .map(|p| p.y)
            .fold(0.0, f64::max);
        let ends = pressure
            .iter()
            .flat_map(|n| n.path_data().unwrap().anchors().map(|(_, _, a)| a.p))
            .filter(|p| p.x < 5.0 || p.x > 95.0)
            .map(|p| p.y.abs())
            .fold(0.0, f64::max);
        assert!(middle > ends * 3.0);
        stroke.nib = Some(NibProfile { points: vec![(0.0, 0.0, 0.2), (1.0, 0.0, 0.2)] });
        let tilted = stroke_pieces(&brush, &line(), &stroke);
        assert!(bounds(&tilted).height() < bounds(&pressure).height() * 0.3);
        stroke.width = 0.0;
        assert!(stroke_pieces(&brush, &line(), &stroke).is_empty());
    }

    #[test]
    fn centerline_width_and_eraser_edits_recompute_node_appearance() {
        use crate::{find, node_pieces};
        use vectorcraft_doc::{Appearance, Document, NodeId, WidthProfile};
        let doc = Document::new(200.0, 200.0);
        assert!(find(&doc, "Rainee Pencil").is_some());
        let mut appearance = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 1.0);
        appearance.stroke_mut().unwrap().brush = Some("Rainee Pencil".into());
        let mut node = Node::path(NodeId(5), PathData::from_bezpath(&line()), appearance);
        let before = node.clone();
        let initial = signature(&node_pieces(&doc, &node).unwrap());
        assert_eq!(node, before, "generation must leave the editable source intact");
        node.appearance.stroke_mut().unwrap().profile = Some(WidthProfile::lens());
        let widened = signature(&node_pieces(&doc, &node).unwrap());
        assert_ne!(initial, widened);
        let mut erased = BezPath::new();
        erased.move_to((0.0, 0.0));
        erased.line_to((35.0, 0.0));
        erased.move_to((65.0, 0.0));
        erased.line_to((100.0, 0.0));
        if let vectorcraft_doc::NodeKind::Path { path, .. } = &mut node.kind {
            *path = PathData::from_bezpath(&erased);
        }
        let after = node_pieces(&doc, &node).unwrap();
        assert_ne!(widened, signature(&after));
        for n in after {
            for (_, _, anchor) in n.path_data().unwrap().anchors() {
                assert!(anchor.p.x <= 35.0 || anchor.p.x >= 65.0, "no geometry in erased centerline gap");
            }
        }
    }

    #[test]
    fn profiled_output_budget_and_invalid_profiles() {
        use vectorcraft_doc::{NibProfile, WidthProfile};
        let b = NaturalBrush { fibers: MAX_FIBERS, ..Default::default() };
        let mut s = StrokeLayer::new(Paint::solid(Color::BLACK), 1.0);
        s.profile = Some(WidthProfile {
            points: (0..MAX_PROFILE_POINTS).map(|i| (i as f64 / (MAX_PROFILE_POINTS - 1) as f64, (i % 3) as f64, 2.0)).collect(),
        });
        s.nib = Some(NibProfile { points: (0..MAX_PROFILE_POINTS).map(|i| (i as f64 / (MAX_PROFILE_POINTS - 1) as f64, i as f64, 0.5)).collect() });
        let mut p = BezPath::new();
        for i in 0..MAX_INPUT_ELEMENTS / 2 {
            p.move_to((0.0, i as f64 * 20.0));
            p.line_to((100.0, i as f64 * 20.0));
        }
        let out = stroke_geometry(&p, &b, &s).unwrap();
        let patches: usize =
            out.iter().map(|n| n.path_data().unwrap().to_bezpath().elements().iter().filter(|e| matches!(e, PathEl::ClosePath)).count()).sum();
        assert!(patches <= MAX_PATCHES);
        assert!(bounds(&out).y1 > 2500.0, "later subpaths must not be lost");
        for points in [
            vec![(0.0, f64::NAN, 1.0)],
            vec![(0.7, 1.0, 1.0), (0.3, 1.0, 1.0)],
            vec![(0.0, -1.0, 1.0)],
            vec![(0.0, 1.0, 1.0); MAX_PROFILE_POINTS + 1],
        ] {
            s.profile = Some(WidthProfile { points });
            assert!(matches!(stroke_geometry(&line(), &b, &s), Err(NaturalError::InvalidProfile)));
        }
    }

    #[test]
    fn dense_actual_sample_profiles_remain_natural_without_diagnostic() {
        use crate::{Brush, BrushKind, stroke_pieces_with_diagnostic};
        use vectorcraft_doc::{NibProfile, WidthProfile};
        let settings = NaturalBrush { width: 4.0, ..Default::default() };
        let brush = Brush { name: "dense native samples".into(), kind: BrushKind::Natural(settings) };
        let mut stroke = StrokeLayer::new(Paint::solid(Color::BLACK), 2.0);
        let positions = || (0..1000).map(|i| i as f64 / 999.0);
        stroke.profile = Some(WidthProfile {
            points: positions()
                .map(|t| {
                    let pressure = (std::f64::consts::PI * t).sin();
                    (t, 0.25 + 0.75 * pressure, 0.25 + 0.5 * pressure)
                })
                .collect(),
        });
        stroke.nib = Some(NibProfile { points: positions().map(|t| (t, 80.0 * t, 0.35 + 0.6 * t)).collect() });
        let (nodes, diagnostic) = stroke_pieces_with_diagnostic(&brush, &line(), &stroke);
        assert_eq!(diagnostic, None, "actual-sample profiles must not trigger ordinary fallback");
        assert_eq!(nodes.len(), settings.fibers);
        assert_eq!(signature(&nodes), signature(&stroke_geometry(&line(), &settings, &stroke).unwrap()));
        assert!(nodes.iter().all(|n| n.opacity < 1.0), "retain natural deposition instead of solid calligraphic fallback");
        let patches: usize = nodes.iter().map(|n| n.path_data().unwrap().subpaths.len()).sum();
        assert!(patches <= MAX_PATCHES);
        let bb = bounds(&nodes);
        assert!(bb.x0 >= 0.0 && bb.x1 <= 100.0 && bb.y0 >= -4.0 && bb.y1 <= 4.0);
        stroke.profile = None;
        stroke.nib = None;
        assert_ne!(signature(&nodes), signature(&stroke_geometry(&line(), &settings, &stroke).unwrap()));
    }

    #[test]
    fn oversized_strokes_keep_bounded_calligraphic_art_and_diagnostic() {
        use crate::{Brush, BrushKind, stroke_pieces, stroke_pieces_with_diagnostic};
        use vectorcraft_doc::WidthProfile;
        let brush = Brush { name: "fallback test".into(), kind: BrushKind::Natural(NaturalBrush::default()) };
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        for i in 0..400 {
            let x = i as f64;
            p.curve_to((x + 0.2, 2.0), (x + 0.8, -2.0), (x + 1.0, 0.0));
        }
        let before = p.clone();
        let s = StrokeLayer::new(Paint::solid(Color::BLACK), 1.0);
        let (out, diagnostic) = stroke_pieces_with_diagnostic(&brush, &p, &s);
        assert_eq!(diagnostic, Some(NaturalError::TooManyElements));
        assert!(!out.is_empty());
        assert_eq!(p, before);
        assert_eq!(signature(&out), signature(&stroke_pieces(&brush, &p, &s)));
        assert_eq!(signature(&out), signature(&stroke_pieces_with_diagnostic(&brush, &p, &s).0));
        let anchors: usize = out.iter().map(|n| n.path_data().unwrap().anchors().count()).sum();
        assert!(anchors <= MAX_FALLBACK_ANCHORS);
        let bb = bounds(&out);
        assert!(bb.x0 <= 0.0 && bb.x1 >= 400.0);
        assert!(bb.y0 >= -3.5 && bb.y1 <= 3.5);
        let wide = StrokeLayer::new(s.paint.clone(), 4.0);
        let wide_out = stroke_pieces_with_diagnostic(&brush, &p, &wide).0;
        assert!(bounds(&wide_out).height() > bb.height());
        let mut profile = s.clone();
        profile.profile = Some(WidthProfile { points: vec![(0.0, 1.0, 1.0); MAX_PROFILE_POINTS + 1] });
        let (out, diagnostic) = stroke_pieces_with_diagnostic(&brush, &line(), &profile);
        assert_eq!(diagnostic, Some(NaturalError::InvalidProfile));
        assert!(!out.is_empty());
        let oversized_weight = StrokeLayer::new(s.paint.clone(), 5000.0);
        let (out, diagnostic) = stroke_pieces_with_diagnostic(&brush, &line(), &oversized_weight);
        assert_eq!(diagnostic, Some(NaturalError::InvalidWidth));
        assert!(!out.is_empty());
        assert!(bounds(&out).height() <= MAX_WIDTH + 1e-6);
        for width in [f64::INFINITY, f64::NAN, -1.0] {
            let (out, diagnostic) = stroke_pieces_with_diagnostic(&brush, &line(), &StrokeLayer::new(s.paint.clone(), width));
            assert!(out.is_empty());
            assert_eq!(diagnostic, Some(NaturalError::InvalidWidth));
        }
        let mut invalid = p.clone();
        invalid.line_to((f64::INFINITY, 0.0));
        assert!(stroke_pieces_with_diagnostic(&brush, &invalid, &s).0.is_empty());
    }

    #[test]
    fn huge_source_fallback_bounds_allocations_and_separate_subpaths() {
        use crate::{Brush, BrushKind, stroke_pieces_with_diagnostic};
        let brush = Brush { name: "stress".into(), kind: BrushKind::Natural(NaturalBrush::default()) };
        let stroke = StrokeLayer::new(Paint::solid(Color::BLACK), 2.0);
        let mut huge = BezPath::new();
        for i in 0..8192 {
            let x = i as f64 * 100.0;
            huge.move_to((x, 0.0));
            huge.curve_to((x + 2.0, 4.0), (x + 8.0, -4.0), (x + 10.0, 0.0));
        }
        let (out, diagnostic) = stroke_pieces_with_diagnostic(&brush, &huge, &stroke);
        assert_eq!(diagnostic, Some(NaturalError::TooManyElements));
        assert!(!out.is_empty());
        let anchors: usize = out.iter().map(|n| n.path_data().unwrap().anchors().count()).sum();
        assert!(anchors <= MAX_FALLBACK_ANCHORS);
        let bb = bounds(&out);
        assert!(bb.x0 <= 0.0 && bb.x1 >= 819110.0);
        assert!(bb.y0 >= -7.0 && bb.y1 <= 7.0);
        for n in out {
            let data = n.path_data().unwrap();
            assert!(data.subpaths.len() <= MAX_FALLBACK_SUBPATHS);
            for seg in data.to_bezpath().segments() {
                assert!(seg.start().x.is_finite() && seg.start().y.is_finite());
                assert!(seg.start().distance(seg.end()) < 20.0, "fallback cannot bridge separate subpaths");
            }
        }
    }

    #[test]
    fn fine_near_continuous_contacts_have_joined_core_and_no_chord_dropout() {
        let mut curve = BezPath::new();
        curve.move_to((0.0, 0.0));
        curve.curve_to((100.0, -80.0), (200.0, -80.0), (300.0, 0.0));
        for media in [NaturalMedia::Pencil, NaturalMedia::DryInk] {
            let b = NaturalBrush { media, width: 12.0, ..Default::default() };
            let samples = chords(&curve, b.width, None);
            assert_eq!(samples.len(), 64, "wide long curves must keep fine subdivision");
            let nodes = generate(&curve, b);
            assert_eq!(nodes.len(), b.fibers);
            let mut continuous_fibers = 0;
            for node in nodes {
                let data = node.path_data().unwrap();
                assert_eq!(data.subpaths.len(), samples.len(), "no fiber may drop a whole chord");
                let continuous = data.subpaths.windows(2).all(|pair| {
                    let a = &pair[0].anchors;
                    let b = &pair[1].anchors;
                    a[1].p.distance(b[0].p) < 1e-7 && a[2].p.distance(b[3].p) < 1e-7
                });
                continuous_fibers += usize::from(continuous);
            }
            assert!(continuous_fibers >= 4, "core contacts must join across every sample seam");
        }
        let b = NaturalBrush { width: 12.0, ..Default::default() };
        let samples = chords(&line(), b.width, None);
        for node in generate(&line(), b) {
            for (patch, chord) in node.path_data().unwrap().subpaths.iter().zip(&samples) {
                let x0 = patch.anchors.iter().map(|a| a.p.x).fold(f64::INFINITY, f64::min);
                let x1 = patch.anchors.iter().map(|a| a.p.x).fold(f64::NEG_INFINITY, f64::max);
                assert!((x1 - x0) / chord.a.distance(chord.b) >= 0.95 - 1e-9);
            }
        }
    }
}
