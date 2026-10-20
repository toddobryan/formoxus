//! The leaf node: one [`Field`] plus the tree state every field shares.

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek, ReflectError, ScalarType, Shape};
use formoxus_spec::{BuiltinWidget, FieldType, RenderedInputType};

use crate::error::{FormAccessError, ValidationMessage};
use crate::field::{Field, FieldValue, populate};
use crate::field_attrs::FieldAttrs;
use crate::form::ErrorsByPath;
use crate::form::FieldSpec;
use crate::label_case::LabelCase;
use crate::nodes::{Edit, Node, NodeContext, SpecsByPath, ValuesByPath, default_label, qualify};
use crate::widgets::{FieldProps, ScalarWidget, WidgetType};

/// Everything the walk knows about a leaf when it builds one.
///
/// What a [`NodeCtor`] is handed, and what `dispatch!` builds its default
/// leaves from, so both go through [`LeafArgs::node`]. `path` is the leaf's
/// real path (`venues.#3.capacity`), for messages; `spec` was matched on its
/// `[]` form (`venues[].capacity`).
#[derive(Debug)]
pub struct LeafArgs<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub peek: Option<Peek<'a, 'static>>,
    pub optional: bool,
    pub wrapper: Option<&'static Shape>,
    /// The spec that names this leaf, if any.
    pub spec: Option<&'a FieldSpec>,
}

impl LeafArgs<'_> {
    /// The leaf node over `field`, with the spec's `label`, `attrs`, widget
    /// and `choices` applied.
    ///
    /// The one place a leaf is assembled, whether `dispatch!` picked the field
    /// or a spec's constructor did, so a constructor cannot forget the rest of
    /// what the spec says. `attrs` is taken whole: nothing derives one, so
    /// there is nothing for the spec's to merge with.
    pub fn node<F: Field>(self, field: F) -> FieldNode<F> {
        let spec = self.spec;
        FieldNode {
            name: self.name.to_string(),
            label: spec.and_then(|s| s.label.clone()),
            optional: self.optional,
            attrs: spec.map(|s| s.attrs.clone()).unwrap_or_default(),
            custom_widget: spec.and_then(|s| s.custom_widget),
            wrapper: self.wrapper,
            value: populate::<F::Value>(self.peek),
            errors: Vec::new(),
            field,
        }
    }
}

/// How a spec builds one leaf, in place of the default for its type.
///
/// An `Arc<dyn Fn>`, not a `fn` pointer: a `fn` pointer could bake in
/// compile-time settings (`min: 3`), but not anything built at runtime, like
/// Step 8's `choices: |ctx| …`.
#[derive(Clone)]
pub struct NodeCtor(
    #[allow(clippy::type_complexity)]
    pub  std::sync::Arc<dyn for<'a> Fn(LeafArgs<'a>) -> Box<dyn Node> + Send + Sync>,
);

impl NodeCtor {
    pub fn new(f: impl for<'a> Fn(LeafArgs<'a>) -> Box<dyn Node> + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(f))
    }

    pub(crate) fn build(&self, args: LeafArgs<'_>) -> Box<dyn Node> {
        (self.0)(args)
    }
}

impl std::fmt::Debug for NodeCtor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NodeCtor(..)")
    }
}

