//! Language shortcuts. Each configured language can have its own shortcut,
//! and one more shortcut cycles through them in order. Applying a language
//! also switches the transcription model and post-processing prompt when the
//! language has them set.

use crate::audio_feedback::{self, SoundType};
use crate::managers::model::ModelManager;
use crate::settings::{self, get_settings, AppSettings, ShortcutBinding, SoundTheme};
use crate::settings_plus::LanguageConfig;
use log::{debug, info, warn};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

const CYCLE_ID: &str = "language_cycle";
const LANGUAGE_PREFIX: &str = "language:";

fn shortcut_bindings(settings: &AppSettings) -> Vec<ShortcutBinding> {
    let cycle = (CYCLE_ID.to_string(), &settings.language_cycle_shortcut);
    let per_language = settings.language_configs.iter().map(|config| {
        (
            format!("{LANGUAGE_PREFIX}{}", config.id),
            &config.shortcut_binding,
        )
    });
    std::iter::once(cycle)
        .chain(per_language)
        .filter(|(_, keys)| !keys.trim().is_empty())
        .map(|(id, keys)| ShortcutBinding {
            name: id.clone(),
            id,
            description: String::new(),
            default_binding: String::new(),
            current_binding: keys.clone(),
        })
        .collect()
}

/// Registers every language shortcut and returns the ones that failed.
pub fn register_all(app: &AppHandle) -> Vec<(ShortcutBinding, String)> {
    let mut failures = Vec::new();
    for binding in shortcut_bindings(&get_settings(app)) {
        if let Err(e) = crate::shortcut::register_shortcut(app, binding.clone()) {
            debug!("Could not register language shortcut '{}': {e}", binding.id);
            failures.push((binding, e));
        }
    }
    failures
}

pub fn unregister_all(app: &AppHandle) {
    for binding in shortcut_bindings(&get_settings(app)) {
        if let Err(e) = crate::shortcut::unregister_shortcut(app, binding.clone()) {
            debug!(
                "Could not unregister language shortcut '{}': {e}",
                binding.id
            );
        }
    }
}

/// Saves the change and re-registers the language shortcuts. When a new or
/// changed shortcut can't be registered, the previous settings and shortcuts
/// are put back.
pub fn update(app: &AppHandle, change: impl FnOnce(&mut AppSettings)) -> Result<(), String> {
    let previous = get_settings(app);
    let previous_bindings = shortcut_bindings(&previous);
    let is_new = |binding: &ShortcutBinding| {
        !previous_bindings
            .iter()
            .any(|old| old.id == binding.id && old.current_binding == binding.current_binding)
    };
    let mut updated = previous.clone();
    change(&mut updated);
    if let Some(keys) = taken_shortcut(&updated, is_new) {
        return Err(format!("'{keys}' is already used by another shortcut"));
    }

    unregister_all(app);
    settings::write_settings(app, updated);
    let new_failures: Vec<String> = register_all(app)
        .into_iter()
        .filter(|(binding, _)| is_new(binding))
        .map(|(binding, e)| format!("'{}': {e}", binding.current_binding))
        .collect();
    if new_failures.is_empty() {
        return Ok(());
    }

    unregister_all(app);
    settings::write_settings(app, previous);
    register_all(app);
    Err(format!(
        "Could not register shortcut {}",
        new_failures.join(", ")
    ))
}

/// Finds a new language shortcut whose keys another shortcut already uses.
/// Registration alone can't catch this while shortcuts are paused for the
/// shortcut recorder.
fn taken_shortcut(
    settings: &AppSettings,
    is_new: impl Fn(&ShortcutBinding) -> bool,
) -> Option<String> {
    let normalize = |keys: &str| keys.to_lowercase().replace(' ', "");
    let language_bindings = shortcut_bindings(settings);
    let (added, kept): (Vec<_>, Vec<_>) = language_bindings.iter().partition(|b| is_new(b));
    let mut taken: Vec<String> = settings
        .bindings
        .values()
        .chain(kept)
        .map(|binding| normalize(&binding.current_binding))
        .collect();
    for binding in added {
        let keys = normalize(&binding.current_binding);
        if taken.contains(&keys) {
            return Some(binding.current_binding.clone());
        }
        taken.push(keys);
    }
    None
}

/// Runs a language shortcut. Returns false when `binding_id` is not one.
pub fn handle_shortcut(app: &AppHandle, binding_id: &str, is_pressed: bool) -> bool {
    let config_id = if binding_id == CYCLE_ID {
        None
    } else if let Some(id) = binding_id.strip_prefix(LANGUAGE_PREFIX) {
        Some(id)
    } else {
        return false;
    };
    if !is_pressed {
        return true;
    }

    let settings = get_settings(app);
    match config_id {
        None => {
            if let Some(next) = next_in_cycle(&settings) {
                play_sounds(app, settings.language_shortcut_sound_theme);
                apply(app, next.clone());
            }
        }
        Some(id) => {
            if let Some(config) = settings.language_configs.iter().find(|c| c.id == id) {
                apply(app, config.clone());
            }
        }
    }
    true
}

