use std::ops::Deref;
use std::rc::Rc;

use dioxus::core::{Attribute, Callback};
use dioxus::signals::Signal;
use dioxus::stores::Store;
use formoxus_spec::Tag;

use crate::bound_field::{BoundField, Leaf, NodeKind, OnFieldEvent, Set};
use crate::classes::Classes;
use crate::defaults::Formoxus;
use crate::field_attrs::FieldAttrs;
use crate::fields::Choice;
use crate::form::Contents;
use crate::nodes::FormWide;
use crate::path::NodePath;
use crate::stores::{ErrorsByPath, ErrorsView, FilesSignal, ValuesByPath};
use crate::widgets::widget::Widget;

/// What a widget is handed to render one node: its bound field, which these
/// props deref to, so `props.current()` and `props.on_input()` work
/// directly; the attributes for its form control; and its choices.
///
/// The node builds these on every render, so they always hold the field's
/// current label, attributes and choices. A widget that renders another
/// widget passes its own props on, cloned, rather than building new ones;
/// [`detached`](Self::detached) is for a widget outside any form.
///
/// `K` is the node's kind, [`Leaf`] unless said otherwise: every
/// [`LeafWidget`](crate::widgets::LeafWidget) takes `WidgetProps`, and
/// [`VariantSelect`](crate::widgets::VariantSelect) takes
/// `WidgetProps<Variant>`.
///
/// [`Leaf`]: crate::bound_field::Leaf
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetProps<K: NodeKind = Leaf> {
    /// The field's attributes BEFORE its errors are applied: its own, its
    /// constraints and `required`, which the node can work out in the form's
    /// scope. [`current_attrs`](Self::current_attrs) adds `aria-invalid` and
    /// `aria-describedby`, reading the store, so a widget spreads that and
    /// never this. Read this directly only where the error attributes don't
    /// matter: a `contains` check, or a `class`.
    ///
    /// Crate-private so a custom widget can't spread this by mistake: it would
    /// compile, look right, and silently lose both aria attributes.
    ///
    /// One exception: `VariantNode` passes the attributes with the error
    /// attributes ALREADY applied, as its select's `help_text` is `None`
    /// (see there), and `VariantSelect` spreads this as is.
    pub(crate) base_attrs: FieldAttrs,
    /// Attributes beyond the field's own, for the form control. Empty from
    /// the form: they are how a widget that renders another adds attributes
    /// to the inner one's element. A widget merges them into what it
    /// spreads, as [`current_attrs`](Self::current_attrs) shows.
    pub extra_attrs: Vec<Attribute>,
    /// The field's choices, read from it on every render (`Field::choices`),
    /// so a field whose choices change is shown with the new ones. `None` for
    /// a field without any; a chooser over a `bool` then offers true/false.
    pub choices: Option<Vec<Choice>>,
    pub(crate) field: BoundField<K>,
}

impl WidgetProps {
    /// Props connected to no form, for rendering a widget on its own: in a
    /// test, or on a page that shows a widget outside any form. A form's
    /// widgets are never built this way; their node builds their props.
    ///
    /// The field is at `path`, and its control has the page id `id`. `attrs`
    /// are its attributes before its errors are applied, which
    /// [`current_attrs`](Self::current_attrs) adds. The form state these
    /// props read starts from the two maps: `values` fills the value store,
    /// and `errors` the error store, so a widget can be shown with a value
    /// and with errors. The props have no label, hint, choices or extra
    /// attributes: set the first two with [`with_label`](Self::with_label) and
    /// [`with_help_text`](Self::with_help_text), and the other two, which are
    /// `pub`, directly.
    ///
    /// **Detached means a write reaches nothing but these props' own stores.**
    /// The bound field writes the value store and tells nobody: no check on
    /// blur or as the user types, and no edit goes anywhere. So **a widget
    /// that renders another widget must not build its props here.** That
    /// compiles and renders, but the inner widget is then outside the form:
    /// its value never reaches the wire, and it is never checked before
    /// submit. Pass on a clone of your own props instead.
    ///
    /// The stores are new, and so is the empty file table, so a file widget
    /// built from these props starts with no file picked. They are signals,
    /// kept until the calling component unmounts, so each call makes a few
    /// more: called in a component's body, that is a few more per render,
    /// which a test of one widget can afford. Call it inside `use_hook` if
    /// that ever matters, and also when the widget's writes must survive a
    /// re-render, since props built on every render start empty every time.
    pub fn detached(
        id: impl Into<String>,
        path: NodePath,
        attrs: FieldAttrs,
        values: ValuesByPath,
        errors: ErrorsByPath,
        classes: Rc<Classes>,
    ) -> Self {
        let form = FormWide {
            // No form, so no form id. Nothing reads it from here: a bound
            // field takes its page id from `id`, not from the form's.
            form_id: String::new(),
            contents: Contents {
                values: Store::new(values.into_iter().collect()),
                files: FilesSignal::default(),
                resets: Signal::default(),
            },
            errors: ErrorsView(Store::new(
                errors
                    .into_iter()
                    .map(|(path, messages)| (path, Some(messages)))
                    .collect(),
            )),
            on_edit: Callback::new(|_| {}),
            on_field_event: OnFieldEvent::unreported(),
            // Nothing reads it from here either: a bound field never does,
            // and `VariantSelect` is handed its casing as a prop. The
            // built-in, from the one place the built-ins live, rather than a
            // `LabelCase::default()`, which would invite skipping the cascade.
            label_case: Formoxus::default().label_case,
            classes,
        };
        Self {
            base_attrs: attrs,
            extra_attrs: Vec::new(),
            choices: None,
            field: BoundField::new(id.into(), path, None, None, Rc::new(form)),
        }
    }

