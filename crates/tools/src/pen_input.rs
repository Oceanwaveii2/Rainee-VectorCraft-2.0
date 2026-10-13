//! Native pen capture contract, independent of the UI, filters and document geometry.
//! Windows Ink units: https://learn.microsoft.com/windows/win32/api/winuser/ns-winuser-pointer_pen_info
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

pub const HISTORY_LIMIT: usize = 2048;
const RECENT_LIMIT: usize = 8192;

/// Original packet values are retained even when a channel is unavailable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RawPenPacket {
    pub pointer_id: u32,
    pub frame_id: u32,
    pub device: u64,
    pub screen_px: [i32; 2],
    pub client_px: Option<[i32; 2]>,
    pub window_dpi: u32,
    pub performance_count: u64,
    pub performance_frequency: u64,
    pub system_tick_ms: u32,
    pub pointer_flags: u32,
    pub pen_flags: u32,
    pub channel_mask: u32,
    pub pressure: u32,
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub rotation: u32,
    pub button_change: i32,
    pub key_states: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Down,
    Drag,
    Up,
    Hover,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PenSample {
    pub phase: Phase,
    /// QPC seconds only. None when unavailable; raw system_tick_ms remains in the packet.
    pub time_seconds: Option<f64>,
    pub pressure: Option<f32>,
    pub tilt_degrees: [Option<i32>; 2],
    /// Only enabled after the actual pen's roll capability is confirmed, not inferred from tilt.
    pub barrel_rotation_degrees: Option<u32>,
    pub eraser: bool,
    pub inverted: bool,
    pub barrel_button: bool,
    /// Windows pointer button bits, not a promise of two individually identifiable side buttons.
    pub pointer_buttons: u32,
}

