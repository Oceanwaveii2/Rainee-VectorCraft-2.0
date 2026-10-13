//! Win32 FFI is isolated here, following the app's system_fonts.rs convention.
//! All buffers are initialized and bounded. No window procedure is replaced and no input is injected.
#![allow(unsafe_code)]

use crate::{
    Log, Record,
    pen_input::{HISTORY_LIMIT, PenCapture, PenSample, RawPenPacket, is_pen_promoted_mouse},
    recording::Scenario,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::POINT,
    Graphics::Gdi::ScreenToClient,
    System::Performance::QueryPerformanceFrequency,
    UI::{
        HiDpi::GetDpiForWindow,
        Input::Pointer::{GetPointerPenInfo, GetPointerPenInfoHistory, GetPointerType, POINTER_PEN_INFO},
        WindowsAndMessaging::{
            GetMessageExtraInfo, MSG, POINTER_INPUT_TYPE, PT_PEN, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
            WM_MOUSEMOVE, WM_POINTERCAPTURECHANGED, WM_POINTERDOWN, WM_POINTERENTER, WM_POINTERLEAVE, WM_POINTERUP, WM_POINTERUPDATE, WM_RBUTTONDOWN,
            WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
        },
    },
};
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct State {
    capture: PenCapture,
    log: Log,
    last: Option<PenSample>,
    last_raw: Option<RawPenPacket>,
    frequency: u64,
    native_errors: u64,
    promoted: u64,
    scenario: Scenario,
    auxiliary_count: u64,
    last_key: Option<u64>,
}

