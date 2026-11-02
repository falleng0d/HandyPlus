//! Handy+ actions tied to a recording session: user scripts, lowering the
//! system volume and pausing Dragon. `started` and `ended` pair up even when
//! a recording is cancelled, and run at most once per session.

use crate::settings::get_settings;
use crate::TranscriptionCoordinator;
use log::{debug, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Recording was accepted and the microphone is starting.
pub fn started(app: &AppHandle) {
    if SESSION_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    let settings = get_settings(app);
    run_script(&settings.on_recording_start_script, "recording start");
    if settings.pause_dragon_when_dictating {
        crate::plus::dragon::pause(app.clone());
    }
}

/// The microphone delivers samples and the start sound has played.
pub fn ready(app: &AppHandle) {
    if !SESSION_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let settings = get_settings(app);
    if settings.lower_volume_while_recording {
        crate::plus::volume::lower_to(settings.volume_while_recording);
    }
}

/// Recording stopped or was cancelled.
pub fn ended(app: &AppHandle) {
    crate::plus::volume::restore();
    if !SESSION_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let settings = get_settings(app);
    run_script(&settings.on_recording_end_script, "recording end");
    if settings.pause_dragon_when_dictating {
        crate::plus::dragon::resume(app.clone());
    }
}

/// The voice detector started or stopped passing speech.
pub fn speech_changed(app: &AppHandle, active: bool) {
    crate::plus::overlay::emit_speech_active(app, active);
    if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
        coordinator.notify_speech(active);
    }
}

/// How long a held recording may go without speech before it stops; `None`
/// when the setting is zero.
pub fn auto_stop_silence(app: &AppHandle) -> Option<Duration> {
    let secs = get_settings(app).auto_stop_silence_secs;
    (secs > 0.0)
        .then(|| Duration::try_from_secs_f32(secs).ok())
        .flatten()
}

fn run_script(script: &str, context: &'static str) {
    let script = script.trim().to_string();
    if script.is_empty() {
        return;
    }
    std::thread::spawn(move || match script_command(&script).output() {
        Ok(output) if output.status.success() => debug!("[{context}] script finished"),
        Ok(output) => warn!(
            "[{context}] script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) => warn!("[{context}] script could not start: {e}"),
    });
}

#[cfg(windows)]
fn script_command(script: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = std::process::Command::new("pwsh");
    command
        .args(["-NoProfile", "-NoLogo", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(windows))]
fn script_command(script: &str) -> std::process::Command {
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg(script);
    command
}
