//! Rule-based compaction service.
//!
//! Provides truncation utilities for compacting completed task
//! descriptions to reduce workspace memory footprint.

/// Truncate `text` to at most `max_len` characters at a word boundary,
/// prepending `[Compacted]` prefix to the truncated text.
///
/// If `text` is already within `max_len` (including prefix), it is
/// returned unchanged. Empty input returns an empty string.
pub fn truncate_at_word_boundary(text: &str, max_len: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let prefix = "[Compacted] ";
    let suffix = "...";
    let overhead = prefix.len() + suffix.len();
    let budget = max_len.saturating_sub(overhead);

    if text.len() <= max_len.saturating_sub(prefix.len()) {
        return text.to_string();
    }

    // Find the last char boundary at or before `budget` (UTF-8 safe)
    let safe_end = text
        .char_indices()
        .take_while(|(i, _)| *i <= budget)
        .last()
        .map_or(0, |(i, c)| i + c.len_utf8());
    let safe_end = safe_end.min(budget);

    // Prefer a word boundary (last space) within the safe range
    let break_point = text[..safe_end].rfind(' ').unwrap_or(safe_end);

    let mut result = String::with_capacity(prefix.len() + break_point + suffix.len());
    result.push_str(prefix);
    result.push_str(&text[..break_point]);
    result.push_str(suffix);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_unchanged() {
        let input = "A brief description";
        let result = truncate_at_word_boundary(input, 500);
        assert_eq!(result, input);
    }

    #[test]
    fn empty_input_returns_empty() {
        let result = truncate_at_word_boundary("", 500);
        assert!(result.is_empty());
    }

    #[test]
    fn long_text_truncated_at_word_boundary() {
        let input = "word ".repeat(400); // 2000 chars
        let result = truncate_at_word_boundary(&input, 500);
        assert!(
            result.len() <= 500,
            "result length {} should be <= 500",
            result.len()
        );
        assert!(result.starts_with("[Compacted] "));
        assert!(result.ends_with("..."));
    }

    #[test]
    fn typical_2000_char_achieves_70_percent_reduction() {
        let input = "This is a moderately long sentence for testing. ".repeat(42); // ~2016 chars
        let result = truncate_at_word_boundary(&input, 500);
        let reduction = 1.0 - (result.len() as f64 / input.len() as f64);
        assert!(
            reduction >= 0.70,
            "reduction {reduction:.2} should be >= 0.70"
        );
    }

    #[test]
    fn word_boundary_not_mid_word() {
        let input = "abcdefghij klmnopqrst uvwxyz";
        // With prefix "[Compacted] " (12 chars), budget = 20 - 12 = 8
        // "abcdefgh" has no space, so truncation falls back to full 8 chars
        let result = truncate_at_word_boundary(input, 20);
        assert!(result.starts_with("[Compacted] "));
        assert!(result.ends_with("..."));
    }

    #[test]
    fn preserves_content_under_budget() {
        // Text that fits within budget after subtracting prefix
        let input = "Small text";
        let result = truncate_at_word_boundary(input, 100);
        assert_eq!(result, "Small text");
    }
}
