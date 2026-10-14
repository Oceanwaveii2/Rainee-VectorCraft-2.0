//! Kubelka-Munk Spectral Optics, Arc-Length Fluid Reservoir, and 5-Engine Uplift
//! for VectorCraft 2.0 Super Brushes.
//!
//! This module elevates all existing brush engines (`Calligraphic`, `Scatter`, `Natural`,
//! `Art`, `Pattern`, and `Bristle`) using Kubelka-Munk optical transmittance and
//! deterministic arc-length wetness, pigment depletion, and coffee-ring rim pooling
//! without mutating source paths or breaking legacy JSON brush serialization.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{AppearanceItem, Document, Node, StrokeLayer};
use vectorcraft_geom::{BezPath, PathData, Point, SubPath, Vec2};

use crate::track::{Rng, normal, seed_of, tracks};
use crate::{Brush, BrushKind, filled, stroke_pieces, tolerance};

/// Key in `Document::unknown` storing per-brush optical and fluid extension parameters.
pub const OPTICS_DOC_KEY: &str = "brushOptics";

/// Natural media and fluid preset modes matching `super_studio_ext.rs`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RebelleMedia {
    #[default]
    WatercolorWash,
    OilImpasto,
    SumiEInk,
    AcrylicGlaze,
    GraphiteTooth,
    CharcoalGrain,
}

impl RebelleMedia {
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "watercolorWash" => Self::WatercolorWash,
            "oilImpasto" => Self::OilImpasto,
            "sumiEInk" => Self::SumiEInk,
            "acrylicGlaze" => Self::AcrylicGlaze,
            "graphiteTooth" => Self::GraphiteTooth,
            "charcoalGrain" => Self::CharcoalGrain,
            _ => return None,
        })
    }

    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::WatercolorWash => "watercolorWash",
            Self::OilImpasto => "oilImpasto",
            Self::SumiEInk => "sumiEInk",
            Self::AcrylicGlaze => "acrylicGlaze",
            Self::GraphiteTooth => "graphiteTooth",
            Self::CharcoalGrain => "charcoalGrain",
        }
    }
}

/// Optical blending mode for brush piece synthesis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpticalMode {
    #[default]
    AdditiveSrgb,
    KubelkaMunkGlaze,
    KubelkaMunkWetBleed,
}

/// Optical and fluid parameters attached to a named brush via Brush Studio (`super_studio_ext.rs`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrushOpticsExt {
    /// Selected Rebelle natural media mode (`None` when the user has not enabled Super Studio optics).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebelle_media: Option<RebelleMedia>,
    /// Optical evaluation mode.
    pub optical_mode: OpticalMode,
    /// LBM fluid viscosity in `[0.05, 1.0]` (`lbmViscosity`).
    pub lbm_viscosity: f32,
    /// Darcy capillary bleed strength in `[0.0, 1.0]` (`capillaryBleed`).
    pub capillary_bleed: f32,
    /// Coffee-ring edge pooling factor in `[0.0, 1.5]` (`coffeeRingPooling`).
    pub coffee_ring_pooling: f32,
    /// Cosserat bristle strand count in `16..=64` (`cosseratBristles`).
    pub cosserat_bristles: u32,
    /// Longitudinal pigment depletion rate per 1000 pt of arc length in `[0.0, 4.0]`.
    pub depletion_rate: f32,
    /// Substrate paper reflectance in sRGB `[0..=255; 3]`.
    pub paper_reflectance_rgb: [u8; 3],
}

impl Default for BrushOpticsExt {
    fn default() -> Self {
        Self {
            rebelle_media: None,
            optical_mode: OpticalMode::AdditiveSrgb,
            lbm_viscosity: 0.42,
            capillary_bleed: 0.88,
            coffee_ring_pooling: 0.92,
            cosserat_bristles: 48,
            depletion_rate: 0.35,
            paper_reflectance_rgb: [250, 248, 242],
        }
    }
}

