//! Wet Strata Layer State Machine & Optical Glaze Metadata (`Document::unknown["layerStrata"]`).
//!
//! Enables Procreate/Rebelle-inspired per-layer drying states (`VectorDry`, `WetFluid`, `DampSet`,
//! `DryGlaze`) and Kubelka-Munk optical film thickness without breaking legacy `.vectorcraft`
//! layer serialization.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Document, NodeId};

/// Key in `Document::unknown` storing per-layer `LayerStrataProps` keyed by `NodeId` integer string.
pub const STRATA_DOC_KEY: &str = "layerStrata";

/// Physical drying state of a layer in the Wet Strata pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StrataState {
    /// Standard vector/alpha compositing (default).
    #[default]
    VectorDry,
    /// Active LBM wet fluid layer: mixes subtractively (wet-on-wet) with underlying pigment.
    WetFluid,
    /// Damp semi-set pigment: capillary motion locked, optical glaze transmittance active.
    DampSet,
    /// Cured Kubelka-Munk optical glaze film over underlying layers.
    DryGlaze,
}

impl StrataState {
    pub const ALL: [Self; 4] = [Self::VectorDry, Self::WetFluid, Self::DampSet, Self::DryGlaze];

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "vectorDry" | "dry" => Self::VectorDry,
            "wetFluid" | "wet" => Self::WetFluid,
            "dampSet" | "damp" => Self::DampSet,
            "dryGlaze" | "glaze" => Self::DryGlaze,
            _ => return None,
        })
    }

    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::VectorDry => "vectorDry",
            Self::WetFluid => "wetFluid",
            Self::DampSet => "dampSet",
            Self::DryGlaze => "dryGlaze",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::VectorDry => "Vector Dry",
            Self::WetFluid => "Wet Fluid",
            Self::DampSet => "Damp Set",
            Self::DryGlaze => "Dry Glaze",
        }
    }

    #[must_use]
    pub fn badge(self) -> &'static str {
        match self {
            Self::VectorDry => "",
            Self::WetFluid => "W",
            Self::DampSet => "Dm",
            Self::DryGlaze => "Gz",
        }
    }

    /// Advances the drying state one step toward `DryGlaze`.
    #[must_use]
    pub fn cured_step(self) -> Self {
        match self {
            Self::WetFluid => Self::DampSet,
            Self::DampSet | Self::DryGlaze => Self::DryGlaze,
            Self::VectorDry => Self::VectorDry,
        }
    }
}

/// Per-layer optical glazing and fluid retention properties.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LayerStrataProps {
    pub state: StrataState,
    /// Optical film thickness $d \in [0.05, 4.0]$ for Kubelka-Munk transmittance.
    pub glaze_thickness: f32,
    /// Fraction of moisture retained per step in `[0.0, 1.0]`.
    pub wetness_retention: f32,
    /// Whether wet pigment bleeds into the layer immediately below.
    pub bleed_into_below: bool,
}

impl Default for LayerStrataProps {
    fn default() -> Self {
        Self {
            state: StrataState::VectorDry,
            glaze_thickness: 1.0,
            wetness_retention: 0.75,
            bleed_into_below: false,
        }
    }
}

impl LayerStrataProps {
    /// Returns `true` when this layer uses Kubelka-Munk optical glazing or wet pigment mixing.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state != StrataState::VectorDry
    }
}

/// Reads the `LayerStrataProps` for layer `id` from `doc` (returns `Default` if unset).
#[must_use]
pub fn layer_strata(doc: &Document, id: NodeId) -> LayerStrataProps {
    let key = id.0.to_string();
    doc.unknown
        .get(STRATA_DOC_KEY)
        .and_then(Value::as_object)
        .and_then(|m| m.get(&key))
        .and_then(|v| serde_json::from_value::<LayerStrataProps>(v.clone()).ok())
        .unwrap_or_default()
}

/// Writes `props` for layer `id` into `Document::unknown["layerStrata"]`.
/// Removes the entry when `props` returns to `VectorDry` so clean documents have zero overhead.
pub fn set_layer_strata(doc: &mut Document, id: NodeId, props: LayerStrataProps) {
    let key = id.0.to_string();
    let mut map = doc
        .unknown
        .get(STRATA_DOC_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(Map::new);
    if props.is_active() {
        if let Ok(val) = serde_json::to_value(props) {
            map.insert(key, val);
        }
    } else {
        map.remove(&key);
    }
    if map.is_empty() {
        doc.unknown.remove(STRATA_DOC_KEY);
    } else {
        doc.unknown.insert(STRATA_DOC_KEY.into(), Value::Object(map));
    }
}

/// Advances layer `id`'s drying state (`WetFluid -> DampSet -> DryGlaze`) and returns the new state.
pub fn cure_layer_strata(doc: &mut Document, id: NodeId) -> StrataState {
    let mut props = layer_strata(doc, id);
    props.state = props.state.cured_step();
    set_layer_strata(doc, id, props);
    props.state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_strata_roundtrips_and_cures_cleanly() {
        let mut d = Document::new(100.0, 100.0);
        let lid = d.layers[0].id;
        assert_eq!(layer_strata(&d, lid), LayerStrataProps::default());
        assert!(!d.unknown.contains_key(STRATA_DOC_KEY));

        let wet = LayerStrataProps {
            state: StrataState::WetFluid,
            glaze_thickness: 1.4,
            wetness_retention: 0.85,
            bleed_into_below: true,
        };
        set_layer_strata(&mut d, lid, wet);
        assert_eq!(layer_strata(&d, lid), wet);

        assert_eq!(cure_layer_strata(&mut d, lid), StrataState::DampSet);
        assert_eq!(cure_layer_strata(&mut d, lid), StrataState::DryGlaze);

        set_layer_strata(&mut d, lid, LayerStrataProps::default());
        assert!(!d.unknown.contains_key(STRATA_DOC_KEY));
    }
}
