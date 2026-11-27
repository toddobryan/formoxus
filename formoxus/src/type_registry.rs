//! The app's own types: which [`Field`] reads each one, for every form that
//! meets it.
//!
//! A model derives only `Facet`, so formoxus cannot tell from a
//! `struct TimeLimit(u32)` that the user types `1h15m`. Two places can say
//! so. `field:` in [`form!`](macro@crate::form) says it for one path. A list
//! of types, declared with [`types!`](macro@crate::types), says it for the
//! type wherever it sits: a struct's field, a list or set item, a variant's
//! field, or the inside of an `Option`. The model stays plain either way.
//!
//! # A list
//!
//! ```ignore
//! // src/types.rs
//! formoxus::types! {
//!     /// The app's own types.
//!     pub struct AppTypes;
//!     Markdown => newtype String,
//!     TimeLimit => field TimeLimitField,
//! }
//!
//! // in each module whose forms meet them
//! use crate::types::AppTypes as FormoxusTypes;
//! ```
//!
//! An entry gives one type its field, in one of two ways:
//!
//! 1. `T => newtype V`: a one-field tuple struct such as
//!    `struct Markdown(String)`, read and written as its `V`, through the
//!    list's own field for `V`. The field is a
//!    [`Newtype`](crate::fields::Newtype).
//! 2. `T => field F`: `T` through `F`, a field of the dev's own whose `Value`
//!    is `T`, built with `F::default()`. An entry takes no settings, so a
//!    default for every `T`, such as a maximum length, goes in `F`'s
//!    `Default`.
//!
//! No newtype is special, `#[facet(transparent)]` included: formoxus ignores
//! it, and a `Markdown(String)` needs an entry like any other type of the
//! app's own.
//!
//! A type no list holds is read by its shape: a struct is a group of
//! fields, an enum a choice of its variants, a `Vec` a list of items. Building
//! a form refuses two kinds of unlisted type, with a panic naming the path
//! and the type and giving the line to add: a one-field tuple struct around a
//! `String`, `bool` or number, and anything formoxus cannot build at all
//! (an array, a `usize`).
//!
//! # `FormoxusTypes`
//!
//! `form!` and [`path!`](macro@crate::path) read the list named
//! `FormoxusTypes` where they are written. The prelude exports one,
//! [`BuiltIns`], which holds only the built-in types, so a module that
//! imports the prelude and needs no types of its own works unchanged. A
//! module that writes `use crate::types::AppTypes as FormoxusTypes;` shadows
//! the prelude's, since an explicit `use` beats a glob.
//!
//! `form!` reads the list at compile time and again at run time:
//!
//! 1. **At compile time**, its checks judge a listed type by its field's
//!    [`KIND`](crate::field::Field::KIND) rather than its shape. A
//!    `widget: checkbox` on a `TimeLimit` path is a compile error when
//!    `TimeLimitField` reads an `Int`, and a set of `Ref<Question>` compiles
//!    when `Ref<Question>` is listed.
//! 2. **At run time**, the spec carries the list
//!    ([`FormSpec::with_types`](crate::form::FormSpec::with_types)). So the
//!    server, rebuilding the form from the wire, reads the same list as the
//!    browser, and so does every later build: an item added, a variant
//!    chosen. Nothing has to be installed for a form to read its own types.
//!
//! A [`FormSpec`](crate::form::FormSpec) built by hand calls
//! `.with_types(AppTypes::registry())` first. Without it, the spec sees only
//! the built-in types and the installed overrides.
//!
//! # Built-in types
//!
//! `String`, `bool`, `i8` to `i64`, `u8` to `u64`, `f32` and `f64`. Every
//! list holds them, and they read the same in every form of every crate: no
//! list can give one a field of its own, and
//! [`TypeRegistry::field_for`] panics for one. A `String` that reads
//! differently is a type of the dev's own, `struct Shouty(String);`, listed
//! with its own field.
//!
//! Left out on purpose: `usize` and `isize`, whose width is 32 bits in the
//! browser and 64 on the server, so one field would accept different values
//! on each side; and `char` and `i128`. A list may give any of them a field.
//!
//! # Libraries: `include` and `replace`
//!
//! A library declares its own `pub` list, and its forms read it. An app (or
//! another library) that uses those types includes it:
//!
//! ```ignore
//! formoxus::types! {
//!     pub struct AppTypes;
//!     include course_lib::CourseTypes;
//!     replace course_lib::Due => field AppDueField,
//!     TimeLimit => field TimeLimitField,
//! }
//! ```
//!
//! 1. `include krate::List;` brings the library's own entries into the
//!    app's list, so the app's forms read a `course_lib::Badge` the way the
//!    library's do. It names the list by its crate and name, wherever in the
//!    library the list is declared, and only a `pub` list can be included.
//!    It brings the library's own entries only, not what the library itself
//!    included or replaced.
//! 2. `replace krate::T => field F` gives an included type a different
//!    field. The type must come from an included list, and a second plain
//!    entry for it is refused. `F` must read the same
//!    [`KIND`](crate::field::Field::KIND) as the field it replaces, because
//!    the library's forms were checked against that one at compile time.
//!
//! # Installing overrides
//!
//! A `replace` changes the app's own forms, through its list. A library's
//! forms carry the library's list, so to change them too the app installs
//! its overrides at the top of `main`:
//!
//! ```ignore
//! fn main() {
//!     AppTypes::install();
//!     dioxus::launch(App);
//! }
//! ```
//!
//! `install` installs the list's `replace` lines and nothing else. An
//! ordinary entry for a type the app did not include stays the app's own: it
//! never reaches a library's forms. Installing is once per process.
//!
//! The overrides are a process-wide value rather than Dioxus context because
//! a form is built in the browser and rebuilt on the server, inside a server
//! function that runs outside any component. Context would reach the first
//! and not the second, and the two would read the same text differently.
//! Both run `main`, so both see what `main` installs. For the same reason
//! they are not part of [`Formoxus`](crate::defaults::Formoxus), which a
//! subtree can provide for itself.
//!
//! # What wins
//!
//! A leaf's field is the first of:
//!
//! 1. the path's own `field:` in `form!`, or the list's field configured by
//!    the path's settings (`max_length: 80`);
//! 2. the installed overrides;
//! 3. the form's own list, which holds the built-ins; for a hand-built spec
//!    with no list, the built-ins.
//!
//! `path!(Model.tags[= value])` writes a set item's key through the same
//! order, so the key matches the text the item's field wrote.
//!
//! # Widgets
//!
//! A leaf's widget is its path's `widget:`, or else its field's
//! [`default_widget`](crate::field::Field::default_widget). A list cannot
//! name a widget for a type yet. [`TypeRegistry::widget_for`] records one,
//! but no form reads it.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

