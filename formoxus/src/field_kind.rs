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
    matches!(
        kind(shape),
        Kind::Int { .. } | Kind::Float { .. } | Kind::Unknown
    )
}

// ── Does a bound fit the field's type? ──────────────────────────────────
//
// `form!` passes each `min`/`max` twice, as `(e) as f64` and `(e) as i128`,
// because a const fn cannot be generic over "some number". Between them the
// two casts answer every question below, whatever type the bound was written
// in. Each check is `true` for a field that is not a number, so a bound on the
// wrong kind of field reports once, from `takes_bound`, not four times.

/// Whether the bound lies within the values the field's type can hold. One
/// outside it either excludes nothing (`min: -1000` on an `i8`) or excludes
/// everything (`min: 1000` on an `i8`), and either way it is a mistake. A bound
/// AT the limit, like `min: 0` on a `u32`, is fine: that is ordinary.
///
/// For a float field this is also what catches `max: 1e50` on an `f32`. Parsing
/// saturates, so `1e39` arrives as `inf` and is refused by a bound it is visibly
/// inside.
pub const fn bound_in_range(shape: &Shape, as_f64: f64, as_i128: i128) -> bool {
    if as_f64.is_nan() {
        return !is_number(shape);
    }
    match kind(shape) {
        // `as i128` saturates, so a float bound far outside `i128` still lands
        // outside every integer type's range.
        Kind::Int { min, max } => min <= as_i128 && as_i128 <= max,
        Kind::Float { max } => -max <= as_f64 && as_f64 <= max,
        _ => true,
    }
}

/// Whether a bound on an integer field is a whole number. `min: 2.0` is, and
/// the runtime accepts it; `min: 1.5` would have to be rounded, and rounding
/// either way changes which values pass.
pub const fn bound_is_whole(shape: &Shape, as_f64: f64, as_i128: i128) -> bool {
    match kind(shape) {
        #[expect(
            clippy::cast_precision_loss,
            clippy::float_cmp,
            reason = "exact equality is the question: both sides round the same way, so \
            an integer bound compares equal and a fractional one cannot"
        )]
        Kind::Int { .. } => as_f64 == as_i128 as f64,
        _ => true,
    }
}

/// Whether a bound on a float field survives the runtime's widening to `f64`.
/// An integer above 2^53 does not: `min: 9_007_199_254_740_993` is stored as
/// ...992, and the value one below the stated minimum gets in. A bound written
/// as a float is already an `f64`, and both casts truncate it alike.
pub const fn bound_is_exact(shape: &Shape, as_f64: f64, as_i128: i128) -> bool {
    match kind(shape) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation is the comparison: it matches `as_i128` exactly when nothing was rounded"
        )]
        Kind::Float { .. } => as_f64 as i128 == as_i128,
        _ => true,
    }
}

// ── Can this widget render this field? ──────────────────────────────────
//
// Mirrors `widgets::scalar::ScalarWidget`'s match arms, which are the only
// place that decides which (value kind, widget) pairs render. A pair with no
// arm panics there, and Dioxus contains the panic to that one component, so
// the field silently vanishes from the form. These checks turn that into a
// build error instead. **Keep them in step with that match.**

/// The widgets `form!` can check, grouped by the rule they share.
///
/// `select_multiple`, `checkbox_multiple` and `file` are absent because nothing
/// renders them for any field; `form!` rejects those while it parses, without
/// needing the field's type. `custom(…)` is absent because it renders whatever
/// it is given, and needs only [`is_single_value`].
#[derive(Clone, Copy, Debug)]
pub enum WidgetClass {
    /// Every `<input type=…>`.
    Input,
    Textarea,
    Checkbox,
    /// `select` and `radio_group`, which pick one value from `choices`.
    Chooser,
}

/// Whether the field is one value, and so can have a widget at all. A struct,
/// list or enum is several, and a widget on one is either rejected at runtime
/// (a field set or list asserts) or silently ignored (an enum always renders its
/// variant picker).
pub const fn is_single_value(shape: &Shape) -> bool {
    !matches!(kind(shape), Kind::Other)
}

/// Whether `widget` can render a single-value field of this type. `has_choices`
/// is whether `form!` was given `choices` for it.
///
/// `true` for anything [`is_single_value`] rejects, so a widget on a struct
/// reports once, from there. `true` for a newtype whose inside is out of reach,
/// for the reason `Kind::Unknown` gives.
pub const fn renders(shape: &Shape, widget: WidgetClass, has_choices: bool) -> bool {
    let kind = kind(shape);
    if matches!(kind, Kind::Other | Kind::Unknown) {
        return true;
    }
    let is_bool = matches!(kind, Kind::Bool);
    match widget {
        WidgetClass::Input => !is_bool,
        WidgetClass::Textarea => matches!(kind, Kind::Text),
        WidgetClass::Checkbox => is_bool,
        // A bool's choices can be derived, so it is the one kind that renders
        // without a list.
        WidgetClass::Chooser => is_bool || has_choices,
    }
}

