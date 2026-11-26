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
//!         .field_for(Newtype::<Markdown, StringField>::new(StringField::default()))
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
//! No newtype is special, `#[facet(transparent)]` included: a
//! `struct Markdown(String)` needs a field like any other type of the app's
//! own. [`Newtype`](crate::fields::Newtype) is that field, reading and
//! writing it through its inner field, registered with `field_for`. A newtype
//! or struct with no field registered and no `field:` makes the build panic,
//! naming the type. A widget registered for `Markdown` changes `Markdown`
//! and leaves every other `String` alone.
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
        self
    }

    /// Registers `widget` for `T`: every path of type `T` renders with it,
    /// unless the path names a `widget:` of its own. It applies whichever
    /// field the path ends up with, the registered one, a `field:`, or a
    /// built-in.
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
        Self::registry()
            .and_then(in_list)
            .or_else(|| types.and_then(in_list)) // TODO: BuiltIns
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
