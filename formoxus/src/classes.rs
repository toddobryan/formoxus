//! The CSS classes formoxus puts on what it renders, and how an app or a
//! form changes them.
//!
//! Every element that takes a class has a SLOT: a part of a built-in widget
//! (its `label`, its `input`), or a fixed one for what no widget renders (the
//! `<form>`, the error list). A slot holds the whole class string, so a
//! composed name like `fx-form-field fx-radio-group` is just one slot's value.
//! The design, and what `Classes::fx()` reproduces, is P3 in
//! `formoxus/ATTRIBUTES_PLAN.md`.

use std::borrow::Cow;

use formoxus_spec::BuiltinWidget;
use indexmap::IndexMap;

use crate::buttons::ButtonType;

/// The classes one element gets: always `base`, plus `invalid` while its
/// field has errors.
///
/// The invalid class is per slot because frameworks disagree on WHERE it goes:
/// Bootstrap marks the input (`is-invalid`), formoxus's own `fx()` marks the
/// wrapper (`fx-invalid`), and Foundation marks the label and the message too.
///
/// `Cow` so a preset can use `&'static str` literals while an app may build a
/// name at run time with `format!`. Plain data, hence the `pub` fields;
/// [`Slot::new`] and [`Slot::with_invalid`] are for writing one inline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The classes the element always has. `""` for none.
    pub base: Cow<'static, str>,
    /// The classes added while the field has errors, or `None` to add nothing.
    pub invalid: Option<Cow<'static, str>>,
}

impl Slot {
    /// A slot with `base` and no invalid class.
    ///
    /// `const`, so a preset's slots can be constants. That is why it takes a
    /// `&'static str` and names `Cow::Borrowed` itself: `.into()` is a trait
    /// method, which a `const fn` cannot call. For a name built at run time,
    /// write the struct literal with `Cow::Owned`.
    pub const fn new(base: &'static str) -> Self {
        Slot {
            base: Cow::Borrowed(base),
            invalid: None,
        }
    }

    /// The same slot, adding `invalid` while the field has errors. Takes a
    /// `&'static str` or a `String` alike.
    pub fn with_invalid(mut self, invalid: impl Into<Cow<'static, str>>) -> Self {
        self.invalid = Some(invalid.into());
        self
    }

    /// The `class` value for this state: `base`, then the invalid class when
    /// `invalid` is true, space-joined; `None` when that is empty, so a
    /// slot of `""` renders no `class` attribute rather than `class=""`.
    ///
    /// `invalid` is the FIELD's state, not a claim about the slot: a slot
    /// with no invalid class just renders its base when it is true. Most of
    /// `fx()`'s slots are like that, since it marks only the wrapper.
    pub fn value(&self, invalid: bool) -> Option<String> {
        let invalid = if invalid {
            self.invalid.as_deref()
        } else {
            None
        };
        let classes: Vec<&str> = [Some(&*self.base), invalid]
            .into_iter()
            .flatten()
            .filter(|c| !c.is_empty())
            .collect();
        (!classes.is_empty()).then(|| classes.join(" "))
    }
}

/// The elements no widget renders, each with a slot of its own. The value in
/// each comment is what `Classes::fx()` gives it.
///
/// `#[non_exhaustive]` so a slot can be added without breaking a `match`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FixedKey {
    /// The `<form>`: `fx-form`.
    Form,
    /// The form's `<h2>` title: `fx-form-title`.
    FormTitle,
    /// The `<ul>` of form-level errors, from the spec's validator:
    /// `fx-form-errors`.
    FormErrors,
    /// One `<li>` in it: `fx-form-error`.
    FormError,
    /// The `<ul>` of button problems: a button whose `fn` does not match how
    /// it was declared. A programmer's mistake, not the user's, so it is
    /// marked apart: `fx-form-errors fx-problems`.
    ButtonProblems,
    /// One `<li>` in it: `fx-form-error fx-problem`, so it styles like a
    /// form error by default and `.fx-problem` reaches it alone.
    ButtonProblem,
    /// The `<div>` around the buttons: `fx-buttons`.
    ButtonRow,
    /// One button, by its type: `fx-button fx-button-{submit, reset,
    /// cancel, destructive, action}`.
    Button(ButtonType),
    /// A list's add-row button: `fx-button fx-button-add-row`.
    AddRowButton,
    /// A row's remove button: `fx-button fx-button-remove-row`.
    RemoveRowButton,
    /// The `<fieldset>` around a nested struct: `fx-group fx-field-set`.
    FieldSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    FieldSetLegend,
    /// The `<fieldset>` around a list: `fx-group fx-list-set`.
    ListSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    ListSetLegend,
    /// The `<div>` around one row of a list: `fx-form-row`.
    ListRow,
    /// The `<fieldset>` around an enum field: `fx-group fx-variant-set`.
    VariantSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    VariantSetLegend,
    /// The ` *` after a required field's caption: `fx-required`.
    RequiredMarker,
    /// The `<ul>` of one field's errors: `fx-field-errors`.
    FieldErrors,
    /// One `<li>` in it: `fx-field-error`.
    FieldError,
}