impl State {
    fn error(&mut self, message: String) {
        self.native_errors += 1;
        self.log.record(Record::Diagnostic { message });
    }
    fn message(&mut self, msg: &MSG) {
        if matches!(msg.message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
            // Only messages delivered to this application's event loop, never global keyboard hooks.
            // Virtual key/scan metadata only, no characters or text fields. Remapped pen buttons
            // remain keyboard actions, never fabricated physical barrel flags or ink samples.
            self.auxiliary_count += 1;
            self.last_key = Some(msg.wParam.0 as u64);
            self.log.record(Record::Auxiliary { message: msg.message, code: msg.wParam.0 as u64, detail: msg.lParam.0 as i64, tick_ms: msg.time });
            if msg.message == WM_KEYDOWN
                && msg.lParam.0 & (1 << 30) == 0
                && let Some(scenario) = Scenario::function_key(msg.wParam.0 as u64)
            {
                self.scenario = scenario;
                self.log.record(Record::Scenario { scenario });
            }
            return;
        }
        if matches!(
            msg.message,
            WM_MOUSEMOVE
                | WM_LBUTTONDOWN
                | WM_LBUTTONUP
                | WM_RBUTTONDOWN
                | WM_RBUTTONUP
                | WM_MBUTTONDOWN
                | WM_MBUTTONUP
                | WM_XBUTTONDOWN
                | WM_XBUTTONUP
        ) {
            // SAFETY: queried synchronously on the event thread for its current message. This
            // is metadata only: the hook returns false and never eats mouse input.
            if is_pen_promoted_mouse(unsafe { GetMessageExtraInfo() }.0 as usize) {
                self.promoted += 1;
            }
            if msg.message != WM_MOUSEMOVE {
                self.auxiliary_count += 1;
                self.log.record(Record::Auxiliary {
                    message: msg.message,
                    code: msg.wParam.0 as u64,
                    detail: msg.lParam.0 as i64,
                    tick_ms: msg.time,
                });
            }
            return;
        }
        if !matches!(msg.message, WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERENTER | WM_POINTERLEAVE | WM_POINTERCAPTURECHANGED) {
            return;
        }
        let id = (msg.wParam.0 & 0xffff) as u32;
        if msg.message == WM_POINTERCAPTURECHANGED || msg.message == WM_POINTERLEAVE {
            self.log.record(Record::Lifecycle {
                pointer_id: id,
                kind: if msg.message == WM_POINTERLEAVE { "window_leave" } else { "capture_lost" }.into(),
            });
            return;
        }
        let mut kind = POINTER_INPUT_TYPE::default();
        // SAFETY: initialized output, valid pointer id from the current window message; failure checked.
        if unsafe { GetPointerType(id, &mut kind) }.is_err() || kind != PT_PEN {
            return;
        }
        let mut latest = POINTER_PEN_INFO::default();
        // SAFETY: initialized output lives throughout the synchronous API call on the owning event thread.
        if let Err(e) = unsafe { GetPointerPenInfo(id, &mut latest) } {
            self.error(format!("GetPointerPenInfo({id}): {e}"));
            return;
        }
        let capacity = (latest.pointerInfo.historyCount as usize).clamp(1, HISTORY_LIMIT);
        let mut native = vec![POINTER_PEN_INFO::default(); capacity];
        let mut count = capacity as u32;
        // SAFETY: allocated count initialized structs; Win32 cannot write beyond entriesCount.
        // Read before winit handles the message and potentially skips the pointer frame history.
        let history = unsafe { GetPointerPenInfoHistory(id, &mut count, Some(native.as_mut_ptr())) };
        let available = count.max(latest.pointerInfo.historyCount);
        if let Err(e) = history {
            self.error(format!("GetPointerPenInfoHistory({id}): {e}; retaining latest packet only"));
            native.clear();
            native.push(latest);
        } else {
            native.truncate((count as usize).min(capacity));
            if native.is_empty() {
                self.error("Empty pen history; retaining latest packet only".into());
                native.push(latest);
            }
        }
        let packets = native
            .into_iter()
            .map(|p| {
                let info = p.pointerInfo;
                let raw = info.ptPixelLocationRaw;
                let mut client = POINT { x: raw.x, y: raw.y };
                // SAFETY: borrowed HWND from the current native packet and initialized writable POINT.
                // ScreenToClient converts physical screen pixels to this DPI-aware window's client space.
                let client_px = unsafe { ScreenToClient(info.hwndTarget, &mut client) }.as_bool().then_some([client.x, client.y]);
                // SAFETY: queried only for the target HWND on the owning thread; 0 means unavailable.
                let dpi = unsafe { GetDpiForWindow(info.hwndTarget) };
                RawPenPacket {
                    pointer_id: info.pointerId,
                    frame_id: info.frameId,
                    device: info.sourceDevice.0 as usize as u64,
                    screen_px: [raw.x, raw.y],
                    client_px,
                    window_dpi: dpi,
                    performance_count: info.PerformanceCount,
                    performance_frequency: self.frequency,
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
        for (raw, normalized) in self.capture.history(packets, available) {
            self.last = Some(normalized.clone());
            self.last_raw = Some(raw.clone());
            // Native packets alone are authoritative. Never append winit Touch or promoted mouse
            // as a second pen sample; genuine mouse messages still reach window controls.
            self.log.record(Record::Sample { raw, normalized });
        }
    }
}

struct Probe {
    state: Rc<RefCell<State>>,
    window: Option<Window>,
    last_title: Instant,
    error: Option<String>,
}

impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        match event_loop.create_window(
            Window::default_attributes()
                .with_title("Cintiq input test | Hover and draw here | Close to save recording")
                .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 650.0)),
        ) {
            Ok(w) => self.window = Some(w),
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
        if matches!(event, WindowEvent::RedrawRequested)
            && let (Some(window), Ok(state)) = (&self.window, self.state.try_borrow())
            && let Err(error) = paint(window, &state)
        {
            self.error = Some(error);
            event_loop.exit();
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_title.elapsed() >= Duration::from_millis(100) {
            if let (Some(window), Ok(state)) = (&self.window, self.state.try_borrow()) {
                let description = state
                    .last
                    .as_ref()
                    .map(|s| {
                        format!(
                            "{:?} P={:?} Tilt={:?} Eraser={} Button={} Roll=unavailable",
                            s.phase, s.pressure, s.tilt_degrees, s.eraser, s.barrel_button
                        )
                    })
                    .unwrap_or_else(|| "No native pen packets yet: hover/draw inside window".into());
                window.set_title(&format!(
                    "Cintiq test {:?} | {description} | packets={} duplicates={} lost={} errors={} keys={:?}{}",
                    state.scenario,
                    state.capture.stats.accepted,
                    state.capture.stats.repeated_history,
                    state.capture.stats.truncated_history + state.log.dropped.load(Ordering::Relaxed),
                    state.native_errors,
                    state.last_key,
                    if state.log.failed.load(Ordering::Relaxed) { " LOG FAILED" } else { "" }
                ));
                window.request_redraw();
            }
            self.last_title = Instant::now();
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100)));
    }
}

