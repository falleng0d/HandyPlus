use rdev::Key;

/// Check if a key is a modifier key and return its canonical name
/// Returns the canonical modifier name (e.g., "ctrl", "shift") for tracking pressed modifiers
pub fn is_modifier_key(k: Key) -> Option<&'static str> {
    match k {
        Key::ControlLeft | Key::ControlRight => Some("ctrl"),
        Key::ShiftLeft | Key::ShiftRight => Some("shift"),
        Key::Alt | Key::AltGr => Some("alt"),
        Key::MetaLeft | Key::MetaRight => Some("meta"),
        _ => None,
    }
}

/// Convert a modifier key to its specific name (left/right variant)
/// Used when a modifier key is the primary shortcut key itself
pub fn modifier_key_to_specific_name(k: Key) -> Option<String> {
    match k {
        Key::ControlLeft => Some("left ctrl".to_string()),
        Key::ControlRight => Some("right ctrl".to_string()),
        Key::ShiftLeft => Some("left shift".to_string()),
        Key::ShiftRight => Some("right shift".to_string()),
        Key::Alt => Some("left alt".to_string()),
        Key::AltGr => Some("right alt".to_string()),
        Key::MetaLeft => Some("left meta".to_string()),
        Key::MetaRight => Some("right meta".to_string()),
        _ => None,
    }
}

/// Convert a Key to its string representation
#[allow(non_snake_case)]
#[allow(unused_variables)]
#[allow(unreachable_patterns)]
pub fn key_to_name(k: Key) -> Option<String> {
    use Key::*;
    let s = match k {
        KeyA => "a",
        KeyB => "b",
        KeyC => "c",
        KeyD => "d",
        KeyE => "e",
        KeyF => "f",
        KeyG => "g",
        KeyH => "h",
        KeyI => "i",
        KeyJ => "j",
        KeyK => "k",
        KeyL => "l",
        KeyM => "m",
        KeyN => "n",
        KeyO => "o",
        KeyP => "p",
        KeyQ => "q",
        KeyR => "r",
        KeyS => "s",
        KeyT => "t",
        KeyU => "u",
        KeyV => "v",
        KeyW => "w",
        KeyX => "x",
        KeyY => "y",
        KeyZ => "z",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        Num0 => "0",
        Space => "space",
        Return => "enter",
        Tab => "tab",
        Escape => "escape",
        F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4", F5 => "f5",
        F6 => "f6", F7 => "f7", F8 => "f8", F9 => "f9", F10 => "f10",
        F11 => "f11", F12 => "f12", F13 => "f13", F14 => "f14",
        F15 => "f15", F16 => "f16", F17 => "f17", F18 => "f18",
        F19 => "f19", F20 => "f20", F21 => "f21", F22 => "f22",
        F23 => "f23", F24 => "f24",
        Minus => "-",
        Equal => "=",
        LeftBracket => "[",
        RightBracket => "]",
        BackSlash => "\\",
        Semicolon => ";",
        Quote => "'",
        Comma => ",",
        Dot => ".",
        Slash => "/",
        BackQuote => "`",
        Backspace => "backspace",
        CapsLock => "capslock",
        Home => "home",
        End => "end",
        PageUp => "pageup",
        PageDown => "pagedown",
        ArrowUp => "up",
        ArrowDown => "down",
        ArrowLeft => "left",
        ArrowRight => "right",
        Insert => "insert",
        Delete => "delete",
        _ => return None,
    };
    Some(s.to_string())
}

/// Normalize a hotkey combo from parts (modifiers + key)
pub fn normalize_combo_from_parts(mods: &mut Vec<String>, key: &str) -> String {
    mods.sort();
    let mut parts = mods.clone();
    parts.push(key.to_string());
    parts.join("+")
}

/// Check if a string is a modifier name (including left/right variants)
fn is_modifier_name(s: &str) -> bool {
    matches!(
        s,
        "ctrl" | "control" | "left ctrl" | "left control" | "right ctrl" | "right control"
        | "shift" | "left shift" | "right shift"
        | "alt" | "option" | "left alt" | "left option" | "right alt" | "right option"
        | "meta" | "command" | "cmd" | "super" | "win" | "windows"
        | "left meta" | "left command" | "left cmd" | "left super" | "left win"
        | "right meta" | "right command" | "right cmd" | "right super" | "right win"
    )
}