/// Every class formoxus emits, one [`Slot`] per element: a part of a built-in
/// widget, or a [`FixedKey`] for what no widget renders.
///
/// Start from a preset and change what differs:
///
/// ```ignore
/// Classes::fx().with_part(BuiltinWidget::Select, "select", Slot::new("form-select"))
/// ```
///
/// A slot that is absent renders no `class` attribute, which is all
/// [`Classes::bare`] is: no slots at all. The map is private so the keying can
/// change without breaking anyone; the builders and lookups are the surface.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Classes {
    slots: IndexMap<SlotKey, Slot>,
}

/// Which element a slot is for. A part is keyed by the `&'static str` name
/// from [`BuiltinWidget::parts`], never a caller's string, so a key always
/// names a part that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SlotKey {
    Part(BuiltinWidget, &'static str),
    Fixed(FixedKey),
}

impl Classes {
    /// The slot for one part of one built-in widget, or `None` if it has
    /// none (it then renders no `class`).
    ///
    /// Panics if `widget` has no part named `part`, as [`Self::with_part`]
    /// does. At a lookup that is a formoxus bug, not an author's: the call
    /// sites name parts as literals.
    pub fn part(&self, widget: BuiltinWidget, part: &str) -> Option<&Slot> {
        self.slots
            .get(&SlotKey::Part(widget, Classes::part_name(widget, part)))
    }

    /// The slot for an element no widget renders, or `None` if it has none.
    pub fn fixed(&self, key: FixedKey) -> Option<&Slot> {
        self.slots.get(&SlotKey::Fixed(key))
    }

    /// The `class` value for one part of one built-in widget in this state,
    /// or `None` to render no `class` at all: [`Self::part`], then
    /// [`Slot::value`]. `invalid` is whether the field has errors.
    ///
    /// `Option` so a call site can write `class: classes.part_class(…)` and
    /// let `rsx!` omit the attribute on `None`.
    pub fn part_class(&self, widget: BuiltinWidget, part: &str, invalid: bool) -> Option<String> {
        self.part(widget, part).and_then(|s| s.value(invalid))
    }

    /// The `class` value for an element no widget renders, as
    /// [`Self::part_class`] is for a widget part.
    pub fn fixed_class(&self, key: FixedKey, invalid: bool) -> Option<String> {
        self.fixed(key).and_then(|s| s.value(invalid))
    }

    /// `slot` on one `part` of one built-in widget, replacing whatever was
    /// there: `Classes::fx().with_part(BuiltinWidget::Select, "select",
    /// Slot::new("form-select"))`.
    ///
    /// Panics if `widget` has no part named `part`, listing the parts it does
    /// have: a name no widget renders is probably a typo, and ignoring it
    /// would leave the class silently missing.
    pub fn with_part(mut self, widget: BuiltinWidget, part: &str, slot: Slot) -> Self {
        self.slots.insert(
            SlotKey::Part(widget, Classes::part_name(widget, part)),
            slot,
        );
        self
    }

