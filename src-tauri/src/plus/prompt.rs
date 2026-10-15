//! Handy+ post-processing prompt variables, filled in before upstream
//! substitutes `${output}`:
//! - `${dictionary}`: the custom words, comma separated;
//! - `${language}`: English name of the selected transcription language;
//! - `${transcript}`: text before the cursor in the focused editor (Windows).

use crate::settings::AppSettings;

pub async fn expand_prompt_variables(settings: &AppSettings, prompt: &str) -> String {
    let before_cursor = if prompt.contains("${transcript}") {
        let lines = settings.context_lines as usize;
        let max_characters = settings.context_max_characters.map(|c| c as usize);
        tokio::task::spawn_blocking(move || {
            crate::plus::context::text_before_cursor(lines, max_characters)
        })
        .await
        .unwrap_or_default()
    } else {
        String::new()
    };
    fill_variables(prompt, settings, &before_cursor)
}

fn fill_variables(prompt: &str, settings: &AppSettings, before_cursor: &str) -> String {
    prompt
        .replace("${dictionary}", &settings.custom_words.join(", "))
        .replace("${language}", &language_name(&settings.selected_language))
        .replace("${transcript}", before_cursor)
}

fn language_name(code: &str) -> String {
    if code == "auto" {
        return "Auto Detect".to_string();
    }
    let base = code.split(['-', '_']).next().unwrap_or(code);
    let language = match base.len() {
        2 => isolang::Language::from_639_1(base),
        3 => isolang::Language::from_639_3(base),
        _ => None,
    };
    language.map_or_else(|| code.to_string(), |l| l.to_name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::get_default_settings;

    fn settings(words: &[&str], language: &str) -> AppSettings {
        AppSettings {
            custom_words: words.iter().map(|w| w.to_string()).collect(),
            selected_language: language.to_string(),
            ..get_default_settings()
        }
    }

    #[test]
    fn fills_every_variable_and_leaves_output_for_upstream() {
        let prompt =
            "Words: ${dictionary}\nLanguage: ${language}\nBefore: ${transcript}\n${output}";

        let filled = fill_variables(prompt, &settings(&["Handy", "Tauri"], "pt-BR"), "Dear Ana,");

        assert_eq!(
            filled,
            "Words: Handy, Tauri\nLanguage: Portuguese\nBefore: Dear Ana,\n${output}"
        );
    }

    #[test]
    fn missing_values_become_empty() {
        let filled = fill_variables("[${dictionary}][${transcript}]", &settings(&[], "en"), "");

        assert_eq!(filled, "[][]");
    }

    #[test]
    fn prompt_without_variables_is_unchanged() {
        let prompt = "Fix punctuation: ${output}";

        assert_eq!(fill_variables(prompt, &settings(&["x"], "en"), "y"), prompt);
    }

    #[test]
    fn language_names_cover_auto_regions_and_unknown_codes() {
        assert_eq!(language_name("auto"), "Auto Detect");
        assert_eq!(language_name("en"), "English");
        assert_eq!(language_name("zh-Hans"), "Chinese");
        assert_eq!(language_name("yue"), "Yue Chinese");
        assert_eq!(language_name("xx"), "xx");
    }
}
