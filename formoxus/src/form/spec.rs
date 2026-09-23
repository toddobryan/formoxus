//! [`FormSpec<T>`] — the author's declaration: title, per-field overrides,
//! validator, and buttons. What `form!` builds, and what [`super::FormState`]
//! applies onto the tree it walks from the model's shape.

use std::{fmt::Debug, marker::PhantomData};

use facet::Facet;
use indexmap::IndexMap;

use crate::buttons::ButtonSpec;
use crate::error::FormError;
use crate::fields::Constraints;
use crate::label_case::LabelCase;
use crate::widgets::{SelectChoice, WidgetType};

#[derive(Clone, Debug)]
pub struct FormSpec<T: Clone + Debug + Facet<'static>> {
    pub(super) title: Option<String>,
    pub(super) fields: IndexMap<String, FieldSpec>,
    pub(super) validator: Option<fn(&T) -> Vec<FormError>>,
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
    _type: PhantomData<T>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldSpec {
    pub label: Option<String>,
    pub constraints: Constraints,
    pub custom_widget: Option<WidgetType>,
    /// What a `<select>` (or another chooser) offers.
    ///
    /// **On the field, not on [`WidgetType`].** A widget type is the dispatch
    /// discriminant and is compared for equality all over the tree; two selects
    /// offering different lists are still the same *kind* of widget. Keeping the
    /// list here also means `ScalarWidget`'s match arms do not have to bind or
    /// ignore a payload that dispatch never reads.
    ///
    /// **Not on `ValueKind` either** — choices are presentation, so swapping a
    /// select for a radio group must not change what the value parses as. See
    /// `.claude/memory/choice_fields_design.md`.
    pub choices: Option<Vec<SelectChoice>>,
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

    pub fn with_custom_widget(mut self, path: &str, c: WidgetType) -> Self {
        self.field(path).custom_widget = Some(c);
        self
    }

    /// Offer `path` a fixed set of choices.
    ///
    /// Takes anything iterable of anything convertible, so a `const` table of
    /// pairs is as good as a built `Vec`:
    ///
    /// ```ignore
    /// const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];
    /// spec.with_choices("state", STATES)
    /// ```
    ///
    /// A choice's `value` is written into the value map verbatim, so it has to
    /// be exactly what this field's type parses from — and it may never be
    /// `""`, which IS absence. The "no value" entry is `Select`'s own, and
    /// offering a second one would make an answered field indistinguishable
    /// from an unanswered one.
    pub fn with_choices(
        mut self,
        path: &str,
        choices: impl IntoIterator<Item = impl Into<SelectChoice>>,
    ) -> Self {
        self.field(path).choices = Some(choices.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_label(mut self, path: &str, l: &str) -> Self {
        self.field(path).label = Some(l.to_string());
        self
    }

    pub fn with_validator(mut self, f: fn(&T) -> Vec<FormError>) -> Self {
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

    pub fn buttons(&self) -> &[ButtonSpec] {
        &self.buttons
    }

    fn field(&mut self, path: &str) -> &mut FieldSpec {
        self.fields.entry(path.to_string()).or_default()
    }

    pub fn with_constraints(mut self, path: &str, constraints: Constraints) -> Self {
        self.field(path).constraints = constraints;
        self
    }
}

impl<T: Clone + Debug + Facet<'static>> Default for FormSpec<T> {
    fn default() -> Self {
        Self::new()
    }
}
