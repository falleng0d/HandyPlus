use enigo::{Enigo, Keyboard};
use std::time::Duration;

/// Types `text` with Enigo. A non-zero interval types one character at a time
/// with that pause between characters, for apps that drop fast input.
pub fn type_text(enigo: &mut Enigo, text: &str, interval_ms: u64) -> Result<(), String> {
    if interval_ms == 0 {
        return crate::input::paste_text_direct(enigo, text);
    }
    let mut buffer = [0u8; 4];
    for (index, character) in text.chars().enumerate() {
        if index > 0 {
            std::thread::sleep(Duration::from_millis(interval_ms));
        }
        enigo
            .text(character.encode_utf8(&mut buffer))
            .map_err(|e| format!("Failed to type text: {e}"))?;
    }
    Ok(())
}
