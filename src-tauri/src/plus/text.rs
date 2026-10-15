//! Changes to the final transcript, after post-processing.

use crate::plus::context::{self, ContextLimits};
use crate::settings::AppSettings;

pub async fn finish(settings: &AppSettings, text: String) -> String {
    let mut text = text;
    if settings.lowercase_first_letter
        || (settings.continuation_lowercase && continues_text(settings).await)
    {
        text = lowercase_first_char(&text);
    }
    if settings.remove_trailing_punctuation {
        text = strip_trailing_punctuation(&text);
    }
    text
}

/// Whether the focused field already has text before the cursor.
async fn continues_text(settings: &AppSettings) -> bool {
    let limits = ContextLimits::from_settings(settings);
    tokio::task::spawn_blocking(move || !context::text_before_cursor(limits).trim().is_empty())
        .await
        .unwrap_or(false)
}

fn lowercase_first_char(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Drops trailing `. , ; : …` and their CJK forms with any whitespace around
/// them, so a phrase can be pasted mid-sentence. `?` and `!` stay.
fn strip_trailing_punctuation(text: &str) -> String {
    text.trim_end_matches(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '.' | ',' | ';' | ':' | '…' | '。' | '，' | '；' | '：' | '、'
            )
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_only_the_first_character() {
        assert_eq!(lowercase_first_char("Hello World"), "hello World");
        assert_eq!(lowercase_first_char("Élan"), "élan");
        assert_eq!(lowercase_first_char(""), "");
    }

    #[test]
    fn strips_trailing_separators_and_keeps_questions() {
        assert_eq!(strip_trailing_punctuation("Hello there."), "Hello there");
        assert_eq!(strip_trailing_punctuation("Wait..."), "Wait");
        assert_eq!(strip_trailing_punctuation("Hello, world; "), "Hello, world");
        assert_eq!(strip_trailing_punctuation("hello . "), "hello");
        assert_eq!(strip_trailing_punctuation("done:"), "done");
        assert_eq!(strip_trailing_punctuation("你好。"), "你好");
        assert_eq!(strip_trailing_punctuation("e.g. this"), "e.g. this");
        assert_eq!(strip_trailing_punctuation("Done!"), "Done!");
        assert_eq!(strip_trailing_punctuation("Really?."), "Really?");
        assert_eq!(strip_trailing_punctuation("really？"), "really？");
        assert_eq!(strip_trailing_punctuation("wait...!"), "wait...!");
    }
}
