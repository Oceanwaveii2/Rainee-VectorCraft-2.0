//! Read Windows Ink history before winit coalesces it. Never replaces a WndProc or injects input.
#![allow(unsafe_code)]
use std::sync::{Arc, Mutex};
use vectorcraft_tools::pen_input::{HISTORY_LIMIT, PenCapture, PenSample, RawPenPacket};
use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::ScreenToClient,
    System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::{
            KeyboardAndMouse::GetFocus,
            Pointer::{GetPointerPenInfo, GetPointerPenInfoHistory, GetPointerType, POINTER_PEN_INFO},
        },
        WindowsAndMessaging::{
            GetForegroundWindow, IsChild, MSG, POINTER_INPUT_TYPE, PT_PEN, WM_KILLFOCUS, WM_POINTERCAPTURECHANGED, WM_POINTERDOWN, WM_POINTERUP,
            WM_POINTERUPDATE, WM_SETFOCUS,
        },
    },
};

#[derive(Default)]
struct State {
    window: usize,
    capture: PenCapture,
    packets: Vec<(RawPenPacket, PenSample)>,
    interrupted: bool,
    frequency: u64,
    focused: bool,
    focus_messages: u64,
    pointer_messages: u64,
    unfocused_pointer_messages: u64,
    ctx: Option<egui::Context>,
    age_count: u64,
    age_sum_ms: f64,
    age_max_ms: f64,
    last_newest_ms: Option<f64>,
}

// GetFocus is thread-local. Requiring the same foreground window also prevents a stale
// thread-local focus handle from accepting input after the user switches applications.
fn owns_focus(window: usize, foreground: usize, focus: usize, child: bool) -> bool {
    window != 0 && foreground == window && focus != 0 && (focus == window || child)
}

fn window_focused(window: usize) -> bool {
    if window == 0 {
        return false;
    }
    let hwnd = HWND(window as *mut std::ffi::c_void);
    // SAFETY: read-only queries on the owning event thread. IsChild safely rejects invalid
    // handles; no thread attachment, focus change, WndProc replacement or input injection.
    let (foreground, focus, child) = unsafe {
        let foreground = GetForegroundWindow();
        let focus = GetFocus();
        (foreground.0 as usize, focus.0 as usize, IsChild(hwnd, focus).as_bool())
    };
    owns_focus(window, foreground, focus, child)
}

