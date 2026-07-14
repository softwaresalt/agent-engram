//! DAX reference extraction for TMDL measure and calculated-column expressions.
//!
//! [`extract_dax_references`] is a safe, dependency-free, string- and
//! comment-aware lexer that surfaces the columns, bracketed references, and
//! function calls a DAX expression references. It is the **stable seam** a
//! future tree-sitter DAX grammar can swap in behind without changing any
//! downstream consumer: the signature and the [`DaxReferences`] shape are the
//! contract.
//!
//! The lexer is a single left-to-right state machine over the expression's
//! characters. Only in the `Normal` state are tokens recognized; brackets and
//! quotes appearing inside string literals (`"..."`) or comments (`// ...`,
//! `/* ... */`) are ignored. Unresolved or malformed tokens are dropped and, for
//! unterminated constructs, surfaced through [`DaxReferences::diagnostics`]
//! rather than being silently misattributed.

/// A column reference discovered in a DAX expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaxColumnRef {
    /// Owning table when the reference is qualified (`'Table'[Col]` or
    /// `Table[Col]`). `None` for a bare bracket reference (`[Col]`) whose owner
    /// is unknown until resolved against a model schema.
    pub table: Option<String>,
    /// Column (or bracketed member) name, captured verbatim between the
    /// brackets.
    pub column: String,
}

/// A lexer diagnostic describing an unterminated DAX construct.
///
/// This is the **syntax-validation seam** the Tier-1 `dax.malformed_ref` lint
/// rule consumes: well-formed input yields an empty
/// [`DaxReferences::diagnostics`], while a truncated construct such as
/// `'Sales[Amount]` is surfaced here rather than dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaxDiagnostic {
    /// An unterminated string literal (`"...` with no closing quote).
    UnterminatedString,
    /// An unterminated quoted table identifier (`'...` with no closing quote).
    UnterminatedQuotedIdentifier,
    /// An unterminated block comment (`/* ...` with no closing `*/`).
    UnterminatedBlockComment,
    /// An unterminated bracketed reference (`[...` with no closing `]`).
    UnterminatedBracket,
}

/// The references extracted from a DAX expression.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DaxReferences {
    /// Column references, both qualified (`Table[Col]`) and unqualified
    /// (`[Col]`, with `table: None`).
    pub columns: Vec<DaxColumnRef>,
    /// Bare bracket references (`[Name]`) whose kind (column vs. measure) is
    /// unknown until resolved against a model schema. An unqualified bracket is
    /// recorded both here and as a provisional `columns` entry with
    /// `table: None`.
    pub bracket_refs: Vec<String>,
    /// Function names invoked (an identifier immediately followed by `(`).
    pub functions: Vec<String>,
    /// Diagnostics for unterminated constructs surfaced by the lexer.
    pub diagnostics: Vec<DaxDiagnostic>,
}

/// Extract the column, bracket, and function references from a DAX expression.
///
/// The lexer is string- and comment-aware: brackets and quotes inside `"..."`
/// literals or `//` / `/* */` comments are ignored. Bare identifiers that are
/// neither immediately followed by `[` (a table qualifier) nor `(` (a function
/// call) — for example `VAR`/`RETURN` locals — are ignored. Unterminated
/// constructs are recorded in [`DaxReferences::diagnostics`].
#[must_use]
pub fn extract_dax_references(expr: &str) -> DaxReferences {
    let chars: Vec<char> = expr.chars().collect();
    let n = chars.len();
    let mut refs = DaxReferences::default();
    let mut i = 0usize;

    while i < n {
        let c = chars[i];
        i = if c.is_whitespace() {
            i + 1
        } else if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            scan_line_comment(&chars, i)
        } else if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            scan_block_comment(&chars, i, &mut refs)
        } else if c == '"' {
            scan_string(&chars, i, &mut refs)
        } else if c == '\'' {
            handle_quoted_table(&chars, i, &mut refs)
        } else if c == '[' {
            handle_unqualified_bracket(&chars, i, &mut refs)
        } else if is_ident_start(c) {
            handle_identifier(&chars, i, &mut refs)
        } else {
            // Operators, digits, parentheses, commas: not references.
            i + 1
        };
    }

    refs
}

/// Skip a `// ... <newline>` line comment; returns the index of the newline (or
/// end of input).
fn scan_line_comment(chars: &[char], start: usize) -> usize {
    let n = chars.len();
    let mut i = start + 2;
    while i < n && chars[i] != '\n' {
        i += 1;
    }
    i
}

