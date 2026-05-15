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

use std::collections::HashMap;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser as CmarkParser, Tag, TagEnd};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// A structure-aware Markdown retrieval unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownChunk {
    /// Retrieval granularity for the chunk.
    pub record_kind: String,
    /// Stable chunk identifier derived from heading structure.
    pub chunk_id: String,
    /// One-based chunk ordinal in document order.
    pub chunk_index: u32,
    /// Display title for the retrieval unit.
    pub title: String,
    /// Full heading ancestry for the chunk.
    pub heading_path: Vec<String>,
    /// One-based starting line of the chunk.
    pub line_start: Option<u32>,
    /// One-based ending line of the chunk.
    pub line_end: Option<u32>,
    /// Text indexed for retrieval.
    pub content: String,
    /// Explicit fallback reason when chunking degrades to file-level retrieval.
    pub fallback_reason: Option<String>,
    /// Advisory lint summary.
    pub lint_summary: Option<String>,
    /// Advisory lint suggestions.
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
struct MarkdownHeading {
    level: u8,
    title: String,
    line_start: usize,
}

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
            Event::End(TagEnd::Heading(_)) if in_heading => {
                in_heading = false;
                let name = heading_text.trim().to_owned();
                if !name.is_empty() {
                    emit_heading(source, heading_start_byte, range.end, name, &mut symbols);
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
            Event::End(TagEnd::CodeBlock) if in_code => {
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

/// Chunk a Markdown document into stable retrieval units.
///
/// Falls back to a single file-level retrieval unit when the document lacks a
/// stable heading spine.
///
/// # Errors
///
/// Never errors; always returns `Ok(Vec<MarkdownChunk>)`.
#[allow(clippy::unnecessary_wraps)]
pub fn chunk_markdown_document(
    source: &str,
) -> Result<Vec<MarkdownChunk>, crate::errors::EngramError> {
    chunk_markdown_document_with_title_hint(source, None)
}

/// Chunk a Markdown document with an optional title hint for fallback advice.
///
/// # Errors
///
/// Never errors; always returns `Ok(Vec<MarkdownChunk>)`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn chunk_markdown_document_with_title_hint(
    source: &str,
    title_hint: Option<&str>,
) -> Result<Vec<MarkdownChunk>, crate::errors::EngramError> {
    let lines: Vec<&str> = source.lines().collect();
    let headings = collect_headings(&lines);
    let findings = lint_findings(&headings);
    let has_h1 = headings.iter().any(|heading| heading.level == 1);
    let stable_heading_spine = headings.first().is_some_and(|heading| heading.level == 1) && has_h1;

    if !stable_heading_spine {
        return Ok(vec![fallback_chunk(source, title_hint, &findings)]);
    }

    let mut heading_path: Vec<String> = Vec::new();
    let mut chunks: Vec<MarkdownChunk> = Vec::new();
    let mut chunk_id_counts: HashMap<String, u32> = HashMap::new();

    for (index, heading) in headings.iter().enumerate() {
        while heading_path.len() >= usize::from(heading.level) {
            heading_path.pop();
        }
        heading_path.push(heading.title.clone());

        let next_line_start = headings
            .get(index + 1)
            .map_or(lines.len() + 1, |next| next.line_start);
        let section_lines = &lines[heading.line_start - 1..next_line_start - 1];
        let content = section_lines.join("\n");

        if content.trim().is_empty() {
            continue;
        }

        let line_start = u32::try_from(heading.line_start).ok();
        let line_end = u32::try_from(next_line_start.saturating_sub(1)).ok();
        let chunk_index = u32::try_from(chunks.len() + 1).unwrap_or(u32::MAX);
        let chunk_id = next_chunk_id(&heading_path, &mut chunk_id_counts);

        chunks.push(MarkdownChunk {
            record_kind: "markdown_chunk".to_owned(),
            chunk_id,
            chunk_index,
            title: heading.title.clone(),
            heading_path: heading_path.clone(),
            line_start,
            line_end,
            content,
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
        });
    }

    if chunks.is_empty() {
        return Ok(vec![fallback_chunk(source, title_hint, &findings)]);
    }

    Ok(chunks)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn collect_headings(lines: &[&str]) -> Vec<MarkdownHeading> {
    let mut headings: Vec<MarkdownHeading> = Vec::new();
    let mut index = frontmatter_end_index(lines).unwrap_or(0);

    while index < lines.len() {
        if let Some((level, title)) = parse_atx_heading(lines[index]) {
            headings.push(MarkdownHeading {
                level,
                title,
                line_start: index + 1,
            });
            index += 1;
            continue;
        }

        if let Some((level, title)) = parse_setext_heading(lines, index) {
            headings.push(MarkdownHeading {
                level,
                title,
                line_start: index + 1,
            });
            index += 2;
            continue;
        }

        index += 1;
    }

    headings
}

fn frontmatter_end_index(lines: &[&str]) -> Option<usize> {
    if lines.first().is_none_or(|line| line.trim() != "---") {
        return None;
    }

    lines.iter().enumerate().skip(1).find_map(|(index, line)| {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            Some(index + 1)
        } else {
            None
        }
    })
}

fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let level = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&level) {
        return None;
    }

    let after_hashes = trimmed[level..].trim();
    if after_hashes.is_empty() {
        return None;
    }

    let title = after_hashes.trim_end_matches('#').trim().to_owned();
    if title.is_empty() {
        return None;
    }

    Some((u8::try_from(level).ok()?, title))
}