use facet::{Facet, Peek, Shape};
use formoxus_spec::FieldType;

use crate::field::Field;
use crate::nodes::NodeCtor;
use crate::widgets::{AnyWidget, LeafWidget};

formoxus_macros::types! {
    /// The built-in types and nothing else: the list a form with none of its
    /// own reads (decisions AC and AD), and the prelude's `FormoxusTypes`.
    /// Every other list holds these too, and none can change them (AF).
    pub struct BuiltIns;
}

/// The installed overrides, set once by [`TypeRegistry::install`]. Empty
/// until then, which is not an error: most apps install nothing.
static REGISTRY: OnceLock<TypeRegistry> = OnceLock::new();

/// A list of types at run time: the field each type gets. See the
/// [module docs](crate::type_registry) for lists and what wins.
///
/// [`types!`](macro@crate::types) builds these, so a dev rarely writes one.
/// Each list has two: `registry()`, everything it holds, which its forms
/// carry in their spec; and `overrides()`, its `replace` lines alone, which
/// its `install()` installs. Built by hand, with [`new`](Self::new) and
/// [`field_for`](Self::field_for), a registry is for a hand-built
/// [`FormSpec`](crate::form::FormSpec) or for [`install`](Self::install).
///
/// Each type gets at most one field and one widget, and a widget that
/// cannot render its type's field is refused. Both panic in the builder call
/// that breaks the rule, so a mistake stops the app at startup rather than
/// when a form first meets the type.
///
/// A type is identified by its `Shape`, which is per concrete type: a
/// generic type gets an entry for each instantiation, `Ref<Question>` and
/// `Ref<Assignment>` separately.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Keyed by `&Shape`, whose `Eq` and `Hash` compare type ids rather than
    /// addresses, so two copies of one type's shape find the same entry.
    pub(crate) types: HashMap<&'static Shape, Registered>,
}