/// Skip a `/* ... */` block comment, recording an
/// [`DaxDiagnostic::UnterminatedBlockComment`] when it never closes.
fn scan_block_comment(chars: &[char], start: usize, refs: &mut DaxReferences) -> usize {
    let n = chars.len();
    let mut i = start + 2;
    while i < n {
        if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
            return i + 2;
        }
        i += 1;
    }
    refs.diagnostics
        .push(DaxDiagnostic::UnterminatedBlockComment);
    i
}

/// Skip a `"..."` string literal (with `""` as an escaped quote), recording an
/// [`DaxDiagnostic::UnterminatedString`] when it never closes.
fn scan_string(chars: &[char], start: usize, refs: &mut DaxReferences) -> usize {
    let n = chars.len();
    let mut i = start + 1;
    while i < n {
        if chars[i] == '"' {
            if i + 1 < n && chars[i + 1] == '"' {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    refs.diagnostics.push(DaxDiagnostic::UnterminatedString);
    i
}

/// Handle a `'Table'` quoted identifier and, when immediately followed by a
/// `[column]`, record the qualified reference.
fn handle_quoted_table(chars: &[char], start: usize, refs: &mut DaxReferences) -> usize {
    let n = chars.len();
    let (name, closed, next) = read_quoted(chars, start);
    if !closed {
        refs.diagnostics
            .push(DaxDiagnostic::UnterminatedQuotedIdentifier);
        return next;
    }
    // A quoted table token is a reference only when immediately followed by a
    // `[column]`; a bare `'Table'` (e.g. `ALL('Table')`) is ignored.
    if next < n && chars[next] == '[' {
        let (column, bclosed, bnext) = read_bracket(chars, next);
        if bclosed {
            refs.columns.push(DaxColumnRef {
                table: Some(name),
                column,
            });
        } else {
            refs.diagnostics.push(DaxDiagnostic::UnterminatedBracket);
        }
        return bnext;
    }
    next
}

/// Handle an unqualified `[Name]` bracket reference, recording it in both
/// `columns` (with `table: None`) and `bracket_refs`.
fn handle_unqualified_bracket(chars: &[char], start: usize, refs: &mut DaxReferences) -> usize {
    let (name, closed, next) = read_bracket(chars, start);
    if closed {
        refs.columns.push(DaxColumnRef {
            table: None,
            column: name.clone(),
        });
        refs.bracket_refs.push(name);
    } else {
        refs.diagnostics.push(DaxDiagnostic::UnterminatedBracket);
    }
    next
}

/// Handle a bare identifier: a `Table[col]` qualifier, a `Func(` call, or an
/// ignored local/keyword (e.g. `VAR`/`RETURN`).
fn handle_identifier(chars: &[char], start: usize, refs: &mut DaxReferences) -> usize {
    let n = chars.len();
    let mut i = start;
    while i < n && is_ident_part(chars[i]) {
        i += 1;
    }
    // Absorb dotted continuations so standard dotted function names such as
    // `STDEV.P` / `NORM.S.DIST` are captured whole rather than truncated to the
    // suffix. A dot is only consumed when another identifier character follows,
    // so a trailing dot or a `.[` member access is left for the main scanner.
    while i + 1 < n && chars[i] == '.' && is_ident_part(chars[i + 1]) {
        i += 1;
        while i < n && is_ident_part(chars[i]) {
            i += 1;
        }
    }
    let ident: String = chars[start..i].iter().collect();
    // Qualified column/measure reference: the bracket must immediately follow
    // the identifier. DAX does not allow whitespace before `[`, and skipping it
    // would wrongly bind a bracket to a preceding keyword (e.g. turn
    // `RETURN [Measure]` into a bogus `RETURN[Measure]` qualified reference).
    if i < n && chars[i] == '[' {
        let (column, bclosed, bnext) = read_bracket(chars, i);
        if bclosed {
            refs.columns.push(DaxColumnRef {
                table: Some(ident),
                column,
            });
        } else {
            refs.diagnostics.push(DaxDiagnostic::UnterminatedBracket);
        }
        return bnext;
    }
    // Function call: DAX tolerates inter-token whitespace before the argument
    // parenthesis (e.g. `SUM ( … )`, `EARLIER (`), so skip spaces before the
    // `(` check. Only a `(` (never a `[`) may follow across whitespace.
    let mut j = i;
    while j < n && chars[j].is_whitespace() {
        j += 1;
    }
    if j < n && chars[j] == '(' {
        refs.functions.push(ident);
    }
    i
}

/// Read a quoted table identifier starting at the opening `'` at `start`.
///
/// Returns the unescaped name (with `''` collapsed to `'`), whether a closing
/// quote was found, and the index just past the construct.
fn read_quoted(chars: &[char], start: usize) -> (String, bool, usize) {
    let n = chars.len();
    let mut i = start + 1;
    let mut name = String::new();
    while i < n {
        if chars[i] == '\'' {
            if i + 1 < n && chars[i + 1] == '\'' {
                name.push('\'');
                i += 2;
                continue;
            }
            return (name, true, i + 1);
        }
        name.push(chars[i]);
        i += 1;
    }
    (name, false, i)
}

/// Read a bracketed name starting at the opening `[` at `start`.
///
/// Returns the unescaped name (with `]]` collapsed to `]`), whether a closing
/// bracket was found, and the index just past the construct.
fn read_bracket(chars: &[char], start: usize) -> (String, bool, usize) {
    let n = chars.len();
    let mut i = start + 1;
    let mut name = String::new();
    while i < n {
        if chars[i] == ']' {
            if i + 1 < n && chars[i + 1] == ']' {
                name.push(']');
                i += 2;
                continue;
            }
            return (name, true, i + 1);
        }
        name.push(chars[i]);
        i += 1;
    }
    (name, false, i)
}

/// Whether `c` can start a bare (unquoted) DAX identifier.
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Whether `c` can continue a bare (unquoted) DAX identifier.
fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(table: Option<&str>, column: &str) -> DaxColumnRef {
        DaxColumnRef {
            table: table.map(std::string::ToString::to_string),
            column: column.to_string(),
        }
    }

    #[test]
    fn qualified_refs_bare_and_quoted_tables() {
        let refs = extract_dax_references(r"SUM(Sales[Amount]) + SUM('Date Table'[Year])");
        assert!(refs.columns.contains(&col(Some("Sales"), "Amount")));
        assert!(refs.columns.contains(&col(Some("Date Table"), "Year")));
        assert!(refs.functions.contains(&"SUM".to_string()));
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn unqualified_bracket_ref_goes_to_both_columns_and_bracket_refs() {
        let refs = extract_dax_references(r"[Total Sales] * 2");
        assert!(refs.columns.contains(&col(None, "Total Sales")));
        assert!(refs.bracket_refs.contains(&"Total Sales".to_string()));
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn function_calls_are_captured() {
        let refs =
            extract_dax_references(r"CALCULATE(DIVIDE([A], [B]), FILTER(Sales, Sales[Qty] > 0))");
        for f in ["CALCULATE", "DIVIDE", "FILTER"] {
            assert!(
                refs.functions.contains(&f.to_string()),
                "missing function {f}"
            );
        }
        // FILTER's first arg `Sales` is a bare table token (no bracket) -> ignored.
        assert!(refs.columns.contains(&col(Some("Sales"), "Qty")));
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn var_return_locals_are_ignored() {
        let refs = extract_dax_references(r"VAR x = SUM(Sales[Amount]) VAR total = x RETURN total");
        // Locals `x`, `total`, keywords VAR/RETURN are neither table-qualified
        // nor function calls -> not references.
        assert_eq!(refs.columns, vec![col(Some("Sales"), "Amount")]);
        assert!(refs.bracket_refs.is_empty());
        assert!(!refs.functions.contains(&"VAR".to_string()));
        assert!(!refs.functions.contains(&"RETURN".to_string()));
    }

    #[test]
    fn dotted_function_names_are_captured_whole() {
        // Standard dotted DAX functions must be reported whole, not truncated to
        // the suffix (`P` / `DIST`).
        let refs = extract_dax_references(r"STDEV.P(Sales[Amount]) + NORM.S.DIST(Sales[Z], 0, 1)");
        assert!(
            refs.functions.contains(&"STDEV.P".to_string()),
            "expected STDEV.P; got {:?}",
            refs.functions
        );
        assert!(
            refs.functions.contains(&"NORM.S.DIST".to_string()),
            "expected NORM.S.DIST; got {:?}",
            refs.functions
        );
        assert!(refs.columns.contains(&col(Some("Sales"), "Amount")));
        assert!(refs.columns.contains(&col(Some("Sales"), "Z")));
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn whitespace_before_argument_paren_still_captures_function() {
        // Valid TMDL uses `SUM ( … )` / `EARLIER (`; the space before `(` must
        // not hide the function name.
        let refs = extract_dax_references(r"SUM ( Sales[Amount] ) - EARLIER ( Sales[Amount] )");
        assert!(
            refs.functions.contains(&"SUM".to_string()),
            "expected SUM; got {:?}",
            refs.functions
        );
        assert!(
            refs.functions.contains(&"EARLIER".to_string()),
            "expected EARLIER; got {:?}",
            refs.functions
        );
        assert!(refs.columns.contains(&col(Some("Sales"), "Amount")));
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn whitespace_before_bracket_does_not_bind_to_keyword() {
        // A bracket must bind immediately, so `RETURN [Total]` stays an
        // unqualified `[Total]` rather than a bogus `RETURN[Total]`.
        let refs = extract_dax_references(r"VAR x = 1 RETURN [Total]");
        assert!(
            refs.columns.contains(&col(None, "Total")),
            "unqualified [Total] must not bind to RETURN; got {:?}",
            refs.columns
        );
        assert!(!refs.functions.contains(&"RETURN".to_string()));
    }

    #[test]
    fn brackets_and_quotes_inside_strings_are_ignored() {
        let refs = extract_dax_references(r#"IF(Sales[Region] = "North[X] 'Y'[Z]", 1, 0)"#);
        // Only the real ref outside the string literal is captured.
        assert_eq!(refs.columns, vec![col(Some("Sales"), "Region")]);
        assert!(refs.bracket_refs.is_empty());
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn brackets_inside_line_and_block_comments_are_ignored() {
        let refs =
            extract_dax_references("// [Ghost] comment\nSUM(Sales[Amount]) /* 'T'[C] block [X] */");
        assert_eq!(refs.columns, vec![col(Some("Sales"), "Amount")]);
        assert!(refs.bracket_refs.is_empty());
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn escaped_double_quote_in_string_does_not_end_string() {
        // The "" is an escaped quote; the [Hidden] stays inside the literal.
        let refs = extract_dax_references(r#"CONCATENATE("a""b [Hidden]", Sales[Real])"#);
        assert_eq!(refs.columns, vec![col(Some("Sales"), "Real")]);
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn escaped_single_quote_in_quoted_table_name() {
        // '' inside a quoted identifier is a literal apostrophe.
        let refs = extract_dax_references(r"SUM('Bob''s Table'[Value])");
        assert_eq!(refs.columns, vec![col(Some("Bob's Table"), "Value")]);
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn escaped_closing_bracket_in_column_name() {
        // ]] inside a bracket is a literal `]`; the name must not truncate.
        let refs = extract_dax_references(r"Sales[Amount ]] USD]");
        assert_eq!(refs.columns, vec![col(Some("Sales"), "Amount ] USD")]);
        assert!(refs.diagnostics.is_empty());
    }

    #[test]
    fn nested_whitespace_inside_brackets_and_quotes_preserved() {
        let refs = extract_dax_references(r"'My  Table'[Col  Name]");
        assert_eq!(refs.columns, vec![col(Some("My  Table"), "Col  Name")]);
    }

    #[test]
    fn unterminated_string_is_diagnosed() {
        let refs = extract_dax_references(r#"Sales[Amount] & "oops"#);
        assert!(
            refs.diagnostics
                .contains(&DaxDiagnostic::UnterminatedString)
        );
        // The valid ref before the malformed construct is still captured.
        assert!(refs.columns.contains(&col(Some("Sales"), "Amount")));
    }

    #[test]
    fn unterminated_quoted_identifier_is_diagnosed() {
        // The canonical truncated-ref example `'Sales[Amount]` must be flagged.
        let refs = extract_dax_references(r"'Sales[Amount]");
        assert!(
            refs.diagnostics
                .contains(&DaxDiagnostic::UnterminatedQuotedIdentifier)
        );
        // Nothing is misattributed as a resolved ref.
        assert!(refs.columns.is_empty());
    }

    #[test]
    fn unterminated_bracket_is_diagnosed() {
        let refs = extract_dax_references(r"Sales[Amount");
        assert!(
            refs.diagnostics
                .contains(&DaxDiagnostic::UnterminatedBracket)
        );
        assert!(refs.columns.is_empty());
    }

    #[test]
    fn unterminated_block_comment_is_diagnosed() {
        let refs = extract_dax_references(r"SUM(Sales[Amount]) /* dangling");
        assert!(
            refs.diagnostics
                .contains(&DaxDiagnostic::UnterminatedBlockComment)
        );
        // The valid ref before the dangling comment is still captured.
        assert!(refs.columns.contains(&col(Some("Sales"), "Amount")));
    }

    #[test]
    fn well_formed_input_yields_no_diagnostics() {
        let refs =
            extract_dax_references(r"CALCULATE(SUM('Fact Sales'[Amount]), 'Date'[Year] = 2024)");
        assert!(refs.diagnostics.is_empty());
    }
}
