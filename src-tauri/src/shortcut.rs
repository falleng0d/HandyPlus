use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::actions::ACTION_MAP;
use crate::hotkey::{
    is_modifier_key, key_to_name, modifier_key_to_specific_name, normalize_combo_from_parts,
    normalize_shortcut_string, validate_shortcut_string,
};
use crate::settings::ShortcutBinding;
use crate::settings::{
    self, get_settings, ClipboardHandling, LLMPrompt, OverlayPosition, PasteMethod, SoundTheme,
};
use crate::ManagedToggleState;
use rdev::{listen, Event, EventType};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use once_cell::sync::OnceCell;

static RUNTIME: OnceCell<Arc<Mutex<ShortcutRuntime>>> = OnceCell::new();

#[derive(Default, Debug)]
struct ShortcutRuntime {
    // id -> binding
    bindings_by_id: HashMap<String, ShortcutBinding>,
    // normalized_combo -> id
    combo_to_id: HashMap<String, String>,
    // currently pressed modifiers (canonical: ctrl|shift|alt|meta)
    pressed_mods: HashSet<String>,
    // track pressed non-modifier keys to avoid repeats
    pressed_keys: HashSet<String>,
    // track pressed modifier keys (specific: "left ctrl", "right shift", etc.) to avoid repeats
    pressed_modifier_keys: HashSet<String>,
}

fn get_runtime() -> &'static Arc<Mutex<ShortcutRuntime>> {
    RUNTIME.get().expect("Shortcut runtime not initialized")
}

pub fn init_shortcuts(app: &AppHandle) {
    // Initialize runtime once
    RUNTIME.get_or_init(|| Arc::new(Mutex::new(ShortcutRuntime::default())));

    // Load settings and populate runtime
    let settings = settings::load_or_create_app_settings(app);
    {
        let rt = get_runtime().clone();
        let mut rt = rt.lock().unwrap();
        rt.bindings_by_id.clear();
        rt.combo_to_id.clear();
        for (_id, binding) in settings.bindings.clone() {
            if let Ok(combo) = normalize_shortcut_string(&binding.current_binding) {
                // last writer wins for duplicates in settings
                rt.combo_to_id.insert(combo, binding.id.clone());
                rt.bindings_by_id.insert(binding.id.clone(), binding);
            }
        }
    }

    // Spawn global listener thread once
    let app_handle = app.clone();
    thread::spawn(move || {
        if let Err(e) = listen(move |event| {
            handle_rdev_event(&app_handle, event);
        }) {
            eprintln!("Global key listener failed: {:?}", e);
        }
    });
}

fn handle_rdev_event(app: &AppHandle, event: Event) {
    let rt_arc = get_runtime().clone();
    let mut rt = rt_arc.lock().unwrap();
    match event.event_type {
        EventType::KeyPress(k) => on_rdev_key_press_event(app, &mut rt, k),
        EventType::KeyRelease(k) => on_rdev_key_release_event(app, &mut rt, k),
        _ => {}
    }
}

/// Handle a key press event from rdev
fn on_rdev_key_press_event(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key) {
    if let Some(mod_name) = is_modifier_key(k) {
        on_modifier_key_press(app, rt, k, mod_name);
    } else if let Some(key_name) = key_to_name(k) {
        on_regular_key_press(app, rt, key_name);
    }
}

/// Handle a key release event from rdev
fn on_rdev_key_release_event(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key) {
    if let Some(mod_name) = is_modifier_key(k) {
        on_modifier_key_release(app, rt, k, mod_name);
    } else if let Some(key_name) = key_to_name(k) {
        on_regular_key_release(app, rt, key_name);
    }
}

/// Handle a modifier key press (e.g., Ctrl, Shift, Alt, Meta)
fn on_modifier_key_press(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key, mod_name: &str) {
    // Track this modifier as pressed
    rt.pressed_mods.insert(mod_name.to_string());

    // Check if this modifier key itself is a registered shortcut
    if let Some(specific_mod_name) = modifier_key_to_specific_name(k) {
        if rt.pressed_modifier_keys.insert(specific_mod_name.clone()) {
            // Newly pressed modifier key - check if it's registered as a shortcut
            trigger_shortcut_if_registered(app, rt, &specific_mod_name);
        }
    }
}

