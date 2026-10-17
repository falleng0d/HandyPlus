//! Reads the focused text field and the app in front, for prompt variables and
//! for finishing the transcript.

use crate::settings::AppSettings;

/// How much of the text before the cursor to keep, counted from the cursor.
/// 0 means no limit.
#[derive(Clone, Copy, Debug)]
pub struct ContextLimits {
    pub paragraphs: usize,
    pub lines: usize,
    pub characters: usize,
}

impl ContextLimits {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            paragraphs: settings.context_max_paragraphs as usize,
            lines: settings.context_lines as usize,
            characters: settings.context_max_characters.unwrap_or(0) as usize,
        }
    }
}

/// Text before the cursor in the focused field, trimmed to `limits`. Empty
/// when the field can't be read. Blocks while the target app answers.
pub fn text_before_cursor(limits: ContextLimits) -> String {
    let started = std::time::Instant::now();
    let text = platform::text_before_cursor(limits.lines).unwrap_or_default();
    let text = limit_tail(&text, limits);
    log::debug!(
        "Text before the cursor in {:?} ({} chars, {} ms): {:?}",
        foreground_app_name(),
        text.chars().count(),
        started.elapsed().as_millis(),
        text
    );
    text
}

/// Name of the app in front, or an empty string when it can't be read.
pub fn foreground_app_name() -> String {
    platform::foreground_app_name()
        .map(|name| name.trim().to_string())
        .unwrap_or_default()
}