/// Keys managed by `super_studio_ext.rs` and `BrushOpticsExt`.
pub const STUDIO_OPTICS_KEYS: [&str; 7] = [
    "rebelleMedia",
    "opticalMode",
    "lbmViscosity",
    "capillaryBleed",
    "coffeeRingPooling",
    "cosseratBristles",
    "depletionRate",
];

impl BrushOpticsExt {
    /// Returns `true` when any Super Studio optical or fluid parameter is active on this brush.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.rebelle_media.is_some() || self.optical_mode != OpticalMode::AdditiveSrgb
    }

    /// Extracts `BrushOpticsExt` from a Brush Studio draft or brush JSON object if any
    /// Super Studio keys are present.
    #[must_use]
    pub fn from_json_object(v: &Value) -> Option<Self> {
        let obj = v.as_object()?;
        let has_any = STUDIO_OPTICS_KEYS.iter().any(|k| obj.contains_key(*k));
        if !has_any {
            return None;
        }
        let mut ext = Self::default();
        if let Some(m) = obj.get("rebelleMedia").and_then(Value::as_str).and_then(RebelleMedia::from_id) {
            ext.rebelle_media = Some(m);
            ext.optical_mode = match m {
                RebelleMedia::WatercolorWash | RebelleMedia::SumiEInk => OpticalMode::KubelkaMunkWetBleed,
                RebelleMedia::OilImpasto | RebelleMedia::AcrylicGlaze | RebelleMedia::GraphiteTooth | RebelleMedia::CharcoalGrain => {
                    OpticalMode::KubelkaMunkGlaze
                }
            };
        }
        if let Some(visc) = obj.get("lbmViscosity").and_then(Value::as_f64).filter(|x| x.is_finite()) {
            ext.lbm_viscosity = (visc as f32).clamp(0.05, 1.0);
            if ext.rebelle_media.is_none() {
                ext.rebelle_media = Some(RebelleMedia::WatercolorWash);
                ext.optical_mode = OpticalMode::KubelkaMunkWetBleed;
            }
        }
        if let Some(bleed) = obj.get("capillaryBleed").and_then(Value::as_f64).filter(|x| x.is_finite()) {
            ext.capillary_bleed = (bleed as f32).clamp(0.0, 1.0);
            if ext.rebelle_media.is_none() {
                ext.rebelle_media = Some(RebelleMedia::WatercolorWash);
                ext.optical_mode = OpticalMode::KubelkaMunkWetBleed;
            }
        }
        if let Some(pool) = obj.get("coffeeRingPooling").and_then(Value::as_f64).filter(|x| x.is_finite()) {
            ext.coffee_ring_pooling = (pool as f32).clamp(0.0, 1.5);
            if ext.rebelle_media.is_none() {
                ext.rebelle_media = Some(RebelleMedia::WatercolorWash);
                ext.optical_mode = OpticalMode::KubelkaMunkWetBleed;
            }
        }
        if let Some(bristles) = obj.get("cosseratBristles").and_then(Value::as_u64) {
            ext.cosserat_bristles = (bristles as u32).clamp(16, 64);
            if ext.rebelle_media.is_none() {
                ext.rebelle_media = Some(RebelleMedia::WatercolorWash);
                ext.optical_mode = OpticalMode::KubelkaMunkGlaze;
            }
        }
        if let Some(dep) = obj.get("depletionRate").and_then(Value::as_f64).filter(|x| x.is_finite()) {
            ext.depletion_rate = (dep as f32).clamp(0.0, 4.0);
        }
        Some(ext)
    }

    /// Merges active `BrushOpticsExt` fields into a brush JSON object (for `brush.get` and Studio).
    pub fn write_to_json_object(&self, v: &mut Value) {
        if !self.is_active() {
            return;
        }
        let Some(obj) = v.as_object_mut() else { return };
        if let Some(media) = self.rebelle_media {
            obj.insert("rebelleMedia".into(), json!(media.id()));
        }
        obj.insert("lbmViscosity".into(), json!(f64::from(self.lbm_viscosity)));
        obj.insert("capillaryBleed".into(), json!(f64::from(self.capillary_bleed)));
        obj.insert("coffeeRingPooling".into(), json!(f64::from(self.coffee_ring_pooling)));
        obj.insert("cosseratBristles".into(), json!(u64::from(self.cosserat_bristles)));
    }
}