impl TypeRegistry {
    /// The installed overrides, or `None` before [`install`](Self::install)
    /// or in an app that never calls it. `None` reads as no overrides.
    ///
    /// Never `get_or_init`: a form built before `install` would then fix an
    /// empty registry in place, and the app's own `install` would panic.
    pub(crate) fn registry() -> Option<&'static TypeRegistry> {
        REGISTRY.get()
    }

    /// An empty registry, the same as [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs this as the process's overrides: asked before each form's
    /// own list, for every form in the process, a library's included. An app
    /// calls it through its list, `AppTypes::install()`, which passes the
    /// list's `replace` lines and nothing else; see
    /// [Installing overrides](crate::type_registry#installing-overrides).
    ///
    /// Call it at the top of `main`, before `launch`. The browser and the
    /// server each run `main`, so each installs the same overrides. A form
    /// built before this call sees none.
    ///
    /// # Panics
    ///
    /// If overrides are already installed. There is one set per process, so a
    /// second would mean two parts of the app disagreeing about a type.
    pub fn install(self) {
        assert!(
            REGISTRY.get().is_none(),
            "formoxus's type overrides are already installed, and they can be installed only \
             once per process. Install them once, at the top of `main`, before `launch`"
        );
        match REGISTRY.set(self) {
            Ok(()) => (),
            _ => panic!(
                "formoxus's type overrides were installed by another thread while this one \
                 was installing them. Install them once, at the top of `main`, before `launch`"
            ),
        }
    }

    /// Gives `field`'s [`Value`](crate::field::Field::Value) type its field:
    /// every path of that type in a form reading this registry parses,
    /// formats and checks through it, unless the path names a `field:` of its
    /// own. A list's `T => field F` entry is this call.
    ///
    /// The type comes from the field, so a field cannot be given to the wrong
    /// one. `Send + Sync` because every form in the process shares it.
    ///
    /// # Panics
    ///
    /// If the type is built in (a `String`, a `bool` or a number), since a
    /// built-in reads the same in every form (decision AF); if the type
    /// already has a field; or if its registered widget's
    /// [`ACCEPTS`](crate::widgets::LeafWidget::ACCEPTS) lacks this field's
    /// [`KIND`](crate::field::Field::KIND). The message names the type.
    #[must_use]
    pub fn field_for<F: Field + Send + Sync>(mut self, field: F) -> Self {
        let shape = F::Value::SHAPE;
        assert!(
            !is_built_in(shape),
            "`{shape}` is built in and reads the same in every form, so no registry can \
             give it a field. For a `{shape}` that reads differently, declare a type of \
             your own (`struct Shouty({shape});`) and register that"
        );
        let registered = self.types.entry(shape).or_default();
        if registered.field.is_some() {
            panic!("`{shape}` already has a field in this registry, and a type can have only one");
        } else if let Some(widget) = registered.widget.as_ref() {
            assert!(
                widget.accepts().contains(F::KIND),
                "`{shape}`'s widget, `{}`, cannot render its field, which reads {:?}",
                widget.name(),
                F::KIND
            );
        }
        Self::add_to_registry(registered, field);
        self
    }

    /// [`field_for`](Self::field_for) without its checks, for the built-in
    /// types it refuses: how every `types!` list's `registry()` gets them.
    /// Not API. Only `types!` calls it, from its own fixed list, and
    /// `every_built_in_is_one_is_built_in_accepts` pins that list against
    /// [`is_built_in`].
    #[doc(hidden)]
    pub fn __built_in<F: Field + Send + Sync>(mut self, field: F) -> Self {
        let registered = self.types.entry(F::Value::SHAPE).or_default();
        Self::add_to_registry(registered, field);
        self
    }

    fn add_to_registry<F: Field + Send + Sync>(registered: &mut Registered, field: F) {
        let formatter = field.clone();
        registered.field = Some(FieldInfo {
            field_type: F::KIND,
            node_ctor: NodeCtor::for_field(field),
            format_value: Arc::new(move |value: Peek<'_, '_>| {
                let value = value
                    .get::<F::Value>()
                    .expect("a registry keys each field by its Value's shape");
                formatter.format(value)
            }),
        });
    }

    /// Records `widget` for `T`. **No form reads it yet**: a leaf's widget is
    /// its path's `widget:` or its field's
    /// [`default_widget`](crate::field::Field::default_widget). Once forms
    /// read it, every path of type `T` will render with it unless the path
    /// names a `widget:` of its own, whichever field the path ends up with.
    ///
    /// `T` is named because a widget does not say what type it renders. For
    /// a newtype it is the newtype, so `widget_for::<Markdown>` changes
    /// `Markdown` and not `String`.
    ///
    /// # Panics
    ///
    /// If `T` already has a widget, or if the widget's
    /// [`ACCEPTS`](crate::widgets::LeafWidget::ACCEPTS) lacks the
    /// [`KIND`](crate::field::Field::KIND) of the field registered for `T`.
    /// The message names the type and the widget.
    #[must_use]
    pub fn widget_for<T: Facet<'static>>(mut self, widget: impl LeafWidget) -> Self {
        let shape = T::SHAPE;
        let widget = AnyWidget::new(widget);
        let registered = self.types.entry(shape).or_default();
        if let Some(widget) = registered.widget.as_ref() {
            panic!(
                "`{shape}` already has a widget in this registry, `{}`, and a type can have \
                 only one",
                widget.name()
            );
        } else if let Some(field_type) = registered.field.as_ref().map(|tac| tac.field_type) {
            assert!(
                widget.accepts().contains(field_type),
                "`{shape}`'s widget, `{}`, cannot render its field, which reads {field_type:?}",
                widget.name()
            );
        }
        registered.widget = Some(widget);
        self
    }

    /// The field a leaf of type `shape` gets, in decision AB.6's order once
    /// the path's own `field:` or settings have had their say: the installed
    /// overrides, then the form's own list, or the built-ins for a form with
    /// none (AD). `None` when neither knows the type, and the walk refuses it.
    pub(crate) fn field_in(
        types: Option<&'static TypeRegistry>,
        shape: &Shape,
    ) -> Option<&'static FieldInfo> {
        Self::find(types, shape, |registered| registered.field.as_ref())
    }

    /// The order itself, once, for every part of an entry: box 6's widget
    /// lookup will be `find(types, shape, |r| r.widget.as_ref())`.
    fn find<T>(
        types: Option<&'static TypeRegistry>,
        shape: &Shape,
        part: impl Fn(&'static Registered) -> Option<&'static T>,
    ) -> Option<&'static T> {
        let in_list = |list: &'static TypeRegistry| list.types.get(shape).and_then(&part);
        Self::registry().and_then(in_list).or_else(|| {
            types
                .or_else(|| Some(BuiltIns::registry()))
                .and_then(in_list)
        })
    }
}