    /// `part`, as the `&'static str` that `widget.parts()` names it by, which
    /// is what a [`SlotKey`] holds. Panics, listing the parts `widget` has,
    /// if it has none called `part`.
    fn part_name(widget: BuiltinWidget, part: &str) -> &'static str {
        widget.parts().iter().find(|p| p.name == part).map_or_else(
            || {
                let names: Vec<&str> = widget.parts().iter().map(|p| p.name).collect();
                panic!(
                    "unknown part `{part}` for widget {widget:?} (it has {})",
                    names.join(", ")
                )
            },
            |p| p.name,
        )
    }

    /// `slot` on the `part` of every built-in widget `filter` keeps that has
    /// one, for a preset that classes a whole family alike:
    ///
    /// ```ignore
    /// // Bootstrap's `form-control` on every input but the checkbox.
    /// Classes::bare().with_parts(
    ///     |w| matches!(w, BuiltinWidget::Input(t) if t != RenderedInputType::Checkbox),
    ///     "input",
    ///     Slot::new("form-control"),
    /// )
    /// ```
    ///
    /// A kept widget without the part is skipped, so `|_| true` with
    /// `"legend"` reaches only the widgets that have a legend. Panics if NO
    /// kept widget has it: like [`Self::with_part`], that is probably a typo.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "by value like `with_part`'s, so the two builders read alike"
    )]
    pub fn with_parts(
        mut self,
        filter: impl Fn(BuiltinWidget) -> bool,
        part: &str,
        slot: Slot,
    ) -> Self {
        let mut matched = false;
        for &widget in BuiltinWidget::ALL.iter().filter(|&&w| filter(w)) {
            if let Some(p) = widget.parts().iter().find(|p| p.name == part) {
                self.slots
                    .insert(SlotKey::Part(widget, p.name), slot.clone());
                matched = true;
            }
        }
        assert!(
            matched,
            "no widget the filter keeps has a part named `{part}`"
        );
        self
    }

    /// `value` on a slot no widget renders, replacing whatever was there:
    /// `Classes::fx().with_fixed(FixedKey::ButtonRow, Slot::new("actions"))`.
    pub fn with_fixed(mut self, slot: FixedKey, value: Slot) -> Self {
        self.slots.insert(SlotKey::Fixed(slot), value);
        self
    }

    /// formoxus's own classes, `fx-`-prefixed: what it rendered before classes
    /// were configurable, byte for byte (`tests/snapshots/` pins that). The
    /// default, so an app that says nothing gets these.
    ///
    /// The widget parts come from a loop over every rendered widget's
    /// [`BuiltinWidget::parts`], so a widget or part added later panics here
    /// until it is given a class, instead of silently rendering unclassed.
    pub fn fx() -> Self {
        let mut slots = IndexMap::new();
        for &widget in BuiltinWidget::ALL.iter().filter(|w| w.is_rendered()) {
            for part in widget.parts() {
                if let Some(slot) = fx_part(widget, part.name) {
                    slots.insert(SlotKey::Part(widget, part.name), slot);
                }
            }
        }
        let fixed = [
            (FixedKey::Form, Slot::new("fx-form")),
            (FixedKey::FormTitle, Slot::new("fx-form-title")),
            (FixedKey::FormErrors, Slot::new("fx-form-errors")),
            (FixedKey::FormError, Slot::new("fx-form-error")),
            (
                FixedKey::ButtonProblems,
                Slot::new("fx-form-errors fx-problems"),
            ),
            (
                FixedKey::ButtonProblem,
                Slot::new("fx-form-error fx-problem"),
            ),
            (FixedKey::ButtonRow, Slot::new("fx-buttons")),
            (
                FixedKey::Button(ButtonType::Submit),
                Slot::new("fx-button fx-button-submit"),
            ),
            (
                FixedKey::Button(ButtonType::Reset),
                Slot::new("fx-button fx-button-reset"),
            ),
            (
                FixedKey::Button(ButtonType::Cancel),
                Slot::new("fx-button fx-button-cancel"),
            ),
            (
                FixedKey::Button(ButtonType::Destructive),
                Slot::new("fx-button fx-button-destructive"),
            ),
            (
                FixedKey::Button(ButtonType::Action),
                Slot::new("fx-button fx-button-action"),
            ),
            (
                FixedKey::AddRowButton,
                Slot::new("fx-button fx-button-add-row"),
            ),
            (
                FixedKey::RemoveRowButton,
                Slot::new("fx-button fx-button-remove-row"),
            ),
            (FixedKey::FieldSet, Slot::new("fx-group fx-field-set")),
            (
                FixedKey::FieldSetLegend,
                Slot::new("fx-legend fx-field-label"),
            ),
            (FixedKey::ListSet, Slot::new("fx-group fx-list-set")),
            (
                FixedKey::ListSetLegend,
                Slot::new("fx-legend fx-field-label"),
            ),
            (FixedKey::ListRow, Slot::new("fx-form-row")),
            (FixedKey::VariantSet, Slot::new("fx-group fx-variant-set")),
            (
                FixedKey::VariantSetLegend,
                Slot::new("fx-legend fx-field-label"),
            ),
            (FixedKey::RequiredMarker, Slot::new("fx-required")),
            (FixedKey::FieldErrors, Slot::new("fx-field-errors")),
            (FixedKey::FieldError, Slot::new("fx-field-error")),
        ];
        slots.extend(fixed.map(|(key, slot)| (SlotKey::Fixed(key), slot)));
        Self { slots }
    }

    /// No classes at all: every element renders without a `class` attribute.
    /// The starting point for an app whose stylesheet targets elements and
    /// attributes, or that adds its own classes slot by slot.
    pub fn bare() -> Self {
        Self {
            slots: IndexMap::new(),
        }
    }
}

