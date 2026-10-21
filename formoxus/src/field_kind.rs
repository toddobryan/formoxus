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
//! Classification mirrors `build::node_for_shape`, except that it compares
//! names: `Shape::scalar_type` compares `TypeId`s and so can never be const.

use facet::{Def, Facet, Shape, StructKind, Type, UserType};
use formoxus_spec::{Attr, BuiltinWidget, FieldControl, FieldType, HtmlElement};

use crate::field::Field;
use crate::field_node::NodeCtor;
use crate::fields::{ChoiceField, DefaultField, IntoChoice};

/// The shape of the field a projection reaches, with the field's type inferred
/// from the closure. The closure is never called.
pub const fn shape_of<M, F: for<'a> Facet<'a>>(_: fn(&M) -> &F) -> &'static Shape {
    F::SHAPE
}

/// The leaf constructor for a field `form!` gives settings to (`min: 13`).
///
/// `form!` sees no types, so it never names the field: the projection pins the
/// model field's type `P`, [`DefaultField`] picks the field for it, and `set`
/// is the settings as assignments (`|f| { f.min = Some(13); }`), each typed by
/// that field. So `min: 300` on a `u8` is rustc's own out-of-range error, and a
/// setting the field lacks does not compile.
pub fn field_for<M, P: DefaultField>(_: fn(&M) -> &P, set: impl FnOnce(&mut P::Field)) -> NodeCtor {
    let mut field = P::Field::default();
    set(&mut field);
    NodeCtor::new(move |args| Box::new(args.node(field.clone())))
}

/// The leaf constructor for a field `form!` gives `choices:`: a
/// [`ChoiceField`] over the model field's own type, so `(1, "Small")` is a
/// `u8` on a `u8` field and an `Option<u8>` field chooses among `u8`s.
///
/// Not a setting on the default field, as `min` is, because choices change
/// WHICH field it is: a `String` with choices is a `ChoiceField<String>`, not
/// a `StringField`. `form!` refuses `choices` beside a validated key, so the
/// two constructors never meet on one field.
pub fn choice_field_for<M, P, C>(_: fn(&M) -> &P, choices: impl IntoIterator<Item = C>) -> NodeCtor
where
    P: DefaultField,
    C: IntoChoice<ValueOf<P>>,
    ValueOf<P>: Send + Sync,
{
    let field = ChoiceField::<ValueOf<P>>::new(choices);
    NodeCtor::new(move |args| Box::new(args.node(field.clone())))
}

/// The value type of a model field's default field: `u8` for a `u8` or an
/// `Option<u8>`. What a choice field over that model field chooses among.
type ValueOf<P> = <<P as DefaultField>::Field as Field>::Value;

