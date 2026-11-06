//! Enter finishes a locked recording, as pressing its shortcut again would.
//! The key is registered only while such a recording runs, so it reaches
//! other apps the rest of the time.

use crate::settings::ShortcutBinding;
use crate::TranscriptionCoordinator;
use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

const ID: &str = "confirm";
const KEY: &str = "enter";

/// Whether the coordinator currently wants the confirm key.
static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Whether the confirm key is registered. The lock also serializes passes.
#[cfg(not(target_os = "linux"))]
static REGISTERED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

fn binding() -> ShortcutBinding {
    ShortcutBinding {
        id: ID.to_string(),
        name: "Confirm".to_string(),
        description: String::new(),
        default_binding: KEY.to_string(),
        current_binding: KEY.to_string(),
    }
}

/// Registers the confirm key while `active`. Spawned tasks may run out of
/// order, so each pass applies the latest request.
pub fn set_active(app: &AppHandle, active: bool) {
    REQUESTED.store(active, Ordering::SeqCst);
    let app = app.clone();
    tauri::async_runtime::spawn(async move { reconcile(&app) });
}

// Dynamic registration is unstable on Linux, as for the cancel shortcut.
#[cfg(target_os = "linux")]
fn reconcile(_app: &AppHandle) {}

#[cfg(not(target_os = "linux"))]
fn reconcile(app: &AppHandle) {
    let mut registered = REGISTERED.lock().unwrap_or_else(|e| e.into_inner());
    let requested = REQUESTED.load(Ordering::SeqCst);
    if requested == *registered {
        return;
    }
    let result = if requested {
        crate::shortcut::register_shortcut(app, binding())
    } else {
        crate::shortcut::unregister_shortcut(app, binding())
    };
    match result {
        Ok(()) => {
            *registered = requested;
            debug!("Confirm key registered: {requested}");
        }
        Err(e) => error!("Failed to update the confirm key: {e}"),
    }
}

/// Handles the confirm key's events; `false` for any other binding.
pub fn handle_shortcut(app: &AppHandle, binding_id: &str, is_pressed: bool) -> bool {
    if binding_id != ID {
        return false;
    }
    if is_pressed {
        if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
            coordinator.notify_confirm();
        }
    }
    true
}