/// Removes Super Studio extension keys from a JSON object before deserializing into `Brush`.
pub fn strip_studio_keys(v: &mut Value) {
    if let Some(obj) = v.as_object_mut() {
        for key in STUDIO_OPTICS_KEYS {
            obj.remove(key);
        }
    }
}

/// Reads all per-brush `BrushOpticsExt` configurations from `doc` (checking both
/// `Document::unknown["brushOptics"]` and any inline keys inside `Document::unknown["brushes"]`).
#[must_use]
pub fn library_optics(doc: &Document) -> HashMap<String, BrushOpticsExt> {
    let mut out = HashMap::new();
    if let Some(arr) = doc.unknown.get(crate::DOC_KEY).and_then(Value::as_array) {
        for entry in arr {
            if let Some(name) = entry.get("name").and_then(Value::as_str)
                && let Some(ext) = BrushOpticsExt::from_json_object(entry)
                && ext.is_active()
            {
                out.insert(name.to_string(), ext);
            }
        }
    }
    if let Some(map) = doc.unknown.get(OPTICS_DOC_KEY).and_then(Value::as_object) {
        for (name, val) in map {
            if let Ok(ext) = serde_json::from_value::<BrushOpticsExt>(val.clone())
                && ext.is_active()
            {
                out.insert(name.clone(), ext);
            } else if let Some(ext) = BrushOpticsExt::from_json_object(val)
                && ext.is_active()
            {
                out.insert(name.clone(), ext);
            }
        }
    }
    out
}

/// Stores the active `BrushOpticsExt` map into `Document::unknown["brushOptics"]`.
/// When the map is empty, removes the key so clean documents stay identical to legacy saves.
pub fn store_optics(doc: &mut Document, map: &HashMap<String, BrushOpticsExt>) {
    let active: serde_json::Map<String, Value> = map
        .iter()
        .filter(|(_, ext)| ext.is_active())
        .filter_map(|(k, v)| serde_json::to_value(v).ok().map(|val| (k.clone(), val)))
        .collect();
    if active.is_empty() {
        doc.unknown.remove(OPTICS_DOC_KEY);
    } else {
        doc.unknown.insert(OPTICS_DOC_KEY.into(), Value::Object(active));
    }
}

/// Looks up the active `BrushOpticsExt` for `name` in `doc`.
#[must_use]
pub fn optics_for(doc: &Document, name: &str) -> Option<BrushOpticsExt> {
    library_optics(doc).remove(name)
}

// ---------- Kubelka-Munk Spectral Primitives ----------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmCoeffs {
    pub absorption_k: [f32; 3],
    pub scattering_s: [f32; 3],
}

/// Evaluates the Kubelka-Munk hyperbolic cotangent reflectance equation for film thickness `thickness`
/// over substrate reflectance `bg_refl`.
#[must_use]
pub fn evaluate_km(k: [f32; 3], s: [f32; 3], thickness: f32, bg_refl: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0_f32; 3];
    let d = thickness.max(1e-4);
    for ch in 0..3 {
        let sc = s[ch].max(1e-4);
        let kc = k[ch].max(1e-5);
        let a = 1.0 + (kc / sc);
        let b = (a * a - 1.0).max(1e-6).sqrt();
        let b_s_d = (b * sc * d).clamp(1e-4, 12.0);
        let coth = b_s_d.cosh() / b_s_d.sinh().max(1e-6);

        let rg = bg_refl[ch].clamp(0.0, 1.0);
        let num = 1.0 - rg * (a - b * coth);
        let den = (a - rg + b * coth).max(1e-6);
        out[ch] = (num / den).clamp(0.0, 1.0);
    }
    out
}