impl Default for Classes {
    /// [`Classes::fx`].
    fn default() -> Self {
        Self::fx()
    }
}

/// `fx()`'s class for one part of one rendered widget, or `None` for a part
/// it leaves bare (only a hidden input, which nobody sees).
///
/// Panics on a pair it does not know, which is a widget or a part added
/// without saying what `fx()` gives it. The widgets not rendered yet are
/// never asked: they get their classes when they ship.
fn fx_part(widget: BuiltinWidget, part: &str) -> Option<Slot> {
    use BuiltinWidget::{Input, RadioGroup, Select, Textarea};
    use formoxus_spec::RenderedInputType::{Checkbox, Hidden};
    let slot = match (widget, part) {
        (Input(Hidden), "input") => return None,
        // A radio group is a fieldset of choices, so its parts are named
        // apart from a single control's.
        (RadioGroup, "fieldset") => {
            Slot::new("fx-form-field fx-radio-group").with_invalid("fx-invalid")
        }
        (RadioGroup, "legend") => Slot::new("fx-legend fx-field-label"),
        (RadioGroup, "div") => Slot::new("fx-choice"),
        (RadioGroup, "input") => Slot::new("fx-control fx-radio"),
        (RadioGroup, "label") => Slot::new("fx-choice-label"),
        // Every other widget: the wrapper carries the invalid mark, the label
        // and the control do not.
        (_, "div") => Slot::new("fx-form-field").with_invalid("fx-invalid"),
        (_, "label") => Slot::new("fx-field-label"),
        (Input(Checkbox), "input") => Slot::new("fx-control fx-checkbox"),
        (Input(_), "input") => Slot::new("fx-control fx-input"),
        (Textarea, "textarea") => Slot::new("fx-control fx-textarea"),
        (Select, "select") => Slot::new("fx-control fx-select"),
        (widget, part) => panic!("`Classes::fx()` has no class for {widget:?}'s `{part}`"),
    };
    Some(slot)
}

#[cfg(test)]
mod tests {
    use super::{Classes, FixedKey, Slot, SlotKey};
    use crate::buttons::ButtonType;
    use formoxus_spec::{BuiltinWidget, RenderedInputType};
    use googletest::prelude::*;
    use std::borrow::Cow;

    #[gtest]
    fn a_base_alone_is_the_value_whatever_the_state() {
        let slot = Slot::new("fx-control fx-input");
        expect_that!(slot.value(false), some(eq("fx-control fx-input")));
        expect_that!(slot.value(true), some(eq("fx-control fx-input")));
    }

    #[gtest]
    fn the_invalid_class_follows_the_base_only_while_invalid() {
        let slot = Slot::new("fx-form-field").with_invalid("fx-invalid");
        expect_that!(slot.value(false), some(eq("fx-form-field")));
        expect_that!(slot.value(true), some(eq("fx-form-field fx-invalid")));
    }

    /// The bug this guards against: `invalid` read as "this slot has an
    /// invalid class" would panic on every errored field's control, since
    /// `fx()` gives no control one.
    #[gtest]
    fn an_invalid_field_with_no_invalid_class_renders_its_base() {
        let slot = Slot::new("fx-control fx-select");
        expect_that!(slot.invalid, none());
        expect_that!(slot.value(true), some(eq("fx-control fx-select")));
    }

    /// No leading space when only the invalid class applies.
    #[gtest]
    fn an_empty_base_leaves_just_the_invalid_class() {
        let slot = Slot::new("").with_invalid("is-invalid");
        expect_that!(slot.value(false), none());
        expect_that!(slot.value(true), some(eq("is-invalid")));
    }