impl State {
    fn interrupt(&mut self) {
        self.packets.clear();
        self.interrupted = true;
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    fn observe_focus(&mut self, focused: bool) {
        if self.focused && !focused {
            self.interrupt();
        }
        self.focused = focused;
    }

    fn refresh_focus(&mut self) {
        self.observe_focus(window_focused(self.window));
    }
}
#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<State>>);
impl Capture {
    pub fn hook(&self) -> eframe::EventLoopBuilderHook {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        let shared = self.clone();
        Box::new(move |builder| {
            builder.with_msg_hook(move |pointer| {
                if pointer.is_null() {
                    return false;
                }
                // SAFETY: winit's synchronous message hook provides a live MSG on this event thread.
                let msg = unsafe { &*(pointer as *const MSG) };
                shared.message(msg);
                false // Retain native UI controls and keyboard mappings, including remapped pen buttons.
            });
        })
    }
    pub fn bind(&self, window: &winit::window::Window, ctx: egui::Context) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(h) = handle.as_raw()
        {
            state.window = h.hwnd.get() as usize;
        }
        state.ctx = Some(ctx);
        state.refresh_focus();
        let mut frequency = 0;
        // SAFETY: initialized scalar output; success and sign checked.
        if unsafe { QueryPerformanceFrequency(&mut frequency) }.is_ok() && frequency > 0 {
            state.frequency = frequency as u64;
        }
    }
    pub fn drain(&self, app: &mut vectorcraft_ui_egui::VectorcraftApp) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // Poll sent focus changes before measuring or dispatching queued input.
        state.refresh_focus();
        let mut now = 0_i64;
        // SAFETY: read-only system clock query with an initialized output. This measures
        // native packet age at UI drain, never display/presentation latency.
        let measured = unsafe { QueryPerformanceCounter(&mut now) }.is_ok() && now > 0;
        let mut measured_packets = 0_usize;
        let mut sum_ms = 0.0;
        let mut newest: Option<f64> = None;
        let mut oldest: Option<f64> = None;
        if measured {
            for (raw, _) in &state.packets {
                if let Some(age) = packet_age_ms(now as u64, raw.performance_count, raw.performance_frequency) {
                    measured_packets += 1;
                    sum_ms += age;
                    newest = Some(newest.map_or(age, |old| old.min(age)));
                    oldest = Some(oldest.map_or(age, |old| old.max(age)));
                }
            }
        }
        if measured_packets > 0 {
            state.last_newest_ms = newest;
            state.age_count = state.age_count.saturating_add(measured_packets as u64);
            state.age_sum_ms += sum_ms;
            state.age_max_ms = state.age_max_ms.max(oldest.unwrap_or(0.0));
        }
        // Sent WM_SETFOCUS/WM_KILLFOCUS can bypass winit's queued-message hook. Poll the
        // actual window state every input frame, including focus loss without pen traffic.
        app.native_pen.capture_status = Some(serde_json::json!({
            "bound": state.window != 0,
            "focused": state.focused,
            "queuedFocusMessages": state.focus_messages,
            "pointerMessages": state.pointer_messages,
            "unfocusedPointerMessages": state.unfocused_pointer_messages,
            "history": state.capture.stats,
            "packetAgeAtUiDrain":{"scope":"native QPC timestamp to UI drain, not input-to-photon",
                "measuredPackets":measured_packets,"newestMs":newest,"oldestMs":oldest,
                "totalMeasuredPackets":state.age_count,"meanMs":state.age_sum_ms/state.age_count.max(1) as f64,
                "maxMs":state.age_max_ms,"lastMeasuredBatchNewestMs":state.last_newest_ms},
        }));
        for (raw, sample) in std::mem::take(&mut state.packets) {
            app.native_pen.push(raw, sample);
        }
        if std::mem::take(&mut state.interrupted) {
            app.native_pen.interrupt();
        }
    }
    fn message(&self, msg: &MSG) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.window == 0 || msg.hwnd.0 as usize != state.window {
            return;
        }
        if msg.message == WM_SETFOCUS {
            state.focus_messages = state.focus_messages.saturating_add(1);
            state.refresh_focus();
            return;
        }
        if matches!(msg.message, WM_POINTERCAPTURECHANGED | WM_KILLFOCUS) {
            if msg.message == WM_KILLFOCUS {
                state.focus_messages = state.focus_messages.saturating_add(1);
                state.focused = false;
            }
            state.interrupt();
            return;
        }
        if !matches!(msg.message, WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP) {
            return;
        }
        state.pointer_messages = state.pointer_messages.saturating_add(1);
        // Refresh BEFORE the gate and BEFORE winit consumes this pointer's history. Binding
        // while hidden must not leave capture disabled when the window is later focused.
        state.refresh_focus();
        if !state.focused {
            state.unfocused_pointer_messages = state.unfocused_pointer_messages.saturating_add(1);
            return;
        }
        let id = (msg.wParam.0 & 0xffff) as u32;
        let mut kind = POINTER_INPUT_TYPE::default();
        // SAFETY: initialized output, ID supplied by the current window's native message.
        if unsafe { GetPointerType(id, &mut kind) }.is_err() || kind != PT_PEN {
            return;
        }
        let mut latest = POINTER_PEN_INFO::default();
        // SAFETY: initialized output on the owning message thread.
        if let Err(e) = unsafe { GetPointerPenInfo(id, &mut latest) } {
            log::warn!("native pen: {e}");
            state.interrupted = true;
            return;
        }
        let capacity = (latest.pointerInfo.historyCount as usize).clamp(1, HISTORY_LIMIT);
        if latest.pointerInfo.historyCount as usize > HISTORY_LIMIT {
            state.interrupted = true;
        }
        let mut history = vec![POINTER_PEN_INFO::default(); capacity];
        let mut count = capacity as u32;
        // SAFETY: API count matches the initialized allocation. Read before winit consumes history.
        if unsafe { GetPointerPenInfoHistory(id, &mut count, Some(history.as_mut_ptr())) }.is_err() {
            history = vec![latest];
            if latest.pointerInfo.historyCount > 1 {
                state.interrupted = true;
            }
        } else {
            history.truncate((count as usize).min(capacity));
        }
        let frequency = state.frequency;
        let packets = history
            .into_iter()
            .map(|p| {
                let info = p.pointerInfo;
                let at = info.ptPixelLocationRaw;
                let mut client = POINT { x: at.x, y: at.y };
                // SAFETY: current HWND and initialized POINT; unavailable mapping stays None.
                let client_px = unsafe { ScreenToClient(info.hwndTarget, &mut client) }.as_bool().then_some([client.x, client.y]);
                // SAFETY: target window borrowed synchronously from pointer metadata.
                let dpi = unsafe { GetDpiForWindow(info.hwndTarget) };
                RawPenPacket {
                    pointer_id: info.pointerId,
                    frame_id: info.frameId,
                    device: info.sourceDevice.0 as usize as u64,
                    screen_px: [at.x, at.y],
                    client_px,
                    window_dpi: dpi,
                    performance_count: info.PerformanceCount,
                    performance_frequency: frequency,
                    system_tick_ms: info.dwTime,
                    pointer_flags: info.pointerFlags.0,
                    pen_flags: p.penFlags,
                    channel_mask: p.penMask,
                    pressure: p.pressure,
                    tilt_x: p.tiltX,
                    tilt_y: p.tiltY,
                    rotation: p.rotation,
                    button_change: info.ButtonChangeType.0,
                    key_states: info.dwKeyStates,
                }
            })
            .collect();
        let accepted = state.capture.history(packets, count.max(latest.pointerInfo.historyCount));
        if state.packets.len() + accepted.len() > 8192 {
            state.packets.clear();
            state.interrupted = true;
        }
        state.packets.extend(accepted);
        if let Some(ctx) = &state.ctx {
            ctx.request_repaint();
        }
    }
}