/// Two constructors are equal only when they are the same `Arc`; a closure has
/// no other identity worth comparing.
impl PartialEq for NodeCtor {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldNode<F: Field> {
    pub name: String,
    pub label: Option<String>,
    pub optional: bool,
    /// Moves into the fields that validate them in Step 4 of `FIELDS_PLAN.md`.
    pub attrs: FieldAttrs,
    /// Becomes `widget: AnyWidget` in Step 5, resolved once by `build.rs`.
    pub custom_widget: Option<WidgetType>,
    /// The newtype this field's value is wrapped in — `Markdown` for a
    /// `String` field standing in for a `Markdown` field. `None` for an
    /// ordinary scalar.
    ///
    /// **The field carries the INNER type, not the newtype**, because a
    /// concrete type cannot be recovered from a runtime `&'static Shape`: the
    /// shape walk only ever has a shape in hand, and `F::Value` is fixed at
    /// compile time. So a `Markdown` field becomes a `String` field that
    /// remembers what to re-wrap it in, and every string-facing operation —
    /// parsing, display, `check`, the widget — goes on working unchanged
    /// against the inner scalar.
    ///
    /// Only [`write_value_into`](Node::write_value_into) consults it.
    pub wrapper: Option<&'static Shape>,
    pub value: FieldValue<F::Value>,
    pub errors: Vec<ValidationMessage>,
    pub field: F,
}

impl<F: Field> FieldNode<F> {
    /// The value family this field carries, and the constraints that apply to it.
    ///
    /// Derived from `F::Value` on every read rather than stored, so it cannot go
    /// stale and nothing about presentation is committed during the SHAPE walk.
    ///
    /// Bounds on integers are i128, since that's the only type that can represent
    /// the max and min values on all other int types. Similarly, bounds on floats are
    /// f64s. In the form! macro (where we have access to the actual type of the field),
    /// we check to make sure the constraints fit.
    fn field_type(&self) -> FieldType {
        // `None` is unreachable: `scalar_node` only builds a `FieldNode` for
        // the scalars it recognises, and `node_for_shape` panics on the rest.
        let scalar = <F::Value as Facet<'static>>::SHAPE
            .scalar_type()
            .expect("FieldNode is only constructed for scalar shapes");

        match scalar {
            ScalarType::String => FieldType::Text,
            ScalarType::Bool => FieldType::Bool,
            ScalarType::I8
            | ScalarType::I16
            | ScalarType::I32
            | ScalarType::I64
            | ScalarType::U8
            | ScalarType::U16
            | ScalarType::U32
            | ScalarType::U64 => FieldType::Int,
            ScalarType::F32 | ScalarType::F64 => FieldType::Float,
            other => panic!(
                "scalar type {other:?} is not supported in FieldNode (field {})",
                self.name
            ),
        }
    }

    /// What this field renders as absent any override.
    ///
    /// `optional` is the one input here that `F::Value` cannot supply: `Option`
    /// peeling wraps rather than parameterizes, so `bool` and `Option<bool>`
    /// both arrive as a `bool` field. A checkbox has two states and an
    /// `Option<bool>` has three, which is the whole reason that flag has to
    /// travel from the walk.
    fn default_widget(&self) -> WidgetType {
        if self.field.choices().is_some() {
            WidgetType::Builtin(BuiltinWidget::Select)
        } else {
            WidgetType::Builtin(BuiltinWidget::default_for(self.field_type(), self.optional))
        }
    }

    /// The widget to render: an override if one was set, else the derived default.
    fn widget(&self) -> WidgetType {
        self.custom_widget.unwrap_or_else(|| self.default_widget())
    }

    fn is_unticked_checkbox(&self) -> bool {
        matches!(self.value, FieldValue::Empty)
            && matches!(
                self.widget(),
                WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Checkbox)),
            )
    }

    /// The raw value `validate` checks: `"false"` for an unticked checkbox,
    /// and [`Node::raw_value`] for everything else.
    ///
    /// An unticked checkbox stays `Empty`, so that `is_present` keeps one
    /// meaning. The places that CONSUME the value know it means `false` instead.
    /// `write_value_into` writes `false` for one, and this is the same rule for
    /// `validate`. That way a rule about a bool sees an untouched box and an
    /// unticked one alike.
    fn raw_value_to_validate(&self) -> String {
        if self.is_unticked_checkbox() {
            "false".to_string()
        } else {
            self.raw_value()
        }
    }
}

