//! Tray menu strings
//!
//! Generated at compile time by build.rs from the "tray" keys in
//! src/i18n/locales/en/translation.json.
//!
//! To add a new tray menu item:
//! 1. Add the key to en/translation.json under "tray"
//! 2. Update tray.rs to use the new field (e.g., strings.new_field)

use once_cell::sync::Lazy;
use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/tray_translations.rs"));

pub fn get_tray_translations() -> TrayStrings {
    TRANSLATIONS
        .get("en")
        .cloned()
        .expect("English translations must exist")
}
