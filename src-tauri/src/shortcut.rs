use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::actions::ACTION_MAP;
use crate::hotkey::{
    is_modifier_key, key_to_name, modifier_key_to_specific_name, normalize_combo_from_parts,
    normalize_shortcut_string, validate_shortcut_string,
};
use crate::settings::ShortcutBinding;
use crate::settings::{self, get_settings};
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
