//! YAML frontmatter parser for markdown files.
//!
//! Provides [`parse`] which splits a markdown document into its optional
//! YAML frontmatter metadata block and the remaining body text.
//! Backed by `serde_yaml` for YAML deserialization.

/// A parsed markdown document with optional YAML frontmatter.
///
/// The `metadata` field is `None` when no frontmatter delimiter (`---`)
/// is present in the input, or when the YAML block is malformed.
/// The `body` field contains all text after the closing `---` delimiter,
/// or the full input when no delimiter is found.
#[derive(Debug, Clone)]
pub struct FrontmatterDocument {
    /// Parsed YAML mapping, or `None` when absent or malformed.
    pub metadata: Option<serde_yaml::Mapping>,
    /// Markdown body text (after the closing `---` delimiter, or full input).
    pub body: String,
}

/// Parse a markdown string into frontmatter metadata and body text.
///
/// Recognises documents that begin with a `---` line on the first line.
/// The metadata block extends to the next `---` line; everything after that
/// is the body. When the opening `---` is absent or no closing `---` is
/// found, `metadata` is `None` and `body` is the complete input.  When both
/// delimiters are present but the YAML is malformed, `metadata` is `None`
/// and `body` contains only the text after the closing `---`.
///
/// # Examples
///
/// ```
/// let doc = engram::services::parsing::frontmatter::parse(
///     "---\nid: 001-T\ntitle: My task\n---\n\n## Body\n",
/// );
/// assert!(doc.metadata.is_some());
/// assert!(doc.body.contains("Body"));
/// ```
#[must_use]
pub fn parse(input: &str) -> FrontmatterDocument {
    let (yaml_block, body) = split_frontmatter(input);
    let metadata =
        yaml_block.and_then(|block| serde_yaml::from_str::<serde_yaml::Mapping>(&block).ok());
    FrontmatterDocument { metadata, body }
}

/// Split `input` into an optional YAML block and a body string.
///
/// Returns `(Some(yaml), body)` when the document starts with `---` and a
/// closing `---` is found on a subsequent line.  Returns `(None, input)` in
/// all other cases.
fn split_frontmatter(input: &str) -> (Option<String>, String) {
    let mut lines = input.lines();

    // First line must be exactly "---".
    match lines.next() {
        Some("---") => {}
        _ => return (None, input.to_string()),
    }

    // Collect YAML lines until we see the closing "---".
    let mut yaml_lines: Vec<&str> = Vec::new();
    let mut rest_lines: Vec<&str> = Vec::new();
    let mut found_close = false;

    for line in lines {
        if found_close {
            rest_lines.push(line);
        } else if line == "---" {
            found_close = true;
        } else {
            yaml_lines.push(line);
        }
    }

    if !found_close {
        return (None, input.to_string());
    }

    let yaml_block = yaml_lines.join("\n");
    let body = rest_lines.join("\n");
    (Some(yaml_block), body)
}
