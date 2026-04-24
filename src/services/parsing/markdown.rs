//! Markdown document parser (T030.003-C).
//!
//! Extracts structural elements from Markdown source files using
//! [`pulldown_cmark`]:
//!
//! - ATX and setext headings (all levels) → [`super::ExtractedClass`]
//!   The `name` field carries the heading text; `line_start` is the
//!   1-based line of the `#` marker or the first text line.
//!
//! - Fenced code blocks → [`super::ExtractedFunction`]
//!   The `signature` field carries the language hint (e.g. `"rust"`);
//!   `body` carries the raw code content between the fences.
//!   Indented code blocks are extracted the same way with an empty
//!   `signature`.
//!
//! - Inline and reference links → [`super::ExtractedEdge::Imports`]
//!   The `import_path` field carries the destination URL.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser as CmarkParser, Tag, TagEnd};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// Parse a Markdown document and extract headings, code blocks, and links.
///
/// This function never fails: pulldown-cmark is lenient and will produce
/// a best-effort result for any byte sequence. The `Err` variant exists
/// only to satisfy the common `Result<ParseResult, EngramError>` contract.
///
/// # Errors
///
/// Never errors; always returns `Ok(ParseResult)`.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn parse_markdown_source(
    source: &str,
) -> Result<ParseResult, crate::errors::EngramError> {
    let mut symbols: Vec<ExtractedSymbol> = Vec::new();
    let mut edges: Vec<ExtractedEdge> = Vec::new();

    // ── Heading state ──────────────────────────────────────────────────────
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut heading_start_byte: usize = 0;

    // ── Code-block state ───────────────────────────────────────────────────
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_body = String::new();
    let mut code_start_byte: usize = 0;
    let mut block_counter: u32 = 0;

    let parser = CmarkParser::new_ext(source, Options::empty()).into_offset_iter();

    for (event, range) in parser {
        match event {
            // ── Heading start ──────────────────────────────────────────────
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading_text.clear();
                heading_start_byte = range.start;
            }

            // ── Heading end ────────────────────────────────────────────────
            Event::End(TagEnd::Heading(_)) => {
                if in_heading {
                    in_heading = false;
                    let name = heading_text.trim().to_owned();
                    if !name.is_empty() {
                        emit_heading(source, heading_start_byte, range.end, name, &mut symbols);
                    }
                }
            }

            // ── Code-block start ───────────────────────────────────────────
            Event::Start(Tag::CodeBlock(ref kind)) => {
                in_code = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code_body.clear();
                code_start_byte = range.start;
            }

            // ── Code-block end ─────────────────────────────────────────────
            Event::End(TagEnd::CodeBlock) => {
                if in_code {
                    in_code = false;
                    block_counter += 1;
                    emit_code_block(
                        source,
                        code_start_byte,
                        range.end,
                        &code_lang,
                        &code_body,
                        block_counter,
                        &mut symbols,
                    );
                }
            }

            // ── Text: accumulate into active container ─────────────────────
            Event::Text(ref text) => {
                if in_heading {
                    heading_text.push_str(text);
                } else if in_code {
                    code_body.push_str(text);
                }
            }

            // ── Links → Imports edge ───────────────────────────────────────
            Event::Start(Tag::Link { ref dest_url, .. }) => {
                let url = dest_url.to_string();
                if !url.is_empty() {
                    edges.push(ExtractedEdge::Imports { import_path: url });
                }
            }

            _ => {}
        }
    }

    Ok(ParseResult { symbols, edges })
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Convert a byte offset within `source` to a 1-based line number.
#[allow(clippy::naive_bytecount)]
fn byte_offset_to_line(source: &str, byte_offset: usize) -> u32 {
    let capped = byte_offset.min(source.len());
    let newlines = source.as_bytes()[..capped]
        .iter()
        .filter(|&&b| b == b'\n')
        .count();
    #[allow(clippy::cast_possible_truncation)]
    let line = (newlines + 1) as u32;
    line
}

/// Emit an [`ExtractedSymbol::Class`] for a Markdown heading.
fn emit_heading(
    source: &str,
    start_byte: usize,
    end_byte: usize,
    name: String,
    symbols: &mut Vec<ExtractedSymbol>,
) {
    let line_start = byte_offset_to_line(source, start_byte);
    let end_capped = end_byte.min(source.len());
    let line_end = if end_capped > 0 {
        byte_offset_to_line(source, end_capped - 1)
    } else {
        line_start
    };
    let body = source[start_byte..end_capped].to_owned();
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;
    symbols.push(ExtractedSymbol::Class(ExtractedClass {
        name,
        line_start,
        line_end,
        docstring: None,
        body,
        body_hash,
        token_count,
    }));
}

/// Emit an [`ExtractedSymbol::Function`] for a Markdown fenced or indented
/// code block.
///
/// Symbol name: `"{lang}_block_{n}"` when a language hint is present,
/// `"block_{n}"` otherwise. `signature` holds the raw language hint string.
fn emit_code_block(
    source: &str,
    start_byte: usize,
    end_byte: usize,
    lang: &str,
    body: &str,
    counter: u32,
    symbols: &mut Vec<ExtractedSymbol>,
) {
    let line_start = byte_offset_to_line(source, start_byte);
    let end_capped = end_byte.min(source.len());
    let line_end = if end_capped > 0 {
        byte_offset_to_line(source, end_capped - 1)
    } else {
        line_start
    };
    let name = if lang.is_empty() {
        format!("block_{counter}")
    } else {
        format!("{lang}_block_{counter}")
    };
    let body_hash = super::sha256_hex(body);
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;
    symbols.push(ExtractedSymbol::Function(ExtractedFunction {
        name,
        line_start,
        line_end,
        signature: lang.to_owned(),
        docstring: None,
        body: body.to_owned(),
        body_hash,
        token_count,
    }));
}