const fn is_number(shape: &Shape) -> bool {
    matches!(kind(shape), Kind::Int { .. } | Kind::Float { .. })
}

pub const fn is_optional(shape: &Shape) -> bool {
    matches!(shape.def, Def::Option(_))
}

enum Kind {
    Text,
    /// The range of the integer type, widened to `i128` since every supported
    /// integer fits.
    Int {
        min: i128,
        max: i128,
    },
    /// The largest finite value of the float type, as an `f64`.
    Float {
        max: f64,
    },
    Bool,
    /// Something that is not one value: a struct, a list, an enum, or a scalar
    /// formoxus does not support.
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

/// The scalars `build::scalar_member` supports, with their limits.
/// `usize`/`isize` are absent there on purpose, so they are absent here too.
const fn primitive(name: &str) -> Kind {
    // `i128::from` is not const, and every one of these widens losslessly.
    const INTS: [(&str, i128, i128); 8] = [
        ("i8", i8::MIN as i128, i8::MAX as i128),
        ("i16", i16::MIN as i128, i16::MAX as i128),
        ("i32", i32::MIN as i128, i32::MAX as i128),
        ("i64", i64::MIN as i128, i64::MAX as i128),
        ("u8", 0, u8::MAX as i128),
        ("u16", 0, u16::MAX as i128),
        ("u32", 0, u32::MAX as i128),
        ("u64", 0, u64::MAX as i128),
    ];
    let mut i = 0;
    while i < INTS.len() {
        let (int, min, max) = INTS[i];
        if str_eq(name, int) {
            return Kind::Int { min, max };
        }
        i += 1;
    }
    if str_eq(name, "f32") {
        return Kind::Float {
            max: f32::MAX as f64,
        };
    }
    if str_eq(name, "f64") {
        return Kind::Float { max: f64::MAX };
    }
    if str_eq(name, "bool") {
        return Kind::Bool;
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
    use super::{
        WidgetClass, bound_in_range, bound_is_exact, bound_is_whole, is_optional, is_single_value,
        renders, takes_bound, takes_length,
    };
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

    // ── Bounds ───────────────────────────────────────────────────────────

    /// What `form!` passes: the bound cast both ways.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "this is the cast form! emits"
    )]
    fn casts(bound: f64) -> (f64, i128) {
        (bound, bound as i128)
    }

    #[gtest]
    fn a_bound_at_the_limit_of_the_type_fits() {
        expect_that!(bound_in_range(u32::SHAPE, 0.0, 0), eq(true));
        expect_that!(bound_in_range(i8::SHAPE, -128.0, -128), eq(true));
        expect_that!(
            bound_in_range(std::primitive::u8::SHAPE, 255.0, 255),
            eq(true)
        );
    }

    #[gtest]
    fn a_bound_past_the_limit_of_the_type_does_not() {
        expect_that!(bound_in_range(i8::SHAPE, -1000.0, -1000), eq(false));
        expect_that!(bound_in_range(u32::SHAPE, -1.0, -1), eq(false));
        expect_that!(
            bound_in_range(std::primitive::u8::SHAPE, 256.0, 256),
            eq(false)
        );
        let (f, i) = casts(1e30);
        expect_that!(bound_in_range(i32::SHAPE, f, i), eq(false));
    }

    #[gtest]
    fn a_float_bound_must_be_finite_within_the_float_type() {
        let (f, i) = casts(1e50);
        expect_that!(bound_in_range(f32::SHAPE, f, i), eq(false));
        expect_that!(bound_in_range(f64::SHAPE, f, i), eq(true));
        let (f, i) = casts(f64::INFINITY);
        expect_that!(bound_in_range(f64::SHAPE, f, i), eq(false));
        let (f, i) = casts(f64::NAN);
        expect_that!(bound_in_range(f64::SHAPE, f, i), eq(false));
    }

    #[gtest]
    fn a_bound_on_an_integer_must_be_whole() {
        let (f, i) = casts(2.0);
        expect_that!(bound_is_whole(u32::SHAPE, f, i), eq(true));
        let (f, i) = casts(1.5);
        expect_that!(bound_is_whole(u32::SHAPE, f, i), eq(false));
        expect_that!(bound_is_whole(f64::SHAPE, f, i), eq(true));
    }