/// Handle a regular (non-modifier) key press
fn on_regular_key_press(app: &AppHandle, rt: &mut ShortcutRuntime, key_name: String) {
    if rt.pressed_keys.insert(key_name.clone()) {
        // Newly pressed non-modifier key
        let mut mods: Vec<String> = rt.pressed_mods.iter().cloned().collect();
        let combo = normalize_combo_from_parts(&mut mods, &key_name);
        trigger_shortcut_if_registered(app, rt, &combo);
    }
}

/// Handle a modifier key release (e.g., Ctrl, Shift, Alt, Meta)
fn on_modifier_key_release(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key, mod_name: &str) {
    rt.pressed_mods.remove(mod_name);

    // Check if this modifier key was registered as a shortcut
    if let Some(specific_mod_name) = modifier_key_to_specific_name(k) {
        if rt.pressed_modifier_keys.remove(&specific_mod_name) {
            // Was pressed; handle PTT stop for modifier shortcuts
            stop_shortcut_if_registered_ptt(app, rt, &specific_mod_name);
        }
    }
}

/// Handle a regular (non-modifier) key release
fn on_regular_key_release(app: &AppHandle, rt: &mut ShortcutRuntime, key_name: String) {
    if rt.pressed_keys.remove(&key_name) {
        // Was pressed; handle PTT stop
        let mut mods: Vec<String> = rt.pressed_mods.iter().cloned().collect();
        let combo = normalize_combo_from_parts(&mut mods, &key_name);
        stop_shortcut_if_registered_ptt(app, rt, &combo);
    }
}

/// Trigger a shortcut action if it's registered (handles both PTT and toggle modes)
fn trigger_shortcut_if_registered(app: &AppHandle, rt: &ShortcutRuntime, combo: &str) {
    if let Some(binding_id) = rt.combo_to_id.get(combo).cloned() {
        let shortcut_string = combo.to_string();
        let settings = get_settings(app);
        if let Some(action) = ACTION_MAP.get(&binding_id) {
            if settings.push_to_talk {
                action.start(app, &binding_id, &shortcut_string);
            } else {
                // toggle behavior
                let toggle_state_manager = app.state::<ManagedToggleState>();
                let mut states = toggle_state_manager.lock().expect("Failed to lock toggle state manager");
                let is_active = states.active_toggles.entry(binding_id.clone()).or_insert(false);
                if *is_active {
                    action.stop(app, &binding_id, &shortcut_string);
                    *is_active = false;
                } else {
                    action.start(app, &binding_id, &shortcut_string);
                    *is_active = true;
                }
            }
        }
    }
}

/// Stop a shortcut action if it's registered and in PTT mode
fn stop_shortcut_if_registered_ptt(app: &AppHandle, rt: &ShortcutRuntime, combo: &str) {
    if let Some(binding_id) = rt.combo_to_id.get(combo).cloned() {
        let shortcut_string = combo.to_string();
        let settings = get_settings(app);
        if settings.push_to_talk {
            if let Some(action) = ACTION_MAP.get(&binding_id) {
                action.stop(app, &binding_id, &shortcut_string);
            }
        }
    }
}

#[derive(Serialize)]
pub struct BindingResponse {
    success: bool,
    binding: Option<ShortcutBinding>,
    error: Option<String>,
}