    /// `None`, not `Some("")`, so `rsx!` omits the attribute instead of
    /// rendering `class=""`.
    #[gtest]
    fn an_empty_slot_has_no_value() {
        expect_that!(Slot::new("").value(false), none());
        expect_that!(Slot::new("").value(true), none());
        expect_that!(Slot::new("").with_invalid("").value(true), none());
    }

    #[gtest]
    fn a_run_time_name_works_like_a_literal() {
        let slot = Slot {
            base: Cow::Owned(format!("btn-{}", "primary")),
            invalid: None,
        }
        .with_invalid(String::from("is-invalid"));
        expect_that!(slot.value(true), some(eq("btn-primary is-invalid")));
        expect_that!(
            slot,
            eq(&Slot::new("btn-primary").with_invalid("is-invalid"))
        );
    }

    // ── The presets ──────────────────────────────────────────────────────

    fn base_of(classes: &Classes, key: SlotKey) -> Option<(&str, Option<&str>)> {
        classes
            .slots
            .get(&key)
            .map(|s| (&*s.base, s.invalid.as_deref()))
    }

    /// Every part a rendered widget has gets an `fx()` class, except a hidden
    /// input, and nothing else does. The widgets not rendered yet get none.
    #[gtest]
    fn fx_classes_every_part_of_every_rendered_widget_but_hidden() {
        let fx = Classes::fx();
        let hidden = BuiltinWidget::Input(RenderedInputType::Hidden);
        for &widget in BuiltinWidget::ALL {
            for part in widget.parts() {
                let expected = widget.is_rendered() && widget != hidden;
                expect_that!(
                    fx.slots.contains_key(&SlotKey::Part(widget, part.name)),
                    eq(expected),
                    "{widget:?}'s `{}`",
                    part.name
                );
            }
        }
    }

    /// The values the literals rendered before P3, from the plan's
    /// inventory. One widget per row of the table, since `fx()` gives every
    /// text-like input the same classes.
    #[gtest]
    fn fx_gives_each_part_its_old_literal() {
        use BuiltinWidget::{Input, RadioGroup, Select, Textarea};
        use RenderedInputType::{Checkbox, Email, Text};
        let fx = Classes::fx();
        let part = |w, p| base_of(&fx, SlotKey::Part(w, p));
        let wrapper = Some(("fx-form-field", Some("fx-invalid")));
        let label = Some(("fx-field-label", None));
        for w in [Input(Text), Input(Email), Input(Checkbox), Textarea, Select] {
            expect_that!(part(w, "div"), eq(wrapper), "{w:?}");
            expect_that!(part(w, "label"), eq(label), "{w:?}");
        }
        expect_that!(
            part(Input(Text), "input"),
            eq(Some(("fx-control fx-input", None)))
        );
        expect_that!(
            part(Input(Email), "input"),
            eq(Some(("fx-control fx-input", None)))
        );
        expect_that!(
            part(Input(Checkbox), "input"),
            eq(Some(("fx-control fx-checkbox", None)))
        );
        expect_that!(
            part(Textarea, "textarea"),
            eq(Some(("fx-control fx-textarea", None)))
        );
        expect_that!(
            part(Select, "select"),
            eq(Some(("fx-control fx-select", None)))
        );
        expect_that!(
            part(RadioGroup, "fieldset"),
            eq(Some(("fx-form-field fx-radio-group", Some("fx-invalid"))))
        );
        expect_that!(
            part(RadioGroup, "legend"),
            eq(Some(("fx-legend fx-field-label", None)))
        );
        expect_that!(part(RadioGroup, "div"), eq(Some(("fx-choice", None))));
        expect_that!(
            part(RadioGroup, "input"),
            eq(Some(("fx-control fx-radio", None)))
        );
        expect_that!(
            part(RadioGroup, "label"),
            eq(Some(("fx-choice-label", None)))
        );
    }

