//! Which constraints a field's type can take, decided at compile time.
//!
//! Support for `form!`, which emits one free `const _` item per constraint key:
//!
//! ```ignore
//! const _: () = assert!(
//!     takes_length(shape_of(|__m: &Profile| &__m.bio)),
//!     "`max_length` applies only to a String field",
//! );
//! ```
//!
//! **A free `const` item, not a `const {}` block in a generic fn.** A generic
//! fn's inline const is evaluated only when the fn is monomorphized. That never
//! happens under `cargo check`, so rust-analyzer would never show the error, and
//! it never happens at all when the fn is never called, which is true of
//! `form!`'s `__paths_exist` witness. A free const item is evaluated by
//! `cargo check`. Its one generic call, [`shape_of`], is a const fn instantiated
//! inside the item, and the projection closure is what lets the field's type be
//! inferred rather than named.
//!
//! Classification mirrors `build::member_for_shape`, except that it compares
//! names: `Shape::scalar_type` compares `TypeId`s and so can never be const.

use facet::{Def, Facet, Shape, StructKind, Type, UserType};

/// The shape of the field a projection reaches, with the field's type inferred
/// from the closure. The closure is never called.
pub const fn shape_of<M, F: for<'a> Facet<'a>>(_: fn(&M) -> &F) -> &'static Shape {
    F::SHAPE
}