#[tauri::command]
pub fn change_binding(
    app: AppHandle,
    id: String,
    binding: String,
) -> Result<BindingResponse, String> {
    let mut settings = get_settings(&app);

    // Get the binding to modify
    let binding_to_modify = match settings.bindings.get(&id) {
        Some(binding) => binding.clone(),
        None => {
            let error_msg = format!("Binding with id '{}' not found", id);
            eprintln!("change_binding error: {}", error_msg);
            return Ok(BindingResponse {
                success: false,
                binding: None,
                error: Some(error_msg),
            });
        }
    };

    // Unregister the existing binding
    if let Err(e) = _unregister_shortcut(&app, binding_to_modify.clone()) {
        let error_msg = format!("Failed to unregister shortcut: {}", e);
        eprintln!("change_binding error: {}", error_msg);
    }

    // Validate the new shortcut before we touch the current registration
    if let Err(e) = validate_shortcut_string(&binding) {
        eprintln!("change_binding validation error: {}", e);
        return Err(e);
    }

    // Create an updated binding
    let mut updated_binding = binding_to_modify;
    updated_binding.current_binding = binding;

    // Register the new binding
    if let Err(e) = _register_shortcut(&app, updated_binding.clone()) {
        let error_msg = format!("Failed to register shortcut: {}", e);
        eprintln!("change_binding error: {}", error_msg);
        return Ok(BindingResponse {
            success: false,
            binding: None,
            error: Some(error_msg),
        });
    }

    // Update the binding in the settings
    settings.bindings.insert(id, updated_binding.clone());

    // Save the settings
    settings::write_settings(&app, settings);

    // Return the updated binding
    Ok(BindingResponse {
        success: true,
        binding: Some(updated_binding),
        error: None,
    })
}

#[tauri::command]
pub fn reset_binding(app: AppHandle, id: String) -> Result<BindingResponse, String> {
    let binding = settings::get_stored_binding(&app, &id);

    change_binding(app, id, binding.default_binding)
}

#[tauri::command]
pub fn change_ptt_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // TODO if the setting is currently false, we probably want to
    // cancel any ongoing recordings or actions
    settings.push_to_talk = enabled;

    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