fn packet_age_ms(now: u64, count: u64, frequency: u64) -> Option<f64> {
    if count == 0 || frequency == 0 || count > now {
        return None;
    }
    Some((now - count) as f64 / frequency as f64 * 1000.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn packet_age_requires_real_clock_and_never_labels_future_samples_as_fast() {
        assert_eq!(super::packet_age_ms(1000, 975, 1000), Some(25.0));
        assert_eq!(super::packet_age_ms(1000, 0, 1000), None);
        assert_eq!(super::packet_age_ms(1000, 975, 0), None);
        assert_eq!(super::packet_age_ms(1000, 1001, 1000), None);
    }
    use super::*;

    #[test]
    fn hidden_bind_recovers_without_a_queued_setfocus_message() {
        let mut state = State::default();
        state.observe_focus(false);
        assert!(!state.focused);
        // This snapshot is queried before pointer history; no WM_SETFOCUS hook is required.
        state.observe_focus(owns_focus(7, 7, 7, false));
        assert!(state.focused);
        assert!(!state.interrupted);
    }

    #[test]
    fn background_and_foreign_windows_are_not_pen_targets() {
        assert!(!owns_focus(0, 0, 0, false));
        assert!(!owns_focus(7, 9, 7, false));
        assert!(!owns_focus(7, 7, 0, false));
        assert!(!owns_focus(7, 7, 9, false));
        assert!(owns_focus(7, 7, 9, true));
    }

    #[test]
    fn missed_killfocus_cancels_once_and_refocus_does_not_resume_a_stroke() {
        let mut state = State::default();
        state.observe_focus(true);
        let raw = RawPenPacket {
            pointer_id: 1,
            frame_id: 1,
            device: 1,
            screen_px: [10, 20],
            client_px: Some([10, 20]),
            window_dpi: 96,
            performance_count: 1,
            performance_frequency: 1000,
            system_tick_ms: 1,
            pointer_flags: 4,
            pen_flags: 0,
            channel_mask: 0,
            pressure: 0,
            tilt_x: 0,
            tilt_y: 0,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        };
        state.packets.push((raw.clone(), raw.normalize(false).unwrap()));
        state.observe_focus(false);
        assert!(state.packets.is_empty());
        assert!(std::mem::take(&mut state.interrupted));
        state.observe_focus(false);
        assert!(!state.interrupted);
        state.observe_focus(true);
        assert!(state.focused);
        assert!(!state.interrupted);
        assert!(state.packets.is_empty());
    }

    #[test]
    fn foreign_focus_message_does_not_interrupt_the_bound_window() {
        let capture = Capture::default();
        {
            let mut state = capture.0.lock().unwrap();
            state.window = 7;
            state.focused = true;
        }
        capture.message(&MSG { hwnd: HWND(9usize as *mut std::ffi::c_void), message: WM_KILLFOCUS, ..Default::default() });
        let state = capture.0.lock().unwrap();
        assert!(state.focused);
        assert!(!state.interrupted);
    }

    #[test]
    fn unfocused_pointer_message_is_counted_but_never_read_as_pen_history() {
        let capture = Capture::default();
        capture.0.lock().unwrap().window = 7;
        capture.message(&MSG { hwnd: HWND(7usize as *mut std::ffi::c_void), message: WM_POINTERUPDATE, ..Default::default() });
        let state = capture.0.lock().unwrap();
        assert_eq!(state.pointer_messages, 1);
        assert_eq!(state.unfocused_pointer_messages, 1);
        assert_eq!(state.capture.stats.received, 0);
        assert!(state.packets.is_empty());
    }
}
