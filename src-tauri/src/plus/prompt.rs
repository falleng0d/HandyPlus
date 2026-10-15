//! Handy+ post-processing prompt variables, filled in before upstream
//! substitutes `${output}`:
//! - `${dictionary}`: the custom words, comma separated;
//! - `${language}`: English name of the selected transcription language;
//! - `${input}` (or `${transcript}`): text before the cursor in the focused field;
//! - `${app}`: name of the app in front.
//!
//! A part wrapped in `<on input>` and `</on input>` is kept only when there is
//! text before the cursor. The tags themselves are always removed.

use crate::plus::context::{self, ContextLimits};
use crate::settings::AppSettings;

const INPUT_VARIABLES: [&str; 2] = ["${input}", "${transcript}"];
const APP_VARIABLE: &str = "${app}";
const OPEN_TAG: &str = "<on input>";
const CLOSE_TAG: &str = "</on input>";

pub async fn expand_prompt_variables(settings: &AppSettings, prompt: &str) -> String {
    let wants_input =
        prompt.contains(OPEN_TAG) || INPUT_VARIABLES.iter().any(|name| prompt.contains(name));
    let wants_app = prompt.contains(APP_VARIABLE);
    let limits = ContextLimits::from_settings(settings);
    let (input, app) = tokio::task::spawn_blocking(move || {
        let input = if wants_input {
            context::text_before_cursor(limits)
        } else {
            String::new()
        };
        let app = if wants_app {
            context::foreground_app_name()
        } else {
            String::new()
        };
        (input, app)
    })
    .await
    .unwrap_or_default();
    fill_variables(prompt, settings, &input, &app)
}

fn fill_variables(prompt: &str, settings: &AppSettings, input: &str, app: &str) -> String {
    let has_input = !input.trim().is_empty();
    let input = if has_input { input } else { "" };
    let mut filled = render_input_sections(prompt, has_input)
        .replace("${dictionary}", &settings.custom_words.join(", "))
        .replace("${language}", &language_name(&settings.selected_language))
        .replace(APP_VARIABLE, app);
    for name in INPUT_VARIABLES {
        filled = filled.replace(name, input);
    }
    filled
}

/// Keeps or drops each `<on input>` part. A missing closing tag runs to the
/// end of the prompt. One newline next to each tag goes with it, so tags on
/// their own lines leave no blank lines.
fn render_input_sections(prompt: &str, has_input: bool) -> String {
    let mut out = String::with_capacity(prompt.len());
    let mut rest = prompt;
    while let Some(open) = rest.find(OPEN_TAG) {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + OPEN_TAG.len()..];
        let (inner, after_close) = match after_open.find(CLOSE_TAG) {
            Some(close) => (&after_open[..close], &after_open[close + CLOSE_TAG.len()..]),
            None => (after_open, ""),
        };
        if has_input {
            out.push_str(strip_trailing_newline(strip_leading_newline(inner)));
            rest = after_close;
        } else {
            rest = strip_leading_newline(after_close);
        }
    }
    out.push_str(rest);
    out
}

fn strip_leading_newline(text: &str) -> &str {
    text.strip_prefix("\r\n")
        .or_else(|| text.strip_prefix('\n'))
        .unwrap_or(text)
}

fn strip_trailing_newline(text: &str) -> &str {
    text.strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text)
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
        let prompt = "Words: ${dictionary}\nLanguage: ${language}\nApp: ${app}\nBefore: ${input}|${transcript}\n${output}";

        let filled = fill_variables(
            prompt,
            &settings(&["Handy", "Tauri"], "pt-BR"),
            "Dear Ana,",
            "Code",
        );

        assert_eq!(
            filled,
            "Words: Handy, Tauri\nLanguage: Portuguese\nApp: Code\nBefore: Dear Ana,|Dear Ana,\n${output}"
        );
    }

    #[test]
    fn missing_values_become_empty() {
        let filled = fill_variables(
            "[${dictionary}][${input}][${app}]",
            &settings(&[], "en"),
            " \n",
            "",
        );

        assert_eq!(filled, "[][][]");
    }

    #[test]
    fn prompt_without_variables_is_unchanged() {
        let prompt = "Fix punctuation: ${output}";

        assert_eq!(
            fill_variables(prompt, &settings(&["x"], "en"), "y", "z"),
            prompt
        );
    }

    #[test]
    fn input_section_on_its_own_lines() {
        let prompt =
            "Clean this.\n<on input>\nContext:\n${input}\n</on input>\nTranscript:\n${output}";

        assert_eq!(
            fill_variables(prompt, &settings(&[], "en"), "DOC", ""),
            "Clean this.\nContext:\nDOC\nTranscript:\n${output}"
        );
        assert_eq!(
            fill_variables(prompt, &settings(&[], "en"), "", ""),
            "Clean this.\nTranscript:\n${output}"
        );
    }

    #[test]
    fn inline_and_repeated_input_sections() {
        let prompt = "start <on input>one ${input}</on input> mid <on input>two</on input> end";

        assert_eq!(
            render_input_sections(prompt, true),
            "start one ${input} mid two end"
        );
        assert_eq!(render_input_sections(prompt, false), "start  mid  end");
    }

    #[test]
    fn input_section_without_closing_tag_runs_to_the_end() {
        let prompt = "keep\n<on input>\nonly with input";

        assert_eq!(render_input_sections(prompt, true), "keep\nonly with input");
        assert_eq!(render_input_sections(prompt, false), "keep\n");
    }

    #[test]
    fn input_section_with_windows_line_endings() {
        let prompt = "A\r\n<on input>\r\nB\r\n</on input>\r\nC";

        assert_eq!(render_input_sections(prompt, true), "A\r\nB\r\nC");
        assert_eq!(render_input_sections(prompt, false), "A\r\nC");
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
