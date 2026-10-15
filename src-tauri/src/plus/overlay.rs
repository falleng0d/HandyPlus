use tauri::{AppHandle, Emitter};

/// Tells the overlay whether the voice detector is currently passing speech.
pub fn emit_speech_active(app: &AppHandle, active: bool) {
    let _ = app.emit_to("recording_overlay", "vad-active", active);
}