pub fn change_audio_feedback_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.audio_feedback = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_audio_feedback_volume_setting(app: AppHandle, volume: f32) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.audio_feedback_volume = volume;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_sound_theme_setting(app: AppHandle, theme: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let parsed = match theme.as_str() {
        "marimba" => SoundTheme::Marimba,
        "pop" => SoundTheme::Pop,
        "custom" => SoundTheme::Custom,
        other => {
            eprintln!("Invalid sound theme '{}', defaulting to marimba", other);
            SoundTheme::Marimba
        }
    };
    settings.sound_theme = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_translate_to_english_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.translate_to_english = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_selected_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.selected_language = language;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_overlay_position_setting(app: AppHandle, position: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let parsed = match position.as_str() {
        "none" => OverlayPosition::None,
        "top" => OverlayPosition::Top,
        "bottom" => OverlayPosition::Bottom,
        other => {
            eprintln!("Invalid overlay position '{}', defaulting to bottom", other);
            OverlayPosition::Bottom
        }
    };
    settings.overlay_position = parsed;
    settings::write_settings(&app, settings);

    // Update overlay position without recreating window
    crate::utils::update_overlay_position(&app);

    Ok(())
}

#[tauri::command]
pub fn change_debug_mode_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.debug_mode = enabled;
    settings::write_settings(&app, settings);

    // Emit event to notify frontend of debug mode change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "debug_mode",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn change_start_hidden_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.start_hidden = enabled;
    settings::write_settings(&app, settings);

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "start_hidden",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn change_autostart_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.autostart_enabled = enabled;
    settings::write_settings(&app, settings);

    // Apply the autostart setting immediately
    let autostart_manager = app.autolaunch();
    if enabled {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "autostart_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn update_custom_words(app: AppHandle, words: Vec<String>) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.custom_words = words;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_word_correction_threshold_setting(
    app: AppHandle,
    threshold: f64,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.word_correction_threshold = threshold;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_paste_method_setting(app: AppHandle, method: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let parsed = match method.as_str() {
        "ctrl_v" => PasteMethod::CtrlV,
        "direct" => PasteMethod::Direct,
        #[cfg(not(target_os = "macos"))]
        "shift_insert" => PasteMethod::ShiftInsert,
        other => {
            eprintln!("Invalid paste method '{}', defaulting to ctrl_v", other);
            PasteMethod::CtrlV
        }
    };
    settings.paste_method = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_clipboard_handling_setting(app: AppHandle, handling: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let parsed = match handling.as_str() {
        "dont_modify" => ClipboardHandling::DontModify,
        "copy_to_clipboard" => ClipboardHandling::CopyToClipboard,
        other => {
            eprintln!(
                "Invalid clipboard handling '{}', defaulting to dont_modify",
                other
            );
            ClipboardHandling::DontModify
        }
    };
    settings.clipboard_handling = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_post_process_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.post_process_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_post_process_base_url_setting(
    app: AppHandle,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let label = settings
        .post_process_provider(&provider_id)
        .map(|provider| provider.label.clone())
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let provider = settings
        .post_process_provider_mut(&provider_id)
        .expect("Provider looked up above must exist");

    if !provider.allow_base_url_edit {
        return Err(format!(
            "Provider '{}' does not allow editing the base URL",
            label
        ));
    }

    provider.base_url = base_url;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_lower_volume_while_recording_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.lower_volume_while_recording = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_volume_while_recording_setting(app: AppHandle, volume: f32) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.volume_while_recording = volume;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_on_recording_start_script_setting(app: AppHandle, script: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.on_recording_start_script = script;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_on_recording_end_script_setting(app: AppHandle, script: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.on_recording_end_script = script;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Generic helper to validate provider exists
fn validate_provider_exists(
    settings: &settings::AppSettings,
    provider_id: &str,
) -> Result<(), String> {
    if !settings
        .post_process_providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("Provider '{}' not found", provider_id));
    }
    Ok(())
}

#[tauri::command]
pub fn change_post_process_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_api_keys.insert(provider_id, api_key);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_post_process_model_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_models.insert(provider_id, model);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn set_post_process_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_provider_id = provider_id;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn add_post_process_prompt(
    app: AppHandle,
    name: String,
    prompt: String,
) -> Result<LLMPrompt, String> {
    let mut settings = get_settings(&app);

    // Generate unique ID using timestamp and random component
    let id = format!("prompt_{}", chrono::Utc::now().timestamp_millis());

    let new_prompt = LLMPrompt {
        id: id.clone(),
        name,
        prompt,
    };

    settings.post_process_prompts.push(new_prompt.clone());
    settings::write_settings(&app, settings);

    Ok(new_prompt)
}

#[tauri::command]
pub fn update_post_process_prompt(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);

    if let Some(existing_prompt) = settings
        .post_process_prompts
        .iter_mut()
        .find(|p| p.id == id)
    {
        existing_prompt.name = name;
        existing_prompt.prompt = prompt;
        settings::write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Prompt with id '{}' not found", id))
    }
}

#[tauri::command]
pub fn delete_post_process_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // Don't allow deleting the last prompt
    if settings.post_process_prompts.len() <= 1 {
        return Err("Cannot delete the last prompt".to_string());
    }

    // Find and remove the prompt
    let original_len = settings.post_process_prompts.len();
    settings.post_process_prompts.retain(|p| p.id != id);

    if settings.post_process_prompts.len() == original_len {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    // If the deleted prompt was selected, select the first one or None
    if settings.post_process_selected_prompt_id.as_ref() == Some(&id) {
        settings.post_process_selected_prompt_id =
            settings.post_process_prompts.first().map(|p| p.id.clone());
    }

    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub async fn fetch_post_process_models(
    app: AppHandle,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let settings = get_settings(&app);

    // Find the provider
    let provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    // Get API key
    let api_key = settings
        .post_process_api_keys
        .get(&provider_id)
        .cloned()
        .unwrap_or_default();

    // Skip fetching if no API key for providers that typically need one
    if api_key.trim().is_empty() && provider.id != "custom" {
        return Err(format!(
            "API key is required for {}. Please add an API key to list available models.",
            provider.label
        ));
    }

    // TODO: In the future, we can use async-openai's models API:
    // let client = crate::llm_client::create_client(provider, api_key)?;
    // let response = client.models().list().await?;
    // return Ok(response.data.iter().map(|m| m.id.clone()).collect());

    // For now, use manual HTTP request to have more control over the endpoint
    fetch_models_manual(provider, api_key).await
}

/// Fetch models using manual HTTP request
/// This gives us more control and avoids issues with non-standard endpoints
async fn fetch_models_manual(
    provider: &crate::settings::PostProcessProvider,
    api_key: String,
) -> Result<Vec<String>, String> {
    // Build the endpoint URL
    let base_url = provider.base_url.trim_end_matches('/');
    let models_endpoint = provider
        .models_endpoint
        .as_ref()
        .map(|s| s.trim_start_matches('/'))
        .unwrap_or("models");
    let endpoint = format!("{}/{}", base_url, models_endpoint);

    // Create HTTP client with headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "HTTP-Referer",
        reqwest::header::HeaderValue::from_static("https://github.com/cjpais/Handy"),
    );
    headers.insert(
        "X-Title",
        reqwest::header::HeaderValue::from_static("Handy"),
    );

    // Add provider-specific headers
    if provider.id == "anthropic" {
        if !api_key.is_empty() {
            headers.insert(
                "x-api-key",
                reqwest::header::HeaderValue::from_str(&api_key)
                    .map_err(|e| format!("Invalid API key: {}", e))?,
            );
        }
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static("2023-06-01"),
        );
    } else if !api_key.is_empty() {
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| format!("Invalid API key: {}", e))?,
        );
    }

    let http_client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Make the request
    let response = http_client
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Model list request failed ({}): {}",
            status, error_text
        ));
    }

    // Parse the response
    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut models = Vec::new();

    // Handle OpenAI format: { data: [ { id: "..." }, ... ] }
    if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
        for entry in data {
            if let Some(id) = entry.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            } else if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                models.push(name.to_string());
            }
        }
    }
    // Handle array format: [ "model1", "model2", ... ]
    else if let Some(array) = parsed.as_array() {
        for entry in array {
            if let Some(model) = entry.as_str() {
                models.push(model.to_string());
            }
        }
    }

    Ok(models)
}