impl RawPenPacket {
    pub fn normalize(&self, confirmed_barrel_rotation: bool) -> Result<PenSample, String> {
        let pressure = if self.channel_mask & 1 != 0 {
            if self.pressure > 1024 {
                return Err("Windows Ink pressure is outside 0..1024".into());
            }
            Some(self.pressure as f32 / 1024.0)
        } else {
            None
        };
        let mut tilt = [None; 2];
        for (axis, value, mask) in [(0usize, self.tilt_x, 4), (1, self.tilt_y, 8)] {
            if self.channel_mask & mask != 0 {
                if !(-90..=90).contains(&value) {
                    return Err("Windows Ink tilt is outside -90..90 degrees".into());
                }
                if let Some(channel) = tilt.get_mut(axis) {
                    *channel = Some(value);
                }
            }
        }
        let rotation = if confirmed_barrel_rotation && self.channel_mask & 2 != 0 {
            if self.rotation > 359 {
                return Err("Windows Ink rotation is outside 0..359 degrees".into());
            }
            Some(self.rotation)
        } else {
            None
        };
        let flags = self.pointer_flags;
        let phase = if flags & 0x8000 != 0 {
            Phase::Cancel
        } else if flags & 0x40000 != 0 {
            Phase::Up
        } else if flags & 0x10000 != 0 {
            Phase::Down
        } else if flags & 4 != 0 {
            Phase::Drag
        } else {
            Phase::Hover
        };
        Ok(PenSample {
            phase,
            time_seconds: (self.performance_count > 0 && self.performance_frequency > 0)
                .then(|| self.performance_count as f64 / self.performance_frequency as f64),
            pressure,
            tilt_degrees: tilt,
            barrel_rotation_degrees: rotation,
            eraser: self.pen_flags & 4 != 0,
            inverted: self.pen_flags & 2 != 0,
            barrel_button: self.pen_flags & 1 != 0,
            pointer_buttons: flags & 0x1f0,
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CaptureStats {
    pub received: u64,
    pub accepted: u64,
    pub repeated_history: u64,
    pub invalid: u64,
    pub truncated_history: u64,
}

/// One capture session. Bounded overlap detection; distinct stationary pressure updates survive.
#[derive(Default)]
pub struct PenCapture {
    recent: VecDeque<RawPenPacket>,
    seen: HashSet<RawPenPacket>,
    pub stats: CaptureStats,
}

impl PenCapture {
    /// Native history arrives newest first. Return chronological, unique, validated packets.
    pub fn history(&mut self, newest_first: Vec<RawPenPacket>, available: u32) -> Vec<(RawPenPacket, PenSample)> {
        self.stats.truncated_history += u64::from(available).saturating_sub(newest_first.len() as u64);
        let mut accepted = Vec::new();
        // Retain the newest bounded subset even if an adapter supplied an oversized batch.
        let kept = newest_first.len().min(HISTORY_LIMIT);
        self.stats.truncated_history += (newest_first.len() - kept) as u64;
        for raw in newest_first.into_iter().take(kept).rev() {
            self.stats.received += 1;
            // Client mapping can change when a window moves between overlapping histories;
            // it is derived context, not part of the native packet's identity.
            let mut identity = raw.clone();
            identity.client_px = None;
            identity.window_dpi = 0;
            if self.seen.contains(&identity) {
                self.stats.repeated_history += 1;
                continue;
            }
            let Ok(sample) = raw.normalize(false) else {
                self.stats.invalid += 1;
                continue;
            };
            if self.recent.len() >= RECENT_LIMIT
                && let Some(old) = self.recent.pop_front()
            {
                self.seen.remove(&old);
            }
            self.seen.insert(identity.clone());
            self.recent.push_back(identity);
            self.stats.accepted += 1;
            accepted.push((raw, sample));
        }
        accepted
    }
}

/// Windows' documented extra-info signature: distinguish pen-promoted mouse from real mouse/touch.
/// https://learn.microsoft.com/windows/win32/tablet/system-events-and-mouse-messages
pub fn is_pen_promoted_mouse(extra_info: usize) -> bool {
    let signature = extra_info as u32;
    signature & 0xffffff00 == 0xff515700 && signature & 0x80 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(n: u64) -> RawPenPacket {
        RawPenPacket {
            pointer_id: 1,
            frame_id: n as u32,
            device: 7,
            screen_px: [-1920, 200],
            client_px: Some([10, 20]),
            window_dpi: 144,
            performance_count: n,
            performance_frequency: 1000,
            system_tick_ms: n as u32,
            pointer_flags: 4,
            pen_flags: 0,
            channel_mask: 13,
            pressure: 512,
            tilt_x: -30,
            tilt_y: 15,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        }
    }
    #[test]
    fn history_is_chronological_without_frame_coalescing() {
        let mut c = PenCapture::default();
        let samples = c.history(vec![packet(3), packet(2), packet(1)], 3);
        assert_eq!(samples.iter().map(|(r, _)| r.performance_count).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(samples[0].1.time_seconds, Some(0.001));
    }
    #[test]
    fn overlapping_history_is_not_a_second_stroke() {
        let mut c = PenCapture::default();
        c.history(vec![packet(2), packet(1)], 2);
        assert_eq!(c.history(vec![packet(3), packet(2)], 2).len(), 1);
        assert_eq!(c.stats.repeated_history, 1);
    }
    #[test]
    fn moved_window_does_not_turn_overlapping_history_into_new_input() {
        let a = packet(1);
        let mut b = a.clone();
        b.client_px = Some([300, 400]);
        b.window_dpi = 192;
        let mut c = PenCapture::default();
        c.history(vec![a], 1);
        assert!(c.history(vec![b], 1).is_empty());
    }
    #[test]
    fn stationary_pressure_and_buttons_are_not_discarded() {
        let a = packet(1);
        let mut b = a.clone();
        b.pressure = 700;
        let mut d = b.clone();
        d.pen_flags = 1;
        assert_eq!(PenCapture::default().history(vec![d, b, a], 3).len(), 3);
    }
    #[test]
    fn missing_channels_and_time_are_not_fabricated() {
        let mut p = packet(0);
        p.channel_mask = 0;
        p.pressure = 9999;
        let s = p.normalize(false).unwrap();
        assert_eq!(s.pressure, None);
        assert_eq!(s.tilt_degrees, [None, None]);
        assert_eq!(s.time_seconds, None);
    }
    #[test]
    fn pressure_uses_api_range_not_advertised_hardware_levels() {
        let mut p = packet(1);
        assert_eq!(p.normalize(false).unwrap().pressure, Some(0.5));
        p.pressure = 1024;
        assert_eq!(p.normalize(false).unwrap().pressure, Some(1.0));
        p.pressure = 1025;
        assert!(p.normalize(false).is_err());
    }
    #[test]
    fn tip_hover_eraser_barrel_and_cancel_remain_distinct() {
        let mut p = packet(1);
        p.pointer_flags = 0;
        assert_eq!(p.normalize(false).unwrap().phase, Phase::Hover);
        p.pointer_flags = 0x10000;
        assert_eq!(p.normalize(false).unwrap().phase, Phase::Down);
        p.pointer_flags = 0x40000;
        assert_eq!(p.normalize(false).unwrap().phase, Phase::Up);
        p.pointer_flags = 0x8000;
        p.pen_flags = 7;
        let s = p.normalize(false).unwrap();
        assert_eq!(s.phase, Phase::Cancel);
        assert!(s.eraser && s.inverted && s.barrel_button);
    }
    #[test]
    fn roll_requires_both_reported_channel_and_confirmed_pen() {
        let mut p = packet(1);
        p.channel_mask |= 2;
        p.rotation = 60;
        assert_eq!(p.normalize(false).unwrap().barrel_rotation_degrees, None);
        assert_eq!(p.normalize(true).unwrap().barrel_rotation_degrees, Some(60));
        p.tilt_x = 91;
        assert!(p.normalize(false).is_err());
    }
    #[test]
    fn truncation_is_reported_and_memory_is_bounded() {
        let mut c = PenCapture::default();
        c.history(vec![packet(1)], 10);
        assert_eq!(c.stats.truncated_history, 9);
        for n in 2..10000 {
            c.history(vec![packet(n)], 1);
        }
        assert!(c.recent.len() <= RECENT_LIMIT && c.seen.len() <= RECENT_LIMIT);
    }
    #[test]
    fn promotion_signature_does_not_suppress_a_real_mouse_or_touch() {
        assert!(is_pen_promoted_mouse(0xff515701));
        assert!(!is_pen_promoted_mouse(0xff515780));
        assert!(!is_pen_promoted_mouse(0));
    }
    #[test]
    fn raw_packet_roundtrip_preserves_coordinates_masks_and_clocks() {
        let p = packet(10);
        assert_eq!(serde_json::from_str::<RawPenPacket>(&serde_json::to_string(&p).unwrap()).unwrap(), p);
    }
}