/// One row of a list, for a `[]` path segment in a projection.
///
/// A plain fn rather than `.iter().next().unwrap()` in the generated code, so
/// that no user lint on `unwrap` fires inside a `form!`. Never called: the
/// projection it sits in is only type-checked.
pub fn row<'a, T: 'a>(_: impl IntoIterator<Item = &'a T>) -> &'a T {
    unreachable!("a form! projection is type-checked, never called")
}

/// Whether `min_length`, `max_length` or `pattern` can apply to this field.
pub const fn takes_length(shape: &Shape) -> bool {
    matches!(kind(shape), Kind::Text | Kind::Unknown)
}

/// Whether `min` or `max` can apply to this field.
pub const fn takes_bound(shape: &Shape) -> bool {
    matches!(kind(shape), Kind::Number | Kind::Unknown)
}

enum Kind {
    Text,
    Number,
    /// Something no constraint applies to: `bool`, a struct, a list, or a
    /// scalar formoxus does not support.
    Other,
    /// A newtype whose inner type is out of reach. A tuple struct's field shape
    /// sits behind a `ShapeRef` fn pointer, and const code cannot call one. The
    /// runtime sees the inner type, so the constraint may well apply; saying
    /// "no" here would reject `Markdown(String)` with `max_length`.
    Unknown,
}

const fn kind(shape: &Shape) -> Kind {
    // `OptionDef::t` is a plain `&'static Shape`, not a `ShapeRef`, so the
    // `Option` layer can be peeled here exactly as `option_member` does.
    if let Def::Option(option) = &shape.def {
        return kind(option.t);
    }
    match shape.module_path {
        // Every user type has a module path, so `None` is a primitive: a user
        // type named `u8` cannot land here.
        None => primitive(shape.type_identifier),
        Some(module) => {
            if str_eq(shape.type_identifier, "String") && str_eq(module, "alloc::string") {
                Kind::Text
            } else if is_newtype(shape) {
                // `#[facet(transparent)]` fills in `inner`, which IS reachable.
                match shape.inner {
                    Some(inner) => kind(inner),
                    None => Kind::Unknown,
                }
            } else {
                Kind::Other
            }
        }
    }
}

/// The scalars `build::scalar_member` supports. `usize`/`isize` are absent there
/// on purpose, so they are absent here too.
const fn primitive(name: &str) -> Kind {
    const NUMBERS: [&str; 10] = [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ];
    let mut i = 0;
    while i < NUMBERS.len() {
        if str_eq(name, NUMBERS[i]) {
            return Kind::Number;
        }
        i += 1;
    }
    Kind::Other
}

/// A one-field tuple struct: the same test as `build::newtype_inner`, minus the
/// part that needs the field's shape.
const fn is_newtype(shape: &Shape) -> bool {
    match &shape.ty {
        Type::User(UserType::Struct(st)) => {
            matches!(st.kind, StructKind::TupleStruct) && st.fields.len() == 1
        }
        _ => false,
    }
}

/// `==` on `str` is a trait method, which const code cannot call.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{takes_bound, takes_length};
    use facet::Facet;
    use googletest::prelude::*;

    #[derive(Facet)]
    struct Markdown(std::string::String);

    #[derive(Facet)]
    #[facet(transparent)]
    struct Slug(std::string::String);

    #[derive(Facet)]
    #[facet(transparent)]
    struct Flag(bool);

    #[derive(Facet)]
    #[expect(
        non_camel_case_types,
        reason = "the point is a user type named like a primitive"
    )]
    struct u8 {
        value: std::primitive::u8,
    }

    #[derive(Facet)]
    struct String;

    #[gtest]
    fn text_takes_lengths_and_not_bounds() {
        expect_that!(takes_length(std::string::String::SHAPE), eq(true));
        expect_that!(takes_bound(std::string::String::SHAPE), eq(false));
    }

    #[gtest]
    fn every_supported_number_takes_bounds_and_not_lengths() {
        for shape in [
            i8::SHAPE,
            i16::SHAPE,
            i32::SHAPE,
            i64::SHAPE,
            std::primitive::u8::SHAPE,
            u16::SHAPE,
            u32::SHAPE,
            u64::SHAPE,
            f32::SHAPE,
            f64::SHAPE,
        ] {
            expect_that!(takes_bound(shape), eq(true), "{shape}");
            expect_that!(takes_length(shape), eq(false), "{shape}");
        }
    }

    #[gtest]
    fn a_bool_takes_nothing() {
        expect_that!(takes_length(bool::SHAPE), eq(false));
        expect_that!(takes_bound(bool::SHAPE), eq(false));
    }

    /// Unsupported by `scalar_member`, so a constraint on one could only ever be
    /// dropped or panic.
    #[gtest]
    fn unsupported_scalars_take_nothing() {
        for shape in [usize::SHAPE, isize::SHAPE, char::SHAPE] {
            expect_that!(takes_bound(shape), eq(false), "{shape}");
            expect_that!(takes_length(shape), eq(false), "{shape}");
        }
    }

    #[gtest]
    fn an_option_is_classified_by_what_it_holds() {
        expect_that!(takes_length(<Option<std::string::String>>::SHAPE), eq(true));
        expect_that!(takes_bound(<Option<u32>>::SHAPE), eq(true));
        expect_that!(takes_bound(<Option<std::string::String>>::SHAPE), eq(false));
    }

    #[gtest]
    fn a_list_takes_nothing() {
        expect_that!(takes_length(<Vec<std::string::String>>::SHAPE), eq(false));
        expect_that!(takes_bound(<Vec<u32>>::SHAPE), eq(false));
    }

    /// Out of reach, so let through: the runtime sees the `String` inside.
    #[gtest]
    fn a_plain_newtype_takes_either() {
        expect_that!(takes_length(Markdown::SHAPE), eq(true));
        expect_that!(takes_bound(Markdown::SHAPE), eq(true));
    }

    #[gtest]
    fn a_transparent_newtype_is_classified_by_what_it_wraps() {
        expect_that!(takes_length(Slug::SHAPE), eq(true));
        expect_that!(takes_bound(Slug::SHAPE), eq(false));
        expect_that!(takes_length(Flag::SHAPE), eq(false));
    }

    /// Names are compared, so a user type must not pass for the real one.
    #[gtest]
    fn a_user_type_named_like_a_scalar_is_not_one() {
        expect_that!(takes_bound(u8::SHAPE), eq(false));
        expect_that!(takes_length(String::SHAPE), eq(false));
    }
}