pub fn run(log: Log) -> Result<(), String> {
    let mut frequency = 0i64;
    // SAFETY: initialized output of the system-wide performance counter frequency; error propagated.
    unsafe { QueryPerformanceFrequency(&mut frequency) }.map_err(|e| e.to_string())?;
    if frequency <= 0 {
        return Err("Invalid native performance counter frequency".into());
    }
    let state = Rc::new(RefCell::new(State {
        capture: PenCapture::default(),
        log: log.clone(),
        last: None,
        last_raw: None,
        frequency: frequency as u64,
        native_errors: 0,
        promoted: 0,
        scenario: Scenario::default(),
        auxiliary_count: 0,
        last_key: None,
    }));
    let hook_state = state.clone();
    let mut builder = EventLoop::builder();
    builder.with_msg_hook(move |pointer| {
        if pointer.is_null() {
            return false;
        }
        // SAFETY: winit's with_msg_hook provides a live MSG pointer for this synchronous callback.
        // It is borrowed only here, never retained. The hook is installed solely on our own event loop.
        let msg = unsafe { &*pointer.cast::<MSG>() };
        if let Ok(mut state) = hook_state.try_borrow_mut() {
            state.message(msg);
        }
        false // Observe; never consume messages or change winit/editor routing in this pilot.
    });
    let event_loop = builder.build().map_err(|e| e.to_string())?;
    let mut probe = Probe { state: state.clone(), window: None, last_title: Instant::now(), error: None };
    let result = event_loop.run_app(&mut probe).map_err(|e| e.to_string());
    if let Ok(state) = state.try_borrow() {
        // On close, blocking is safe: input has ended. Ensure the final summary is not lost to a full queue.
        log.send
            .send(Record::Summary {
                stats: state.capture.stats.clone(),
                native_errors: state.native_errors,
                promoted_mouse_messages: state.promoted,
                log_queue_dropped: log.dropped.load(Ordering::Relaxed),
            })
            .map_err(|e| e.to_string())?;
        println!(
            "Captured {} packets, {} repeated-history packets, {} history truncations, {} native errors, {} logging drops",
            state.capture.stats.accepted,
            state.capture.stats.repeated_history,
            state.capture.stats.truncated_history,
            state.native_errors,
            log.dropped.load(Ordering::Relaxed)
        );
    }
    if let Some(error) = probe.error {
        return Err(error);
    }
    result
}

