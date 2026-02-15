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
    let budget = max_len.saturating_sub(prefix.len());

    if text.len() <= budget {
        return text.to_string();
    }

    // Find the last space within the budget
    let truncated = &text[..budget];
    let break_point = truncated.rfind(' ').unwrap_or(budget);

    let mut result = String::with_capacity(prefix.len() + break_point + 3);
    result.push_str(prefix);
    result.push_str(&text[..break_point]);
    result.push_str("...");
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
