//! [`FormSpec<T>`] — the author's declaration: title, per-field overrides,
//! validator, and buttons. What `form!` builds, and what [`super::FormState`]
//! applies onto the tree it walks from the model's shape.

use std::{fmt::Debug, marker::PhantomData};

use facet::Facet;
use heck::ToSnakeCase;
use indexmap::IndexMap;

use crate::buttons::ButtonSpec;
use crate::classes::ClassEdit;
use crate::classes::Classes;
use crate::error::ValidationError;
use crate::field_attrs::FieldAttrs;
use crate::label_case::LabelCase;
use crate::widgets::AnyWidget;

type ValidationFn<T> = Option<fn(&T) -> Vec<ValidationError<T>>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldSpec {
    pub label: Option<String>,
    pub attrs: FieldAttrs,
    pub widget: Option<AnyWidget>,
    /// Builds this leaf during the shape walk, instead of the default field
    /// for its type. What `field:` and a typed field's settings expand to.
    pub node: Option<crate::field_node::NodeCtor>,
}

#[derive(Clone, Debug)]
pub struct FormSpec<T: Clone + Debug + Facet<'static>> {
    pub(super) title: Option<String>,
    pub(super) fields: IndexMap<String, FieldSpec>,
    pub(super) validator: ValidationFn<T>,
    /// Declaration order, which is display order — `form!` collects a `Vec`
    /// for exactly this reason.
    pub(super) buttons: Vec<ButtonSpec>,
    /// How a field's name becomes its label when nothing names one explicitly.
    ///
    /// `None` means "not stated at this level", not "Title" — so that a future
    /// app-level default read from context can fill it in, and an explicit
    /// per-form setting can still override that. See
    /// `.claude/memory/config_cascade.md`.
    pub(super) label_case: Option<LabelCase>,
    pub(super) use_browser_validation: Option<bool>,
    pub(super) classes: Option<Classes>,
    pub(super) edits: Vec<ClassEdit>,
    _type: PhantomData<T>,
}

impl<T: Clone + Debug + Facet<'static>> FormSpec<T> {
    pub fn with_class_edit(mut self, edit: ClassEdit) -> Self {
        self.edits.push(edit);
        self
    }
}

impl<T: Clone + Debug + Facet<'static>> FormSpec<T> {
    pub fn new() -> Self {
        Self {
            title: None,
            fields: IndexMap::new(),
            validator: None,
            buttons: Vec::new(),
            label_case: None,
            use_browser_validation: None,
            classes: None,
            edits: Vec::new(),
            _type: PhantomData,
        }
    }

    /// Set the casing every derived label in this form uses.
    ///
    /// Form-wide and not per-field, following `leptos_form`'s `rename_all`:
    /// casing is a consistency property of a whole form, and a form whose
    /// fields disagreed about it would just look broken.
    pub fn with_label_case(mut self, case: LabelCase) -> Self {
        self.label_case = Some(case);
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_widget(mut self, path: &str, widget: impl Into<AnyWidget>) -> Self {
        self.field(path).widget = Some(widget.into());
        self
    }

    /// Build the leaf at `path` (a `[]` row path is fine) with `ctor` instead
    /// of the default for its type.
    pub fn with_node(mut self, path: &str, ctor: crate::field_node::NodeCtor) -> Self {
        self.field(path).node = Some(ctor);
        self
    }

    pub fn with_label(mut self, path: &str, l: &str) -> Self {
        self.field(path).label = Some(l.to_string());
        self
    }

    pub fn with_validator(mut self, f: fn(&T) -> Vec<ValidationError<T>>) -> Self {
        self.validator = Some(f);
        self
    }

    /// Replaces rather than appends: `form!` allows one `buttons:` block, so
    /// a second call is a caller changing its mind, not adding to a list.
    pub fn with_buttons(mut self, buttons: Vec<ButtonSpec>) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn with_use_browser_validation(mut self, use_browser_validation: bool) -> Self {
        self.use_browser_validation = Some(use_browser_validation);
        self
    }

    pub fn with_classes(mut self, classes: Classes) -> Self {
        self.classes = Some(classes);
        self
    }

    pub fn buttons(&self) -> &[ButtonSpec] {
        &self.buttons
    }

    /// The spec for `path`, which every `with_*` taking a path goes through.
    ///
    /// # Panics
    ///
    /// If `path` names no node in a form over `T`. The spec would be stored
    /// and never looked up, so a typo would do nothing at all. `form!` only
    /// writes paths it has already checked at compile time.
    fn field(&mut self, path: &str) -> &mut FieldSpec {
        if let Some(why) = crate::build::spec_key_error(T::SHAPE, path) {
            panic!("{path}: names nothing in a form over `{}`: {why}", T::SHAPE);
        }
        self.fields.entry(path.to_string()).or_default()
    }

    /// Attributes to render on the field at `path`. They only render: what
    /// formoxus validates (`min`, `max_length`, `pattern`, `required_true`, …)
    /// is a setting of the field, given with [`with_node`](Self::with_node).
    ///
    /// # Panics
    ///
    /// If `attrs` holds an attribute formoxus validates. It would render and
    /// never be checked, so the browser and the server would disagree.
    pub fn with_attrs(mut self, path: &str, attrs: FieldAttrs) -> Self {
        if let Some(attr) = attrs.first_validated() {
            // `form!`'s spelling (`max_length`), not the HTML name (`maxlength`):
            // the one the dev wrote, and the field's setting is named the same.
            panic!(
                "{path}: `{}` is validated, so it is a setting of the field, not an \
                 attribute; give it with `with_node`",
                attr.variant_name().to_snake_case()
            );
        }
        self.field(path).attrs = attrs;
        self
    }
}

impl<T: Clone + Debug + Facet<'static>> Default for FormSpec<T> {
    fn default() -> Self {
        Self::new()
    }
}
