//! [`FormSpec<T>`] — the author's declaration: title, per-field overrides,
//! validator, and buttons. What `form!` builds, and what [`super::FormState`]
//! applies onto the tree it walks from the model's shape.

use std::{fmt::Debug, marker::PhantomData};

use facet::Facet;
use facet::Shape;
use heck::ToSnakeCase;
use indexmap::IndexMap;

use crate::buttons::ButtonSpec;
use crate::classes::ClassEdit;
use crate::classes::Classes;
use crate::error::ValidationError;
use crate::field_attrs::FieldAttrs;
use crate::field_kind::is_list;
use crate::field_kind::is_list_of_leaves;
use crate::field_kind::is_set;
use crate::label_case::LabelCase;
use crate::nodes::{SetFeedback, SetOrder};
use crate::path::ModelPath;
use crate::widgets::AnyWidget;
use crate::widgets::item_buttons::ItemAdder;

type ValidationFn<T> = Option<fn(&T) -> Vec<ValidationError<T>>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldSpec {
    pub label: Option<String>,
    /// The field's hint (`help_text:`), shown under its caption and
    /// describing its control.
    pub help_text: Option<String>,
    /// The field's own check (`validator:`), run once everything under the
    /// field is valid; its messages are the field's.
    pub validator: Option<crate::nodes::FieldValidator>,
    /// A list's or set's fewest and most items (`min_items:`, `max_items:`).
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub attrs: FieldAttrs,
    pub widget: Option<AnyWidget>,
    /// Builds this leaf during the shape walk, instead of the default field
    /// for its type. What `field:` and a typed field's settings expand to.
    pub node: Option<crate::field_node::NodeCtor>,
    pub adder: Option<ItemAdder>,
    pub treat_as_set: bool,
    pub sort_by: Option<SetOrder>,
    /// A set's words for what an edit did (`feedback:`), in place of
    /// formoxus's own.
    pub feedback: Option<SetFeedback>,
    /// A set's view of each item's text (`item:`), in place of
    /// [`ItemText`](crate::widgets::ItemText).
    pub item_view: Option<crate::widgets::ItemView>,
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
    /// `form!`'s `live_validation:`. `None` is "not stated", as for
    /// `label_case`.
    pub(super) live_validation: Option<bool>,
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
            live_validation: None,
            classes: None,
            edits: Vec::new(),
            _type: PhantomData,
        }
    }

    /// The shape `path` names, which every `with_*` taking a path checks,
    /// through [`field`](Self::field) or directly.
    ///
    /// # Panics
    ///
    /// If `path` names one item (`answers[2]`, `tags[=rust]`): a spec names
    /// every item at once (`answers[]`), and `path!` spells both, so this is
    /// the one mistake the type lets through (decision I in `PATHS_PLAN.md`).
    ///
    /// If `path` names no node in a form over `T`, as `spec_key_shape` judges
    /// it: the spec would be stored and never looked up, so it would do nothing
    /// at all. A `ModelPath` comes only from `path!` or `form!`, which refuse
    /// the same paths at compile time, so this is a backstop: the two checks
    /// are separate code, and a disagreement between them should be loud.
    fn shape_at(path: &ModelPath<T>) -> &'static Shape {
        if let Some(every_item) = every_item_spelling(path.as_str()) {
            panic!(
                "{path}: names one item, but a spec names every item at once; \
                 write `{every_item}`"
            );
        }
        crate::build::spec_key_shape(T::SHAPE, path.as_str()).unwrap_or_else(|why| {
            panic!("{path}: names nothing in a form over `{}`: {why}", T::SHAPE)
        })
    }

    /// The spec for `path`, made empty the first time a `with_*` names it.
    ///
    /// # Panics
    ///
    /// As [`shape_at`](Self::shape_at) does, if `path` names nothing.
    fn field(&mut self, path: &ModelPath<T>) -> &mut FieldSpec {
        Self::shape_at(path);
        self.fields.entry(path.to_string()).or_default()
    }

    /// The spec for `path`, as [`field`](Self::field) gives it, for a setting
    /// only a list can take. `key` names the setting, for the message.
    ///
    /// # Panics
    ///
    /// As `field` does, and if `path` names anything but a list.
    fn collection_field(&mut self, path: &ModelPath<T>, key: &str) -> &mut FieldSpec {
        let shape = Self::shape_at(path);
        assert!(
            is_list(shape) || is_set(shape),
            "{path}: `{key}` applies only to list or set, not `{shape}`"
        );
        self.field(path)
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

    pub fn with_widget(mut self, path: &ModelPath<T>, widget: impl Into<AnyWidget>) -> Self {
        self.field(path).widget = Some(widget.into());
        self
    }

    /// Build the leaf at `path` (a `[]` item path is fine) with `ctor` instead
    /// of the default for its type.
    pub fn with_node(mut self, path: &ModelPath<T>, ctor: crate::field_node::NodeCtor) -> Self {
        self.field(path).node = Some(ctor);
        self
    }

    pub fn with_label(mut self, path: &ModelPath<T>, l: &str) -> Self {
        self.field(path).label = Some(l.to_string());
        self
    }

    /// A check of the field at `path`, of any kind (`validator:` in `form!`):
    /// its messages are shown where that field shows its own errors. Runs
    /// only once everything under the field is valid, so a struct's check
    /// sees a whole struct, and only on a value that is there.
    pub fn with_field_validator(
        mut self,
        path: &ModelPath<T>,
        validator: crate::nodes::FieldValidator,
    ) -> Self {
        self.field(path).validator = Some(validator);
        self
    }

    /// The fewest items the list or set at `path` may have (`min_items:` in
    /// `form!`): checked in `validate`, Remove disabled at it, the legend
    /// marked required when it is at least 1, and a blank list (`empty_form`)
    /// built with that many blank items.
    pub fn with_min_items(mut self, path: &ModelPath<T>, min: usize) -> Self {
        self.field(path).min_items = Some(min);
        self
    }

    /// The most items the list or set at `path` may have (`max_items:` in
    /// `form!`): checked in `validate`, Add disabled at it, and a submission
    /// listing more refused before any of them are built.
    pub fn with_max_items(mut self, path: &ModelPath<T>, max: usize) -> Self {
        self.field(path).max_items = Some(max);
        self
    }

    /// A hint for the field at `path` (`help_text:` in `form!`): a line under
    /// its caption saying what it wants, which its control is described by
    /// (`aria-describedby`). On a list, set, struct or enum, under its legend.
    pub fn with_help_text(mut self, path: &ModelPath<T>, text: &str) -> Self {
        self.field(path).help_text = Some(text.to_string());
        self
    }

    pub fn with_adder(mut self, path: &ModelPath<T>, adder: ItemAdder) -> Self {
        self.collection_field(path, "adder").adder = Some(adder);
        self
    }

    pub fn treat_as_set(mut self, path: &ModelPath<T>) -> Self {
        let shape = Self::shape_at(path);
        assert!(
            !is_set(shape),
            "{path} is already a set; don't call `treat_as_set`"
        );
        assert!(
            is_list(shape),
            "{path}: `treat_as_set` applies only to a list, not `{shape}`"
        );
        assert!(
            is_list_of_leaves(shape),
            "{path}: `treat_as_set` applies only to a list of single values (a string, number, bool, enum without data, or newtype over one): a set's items are shown, not edited"
        );
        self.field(path).treat_as_set = true;
        self
    }

    pub fn with_sort_by(mut self, path: &ModelPath<T>, order: SetOrder) -> Self {
        self.set_field(path, "sort_by").sort_by = Some(order);
        self
    }

    /// Replace formoxus's words for what an edit to the set at `path` did
    /// ("“rust” was added successfully.") with the dev's.
    pub fn with_feedback(mut self, path: &ModelPath<T>, feedback: SetFeedback) -> Self {
        self.set_field(path, "feedback").feedback = Some(feedback);
        self
    }

    /// Show each item of the set at `path` with `view` instead of its bare
    /// text. Formoxus still renders the item's errors, Edit and Remove.
    pub fn with_item_view(mut self, path: &ModelPath<T>, view: crate::widgets::ItemView) -> Self {
        self.set_field(path, "item").item_view = Some(view);
        self
    }

    /// The spec of the set at `path`, for a key only a set takes: a
    /// `Def::Set`, or a list `treat_as_set` has already marked, since `form!`
    /// emits `treat_as_set` first (decision B in box 5).
    fn set_field(&mut self, path: &ModelPath<T>, key: &str) -> &mut FieldSpec {
        let shape = Self::shape_at(path);
        assert!(
            !is_list(shape) || self.field(path).treat_as_set,
            "{path}: if you want to treat this list as a set, call `treat_as_set` first"
        );
        assert!(
            is_set(shape) || is_list(shape),
            "{path}: `{key}` applies only to a set, not `{shape}`"
        );
        self.field(path)
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

    /// Whether this form checks fields as the user goes, overriding the app's
    /// [`Formoxus::live_validation`](crate::defaults::Formoxus::live_validation).
    /// `form!`'s `live_validation: on | off`.
    pub fn with_live_validation(mut self, live_validation: bool) -> Self {
        self.live_validation = Some(live_validation);
        self
    }

    pub fn with_classes(mut self, classes: Classes) -> Self {
        self.classes = Some(classes);
        self
    }

    pub fn buttons(&self) -> &[ButtonSpec] {
        &self.buttons
    }

    /// Attributes to render on the field at `path`. They only render: what
    /// formoxus validates (`min`, `max_length`, `pattern`, `required_true`, …)
    /// is a setting of the field, given with [`with_node`](Self::with_node).
    ///
    /// # Panics
    ///
    /// If `attrs` holds an attribute formoxus validates. It would render and
    /// never be checked, so the browser and the server would disagree.
    pub fn with_attrs(mut self, path: &ModelPath<T>, attrs: FieldAttrs) -> Self {
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

/// `None` for a spec path, whose only brackets are `[]`; for a path naming one
/// item, the spelling that names every item instead: each `[2]` or `[=rust]`
/// becomes `[]`, so `answers[2].text` gives `answers[].text`. A value never
/// holds a `]` of its own (`item_value_key` encodes it), so the first `]`
/// closes its bracket.
fn every_item_spelling(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    let mut changed = false;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..=open]);
        let close = rest[open..].find(']').map_or(rest.len(), |at| open + at);
        changed |= close != open + 1;
        out.push(']');
        rest = rest.get(close + 1..).unwrap_or("");
    }
    out.push_str(rest);
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use super::every_item_spelling;

    #[gtest]
    fn a_spec_path_needs_no_respelling() {
        expect_that!(every_item_spelling("title"), none());
        expect_that!(every_item_spelling("answers[].text"), none());
        expect_that!(every_item_spelling("rows[][]"), none());
        expect_that!(every_item_spelling("groups[].$Random.set[]"), none());
    }

    #[gtest]
    fn an_index_or_a_value_becomes_brackets() {
        expect_that!(every_item_spelling("answers[2]"), some(eq("answers[]")));
        expect_that!(
            every_item_spelling("answers[2].text"),
            some(eq("answers[].text"))
        );
        expect_that!(every_item_spelling("tags[=rust]"), some(eq("tags[]")));
        expect_that!(
            every_item_spelling("groups[1].$Random.set[=q%2E2]"),
            some(eq("groups[].$Random.set[]"))
        );
    }

    /// **One `[]` among indices is still a path naming one item,** and is
    /// respelled whole.
    #[gtest]
    fn a_mix_of_brackets_and_an_index_is_respelled() {
        expect_that!(every_item_spelling("rows[][0]"), some(eq("rows[][]")));
    }
}
