use crate::managers::audio::AudioRecordingManager;
use crate::settings::{self, AppSettings};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Persists a voice-detection tuning change and rebuilds the detector with it.
/// The previous settings are restored if the detector can't be rebuilt (for
/// example while a recording is in progress).
async fn apply_vad_tuning(
    app: AppHandle,
    change: impl FnOnce(&mut AppSettings),
) -> Result<(), String> {
    let previous = settings::get_settings(&app);
    let mut updated = previous.clone();
    change(&mut updated);
    let backend = updated.vad_backend;
    settings::write_settings(&app, updated);

    let manager = app.state::<Arc<AudioRecordingManager>>().inner().clone();
    let rebuilt = tokio::task::spawn_blocking(move || manager.update_vad_backend(backend))
        .await
        .map_err(|e| format!("audio task join failed: {e}"))
        .and_then(|result| result.map_err(|e| format!("Failed to update voice detection: {e}")));
    if rebuilt.is_err() {
        settings::write_settings(&app, previous);
    }
    rebuilt
}

fn update_settings(app: &AppHandle, change: impl FnOnce(&mut AppSettings)) {
    let mut current = settings::get_settings(app);
    change(&mut current);
    settings::write_settings(app, current);
}

#[tauri::command]
#[specta::specta]
pub fn change_context_lines_setting(app: AppHandle, lines: u32) -> Result<(), String> {
    update_settings(&app, |s| s.context_lines = lines);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_context_max_characters_setting(
    app: AppHandle,
    max_characters: Option<u32>,
) -> Result<(), String> {
    update_settings(&app, |s| s.context_max_characters = max_characters);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn change_vad_threshold_setting(app: AppHandle, threshold: f32) -> Result<(), String> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(format!(
            "Threshold must be between 0 and 1, got {threshold}"
        ));
    }
    apply_vad_tuning(app, |s| s.vad_threshold = threshold).await
}

#[tauri::command]
#[specta::specta]
pub async fn change_vad_prefill_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    apply_vad_tuning(app, |s| s.vad_prefill_ms = ms).await
}

#[tauri::command]
#[specta::specta]
pub async fn change_vad_hangover_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    apply_vad_tuning(app, |s| s.vad_hangover_ms = ms).await
}

#[tauri::command]
#[specta::specta]
pub async fn change_vad_onset_ms_setting(app: AppHandle, ms: u64) -> Result<(), String> {
    apply_vad_tuning(app, |s| s.vad_onset_ms = ms).await
}