#[tauri::command]
pub fn set_post_process_selected_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // Verify the prompt exists
    if !settings.post_process_prompts.iter().any(|p| p.id == id) {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    settings.post_process_selected_prompt_id = Some(id);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_mute_while_recording_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.mute_while_recording = enabled;
    settings::write_settings(&app, settings);

    Ok(())
}



/// Temporarily unregister a binding while the user is editing it in the UI.
/// This avoids firing the action while keys are being recorded.
#[tauri::command]
pub fn suspend_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        if let Err(e) = _unregister_shortcut(&app, b) {
            eprintln!("suspend_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

/// Re-register the binding after the user has finished editing.
#[tauri::command]
pub fn resume_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        if let Err(e) = _register_shortcut(&app, b) {
            eprintln!("resume_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

fn _register_shortcut(_app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    // Validate human-level rules first
    if let Err(e) = validate_shortcut_string(&binding.current_binding) {
        eprintln!(
            "_register_shortcut validation error for binding '{}': {}",
            binding.current_binding, e
        );
        return Err(e);
    }

    // Normalize combo
    let combo = normalize_shortcut_string(&binding.current_binding)?;

    // Update runtime maps
    let rt_arc = get_runtime().clone();
    let mut rt = rt_arc.lock().unwrap();

    // Check duplicates: same combo already mapped to a different id
    if let Some(existing_id) = rt.combo_to_id.get(&combo) {
        if existing_id != &binding.id {
            let error_msg = format!("Shortcut '{}' is already in use", binding.current_binding);
            eprintln!("_register_shortcut duplicate error: {}", error_msg);
            return Err(error_msg);
        }
    }

    rt.combo_to_id.insert(combo, binding.id.clone());
    rt.bindings_by_id.insert(binding.id.clone(), binding);

    Ok(())
}

fn _unregister_shortcut(_app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let combo = normalize_shortcut_string(&binding.current_binding)?;
    let rt_arc = get_runtime().clone();
    let mut rt = rt_arc.lock().unwrap();

    // Remove combo mapping if it points to this id
    if let Some(existing_id) = rt.combo_to_id.get(&combo) {
        if existing_id == &binding.id {
            rt.combo_to_id.remove(&combo);
        }
    }

    // Remove binding by id
    rt.bindings_by_id.remove(&binding.id);

    Ok(())
}