/// One row of a list, for a `[]` path segment in a projection.
///
/// A plain fn rather than `.iter().next().unwrap()` in the generated code, so
/// that no user lint on `unwrap` fires inside a `form!`. Never called: the
/// projection it sits in is only type-checked.
pub fn row<'a, T: 'a>(_: impl IntoIterator<Item = &'a T>) -> &'a T {
    unreachable!("a form! projection is type-checked, never called")
}

/// Whether `attr` can apply to this field: the table's `for:` column, asked
/// of the field's type. `form!` emits one of these per table attribute a field
/// body declares.
///
/// An enum allows the attributes that apply to `<select>`
///
/// An opaque newtype is let through, since nothing here can see what it
/// wraps; a struct or list is refused, since no attribute applies to one.
pub const fn applies(attr: Attr, shape: &Shape) -> bool {
    match kind(shape) {
        // no need to check Attr::Multiple since it's owned by Formoxus
        Kind::Enum => {
            !attr.validated() && attr.is_valid_on(HtmlElement::Control(FieldControl::Select))
        }
        Kind::Unknown => true,
        Kind::Other => false,
        _ => match field_type(shape) {
            Some(t) => attr.applies_to(t),
            None => false,
        },
    }
}

/// Whether `min_length` or `max_length` can apply to this field.
pub const fn takes_length(shape: &Shape) -> bool {
    applies(Attr::MinLength, shape)
}

/// Whether `pattern` can apply to this field. Its own check rather than
/// `takes_length`'s, so the question asked of the table names the attribute.
pub const fn takes_pattern(shape: &Shape) -> bool {
    applies(Attr::Pattern, shape)
}

/// Whether `min` or `max` can apply to this field.
pub const fn takes_bound(shape: &Shape) -> bool {
    applies(Attr::Min, shape)
}

/// Whether `required_true` can apply: only to a bool, where it means the value
/// must be `true`. Every other field is already required by its type unless it
/// is an `Option`, so on anything else the key would mean nothing.
///
/// An `Option<bool>` passes HERE, because `kind` peels the `Option`.
/// [`required_is_not_optional`] refuses it, with its own message.
pub const fn takes_required(shape: &Shape) -> bool {
    applies(Attr::RequiredTrue, shape)
}

/// Whether `required_true` avoids an `Option`. An optional field may be left
/// unanswered, so it cannot also be required to be true.
///
/// `true` for anything [`takes_required`] rejects, so `required` on an
/// `Option<String>` reports once, from there, not twice.
pub const fn required_is_not_optional(shape: &Shape) -> bool {
    !is_optional(shape) || !takes_required(shape)
}

// ── Can this widget render this field? ──────────────────────────────────
//
// Which (field type, widget) pairs render is `BuiltinWidget::renders`. These
// checks, called from `form!`'s `const _` items, are the only place it is
// enforced: no widget type checks its field's type at runtime, so a pair
// `form!` refuses still renders from a hand-built `FormSpec` (`widget_matrix`
// marks those `ok*`).

/// Whether the field is one value, and so can have a widget at all. A struct,
/// list or enum is several, and a widget on one is either rejected at runtime
/// (a field set or list asserts) or silently ignored (an enum always renders its
/// variant picker).
pub const fn is_single_value(shape: &Shape) -> bool {
    !matches!(kind(shape), Kind::Other | Kind::Enum)
}

/// Whether `widget` can render a single-value field of this type. `has_choices`
/// is whether `form!` was given `choices` for it.
///
/// `true` for anything [`is_single_value`] rejects, so a widget on a struct
/// reports once, from there. `true` for a newtype whose inside is out of reach,
/// for the reason `Kind::Unknown` gives. Everything else is the field's
/// `FieldType`, and the rule is [`BuiltinWidget::renders`]; only the shape side
/// lives here, because `formoxus-spec` has no facet.
pub const fn renders(shape: &Shape, widget: BuiltinWidget, has_choices: bool) -> bool {
    let field_type = match kind(shape) {
        Kind::Enum | Kind::Other | Kind::Unknown => return true,
        Kind::Text => FieldType::Text,
        Kind::Int => FieldType::Int,
        Kind::Float => FieldType::Float,
        Kind::Bool => FieldType::Bool,
    };
    widget.renders(field_type, has_choices)
}

pub const fn is_optional(shape: &Shape) -> bool {
    matches!(shape.def, Def::Option(_))
}

pub const fn is_enum(shape: &Shape) -> bool {
    matches!(kind(shape), Kind::Enum)
}

/// Whether this field holds an integer, `Option` peeled: what refuses a
/// fractional `step` on one.
pub const fn is_int(shape: &Shape) -> bool {
    matches!(kind(shape), Kind::Int)
}

enum Kind {
    Text,
    Int,
    Float,
    Bool,
    /// An enum, which renders a `<select>` for the variants
    Enum,
    /// Something that is not one value: a struct, a list, or a scalar
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
    // `Option` layer can be peeled here exactly as `option_node` does.
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
            } else if matches!(shape.ty, Type::User(UserType::Enum(_))) {
                Kind::Enum
            } else {
                Kind::Other
            }
        }
    }
}

