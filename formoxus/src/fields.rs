//! Leaf members: a single input, its parsed value, and the two vtable-driven
//! conversions that replace `FromStr`/`Display` bounds on the model.
use crate::ErrorsByPath;
use crate::error::{FormAccessError, ValidationMessage};
use crate::label_case::LabelCase;
use crate::members::RenderCtx;
use crate::members::{Edit, FormMember, SpecsByPath, ValuesByPath, default_label, qualify};
use crate::widgets::{Choice, FieldProps, InputType, ScalarWidget, WidgetType};
use dioxus::prelude::*;
use facet::{Facet, Partial, Peek, ReflectError, ScalarType};
use formoxus_attrs::{Attr, AttrKey, AttrValue, Bound, FieldType};
use indexmap::IndexMap;
use regress::Regex;
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue<T: Clone + Debug + PartialEq> {
    Empty,
    Valid(T),
    Invalid {
        raw: String,
        error: ValidationMessage,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormField<T: Clone + Debug + PartialEq + for<'f> Facet<'f>> {
    pub name: String,
    pub label: Option<String>,
    pub optional: bool,
    pub attrs: FieldAttrs,
    pub custom_widget: Option<WidgetType>,
    /// What a chooser offers, if the spec named a list. `None` for a field no
    /// spec gave choices to — which is every field rendered as an `<input>`.
    pub choices: Option<Vec<Choice>>,
    /// The newtype this field's value is wrapped in — `Markdown` for a
    /// `FormField<String>` standing in for a `Markdown` field. `None` for an
    /// ordinary scalar.
    ///
    /// **The field carries the INNER type, not the newtype**, because a
    /// concrete `T` cannot be recovered from a runtime `&'static Shape`: the
    /// shape walk only ever has a shape in hand, and `FormField<T>` needs a
    /// type at compile time. So a `Markdown` field becomes a
    /// `FormField<String>` that remembers what to re-wrap it in, and every
    /// string-facing operation — parsing, display, `check`, the widget —
    /// goes on working unchanged against the inner scalar.
    ///
    /// Only [`write_value_into`](FormMember::write_value_into) consults it.
    pub wrapper: Option<&'static facet::Shape>,
    pub value: FieldValue<T>,
    pub errors: Vec<ValidationMessage>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldAttrs(IndexMap<AttrKey, AttrValue>);

impl FieldAttrs {
    pub fn get(&self, attr: Attr) -> Option<&AttrValue> {
        self.0.get(&AttrKey::Std(attr))
    }

    pub fn contains(&self, attr: Attr) -> bool {
        self.0.contains_key(&AttrKey::Std(attr))
    }

    fn to_attributes(&self, field_type: FieldType) -> Vec<Attribute> {
        let mut out = Vec::new();
        for (attr_key, attr_value) in &self.0 {
            let name = match attr_key {
                AttrKey::Std(attr) => attr.name(),
                AttrKey::NonStd(name) => name,
            };
            let attribute = match attr_value {
                AttrValue::Int(n) => Attribute::new(name, *n, None, false),
                AttrValue::Regex(patt) => Attribute::new(name, *patt, None, false),
                AttrValue::Flag => Attribute::new(name, true, None, false),
                AttrValue::Bound(b) => match field_type {
                    FieldType::Int => Attribute::new(name, int_bound(*b, name), None, false),
                    _ => Attribute::new(name, float_bound(*b), None, false),
                },
                AttrValue::String(s) => Attribute::new(name, s.clone(), None, false),
                AttrValue::List(vs) => {
                    // TODO: check style vs class, for now just join with space
                    Attribute::new(name, vs.join(" "), None, false)
                }
            };
            out.push(attribute);
        }
        out
    }

    pub fn merge_with_attrs(
        &self,
        field_type: FieldType,
        extras: Vec<Attribute>,
    ) -> Vec<Attribute> {
        let mut mapped_attrs: IndexMap<&'static str, Attribute> = self
            .to_attributes(field_type)
            .into_iter()
            .map(|a| (a.name, a))
            .collect();

        for attr in extras {
            mapped_attrs.insert(attr.name, attr);
        }

        mapped_attrs.into_values().collect()
    }
}

impl<const N: usize> From<[(AttrKey, AttrValue); N]> for FieldAttrs {
    fn from(arr: [(AttrKey, AttrValue); N]) -> Self {
        Self(IndexMap::from(arr))
    }
}

/// A bound on a float field, as an `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a bound above 2^53 does not survive this widening; form! rejects \
    one at compile time (`field_kind::bound_is_exact`), so only a hand-built \
    spec can still reach here with one"
)]
fn float_bound(bound: Bound) -> f64 {
    match bound {
        Bound::Int(n) => n as f64,
        Bound::Float(x) => x,
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

/// A bound on an integer field, as the `i128` it is compared as.
///
/// A whole float bound is accepted, because `min: 2.0` means what it says
/// and `form!` cannot reject it: it cannot tell `2.0` from `2` when the
/// bound is a named const. A fractional one still panics. `form!` rejects
/// that at compile time (`field_kind::bound_is_whole`), so only a hand-built
/// spec can reach the panic.
fn int_bound(bound: Bound, which: &str) -> i128 {
    match bound {
        Bound::Int(n) => n,
        #[expect(
            clippy::cast_possible_truncation,
            reason = "only reached once `fract() == 0.0` says nothing is truncated"
        )]
        Bound::Float(x) if x.is_finite() && x.fract() == 0.0 => x as i128,
        Bound::Float(x) => panic!("Integer field does not support the fractional {which} {x}"),
    }
}

impl<T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static> FormField<T> {
    /// The value family this field carries, and the constraints that apply to it.
    ///
    /// Derived from `T` on every read rather than stored, so it cannot go stale
    /// and nothing about presentation is committed during the SHAPE walk. Constraint
    /// fields are added by the form! macro based on the field's path.
    ///
    /// Bounds on integers are i128, since that's the only type that can represent
    /// the max and min values on all other int types. Similarly, bounds on floats are
    /// f64s. In the form! macro (where we have access to the actual type of the field),
    /// we check to make sure the constraints fit.
    fn field_type(&self) -> FieldType {
        // `None` is unreachable: `scalar_member` only builds a `FormField` for
        // the scalars it recognises, and `member_for_shape` panics on the rest.
        let scalar = T::SHAPE
            .scalar_type()
            .expect("FormField is only constructed for scalar shapes");

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
                "scalar type {other:?} is not supported in FormField (field {})",
                self.name
            ),
        }
    }

    /// Formoxus's own check of a value against this field's validated
    /// attributes. `raw_value` has already parsed as `T`; `validate` returns
    /// before reaching here on an empty or unparseable value.
    ///
    /// Walks [`Attr::ALL`], not the map, so the messages come out in the
    /// table's order (lengths, then pattern, then bounds) whatever order
    /// `form!` inserted the attributes in. The field type is an INPUT here, not
    /// the dispatch: it only matters to `min`/`max`, which read the value as an
    /// `i128` or an `f64`.
    fn check(&self, raw_value: &str) -> Vec<ValidationMessage> {
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
    /// `optional` is the one input here that `T` cannot supply: `Option` peeling
    /// wraps rather than parameterizes, so `bool` and `Option<bool>` both arrive
    /// as `FormField<bool>`. A checkbox has two states and an `Option<bool>` has
    /// three, which is the whole reason that flag has to travel from the walk.
    fn default_widget(&self) -> WidgetType {
        match self.field_type() {
            // Int and Float are deliberately `text`, not `number`: `type="number"` hands back `""`
            // for anything the browser dislikes, so a half-typed value vanishes.
            FieldType::Text | FieldType::Int | FieldType::Float => {
                WidgetType::Input(InputType::Text)
            }

            // An `Option<bool>` has three states and a checkbox has two, so the
            // optional case gets a `Select` — reusing the one implementation of
            // the "no value" option rather than growing a third checkbox state
            // the DOM would have to be talked into.
            FieldType::Bool if self.optional => WidgetType::Select,
            FieldType::Bool => WidgetType::Checkbox,
        }
    }

    /// The widget to render: an override if one was set, else the derived default.
    fn widget(&self) -> WidgetType {
        self.custom_widget
            .clone()
            .unwrap_or_else(|| self.default_widget())
    }

    fn is_unticked_checkbox(&self) -> bool {
        matches!(self.value, FieldValue::Empty) && matches!(self.widget(), WidgetType::Checkbox)
    }

    /// The raw value `validate` checks: `"false"` for an unticked checkbox,
    /// and [`FormMember::raw_value`] for everything else.
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

impl<T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static> FormMember for FormField<T> {
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
            // bound on `T` — the exact mirror of `parse_scalar` going the other
            // way. `{t:?}` would be wrong here: `Debug` quotes strings, and
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

        // No `FromStr` bound on `T`: facet's own parse vtable does this from
        // the shape, so a custom type only has to derive `Facet`, not
        // implement `FromStr` the way the macro-based version required.
        self.value = match parse_scalar::<T>(raw) {
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

    fn collect_errors(&self, prefix: &str, out: &mut crate::form::ErrorsByPath) {
        // Clean fields contribute nothing — see the trait's contract. Inserting
        // `(path, [])` here would make `FormErrors.fields` non-empty for a form
        // that passed, so a caller could not read "did it pass?" off the shape.
        if self.errors.is_empty() {
            return;
        }
        out.insert(qualify(prefix, &self.name), self.errors.clone());
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        rsx! {
            ScalarWidget {
                field_type: self.field_type(),
                field_attrs: self.attrs.clone(),
                required_true: self.attrs.contains(Attr::RequiredTrue),
                widget: self.widget(),
                choices: self.choices.clone(),
                values: ctx.values,
                props: FieldProps {
                    path: ctx.path(&self.name),
                    label: self.label(ctx.label_case),
                    required: ctx.required,
                    errors: self.errors.clone(),
                    aria_invalid: (!self.errors.is_empty()).then_some("true"),
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
        self.errors.extend(self.check(&raw));
        if let Some(choices) = &self.choices
            && !choices.iter().any(|c| c.value == raw)
        {
            self.errors
                .push(ValidationMessage("not one of the available choices".into()));
        }
    }

    fn clone_box(&self) -> Box<dyn FormMember> {
        Box::new(self.clone())
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty() || matches!(self.value, FieldValue::Invalid { .. })
    }

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        // The one place `wrapper` matters. The slot the parent opened is the
        // NEWTYPE's, so a bare `set` of the inner scalar would be a type error;
        // descending into field 0, setting there, and coming back up builds the
        // wrapper around it. `None` is the ordinary case and stays a plain set.
        let set = |p: Partial<'p>, t: T| -> Result<Partial<'p>, ReflectError> {
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
            // through `parse_scalar` rather than `partial.set(false)` is what
            // keeps this generic: nothing in scope can prove `T == bool`, but
            // the parse vtable resolves the real shape at runtime and doesn't
            // need to be told.
            FieldValue::Empty if self.is_unticked_checkbox() => set(
                partial,
                parse_scalar::<T>("false")
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

    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
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

    fn distribute_specs(&mut self, prefix: &str, fields: &SpecsByPath) {
        if let Some(spec) = fields.get(&qualify(prefix, &self.name)) {
            self.custom_widget = spec.custom_widget.clone().or(self.custom_widget.take());
            self.label = spec.label.clone().or(self.label.take());
            self.choices = spec.choices.clone().or(self.choices.take());
            // Replaced whole, not merged field by field like the three above.
            // Those three have something to preserve — `build` derives a label
            // from the field name and a widget from the shape, so the spec has
            // to mean "mine if I said anything, yours otherwise". Nothing
            // derives an attribute: `scalar_member` always writes
            // `FieldAttrs::default()`, so there has never been anything here
            // to keep, and one rule is easier to hold than two — `with_attrs`
            // already replaces wholesale on the spec side.
            //
            // The day something does derive one (a newtype declaring its own
            // range, say), this line starts silently discarding it and wants
            // the per-field `.or()` treatment instead.
            self.attrs = spec.attrs.clone();
        }
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
    }
}

/// Parse a raw input string into `X` using `X`'s own facet parse vtable —
/// the runtime equivalent of the `T: FromStr` bound the macro-based version
/// leaned on.
///
/// **`pub` only so the test suite can reach it from `tests/`, and `doc(hidden)`
/// because that is the whole reason.** It is an implementation detail of how a
/// leaf converts, not a service this crate offers; treat its signature as
/// unstable.
#[doc(hidden)]
pub fn parse_scalar<X>(raw: &str) -> Result<X, ValidationMessage>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    let partial = Partial::alloc::<X>().map_err(|e| ValidationMessage(e.to_string()))?;
    let partial = partial
        .parse_from_str(raw)
        .map_err(|_| ValidationMessage(format!("{raw:?} isn't a valid {}", X::SHAPE)))?;
    partial
        .build()
        .map_err(|e| ValidationMessage(e.to_string()))?
        .materialize::<X>()
        .map_err(|e| ValidationMessage(e.to_string()))
}

pub(crate) fn populate<X>(peek: Option<Peek<'_, 'static>>) -> FieldValue<X>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    match peek {
        None => FieldValue::Empty,
        // `""` IS absence, and that has to hold at BOTH boundaries. `distribute_values`
        // already collapses an empty input to `Empty`; without the same collapse
        // here, populating kept `Some("")` alive and `leaves() -> apply()` silently
        // stopped being an identity — the very invariant the uncontrolled design
        // rests on. Comparing the *display* string is what makes the two agree
        // exactly, since that's the string `raw_value` would have emitted.
        //
        // Only `String` can actually reach this: `true`/`0`/`0.0` are never empty.
        // The cost is that a required `String` holding `""` can't round-trip — but
        // that's HTML5's rule, not ours (an empty required input is `valueMissing`),
        // so no browser form could round-trip it either. Failing the same way on
        // both paths beats depending on which path the value arrived through.
        Some(p) if p.to_string().is_empty() => FieldValue::Empty,
        Some(p) => FieldValue::Valid(
            p.get::<X>()
                .expect("scalar_type matched, so this get should be the right type")
                .clone(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::core::AttributeValue;
    use googletest::prelude::*;

    // ── Helpers ──────────────────────────────────────────────────────────
    //
    // A constraint test needs a field TYPE (which decides how `min`/`max`
    // read the value) and the field's attributes. These build a `FormField`
    // of the right type, holding nothing: `check` takes the raw value as an
    // argument, so the field's own value never matters here.

    /// Table attributes only, which is all a constraint test needs; the
    /// author-attribute tests at the bottom build `NonStd` keys themselves.
    fn all(entries: &[(Attr, AttrValue)]) -> FieldAttrs {
        FieldAttrs(
            entries
                .iter()
                .map(|(attr, value)| (AttrKey::Std(*attr), value.clone()))
                .collect(),
        )
    }

    fn holding<X>(attrs: FieldAttrs) -> FormField<X>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        FormField {
            name: "f".to_string(),
            label: None,
            optional: false,
            attrs,
            custom_widget: None,
            choices: None,
            wrapper: None,
            value: FieldValue::Empty,
            errors: Vec::new(),
        }
    }

    fn text(
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<&'static str>,
    ) -> FormField<String> {
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

    fn int(min: Option<i128>, max: Option<i128>) -> FormField<i64> {
        holding(bounds(min.map(Bound::Int), max.map(Bound::Int)))
    }

    fn float(min: Option<f64>, max: Option<f64>) -> FormField<f64> {
        holding(bounds(min.map(Bound::Float), max.map(Bound::Float)))
    }

    fn boolean(required_true: bool) -> FormField<bool> {
        if required_true {
            holding(all(&[(Attr::RequiredTrue, AttrValue::Flag)]))
        } else {
            holding(FieldAttrs::default())
        }
    }

    fn messages<X>(field: &FormField<X>, raw: &str) -> Vec<String>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        field.check(raw).into_iter().map(|e| e.0).collect()
    }

    fn names<X>(field: &FormField<X>) -> Vec<&'static str>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        field
            .attrs
            .to_attributes(field.field_type())
            .iter()
            .map(|a| a.name)
            .collect()
    }

    /// The value of the one emitted attribute called `name`.
    fn emitted<X>(field: &FormField<X>, name: &str) -> AttributeValue
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        let attrs = field.attrs.to_attributes(field.field_type());
        let found: Vec<&Attribute> = attrs.iter().filter(|a| a.name == name).collect();
        assert!(
            found.len() == 1,
            "expected one `{name}`, got {}",
            found.len()
        );
        found[0].value.clone()
    }

    // ── Constraint attributes ────────────────────────────────────────────
    //
    // `html_attributes` is the render-side half of a constraint: the
    // same `min_length` that `check` enforces in Rust also has to reach the
    // browser, or the two disagree about what the field allows. These pin the
    // mapping. That an attribute then lands on the right element is a widget
    // question, tested from outside in `tests/suite/`.

    /// Constructing an `Attribute` needs no Dioxus runtime, which is why this
    /// mapping is a plain function and lives here rather than in a widget.
    #[gtest]
    fn a_field_with_no_constraints_gets_no_attributes() {
        expect_that!(names(&text(None, None, None)), is_empty());
        expect_that!(names(&boolean(false)), is_empty());
    }

    /// HTML `required` on a checkbox means "must be ticked", which is exactly
    /// `required_true` on a bool and nothing else. A plain bool must NOT get
    /// it (above), because unticked is a complete answer there.
    #[gtest]
    fn a_required_true_bool_becomes_the_required_attribute() {
        expect_that!(names(&boolean(true)), elements_are![eq(&"required")]);
        expect_that!(
            emitted(&boolean(true), "required"),
            eq(&AttributeValue::Bool(true))
        );
    }

    /// In the order the attributes were given, NOT `Attr::ALL` order: the
    /// map is an `IndexMap`, so the markup follows what the author wrote.
    /// Given here in neither table order nor its reverse, so a walk of
    /// `Attr::ALL` either way would fail it.
    #[gtest]
    fn each_text_constraint_becomes_its_html_attribute_in_the_order_given() {
        let field = holding::<String>(all(&[
            (Attr::MinLength, AttrValue::Int(3)),
            (Attr::Pattern, AttrValue::Regex(r"\d{5}")),
            (Attr::MaxLength, AttrValue::Int(10)),
        ]));
        expect_that!(
            names(&field),
            elements_are![eq(&"minlength"), eq(&"pattern"), eq(&"maxlength")]
        );
    }

    /// One `max_length` means ONE `maxlength`, because the attributes are
    /// keyed by `AttrKey`: HTML takes the first of a duplicated attribute, so a
    /// second entry would be silently dropped rather than loudly wrong.
    #[gtest]
    fn a_repeated_attribute_replaces_rather_than_duplicating() {
        let mut map = IndexMap::new();
        map.insert(AttrKey::Std(Attr::MaxLength), AttrValue::Int(5));
        map.insert(AttrKey::Std(Attr::MaxLength), AttrValue::Int(99));
        let field = holding::<String>(FieldAttrs(map));
        expect_that!(names(&field), elements_are![eq(&"maxlength")]);
        expect_that!(
            emitted(&field, "maxlength"),
            eq(&AttributeValue::Int(99)),
            "the later insert should win"
        );
    }

    /// Every emitted name is a name the table knows, so the HTML name can
    /// never drift from the attribute it came from.
    #[gtest]
    fn every_emitted_name_is_the_tables_name() {
        let emitted_names = [
            names(&text(Some(1), Some(2), Some("x"))),
            names(&int(Some(1), Some(2))),
            names(&float(Some(1.0), Some(2.0))),
            names(&boolean(true)),
        ]
        .concat();
        for name in emitted_names {
            expect_that!(Attr::from_name(name), some(anything()), "{name}");
        }
    }

    /// A length is a NUMBER, not a stringified one. It matters beyond tidiness:
    /// a numeric `AttributeValue` renders unquoted (`maxlength=10`), so a test
    /// asserting `maxlength="10"` against the markup would not match.
    #[gtest]
    fn a_length_is_a_number_not_a_string() {
        expect_that!(
            emitted(&text(None, Some(10), None), "maxlength"),
            eq(&AttributeValue::Int(10))
        );
    }

    /// **The pattern reaches the DOM UNANCHORED.** `check` wraps it as
    /// `^(?:…)$` because HTML does the same implicitly — so handing the browser
    /// an already-wrapped pattern would anchor it twice. The raw pattern is what
    /// keeps the two in agreement, which is the reason `regress` was chosen over
    /// `regex` in the first place.
    #[gtest]
    fn a_pattern_reaches_the_attribute_unanchored() {
        expect_that!(
            emitted(&text(None, None, Some(r"\d{5}")), "pattern"),
            eq(&AttributeValue::Text(r"\d{5}".to_string()))
        );
    }

    /// A bound is emitted as the FIELD's kind, not the bound's: an int field
    /// emits ints and a float field floats, after the same widening `check`
    /// uses, rather than both widening to text. `Float` is also why no `step`
    /// is emitted — see the note on `constraint_attributes`.
    #[gtest]
    fn a_numeric_bound_keeps_its_fields_kind() {
        let ints = int(Some(1), Some(9));
        expect_that!(emitted(&ints, "min"), eq(&AttributeValue::Int(1)));
        expect_that!(emitted(&ints, "max"), eq(&AttributeValue::Int(9)));

        let floats = float(Some(1.5), None);
        expect_that!(emitted(&floats, "min"), eq(&AttributeValue::Float(1.5)));
        expect_that!(names(&floats).len(), eq(1), "an absent bound emits nothing");

        // Written as the other kind, widened to the field's.
        let int_bound_on_float = holding::<f64>(bounds(Some(Bound::Int(0)), None));
        expect_that!(
            emitted(&int_bound_on_float, "min"),
            eq(&AttributeValue::Float(0.0))
        );
        let whole_float_on_int = holding::<i64>(bounds(Some(Bound::Float(2.0)), None));
        expect_that!(
            emitted(&whole_float_on_int, "min"),
            eq(&AttributeValue::Int(2))
        );
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

    /// `check` runs only on a `FieldValue::Valid`, so the raw string is always
    /// this type's own canonical display and always re-parses. Pinning the panic
    /// documents that: a caller reaching here with unparsed input has skipped
    /// the guard in `validate`, and a silent `unwrap_or` would turn that bug
    /// into a field that quietly passes every bound.
    #[gtest]
    #[should_panic(expected = "should have already successfully parsed")]
    fn checking_an_unparsed_int_is_a_caller_bug() {
        let _ = int(Some(0), None).check("not a number");
    }

    // ── Constraints reach the field ──────────────────────────────────────
    //
    // Everything above calls `check` with a raw string. These go in through
    // `validate` with a real value, so the field's own type and value are what
    // get checked, the way a submitted form is.

    fn a_field<X>(value: X, attrs: FieldAttrs) -> FormField<X>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        FormField {
            name: "f".to_string(),
            label: None,
            optional: false,
            attrs,
            custom_widget: None,
            choices: None,
            wrapper: None,
            // `Valid`, never `Empty` — `validated` asserts this rather than
            // trusting it, since the early return in `validate` would make a
            // test look satisfied when nothing was checked.
            value: FieldValue::Valid(value),
            errors: Vec::new(),
        }
    }

    fn validated<X>(value: X, attrs: FieldAttrs) -> Vec<String>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        let mut field = a_field(value, attrs);

        // `validate` returns early on an `Empty` or `Invalid` value, BEFORE it
        // reaches `check` at all. A test that tripped that early return
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
    // `check` is covered above. These go through `validate`, because that is
    // where an unticked checkbox could slip past: it is `Empty`, and `Empty`
    // returns early for every other field.

    /// A non-optional bool field, holding `value`, rendered by `widget` (`None`
    /// means the default, a checkbox).
    fn a_bool_field(
        value: FieldValue<bool>,
        widget: Option<WidgetType>,
        required_true: bool,
    ) -> FormField<bool> {
        FormField {
            name: "agreed".to_string(),
            label: None,
            optional: false,
            attrs: if required_true {
                all(&[(Attr::RequiredTrue, AttrValue::Flag)])
            } else {
                FieldAttrs::default()
            },
            custom_widget: widget,
            choices: None,
            wrapper: None,
            value,
            errors: Vec::new(),
        }
    }

    fn validation_messages(mut field: FormField<bool>) -> Vec<String> {
        field.validate();
        field.errors.into_iter().map(|e| e.0).collect()
    }

    /// The case issue #6 is about. A box nobody touched is `Empty`, and so is
    /// every unticked box that reaches the server, since an unticked checkbox
    /// is left out of the form data. A rule that only `check` knew about would
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
        let field = a_bool_field(FieldValue::Empty, Some(WidgetType::Select), true);
        expect_that!(
            validation_messages(field),
            elements_are![eq("This field is required.")]
        );
    }

    #[gtest]
    fn a_required_true_select_rejects_false() {
        let field = a_bool_field(FieldValue::Valid(false), Some(WidgetType::Select), true);
        expect_that!(
            validation_messages(field),
            elements_are![eq("this value must be true")]
        );
    }

    // ── Author attributes ────────────────────────────────────────────────
    //
    // A quoted `form!` key is a `NonStd` entry holding an `AttrValue::String`.
    // It never reaches `check`, which walks `Attr::ALL`, so it cannot change
    // what a value parses as or what `check` allows. These pin the conversion
    // `render` relies on. That the attributes then land on the right element
    // is a widget question, tested from outside in `tests/suite/`.

    fn author(pairs: &[(&'static str, &str)]) -> Vec<Attribute> {
        FieldAttrs(
            pairs
                .iter()
                .map(|(name, value)| {
                    (
                        AttrKey::NonStd(name),
                        AttrValue::String((*value).to_string()),
                    )
                })
                .collect(),
        )
        .to_attributes(FieldType::Text)
    }

    #[gtest]
    fn no_author_attributes_means_no_attributes() {
        expect_that!(author(&[]), is_empty());
    }

    /// Order is kept because the map is an `IndexMap`, so the rendered markup
    /// follows the order the author wrote them in.
    #[gtest]
    fn author_attributes_keep_their_names_and_order() {
        let attrs = author(&[("hx-get", "/x"), ("data-id", "7")]);
        expect_that!(
            attrs.iter().map(|a| a.name).collect::<Vec<_>>(),
            elements_are![eq(&"hx-get"), eq(&"data-id")]
        );
    }

    /// A table attribute and a quoted one share the map, so their relative
    /// order is the author's too: the reason for one map rather than two.
    #[gtest]
    fn table_and_quoted_attributes_interleave_in_the_order_given() {
        let field = holding::<String>(FieldAttrs(IndexMap::from([
            (
                AttrKey::NonStd("hx-get"),
                AttrValue::String("/x".to_string()),
            ),
            (AttrKey::Std(Attr::MaxLength), AttrValue::Int(5)),
            (
                AttrKey::NonStd("data-id"),
                AttrValue::String("7".to_string()),
            ),
        ])));
        expect_that!(
            names(&field),
            elements_are![eq(&"hx-get"), eq(&"maxlength"), eq(&"data-id")]
        );
    }

    /// A string attribute is always TEXT, even when it looks like a number:
    /// `"data-rows": "4"` renders `data-rows="4"`. That is unlike `max_length`,
    /// whose `maxlength` is an `AttributeValue::Int` and renders unquoted. Both
    /// are valid HTML, but a test matching the markup has to know which it is.
    #[gtest]
    fn an_author_attribute_is_text_even_when_numeric() {
        let attrs = author(&[("data-rows", "4")]);
        expect_that!(attrs[0].value, eq(&AttributeValue::Text("4".to_string())));
    }

    /// Plain attributes, the same as a table attribute gets: no namespace
    /// (that is only for `style`) and not volatile.
    #[gtest]
    fn an_author_attribute_has_no_namespace_and_is_not_volatile() {
        let attrs = author(&[("hx-get", "/x")]);
        expect_that!(attrs[0].namespace, none());
        expect_that!(attrs[0].volatile, eq(false));
    }
}
