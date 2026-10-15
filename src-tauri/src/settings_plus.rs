//! Handy+ settings migrations. They run from `apply_settings_migrations` and
//! track their own version so upstream's schema numbering stays independent.

use crate::settings::{AppSettings, TranscribeAcceleratorSetting};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

pub const CURRENT_PLUS_SETTINGS_VERSION: u32 = 1;

pub const DEFAULT_VAD_THRESHOLD: f32 = 0.3;
pub const DEFAULT_CONTEXT_LINES: u32 = 30;
pub const DEFAULT_TRANSCRIPTION_TIMEOUT_SECS: u32 = 120;
pub const DEFAULT_POST_PROCESS_TIMEOUT_SECS: u32 = 15;
pub const DEFAULT_VOLUME_WHILE_RECORDING: f32 = 0.3;

/// Silero frame length used by the rdev-era builds, which stored VAD timings
/// as frame counts.
const RDEV_ERA_VAD_FRAME_MS: u64 = 30;

/// A language the user can switch to with a shortcut. `prompt_id` and `model`
/// are applied with it when set.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
pub struct LanguageConfig {
    pub id: String,
    pub language: String,
    #[serde(default)]
    pub shortcut_binding: String,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

pub fn default_plus_settings_version() -> u32 {
    CURRENT_PLUS_SETTINGS_VERSION
}

pub fn apply_plus_migrations(settings: &mut AppSettings, stored: &Value) -> bool {
    let stored_version = stored
        .get("plus_settings_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if stored_version >= u64::from(CURRENT_PLUS_SETTINGS_VERSION) {
        return false;
    }

    if stored_version < 1 {
        migrate_rdev_era_store(settings, stored);
    }

    settings.plus_settings_version = CURRENT_PLUS_SETTINGS_VERSION;
    true
}

/// Converts a store written by the rdev-based Handy+ builds (0.8.x).
fn migrate_rdev_era_store(settings: &mut AppSettings, stored: &Value) {
    for binding in settings.bindings.values_mut() {
        binding.current_binding = rdev_shortcut_to_handy_keys(&binding.current_binding);
        binding.default_binding = rdev_shortcut_to_handy_keys(&binding.default_binding);
    }
    for config in &mut settings.language_configs {
        config.shortcut_binding = rdev_shortcut_to_handy_keys(&config.shortcut_binding);
    }
    settings.language_cycle_shortcut =
        rdev_shortcut_to_handy_keys(&settings.language_cycle_shortcut);
    move_shortcut_to_post_processing(settings, stored);

    for (frames_key, ms) in [
        ("vad_prefill_frames", &mut settings.vad_prefill_ms),
        ("vad_hangover_frames", &mut settings.vad_hangover_ms),
        ("vad_onset_frames", &mut settings.vad_onset_ms),
    ] {
        if let Some(frames) = stored.get(frames_key).and_then(Value::as_u64) {
            *ms = frames * RDEV_ERA_VAD_FRAME_MS;
        }
    }

    if stored.get("transcribe_accelerator").is_none()
        && stored.get("whisper_accelerator").and_then(Value::as_str) == Some("cpu")
    {
        settings.transcribe_accelerator = TranscribeAcceleratorSetting::Cpu;
    }
}

/// The rdev-era transcribe shortcut post-processed whenever post-processing was
/// on. Upstream gives post-processing its own shortcut instead, so the user's
/// shortcut moves there and plain transcription gets its default shortcut back.
fn move_shortcut_to_post_processing(settings: &mut AppSettings, stored: &Value) {
    let post_processing_on =
        stored.get("post_process_enabled").and_then(Value::as_bool) == Some(true);
    if !post_processing_on
        || settings
            .bindings
            .contains_key("transcribe_with_post_process")
    {
        return;
    }
    let defaults = crate::settings::get_default_settings().bindings;
    let (Some(transcribe), Some(default_transcribe), Some(post_process)) = (
        settings.bindings.get_mut("transcribe"),
        defaults.get("transcribe"),
        defaults.get("transcribe_with_post_process"),
    ) else {
        return;
    };
    let mut post_process = post_process.clone();
    post_process.current_binding = std::mem::replace(
        &mut transcribe.current_binding,
        default_transcribe.current_binding.clone(),
    );
    transcribe.default_binding = default_transcribe.default_binding.clone();
    settings
        .bindings
        .insert(post_process.id.clone(), post_process);
}

/// Rewrites an rdev-era shortcut ("right alt", "left ctrl+f1", "ctrl+delete")
/// into the names handy-keys parses ("alt_right", "ctrl_left+f1",
/// "ctrl+forwarddelete"). Strings already in handy-keys form pass through.
pub fn rdev_shortcut_to_handy_keys(shortcut: &str) -> String {
    if shortcut.trim().is_empty() {
        return String::new();
    }
    shortcut
        .split('+')
        .map(|part| rdev_key_to_handy_keys(&part.trim().to_lowercase()))
        .collect::<Vec<_>>()
        .join("+")
}

fn rdev_key_to_handy_keys(name: &str) -> String {
    if let Some((side, modifier)) = name.split_once(' ') {
        if let (Some(side), Some(modifier)) = (side_suffix(side), modifier_name(modifier)) {
            return format!("{modifier}_{side}");
        }
    }
    match name {
        // rdev's Delete is the forward-delete key; handy-keys uses "delete"
        // for Backspace.
        "delete" => "forwarddelete".to_string(),
        _ => modifier_name(name).unwrap_or(name).to_string(),
    }
}

fn side_suffix(side: &str) -> Option<&'static str> {
    match side {
        "left" => Some("left"),
        "right" => Some("right"),
        _ => None,
    }
}