    #[gtest]
    fn fx_gives_each_fixed_slot_its_old_literal() {
        let fx = Classes::fx();
        let expected = [
            (FixedKey::Form, "fx-form"),
            (FixedKey::FormTitle, "fx-form-title"),
            (FixedKey::FormErrors, "fx-form-errors"),
            (FixedKey::FormError, "fx-form-error"),
            (FixedKey::ButtonProblems, "fx-form-errors fx-problems"),
            (FixedKey::ButtonProblem, "fx-form-error fx-problem"),
            (FixedKey::ButtonRow, "fx-buttons"),
            (
                FixedKey::Button(ButtonType::Submit),
                "fx-button fx-button-submit",
            ),
            (
                FixedKey::Button(ButtonType::Reset),
                "fx-button fx-button-reset",
            ),
            (
                FixedKey::Button(ButtonType::Cancel),
                "fx-button fx-button-cancel",
            ),
            (
                FixedKey::Button(ButtonType::Destructive),
                "fx-button fx-button-destructive",
            ),
            (
                FixedKey::Button(ButtonType::Action),
                "fx-button fx-button-action",
            ),
            (FixedKey::AddRowButton, "fx-button fx-button-add-row"),
            (FixedKey::RemoveRowButton, "fx-button fx-button-remove-row"),
            (FixedKey::FieldSet, "fx-group fx-field-set"),
            (FixedKey::FieldSetLegend, "fx-legend fx-field-label"),
            (FixedKey::ListSet, "fx-group fx-list-set"),
            (FixedKey::ListSetLegend, "fx-legend fx-field-label"),
            (FixedKey::ListRow, "fx-form-row"),
            (FixedKey::VariantSet, "fx-group fx-variant-set"),
            (FixedKey::VariantSetLegend, "fx-legend fx-field-label"),
            (FixedKey::RequiredMarker, "fx-required"),
            (FixedKey::FieldErrors, "fx-field-errors"),
            (FixedKey::FieldError, "fx-field-error"),
        ];
        for (slot, base) in expected {
            expect_that!(
                base_of(&fx, SlotKey::Fixed(slot)),
                eq(Some((base, None))),
                "{slot:?}"
            );
        }
        // And no fixed slot beyond these: the list above is all of them.
        let fixed = fx
            .slots
            .keys()
            .filter(|k| matches!(k, SlotKey::Fixed(_)))
            .count();
        expect_that!(fixed, eq(expected.len()));
    }

    #[gtest]
    fn bare_has_no_slots_and_the_default_is_fx() {
        expect_that!(Classes::bare().slots.len(), eq(0));
        expect_that!(Classes::default(), eq(&Classes::fx()));
    }

    // ── The builders ─────────────────────────────────────────────────────

    #[gtest]
    fn with_part_replaces_one_slot_and_nothing_else() {
        let select = BuiltinWidget::Select;
        let changed = Classes::fx().with_part(select, "select", Slot::new("form-select"));
        expect_that!(
            base_of(&changed, SlotKey::Part(select, "select")),
            eq(Some(("form-select", None)))
        );
        let mut unchanged = changed.clone();
        unchanged.slots.insert(
            SlotKey::Part(select, "select"),
            Slot::new("fx-control fx-select"),
        );
        expect_that!(unchanged, eq(&Classes::fx()));
    }

