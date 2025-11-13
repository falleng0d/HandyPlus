use rdev::Key;

/// Check if a key is a modifier key and return its canonical name
pub fn is_modifier_key(k: Key) -> Option<&'static str> {
    match k {
        Key::ControlLeft | Key::ControlRight => Some("ctrl"),
        Key::ShiftLeft | Key::ShiftRight => Some("shift"),
        Key::Alt | Key::AltGr => Some("alt"),
        Key::MetaLeft | Key::MetaRight => Some("meta"),
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
        Enter => "enter",
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

/// Normalize a shortcut string to canonical form
/// Accepts various modifier names and returns normalized "mod1+mod2+key" format
pub fn normalize_shortcut_string(raw: &str) -> Result<String, String> {
    let mut mods: Vec<String> = Vec::new();
    let mut key: Option<String> = None;
    for part in raw.split('+') {
        let p = part.trim().to_lowercase();
        let canon = match p.as_str() {
            "control" => Some("ctrl"),
            "ctrl" => Some("ctrl"),
            "shift" => Some("shift"),
            "alt" | "option" => Some("alt"),
            "meta" | "command" | "cmd" | "super" | "win" | "windows" => Some("meta"),
            _ => None,
        };
        if let Some(m) = canon {
            mods.push(m.to_string());
        } else {
            if key.is_some() {
                return Err("Shortcut must not contain more than one non-modifier key".into());
            }
            key = Some(p);
        }
    }
    let key = key.ok_or_else(|| "Shortcut must contain a non-modifier key".to_string())?;
    Ok(normalize_combo_from_parts(&mut mods, &key))
}

/// Validate that a shortcut string contains at least one non-modifier key
pub fn validate_shortcut_string(raw: &str) -> Result<(), String> {
    let modifiers = [
        "ctrl", "control", "shift", "alt", "option", "meta", "command", "cmd", "super", "win",
        "windows",
    ];
    let has_non_modifier = raw
        .split('+')
        .any(|part| !modifiers.contains(&part.trim().to_lowercase().as_str()));
    if has_non_modifier {
        Ok(())
    } else {
        Err("Shortcut must contain at least one non-modifier key".into())
    }
}
