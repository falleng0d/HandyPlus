//! Upstream shortcuts always have keys. Handy+ lets a shortcut with no
//! default keys be cleared by resetting it.

use log::warn;
use tauri::AppHandle;

use crate::settings::{self, ShortcutBinding};
use crate::shortcut::{self, BindingResponse};

pub fn unassign(app: &AppHandle, binding: ShortcutBinding) -> Result<BindingResponse, String> {
    if let Err(e) = shortcut::unregister_shortcut(app, binding.clone()) {
        warn!("Failed to unregister shortcut '{}': {}", binding.id, e);
    }
    let mut cleared = binding;
    cleared.current_binding = String::new();
    let mut settings = settings::get_settings(app);
    settings
        .bindings
        .insert(cleared.id.clone(), cleared.clone());
    settings::write_settings(app, settings);
    crate::secure_input::reconcile_fallback(app);
    Ok(BindingResponse {
        success: true,
        binding: Some(cleared),
        error: None,
    })
}