/// One type's entry. The field and the widget are kept apart, not folded into
/// one constructor, because they are chosen separately: a path's `field:`
/// replaces the registered field and keeps the registered widget.
#[derive(Debug, Default)]
pub(crate) struct Registered {
    pub(crate) field: Option<FieldInfo>,
    pub(crate) widget: Option<AnyWidget>,
}

/// A registered field, erased: `Field` cannot be a trait object (it has an
/// associated type, an associated const and an `impl Future` method), so the
/// field lives inside the constructor, and its `KIND` is copied out beside
/// it for the widget check.
pub(crate) struct FieldInfo {
    pub(crate) field_type: FieldType,
    pub(crate) node_ctor: NodeCtor,
    pub(crate) format_value:
        Arc<dyn for<'mem, 'facet> Fn(Peek<'mem, 'facet>) -> String + Send + Sync>,
}

impl Debug for FieldInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldInfo")
            .field("field_type", &self.field_type)
            .field("node_ctor", &self.node_ctor)
            .field("format_value", &"Fn(Field::Value) -> String")
            .finish()
    }
}

#[diagnostic::on_unimplemented(
    message = "`{V}` has no field in `{Self}`",
    note = "`{Self}` is the `FormoxusTypes` in scope here. List `{V}` in a `formoxus::types!` \
            (`{V} => newtype String`, or `{V} => field YourField`) and point this module's \
            `FormoxusTypes` at that list, or name a field for this path with `field:`"
)]
pub trait FieldFor<V> {
    /// What a path's settings in `form!` configure: the field
    /// itself, or for a newtype entry its inner field.
    type Settings: Default;