/// Readable live instrument panel; OS text and original solid colors, no third-party artwork.
fn paint(window: &Window, state: &State) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{COLORREF, HWND, RECT};
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
    use windows::core::w;
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Test window has no Win32 handle".into());
    };
    let hwnd = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
    let mut rect = RECT::default();
    // SAFETY: HWND is borrowed from the live window on its event thread; outputs initialized.
    unsafe { GetClientRect(hwnd, &mut rect) }.map_err(|e| e.to_string())?;
    // SAFETY: GDI objects belong to this function. Restore the previous font before deleting
    // ours, release the DC on every path, and never store any native pointer after returning.
    unsafe {
        let dc = GetDC(Some(hwnd));
        if dc.0.is_null() {
            return Err("Could not acquire test window paint context".into());
        }
        let brush = CreateSolidBrush(COLORREF(0x00221c18));
        if brush.0.is_null() {
            ReleaseDC(Some(hwnd), dc);
            return Err("Could not create panel brush".into());
        }
        let scale = window.scale_factor().clamp(0.5, 4.0);
        let font = CreateFontW(
            (24.0 * scale) as i32,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
            0,
            w!("Segoe UI"),
        );
        if font.0.is_null() {
            // Best-effort cleanup on allocation failure; preserve the original failure.
            let _ = DeleteObject(HGDIOBJ(brush.0));
            ReleaseDC(Some(hwnd), dc);
            return Err("Could not create panel font".into());
        }
        FillRect(dc, &rect, brush);
        let previous = SelectObject(dc, HGDIOBJ(font.0));
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00eee8e5));
        let mut lines = vec![
            "Cintiq Native Input Test - Windows Ink".to_string(),
            "Move this window onto your Cintiq; hover and draw inside it.".into(),
            "F1 slow curves | F2 fast strokes | F3 corners | F4 reversals".into(),
            "F5 pressure changes | F6 buttons (configured shortcuts: [ and ])".into(),
            "This is a sensor test, not a drawing canvas. Close to finish the log.".into(),
            format!("Scenario: {:?}   Key code: {:?}   Auxiliary events: {}", state.scenario, state.last_key, state.auxiliary_count),
        ];
        if let Some(sample) = &state.last {
            lines.push(format!("Phase: {:?}   Pressure: {:?}   Tilt X/Y: {:?}", sample.phase, sample.pressure, sample.tilt_degrees));
            lines.push(format!("Eraser: {}   Inverted: {}   Barrel button: {}", sample.eraser, sample.inverted, sample.barrel_button));
            lines.push(format!("Pointer button bits: 0x{:x}   Barrel roll: unavailable", sample.pointer_buttons));
            if let Some(raw) = &state.last_raw {
                lines.push(format!("Screen: {:?}   Client: {:?}   DPI: {}", raw.screen_px, raw.client_px, raw.window_dpi));
                lines.push(format!(
                    "QPC: {} / {}   Tick ms: {}   Channel mask: 0x{:x}",
                    raw.performance_count, raw.performance_frequency, raw.system_tick_ms, raw.channel_mask
                ));
            }
        } else {
            lines.push("Waiting for native pen packets (ordinary mouse does not count).".into());
        }
        lines.push("".into());
        lines.push(format!(
            "Accepted: {}   Repeated history: {}   Invalid: {}",
            state.capture.stats.accepted, state.capture.stats.repeated_history, state.capture.stats.invalid
        ));
        lines.push(format!(
            "History truncated: {}   Log drops: {}   Native errors: {}",
            state.capture.stats.truncated_history,
            state.log.dropped.load(Ordering::Relaxed),
            state.native_errors
        ));
        lines.push(format!("Pen-promoted mouse messages observed: {} (not recorded as pen samples)", state.promoted));
        if state.log.failed.load(Ordering::Relaxed) {
            lines.push("ERROR: recording writer failed - this capture cannot pass acceptance.".into());
        }
        let mut drawn = true;
        for (index, line) in lines.iter().enumerate() {
            let text: Vec<u16> = line.encode_utf16().collect();
            drawn &= TextOutW(dc, (20.0 * scale) as i32, ((20.0 + index as f64 * 35.0) * scale) as i32, &text).as_bool();
        }
        SelectObject(dc, previous);
        let font_deleted = DeleteObject(HGDIOBJ(font.0)).as_bool();
        let brush_deleted = DeleteObject(HGDIOBJ(brush.0)).as_bool();
        ReleaseDC(Some(hwnd), dc);
        if !font_deleted || !brush_deleted {
            return Err("Could not release sensor panel graphics objects".into());
        }
        if !drawn {
            return Err("Could not draw sensor status text".into());
        }
    }
    Ok(())
}
