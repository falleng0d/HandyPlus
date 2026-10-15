/// Drops trailing periods from the final text, so a dictated phrase can be
/// pasted mid-sentence.
pub fn strip_trailing_periods(text: &str) -> String {
    text.trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_trailing_periods;

    #[test]
    fn removes_only_trailing_periods() {
        assert_eq!(strip_trailing_periods("Hello there."), "Hello there");
        assert_eq!(strip_trailing_periods("Wait..."), "Wait");
        assert_eq!(strip_trailing_periods("e.g. this"), "e.g. this");
        assert_eq!(strip_trailing_periods("Done!"), "Done!");
    }
}