fn next_in_cycle(settings: &AppSettings) -> Option<&LanguageConfig> {
    let configs = &settings.language_configs;
    let next = configs
        .iter()
        .position(|c| c.language == settings.selected_language)
        .map_or(0, |current| (current + 1) % configs.len().max(1));
    configs.get(next)
}

fn play_sounds(app: &AppHandle, theme: SoundTheme) {
    let app = app.clone();
    std::thread::spawn(move || {
        audio_feedback::play_theme_sound_blocking(&app, theme, SoundType::Start);
        audio_feedback::play_theme_sound_blocking(&app, theme, SoundType::Stop);
    });
}

fn apply(app: &AppHandle, config: LanguageConfig) {
    info!("Switching to language '{}'", config.language);
    let mut settings = get_settings(app);
    settings.selected_language = config.language.clone();
    if let Some(prompt_id) = config.prompt_id.filter(|id| !id.is_empty()) {
        settings.post_process_selected_prompt_id = Some(prompt_id);
    }
    settings::write_settings(app, settings);
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({ "setting": "selected_language", "value": config.language }),
    );

    if let Some(model_id) = config.model.filter(|id| !id.is_empty()) {
        switch_model(app, model_id);
    }
}

/// Downloads the model first when needed, then makes it the active model.
fn switch_model(app: &AppHandle, model_id: String) {
    if get_settings(app).selected_model == model_id {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let model_manager = app.state::<Arc<ModelManager>>().inner().clone();
        match model_manager.get_model_info(&model_id) {
            None => {
                warn!("Language model '{model_id}' not found");
                return;
            }
            Some(info) if info.is_downloading => return,
            Some(info) if !info.is_downloaded => {
                if let Err(e) = model_manager.download_model(&model_id).await {
                    warn!("Could not download language model '{model_id}': {e}");
                    return;
                }
            }
            Some(_) => {}
        }
        let switched = tauri::async_runtime::spawn_blocking(move || {
            let result = crate::commands::models::switch_active_model(&app, &model_id);
            crate::tray::update_tray_menu(&app);
            result.map_err(|e| format!("Could not switch to language model '{model_id}': {e}"))
        })
        .await;
        match switched {
            Ok(Err(e)) => warn!("{e}"),
            Err(e) => warn!("Language model switch task failed: {e}"),
            Ok(Ok(())) => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(id: &str, language: &str, keys: &str) -> LanguageConfig {
        LanguageConfig {
            id: id.to_string(),
            language: language.to_string(),
            shortcut_binding: keys.to_string(),
            prompt_id: None,
            model: None,
        }
    }

    fn settings_with(configs: Vec<LanguageConfig>, selected: &str) -> AppSettings {
        let mut settings = settings::get_default_settings();
        settings.language_configs = configs;
        settings.selected_language = selected.to_string();
        settings
    }

    #[test]
    fn cycle_moves_to_the_next_language_and_wraps() {
        let configs = vec![config("a", "en", ""), config("b", "pt", "")];
        assert_eq!(
            next_in_cycle(&settings_with(configs.clone(), "en"))
                .unwrap()
                .id,
            "b"
        );
        assert_eq!(
            next_in_cycle(&settings_with(configs.clone(), "pt"))
                .unwrap()
                .id,
            "a"
        );
        assert_eq!(
            next_in_cycle(&settings_with(configs, "de")).unwrap().id,
            "a"
        );
        assert!(next_in_cycle(&settings_with(Vec::new(), "en")).is_none());
    }

    #[test]
    fn only_languages_with_a_shortcut_are_registered() {
        let mut settings = settings_with(
            vec![config("a", "en", "ctrl+f1"), config("b", "pt", " ")],
            "en",
        );
        settings.language_cycle_shortcut = "alt_right+l".to_string();

        let bindings = shortcut_bindings(&settings);
        let ids: Vec<_> = bindings.iter().map(|b| b.id.as_str()).collect();

        assert_eq!(ids, ["language_cycle", "language:a"]);
        assert_eq!(bindings[1].current_binding, "ctrl+f1");
    }

    #[test]
    fn new_shortcut_cannot_reuse_keys_of_another_shortcut() {
        let mut settings = settings_with(
            vec![config("a", "en", "ctrl+f1"), config("b", "pt", "")],
            "en",
        );
        let transcribe = settings.bindings["transcribe"].current_binding.clone();
        let all_new = |_: &ShortcutBinding| true;

        assert_eq!(taken_shortcut(&settings, all_new), None);

        settings.language_configs[1].shortcut_binding = transcribe.to_uppercase();
        assert_eq!(
            taken_shortcut(&settings, all_new),
            Some(transcribe.to_uppercase())
        );

        settings.language_configs[1].shortcut_binding = "ctrl+f1".to_string();
        let only_b = |b: &ShortcutBinding| b.id == "language:b";
        assert_eq!(
            taken_shortcut(&settings, only_b),
            Some("ctrl+f1".to_string())
        );
    }
}
