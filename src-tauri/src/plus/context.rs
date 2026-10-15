//! Context providers for the text editor that currently has focus.

#[cfg(windows)]
fn read_before_cursor<T: uiautomation::actions::Text>(
    control: T,
    lines: usize,
    max_characters: Option<usize>,
) -> uiautomation::Result<String> {
    use uiautomation::types::{TextPatternRangeEndpoint, TextUnit};

    let (_, caret) = control.get_caret_range()?;
    let before = caret.clone();
    before.move_endpoint_by_unit(
        TextPatternRangeEndpoint::Start,
        TextUnit::Line,
        -(lines.min(i32::MAX as usize) as i32),
    )?;
    let text = before.get_text(-1)?;
    Ok(match max_characters {
        Some(limit) => {
            let mut chars: Vec<char> = text.chars().rev().take(limit).collect();
            chars.reverse();
            chars.into_iter().collect()
        }
        None => text,
    })
}

#[cfg(windows)]
pub fn text_before_cursor(lines: usize, max_characters: Option<usize>) -> String {
    use uiautomation::controls::{EditControl, TextControl};
    use uiautomation::UIAutomation;

    let result = (|| -> uiautomation::Result<String> {
        let automation = UIAutomation::new()?;
        let focused = automation.get_focused_element()?;
        if let Ok(edit) = EditControl::try_from(&focused) {
            return read_before_cursor(edit, lines, max_characters);
        }
        let text = TextControl::try_from(focused)?;
        read_before_cursor(text, lines, max_characters)
    })();

    result.unwrap_or_default()
}

#[cfg(not(windows))]
pub fn text_before_cursor(_lines: usize, _max_characters: Option<usize>) -> String {
    String::new()
}
