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
/// type or path spelling, yielding its canonical, generic-free form.
#[must_use]
pub fn normalize_generics(type_expr: &str) -> String {
    // Fold turbofish `::<` into `<` so the separator does not survive stripping.
    let pre = type_expr.replace("::<", "<");
    let mut out = String::with_capacity(pre.len());
    let mut depth: u32 = 0;
    for ch in pre.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_single_type_param() {
        assert_eq!(normalize_generics("Type<T>::method"), "Type::method");
    }

    #[test]
    fn strips_turbofish() {
        assert_eq!(normalize_generics("Type::<T>::method"), "Type::method");
    }

    #[test]
    fn strips_lifetime() {
        assert_eq!(normalize_generics("Type<'a>::method"), "Type::method");
    }

    #[test]
    fn strips_multiple_args() {
        assert_eq!(normalize_generics("Type<A, B>::method"), "Type::method");
    }

    #[test]
    fn strips_on_path_tail_type() {
        assert_eq!(normalize_generics("a::b::Widget<T>"), "a::b::Widget");
    }

    #[test]
    fn strips_nested_generics() {
        assert_eq!(normalize_generics("Vec<HashMap<K, V>>"), "Vec");
    }

    #[test]
    fn leaves_plain_paths_untouched() {
        assert_eq!(normalize_generics("Foo"), "Foo");
        assert_eq!(normalize_generics("Foo::bar"), "Foo::bar");
    }

    #[test]
    fn preserves_leading_colon() {
        assert_eq!(normalize_generics("::std::mem::swap"), "::std::mem::swap");
    }

    #[test]
    fn generic_variants_converge() {
        let a = normalize_generics("Foo<u8>::get");
        let b = normalize_generics("Foo<u16>::get");
        let c = normalize_generics("Foo::<u8>::get");
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, "Foo::get");
    }

    #[test]
    fn foo_and_generic_foo_converge() {
        assert_eq!(normalize_generics("Foo<T>"), normalize_generics("Foo"));
    }

    #[test]
    fn distinct_types_stay_distinct() {
        assert_ne!(normalize_generics("Foo<T>"), normalize_generics("Bar<T>"));
    }
}
