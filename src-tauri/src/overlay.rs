use crate::settings;
use crate::settings::OverlayPosition;
use enigo::{Enigo, Mouse};
use log::{debug, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindowBuilder};

const OVERLAY_WIDTH: f64 = 172.0;
const OVERLAY_HEIGHT: f64 = 36.0;

#[cfg(target_os = "macos")]
const OVERLAY_TOP_OFFSET: f64 = 46.0;
#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_TOP_OFFSET: f64 = 4.0;

#[cfg(target_os = "macos")]
const OVERLAY_BOTTOM_OFFSET: f64 = 15.0;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_BOTTOM_OFFSET: f64 = 40.0;

static OVERLAY_VISIBILITY_TOKEN: AtomicU64 = AtomicU64::new(0);

fn log_overlay_window_result(action: &str, result: tauri::Result<()>) {
    match result {
        Ok(()) => debug!("Recording overlay: {}", action),
        Err(err) => warn!("Recording overlay: failed to {}: {}", action, err),
    }
}

#[cfg(target_os = "windows")]
fn refresh_overlay_always_on_top(window: &tauri::WebviewWindow, reason: &str) {
    debug!(
        "Recording overlay: forcing always-on-top refresh on Windows ({})",
        reason
    );
    log_overlay_window_result(
        "clear always-on-top before reapplying",
        window.set_always_on_top(false),
    );
    log_overlay_window_result("reapply always-on-top", window.set_always_on_top(true));
}

#[cfg(not(target_os = "windows"))]
fn refresh_overlay_always_on_top(window: &tauri::WebviewWindow, reason: &str) {
    debug!("Recording overlay: ensuring always-on-top ({})", reason);
    log_overlay_window_result("ensure always-on-top", window.set_always_on_top(true));
}

#[cfg(target_os = "windows")]
fn clear_overlay_always_on_top(window: &tauri::WebviewWindow, reason: &str) {
    debug!(
        "Recording overlay: clearing always-on-top on Windows ({})",
        reason
    );
    log_overlay_window_result("clear always-on-top", window.set_always_on_top(false));
}

#[cfg(not(target_os = "windows"))]
fn clear_overlay_always_on_top(_window: &tauri::WebviewWindow, _reason: &str) {}

fn show_overlay_window(app_handle: &AppHandle, overlay_state: &str) {
    let _visibility_token = bump_overlay_visibility_token();
    update_overlay_position(app_handle);

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        debug!("Recording overlay: showing '{}' state", overlay_state);
        log_overlay_window_result("show window", overlay_window.show());
        refresh_overlay_always_on_top(&overlay_window, overlay_state);

        match overlay_window.emit("show-overlay", overlay_state) {
            Ok(()) => debug!(
                "Recording overlay: emitted show-overlay for '{}'",
                overlay_state
            ),
            Err(err) => warn!(
                "Recording overlay: failed to emit show-overlay for '{}': {}",
                overlay_state, err
            ),
        }
    } else {
        warn!(
            "Recording overlay: window not found while showing '{}'",
            overlay_state
        );
    }
}

fn bump_overlay_visibility_token() -> u64 {
    OVERLAY_VISIBILITY_TOKEN.fetch_add(1, Ordering::SeqCst) + 1
}

fn get_monitor_with_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    let enigo = Enigo::new(&Default::default());
    if let Ok(enigo) = enigo {
        if let Ok(mouse_location) = enigo.location() {
            if let Ok(monitors) = app_handle.available_monitors() {
                for monitor in monitors {
                    let is_within =
                        is_mouse_within_monitor(mouse_location, monitor.position(), monitor.size());
                    if is_within {
                        return Some(monitor);
                    }
                }
            }
        }
    }

    app_handle.primary_monitor().ok().flatten()
}

