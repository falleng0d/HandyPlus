use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::actions::ACTION_MAP;
use crate::hotkey::{
    is_modifier_key, key_to_name, modifier_key_to_specific_name, normalize_combo_from_parts,
    normalize_shortcut_string, validate_shortcut_string,
};
use crate::managers::model::ModelManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::ShortcutBinding;
use crate::settings::{self, get_settings, LanguageConfig};
use crate::ManagedToggleState;
use once_cell::sync::OnceCell;
use rdev::{grab, Event, EventType};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;

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

        // Load language cycle shortcut
        if !settings.language_cycle_shortcut.is_empty() {
            if let Ok(combo) = normalize_shortcut_string(&settings.language_cycle_shortcut) {
                rt.combo_to_id.insert(combo, "language_cycle".to_string());
            }
        }

        // Load per-language shortcuts
        for config in &settings.language_configs {
            if !config.shortcut_binding.is_empty() {
                if let Ok(combo) = normalize_shortcut_string(&config.shortcut_binding) {
                    let binding_id = format!("language:{}", config.id);
                    rt.combo_to_id.insert(combo, binding_id);
                }
            }
        }
    }

    // Spawn global listener thread once
    let app_handle = app.clone();
    thread::spawn(move || {
        if let Err(e) = grab(move |event| handle_rdev_event(&app_handle, event)) {
            eprintln!("Global key listener failed: {:?}", e);
        }
    });
}