fn modifier_name(name: &str) -> Option<&'static str> {
    match name {
        "ctrl" | "control" => Some("ctrl"),
        "shift" => Some("shift"),
        "alt" | "option" => Some("alt"),
        "meta" | "command" | "cmd" | "super" | "win" | "windows" => Some("super"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{apply_settings_migrations, ShortcutActivation};

    /// Snapshot of the maintainer's Handy+ 0.8.3 store, with secrets and
    /// prompt text replaced. It stands in for the real store: extend it, don't
    /// rewrite it.
    fn rdev_era_store() -> Value {
        serde_json::from_str(
            r##"{
            "always_on_microphone": false,
            "audio_feedback": false,
            "audio_feedback_volume": 0.10000000149011612,
            "autostart_enabled": true,
            "bindings": {
                "transcribe": {
                    "current_binding": "right alt",
                    "default_binding": "ctrl+space",
                    "description": "Converts your speech into text.",
                    "id": "transcribe",
                    "name": "Transcribe"
                }
            },
            "clipboard_handling": "dont_modify",
            "custom_word_correction_enabled": true,
            "custom_words": ["Handy", "Tauri", "Parakeet"],
            "debug_mode": true,
            "history_limit": 5,
            "language_configs": [
                {"id": "lang_1772145087415_gk2c6", "language": "en", "model": "parakeet-tdt-0.6b-v2", "prompt_id": "prompt_1763226259143", "shortcut_binding": ""},
                {"id": "lang_1772221481058_t2ztm", "language": "pt", "model": "handy-computer/whisper-large-v3-turbo-gguf/whisper-large-v3-turbo-Q8_0.gguf", "prompt_id": "prompt_1772221696122", "shortcut_binding": ""}
            ],
            "language_cycle_shortcut": "",
            "language_shortcut_sound_theme": "pop",
            "lower_volume_while_recording": true,
            "model_unload_timeout": "hour1",
            "mute_while_recording": false,
            "on_recording_end_script": "",
            "on_recording_start_script": "",
            "ort_accelerator": "auto",
            "overlay_position": "bottom",
            "paste_method": "direct",
            "pause_dragon_when_dictating": false,
            "post_process_api_keys": {"anthropic": "", "custom": "", "openai": "", "openrouter": ""},
            "post_process_enabled": true,
            "post_process_models": {"anthropic": "", "custom": "@openrouter/@preset/glm-5-3-speed", "openai": "", "openrouter": ""},
            "post_process_prompts": [
                {"id": "default_improve_transcriptions", "name": "Improve Transcriptions", "prompt": "Clean up: ${output}"},
                {"id": "prompt_1763226259143", "name": "Custom", "prompt": "Words: ${dictionary}\n${output}"},
                {"id": "prompt_1764512876451", "name": "Serious", "prompt": "${output}"},
                {"id": "prompt_1772221696122", "name": "PT-BR", "prompt": "${output}"}
            ],
            "post_process_provider_id": "custom",
            "post_process_providers": [
                {"allow_base_url_edit": false, "base_url": "https://api.openai.com/v1", "id": "openai", "label": "OpenAI", "models_endpoint": "/models", "supports_structured_output": true},
                {"allow_base_url_edit": true, "base_url": "https://api.portkey.ai/v1", "id": "custom", "label": "Custom", "models_endpoint": "/models", "supports_structured_output": false}
            ],
            "post_process_selected_prompt_id": "prompt_1763226259143",
            "push_to_talk": true,
            "selected_language": "en",
            "selected_microphone": "Microphone (TONOR TC30 Audio Device)",
            "selected_model": "handy-computer/cohere-transcribe-03-2026-gguf/cohere-transcribe-03-2026-Q5_K_M.gguf",
            "selected_output_device": "Speakers (Realtek(R) Audio)",
            "sound_theme": "marimba",
            "start_hidden": true,
            "translate_to_english": false,
            "typing_interval_ms": 10,
            "vad_hangover_frames": 15,
            "vad_onset_frames": 2,
            "vad_prefill_frames": 15,
            "vad_threshold": 0.6000000238418579,
            "volume_while_recording": 0.05999999865889549,
            "whisper_accelerator": "auto",
            "whisper_gpu_device": -1,
            "word_correction_threshold": 0.0
        }"##,
        )
        .expect("fixture is valid JSON")
    }

    fn load(stored: &Value) -> AppSettings {
        let mut settings: AppSettings = serde_json::from_value(stored.clone())
            .expect("a Handy+ 0.8.3 store must parse without salvage");
        assert!(apply_settings_migrations(&mut settings, stored));
        settings
    }

    #[test]
    fn rdev_era_store_keeps_the_users_setup() {
        let settings = load(&rdev_era_store());

        assert_eq!(
            settings.bindings["transcribe_with_post_process"].current_binding,
            "alt_right"
        );
        assert_eq!(
            settings.bindings["transcribe"].current_binding,
            crate::settings::get_default_settings().bindings["transcribe"].current_binding
        );
        assert_eq!(settings.shortcut_activation, ShortcutActivation::PushToTalk);
        assert_eq!(
            settings.selected_model,
            "handy-computer/cohere-transcribe-03-2026-gguf/cohere-transcribe-03-2026-Q5_K_M.gguf"
        );
        assert_eq!(settings.post_process_provider_id, "custom");
        assert_eq!(
            settings.post_process_selected_prompt_id.as_deref(),
            Some("prompt_1763226259143")
        );
        assert_eq!(settings.post_process_prompts.len(), 4);
        assert_eq!(settings.custom_words.len(), 3);
        assert_eq!(settings.history_limit, 5);
        assert_eq!(settings.typing_interval_ms, 10);
        assert!(settings.custom_word_correction_enabled);
        assert!(settings.lower_volume_while_recording);
        assert!((settings.volume_while_recording - 0.06).abs() < 1e-6);
        assert_eq!(settings.language_configs.len(), 2);
        assert_eq!(settings.language_configs[1].language, "pt");
        assert_eq!(
            settings.language_configs[1].prompt_id.as_deref(),
            Some("prompt_1772221696122")
        );
        assert_eq!(
            settings.language_shortcut_sound_theme,
            crate::settings::SoundTheme::Pop
        );
        assert_eq!(
            settings.transcribe_accelerator,
            TranscribeAcceleratorSetting::Auto
        );
        assert_eq!(settings.transcribe_gpu_device, None);
        assert_eq!(
            settings.plus_settings_version,
            CURRENT_PLUS_SETTINGS_VERSION
        );
    }

    #[test]
    fn rdev_era_language_shortcuts_use_handy_keys_names() {
        let mut stored = rdev_era_store();
        stored["language_cycle_shortcut"] = "right ctrl+l".into();
        stored["language_configs"][0]["shortcut_binding"] = "left alt+f9".into();

        let settings = load(&stored);

        assert_eq!(settings.language_cycle_shortcut, "ctrl_right+l");
        assert_eq!(settings.language_configs[0].shortcut_binding, "alt_left+f9");
        assert_eq!(settings.language_configs[1].shortcut_binding, "");
    }

    #[test]
    fn rdev_era_vad_frames_become_milliseconds() {
        let mut stored = rdev_era_store();
        stored["vad_prefill_frames"] = 10.into();
        stored["vad_hangover_frames"] = 20.into();
        stored["vad_onset_frames"] = 3.into();

        let settings = load(&stored);

        assert!((settings.vad_threshold - 0.6).abs() < 1e-6);
        assert_eq!(settings.vad_prefill_ms, 300);
        assert_eq!(settings.vad_hangover_ms, 600);
        assert_eq!(settings.vad_onset_ms, 90);
    }

    #[test]
    fn shortcut_stays_on_plain_transcription_when_post_processing_was_off() {
        let mut stored = rdev_era_store();
        stored["post_process_enabled"] = false.into();

        let settings = load(&stored);

        assert_eq!(settings.bindings["transcribe"].current_binding, "alt_right");
        assert!(!settings
            .bindings
            .contains_key("transcribe_with_post_process"));
    }

    #[test]
    fn rdev_era_cpu_accelerator_stays_on_cpu() {
        let mut stored = rdev_era_store();
        stored["whisper_accelerator"] = "cpu".into();

        let settings = load(&stored);

        assert_eq!(
            settings.transcribe_accelerator,
            TranscribeAcceleratorSetting::Cpu
        );
    }

    #[test]
    fn migrated_store_is_not_migrated_again() {
        let settings = load(&rdev_era_store());
        let stored = serde_json::to_value(&settings).unwrap();
        let mut reloaded: AppSettings = serde_json::from_value(stored.clone()).unwrap();

        assert!(!apply_plus_migrations(&mut reloaded, &stored));
        assert_eq!(
            reloaded.bindings["transcribe_with_post_process"].current_binding,
            "alt_right"
        );
    }

    #[test]
    fn rdev_shortcuts_become_handy_keys_names() {
        for (rdev, handy_keys) in [
            ("right alt", "alt_right"),
            ("left ctrl", "ctrl_left"),
            ("right shift", "shift_right"),
            ("left meta", "super_left"),
            ("ctrl+space", "ctrl+space"),
            ("alt+shift+a", "alt+shift+a"),
            ("left ctrl+f1", "ctrl_left+f1"),
            ("meta+delete", "super+forwarddelete"),
            ("ctrl+backspace", "ctrl+backspace"),
            ("alt_right", "alt_right"),
            ("", ""),
        ] {
            assert_eq!(rdev_shortcut_to_handy_keys(rdev), handy_keys, "{rdev}");
        }
    }

    #[test]
    fn migrated_shortcuts_parse_in_handy_keys() {
        for rdev in [
            "right alt",
            "left ctrl+f1",
            "meta+delete",
            "ctrl+shift+space",
        ] {
            let converted = rdev_shortcut_to_handy_keys(rdev);
            assert!(
                converted.parse::<handy_keys::Hotkey>().is_ok(),
                "{rdev} -> {converted}"
            );
        }
    }
}