/// Normalize a shortcut string to canonical form
/// Accepts various modifier names and returns normalized "mod1+mod2+key" format
/// Now supports:
/// - Simple keys: "a", "f1", "space"
/// - Modifier combinations: "ctrl+space", "left ctrl+f1", "shift+a"
/// - Single modifiers as shortcuts: "left alt", "right ctrl"
pub fn normalize_shortcut_string(raw: &str) -> Result<String, String> {
    let parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();

    // Handle single modifier key as the shortcut itself
    if parts.len() == 1 {
        let p = parts[0].to_lowercase();
        if is_modifier_name(&p) {
            // Single modifier key - return it as-is (normalized)
            return Ok(normalize_modifier_name(&p));
        }
        // Single non-modifier key
        return Ok(p);
    }

    // Handle modifier + key combinations
    let mut mods: Vec<String> = Vec::new();
    let mut key: Option<String> = None;

    for part in parts {
        let p = part.to_lowercase();

        // Check if this is a modifier name
        if is_modifier_name(&p) {
            // Normalize the modifier name to canonical form
            let canon = normalize_modifier_to_canonical(&p);
            mods.push(canon);
        } else {
            // This is the primary key
            if key.is_some() {
                return Err("Shortcut must not contain more than one non-modifier key".into());
            }
            key = Some(p);
        }
    }

    let key = key.ok_or_else(|| "Shortcut with multiple parts must contain a non-modifier key".to_string())?;
    Ok(normalize_combo_from_parts(&mut mods, &key))
}

/// Normalize a modifier name to its canonical form for tracking
/// "left ctrl" -> "ctrl", "right shift" -> "shift", etc.
fn normalize_modifier_to_canonical(name: &str) -> String {
    match name {
        "control" | "ctrl" | "left ctrl" | "left control" | "right ctrl" | "right control" => "ctrl".to_string(),
        "shift" | "left shift" | "right shift" => "shift".to_string(),
        "alt" | "option" | "left alt" | "left option" | "right alt" | "right option" => "alt".to_string(),
        "meta" | "command" | "cmd" | "super" | "win" | "windows"
        | "left meta" | "left command" | "left cmd" | "left super" | "left win"
        | "right meta" | "right command" | "right cmd" | "right super" | "right win" => "meta".to_string(),
        _ => name.to_string(),
    }
}

/// Normalize a modifier name to its specific form (preserving left/right)
/// Used when a modifier is the primary shortcut key
fn normalize_modifier_name(name: &str) -> String {
    match name {
        "left control" => "left ctrl".to_string(),
        "right control" => "right ctrl".to_string(),
        "left option" => "left alt".to_string(),
        "right option" => "right alt".to_string(),
        "left command" | "left cmd" | "left super" | "left win" => "left meta".to_string(),
        "right command" | "right cmd" | "right super" | "right win" => "right meta".to_string(),
        "command" | "cmd" | "super" | "win" | "windows" => "meta".to_string(),
        "control" => "ctrl".to_string(),
        "option" => "alt".to_string(),
        _ => name.to_string(),
    }
}