impl<F: Field> Node for FieldNode<F> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn raw_value(&self) -> String {
        match &self.value {
            FieldValue::Empty => String::new(),
            // Formatted through facet's display vtable rather than a `Display`
            // bound on the value — the exact mirror of `parse_scalar` going the
            // other way. `{t:?}` would be wrong here: `Debug` quotes strings, and
            // `parse_scalar` faithfully parses those quotes back into the value.
            FieldValue::Valid(t) => Peek::new(t).to_string(),
            FieldValue::Invalid { raw, .. } => raw.clone(),
        }
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        out.push((qualify(prefix, &self.name), self.raw_value()));
    }

    fn distribute_values(&mut self, prefix: &str, values: &ValuesByPath) {
        let Some(raw) = values.get(&qualify(prefix, &self.name)) else {
            return; // nothing supplied for this field; leave it as it stands
        };

        // An empty input means "unfilled", which is what `Empty` encodes —
        // that's what lets required-validation still fire on a blanked field.
        if raw.is_empty() {
            self.value = FieldValue::Empty;
            return;
        }

        self.value = match self.field.parse(raw) {
            Ok(t) => FieldValue::Valid(t),
            Err(error) => FieldValue::Invalid {
                raw: raw.clone(),
                error,
            },
        };
    }

    fn distribute_errors(&mut self, prefix: &str, errors: &mut ErrorsByPath) {
        if let Some(found) = errors.shift_remove(&qualify(prefix, &self.name)) {
            self.errors.extend(found);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut ErrorsByPath) {
        // Clean fields contribute nothing — see the trait's contract. Inserting
        // `(path, [])` here would make `FormErrors.fields` non-empty for a form
        // that passed, so a caller could not read "did it pass?" off the shape.
        if self.errors.is_empty() {
            return;
        }
        out.insert(qualify(prefix, &self.name), self.errors.clone());
    }

    fn render(&self, ctx: &NodeContext) -> Element {
        let widget = self.widget();
        // The field's own attributes (`min`, `pattern`, …), so the browser
        // checks what `check` does, then the author's, which only render.
        let field_attrs = FieldAttrs::with_owned_attrs(
            &ctx.id(&self.name),
            &widget,
            ctx.required,
            self.has_errors_here(),
            &self.field.widget_attrs().then(&self.attrs),
        );

        rsx! {
            ScalarWidget {
                field_type: self.field_type(),
                field_attrs: field_attrs,
                widget: widget,
                choices: self.field.choices(),
                values: ctx.values,
                props: FieldProps {
                    id: ctx.id(&self.name),
                    path: ctx.path(&self.name),
                    label: self.label(ctx.label_case),
                    classes: ctx.classes.clone(),
                    errors: self.errors.clone(),
                },
            }
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        // An `Invalid` value carries its own parse error, and this is the only
        // route that error has to the screen: the widget boundary is
        // `(path, label, required, errors)`, so a widget cannot reach into
        // `FieldValue` to find it. Hoisting here rather than merging in
        // `render` also keeps one answer to "what is wrong with this field".
        //
        // The `match` is what keeps a parse error from collecting a spurious
        // "required" on top of it — exactly one error either way.
        let error = match &self.value {
            FieldValue::Empty if !self.is_unticked_checkbox() => {
                Some(ValidationMessage("This field is required.".to_string()))
            }
            FieldValue::Invalid { error, .. } => Some(error.clone()),
            _ => None,
        };
        // An invalid value or a missing one stops here: it gets exactly one
        // error, and an empty field cannot fail a constraint.
        if let Some(error) = error {
            self.errors.push(error);
            return;
        }
        // What is left is a `Valid` value or an unticked checkbox, the only
        // `Empty` the match above lets through. An unticked checkbox means
        // `false`, so the field checks it as `false`, parsed the field's way.
        let raw = self.raw_value_to_validate();
        if let FieldValue::Valid(value) = &self.value {
            self.errors.extend(self.field.check(value));
        } else {
            let value = self
                .field
                .parse(&raw)
                .expect("`Boolean` input kind is only ever derived from a `bool` shape");
            self.errors.extend(self.field.check(&value));
        }
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn has_errors_within(&self) -> bool {
        self.has_errors_here() || matches!(self.value, FieldValue::Invalid { .. })
    }

    fn has_errors_here(&self) -> bool {
        !self.errors.is_empty()
    }

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        // The one place `wrapper` matters. The slot the parent opened is the
        // NEWTYPE's, so a bare `set` of the inner scalar would be a type error;
        // descending into field 0, setting there, and coming back up builds the
        // wrapper around it. `None` is the ordinary case and stays a plain set.
        let set = |p: Partial<'p>, t: F::Value| -> Result<Partial<'p>, ReflectError> {
            match self.wrapper {
                None => p.set(t),
                Some(_) => p.begin_nth_field(0)?.set(t)?.end(),
            }
        };
        let partial = match &self.value {
            FieldValue::Valid(t) => set(partial, t.clone())?,
            // An unticked checkbox is `Empty` like any other unfilled input, and
            // stays that way so `is_present` keeps one meaning. `false` is
            // synthesised here instead, at the last possible moment. Going
            // through the field's `parse` rather than `partial.set(false)` is
            // what keeps this generic: nothing in scope can prove
            // `F::Value == bool`, but the parse resolves the real shape at
            // runtime and doesn't need to be told.
            FieldValue::Empty if self.is_unticked_checkbox() => set(
                partial,
                self.field
                    .parse("false")
                    .expect("`Boolean` input kind is only ever derived from a `bool` shape"),
            )?,
            // Required-vs-optional was decided from the Model's own shape at
            // construction time (`Def::Option` — see the earlier discussion):
            // `required == false` means the Model's field is really
            // `Option<T>`, so the value written back has to be wrapped/`None`
            // to match, not the bare `T` the `required` branch writes.
            _ => {
                unreachable!("write_into should only run after validate() has confirmed no errors")
            }
        };
        Ok(partial)
    }

    fn is_present(&self) -> bool {
        self.value != FieldValue::Empty
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
    }

    fn edit(
        &mut self,
        prefix: &str,
        edit: &Edit,
        _specs: &SpecsByPath,
    ) -> Result<(), FormAccessError> {
        // A leaf can only ever be the wrong answer, but which wrong answer is
        // worth saying: hitting a real field means the caller's path was right
        // and its *expectation* was wrong.
        let path = edit.path();
        Err(if path == qualify(prefix, &self.name) {
            // The article has to travel with the noun, so this carries both.
            let wanted = match edit {
                Edit::ChooseVariant { .. } => "an enum",
                Edit::AddRow { .. } | Edit::RemoveRow { .. } => "a list",
            };
            FormAccessError(format!("{path} is a field, not {wanted}"))
        } else {
            FormAccessError(format!("{path} is a field, so edits cannot be applied"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::{BoolField, NumberField, StringField};
    use googletest::prelude::*;

    // What each field's `check` allows is tested beside the field. These test
    // the NODE: that `validate` hands the field its value, and the cases only
    // the node can decide, like what an empty bool means.

    fn node<F: Field>(value: FieldValue<F::Value>, field: F) -> FieldNode<F> {
        FieldNode {
            name: "f".to_string(),
            label: None,
            optional: false,
            attrs: FieldAttrs::default(),
            custom_widget: None,
            wrapper: None,
            value,
            errors: Vec::new(),
            field,
        }
    }

    fn validation_messages<F: Field>(mut node: FieldNode<F>) -> Vec<String> {
        node.validate();
        node.errors.into_iter().map(|e| e.0).collect()
    }

    // ── Settings reach the check ─────────────────────────────────────────

    /// Through `validate` with a real value, the way a submitted form is.
    ///
    /// `Valid`, never `Empty`: `validate` returns early on an `Empty` or
    /// `Invalid` value, before the field's `check`, and a test that tripped
    /// that early return would read exactly like a setting that was checked
    /// and satisfied. So the precondition is asserted, not trusted.
    fn validated<F: Field>(value: F::Value, field: F) -> Vec<String> {
        let node = node(FieldValue::Valid(value), field);
        assert!(
            !node.raw_value().is_empty(),
            "an empty raw value is absence, which validate handles before the check"
        );
        validation_messages(node)
    }

    #[gtest]
    fn a_length_setting_reaches_the_check() {
        let short = StringField {
            max_length: Some(3),
            ..StringField::default()
        };
        expect_that!(validated("abc".to_string(), short.clone()), is_empty());
        expect_that!(validated("hello".to_string(), short), len(eq(1)));
    }

    #[gtest]
    fn bounds_reach_the_check() {
        let between = NumberField {
            min: Some(1_i32),
            max: Some(10),
        };
        expect_that!(validated(0, between.clone()), len(eq(1)));
        expect_that!(validated(5, between.clone()), is_empty());
        expect_that!(validated(11, between), len(eq(1)));
    }

    // ── `required_true` on a bool, through `validate` ───────────────────
    //
    // An unticked checkbox is `Empty`, and `Empty` returns early for every
    // other field, so this is where one could slip past `BoolField`.

    /// A non-optional bool field, holding `value`, rendered by `widget` (`None`
    /// means the default, a checkbox).
    fn a_bool_field(
        value: FieldValue<bool>,
        widget: Option<WidgetType>,
        required_true: bool,
    ) -> FieldNode<BoolField> {
        FieldNode {
            name: "agreed".to_string(),
            custom_widget: widget,
            ..node(value, BoolField { required_true })
        }
    }

    /// The case issue #6 is about. A box nobody touched is `Empty`, and so is
    /// every unticked box that reaches the server, since an unticked checkbox
    /// is left out of the form data. The node hands it to the field as `false`.
    #[gtest]
    fn an_untouched_required_true_checkbox_is_rejected() {
        let field = a_bool_field(FieldValue::Empty, None, true);
        expect_that!(
            validation_messages(field),
            elements_are![eq("this value must be true")]
        );
    }

    /// Ticked and then unticked arrives as `Valid(false)`, not `Empty`. Both
    /// must get the same answer, or whether the rule holds would depend on how
    /// the user got there.
    #[gtest]
    fn a_ticked_then_unticked_required_true_checkbox_is_rejected_the_same_way() {
        let field = a_bool_field(FieldValue::Valid(false), None, true);
        expect_that!(
            validation_messages(field),
            elements_are![eq("this value must be true")]
        );
    }

    #[gtest]
    fn a_ticked_required_true_checkbox_passes() {
        let field = a_bool_field(FieldValue::Valid(true), None, true);
        expect_that!(validation_messages(field), is_empty());
    }

    /// Without the rule, an unticked box is a complete answer: no "required"
    /// error and no other either.
    #[gtest]
    fn an_untouched_plain_checkbox_passes() {
        let field = a_bool_field(FieldValue::Empty, None, false);
        expect_that!(validation_messages(field), is_empty());
    }

    /// On a `select` an empty bool really is missing, so it gets the presence
    /// error and nothing else. It gets one error, not that plus "must be true".
    #[gtest]
    fn an_empty_required_true_select_gets_only_the_required_error() {
        let field = a_bool_field(
            FieldValue::Empty,
            Some(WidgetType::Builtin(BuiltinWidget::Select)),
            true,
        );
        expect_that!(
            validation_messages(field),
            elements_are![eq("This field is required.")]
        );
    }

    #[gtest]
    fn a_required_true_select_rejects_false() {
        let field = a_bool_field(
            FieldValue::Valid(false),
            Some(WidgetType::Builtin(BuiltinWidget::Select)),
            true,
        );
        expect_that!(
            validation_messages(field),
            elements_are![eq("this value must be true")]
        );
    }
}