    #[gtest]
    #[should_panic(
        expected = "unknown part `lable` for widget Textarea (it has div, label, textarea)"
    )]
    fn with_part_panics_on_a_part_the_widget_lacks_naming_the_ones_it_has() {
        let _ = Classes::fx().with_part(BuiltinWidget::Textarea, "lable", Slot::new("x"));
    }

    #[gtest]
    fn with_fixed_replaces_one_slot_and_nothing_else() {
        let row = SlotKey::Fixed(FixedKey::ButtonRow);
        let changed = Classes::fx().with_fixed(
            FixedKey::ButtonRow,
            Slot::new("actions").with_invalid("has-problems"),
        );
        expect_that!(
            base_of(&changed, row),
            eq(Some(("actions", Some("has-problems"))))
        );
        let mut unchanged = changed.clone();
        unchanged.slots.insert(row, Slot::new("fx-buttons"));
        expect_that!(unchanged, eq(&Classes::fx()));
    }

    /// On `bare()` it adds the slot rather than replacing one.
    #[gtest]
    fn with_fixed_adds_a_slot_to_bare() {
        let classes = Classes::bare().with_fixed(FixedKey::Form, Slot::new("form"));
        expect_that!(classes.slots.len(), eq(1));
        expect_that!(
            base_of(&classes, SlotKey::Fixed(FixedKey::Form)),
            eq(Some(("form", None)))
        );
    }

    #[gtest]
    fn with_parts_classes_every_kept_widget_that_has_the_part() {
        use RenderedInputType::{Checkbox, Hidden, Password, Text};
        let text_like = |w| matches!(w, BuiltinWidget::Input(t) if t != Checkbox && t != Hidden);
        let classes = Classes::bare().with_parts(text_like, "input", Slot::new("form-control"));
        for w in [BuiltinWidget::Input(Text), BuiltinWidget::Input(Password)] {
            expect_that!(
                base_of(&classes, SlotKey::Part(w, "input")),
                eq(Some(("form-control", None))),
                "{w:?}"
            );
        }
        for w in [BuiltinWidget::Input(Checkbox), BuiltinWidget::Input(Hidden)] {
            expect_that!(
                base_of(&classes, SlotKey::Part(w, "input")),
                none(),
                "{w:?}"
            );
        }
        // 13 rendered text-like types; nothing else was touched.
        expect_that!(classes.slots.len(), eq(13));
    }

    /// `|_| true` keeps every widget, and only the ones with a legend get one.
    #[gtest]
    fn with_parts_skips_a_kept_widget_without_the_part() {
        let classes = Classes::bare().with_parts(|_| true, "legend", Slot::new("x"));
        let with_legend: Vec<BuiltinWidget> = classes
            .slots
            .keys()
            .map(|k| match k {
                SlotKey::Part(w, _) => *w,
                SlotKey::Fixed(f) => panic!("a fixed slot appeared: {f:?}"),
            })
            .collect();
        expect_that!(
            with_legend,
            unordered_elements_are![
                eq(&BuiltinWidget::RadioGroup),
                eq(&BuiltinWidget::CheckboxMultiple)
            ]
        );
    }

    #[gtest]
    #[should_panic(expected = "no widget the filter keeps has a part named `textarea`")]
    fn with_parts_panics_when_no_kept_widget_has_the_part() {
        let _ = Classes::bare().with_parts(
            |w| matches!(w, BuiltinWidget::Input(_)),
            "textarea",
            Slot::new("x"),
        );
    }

    // ── The lookups ──────────────────────────────────────────────────────

    #[gtest]
    fn part_finds_a_slot_and_bare_has_none() {
        let select = BuiltinWidget::Select;
        expect_that!(
            Classes::fx().part(select, "select"),
            some(eq(&Slot::new("fx-control fx-select")))
        );
        expect_that!(Classes::bare().part(select, "select"), none());
    }

    #[gtest]
    #[should_panic(expected = "unknown part `selct` for widget Select (it has div, label, select)")]
    fn part_panics_on_a_part_the_widget_lacks() {
        let _ = Classes::fx().part(BuiltinWidget::Select, "selct");
    }

    /// A part the widget HAS but `fx()` leaves bare is `None`, not a panic.
    #[gtest]
    fn a_real_part_with_no_slot_is_none() {
        let hidden = BuiltinWidget::Input(RenderedInputType::Hidden);
        expect_that!(Classes::fx().part(hidden, "input"), none());
        expect_that!(Classes::fx().part_class(hidden, "input", true), none());
    }

    #[gtest]
    fn fixed_finds_a_slot_and_bare_has_none() {
        expect_that!(
            Classes::fx().fixed(FixedKey::ButtonRow),
            some(eq(&Slot::new("fx-buttons")))
        );
        expect_that!(Classes::bare().fixed(FixedKey::ButtonRow), none());
    }

    /// `invalid` reaches `Slot::value`: the wrapper gains `fx-invalid`, the
    /// control, which has no invalid class, does not change.
    #[gtest]
    fn part_class_follows_the_fields_state() {
        let text = BuiltinWidget::Input(RenderedInputType::Text);
        let fx = Classes::fx();
        expect_that!(fx.part_class(text, "div", false), some(eq("fx-form-field")));
        expect_that!(
            fx.part_class(text, "div", true),
            some(eq("fx-form-field fx-invalid"))
        );
        expect_that!(
            fx.part_class(text, "input", true),
            some(eq("fx-control fx-input"))
        );
        expect_that!(Classes::bare().part_class(text, "div", true), none());
    }

    #[gtest]
    fn fixed_class_follows_the_fields_state() {
        let classes = Classes::bare().with_fixed(
            FixedKey::FieldErrors,
            Slot::new("errors").with_invalid("shown"),
        );
        expect_that!(
            classes.fixed_class(FixedKey::FieldErrors, false),
            some(eq("errors"))
        );
        expect_that!(
            classes.fixed_class(FixedKey::FieldErrors, true),
            some(eq("errors shown"))
        );
        expect_that!(classes.fixed_class(FixedKey::Form, false), none());
        expect_that!(
            Classes::fx().fixed_class(FixedKey::Form, false),
            some(eq("fx-form"))
        );
    }
}