/// Validate that a shortcut string is valid
/// Now allows single modifier keys as shortcuts
pub fn validate_shortcut_string(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Shortcut cannot be empty".into());
    }

    let parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();

    // Single part can be either a modifier or a regular key
    if parts.len() == 1 {
        return Ok(());
    }

    // Multiple parts: must have at least one non-modifier
    let has_non_modifier = parts.iter().any(|part| {
        let p = part.to_lowercase();
        !is_modifier_name(&p)
    });

    if has_non_modifier {
        Ok(())
    } else {
        Err("Shortcut with multiple modifiers must contain at least one non-modifier key".into())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_simple_keys() {
        assert_eq!(normalize_shortcut_string("a").unwrap(), "a");
        assert_eq!(normalize_shortcut_string("f1").unwrap(), "f1");
        assert_eq!(normalize_shortcut_string("space").unwrap(), "space");
        assert_eq!(normalize_shortcut_string("Space").unwrap(), "space");
    }

    #[test]
    fn test_normalize_modifier_combinations() {
        // Basic combinations
        assert_eq!(normalize_shortcut_string("ctrl+space").unwrap(), "ctrl+space");
        assert_eq!(normalize_shortcut_string("shift+a").unwrap(), "shift+a");

        // Left/right modifiers with keys
        assert_eq!(normalize_shortcut_string("left ctrl+f1").unwrap(), "ctrl+f1");
        assert_eq!(normalize_shortcut_string("right shift+a").unwrap(), "shift+a");

        // Multiple modifiers
        assert_eq!(normalize_shortcut_string("ctrl+shift+a").unwrap(), "ctrl+shift+a");
        assert_eq!(normalize_shortcut_string("left ctrl+shift+f1").unwrap(), "ctrl+shift+f1");

        // Case insensitive
        assert_eq!(normalize_shortcut_string("CTRL+SPACE").unwrap(), "ctrl+space");
        assert_eq!(normalize_shortcut_string("Left Ctrl+F1").unwrap(), "ctrl+f1");
    }

    #[test]
    fn test_normalize_single_modifiers() {
        // Single modifier keys as shortcuts
        assert_eq!(normalize_shortcut_string("left alt").unwrap(), "left alt");
        assert_eq!(normalize_shortcut_string("right ctrl").unwrap(), "right ctrl");
        assert_eq!(normalize_shortcut_string("left shift").unwrap(), "left shift");
        assert_eq!(normalize_shortcut_string("right alt").unwrap(), "right alt");

        // Canonical modifiers default to left
        assert_eq!(normalize_shortcut_string("ctrl").unwrap(), "ctrl");
        assert_eq!(normalize_shortcut_string("alt").unwrap(), "alt");
        assert_eq!(normalize_shortcut_string("shift").unwrap(), "shift");

        // Aliases
        assert_eq!(normalize_shortcut_string("left option").unwrap(), "left alt");
        assert_eq!(normalize_shortcut_string("left control").unwrap(), "left ctrl");
    }

    #[test]
    fn test_validate_shortcuts() {
        // Valid shortcuts
        assert!(validate_shortcut_string("a").is_ok());
        assert!(validate_shortcut_string("ctrl+space").is_ok());
        assert!(validate_shortcut_string("left alt").is_ok());
        assert!(validate_shortcut_string("ctrl+shift+a").is_ok());

        // Invalid shortcuts
        assert!(validate_shortcut_string("").is_err());
        assert!(validate_shortcut_string("   ").is_err());
        assert!(validate_shortcut_string("ctrl+shift").is_err()); // Multiple modifiers without key
    }

    #[test]
    fn test_modifier_key_detection() {
        use rdev::Key;

        assert_eq!(is_modifier_key(Key::ControlLeft), Some("ctrl"));
        assert_eq!(is_modifier_key(Key::ControlRight), Some("ctrl"));
        assert_eq!(is_modifier_key(Key::ShiftLeft), Some("shift"));
        assert_eq!(is_modifier_key(Key::ShiftRight), Some("shift"));
        assert_eq!(is_modifier_key(Key::Alt), Some("alt"));
        assert_eq!(is_modifier_key(Key::AltGr), Some("alt"));
        assert_eq!(is_modifier_key(Key::MetaLeft), Some("meta"));
        assert_eq!(is_modifier_key(Key::MetaRight), Some("meta"));

        assert_eq!(is_modifier_key(Key::KeyA), None);
        assert_eq!(is_modifier_key(Key::Space), None);
    }

    #[test]
    fn test_modifier_key_to_specific_name() {
        use rdev::Key;

        assert_eq!(modifier_key_to_specific_name(Key::ControlLeft), Some("left ctrl".to_string()));
        assert_eq!(modifier_key_to_specific_name(Key::ControlRight), Some("right ctrl".to_string()));
        assert_eq!(modifier_key_to_specific_name(Key::ShiftLeft), Some("left shift".to_string()));
        assert_eq!(modifier_key_to_specific_name(Key::ShiftRight), Some("right shift".to_string()));
        assert_eq!(modifier_key_to_specific_name(Key::Alt), Some("left alt".to_string()));
        assert_eq!(modifier_key_to_specific_name(Key::AltGr), Some("right alt".to_string()));

        assert_eq!(modifier_key_to_specific_name(Key::KeyA), None);
    }

    #[test]
    fn test_key_to_name() {
        use rdev::Key;

        assert_eq!(key_to_name(Key::KeyA), Some("a".to_string()));
        assert_eq!(key_to_name(Key::Space), Some("space".to_string()));
        assert_eq!(key_to_name(Key::F1), Some("f1".to_string()));
        assert_eq!(key_to_name(Key::Return), Some("enter".to_string()));
    }

    #[test]
    fn test_normalize_combo_from_parts() {
        let mut mods = vec!["ctrl".to_string(), "shift".to_string()];
        assert_eq!(normalize_combo_from_parts(&mut mods, "a"), "ctrl+shift+a");

        let mut mods = vec!["shift".to_string(), "ctrl".to_string()];
        assert_eq!(normalize_combo_from_parts(&mut mods, "a"), "ctrl+shift+a"); // Should be sorted

        let mut mods = vec![];
        assert_eq!(normalize_combo_from_parts(&mut mods, "space"), "space");
    }
}