/// Keeps the end of `text` within every limit; the shortest result wins.
/// Line endings become `\n` once any limit is set.
fn limit_tail(text: &str, limits: ContextLimits) -> String {
    if limits.paragraphs == 0 && limits.lines == 0 && limits.characters == 0 {
        return text.to_string();
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let chars: Vec<char> = normalized.chars().collect();
    let mut start = 0;
    if limits.characters > 0 {
        start = start.max(chars.len().saturating_sub(limits.characters));
    }
    if limits.lines > 0 {
        start = start.max(last_lines_start(&chars, limits.lines));
    }
    if limits.paragraphs > 0 {
        start = start.max(last_paragraphs_start(&chars, limits.paragraphs));
    }
    chars[start..].iter().collect()
}

fn last_lines_start(chars: &[char], count: usize) -> usize {
    let mut seen = 0;
    for index in (0..chars.len()).rev() {
        if chars[index] == '\n' {
            seen += 1;
            if seen == count {
                return index + 1;
            }
        }
    }
    0
}

/// Paragraphs are separated by two or more newlines in a row.
fn last_paragraphs_start(chars: &[char], count: usize) -> usize {
    let mut seen = 0;
    let mut index = chars.len();
    while index > 0 {
        index -= 1;
        if chars[index] != '\n' {
            continue;
        }
        let run_end = index;
        while index > 0 && chars[index - 1] == '\n' {
            index -= 1;
        }
        if run_end > index {
            seen += 1;
            if seen == count {
                return run_end + 1;
            }
        }
    }
    0
}

#[cfg(windows)]
mod platform {
    use std::sync::mpsc;
    use std::time::Duration;
    use uiautomation::patterns::{UITextPattern, UITextRange, UIValuePattern};
    use uiautomation::types::{TextPatternRangeEndpoint, TextUnit};
    use uiautomation::UIAutomation;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    /// How long to wait for the focused app, so a hung app can't stall
    /// transcription.
    const READ_TIMEOUT: Duration = Duration::from_millis(500);

    /// Empty range at the cursor, from the caret when the control reports it
    /// and from the start of the selection otherwise.
    fn cursor_range(pattern: &UITextPattern) -> Option<UITextRange> {
        if let Ok((_, caret)) = pattern.get_caret_range() {
            return Some(caret);
        }
        let selection = pattern.get_selection().ok()?.into_iter().next()?;
        selection
            .move_endpoint_by_range(
                TextPatternRangeEndpoint::End,
                &selection,
                TextPatternRangeEndpoint::Start,
            )
            .ok()?;
        Some(selection)
    }

    fn text_before(pattern: &UITextPattern, lines: usize) -> Option<String> {
        let range = cursor_range(pattern)?;
        let lines = if lines == 0 {
            i32::MAX
        } else {
            lines.min(i32::MAX as usize) as i32
        };
        range
            .move_endpoint_by_unit(TextPatternRangeEndpoint::Start, TextUnit::Line, -lines)
            .ok()?;
        // Chromium ranges can move past the focused field into the rest of the page.
        let field = pattern.get_document_range().ok()?;
        let before_field = range
            .compare_endpoints(
                TextPatternRangeEndpoint::Start,
                &field,
                TextPatternRangeEndpoint::Start,
            )
            .ok()?
            < 0;
        if before_field {
            range
                .move_endpoint_by_range(
                    TextPatternRangeEndpoint::Start,
                    &field,
                    TextPatternRangeEndpoint::Start,
                )
                .ok()?;
        }
        range.get_text(-1).ok()
    }

    fn read_focused(lines: usize) -> Option<String> {
        let automation = UIAutomation::new_direct().ok()?;
        let focused = automation.get_focused_element().ok()?;
        log::debug!(
            "Focused element: {:?} {:?}",
            focused.get_control_type(),
            focused.get_classname()
        );
        if let Ok(pattern) = focused.get_pattern::<UITextPattern>() {
            if let Some(text) = text_before(&pattern, lines) {
                log::debug!("Read the text before the cursor from the text pattern");
                return Some(text);
            }
            if let Ok(text) = pattern
                .get_document_range()
                .and_then(|range| range.get_text(-1))
            {
                log::debug!("Read the whole text pattern; the cursor is unknown");
                return Some(text);
            }
        }
        let value = focused
            .get_pattern::<UIValuePattern>()
            .and_then(|pattern| pattern.get_value())
            .ok();
        log::debug!("Read the value pattern: {}", value.is_some());
        value
    }

    /// Scintilla editors (Notepad++, Notepad4) have no UI Automation support,
    /// but answer window messages from other processes.
    fn scintilla_text_before_cursor() -> Option<String> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
            SendMessageTimeoutW, GUITHREADINFO, SMTO_ABORTIFHUNG, WM_GETTEXT,
        };
        const SCI_GETCURRENTPOS: u32 = 2008;
        const SCI_GETCODEPAGE: u32 = 2137;
        const SC_CP_UTF8: usize = 65001;

        unsafe {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let mut info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            GetGUIThreadInfo(thread, &mut info).ok()?;
            let field = info.hwndFocus;
            if field.0.is_null() {
                return None;
            }
            let mut class = [0u16; 64];
            let length = GetClassNameW(field, &mut class) as usize;
            if String::from_utf16_lossy(&class[..length]) != "Scintilla" {
                return None;
            }
            let send = |message: u32, wparam: usize, lparam: isize| -> Option<usize> {
                let mut result = 0usize;
                let sent = SendMessageTimeoutW(
                    field,
                    message,
                    WPARAM(wparam),
                    LPARAM(lparam),
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                );
                (sent.0 != 0).then_some(result)
            };
            if send(SCI_GETCODEPAGE, 0, 0)? != SC_CP_UTF8 {
                return None;
            }
            // A UTF-8 byte offset, never fewer than the UTF-16 units before it.
            let cursor = send(SCI_GETCURRENTPOS, 0, 0)?;
            let mut buffer = vec![0u16; cursor + 1];
            let copied = send(WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize)?;
            let text = String::from_utf16_lossy(&buffer[..copied.min(cursor)]);
            let mut end = cursor.min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            Some(text[..end].to_string())
        }
    }

    /// Reads at most `lines` lines (0 for all) up to the cursor. Fields without
    /// a readable cursor give their whole text.
    pub fn text_before_cursor(lines: usize) -> Option<String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(text) = scintilla_text_before_cursor() {
                log::debug!("Read the text before the cursor from Scintilla");
                let _ = sender.send(Some(text));
                return;
            }
            let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            let text = if initialized {
                read_focused(lines)
            } else {
                None
            };
            if initialized {
                unsafe { CoUninitialize() };
            }
            let _ = sender.send(text);
        });
        receiver.recv_timeout(READ_TIMEOUT).ok().flatten()
    }

    /// File name of the foreground window's program, without `.exe`.
    pub fn foreground_app_name() -> Option<String> {
        use windows::core::PWSTR;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };

        unsafe {
            let window = GetForegroundWindow();
            if window.0.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(window, Some(&mut pid));
            if pid == 0 {
                return None;
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buffer = [0u16; 260];
            let mut size = buffer.len() as u32;
            let read = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(process);
            read.ok()?;
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            std::path::Path::new(&path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXSelectedTextRangeAttribute,
        kAXValueAttribute, kAXValueTypeCFRange, AXUIElementCopyAttributeValue,
        AXUIElementCreateSystemWide, AXUIElementSetMessagingTimeout, AXValueGetValue, AXValueRef,
    };
    use core_foundation::base::{CFGetTypeID, CFRange, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::ptr;

    /// Seconds to wait for the target app, so a hung app can't stall the
    /// transcription.
    const MESSAGING_TIMEOUT: f32 = 0.5;
    const AX_FOCUSED_APPLICATION: &str = "AXFocusedApplication";
    const AX_TITLE: &str = "AXTitle";

    /// Returns an owned reference, or null when the attribute can't be read.
    unsafe fn copy_attribute(element: CFTypeRef, attribute: &str) -> CFTypeRef {
        let attribute = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let error = AXUIElementCopyAttributeValue(
            element as _,
            attribute.as_concrete_TypeRef(),
            &mut value,
        );
        if error != kAXErrorSuccess {
            return ptr::null();
        }
        value
    }

    /// Takes ownership of `value` and returns it as a string when it is one.
    unsafe fn take_string(value: CFTypeRef) -> Option<String> {
        if value.is_null() {
            return None;
        }
        if CFGetTypeID(value) != CFString::type_id() {
            CFRelease(value);
            return None;
        }
        Some(CFString::wrap_under_create_rule(value as CFStringRef).to_string())
    }

    /// Reads an attribute of the system-wide element, such as the focused
    /// field or app. The caller owns the result.
    unsafe fn copy_system_attribute(attribute: &str) -> CFTypeRef {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return ptr::null();
        }
        AXUIElementSetMessagingTimeout(system, MESSAGING_TIMEOUT);
        let value = copy_attribute(system as CFTypeRef, attribute);
        CFRelease(system as CFTypeRef);
        value
    }

    /// Cursor position in UTF-16 units, taken from the start of the selection.
    unsafe fn cursor_utf16_offset(element: CFTypeRef) -> Option<usize> {
        let range_value = copy_attribute(element, kAXSelectedTextRangeAttribute);
        if range_value.is_null() {
            return None;
        }
        let mut range = CFRange {
            location: 0,
            length: 0,
        };
        let read = AXValueGetValue(
            range_value as AXValueRef,
            kAXValueTypeCFRange,
            &mut range as *mut _ as *mut c_void,
        );
        CFRelease(range_value);
        (read && range.location >= 0).then_some(range.location as usize)
    }

    /// Reads the whole text up to the cursor; `limit_tail` trims it. Returns
    /// the whole field when the cursor position is unknown.
    pub fn text_before_cursor(_lines: usize) -> Option<String> {
        unsafe {
            let focused = copy_system_attribute(kAXFocusedUIElementAttribute);
            if focused.is_null() {
                return None;
            }
            let Some(text) = take_string(copy_attribute(focused, kAXValueAttribute)) else {
                CFRelease(focused);
                return None;
            };
            let cursor = cursor_utf16_offset(focused);
            CFRelease(focused);
            Some(match cursor {
                Some(offset) => {
                    let units: Vec<u16> = text.encode_utf16().collect();
                    String::from_utf16_lossy(&units[..offset.min(units.len())])
                }
                None => text,
            })
        }
    }

    pub fn foreground_app_name() -> Option<String> {
        unsafe {
            let app = copy_system_attribute(AX_FOCUSED_APPLICATION);
            if app.is_null() {
                return None;
            }
            let title = copy_attribute(app, AX_TITLE);
            CFRelease(app);
            take_string(title)
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    pub fn text_before_cursor(_lines: usize) -> Option<String> {
        None
    }

    pub fn foreground_app_name() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(paragraphs: usize, lines: usize, characters: usize) -> ContextLimits {
        ContextLimits {
            paragraphs,
            lines,
            characters,
        }
    }

    #[test]
    fn no_limits_keep_the_text_as_is() {
        assert_eq!(limit_tail("a\r\nb", limits(0, 0, 0)), "a\r\nb");
    }

    #[test]
    fn character_limit_counts_characters_not_bytes() {
        assert_eq!(limit_tail("hello world", limits(0, 0, 5)), "world");
        assert_eq!(limit_tail("áéíó", limits(0, 0, 2)), "íó");
        assert_eq!(limit_tail("hi", limits(0, 0, 50)), "hi");
    }

    #[test]
    fn line_limit_keeps_the_last_lines() {
        assert_eq!(limit_tail("l1\nl2\nl3\nl4", limits(0, 2, 0)), "l3\nl4");
        assert_eq!(limit_tail("l1\r\nl2\r\nl3", limits(0, 2, 0)), "l2\nl3");
        assert_eq!(limit_tail("l1\nl2", limits(0, 5, 0)), "l1\nl2");
    }

    #[test]
    fn paragraph_limit_treats_a_run_of_blank_lines_as_one_break() {
        assert_eq!(limit_tail("p1\n\np2\n\np3", limits(1, 0, 0)), "p3");
        assert_eq!(limit_tail("p1\n\np2\n\np3", limits(2, 0, 0)), "p2\n\np3");
        assert_eq!(limit_tail("p1\r\n\r\np2", limits(1, 0, 0)), "p2");
        assert_eq!(limit_tail("p1\n\n\np2", limits(1, 0, 0)), "p2");
        assert_eq!(limit_tail("p1\n\n\np2", limits(2, 0, 0)), "p1\n\n\np2");
        assert_eq!(limit_tail("l1\nl2", limits(1, 0, 0)), "l1\nl2");
    }

    #[test]
    fn the_strictest_limit_wins() {
        assert_eq!(limit_tail("aaaa\nbbbb\ncccc", limits(0, 2, 4)), "cccc");
    }
}