    /// [`detached`](Self::detached) props with a label, which the widget
    /// shows as its caption.
    pub fn with_label(self, label: impl Into<String>) -> Self {
        Self {
            field: self.field.with_label(label.into()),
            ..self
        }
    }

    /// [`detached`](Self::detached) props with a hint, which the widget shows
    /// and its control's `aria-describedby` names.
    pub fn with_help_text(self, help_text: impl Into<String>) -> Self {
        Self {
            field: self.field.with_help_text(help_text.into()),
            ..self
        }
    }
}

impl<K: NodeKind> Deref for WidgetProps<K> {
    type Target = BoundField<K>;

    fn deref(&self) -> &BoundField<K> {
        &self.field
    }
}

impl<K: NodeKind> WidgetProps<K> {
    /// The field's attributes as its form control should carry them: its own
    /// from `form!` and its constraints, `required`, and then the two that
    /// follow its errors, `aria-invalid` and `aria-describedby`. A widget spreads
    /// these, with [`extra_attrs`](Self::extra_attrs) merged in, onto the element
    /// it renders:
    ///
    /// ```ignore
    /// let attrs = props.current_attrs().merge_with_attrs(props.extra_attrs.clone());
    /// ```
    ///
    /// This is the only way a custom widget reaches the field's attributes. If
    /// a widget skipped it, its markup would look right, but a screen reader
    /// would never hear that the field is invalid, or read its hint and errors.
    ///
    /// **Call it from a component.** It reads the field's errors from the
    /// store, and a read subscribes whatever is rendering at that moment.
    /// [`LeafWidget::render`](super::LeafWidget::render) runs in the form's scope, so calling it there makes
    /// every error re-render the whole form. Have `render` return a
    /// `#[component]` that takes the props and calls this, as the built-in
    /// widgets do (`TextareaView`, `SelectView`). Then an error re-renders only
    /// that field.
    ///
    /// `aria-describedby` names the hint only when [`help_text`](BoundField::help_text)
    /// is set, so a widget that renders the hint itself still passes it here.
    pub fn current_attrs(&self) -> FieldAttrs {
        self.base_attrs
            .with_error_attrs(self.id(), self.help_text(), self.validity())
    }

    /// The form control's `class`: `part`'s slot, as the field's own `class`
    /// and `class_plus` replace or extend it.
    pub fn control_class<W: Widget>(&self, tag: Tag) -> Option<String> {
        self.base_attrs
            .class(self.classes().part::<W>(tag), self.is_invalid())
    }

    /// The bound field, owned, for an event handler, which must be `'static`.
    /// For a read, the props deref to it.
    pub fn field(&self) -> BoundField<K> {
        self.field.clone()
    }
}

/// What a set widget is handed to render a whole set as one control: its
/// [`SetWidget`](crate::widgets::SetWidget)'s `render` takes these, as
/// [`CheckboxMultiple`](crate::widgets::CheckboxMultiple) and
/// [`SelectMultiple`](crate::widgets::SelectMultiple) do. They deref to the
/// set's bound field, which is how the widget adds and removes items.
#[derive(Clone, Debug, PartialEq)]
pub struct SetWidgetProps {
    /// Every value the set may hold, each one a box to tick or an option to
    /// select.
    pub choices: Vec<Choice>,
    /// The values the set holds now, as raw strings, in its sorted order.
    pub chosen: Vec<String>,
    /// Whether the set needs at least one item (`min_items:` above zero),
    /// for the required marker.
    pub required: bool,
    /// The most items it may hold (`max_items:`), for a widget that stops
    /// offering more there.
    pub max_items: Option<usize>,
    pub(crate) field: BoundField<Set>,
}

impl SetWidgetProps {
    /// The set's bound field, owned, for an event handler, which must be
    /// `'static`. For a read, the props deref to it.
    pub fn field(&self) -> BoundField<Set> {
        self.field.clone()
    }
}

impl Deref for SetWidgetProps {
    type Target = BoundField<Set>;

    fn deref(&self) -> &BoundField<Set> {
        &self.field
    }
}
