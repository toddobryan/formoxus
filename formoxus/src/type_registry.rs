//! The app's own types: which [`Field`] reads one, and which widget renders
//! it, for every form in the app.
//!
//! A model derives only `Facet`, so formoxus cannot tell from a
//! `struct TimeLimit(u32)` that the user types `1h15m`. Two places can say
//! so. `field:` in [`form!`](macro@crate::form) says it for one path; a
//! [`TypeRegistry`] says it for the type, wherever the type sits: a struct's
//! field, a list or set item, a variant's field, or the inside of an
//! `Option`. The model stays plain either way.
//!
//! ```ignore
//! fn main() {
//!     TypeRegistry::new()
//!         .field_for(TimeLimitField)
//!         .widget_for::<Markdown>(MarkdownWidget)
//!         .install();
//!     dioxus::launch(App);
//! }
//! ```
//!
//! # What wins
//!
//! A path is more specific than a type, and a type than a built-in:
//!
//! 1. The field: the path's `field:`, else the field registered for the
//!    path's type, else the built-in for the type (a `String`, a number, a
//!    `bool`). Registering a field for a built-in type replaces the built-in
//!    everywhere in the app.
//! 2. The widget: the path's `widget:`, else the widget registered for the
//!    path's type, else the chosen field's
//!    [`default_widget`](crate::field::Field::default_widget). A registered
//!    widget applies even under a `field:`.
//!
//! A `#[facet(transparent)]` type needs no registered field, since it reads
//! and writes as its inner type. Its widget is still looked up by its own
//! type, so `widget_for::<Markdown>` changes a transparent `Markdown(String)`
//! and leaves every other `String` alone. A plain newtype or struct with no
//! field registered and no `field:` makes the build panic, naming the type.
//!
//! # Why installed, not provided
//!
//! The registry is a process-wide value set once in `main`, not Dioxus
//! context. A form is built on the client and rebuilt on the server, and a
//! server function runs outside any component, so context would reach the
//! first and not the second, and the two would read the same text
//! differently. Both run `main`, so both see what `main` installs. For the
//! same reason the registry is not part of
//! [`Formoxus`](crate::defaults::Formoxus), which a subtree can provide for
//! itself.
//!
//! `form!` cannot see the registry: its checks run at compile time. So a
//! `widget:` on a path whose type is registered is checked against the
//! registered field's [`KIND`](crate::field::Field::KIND) when the form is
//! built.

use std::collections::HashMap;
use std::sync::OnceLock;

use facet::{Facet, Shape};
use formoxus_spec::FieldType;

use crate::field::Field;
use crate::nodes::NodeCtor;
use crate::widgets::{AnyWidget, LeafWidget};

/// Set once, by [`TypeRegistry::install`]. Empty until then, which is not an
/// error: a form over built-in types needs no registry.
static REGISTRY: OnceLock<TypeRegistry> = OnceLock::new();

/// The fields and widgets the app gives its own types. Built in `main` and
/// [installed](Self::install) before `launch`; see the
/// [module docs](crate::type_registry) for what it is for and what wins.
///
/// Each type gets at most one field and one widget, and a widget that
/// cannot render its type's field is refused. Both panic in the builder call
/// that breaks the rule, so a mistake stops the app at startup rather than
/// when a form first meets the type.
///
/// A type is identified by its `Shape`, which is per concrete type: a
/// generic type is registered once for each instantiation, `Ref<Question>`
/// and `Ref<Assignment>` separately.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Keyed by `&Shape`, whose `Eq` and `Hash` compare type ids rather than
    /// addresses, so two copies of one type's shape find the same entry.
    pub(crate) types: HashMap<&'static Shape, Registered>,
}

impl TypeRegistry {
    /// The installed registry, or `None` before [`install`](Self::install) or
    /// in an app that never calls it. `None` reads as an empty registry.
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

    /// Makes this the registry every form in the process is built with.
    ///
    /// Call it at the top of `main`, before `launch`. The browser and the
    /// server each run `main`, so each installs the same registry, and a
    /// form built before this call sees no registered types at all.
    ///
    /// # Panics
    ///
    /// If a registry is already installed. There is one per process, so a
    /// second would mean two parts of the app disagreeing about a type.
    pub fn install(self) {
        assert!(
            REGISTRY.get().is_none(),
            "The TypeRegistry was already installed. It can't be installed twice."
        );
        match REGISTRY.set(self) {
            Ok(()) => (),
            _ => panic!("The TypeRegistry could not be installed."),
        }
    }

    /// Registers `field` for its [`Value`](crate::field::Field::Value) type:
    /// every path of that type parses, formats and checks through it,
    /// unless the path names a `field:` of its own.
    ///
    /// The type comes from the field, so a field cannot be registered for the
    /// wrong one. `Send + Sync` because every form in the process shares it.
    ///
    /// # Panics
    ///
    /// If the type already has a field, or if its registered widget's
    /// [`ACCEPTS`](crate::widgets::LeafWidget::ACCEPTS) lacks this field's
    /// [`KIND`](crate::field::Field::KIND). The message names the type.
    #[must_use]
    pub fn field_for<F: Field + Send + Sync>(mut self, field: F) -> Self {
        let shape = F::Value::SHAPE;
        let registered = self.types.entry(shape).or_default();
        if let Some(field_type) = registered.field.as_ref().map(|tac| tac.field_type) {
            panic!("field type {field_type:?} already registered for type {shape}");
        } else if let Some(widget) = registered.widget.as_ref() {
            assert!(
                widget.accepts().contains(F::KIND),
                "widget {} not compatible with type {}",
                widget.name(),
                shape
            );
        }
        registered.field = Some(TypeAndCtor {
            field_type: F::KIND,
            node_ctor: NodeCtor::for_field(field),
        });
        self
    }

    /// Registers `widget` for `T`: every path of type `T` renders with it,
    /// unless the path names a `widget:` of its own. It applies whichever
    /// field the path ends up with, the registered one, a `field:`, or a
    /// built-in.
    ///
    /// `T` is named because a widget does not say what type it renders. For
    /// a `#[facet(transparent)]` type it is the outer type, so
    /// `widget_for::<Markdown>` changes `Markdown` and not `String`.
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
                "type {shape} already has {} registered as a widget",
                widget.name()
            );
        } else if let Some(field_type) = registered.field.as_ref().map(|tac| tac.field_type) {
            assert!(
                widget.accepts().contains(field_type),
                "widget {} not compatible with type {}",
                widget.name(),
                shape
            );
        }
        registered.widget = Some(widget);
        self
    }
}

/// One type's entry. The field and the widget are kept apart, not folded into
/// one constructor, because they are chosen separately: a path's `field:`
/// replaces the registered field and keeps the registered widget.
#[derive(Debug, Default)]
pub(crate) struct Registered {
    pub(crate) field: Option<TypeAndCtor>,
    pub(crate) widget: Option<AnyWidget>,
}

/// A registered field, erased: `Field` cannot be a trait object (it has an
/// associated type, an associated const and an `impl Future` method), so the
/// field lives inside the constructor, and its `KIND` is copied out beside
/// it for the widget check.
#[derive(Debug)]
pub(crate) struct TypeAndCtor {
    pub(crate) field_type: FieldType,
    pub(crate) node_ctor: NodeCtor,
}