fn parse_setext_heading(lines: &[&str], index: usize) -> Option<(u8, String)> {
    let current = lines.get(index)?.trim();
    let underline = lines.get(index + 1)?.trim();
    if current.is_empty() || underline.is_empty() {
        return None;
    }

    let level = if underline.chars().all(|character| character == '=') {
        1
    } else if underline.chars().all(|character| character == '-') {
        2
    } else {
        return None;
    };

    Some((level, current.to_owned()))
}

fn lint_findings(headings: &[MarkdownHeading]) -> Vec<String> {
    let mut findings: Vec<String> = Vec::new();
    let has_h1 = headings.iter().any(|heading| heading.level == 1);
    if !has_h1 {
        findings.push("missing_h1".to_owned());
    }
    if headings.first().is_some_and(|heading| heading.level != 1) {
        findings.push("missing_leading_h1".to_owned());
    }
    findings
}

fn fallback_chunk(source: &str, title_hint: Option<&str>, findings: &[String]) -> MarkdownChunk {
    let fallback_title = title_hint.unwrap_or("Notes");
    let line_end = u32::try_from(source.lines().count()).ok();
    let lint_summary = (!findings.is_empty()).then(|| findings.join(", "));
    let mut suggestions = Vec::new();
    if findings.iter().any(|finding| finding == "missing_h1") {
        suggestions.push(format!("# {fallback_title}"));
    }

    MarkdownChunk {
        record_kind: "file".to_owned(),
        chunk_id: "file".to_owned(),
        chunk_index: 1,
        title: fallback_title.to_owned(),
        heading_path: Vec::new(),
        line_start: Some(1),
        line_end,
        content: source.to_owned(),
        fallback_reason: Some("missing_heading_structure".to_owned()),
        lint_summary,
        suggestions,
    }
}

fn next_chunk_id(heading_path: &[String], counts: &mut HashMap<String, u32>) -> String {
    let base = slugify_heading_path(heading_path);
    let base = if base.is_empty() {
        let heading_key = heading_path.join(" > ");
        format!("section-{}", &super::sha256_hex(&heading_key)[..8])
    } else {
        base
    };
    let occurrence = counts.entry(base.clone()).or_insert(0);
    *occurrence += 1;
    if *occurrence == 1 {
        base
    } else {
        format!("{base}--{occurrence}")
    }
}

fn slugify_heading_path(heading_path: &[String]) -> String {
    heading_path
        .iter()
        .map(|segment| {
            let mut slug = String::new();
            let mut previous_dash = false;
            for character in segment.chars().flat_map(char::to_lowercase) {
                if character.is_ascii_alphanumeric() {
                    slug.push(character);
                    previous_dash = false;
                } else if !previous_dash {
                    slug.push('-');
                    previous_dash = true;
                }
            }
            slug.trim_matches('-').to_owned()
        })
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

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