    /// 2^53 + 1 is the first integer an `f64` cannot hold.
    #[expect(clippy::cast_precision_loss, reason = "the rounding is the point")]
    #[gtest]
    fn an_integer_bound_on_a_float_must_survive_the_widening() {
        let exact: i128 = 1 << 53;
        let inexact = exact + 1;
        expect_that!(bound_is_exact(f64::SHAPE, exact as f64, exact), eq(true));
        expect_that!(
            bound_is_exact(f64::SHAPE, inexact as f64, inexact),
            eq(false)
        );
        expect_that!(
            bound_is_exact(i64::SHAPE, inexact as f64, inexact),
            eq(true)
        );
    }

    /// Every bound check passes on a field that is not a number, so a bound on
    /// the wrong kind of field reports once, from `takes_bound`.
    #[gtest]
    fn the_bound_checks_leave_a_non_number_to_takes_bound() {
        let (f, i) = casts(f64::NAN);
        let text = std::string::String::SHAPE;
        expect_that!(bound_in_range(text, f, i), eq(true));
        expect_that!(bound_is_whole(text, 1.5, 1), eq(true));
        expect_that!(bound_is_exact(text, 0.0, 1), eq(true));
    }

    // ── Widgets ──────────────────────────────────────────────────────────

    #[gtest]
    fn an_input_renders_text_and_numbers_but_not_a_bool() {
        for shape in [std::string::String::SHAPE, u32::SHAPE, f64::SHAPE] {
            expect_that!(
                renders(shape, WidgetClass::Input, false),
                eq(true),
                "{shape}"
            );
        }
        expect_that!(renders(bool::SHAPE, WidgetClass::Input, false), eq(false));
    }

    #[gtest]
    fn a_textarea_renders_only_text() {
        expect_that!(
            renders(std::string::String::SHAPE, WidgetClass::Textarea, false),
            eq(true)
        );
        expect_that!(renders(u32::SHAPE, WidgetClass::Textarea, false), eq(false));
        expect_that!(
            renders(bool::SHAPE, WidgetClass::Textarea, false),
            eq(false)
        );
    }

    #[gtest]
    fn a_checkbox_renders_only_a_bool_optional_or_not() {
        expect_that!(renders(bool::SHAPE, WidgetClass::Checkbox, false), eq(true));
        expect_that!(
            renders(<Option<bool>>::SHAPE, WidgetClass::Checkbox, false),
            eq(true)
        );
        expect_that!(
            renders(std::string::String::SHAPE, WidgetClass::Checkbox, false),
            eq(false)
        );
    }

    #[gtest]
    fn a_chooser_needs_choices_except_for_a_bool() {
        expect_that!(renders(bool::SHAPE, WidgetClass::Chooser, false), eq(true));
        expect_that!(renders(u32::SHAPE, WidgetClass::Chooser, false), eq(false));
        expect_that!(renders(u32::SHAPE, WidgetClass::Chooser, true), eq(true));
        expect_that!(
            renders(std::string::String::SHAPE, WidgetClass::Chooser, true),
            eq(true)
        );
    }

    /// Reads `def` directly, so — unlike every other predicate here — it does
    /// NOT look through the `Option`. That is the whole point: `radio_group` is
    /// rejected on an optional field however renderable the inner type is.
    #[gtest]
    fn is_optional_sees_the_option_itself_not_what_it_holds() {
        expect_that!(is_optional(<Option<bool>>::SHAPE), eq(true));
        expect_that!(is_optional(<Option<std::string::String>>::SHAPE), eq(true));
        expect_that!(is_optional(bool::SHAPE), eq(false));
        expect_that!(is_optional(std::string::String::SHAPE), eq(false));
    }

    #[gtest]
    fn only_a_single_value_can_have_a_widget() {
        expect_that!(is_single_value(std::string::String::SHAPE), eq(true));
        expect_that!(is_single_value(<Option<bool>>::SHAPE), eq(true));
        expect_that!(is_single_value(Markdown::SHAPE), eq(true));
        expect_that!(
            is_single_value(<Vec<std::string::String>>::SHAPE),
            eq(false)
        );
    }

    /// Reported once, from `is_single_value`, and let through as "can't tell".
    #[gtest]
    fn renders_leaves_structs_and_opaque_newtypes_to_other_checks() {
        let list = <Vec<std::string::String>>::SHAPE;
        expect_that!(renders(list, WidgetClass::Textarea, false), eq(true));
        expect_that!(
            renders(Markdown::SHAPE, WidgetClass::Checkbox, false),
            eq(true)
        );
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