/// The scalars `build::scalar_node` supports. `usize`/`isize` are absent there
/// on purpose, so they are absent here too.
const fn primitive(name: &str) -> Kind {
    const INTS: [&str; 8] = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
    let mut i = 0;
    while i < INTS.len() {
        if str_eq(name, INTS[i]) {
            return Kind::Int;
        }
        i += 1;
    }
    if str_eq(name, "f32") || str_eq(name, "f64") {
        return Kind::Float;
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

const fn field_type(shape: &Shape) -> Option<FieldType> {
    match kind(shape) {
        Kind::Text => Some(FieldType::Text),
        Kind::Int => Some(FieldType::Int),
        Kind::Float => Some(FieldType::Float),
        Kind::Bool => Some(FieldType::Bool),
        Kind::Enum | Kind::Other | Kind::Unknown => None,
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
        applies, is_enum, is_optional, is_single_value, renders, required_is_not_optional,
        takes_bound, takes_length, takes_pattern, takes_required,
    };
    use facet::Facet;
    use formoxus_spec::{Attr, BuiltinWidget, RenderedInputType};
    use googletest::prelude::*;

    const TEXT: BuiltinWidget = BuiltinWidget::Input(RenderedInputType::Text);
    const CHECKBOX: BuiltinWidget = BuiltinWidget::Input(RenderedInputType::Checkbox);

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

    #[derive(Facet)]
    #[repr(u8)]
    enum Shipping {
        Ground,
        Air,
    }

    #[derive(Facet)]
    #[repr(u8)]
    #[expect(dead_code, reason = "only its shape is read")]
    enum Footprint {
        Circle { radius: f64 },
        Square(f64),
    }

    #[derive(Facet)]
    struct Address {
        city: std::string::String,
    }

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

    // ── Widgets ──────────────────────────────────────────────────────────

    #[gtest]
    fn an_input_renders_text_and_numbers_but_not_a_bool() {
        for shape in [std::string::String::SHAPE, u32::SHAPE, f64::SHAPE] {
            expect_that!(renders(shape, TEXT, false), eq(true), "{shape}");
        }
        expect_that!(renders(bool::SHAPE, TEXT, false), eq(false));
    }

    #[gtest]
    fn a_textarea_renders_only_text() {
        expect_that!(
            renders(std::string::String::SHAPE, BuiltinWidget::Textarea, false),
            eq(true)
        );
        expect_that!(
            renders(u32::SHAPE, BuiltinWidget::Textarea, false),
            eq(false)
        );
        expect_that!(
            renders(bool::SHAPE, BuiltinWidget::Textarea, false),
            eq(false)
        );
    }

    #[gtest]
    fn a_checkbox_renders_only_a_bool_optional_or_not() {
        expect_that!(renders(bool::SHAPE, CHECKBOX, false), eq(true));
        expect_that!(renders(<Option<bool>>::SHAPE, CHECKBOX, false), eq(true));
        expect_that!(
            renders(std::string::String::SHAPE, CHECKBOX, false),
            eq(false)
        );
    }

    #[gtest]
    fn a_chooser_needs_choices_except_for_a_bool() {
        expect_that!(renders(bool::SHAPE, BuiltinWidget::Select, false), eq(true));
        expect_that!(renders(u32::SHAPE, BuiltinWidget::Select, false), eq(false));
        expect_that!(renders(u32::SHAPE, BuiltinWidget::Select, true), eq(true));
        expect_that!(
            renders(std::string::String::SHAPE, BuiltinWidget::Select, true),
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
        expect_that!(renders(list, BuiltinWidget::Textarea, false), eq(true));
        expect_that!(renders(Markdown::SHAPE, CHECKBOX, false), eq(true));
    }

    #[gtest]
    fn a_bool_takes_nothing() {
        expect_that!(takes_length(bool::SHAPE), eq(false));
        expect_that!(takes_bound(bool::SHAPE), eq(false));
    }

    /// `required_true` means "must be true", so a bool is the only kind that
    /// takes it.
    #[gtest]
    fn only_a_bool_takes_required() {
        expect_that!(takes_required(bool::SHAPE), eq(true));
        for shape in [std::string::String::SHAPE, i32::SHAPE, f64::SHAPE] {
            expect_that!(takes_required(shape), eq(false), "{shape}");
        }
        expect_that!(takes_required(<Vec<bool>>::SHAPE), eq(false));
        expect_that!(takes_required(Flag::SHAPE), eq(true));
    }

    /// `kind` peels the `Option`, so `takes_required` alone would let an
    /// `Option<bool>` through. The second check is what refuses it.
    #[gtest]
    fn an_optional_bool_takes_the_type_check_but_fails_the_optional_one() {
        let maybe = <Option<bool>>::SHAPE;
        expect_that!(takes_required(maybe), eq(true));
        expect_that!(required_is_not_optional(maybe), eq(false));
        expect_that!(required_is_not_optional(bool::SHAPE), eq(true));
    }

    /// `required` on an `Option<String>` is wrong for two reasons, and should
    /// report once, from `takes_required`, rather than twice.
    #[gtest]
    fn an_optional_non_bool_is_left_to_the_type_check() {
        let maybe = <Option<std::string::String>>::SHAPE;
        expect_that!(takes_required(maybe), eq(false));
        expect_that!(required_is_not_optional(maybe), eq(true));
    }

    /// Unsupported by `scalar_node`, so a constraint on one could only ever be
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

    // ── Enums ────────────────────────────────────────────────────────────

    /// What an enum may take: anything its variant `<select>` can carry that
    /// formoxus does not validate. Asked of a unit enum, a data enum and an
    /// optional one, since all three render that same `<select>`.
    #[gtest]
    fn an_enum_takes_what_its_variant_select_can_carry() {
        for shape in [Shipping::SHAPE, Footprint::SHAPE, <Option<Shipping>>::SHAPE] {
            for attr in [
                Attr::Class,
                Attr::ClassPlus,
                Attr::Style,
                Attr::Disabled,
                Attr::Autofocus,
                Attr::AriaLabel,
            ] {
                expect_that!(applies(attr, shape), eq(true), "{attr:?} on {shape}");
            }
        }
    }

    /// Refused for one of two reasons: formoxus validates it, and an enum
    /// submits no value to validate (`pattern`, `min_length`, `min`, `max`);
    /// or a `<select>` cannot carry it (`placeholder`, `readonly`, `rows`).
    #[gtest]
    fn an_enum_refuses_validated_keys_and_ones_a_select_cannot_carry() {
        for shape in [Shipping::SHAPE, Footprint::SHAPE, <Option<Shipping>>::SHAPE] {
            for attr in [
                Attr::Pattern,
                Attr::MinLength,
                Attr::Min,
                Attr::Max,
                Attr::Placeholder,
                Attr::Readonly,
                Attr::Rows,
            ] {
                expect_that!(applies(attr, shape), eq(false), "{attr:?} on {shape}");
            }
            expect_that!(takes_length(shape), eq(false), "{shape}");
            expect_that!(takes_bound(shape), eq(false), "{shape}");
            expect_that!(takes_pattern(shape), eq(false), "{shape}");
            expect_that!(takes_required(shape), eq(false), "{shape}");
        }
    }

    /// An enum always renders its variant picker, so a `widget:` on one is
    /// refused by `is_single_value`, and `renders` leaves it to that check so
    /// the mistake reports once.
    #[gtest]
    fn an_enum_still_takes_no_widget() {
        for shape in [Shipping::SHAPE, Footprint::SHAPE, <Option<Shipping>>::SHAPE] {
            expect_that!(is_single_value(shape), eq(false), "{shape}");
            for widget in [BuiltinWidget::Textarea, CHECKBOX] {
                expect_that!(
                    renders(shape, widget, false),
                    eq(true),
                    "{widget:?} on {shape}"
                );
            }
        }
    }

    /// facet types `Option<T>` itself as a `UserType::Enum` (`Some`/`None`),
    /// so asking `shape.ty` would call every optional field an enum, and
    /// `min` on an `Option<String>` would get the enum message. `is_enum`
    /// asks `kind`, which peels the `Option` first.
    #[gtest]
    fn an_option_is_an_enum_only_when_it_holds_one() {
        expect_that!(is_enum(Shipping::SHAPE), eq(true));
        expect_that!(is_enum(<Option<Shipping>>::SHAPE), eq(true));
        expect_that!(is_enum(<Option<std::string::String>>::SHAPE), eq(false));
        expect_that!(is_enum(<Option<u32>>::SHAPE), eq(false));
        expect_that!(is_enum(Address::SHAPE), eq(false));
    }

    /// The reason `Kind::Enum` is its own kind rather than a loosened
    /// `Kind::Other`: a struct's field set renders nothing an attribute could
    /// land on, so `class` on one must stay a compile error.
    #[gtest]
    fn a_struct_and_a_list_still_take_nothing_not_even_class() {
        for shape in [Address::SHAPE, <Vec<std::string::String>>::SHAPE] {
            expect_that!(applies(Attr::Class, shape), eq(false), "{shape}");
            expect_that!(applies(Attr::Style, shape), eq(false), "{shape}");
        }
    }

    /// `pattern` asks the table about itself rather than borrowing the length
    /// check, so these mirror the length cases exactly: text yes (optional or
    /// not), numbers, bools and lists no, an opaque newtype let through.
    #[gtest]
    fn pattern_applies_to_text_and_nothing_else() {
        expect_that!(takes_pattern(std::string::String::SHAPE), eq(true));
        expect_that!(
            takes_pattern(<Option<std::string::String>>::SHAPE),
            eq(true)
        );
        expect_that!(takes_pattern(Markdown::SHAPE), eq(true));
        for shape in [
            u32::SHAPE,
            f64::SHAPE,
            bool::SHAPE,
            <Vec<std::string::String>>::SHAPE,
        ] {
            expect_that!(takes_pattern(shape), eq(false), "{shape}");
        }
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
