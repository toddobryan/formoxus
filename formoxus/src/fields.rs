//! Leaf members: a single input, its parsed value, and the two vtable-driven
//! conversions that replace `FromStr`/`Display` bounds on the model.

use crate::RenderCtx;
use crate::error::{FieldError, FormAccessError};
use crate::label_case::LabelCase;
use crate::members::{Edit, FieldSpecs, FormMember, default_label, no_such_path, qualify};
use crate::widgets::{FieldProps, InputType, ScalarWidget, SelectChoice, WidgetType};
use dioxus::prelude::*;
use facet::{Facet, Partial, Peek, ReflectError, ScalarType};
use regress::Regex;
use std::{collections::HashMap, fmt::Debug};

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue<T: Clone + Debug + PartialEq> {
    Empty,
    Valid(T),
    Invalid { raw: String, error: FieldError },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormField<T: Clone + Debug + PartialEq + for<'f> Facet<'f>> {
    pub name: String,
    pub label: Option<String>,
    pub optional: bool,
    pub constraints: Constraints,
    pub custom_widget: Option<WidgetType>,
    /// What a chooser offers, if the spec named a list. `None` for a field no
    /// spec gave choices to — which is every field rendered as an `<input>`.
    pub choices: Option<Vec<SelectChoice>>,
    /// The newtype this field's value is wrapped in — `Markdown` for a
    /// `FormField<String>` standing in for a `Markdown` field. `None` for an
    /// ordinary scalar.
    ///
    /// **The field carries the INNER type, not the newtype**, because a
    /// concrete `T` cannot be recovered from a runtime `&'static Shape`: the
    /// shape walk only ever has a shape in hand, and `FormField<T>` needs a
    /// type at compile time. So a `Markdown` field becomes a
    /// `FormField<String>` that remembers what to re-wrap it in, and every
    /// string-facing operation — parsing, display, `ValueKind`, the widget —
    /// goes on working unchanged against the inner scalar.
    ///
    /// Only [`write_value_into`](FormMember::write_value_into) consults it.
    pub wrapper: Option<&'static facet::Shape>,
    pub value: FieldValue<T>,
    pub errors: Vec<FieldError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueKind {
    Text {
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<&'static str>,
    },
    Int {
        min: Option<i128>,
        max: Option<i128>,
    }, // from the type; author bounds join later as separate Options
    Float {
        min: Option<f64>,
        max: Option<f64>,
    },
    Bool,
    /*Temporal,
    Choice,
    MultiChoice,
    File,*/
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Bound {
    Int(i128),
    Float(f64),
}

macro_rules! bound_from_int {
    // Infallible and lossless — `i128::from` says so in the type system.
    (from: $($t:ty),* $(,)?) => { $(
        impl From<$t> for Bound {
            fn from(v: $t) -> Self { Bound::Int(i128::from(v)) }
        }
    )* };
    // `usize`/`isize` have no `From<_> for i128` — their width is
    // target-dependent — so these need the cast. Lossless on every target
    // Rust supports: `i128` is wider than any pointer.
    (cast: $($t:ty),* $(,)?) => { $(
        impl From<$t> for Bound {
            fn from(v: $t) -> Self { Bound::Int(v as i128) }
        }
    )* };
}
bound_from_int!(from: i8, i16, i32, i64, i128, u8, u16, u32, u64);
bound_from_int!(cast: isize, usize);

impl From<f32> for Bound {
    fn from(v: f32) -> Self {
        Bound::Float(f64::from(v))
    }
}
impl From<f64> for Bound {
    fn from(v: f64) -> Self {
        Bound::Float(v)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Constraints {
    pub min: Option<Bound>,
    pub max: Option<Bound>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<&'static str>,
}

impl ValueKind {
    fn check(&self, raw_value: &str) -> Vec<FieldError> {
        let mut errors: Vec<FieldError> = Vec::new();
        match self {
            ValueKind::Text {
                min_length,
                max_length,
                pattern,
            } => {
                if let Some(min) = min_length
                    && raw_value.chars().count() < *min
                {
                    errors.push(FieldError(format!("length must be at least {min}")));
                }
                if let Some(max) = max_length
                    && raw_value.chars().count() > *max
                {
                    errors.push(FieldError(format!("length must be at most {max}")));
                }
                if let Some(patt) = pattern {
                    let re = Regex::with_flags(&format!("^(?:{patt})$"), "v")
                        .expect("this regex should have parsed at compile time");
                    if re.find(raw_value).is_none() {
                        errors.push(FieldError(format!(
                            "input should match the regular expression {patt}"
                        )));
                    }
                }
            }
            ValueKind::Int { min, max } => {
                let n: i128 = raw_value
                    .parse()
                    .expect("this int should have already successfully parsed");
                match (min, max) {
                    (Some(min), Some(max)) => {
                        if n < *min || n > *max {
                            errors.push(FieldError(format!(
                                "number must be in the range {min} up to (and including) {max}"
                            )));
                        }
                    }
                    (Some(min), None) => {
                        if n < *min {
                            errors.push(FieldError(format!("number must be at least {min}")));
                        }
                    }
                    (None, Some(max)) => {
                        if n > *max {
                            errors.push(FieldError(format!("number must be at most {max}")));
                        }
                    }
                    (None, None) => (),
                }
            }
            ValueKind::Float { min, max } => {
                let n: f64 = raw_value
                    .parse()
                    .expect("this float should have already successfully parsed");
                match (min, max) {
                    (Some(min), Some(max)) => {
                        if n < *min || n > *max || n.is_nan() {
                            errors.push(FieldError(format!(
                                "number must be in the range {min} up to (and including) {max}"
                            )));
                        }
                    }
                    (Some(min), None) => {
                        if n < *min || n.is_nan() {
                            errors.push(FieldError(format!("number must be at least {min}")));
                        }
                    }
                    (None, Some(max)) => {
                        if n > *max || n.is_nan() {
                            errors.push(FieldError(format!("number must be at most {max}")));
                        }
                    }
                    (None, None) => (),
                }
            }
            ValueKind::Bool => (),
        }
        errors
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
    fn value_kind(&self) -> ValueKind {
        // `None` is unreachable: `scalar_member` only builds a `FormField` for
        // the scalars it recognises, and `member_for_shape` panics on the rest.
        let scalar = T::SHAPE
            .scalar_type()
            .expect("FormField is only constructed for scalar shapes");

        match scalar {
            ScalarType::String => ValueKind::Text {
                min_length: self.constraints.min_length,
                max_length: self.constraints.max_length,
                pattern: self.constraints.pattern,
            },
            ScalarType::Bool => ValueKind::Bool,
            ScalarType::I8
            | ScalarType::I16
            | ScalarType::I32
            | ScalarType::I64
            | ScalarType::U8
            | ScalarType::U16
            | ScalarType::U32
            | ScalarType::U64 => {
                let min = self.constraints.min.map(|b| self.int_bound(b, "min"));
                let max = self.constraints.max.map(|b| self.int_bound(b, "max"));
                ValueKind::Int { min, max }
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a bound above 2^53 does not survive this widening; form! rejects \
                one at compile time (`field_kind::bound_is_exact`), so only a hand-built \
                spec can still reach here with one"
            )]
            ScalarType::F32 | ScalarType::F64 => {
                let min = match self.constraints.min {
                    None => None,
                    Some(Bound::Int(min)) => Some(min as f64),
                    Some(Bound::Float(min)) => Some(min),
                };
                let max = match self.constraints.max {
                    None => None,
                    Some(Bound::Int(max)) => Some(max as f64),
                    Some(Bound::Float(max)) => Some(max),
                };
                ValueKind::Float { min, max }
            }
            other => panic!(
                "scalar type {other:?} is not supported in FormField (field {})",
                self.name
            ),
        }
    }

    /// A bound on an integer field, as the `i128` `ValueKind::Int` holds.
    ///
    /// A whole float bound is accepted, because `min: 2.0` means what it says
    /// and `form!` cannot reject it: it cannot tell `2.0` from `2` when the
    /// bound is a named const. A fractional one still panics. `form!` rejects
    /// that at compile time (`field_kind::bound_is_whole`), so only a hand-built
    /// spec can reach the panic.
    fn int_bound(&self, bound: Bound, which: &str) -> i128 {
        match bound {
            Bound::Int(n) => n,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "only reached once `fract() == 0.0` says nothing is truncated"
            )]
            Bound::Float(x) if x.is_finite() && x.fract() == 0.0 => x as i128,
            Bound::Float(x) => panic!(
                "Integer field {} does not support the fractional {which} {x}",
                self.name
            ),
        }
    }

    /// What this field renders as absent any override.
    ///
    /// `optional` is the one input here that `T` cannot supply: `Option` peeling
    /// wraps rather than parameterizes, so `bool` and `Option<bool>` both arrive
    /// as `FormField<bool>`. A checkbox has two states and an `Option<bool>` has
    /// three, which is the whole reason that flag has to travel from the walk.
    fn default_widget(&self) -> WidgetType {
        match self.value_kind() {
            // Int and Float are deliberately `text`, not `number`: `type="number"` hands back `""`
            // for anything the browser dislikes, so a half-typed value vanishes.
            ValueKind::Text { .. } | ValueKind::Int { .. } | ValueKind::Float { .. } => {
                WidgetType::Input(InputType::Text)
            }
            // An `Option<bool>` has three states and a checkbox has two, so the
            // optional case gets a `Select` — reusing the one implementation of
            // the "no value" option rather than growing a third checkbox state
            // the DOM would have to be talked into.
            ValueKind::Bool if self.optional => WidgetType::Select,
            ValueKind::Bool => WidgetType::Checkbox,
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

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        out.push((qualify(prefix, &self.name), self.raw_value()));
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
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

    fn render(&self, ctx: &RenderCtx) -> Element {
        rsx! {
            ScalarWidget {
                value_kind: self.value_kind(),
                widget: self.widget(),
                choices: self.choices.clone(),
                values: ctx.values,
                props: FieldProps {
                    path: ctx.path(&self.name),
                    label: self.label(ctx.label_case),
                    required: ctx.required,
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
                Some(FieldError("This field is required.".to_string()))
            }
            FieldValue::Invalid { error, .. } => Some(error.clone()),
            _ => None,
        };
        // return an invalid or empty required field error (ignoring checkboxes) immediately,
        // and an empty field can't fail any constraints, so also return
        if error.is_some() || matches!(self.value, FieldValue::Empty) {
            self.errors.extend(error);
            return;
        }
        // now check constraints on particular types, all values should be FieldValue::Valid(t)
        let raw = self.raw_value();
        self.errors.extend(self.value_kind().check(&raw));
        if let Some(choices) = &self.choices
            && !choices.iter().any(|c| c.value == raw)
        {
            self.errors
                .push(FieldError("not one of the available choices".into()));
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

    fn push_field_error(
        &mut self,
        prefix: &str,
        path: &str,
        error: FieldError,
    ) -> Result<(), FormAccessError> {
        if path == qualify(prefix, &self.name) {
            self.errors.push(error);
            Ok(())
        } else {
            Err(no_such_path(path))
        }
    }

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        if let Some(spec) = fields.get(&qualify(prefix, &self.name)) {
            self.custom_widget = spec.custom_widget.clone().or(self.custom_widget.take());
            self.label = spec.label.clone().or(self.label.take());
            self.choices = spec.choices.clone().or(self.choices.take());
            // Replaced whole, not merged field by field like the three above.
            // Those three have something to preserve — `build` derives a label
            // from the field name and a widget from the shape, so the spec has
            // to mean "mine if I said anything, yours otherwise". Nothing
            // derives a CONSTRAINT: `scalar_member` always writes
            // `Constraints::default()`, so there has never been anything here
            // to keep, and one rule is easier to hold than two — `with_constraints`
            // already replaces wholesale on the spec side.
            //
            // The day something does derive one (a newtype declaring its own
            // range, say), this line starts silently discarding it and wants
            // the per-field `.or()` treatment instead.
            self.constraints = spec.constraints.clone();
        }
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
    }

    fn collect_errors(&self, prefix: &str, out: &mut crate::form::FieldErrors) {
        // Clean fields contribute nothing — see the trait's contract. Pushing
        // `(path, [])` here would make `FormErrors.fields` non-empty for a form
        // that passed, so a caller could not read "did it pass?" off the shape.
        if self.errors.is_empty() {
            return;
        }
        out.push((qualify(prefix, &self.name), self.errors.clone()));
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
pub fn parse_scalar<X>(raw: &str) -> Result<X, FieldError>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    let partial = Partial::alloc::<X>().map_err(|e| FieldError(e.to_string()))?;
    let partial = partial
        .parse_from_str(raw)
        .map_err(|_| FieldError(format!("{raw:?} isn't a valid {}", X::SHAPE)))?;
    partial
        .build()
        .map_err(|e| FieldError(e.to_string()))?
        .materialize::<X>()
        .map_err(|e| FieldError(e.to_string()))
}

pub(crate) fn populate<X>(peek: Option<Peek<'_, 'static>>) -> FieldValue<X>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    match peek {
        None => FieldValue::Empty,
        // `""` IS absence, and that has to hold at BOTH boundaries. `apply_leaves`
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
    use googletest::prelude::*;

    fn text(
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<&'static str>,
    ) -> ValueKind {
        ValueKind::Text {
            min_length,
            max_length,
            pattern,
        }
    }

    fn messages(kind: &ValueKind, raw: &str) -> Vec<String> {
        kind.check(raw).into_iter().map(|e| e.0).collect()
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
        let kind = ValueKind::Int {
            min: None,
            max: None,
        };
        expect_that!(messages(&kind, "0"), is_empty());
        expect_that!(
            messages(&kind, "-170141183460469231731687303715884105728"),
            is_empty()
        );
    }

    #[gtest]
    fn int_bounds_are_inclusive() {
        let kind = ValueKind::Int {
            min: Some(1),
            max: Some(10),
        };
        expect_that!(messages(&kind, "0"), len(eq(1)));
        expect_that!(messages(&kind, "1"), is_empty());
        expect_that!(messages(&kind, "10"), is_empty());
        expect_that!(messages(&kind, "11"), len(eq(1)));
    }

    #[gtest]
    fn a_one_sided_int_bound_says_which_side() {
        let low = ValueKind::Int {
            min: Some(0),
            max: None,
        };
        expect_that!(
            messages(&low, "-1"),
            elements_are![contains_substring("at least 0")]
        );

        let high = ValueKind::Int {
            min: None,
            max: Some(100),
        };
        expect_that!(
            messages(&high, "101"),
            elements_are![contains_substring("at most 100")]
        );
    }

    // ── Float ────────────────────────────────────────────────────────────

    #[gtest]
    fn float_bounds_are_inclusive() {
        let kind = ValueKind::Float {
            min: Some(0.0),
            max: Some(1.0),
        };
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
        let free = ValueKind::Float {
            min: None,
            max: None,
        };
        expect_that!(messages(&free, "nan"), is_empty());
        expect_that!(messages(&free, "NaN"), is_empty());

        expect_that!(
            messages(
                &ValueKind::Float {
                    min: Some(0.0),
                    max: Some(1.0)
                },
                "nan"
            ),
            len(eq(1))
        );
        expect_that!(
            messages(
                &ValueKind::Float {
                    min: Some(0.0),
                    max: None
                },
                "nan"
            ),
            len(eq(1))
        );
        expect_that!(
            messages(
                &ValueKind::Float {
                    min: None,
                    max: Some(1.0)
                },
                "nan"
            ),
            len(eq(1))
        );
    }

    /// Infinity is an ordinary float: allowed when unbounded, and compared
    /// normally when not. Parsing saturates rather than failing, so `1e400`
    /// arrives here as `inf` and a stated maximum is what catches it.
    #[gtest]
    fn infinity_is_allowed_unbounded_and_compared_when_bounded() {
        let free = ValueKind::Float {
            min: None,
            max: None,
        };
        expect_that!(messages(&free, "inf"), is_empty());
        expect_that!(messages(&free, "1e400"), is_empty());

        let capped = ValueKind::Float {
            min: None,
            max: Some(100.0),
        };
        expect_that!(messages(&capped, "1e400"), len(eq(1)));
        expect_that!(messages(&capped, "-inf"), is_empty());
    }

    // ── Bool ─────────────────────────────────────────────────────────────

    #[gtest]
    fn a_bool_has_nothing_to_constrain() {
        expect_that!(messages(&ValueKind::Bool, "true"), is_empty());
        expect_that!(messages(&ValueKind::Bool, "false"), is_empty());
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
        let _ = ValueKind::Int {
            min: Some(0),
            max: None,
        }
        .check("not a number");
    }

    // ── Constraints reach the field ──────────────────────────────────────
    //
    // Everything above tests `ValueKind::check` directly. These four go in
    // through `FormField`, which is the only way to exercise `value_kind()` —
    // the one place that pairs a constraint with the field's actual type.

    fn a_field<X>(value: X, constraints: Constraints) -> FormField<X>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        FormField {
            name: "f".to_string(),
            label: None,
            optional: false,
            constraints,
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

    fn validated<X>(value: X, constraints: Constraints) -> Vec<String>
    where
        X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
    {
        let mut field = a_field(value, constraints);

        // `validate` returns early on an `Empty` or `Invalid` value, BEFORE it
        // reaches `value_kind()` at all. A test that tripped that early return
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
        let short = Constraints {
            max_length: Some(3),
            ..Default::default()
        };
        expect_that!(validated("abc".to_string(), short.clone()), is_empty());
        expect_that!(validated("hello".to_string(), short), len(eq(1)));
    }

    #[gtest]
    fn integer_bounds_on_the_field_reach_the_check() {
        let between = Constraints {
            min: Some(Bound::Int(1)),
            max: Some(Bound::Int(10)),
            ..Default::default()
        };
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
        let non_negative = Constraints {
            min: Some(Bound::Int(0)),
            ..Default::default()
        };
        expect_that!(validated(-1.5_f64, non_negative.clone()), len(eq(1)));
        expect_that!(validated(0.5_f64, non_negative), is_empty());
    }

    /// The other direction, when the float is whole: `min: 2.0` means 2, so
    /// it is used as 2. `form!` cannot reject it, because it cannot tell `2.0`
    /// from `2` when the bound is a named const.
    #[gtest]
    fn a_whole_float_bound_on_an_integer_field_is_used_as_an_integer() {
        let at_least_two = Constraints {
            min: Some(Bound::Float(2.0)),
            ..Default::default()
        };
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
        let fractional = Constraints {
            min: Some(Bound::Float(1.5)),
            ..Default::default()
        };
        let _ = validated(3_i32, fractional);
    }
}