    /// The field a path of type `V` gets.
    type Field: Field + Send + Sync;

    /// The field, from settings.
    fn build(settings: Self::Settings) -> Self::Field;
}

/// Every type a list holds beside the built-ins, with its field's
/// [`KIND`](crate::field::Field::KIND): what `form!`'s compile-time widget
/// checks judge a listed type by, in place of its shape (decision AB.10.3).
/// `types!` writes it; a `const fn` can read a const but cannot ask whether a
/// trait is implemented, so the list is spelled out as data.
#[doc(hidden)]
pub trait TypeKinds {
    const KINDS: &'static [(&'static Shape, FieldType)];
}

/// Whether `shape` is one of the built-in types: `String`, `bool`, and the
/// ten numbers from `i8` to `f64`. Not `usize`, `isize`, `char` or `i128`,
/// which a list may give a field. [`TypeRegistry::field_for`] refuses these,
/// since a built-in reads the same in every form (decision AF).
///
/// A second copy of `types!`' own list (`built_ins()` in formoxus-macros),
/// which the macro needs at compile time; a test keeps the two in step.
pub(crate) fn is_built_in(shape: &Shape) -> bool {
    shape == String::SHAPE
        || shape == bool::SHAPE
        || shape == u8::SHAPE
        || shape == u16::SHAPE
        || shape == u32::SHAPE
        || shape == u64::SHAPE
        || shape == i8::SHAPE
        || shape == i16::SHAPE
        || shape == i32::SHAPE
        || shape == i64::SHAPE
        || shape == f32::SHAPE
        || shape == f64::SHAPE
}

#[cfg(test)]
mod tests {
    use super::{BuiltIns, is_built_in};
    use facet::Facet;
    use googletest::prelude::*;

    /// The two copies of the built-in list agree: every type `types!`
    /// registers as built in is one `is_built_in` accepts, and every type it
    /// accepts is registered. A type added to one copy alone fails here,
    /// rather than as a refused `field_for` or an unrefused override.
    #[gtest]
    fn every_built_in_is_one_is_built_in_accepts() {
        let registered: Vec<_> = BuiltIns::registry().types.keys().copied().collect();
        for shape in &registered {
            expect_that!(is_built_in(shape), eq(true), "{shape}");
        }
        let built_ins = [
            String::SHAPE,
            bool::SHAPE,
            i8::SHAPE,
            i16::SHAPE,
            i32::SHAPE,
            i64::SHAPE,
            u8::SHAPE,
            u16::SHAPE,
            u32::SHAPE,
            u64::SHAPE,
            f32::SHAPE,
            f64::SHAPE,
        ];
        expect_that!(registered.len(), eq(built_ins.len()));
        for shape in built_ins {
            expect_that!(is_built_in(shape), eq(true), "{shape}");
            expect_that!(registered.contains(&shape), eq(true), "{shape}");
        }
    }

    /// Near misses: the numbers formoxus leaves out on purpose.
    #[gtest]
    fn is_built_in_refuses_what_is_not_built_in() {
        for shape in [usize::SHAPE, isize::SHAPE, char::SHAPE, i128::SHAPE] {
            expect_that!(is_built_in(shape), eq(false), "{shape}");
        }
    }
}
