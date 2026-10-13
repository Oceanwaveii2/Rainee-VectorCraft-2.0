//! Lightweight resident-texture navigation. The host's existing GPU compositor draws the
//! transformed quad; Vello CPU remains authoritative and refreshes asynchronously.
use crate::CacheKey;

#[derive(Default)]
pub struct Navigation {
    requested: Option<CacheKey>,
    moved_at: f64,
    submitted_at: f64,
    pub deferred_frames: u64,
    pub raster_submissions: u64,
    pub uploads: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> CacheKey {
        CacheKey { doc: 1, zoom: 1.0, w: 800, h: 600, ppp: 1.0, ..Default::default() }
    }
    #[test]
    fn movement_reuses_texture_then_refreshes_after_pause() {
        let old = key();
        let mut target = old.clone();
        let mut nav = Navigation::default();
        assert!(!nav.defer(Some(&old), &old, 10.0, true));
        target.cx = 40.0;
        assert!(nav.defer(Some(&old), &target, 20.0, true));
        assert!(nav.defer(Some(&old), &target, 60.0, true));
        assert!(!nav.defer(Some(&old), &target, 101.0, true));
        assert_eq!(nav.deferred_frames, 2);
    }
    #[test]
    fn continuous_navigation_cannot_starve_newly_exposed_art() {
        let old = key();
        let mut target = old.clone();
        let mut nav = Navigation::default();
        nav.defer(Some(&old), &old, 0.0, true);
        for tick in 1..10 {
            target.cx = tick as f64;
            let delayed = nav.defer(Some(&old), &target, tick as f64 * 20.0, true);
            if tick == 8 { assert!(!delayed) } else { assert!(delayed) }
        }
    }
    #[test]
    fn edits_settings_and_cpu_fallback_do_not_reuse_content() {
        let old = key();
        let mut variants = vec![];
        let mut target = old.clone();
        target.revision += 1;
        variants.push(target);
        let mut target = old.clone();
        target.doc += 1;
        variants.push(target);
        let mut target = old.clone();
        target.ppp = 2.0;
        variants.push(target);
        let mut target = old.clone();
        target.w += 1;
        variants.push(target);
        let mut target = old.clone();
        target.outline = true;
        variants.push(target);
        let mut target = old.clone();
        target.overprint = true;
        variants.push(target);
        let mut target = old.clone();
        target.mask_view = Some(vectorcraft_doc::NodeId(7));
        variants.push(target);
        let mut target = old.clone();
        target.proof = Some(Default::default());
        variants.push(target);
        let mut target = old.clone();
        target.pixel = Some([0, 0, 800, 600]);
        variants.push(target);
        for target in variants {
            assert!(!Navigation::same_content(&old, &target));
            assert!(!Navigation::default().defer(Some(&old), &target, 10.0, true));
        }
        let mut target = old.clone();
        target.cx = 1.0;
        assert!(!Navigation::default().defer(Some(&old), &target, 10.0, false));
    }
    #[test]
    fn view_setting_does_not_edit_document_and_roundtrips() {
        let mut app = crate::VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", serde_json::json!({})).unwrap();
        let doc = app.session.doc().unwrap().doc.clone();
        assert!(app.ui.gpu_navigation);
        app.run("view.gpuNavigation", serde_json::json!({"enabled":false})).unwrap();
        assert!(!app.ui.gpu_navigation);
        assert!(app.run("view.gpuNavigation", serde_json::json!({"enabled":"yes"})).is_err());
        let state: crate::state::UiState = serde_json::from_value(serde_json::to_value(&app.ui).unwrap()).unwrap();
        assert!(!state.gpu_navigation);
        assert_eq!(doc, app.session.doc().unwrap().doc);
    }
}

impl Navigation {
    /// No reuse across edits, document changes, raster settings, pixel preview or DPI/size changes.
    pub fn same_content(a: &CacheKey, b: &CacheKey) -> bool {
        a.doc == b.doc
            && a.revision == b.revision
            && a.w == b.w
            && a.h == b.h
            && a.ppp == b.ppp
            && a.outline == b.outline
            && a.trim == b.trim
            && a.hidden == b.hidden
            && a.anti_alias == b.anti_alias
            && a.placed == b.placed
            && a.pixel == b.pixel
            && a.smooth_images == b.smooth_images
            && a.proof == b.proof
            && a.overprint == b.overprint
            && a.mask_view == b.mask_view
    }

    pub fn defer(&mut self, cached: Option<&CacheKey>, target: &CacheKey, now: f64, enabled: bool) -> bool {
        let eligible = enabled && cached.is_some_and(|old| old != target && old.pixel.is_none() && Self::same_content(old, target));
        if !eligible {
            self.requested = None;
            self.submitted_at = now;
            return false;
        }
        if self.requested.as_ref() != Some(target) {
            self.requested = Some(target.clone());
            self.moved_at = now;
        }
        // Crisp refresh after an 80 ms pause, and at least every 160 ms during continuous motion.
        // This avoids starving newly exposed artwork during a long pan.
        if now - self.moved_at < 80.0 && now - self.submitted_at < 160.0 {
            self.deferred_frames += 1;
            true
        } else {
            self.submitted_at = now;
            false
        }
    }
}
