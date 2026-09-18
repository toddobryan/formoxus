//! Leaf members: a single input, its parsed value, and the two vtable-driven
//! conversions that replace `FromStr`/`Display` bounds on the model.

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek, ReflectError, ScalarType};
use std::{collections::HashMap, fmt::Debug};
use crate::error::{FieldError, FormAccessError};
use crate::reflect::RenderCtx;
use crate::reflect::members::{Edit, FieldSpecs, FormMember, default_label, no_such_path, qualify};
use crate::reflect::widgets::{ControlType, FieldProps, InputType, ScalarInput};

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
    pub custom_control: Option<ControlType>,
    /// The newtype this field's value is wrapped in — `Markdown` for a
    /// `FormField<String>` standing in for a `Markdown` field. `None` for an
    /// ordinary scalar.
    ///
    /// **The field carries the INNER type, not the newtype**, because a
    /// concrete `T` cannot be recovered from a runtime `&'static Shape`: the
    /// shape walk only ever has a shape in hand, and `FormField<T>` needs a
    /// type at compile time. So a `Markdown` field becomes a
    /// `FormField<String>` that remembers what to re-wrap it in, and every
    /// string-facing operation — parsing, display, `ValueKind`, the control —
    /// goes on working unchanged against the inner scalar.
    ///
    /// Only [`write_value_into`](FormMember::write_value_into) consults it.
    pub wrapper: Option<&'static facet::Shape>,
    pub value: FieldValue<T>,
    pub errors: Vec<FieldError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueKind {
    Text { min_length: Option<usize>, max_length: Option<usize>, pattern: Option<&'static str> },
    Int { min: i128, max: i128 },   // from the type; author bounds join later as separate Options
    Float,
    Bool,
    /*Temporal,
    Choice,
    MultiChoice,
    File,*/
}

impl<T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static> FormField<T> {
    /// The value family this field carries, and the constraints that apply to it.
    ///
    /// Derived from `T` on every read rather than stored, so it cannot go stale
    /// and nothing about presentation is committed during the SHAPE walk. The
    /// constraint fields are all `None` for now — this is where an author's
    /// `#[facet(formoxus::max_length(…))]` will merge in, and the reason they
    /// live on the *value* kind rather than on `ControlType` is that overriding
    /// a `Text` control to `Textarea` or `Password` must not discard validation.
    ///
    /// Bounds come from the type itself rather than being written out, so they
    /// can't drift from `T`. They're `i128` because that's the only std integer
    /// holding both `i64::MIN` and `u64::MAX` — **this silently breaks if `u128`
    /// is ever added**, since `u128::MAX` would truncate through the `as` cast.
    fn value_kind(&self) -> ValueKind {
        macro_rules! int {
            ($t:ty) => {
                ValueKind::Int { min: <$t>::MIN as i128, max: <$t>::MAX as i128 }
            };
        }

        // `None` is unreachable: `scalar_member` only builds a `FormField` for
        // the scalars it recognises, and `member_for_shape` panics on the rest.
        let scalar = T::SHAPE
            .scalar_type()
            .expect("FormField is only constructed for scalar shapes");

        match scalar {
            ScalarType::String => ValueKind::Text {
                min_length: None,
                max_length: None,
                pattern: None,
            },
            ScalarType::Bool => ValueKind::Bool,
            ScalarType::I8 => int!(i8),
            ScalarType::I16 => int!(i16),
            ScalarType::I32 => int!(i32),
            ScalarType::I64 => int!(i64),
            ScalarType::U8 => int!(u8),
            ScalarType::U16 => int!(u16),
            ScalarType::U32 => int!(u32),
            ScalarType::U64 => int!(u64),
            ScalarType::F32 | ScalarType::F64 => ValueKind::Float,
            other => panic!(
                "scalar type {other:?} is not supported in FormField (field {})",
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
    fn default_control(&self) -> ControlType {
        match self.value_kind() {
            ValueKind::Text { .. } => ControlType::Input(InputType::Text),
            // Deliberately `text`, not `number`: `type="number"` hands back `""`
            // for anything the browser dislikes, so a half-typed value vanishes.
            ValueKind::Int { .. } | ValueKind::Float => ControlType::Input(InputType::Text),
            ValueKind::Bool if self.optional => ControlType::Select,
            ValueKind::Bool => ControlType::Checkbox,
        }
    }

    /// The control to render: an override if one was set, else the derived default.
    fn control(&self) -> ControlType {
        self.custom_control
            .clone()
            .unwrap_or_else(|| self.default_control())
    }

    fn is_unticked_checkbox(&self) -> bool {
        matches!(self.value, FieldValue::Empty) && matches!(self.control(), ControlType::Checkbox)
    }
}

impl<T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static> FormMember for FormField<T> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self) -> Option<String> {
        self.label.clone().or_else(|| default_label(&self.name))
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
            ScalarInput {
                value_kind: self.value_kind(),
                control: self.control(),
                values: ctx.values,
                props: FieldProps {
                    path: ctx.path(&self.name),
                    label: self.label(),
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
        self.errors.extend(error);
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
            _ => unreachable!("write_into should only run after validate() has confirmed no errors"),
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

    fn push_field_error(&mut self, prefix: &str, path: &str, error: FieldError) -> Result<(), FormAccessError> {
        if path == qualify(prefix, &self.name) {
            self.errors.push(error);
            Ok(())
        } else {
            Err(no_such_path(path))
        }
    }

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        if let Some(spec) = fields.get(&qualify(prefix, &self.name)) {
            self.custom_control = spec.custom_control.clone().or(self.custom_control.take());
            self.label = spec.label.clone().or(self.label.take());
        }
    }
    
    fn clear_errors(&mut self) {
        self.errors.clear();
    }
    
    fn collect_errors(&self, prefix: &str, out: &mut super::form::FieldErrors) {
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
pub(crate) fn parse_scalar<X>(raw: &str) -> Result<X, FieldError>
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
