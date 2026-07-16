//! A5 — generic-parameter normalisation for canonical names.
//!
//! Strips generic arguments so a definition spelling and a call spelling of the
//! same item converge on one canonical form, applied symmetrically at indexing
//! (A6) and resolution (Unit B):
//!
//! - `Type<T>::method`, `Type::<T>::method` (turbofish), `Type<'a>::method`,
//!   `Type<A, B>::method` → `Type::method`;
//! - nested generics (`Vec<HashMap<K, V>>`) collapse fully;
//! - distinct types stay distinct; a leading `::` is preserved.
//!
//! `Foo<T>` and `Foo` converge (same method surface — D12); `Foo` and `Bar`
//! never do.

/// Remove all generic argument groups (`<…>`, including turbofish `::<…>`) from a
/// type or path spelling, yielding its canonical, generic-free form — or `None`
/// (fail-closed) when the angle-bracket nesting is unbalanced.
///
/// The naive depth counter cannot safely elide `<…>` groups that themselves
/// contain a bare `>` token: a return arrow (`Fn() -> C`) or a const-generic
/// comparison (`<{ N > 0 }>`) closes the group early, so the remaining text
/// would leak into — or truncate — the canonical name (`Widget<Fn() -> C>::method`
/// → the garbage `Widget C::method`). Such spellings drive the running depth
/// below zero (an extra `>`) or leave it above zero at the end (an unclosed
/// `<`); both now yield `None`. An unresolved identity is safer than a wrong one
/// (D4): the caller stores an empty `canonical_path`, never a false match target.
#[must_use]
pub fn normalize_generics(type_expr: &str) -> Option<String> {
    // Fold turbofish `::<` into `<` so the separator does not survive stripping.
    let pre = type_expr.replace("::<", "<");
    let mut out = String::with_capacity(pre.len());
    let mut depth: u32 = 0;
    for ch in pre.chars() {
        match ch {
            '<' => depth += 1,
            // An unbalanced `>` (more closers than openers) means the spelling is
            // not a clean type path — fail closed rather than emit garbage.
            '>' => depth = depth.checked_sub(1)?,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    if depth != 0 {
        return None; // unclosed `<…>` group
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_single_type_param() {
        assert_eq!(
            normalize_generics("Type<T>::method").as_deref(),
            Some("Type::method")
        );
    }

    #[test]
    fn strips_turbofish() {
        assert_eq!(
            normalize_generics("Type::<T>::method").as_deref(),
            Some("Type::method")
        );
    }

    #[test]
    fn strips_lifetime() {
        assert_eq!(
            normalize_generics("Type<'a>::method").as_deref(),
            Some("Type::method")
        );
    }

    #[test]
    fn strips_multiple_args() {
        assert_eq!(
            normalize_generics("Type<A, B>::method").as_deref(),
            Some("Type::method")
        );
    }

    #[test]
    fn strips_on_path_tail_type() {
        assert_eq!(
            normalize_generics("a::b::Widget<T>").as_deref(),
            Some("a::b::Widget")
        );
    }

    #[test]
    fn strips_nested_generics() {
        assert_eq!(
            normalize_generics("Vec<HashMap<K, V>>").as_deref(),
            Some("Vec")
        );
    }

    #[test]
    fn leaves_plain_paths_untouched() {
        assert_eq!(normalize_generics("Foo").as_deref(), Some("Foo"));
        assert_eq!(normalize_generics("Foo::bar").as_deref(), Some("Foo::bar"));
    }

    #[test]
    fn preserves_leading_colon() {
        assert_eq!(
            normalize_generics("::std::mem::swap").as_deref(),
            Some("::std::mem::swap")
        );
    }

    #[test]
    fn generic_variants_converge() {
        let a = normalize_generics("Foo<u8>::get");
        let b = normalize_generics("Foo<u16>::get");
        let c = normalize_generics("Foo::<u8>::get");
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a.as_deref(), Some("Foo::get"));
    }

    #[test]
    fn foo_and_generic_foo_converge() {
        assert_eq!(normalize_generics("Foo<T>"), normalize_generics("Foo"));
    }

    #[test]
    fn distinct_types_stay_distinct() {
        assert_ne!(normalize_generics("Foo<T>"), normalize_generics("Bar<T>"));
    }

    #[test]
    fn fail_closed_on_return_arrow_in_generics() {
        // `->` closes the `<…>` group early (its `>`), then the real closer drives
        // depth below zero: garbage `Widget C::method` must not be emitted.
        assert_eq!(normalize_generics("Widget<Fn() -> C>::method"), None);
    }

    #[test]
    fn fail_closed_on_const_generic_comparison() {
        assert_eq!(normalize_generics("Widget<{ N > 0 }>::method"), None);
    }

    #[test]
    fn fail_closed_on_unclosed_generic() {
        assert_eq!(normalize_generics("Foo<Bar"), None);
    }

    #[test]
    fn fail_closed_on_stray_closer() {
        assert_eq!(normalize_generics("Foo>::method"), None);
    }
}