fn is_mouse_within_monitor(
    mouse_pos: (i32, i32),
    monitor_pos: &PhysicalPosition<i32>,
    monitor_size: &PhysicalSize<u32>,
) -> bool {
    let (mouse_x, mouse_y) = mouse_pos;
    let PhysicalPosition {
        x: monitor_x,
        y: monitor_y,
    } = *monitor_pos;
    let PhysicalSize {
        width: monitor_width,
        height: monitor_height,
    } = *monitor_size;

    mouse_x >= monitor_x
        && mouse_x < (monitor_x + monitor_width as i32)
        && mouse_y >= monitor_y
        && mouse_y < (monitor_y + monitor_height as i32)
}

fn calculate_overlay_position(app_handle: &AppHandle) -> Option<(f64, f64)> {
    if let Some(monitor) = get_monitor_with_cursor(app_handle) {
        let work_area = monitor.work_area();
        let scale = monitor.scale_factor();
        let work_area_width = work_area.size.width as f64 / scale;
        let work_area_height = work_area.size.height as f64 / scale;
        let work_area_x = work_area.position.x as f64 / scale;
        let work_area_y = work_area.position.y as f64 / scale;

        let settings = settings::get_settings(app_handle);

        let x = work_area_x + (work_area_width - OVERLAY_WIDTH) / 2.0;
        let y = match settings.overlay_position {
            OverlayPosition::Top => work_area_y + OVERLAY_TOP_OFFSET,
            OverlayPosition::Bottom | OverlayPosition::None => {
                // don't subtract the overlay height it puts it too far up
                work_area_y + work_area_height - OVERLAY_BOTTOM_OFFSET
            }
        };

        return Some((x, y));
    }
    None
}

/// Creates the recording overlay window and keeps it hidden by default
pub fn create_recording_overlay(app_handle: &AppHandle) {
    if let Some((x, y)) = calculate_overlay_position(app_handle) {
        match WebviewWindowBuilder::new(
            app_handle,
            "recording_overlay",
            tauri::WebviewUrl::App("src/overlay/index.html".into()),
        )
        .title("Recording")
        .position(x, y)
        .resizable(false)
        .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
        .shadow(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .accept_first_mouse(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()
        {
            Ok(_window) => {
                debug!("Recording overlay window created successfully (hidden)");
            }
            Err(e) => {
                debug!("Failed to create recording overlay window: {}", e);
            }
        }
    }
}

/// Shows the recording overlay window with fade-in animation
pub fn show_recording_overlay(app_handle: &AppHandle) {
    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        return;
    }

    show_overlay_window(app_handle, "recording");
}

/// Shows the transcribing overlay window
pub fn show_transcribing_overlay(app_handle: &AppHandle) {
    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        return;
    }

    show_overlay_window(app_handle, "transcribing");
}

/// Updates the overlay window position based on current settings
pub fn update_overlay_position(app_handle: &AppHandle) {
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
    }
}

/// Hides the recording overlay window with fade-out animation
pub fn hide_recording_overlay(app_handle: &AppHandle) {
    // Always hide the overlay regardless of settings - if setting was changed while recording,
    // we still want to hide it properly
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        debug!("Recording overlay: requested hide with fade-out animation");
        match overlay_window.emit("hide-overlay", ()) {
            Ok(()) => debug!("Recording overlay: emitted hide-overlay"),
            Err(err) => warn!("Recording overlay: failed to emit hide-overlay: {}", err),
        }
        // Hide the window after a short delay to allow animation to complete
        let window_clone = overlay_window.clone();
        let hide_token = bump_overlay_visibility_token();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            if OVERLAY_VISIBILITY_TOKEN.load(Ordering::SeqCst) == hide_token {
                debug!(
                    "Recording overlay: executing hide for visibility token {}",
                    hide_token
                );
                clear_overlay_always_on_top(&window_clone, "before hide");
                log_overlay_window_result("hide window", window_clone.hide());
            } else {
                debug!(
                    "Recording overlay: skipping stale hide for visibility token {}",
                    hide_token
                );
            }
        });
    } else {
        warn!("Recording overlay: window not found while hiding");
    }
}

pub fn emit_levels(app_handle: &AppHandle, levels: &Vec<f32>) {
    // emit levels to main app
    let _ = app_handle.emit("mic-level", levels);

    // also emit to the recording overlay if it's open
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let _ = overlay_window.emit("mic-level", levels);
    }
}