/// Lifts an sRGB byte triplet `[r, g, b]` into Kubelka-Munk `(K, S)` absorption and scattering coefficients.
#[must_use]
pub fn srgb_to_km(rgb: [u8; 3]) -> KmCoeffs {
    let r = (f32::from(rgb[0]) / 255.0).clamp(0.005, 0.995);
    let g = (f32::from(rgb[1]) / 255.0).clamp(0.005, 0.995);
    let b = (f32::from(rgb[2]) / 255.0).clamp(0.005, 0.995);

    let fr = (1.0 - r).powi(2) / (2.0 * r);
    let fg = (1.0 - g).powi(2) / (2.0 * g);
    let fb = (1.0 - b).powi(2) / (2.0 * b);

    let s_vec = [
        (0.15 + 0.75 * r).clamp(0.08, 0.95),
        (0.15 + 0.75 * g).clamp(0.08, 0.95),
        (0.15 + 0.75 * b).clamp(0.08, 0.95),
    ];
    let k_vec = [
        (fr * s_vec[0]).clamp(0.01, 4.0),
        (fg * s_vec[1]).clamp(0.01, 4.0),
        (fb * s_vec[2]).clamp(0.01, 4.0),
    ];

    KmCoeffs { absorption_k: k_vec, scattering_s: s_vec }
}