fn handle_rdev_event(app: &AppHandle, event: Event) -> Option<Event> {
    let rt_arc = get_runtime().clone();
    let mut rt = rt_arc.lock().unwrap();
    let handled = match event.event_type {
        EventType::KeyPress(k) => on_rdev_key_press_event(app, &mut rt, k),
        EventType::KeyRelease(k) => on_rdev_key_release_event(app, &mut rt, k),
        _ => false,
    };

    if handled {
        None
    } else {
        Some(event)
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

/// Handle a key press event from rdev
fn on_rdev_key_press_event(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key) -> bool {
    if let Some(mod_name) = is_modifier_key(k) {
        on_modifier_key_press(app, rt, k, mod_name)
    } else if let Some(key_name) = key_to_name(k) {
        on_regular_key_press(app, rt, key_name)
    } else {
        false
    }
}

/// Handle a key release event from rdev
fn on_rdev_key_release_event(app: &AppHandle, rt: &mut ShortcutRuntime, k: rdev::Key) -> bool {
    if let Some(mod_name) = is_modifier_key(k) {
        on_modifier_key_release(app, rt, k, mod_name)
    } else if let Some(key_name) = key_to_name(k) {
        on_regular_key_release(app, rt, key_name)
    } else {
        false
    }
}

/// Handle a modifier key press (e.g., Ctrl, Shift, Alt, Meta)
fn on_modifier_key_press(
    app: &AppHandle,
    rt: &mut ShortcutRuntime,
    k: rdev::Key,
    mod_name: &str,
) -> bool {
    // Track this modifier as pressed
    rt.pressed_mods.insert(mod_name.to_string());

    // Check if this modifier key itself is a registered shortcut
    if let Some(specific_mod_name) = modifier_key_to_specific_name(k) {
        let is_registered = rt.combo_to_id.contains_key(&specific_mod_name);
        if rt.pressed_modifier_keys.insert(specific_mod_name.clone()) {
            // Newly pressed modifier key - check if it's registered as a shortcut
            if is_registered {
                trigger_shortcut_if_registered(app, rt, &specific_mod_name);
            }
        }
        return is_registered;
    }
    false
}

/// Handle a regular (non-modifier) key press
fn on_regular_key_press(app: &AppHandle, rt: &mut ShortcutRuntime, key_name: String) -> bool {
    let is_new_press = rt.pressed_keys.insert(key_name.clone());

    let mut mods: Vec<String> = rt.pressed_mods.iter().cloned().collect();
    let combo = normalize_combo_from_parts(&mut mods, &key_name);
    let is_registered = rt.combo_to_id.contains_key(&combo);

    if is_new_press {
        if is_registered {
            trigger_shortcut_if_registered(app, rt, &combo);
        }
    }
    is_registered
}

/// Handle a modifier key release (e.g., Ctrl, Shift, Alt, Meta)
fn on_modifier_key_release(
    app: &AppHandle,
    rt: &mut ShortcutRuntime,
    k: rdev::Key,
    mod_name: &str,
) -> bool {
    rt.pressed_mods.remove(mod_name);

    // Check if this modifier key was registered as a shortcut
    if let Some(specific_mod_name) = modifier_key_to_specific_name(k) {
        let is_registered = rt.combo_to_id.contains_key(&specific_mod_name);
        if rt.pressed_modifier_keys.remove(&specific_mod_name) {
            // Was pressed; handle PTT stop for modifier shortcuts
            if is_registered {
                stop_shortcut_if_registered_ptt(app, rt, &specific_mod_name);
            }
        }
        return is_registered;
    }
    false
}

/// Handle a regular (non-modifier) key release
fn on_regular_key_release(app: &AppHandle, rt: &mut ShortcutRuntime, key_name: String) -> bool {
    let was_pressed = rt.pressed_keys.remove(&key_name);

    let mut mods: Vec<String> = rt.pressed_mods.iter().cloned().collect();
    let combo = normalize_combo_from_parts(&mut mods, &key_name);
    let is_registered = rt.combo_to_id.contains_key(&combo);

    if was_pressed {
        if is_registered {
            stop_shortcut_if_registered_ptt(app, rt, &combo);
        }
    }
    is_registered
}

/// Trigger a shortcut action if it's registered (handles both PTT and toggle modes)
fn trigger_shortcut_if_registered(app: &AppHandle, rt: &ShortcutRuntime, combo: &str) {
    if let Some(binding_id) = rt.combo_to_id.get(combo).cloned() {
        let shortcut_string = combo.to_string();
        let settings = get_settings(app);

        // Handle language cycle shortcut (instant toggle, no PTT semantics)
        if binding_id == "language_cycle" {
            apply_language_cycle(app, &settings);
            return;
        }

        // Handle per-language shortcuts (instant toggle, no PTT semantics)
        if let Some(config_id) = binding_id.strip_prefix("language:") {
            if let Some(config) = settings
                .language_configs
                .iter()
                .find(|c| c.id == config_id)
                .cloned()
            {
                apply_language_config(app, config);
            }
            return;
        }

        if let Some(action) = ACTION_MAP.get(&binding_id) {
            if settings.push_to_talk {
                action.start(app, &binding_id, &shortcut_string);
            } else {
                // toggle behavior
                let toggle_state_manager = app.state::<ManagedToggleState>();
                let mut states = toggle_state_manager
                    .lock()
                    .expect("Failed to lock toggle state manager");
                let is_active = states
                    .active_toggles
                    .entry(binding_id.clone())
                    .or_insert(false);
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
        // Language shortcuts are instant-fire; skip PTT stop
        if binding_id == "language_cycle" || binding_id.starts_with("language:") {
            return;
        }

        let shortcut_string = combo.to_string();
        let settings = get_settings(app);
        if settings.push_to_talk {
            if let Some(action) = ACTION_MAP.get(&binding_id) {
                action.stop(app, &binding_id, &shortcut_string);
            }
        }
    }
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

// ── Language action helpers ──────────────────────────────────────────────────

/// Apply the next language in the cycle, wrapping around at the end.
fn apply_language_cycle(app: &AppHandle, settings: &settings::AppSettings) {
    let configs = &settings.language_configs;
    if configs.is_empty() {
        return;
    }

    let current_lang = &settings.selected_language;
    let current_idx = configs.iter().position(|c| c.language == *current_lang);

    let next_idx = match current_idx {
        Some(idx) => (idx + 1) % configs.len(),
        None => 0,
    };

    let next_config = configs[next_idx].clone();
    apply_language_config(app, next_config);
}

/// Apply a specific language configuration (language + optional prompt + optional dictation model override).
fn apply_language_config(app: &AppHandle, config: LanguageConfig) {
    update_language_settings(app, &config);

    if let Some(model_id) = config.model.clone().filter(|m| !m.is_empty()) {
        apply_model_override(app, &model_id);
    }

    emit_language_changed_event(app, &config);
}

/// Update settings with language, prompt, and model selections from the config.
fn update_language_settings(app: &AppHandle, config: &LanguageConfig) {
    let mut settings = get_settings(app);

    settings.selected_language = config.language.clone();

    if let Some(ref prompt_id) = config.prompt_id {
        if !prompt_id.is_empty() {
            settings.post_process_selected_prompt_id = Some(prompt_id.clone());
        }
    }

    if let Some(ref model_id) = config.model {
        if !model_id.is_empty() {
            settings.selected_model = model_id.clone();
        }
    }

    settings::write_settings(app, settings);
}

/// Handle loading or downloading a model override.
fn apply_model_override(app: &AppHandle, model_id: &str) {
    let model_manager = app.state::<Arc<ModelManager>>().inner().clone();
    let transcription_manager = app.state::<Arc<TranscriptionManager>>().inner().clone();

    match model_manager.get_model_info(model_id) {
        Some(model_info) if model_info.is_downloaded => {
            load_model_in_thread(transcription_manager, model_id);
        }
        Some(model_info) if !model_info.is_downloading => {
            download_and_load_model(model_manager, transcription_manager, model_id);
        }
        None => {
            eprintln!("Language override model '{}' not found", model_id);
        }
        _ => {} // Already downloading, do nothing
    }
}

/// Load a model in a background thread.
fn load_model_in_thread(transcription_manager: Arc<TranscriptionManager>, model_id: &str) {
    let model_id = model_id.to_string();
    std::thread::spawn(move || {
        if let Err(e) = transcription_manager.load_model(&model_id) {
            eprintln!(
                "Failed to load language override model '{}': {}",
                model_id, e
            );
        }
    });
}

/// Download a model and then load it.
fn download_and_load_model(
    model_manager: Arc<ModelManager>,
    transcription_manager: Arc<TranscriptionManager>,
    model_id: &str,
) {
    let model_id = model_id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = model_manager.download_model(&model_id).await {
            eprintln!(
                "Failed to download language override model '{}': {}",
                model_id, e
            );
            return;
        }

        if let Err(e) = transcription_manager.load_model(&model_id) {
            eprintln!(
                "Failed to load language override model '{}': {}",
                model_id, e
            );
        }
    });
}

/// Emit a language-changed event to notify the frontend.
fn emit_language_changed_event(app: &AppHandle, config: &LanguageConfig) {
    let settings = get_settings(app);

    let _ = app.emit(
        "language-changed",
        serde_json::json!({
            "language": config.language,
            "config_id": config.id,
            "prompt_id": settings.post_process_selected_prompt_id,
        }),
    );
}

// ── Language shortcut commands ───────────────────────────────────────────────

/// Save an updated list of language configs and (re)register their shortcuts.
#[tauri::command]
pub fn update_language_configs(app: AppHandle, configs: Vec<LanguageConfig>) -> Result<(), String> {
    let current_settings = get_settings(&app);

    // Unregister all old language shortcuts from runtime
    {
        let rt_arc = get_runtime().clone();
        let mut rt = rt_arc.lock().unwrap();
        for config in &current_settings.language_configs {
            if !config.shortcut_binding.is_empty() {
                if let Ok(combo) = normalize_shortcut_string(&config.shortcut_binding) {
                    let binding_id = format!("language:{}", config.id);
                    if rt.combo_to_id.get(&combo) == Some(&binding_id) {
                        rt.combo_to_id.remove(&combo);
                    }
                }
            }
        }
    }

    // Save updated configs
    let mut updated_settings = current_settings;
    updated_settings.language_configs = configs.clone();
    settings::write_settings(&app, updated_settings);

    // Register new language shortcuts
    {
        let rt_arc = get_runtime().clone();
        let mut rt = rt_arc.lock().unwrap();
        for config in &configs {
            if !config.shortcut_binding.is_empty() {
                if let Ok(combo) = normalize_shortcut_string(&config.shortcut_binding) {
                    let binding_id = format!("language:{}", config.id);
                    rt.combo_to_id.insert(combo, binding_id);
                }
            }
        }
    }

    Ok(())
}

/// Save the language cycle shortcut and (re)register it.
#[tauri::command]
pub fn update_language_cycle_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    let current_settings = get_settings(&app);

    // Unregister old cycle shortcut
    {
        let rt_arc = get_runtime().clone();
        let mut rt = rt_arc.lock().unwrap();
        if !current_settings.language_cycle_shortcut.is_empty() {
            if let Ok(combo) = normalize_shortcut_string(&current_settings.language_cycle_shortcut)
            {
                if rt.combo_to_id.get(&combo) == Some(&"language_cycle".to_string()) {
                    rt.combo_to_id.remove(&combo);
                }
            }
        }
    }

    // Save new shortcut
    let mut updated_settings = current_settings;
    updated_settings.language_cycle_shortcut = shortcut.clone();
    settings::write_settings(&app, updated_settings);

    // Register new cycle shortcut
    if !shortcut.is_empty() {
        if let Ok(combo) = normalize_shortcut_string(&shortcut) {
            let rt_arc = get_runtime().clone();
            let mut rt = rt_arc.lock().unwrap();
            rt.combo_to_id.insert(combo, "language_cycle".to_string());
        }
    }

    Ok(())
}

/// Temporarily unregister a per-language shortcut while the user is editing it.
#[tauri::command]
pub fn suspend_language_shortcut(app: AppHandle, config_id: String) -> Result<(), String> {
    let settings = get_settings(&app);
    if let Some(config) = settings.language_configs.iter().find(|c| c.id == config_id) {
        if !config.shortcut_binding.is_empty() {
            if let Ok(combo) = normalize_shortcut_string(&config.shortcut_binding) {
                let rt_arc = get_runtime().clone();
                let mut rt = rt_arc.lock().unwrap();
                let binding_id = format!("language:{}", config_id);
                if rt.combo_to_id.get(&combo) == Some(&binding_id) {
                    rt.combo_to_id.remove(&combo);
                }
            }
        }
    }
    Ok(())
}

/// Re-register a per-language shortcut after the user finishes editing.
#[tauri::command]
pub fn resume_language_shortcut(app: AppHandle, config_id: String) -> Result<(), String> {
    let settings = get_settings(&app);
    if let Some(config) = settings.language_configs.iter().find(|c| c.id == config_id) {
        if !config.shortcut_binding.is_empty() {
            if let Ok(combo) = normalize_shortcut_string(&config.shortcut_binding) {
                let rt_arc = get_runtime().clone();
                let mut rt = rt_arc.lock().unwrap();
                let binding_id = format!("language:{}", config_id);
                if let Some(existing_id) = rt.combo_to_id.get(&combo) {
                    if existing_id != &binding_id {
                        return Err(format!(
                            "Shortcut '{}' is already in use",
                            config.shortcut_binding
                        ));
                    }
                }
                rt.combo_to_id.insert(combo, binding_id);
            }
        }
    }
    Ok(())
}

/// Temporarily unregister the language cycle shortcut while the user is editing it.
#[tauri::command]
pub fn suspend_language_cycle_shortcut(app: AppHandle) -> Result<(), String> {
    let settings = get_settings(&app);
    if !settings.language_cycle_shortcut.is_empty() {
        if let Ok(combo) = normalize_shortcut_string(&settings.language_cycle_shortcut) {
            let rt_arc = get_runtime().clone();
            let mut rt = rt_arc.lock().unwrap();
            if rt.combo_to_id.get(&combo) == Some(&"language_cycle".to_string()) {
                rt.combo_to_id.remove(&combo);
            }
        }
    }
    Ok(())
}

/// Re-register the language cycle shortcut after the user finishes editing.
#[tauri::command]
pub fn resume_language_cycle_shortcut(app: AppHandle) -> Result<(), String> {
    let settings = get_settings(&app);
    if !settings.language_cycle_shortcut.is_empty() {
        if let Ok(combo) = normalize_shortcut_string(&settings.language_cycle_shortcut) {
            let rt_arc = get_runtime().clone();
            let mut rt = rt_arc.lock().unwrap();
            let binding_id = "language_cycle".to_string();
            if let Some(existing_id) = rt.combo_to_id.get(&combo) {
                if existing_id != &binding_id {
                    return Err(format!(
                        "Shortcut '{}' is already in use",
                        settings.language_cycle_shortcut
                    ));
                }
            }
            rt.combo_to_id.insert(combo, binding_id);
        }
    }
    Ok(())
}
