//! The leaf node: one [`Field`] plus the tree state every field shares.

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek, ReflectError, ScalarType, Shape};
use formoxus_spec::{Attr, AttrValue, BuiltinWidget, FieldType, RenderedInputType};
use regress::Regex;

use crate::error::{FormAccessError, ValidationMessage};
use crate::field::{Field, FieldValue, populate};
use crate::field_attrs::{FieldAttrs, float_bound, int_bound};
use crate::form::ErrorsByPath;
use crate::form::FieldSpec;
use crate::label_case::LabelCase;
use crate::nodes::{Edit, Node, NodeContext, SpecsByPath, ValuesByPath, default_label, qualify};
use crate::widgets::{Choice, FieldProps, ScalarWidget, WidgetType};

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
            choices: spec.and_then(|s| s.choices.clone()),
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
    /// Becomes `widget: Box<dyn Widget>` in Step 5.
    pub custom_widget: Option<WidgetType>,
    /// What a chooser offers, if the spec named a list. `None` for a field no
    /// spec gave choices to — which is every field rendered as an `<input>`.
    /// Moves into `ChoiceField` in Step 4.
    pub choices: Option<Vec<Choice>>,
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

    /// Formoxus's own check of a value against this field's validated
    /// attributes. `raw_value` has already parsed as `F::Value`; `validate`
    /// returns before reaching here on an empty or unparseable value.
    ///
    /// Not [`Field::check`], which checks the parsed value and runs after this.
    /// This one goes away in Step 4, when the attributes move into the fields.
    ///
    /// Walks [`Attr::ALL`], not the map, so the messages come out in the
    /// table's order (lengths, then pattern, then bounds) whatever order
    /// `form!` inserted the attributes in. The field type is an INPUT here, not
    /// the dispatch: it only matters to `min`/`max`, which read the value as an
    /// `i128` or an `f64`.
    fn check_attrs(&self, raw_value: &str) -> Vec<ValidationMessage> {
        let field_type = self.field_type();
        let mut errors = Vec::new();
        // TODO: the Max and Min handling assumes an ordering for Attr::ALL.
        //       We should avoid that.
        for &attr in Attr::ALL {
            let Some(value) = self.attrs.get(attr) else {
                continue;
            };
            // `form!` refuses an attribute that does not apply to the field's
            // type at compile time, so only a hand-built spec reaches this.
            // Skipping keeps what such a spec always got: silently ignored.
            if !attr.validated() || !attr.applies_to(field_type) {
                continue;
            }
            match (attr, value) {
                (Attr::MinLength, AttrValue::Int(min)) => {
                    if raw_value.chars().count() < *min {
                        errors.push(ValidationMessage(format!("length must be at least {min}")));
                    }
                }
                (Attr::MaxLength, AttrValue::Int(max)) => {
                    if raw_value.chars().count() > *max {
                        errors.push(ValidationMessage(format!("length must be at most {max}")));
                    }
                }
                (Attr::Pattern, AttrValue::Regex(patt)) => {
                    let re = Regex::with_flags(&format!("^(?:{patt})$"), "v")
                        .expect("this regex should have parsed at compile time");
                    if re.find(raw_value).is_none() {
                        errors.push(ValidationMessage(format!(
                            "input should match the regular expression {patt}"
                        )));
                    }
                }
                // One message for the pair ("in the range X up to Y"), so it
                // is produced once, on `Min`, and `Max` stands aside when
                // `Min` is there to cover both.
                (Attr::Min | Attr::Max, AttrValue::Bound(_)) => {
                    if attr == Attr::Max && self.attrs.contains(Attr::Min) {
                        continue;
                    }
                    errors.extend(self.check_bounds(field_type, raw_value));
                }
                (Attr::RequiredTrue, AttrValue::Flag) => {
                    let is_true: bool = raw_value
                        .parse()
                        .expect("this bool should have already successfully parsed");
                    if !is_true {
                        errors.push(ValidationMessage("this value must be true".to_string()));
                    }
                }
                // Presence: decided in `validate`, before a value reaches here.
                (Attr::Required, _) => {}
                (attr, value) => panic!(
                    "field {}: {attr:?} cannot hold the value {value:?}",
                    self.name
                ),
            }
        }
        errors
    }

    /// The `min`/`max` check, for whichever of the two the field has.
    ///
    /// An integer field compares as `i128` (every supported integer fits), a
    /// float field as `f64`. A bound written as the other kind is widened to
    /// match: `int_bound` for an integer field, a cast for a float field.
    fn check_bounds(&self, field_type: FieldType, raw_value: &str) -> Option<ValidationMessage> {
        let bound = |attr: Attr| match self.attrs.get(attr) {
            Some(AttrValue::Bound(b)) => Some(*b),
            _ => None,
        };
        let (min, max) = (bound(Attr::Min), bound(Attr::Max));
        match field_type {
            FieldType::Int => {
                let n: i128 = raw_value
                    .parse()
                    .expect("this int should have already successfully parsed");
                let min = min.map(|b| int_bound(b, "min"));
                let max = max.map(|b| int_bound(b, "max"));
                range_message(
                    min,
                    max,
                    min.is_some_and(|m| n < m),
                    max.is_some_and(|m| n > m),
                )
            }
            FieldType::Float => {
                let n: f64 = raw_value
                    .parse()
                    .expect("this float should have already successfully parsed");
                let min = min.map(float_bound);
                let max = max.map(float_bound);
                // NaN compares false to everything, so it would slip past both
                // bounds; a bounded float field rejects it outright.
                range_message(
                    min,
                    max,
                    min.is_some_and(|m| n < m || n.is_nan()),
                    max.is_some_and(|m| n > m || n.is_nan()),
                )
            }
            // `applies_to` lets only number fields through to here.
            FieldType::Text | FieldType::Bool => None,
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
        WidgetType::Builtin(BuiltinWidget::default_for(self.field_type(), self.optional))
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

/// The message for a value outside its bounds, or `None` when it is inside.
/// One message covers both ends when the field has both, which is why `min`
/// and `max` are checked together.
fn range_message<N: std::fmt::Display>(
    min: Option<N>,
    max: Option<N>,
    below: bool,
    above: bool,
) -> Option<ValidationMessage> {
    let message = match (min, max) {
        (Some(min), Some(max)) if below || above => {
            format!("number must be in the range {min} up to (and including) {max}")
        }
        (Some(min), None) if below => format!("number must be at least {min}"),
        (None, Some(max)) if above => format!("number must be at most {max}"),
        _ => return None,
    };
    Some(ValidationMessage(message))
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
        let field_attrs = FieldAttrs::with_owned_attrs(
            &ctx.id(&self.name),
            &widget,
            ctx.required,
            self.has_errors_here(),
            &self.attrs,
        );

        rsx! {
            ScalarWidget {
                field_type: self.field_type(),
                field_attrs: field_attrs,
                widget: widget,
                choices: self.choices.clone(),
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
        // What is left is a `Valid` value or an unticked checkbox. An unticked
        // checkbox is the only `Empty` the match above lets through, and its
        // constraints are checked against `"false"`.
        let raw = self.raw_value_to_validate();
        self.errors.extend(self.check_attrs(&raw));
        if let Some(choices) = &self.choices
            && !choices.iter().any(|c| c.value == raw)
        {
            self.errors
                .push(ValidationMessage("not one of the available choices".into()));
        }
        // The field's own check, on the parsed value. An unticked checkbox has
        // none, so it is skipped here; `BoolField` (Step 4) settles that.
        if let FieldValue::Valid(value) = &self.value {
            self.errors.extend(self.field.check(value));
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
    use std::fmt::Debug;
    use std::marker::PhantomData;

    use super::*;
    use crate::field::ScalarField;
    use formoxus_spec::Bound;
    use googletest::prelude::*;

    // ── Helpers ──────────────────────────────────────────────────────────
    //
    // A constraint test needs a field TYPE (which decides how `min`/`max`
    // read the value) and the field's attributes. These build a `FieldNode`
    // of the right type, holding nothing: `check_attrs` takes the raw value
    // as an argument, so the field's own value never matters here.

    /// Built through `with_default`, which appends in order, because the
    /// map inside `FieldAttrs` is private to `field_attrs`.
    fn all(entries: &[(Attr, AttrValue)]) -> FieldAttrs {
        entries
            .iter()
            .fold(FieldAttrs::default(), |attrs, (attr, value)| {
                attrs.with_default(*attr, value.clone())
            })
    }

    fn node<X>(value: FieldValue<X>, attrs: FieldAttrs) -> FieldNode<ScalarField<X>>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        FieldNode {
            name: "f".to_string(),
            label: None,
            optional: false,
            attrs,
            custom_widget: None,
            choices: None,
            wrapper: None,
            value,
            errors: Vec::new(),
            field: ScalarField(PhantomData),
        }
    }

    fn holding<X>(attrs: FieldAttrs) -> FieldNode<ScalarField<X>>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        node(FieldValue::Empty, attrs)
    }

    fn text(
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<&'static str>,
    ) -> FieldNode<ScalarField<String>> {
        let mut entries = Vec::new();
        entries.extend(min_length.map(|n| (Attr::MinLength, AttrValue::Int(n))));
        entries.extend(max_length.map(|n| (Attr::MaxLength, AttrValue::Int(n))));
        entries.extend(pattern.map(|p| (Attr::Pattern, AttrValue::Regex(p))));
        holding(all(&entries))
    }

    fn bounds(min: Option<Bound>, max: Option<Bound>) -> FieldAttrs {
        let mut entries = Vec::new();
        entries.extend(min.map(|b| (Attr::Min, AttrValue::Bound(b))));
        entries.extend(max.map(|b| (Attr::Max, AttrValue::Bound(b))));
        all(&entries)
    }

    fn int(min: Option<i128>, max: Option<i128>) -> FieldNode<ScalarField<i64>> {
        holding(bounds(min.map(Bound::Int), max.map(Bound::Int)))
    }

    fn float(min: Option<f64>, max: Option<f64>) -> FieldNode<ScalarField<f64>> {
        holding(bounds(min.map(Bound::Float), max.map(Bound::Float)))
    }

    fn boolean(required_true: bool) -> FieldNode<ScalarField<bool>> {
        if required_true {
            holding(all(&[(Attr::RequiredTrue, AttrValue::Flag)]))
        } else {
            holding(FieldAttrs::default())
        }
    }

    fn messages<X>(field: &FieldNode<ScalarField<X>>, raw: &str) -> Vec<String>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        field.check_attrs(raw).into_iter().map(|e| e.0).collect()
    }

    // ── Text: lengths ────────────────────────────────────────────────────

    #[gtest]
    fn a_text_field_with_no_constraints_never_complains() {
        expect_that!(messages(&text(None, None, None), ""), is_empty());
        expect_that!(
            messages(&text(None, None, None), "anything at all"),
            is_empty()
        );
    }

    #[gtest]
    fn min_length_is_inclusive() {
        let kind = text(Some(3), None, None);
        expect_that!(messages(&kind, "ab"), len(eq(1)));
        expect_that!(messages(&kind, "abc"), is_empty());
        expect_that!(messages(&kind, "abcd"), is_empty());
    }

    #[gtest]
    fn max_length_is_inclusive() {
        let kind = text(None, Some(3), None);
        expect_that!(messages(&kind, "abc"), is_empty());
        expect_that!(messages(&kind, "abcd"), len(eq(1)));
    }

    /// **Characters, not bytes.** `José` is four characters and five bytes, so a
    /// `len()` here would reject a name that fits.
    #[gtest]
    fn length_counts_characters_not_bytes() {
        expect_that!(messages(&text(None, Some(4), None), "José"), is_empty());
        expect_that!(messages(&text(Some(4), None, None), "José"), is_empty());
        // The same string is 5 bytes, which a byte count would have rejected.
        expect_that!("José".len(), eq(5));
    }

    /// Astral-plane characters are one `char` each, so an emoji costs one, not
    /// four. HTML counts UTF-16 code units and would say two — a divergence
    /// worth knowing about rather than a bug to fix, since the server's count is
    /// the one that decides.
    #[gtest]
    fn an_emoji_counts_as_one_character() {
        expect_that!(messages(&text(None, Some(1), None), "🦀"), is_empty());
    }

    #[gtest]
    fn both_length_bounds_can_fail_independently() {
        let kind = text(Some(2), Some(4), None);
        expect_that!(messages(&kind, "a"), len(eq(1)));
        expect_that!(messages(&kind, "abc"), is_empty());
        expect_that!(messages(&kind, "abcde"), len(eq(1)));
    }

    // ── Text: pattern ────────────────────────────────────────────────────

    #[gtest]
    fn a_pattern_accepts_what_it_describes() {
        expect_that!(
            messages(&text(None, None, Some(r"\d{5}")), "90210"),
            is_empty()
        );
    }

    /// **The anchoring is the whole point.** HTML implicitly wraps a `pattern` as
    /// `^(?:…)$`, while a bare regex search finds a substring — so without the
    /// wrap the server would accept values the browser rejects, which is the
    /// worst direction for the two to disagree.
    #[gtest]
    fn a_pattern_must_match_the_entire_value() {
        let kind = text(None, None, Some(r"\d{5}"));
        expect_that!(messages(&kind, "abc12345xyz"), len(eq(1)));
        expect_that!(messages(&kind, "90210-1234"), len(eq(1)));
    }

    /// The `(?:…)` in the wrap is load-bearing, not decoration: `^a|b$` parses
    /// as "starts with a" OR "ends with b", so an un-grouped alternation would
    /// silently accept both halves of the wrong thing.
    #[gtest]
    fn an_alternation_is_grouped_before_it_is_anchored() {
        let kind = text(None, None, Some("cat|dog"));
        expect_that!(messages(&kind, "cat"), is_empty());
        expect_that!(messages(&kind, "dog"), is_empty());
        expect_that!(messages(&kind, "catfish"), len(eq(1)));
        expect_that!(messages(&kind, "hotdog"), len(eq(1)));
    }

    #[gtest]
    fn a_length_and_a_pattern_both_report() {
        let kind = text(Some(10), None, Some(r"\d+"));
        expect_that!(messages(&kind, "abc"), len(eq(2)));
    }

    /// `regress` gives ECMAScript semantics, which is the point of choosing it
    /// over `regex`: these four all match what a browser does, so the server
    /// cannot disagree with the client about what a `pattern` means.
    ///
    /// Anchors are NOT multiline — `\d{5}` rejects `"12345\n67890"` — and `$`
    /// does not match before a trailing newline the way Perl's does. Both
    /// matter for `pattern` on a `<textarea>`, where a value legitimately
    /// contains newlines.
    #[gtest]
    fn anchors_and_dot_follow_javascript_not_perl() {
        let five = text(None, None, Some(r"\d{5}"));
        expect_that!(messages(&five, "12345\n67890"), len(eq(1)));
        expect_that!(messages(&five, "12345\n"), len(eq(1)));

        // `.` excludes newline (no `s` flag), so spanning lines has to be asked
        // for explicitly.
        expect_that!(messages(&text(None, None, Some(r".+")), "a\nb"), len(eq(1)));
        expect_that!(
            messages(&text(None, None, Some(r"(.|\n)+")), "a\nb"),
            is_empty()
        );
    }

    /// HTML compiles `pattern` with the `v` flag, so this must too. Without
    /// it, `\p{L}` is an escaped `p` followed by a literal `{L}`, and the
    /// server would reject a value every browser accepts.
    #[gtest]
    fn a_pattern_is_compiled_with_the_v_flag() {
        let letters = text(None, None, Some(r"\p{L}+"));
        expect_that!(messages(&letters, "héllo"), is_empty());
        expect_that!(messages(&letters, "p{L}"), len(eq(1)));
    }

    // ── Int ──────────────────────────────────────────────────────────────

    #[gtest]
    fn an_int_with_no_bounds_never_complains() {
        let kind = int(None, None);
        expect_that!(messages(&kind, "0"), is_empty());
        expect_that!(
            messages(&kind, "-170141183460469231731687303715884105728"),
            is_empty()
        );
    }

    #[gtest]
    fn int_bounds_are_inclusive() {
        let kind = int(Some(1), Some(10));
        expect_that!(messages(&kind, "0"), len(eq(1)));
        expect_that!(messages(&kind, "1"), is_empty());
        expect_that!(messages(&kind, "10"), is_empty());
        expect_that!(messages(&kind, "11"), len(eq(1)));
    }

    #[gtest]
    fn a_one_sided_int_bound_says_which_side() {
        let low = int(Some(0), None);
        expect_that!(
            messages(&low, "-1"),
            elements_are![contains_substring("at least 0")]
        );

        let high = int(None, Some(100));
        expect_that!(
            messages(&high, "101"),
            elements_are![contains_substring("at most 100")]
        );
    }

    // ── Float ────────────────────────────────────────────────────────────

    #[gtest]
    fn float_bounds_are_inclusive() {
        let kind = float(Some(0.0), Some(1.0));
        expect_that!(messages(&kind, "-0.1"), len(eq(1)));
        expect_that!(messages(&kind, "0"), is_empty());
        expect_that!(messages(&kind, "1"), is_empty());
        expect_that!(messages(&kind, "1.1"), len(eq(1)));
    }

    /// **NaN is rejected only when a bound exists**, which is Todd's rule and the
    /// only coherent one: every comparison against NaN is false, so an unguarded
    /// range check would let it through silently. With no range stated there is
    /// nothing for it to be outside of, and a float field is entitled to hold it.
    #[gtest]
    fn nan_passes_an_unbounded_float_and_fails_a_bounded_one() {
        let free = float(None, None);
        expect_that!(messages(&free, "nan"), is_empty());
        expect_that!(messages(&free, "NaN"), is_empty());

        expect_that!(messages(&float(Some(0.0), Some(1.0)), "nan"), len(eq(1)));
        expect_that!(messages(&float(Some(0.0), None), "nan"), len(eq(1)));
        expect_that!(messages(&float(None, Some(1.0)), "nan"), len(eq(1)));
    }

    /// Infinity is an ordinary float: allowed when unbounded, and compared
    /// normally when not. Parsing saturates rather than failing, so `1e400`
    /// arrives here as `inf` and a stated maximum is what catches it.
    #[gtest]
    fn infinity_is_allowed_unbounded_and_compared_when_bounded() {
        let free = float(None, None);
        expect_that!(messages(&free, "inf"), is_empty());
        expect_that!(messages(&free, "1e400"), is_empty());

        let capped = float(None, Some(100.0));
        expect_that!(messages(&capped, "1e400"), len(eq(1)));
        expect_that!(messages(&capped, "-inf"), is_empty());
    }

    // ── Bool ─────────────────────────────────────────────────────────────

    /// Without `required_true`, `false` is a complete answer, so a bool has
    /// nothing to check.
    #[gtest]
    fn a_plain_bool_accepts_either_value() {
        let plain = boolean(false);
        expect_that!(messages(&plain, "true"), is_empty());
        expect_that!(messages(&plain, "false"), is_empty());
    }

    /// "I agree to the terms" (issue #6).
    #[gtest]
    fn a_required_true_bool_rejects_false() {
        let must_agree = boolean(true);
        expect_that!(messages(&must_agree, "true"), is_empty());
        expect_that!(
            messages(&must_agree, "false"),
            elements_are![eq("this value must be true")]
        );
    }

    // ── The caller's contract ────────────────────────────────────────────

    /// `check_attrs` runs only on a `FieldValue::Valid`, so the raw string is always
    /// this type's own canonical display and always re-parses. Pinning the panic
    /// documents that: a caller reaching here with unparsed input has skipped
    /// the guard in `validate`, and a silent `unwrap_or` would turn that bug
    /// into a field that quietly passes every bound.
    #[gtest]
    #[should_panic(expected = "should have already successfully parsed")]
    fn checking_an_unparsed_int_is_a_caller_bug() {
        let _ = int(Some(0), None).check_attrs("not a number");
    }

    // ── Constraints reach the field ──────────────────────────────────────
    //
    // Everything above calls `check_attrs` with a raw string. These go in through
    // `validate` with a real value, so the field's own type and value are what
    // get checked, the way a submitted form is.

    fn a_field<X>(value: X, attrs: FieldAttrs) -> FieldNode<ScalarField<X>>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        // `Valid`, never `Empty` — `validated` asserts this rather than
        // trusting it, since the early return in `validate` would make a
        // test look satisfied when nothing was checked.
        node(FieldValue::Valid(value), attrs)
    }

    fn validated<X>(value: X, attrs: FieldAttrs) -> Vec<String>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        let mut field = a_field(value, attrs);

        // `validate` returns early on an `Empty` or `Invalid` value, BEFORE it
        // reaches `check_attrs` at all. A test that tripped that early return
        // would come back with no errors and read exactly like a constraint
        // that was checked and satisfied — so the precondition is asserted
        // here instead of trusted.
        assert!(
            matches!(field.value, FieldValue::Valid(_)),
            "a constraint test needs a Valid value, or validate checks nothing"
        );
        // `\"\"` IS absence, and `populate` collapses an empty display to
        // `Empty`. So a `Valid` value that renders as `\"\"` is a state the
        // real system cannot produce, and a constraint verdict on it would not
        // mean anything either.
        assert!(
            !field.raw_value().is_empty(),
            "an empty raw value is absence, which validate handles before constraints"
        );

        field.validate();
        field.errors.into_iter().map(|e| e.0).collect()
    }

    #[gtest]
    fn a_length_constraint_on_the_field_reaches_the_check() {
        let short = all(&[(Attr::MaxLength, AttrValue::Int(3))]);
        expect_that!(validated("abc".to_string(), short.clone()), is_empty());
        expect_that!(validated("hello".to_string(), short), len(eq(1)));
    }

    #[gtest]
    fn integer_bounds_on_the_field_reach_the_check() {
        let between = bounds(Some(Bound::Int(1)), Some(Bound::Int(10)));
        expect_that!(validated(0_i32, between.clone()), len(eq(1)));
        expect_that!(validated(5_i32, between.clone()), is_empty());
        expect_that!(validated(11_i32, between), len(eq(1)));
    }

    /// The cross-flavour case, and the one that matters most in practice:
    /// `min: 0` on a float field is what people actually write, and an
    /// unsuffixed `0` is an `i32`, so it arrives as `Bound::Int`. Widening it
    /// is what keeps that from silently meaning "no minimum".
    #[gtest]
    fn an_integer_bound_on_a_float_field_is_widened_not_dropped() {
        let non_negative = bounds(Some(Bound::Int(0)), None);
        expect_that!(validated(-1.5_f64, non_negative.clone()), len(eq(1)));
        expect_that!(validated(0.5_f64, non_negative), is_empty());
    }

    /// The other direction, when the float is whole: `min: 2.0` means 2, so
    /// it is used as 2. `form!` cannot reject it, because it cannot tell `2.0`
    /// from `2` when the bound is a named const.
    #[gtest]
    fn a_whole_float_bound_on_an_integer_field_is_used_as_an_integer() {
        let at_least_two = bounds(Some(Bound::Float(2.0)), None);
        expect_that!(validated(1_i32, at_least_two.clone()), len(eq(1)));
        expect_that!(validated(2_i32, at_least_two), is_empty());
    }

    /// A FRACTIONAL float bound on an integer field has no sensible answer, so
    /// it is a panic rather than a silent drop. Rounding would invent a bound
    /// the author did not write, and the correct direction differs by end: a
    /// `min` would ceil where a `max` would floor.
    ///
    /// Only a hand-built `FormSpec` can reach this, since `form!` rejects it at
    /// compile time. That is why the message is for a caller and not for the
    /// person filling in the form.
    #[gtest]
    #[should_panic(expected = "does not support the fractional min")]
    fn a_fractional_bound_on_an_integer_field_is_a_caller_bug() {
        let fractional = bounds(Some(Bound::Float(1.5)), None);
        let _ = validated(3_i32, fractional);
    }

    // ── `required_true` on a bool, through `validate` ───────────────────
    //
    // `check_attrs` is covered above. These go through `validate`, because that is
    // where an unticked checkbox could slip past: it is `Empty`, and `Empty`
    // returns early for every other field.

    /// A non-optional bool field, holding `value`, rendered by `widget` (`None`
    /// means the default, a checkbox).
    fn a_bool_field(
        value: FieldValue<bool>,
        widget: Option<WidgetType>,
        required_true: bool,
    ) -> FieldNode<ScalarField<bool>> {
        let attrs = if required_true {
            all(&[(Attr::RequiredTrue, AttrValue::Flag)])
        } else {
            FieldAttrs::default()
        };
        FieldNode {
            name: "agreed".to_string(),
            custom_widget: widget,
            ..node(value, attrs)
        }
    }

    fn validation_messages(mut field: FieldNode<ScalarField<bool>>) -> Vec<String> {
        field.validate();
        field.errors.into_iter().map(|e| e.0).collect()
    }

    /// The case issue #6 is about. A box nobody touched is `Empty`, and so is
    /// every unticked box that reaches the server, since an unticked checkbox
    /// is left out of the form data. A rule that only `check_attrs` knew about would
    /// never see it.
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
    /// error and no constraint error either.
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