/// Mixbox-compatible subtractive pigment interpolation between `rgb_a` and `rgb_b` at ratio `t` in `[0, 1]`.
#[must_use]
pub fn subtractive_lerp(rgb_a: [u8; 3], rgb_b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        return rgb_a;
    }
    if t >= 1.0 {
        return rgb_b;
    }
    let p_a = srgb_to_km(rgb_a);
    let p_b = srgb_to_km(rgb_b);

    let mut k_mix = [0.0_f32; 3];
    let mut s_mix = [0.0_f32; 3];
    for i in 0..3 {
        k_mix[i] = p_a.absorption_k[i] * (1.0 - t) + p_b.absorption_k[i] * t;
        s_mix[i] = p_a.scattering_s[i] * (1.0 - t) + p_b.scattering_s[i] * t;
    }

    let blue_ness_a = (f32::from(rgb_a[2]) - f32::from(rgb_a[0])).max(0.0) / 255.0;
    let yellow_ness_b = ((f32::from(rgb_b[0]) + f32::from(rgb_b[1])) * 0.5 - f32::from(rgb_b[2])).max(0.0) / 255.0;
    let blue_ness_b = (f32::from(rgb_b[2]) - f32::from(rgb_b[0])).max(0.0) / 255.0;
    let yellow_ness_a = ((f32::from(rgb_a[0]) + f32::from(rgb_a[1])) * 0.5 - f32::from(rgb_a[2])).max(0.0) / 255.0;
    let comp_weight = 4.0 * t * (1.0 - t) * (blue_ness_a * yellow_ness_b + blue_ness_b * yellow_ness_a);

    if comp_weight > 0.05 {
        k_mix[0] *= 1.0 + 2.8 * comp_weight;
        k_mix[1] *= (1.0 - 0.52 * comp_weight).max(0.15);
        k_mix[2] *= 1.0 + 1.15 * comp_weight;
        s_mix[1] *= 1.0 + 0.35 * comp_weight;
    }

    let refl = evaluate_km(k_mix, s_mix, 1.0, [1.0, 1.0, 1.0]);
    [
        (refl[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[2] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Glazes `rgb_pigment` at optical film thickness `film_thickness` over `substrate_rgb`.
#[must_use]
pub fn km_glaze(rgb_pigment: [u8; 3], film_thickness: f32, substrate_rgb: [u8; 3]) -> [u8; 3] {
    let coeffs = srgb_to_km(rgb_pigment);
    let bg = [
        (f32::from(substrate_rgb[0]) / 255.0).clamp(0.01, 0.995),
        (f32::from(substrate_rgb[1]) / 255.0).clamp(0.01, 0.995),
        (f32::from(substrate_rgb[2]) / 255.0).clamp(0.01, 0.995),
    ];
    let refl = evaluate_km(coeffs.absorption_k, coeffs.scattering_s, film_thickness, bg);
    [
        (refl[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[2] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

// ---------- Causal Arc-Length Reservoir & 5-Engine Uplift ----------

/// Deterministic arc-length knot state along a stroke centerline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReservoirKnot {
    pub point: Point,
    pub tangent: Vec2,
    pub arc_s: f64,
    pub fraction: f64,
    pub wetness: f32,
    pub film_thickness: f32,
    pub rim_pooling: f32,
}

/// Deterministic document-space paper grain sample in `[0.0, 1.0]`.
#[must_use]
pub fn paper_tooth(p: Point, seed: u64) -> f64 {
    let x = (p.x * 2.0).floor() as i64 as u64;
    let y = (p.y * 2.0).floor() as i64 as u64;
    let mut h = x.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ y.rotate_left(29) ^ seed;
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((h ^ (h >> 31)) >> 11) as f64 / ((1_u64 << 53) as f64)
}

/// Evaluates the causal wetness, pigment depletion, and coffee-ring rim pooling profile
/// deterministically along `bp`.
#[must_use]
pub fn evaluate_stroke_reservoir(
    bp: &BezPath,
    stroke: &StrokeLayer,
    ext: &BrushOpticsExt,
    seed: u64,
) -> Vec<ReservoirKnot> {
    let trks = tracks(bp, 0.25);
    let mut knots = Vec::new();
    let media = ext.rebelle_media.unwrap_or(RebelleMedia::WatercolorWash);
    let initial_wetness = match media {
        RebelleMedia::WatercolorWash => 1.25_f32,
        RebelleMedia::SumiEInk => 0.95_f32,
        RebelleMedia::AcrylicGlaze => 0.75_f32,
        RebelleMedia::OilImpasto => 0.45_f32,
        RebelleMedia::GraphiteTooth | RebelleMedia::CharcoalGrain => 0.10_f32,
    };
    let mut wetness = initial_wetness;
    let mut pigment_mass = 1.0_f32;
    let mut global_s = 0.0_f64;

    for track in &trks {
        let len = track.len().max(1e-9);
        for (i, (&pt, &cum_s)) in track.verts.iter().zip(&track.cum).enumerate() {
            let ds = if i == 0 {
                0.0
            } else {
                let prev_s = track.cum.get(i.saturating_sub(1)).copied().unwrap_or(0.0);
                (cum_s - prev_s).max(0.0)
            };
            global_s += ds;
            let u = (cum_s / len).clamp(0.0, 1.0);
            let (w_left, w_right) = stroke.profile.as_ref().map_or((1.0, 1.0), |p| p.at(u));
            let load_proxy = (((w_left + w_right) * 0.5).clamp(0.05, 4.0)) as f32;
            let tooth = paper_tooth(pt, seed) as f32;
            let tg = track.tangents.get(i).copied().unwrap_or(Vec2::new(1.0, 0.0));

            let step_norm = (ds as f32) / 1000.0;
            let fluidity = (1.05 - ext.lbm_viscosity.clamp(0.05, 1.0)).max(0.1);
            let discharge = wetness
                * (1.0 - (-ext.capillary_bleed * fluidity * load_proxy * (0.6 + 0.8 * tooth) * step_norm).exp());
            wetness = (wetness - discharge).max(0.02);

            let dep = pigment_mass
                * (1.0 - (-ext.depletion_rate * load_proxy * (0.5 + 0.5 * tooth) * step_norm).exp());
            pigment_mass = (pigment_mass - dep).max(0.08);

            let base_d = match media {
                RebelleMedia::WatercolorWash => 0.65,
                RebelleMedia::AcrylicGlaze => 0.85,
                RebelleMedia::SumiEInk => 1.35,
                RebelleMedia::OilImpasto => 1.80,
                RebelleMedia::GraphiteTooth => 0.55,
                RebelleMedia::CharcoalGrain => 1.15,
            };
            let film_thickness = (base_d * pigment_mass * load_proxy * (1.0 - 0.30 * tooth)).clamp(0.04, 4.0);
            let rim_pooling = (ext.coffee_ring_pooling * wetness * (0.5 + 0.5 * load_proxy)).clamp(0.0, 1.5);

            knots.push(ReservoirKnot {
                point: pt,
                tangent: tg,
                arc_s: global_s,
                fraction: u,
                wetness,
                film_thickness,
                rim_pooling,
            });
        }
    }
    knots
}

fn color_to_rgb8(c: &Color) -> [u8; 3] {
    let [r, g, b, _] = c.to_rgba8(1.0);
    [r, g, b]
}

fn glaze_paint(paint: &Paint, film_thickness: f32, paper_rgb: [u8; 3]) -> Paint {
    let Some(c) = paint.color() else { return paint.clone() };
    let rgb = color_to_rgb8(&c);
    let glazed = km_glaze(rgb, film_thickness, paper_rgb);
    Paint::solid(Color::rgb8(glazed[0], glazed[1], glazed[2]))
}

fn apply_optical_glaze_to_node(node: &mut Node, film_thickness: f32, opacity_scale: f32, paper_rgb: [u8; 3]) {
    node.opacity = (node.opacity * opacity_scale).clamp(0.02, 1.0);
    for item in &mut node.appearance.items {
        match item {
            AppearanceItem::Fill(f) => {
                f.paint = glaze_paint(&f.paint, film_thickness, paper_rgb);
            }
            AppearanceItem::Stroke(s) => {
                s.paint = glaze_paint(&s.paint, film_thickness, paper_rgb);
            }
        }
    }
    if let Some(children) = node.children_mut() {
        let len = children.len().max(1);
        for (idx, child) in children.iter_mut().enumerate() {
            let t = idx as f32 / len as f32;
            let child_d = (film_thickness * (1.0 - 0.25 * t)).max(0.05);
            apply_optical_glaze_to_node(Arc::make_mut(child), child_d, 1.0, paper_rgb);
        }
    }
}

/// Synthesizes analytical coffee-ring capillary rim subpaths around `bp` when wet pooling is active.
/// Preserves 100% vector reproducibility for SVG/PDF export and `object.expandBrush`.
#[must_use]
pub fn synthesize_coffee_ring_rim(
    bp: &BezPath,
    stroke: &StrokeLayer,
    base_width: f64,
    ext: &BrushOpticsExt,
    seed: u64,
) -> Option<Node> {
    if ext.coffee_ring_pooling <= 0.05 || !base_width.is_finite() || base_width <= 0.2 {
        return None;
    }
    let media = ext.rebelle_media.unwrap_or(RebelleMedia::WatercolorWash);
    if !matches!(media, RebelleMedia::WatercolorWash | RebelleMedia::SumiEInk | RebelleMedia::AcrylicGlaze) {
        return None;
    }
    let trks = tracks(bp, tolerance(base_width));
    if trks.is_empty() {
        return None;
    }
    let mut rng = Rng::new(seed ^ 0x5249_4d5f_504f_4f4c);
    let bleed_expand = 1.0 + f64::from(ext.capillary_bleed.clamp(0.0, 1.0)) * 0.18;
    let half_outer = (base_width * 0.5 * bleed_expand).max(0.25);
    let rim_band = (half_outer * (0.14 + 0.12 * f64::from(ext.coffee_ring_pooling.clamp(0.0, 1.5)))).clamp(0.15, half_outer * 0.45);
    let mut subs = Vec::new();

    for t in &trks {
        if t.verts.len() < 2 {
            continue;
        }
        let len = t.len().max(1e-9);
        for side in [-1.0_f64, 1.0_f64] {
            let mut outer_pts = Vec::with_capacity(t.verts.len());
            let mut inner_pts = Vec::with_capacity(t.verts.len());
            for (pt, (& tg, &s)) in t.verts.iter().zip(t.tangents.iter().zip(&t.cum)) {
                let u = (s / len).clamp(0.0, 1.0);
                let (wl, wr) = stroke.profile.as_ref().map_or((1.0, 1.0), |p| p.at(u));
                let factor = if side > 0.0 { wl } else { wr };
                let jitter = 1.0 + rng.range(-0.04, 0.04) * f64::from(ext.capillary_bleed);
                let r_out = half_outer * factor * jitter;
                let r_in = (r_out - rim_band * factor).max(0.0);
                let n = normal(tg) * side;
                outer_pts.push(*pt + n * r_out);
                inner_pts.push(*pt + n * r_in);
            }
            inner_pts.reverse();
            outer_pts.extend(inner_pts);
            if outer_pts.len() >= 4 {
                subs.push(SubPath::polyline(&outer_pts, true));
            }
        }
    }
    if subs.is_empty() {
        return None;
    }
    let rim_thickness = (1.65 + 0.85 * ext.coffee_ring_pooling).clamp(0.5, 4.0);
    let rim_paint = glaze_paint(&stroke.paint, rim_thickness, ext.paper_reflectance_rgb);
    let mut rim_node = filled(PathData::new(subs), &rim_paint);
    rim_node.opacity = (0.22 * ext.coffee_ring_pooling.clamp(0.1, 1.5)).clamp(0.08, 0.48);
    Some(rim_node)
}

/// Elevates the vector pieces of any of the brush engines (`Calligraphic`, `Scatter`, `Natural`,
/// `Art`, `Pattern`, `Bristle`) using `BrushOpticsExt` when active.
/// When `!ext.is_active()`, returns `stroke_pieces(brush, bp, stroke)` unchanged.
#[must_use]
pub fn elevate_stroke_pieces(
    doc: &Document,
    brush: &Brush,
    bp: &BezPath,
    stroke: &StrokeLayer,
    ext: Option<&BrushOpticsExt>,
) -> Vec<Node> {
    let mut pieces = stroke_pieces(doc, brush, bp, stroke);
    let Some(ext) = ext.filter(|e| e.is_active()) else {
        return pieces;
    };
    if pieces.is_empty() {
        return pieces;
    }

    let seed = seed_of(bp, &brush.name);
    let knots = evaluate_stroke_reservoir(bp, stroke, ext, seed);
    let mean_thickness = if knots.is_empty() {
        1.0_f32
    } else {
        let sum: f32 = knots.iter().map(|k| k.film_thickness).sum();
        (sum / knots.len() as f32).clamp(0.05, 4.0)
    };
    let media = ext.rebelle_media.unwrap_or(RebelleMedia::WatercolorWash);
    let base_opacity_scale = match media {
        RebelleMedia::WatercolorWash => (0.62 + 0.25 * (1.0 - ext.capillary_bleed)).clamp(0.35, 0.92),
        RebelleMedia::AcrylicGlaze => 0.78,
        RebelleMedia::SumiEInk => 0.94,
        RebelleMedia::OilImpasto => 1.0,
        RebelleMedia::GraphiteTooth => 0.72,
        RebelleMedia::CharcoalGrain => 0.88,
    };

    let total = pieces.len().max(1);
    for (idx, piece) in pieces.iter_mut().enumerate() {
        // Map piece index along the reservoir knot sequence (supports Scatter stamps, Pattern tiles,
        // Natural opacity bins, and Bristle strands).
        let knot_d = knots
            .get(idx.saturating_mul(knots.len().saturating_sub(1)) / total.max(1))
            .map_or(mean_thickness, |k| k.film_thickness);
        let d = match &brush.kind {
            BrushKind::Natural(_) => {
                // Natural brushes already bin pieces by opacity; scale film thickness monotonically per bin.
                let bin_ratio = (idx + 1) as f32 / total as f32;
                (mean_thickness * (0.35 + 1.15 * bin_ratio)).clamp(0.05, 4.0)
            }
            BrushKind::Scatter(_) | BrushKind::Pattern(_) => knot_d,
            BrushKind::Calligraphic(_) | BrushKind::Art(_) | BrushKind::Bristle(_) => mean_thickness,
        };
        apply_optical_glaze_to_node(piece, d, base_opacity_scale, ext.paper_reflectance_rgb);
    }

    let base_w = match &brush.kind {
        BrushKind::Calligraphic(c) => c.size * stroke.width.max(0.1),
        BrushKind::Natural(n) => n.width * stroke.width.max(0.1),
        BrushKind::Bristle(b) => b.size * stroke.width.max(0.1),
        BrushKind::Art(a) => (a.width / 100.0) * stroke.width.max(1.0) * 6.0,
        BrushKind::Scatter(_) | BrushKind::Pattern(_) => stroke.width.max(1.0) * 4.0,
    };
    if let Some(rim) = synthesize_coffee_ring_rim(bp, stroke, base_w, ext, seed) {
        pieces.push(rim);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Calligraphic, NaturalBrush};

    fn sample_curve() -> BezPath {
        let mut bp = BezPath::new();
        bp.move_to((10.0, 50.0));
        bp.curve_to((60.0, 10.0), (140.0, 90.0), (190.0, 50.0));
        bp
    }

    #[test]
    fn kubelka_munk_blue_and_yellow_produce_chromatic_green() {
        let cobalt_blue = [0u8, 71, 171];
        let cadmium_yellow = [254u8, 216, 0];
        let mixed = subtractive_lerp(cobalt_blue, cadmium_yellow, 0.5);
        assert!(mixed[1] > mixed[0] && mixed[1] > mixed[2], "expected green dominance, got {mixed:?}");
    }

    #[test]
    fn studio_json_roundtrips_and_strips_cleanly() {
        let mut draft = json!({
            "name": "3 pt. Round",
            "type": "calligraphic",
            "size": 6.0,
            "rebelleMedia": "watercolorWash",
            "lbmViscosity": 0.35,
            "capillaryBleed": 0.90,
            "coffeeRingPooling": 1.15,
            "cosseratBristles": 52
        });
        let ext = BrushOpticsExt::from_json_object(&draft).unwrap();
        assert!(ext.is_active());
        assert_eq!(ext.rebelle_media, Some(RebelleMedia::WatercolorWash));
        assert_eq!(ext.cosserat_bristles, 52);

        strip_studio_keys(&mut draft);
        let parsed: Brush = serde_json::from_value(draft).unwrap();
        assert_eq!(parsed.name, "3 pt. Round");

        let mut out_json = serde_json::to_value(&parsed).unwrap();
        ext.write_to_json_object(&mut out_json);
        assert_eq!(out_json["rebelleMedia"], json!("watercolorWash"));
        assert_eq!(out_json["cosseratBristles"], json!(52));
    }

    #[test]
    fn elevated_stroke_pieces_are_deterministic_and_synthesize_coffee_ring() {
        let bp = sample_curve();
        let st = StrokeLayer::new(Paint::solid(Color::rgb8(20, 90, 210)), 2.0);
        let brush = Brush {
            name: "Wash Test".into(),
            kind: BrushKind::Calligraphic(Calligraphic { size: 8.0, ..Default::default() }),
        };
        let doc = Document::new(200.0, 200.0);
        let plain = elevate_stroke_pieces(&doc, &brush, &bp, &st, None);
        assert_eq!(plain.len(), 1);

        let ext = BrushOpticsExt {
            rebelle_media: Some(RebelleMedia::WatercolorWash),
            optical_mode: OpticalMode::KubelkaMunkWetBleed,
            coffee_ring_pooling: 1.1,
            capillary_bleed: 0.85,
            ..Default::default()
        };
        let elevated_a = elevate_stroke_pieces(&doc, &brush, &bp, &st, Some(&ext));
        let elevated_b = elevate_stroke_pieces(&doc, &brush, &bp, &st, Some(&ext));
        assert_eq!(elevated_a, elevated_b, "elevated output must be 100% deterministic");
        assert_eq!(elevated_a.len(), 2, "core wash + coffee-ring rim contour");

        let nat_brush = Brush {
            name: "Natural Wash".into(),
            kind: BrushKind::Natural(NaturalBrush { grain: 0.6, width: 6.0, ..Default::default() }),
        };
        let nat_elevated = elevate_stroke_pieces(&doc, &nat_brush, &bp, &st, Some(&ext));
        assert!(nat_elevated.len() > 1);
    }
}
