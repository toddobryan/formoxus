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

use formoxus_spec::Tag;
use indexmap::IndexMap;

use crate::buttons::ButtonType;
use crate::widgets::{
    CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput, MonthInput,
    NumberInput, PasswordInput, RadioGroup, SearchInput, Select, TelInput, TextInput, Textarea,
    TimeInput, UrlInput, WeekInput, Widget,
};

/// Which slot a class is for: one part of one widget, by the widget's
/// `NAME` and the part's [`Tag`], or an element no widget renders.
///
/// Crate-private, and built only by [`Classes::key`] from a `W: Widget`, so a
/// widget's slot is never named by a string a caller wrote, and every part is
/// checked against `W::PARTS`. Here rather than in `formoxus-spec`: `form!`
/// no longer builds one while parsing, it emits [`ClassEdit::part`] instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SlotKey {
    Part(&'static str, Tag),
    Fixed(FixedKey),
}

/// One change a form makes to the classes it inherits: what a `form!`
/// `classes_plus:` entry becomes, applied on top of the app's [`Classes`] at
/// the `Form` boundary.
///
/// Names a slot, then changes its base, its invalid class, or both, each by
/// replacing or appending:
///
/// ```ignore
/// ClassEdit::part::<TextInput>(Tag::Input)
///     .replace_base("form-control")
///     .append_invalid("is-invalid")
/// ```
///
/// A piece the edit does not mention is left as it was. Opaque, so the slot
/// key inside stays an implementation detail and every part is checked.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassEdit {
    key: SlotKey,
    base: Option<Change>,
    invalid: Option<Change>,
}

/// What an edit does to one piece of a slot.
#[derive(Clone, Debug, PartialEq)]
enum Change {
    Replace(Cow<'static, str>),
    /// Space-joined after what is there.
    Append(Cow<'static, str>),
}

impl ClassEdit {
    /// An edit of `W`'s `tag` slot, changing nothing until a
    /// `replace_*`/`append_*` says what. Panics, listing `W::PARTS`, if `W`
    /// has no such part, as [`Classes::with_part`] does.
    pub fn part<W: Widget>(tag: Tag) -> Self {
        Self::of(Classes::key::<W>(tag))
    }

    /// An edit of the slot for `key`, changing nothing until told what.
    pub fn fixed(key: FixedKey) -> Self {
        Self::of(SlotKey::Fixed(key))
    }

    fn of(key: SlotKey) -> Self {
        Self {
            key,
            base: None,
            invalid: None,
        }
    }

    /// Make the slot's base `classes`.
    pub fn replace_base(mut self, classes: impl Into<Cow<'static, str>>) -> Self {
        self.base = Some(Change::Replace(classes.into()));
        self
    }

    /// Add `classes` after the slot's base.
    pub fn append_base(mut self, classes: impl Into<Cow<'static, str>>) -> Self {
        self.base = Some(Change::Append(classes.into()));
        self
    }

    /// Make the slot's invalid class `classes`; `""` removes it.
    pub fn replace_invalid(mut self, classes: impl Into<Cow<'static, str>>) -> Self {
        self.invalid = Some(Change::Replace(classes.into()));
        self
    }

    /// Add `classes` after the slot's invalid class, or make them its invalid
    /// class if it has none.
    pub fn append_invalid(mut self, classes: impl Into<Cow<'static, str>>) -> Self {
        self.invalid = Some(Change::Append(classes.into()));
        self
    }
}

impl Change {
    /// `current` after this change. `None` in and out means "nothing there".
    fn apply(&self, current: Option<&str>) -> Option<Cow<'static, str>> {
        let result: Cow<'static, str> = match self {
            Change::Replace(classes) => classes.clone(),
            Change::Append(classes) => match current {
                Some(current) if !current.is_empty() && !classes.is_empty() => {
                    Cow::Owned(format!("{current} {classes}"))
                }
                Some(current) if classes.is_empty() => Cow::Owned(current.to_string()),
                _ => classes.clone(),
            },
        };
        (!result.is_empty()).then_some(result)
    }
}

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
    ///
    /// **Why this builds a whole string instead of letting `rsx!` add a
    /// class.** The tempting version is two `class:` attributes on the
    /// element:
    ///
    /// ```text
    /// div { class: "fx-form-field", class: if invalid { "fx-invalid" }, … }
    /// ```
    ///
    /// `rsx!` does merge those — it joins every `class:` value with a space.
    /// But it joins them *unconditionally*: when `invalid` is false the second
    /// value is the empty string, and the result is `class="fx-form-field "`,
    /// with a trailing space, on every field that has no error. Harmless to a
    /// browser, but it is on nearly every field on every page, and it breaks
    /// any test that matches the attribute exactly. One joined string never
    /// produces the stray space.
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

/// The elements no widget renders, each with a slot of its own. Defined in
/// `formoxus-spec`, so `form!` can check a `classes_plus:` name against it at
/// compile time; re-exported here so `formoxus::classes::FixedKey` is its
/// address.
pub use formoxus_spec::FixedKey;

/// Every class formoxus emits, one [`Slot`] per element: a part of a widget,
/// or a [`FixedKey`] for what no widget renders.
///
/// A preset, one CSS vocabulary among possible others: a widget says only
/// WHERE classes can go (`Widget::PARTS`), and a `Classes` says what they are.
/// So a dev's own widget has no slots in [`Classes::fx`] until the app gives
/// it some. Start from a preset and change what differs:
///
/// ```ignore
/// Classes::fx().with_part::<Select>(Tag::Select, Slot::new("form-select"))
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

impl Classes {
    /// The slot for `W`'s `tag` part, or `None` if it has none (it then
    /// renders no `class`). Panics if `W` has no such part, as
    /// [`Self::with_part`] does; at a lookup that is a bug in the widget.
    pub fn part<W: Widget>(&self, tag: Tag) -> Option<&Slot> {
        self.slots.get(&Classes::key::<W>(tag))
    }

    /// The key for `W`'s `tag` slot, the one way a part's key is built.
    /// Panics, listing `W::PARTS`, if `W` has no such part: a tag the widget
    /// never renders a class on is a typo, and ignoring it would leave the
    /// class silently missing.
    fn key<W: Widget>(tag: Tag) -> SlotKey {
        assert!(
            has_part::<W>(tag),
            "`{}` has no part `{}` (it has {})",
            W::NAME,
            tag.as_str(),
            if W::PARTS.is_empty() {
                "none".to_string()
            } else {
                W::PARTS
                    .iter()
                    .map(Tag::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        SlotKey::Part(W::NAME, tag)
    }

    /// The slot for an element no widget renders, or `None` if it has none.
    pub fn fixed(&self, key: FixedKey) -> Option<&Slot> {
        self.slots.get(&SlotKey::Fixed(key))
    }

    /// The `class` value for `W`'s `tag` part in this state, or `None` to
    /// render no `class` at all: [`Self::part`], then [`Slot::value`].
    /// `invalid` is whether the field has errors.
    ///
    /// `Option` so a call site can write `class: classes.part_class(…)` and
    /// let `rsx!` omit the attribute on `None`.
    pub fn part_class<W: Widget>(&self, tag: Tag, invalid: bool) -> Option<String> {
        self.part::<W>(tag).and_then(|s| s.value(invalid))
    }

    /// The `class` value for an element no widget renders, as
    /// [`Self::part_class`] is for a widget part.
    pub fn fixed_class(&self, key: FixedKey, invalid: bool) -> Option<String> {
        self.fixed(key).and_then(|s| s.value(invalid))
    }

    /// `slot` on `W`'s `tag` part, replacing whatever was there:
    /// `Classes::fx().with_part::<Select>(Tag::Select, Slot::new("form-select"))`.
    /// How an app classes a dev's own widget, too. Panics, listing
    /// `W::PARTS`, if `W` has no such part.
    pub fn with_part<W: Widget>(mut self, tag: Tag, slot: Slot) -> Self {
        self.slots.insert(Classes::key::<W>(tag), slot);
        self
    }

    /// `value` on a slot no widget renders, replacing whatever was there:
    /// `Classes::fx().with_fixed(FixedKey::ButtonRow, Slot::new("actions"))`.
    pub fn with_fixed(mut self, slot: FixedKey, value: Slot) -> Self {
        self.slots.insert(SlotKey::Fixed(slot), value);
        self
    }

    /// These classes with `edit` applied: the slot it names, starting from an
    /// empty one if there is none, with its base and invalid class changed as
    /// the edit says. How `Form::classes()` lays a form's `classes_plus:` over
    /// what it inherits.
    #[must_use]
    pub fn with_edit(mut self, edit: &ClassEdit) -> Self {
        let slot = self.slots.entry(edit.key).or_insert_with(|| Slot::new(""));
        if let Some(change) = &edit.base {
            slot.base = change.apply(Some(&slot.base)).unwrap_or(Cow::Borrowed(""));
        }
        if let Some(change) = &edit.invalid {
            slot.invalid = change.apply(slot.invalid.as_deref());
        }
        self
    }

    /// formoxus's own classes, `fx-`-prefixed: what it rendered before classes
    /// were configurable, byte for byte (`tests/snapshots/` pins that). The
    /// default, so an app that says nothing gets these.
    ///
    /// The widget parts come from a loop over every built-in widget's
    /// `Widget::PARTS`, asking `fx_part` for each, so a built-in or a part
    /// added later panics here until it is given a class, instead of silently
    /// rendering unclassed. A dev's own widget is not in the list: a preset
    /// classes only the widgets it knows.
    pub fn fx() -> Self {
        let mut slots = IndexMap::new();
        for (name, parts) in BUILTINS {
            for &tag in parts {
                // Built directly, not through `key::<W>`: the name is all
                // there is here, and the tag comes from `W::PARTS` itself.
                let slot = fx_part(name, tag);
                slots.insert(SlotKey::Part(name, tag), slot);
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

/// Every built-in widget's `NAME` and `PARTS`, what [`Classes::fx`] loops
/// over. Named one by one, since nothing can iterate "every type that
/// implements `Widget`"; the array's length makes the compiler count.
const BUILTINS: [(&str, &[Tag]); 18] = [
    parts_of::<TextInput>(),
    parts_of::<PasswordInput>(),
    parts_of::<NumberInput>(),
    parts_of::<EmailInput>(),
    parts_of::<TelInput>(),
    parts_of::<UrlInput>(),
    parts_of::<SearchInput>(),
    parts_of::<ColorInput>(),
    parts_of::<DateInput>(),
    parts_of::<TimeInput>(),
    parts_of::<DatetimeLocalInput>(),
    parts_of::<MonthInput>(),
    parts_of::<WeekInput>(),
    parts_of::<HiddenInput>(),
    parts_of::<CheckboxInput>(),
    parts_of::<Textarea>(),
    parts_of::<Select>(),
    parts_of::<RadioGroup>(),
];

/// Whether `W` renders a class on `tag`: `tag` is in `W::PARTS`. The one
/// rule for it, read by [`Classes`]'s builders and lookups at run time and by
/// `form!`'s `classes_plus:` in a `const _` assert, where rustc answers it in
/// the dev's crate.
///
/// A `while` loop with [`Tag::same`], since a `const fn` can neither call
/// `contains` nor compare with a derived `==`.
pub const fn has_part<W: Widget>(tag: Tag) -> bool {
    let mut i = 0;
    while i < W::PARTS.len() {
        if W::PARTS[i].same(tag) {
            return true;
        }
        i += 1;
    }
    false
}

/// A widget's `NAME` and `PARTS`, one row of [`BUILTINS`].
const fn parts_of<W: Widget>() -> (&'static str, &'static [Tag]) {
    (W::NAME, W::PARTS)
}

/// `fx()`'s class for one part of one built-in widget.
///
/// Panics on a pair it does not know, which is a widget/part added
/// without saying what `fx()` gives it.
fn fx_part(widget_name: &'static str, tag: Tag) -> Slot {
    match (widget_name, tag) {
        // A radio group is a fieldset of choices, so its parts are named
        // apart from a single control's.
        (RadioGroup::NAME, Tag::Fieldset) => {
            Slot::new("fx-form-field fx-radio-group").with_invalid("fx-invalid")
        }
        (RadioGroup::NAME, Tag::Legend) => Slot::new("fx-legend fx-field-label"),
        (RadioGroup::NAME, Tag::Div) => Slot::new("fx-choice"),
        (RadioGroup::NAME, Tag::Input) => Slot::new("fx-control fx-radio"),
        (RadioGroup::NAME, Tag::Label) => Slot::new("fx-choice-label"),
        // Every other widget: the wrapper carries the invalid mark, the label
        // and the control do not.
        (_, Tag::Div) => Slot::new("fx-form-field").with_invalid("fx-invalid"),
        (_, Tag::Label) => Slot::new("fx-field-label"),
        (CheckboxInput::NAME, Tag::Input) => Slot::new("fx-control fx-checkbox"),
        (_, Tag::Input) => Slot::new("fx-control fx-input"),
        (Textarea::NAME, Tag::Textarea) => Slot::new("fx-control fx-textarea"),
        (Select::NAME, Tag::Select) => Slot::new("fx-control fx-select"),
        (widget, tag) => panic!(
            "`Classes::fx()` has no class for {widget}'s `{}`",
            tag.as_str()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{BUILTINS, ClassEdit, Classes, FixedKey, Slot, SlotKey};
    use crate::buttons::ButtonType;
    use crate::widgets::{
        CheckboxInput, EmailInput, HiddenInput, RadioGroup, Select, TextInput, Textarea, Widget,
        WidgetProps,
    };
    use dioxus::prelude::*;
    use formoxus_spec::{AttrSet, Tag};
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

    /// Every part every built-in declares gets an `fx()` class. A hidden
    /// input declares none, so it gets none.
    #[gtest]
    fn fx_classes_every_part_of_every_builtin() {
        let fx = Classes::fx();
        for (name, parts) in BUILTINS {
            for &tag in parts {
                expect_that!(
                    fx.slots.contains_key(&SlotKey::Part(name, tag)),
                    eq(true),
                    "{name}'s `{}`",
                    tag.as_str()
                );
            }
        }
    }

    /// And the other way: every part slot `fx()` sets is a part its widget
    /// declares, so `fx()` never classes an element nothing renders. What
    /// `fx_part`'s panic guarded when parts were strings.
    #[gtest]
    fn every_part_fx_classes_is_one_its_widget_declares() {
        for key in Classes::fx().slots.keys() {
            let SlotKey::Part(name, tag) = *key else {
                continue;
            };
            let parts = BUILTINS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, parts)| *parts);
            expect_that!(
                parts,
                some(contains(eq(&tag))),
                "{name}'s `{}`",
                tag.as_str()
            );
        }
    }

    /// The values the literals rendered before P3, from the plan's
    /// inventory. One widget per row of the table, since `fx()` gives every
    /// text-like input the same classes.
    #[gtest]
    fn fx_gives_each_part_its_old_literal() {
        let fx = Classes::fx();
        let part = |name, tag| base_of(&fx, SlotKey::Part(name, tag));
        let wrapper = Some(("fx-form-field", Some("fx-invalid")));
        let label = Some(("fx-field-label", None));
        for name in [
            TextInput::NAME,
            EmailInput::NAME,
            CheckboxInput::NAME,
            Textarea::NAME,
            Select::NAME,
        ] {
            expect_that!(part(name, Tag::Div), eq(wrapper), "{name}");
            expect_that!(part(name, Tag::Label), eq(label), "{name}");
        }
        expect_that!(
            part(TextInput::NAME, Tag::Input),
            eq(Some(("fx-control fx-input", None)))
        );
        expect_that!(
            part(EmailInput::NAME, Tag::Input),
            eq(Some(("fx-control fx-input", None)))
        );
        expect_that!(
            part(CheckboxInput::NAME, Tag::Input),
            eq(Some(("fx-control fx-checkbox", None)))
        );
        expect_that!(
            part(Textarea::NAME, Tag::Textarea),
            eq(Some(("fx-control fx-textarea", None)))
        );
        expect_that!(
            part(Select::NAME, Tag::Select),
            eq(Some(("fx-control fx-select", None)))
        );
        expect_that!(
            part(RadioGroup::NAME, Tag::Fieldset),
            eq(Some(("fx-form-field fx-radio-group", Some("fx-invalid"))))
        );
        expect_that!(
            part(RadioGroup::NAME, Tag::Legend),
            eq(Some(("fx-legend fx-field-label", None)))
        );
        expect_that!(
            part(RadioGroup::NAME, Tag::Div),
            eq(Some(("fx-choice", None)))
        );
        expect_that!(
            part(RadioGroup::NAME, Tag::Input),
            eq(Some(("fx-control fx-radio", None)))
        );
        expect_that!(
            part(RadioGroup::NAME, Tag::Label),
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

    // ── A dev's widget ───────────────────────────────────────────────────

    /// A widget no preset has heard of, with two parts.
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Notes;

    impl Widget for Notes {
        const NAME: &'static str = "notes";
        const PARTS: &'static [Tag] = &[Tag::Div, Tag::Textarea];
        const EXTENDS: AttrSet = AttrSet::EMPTY;
        fn render(&self, _props: WidgetProps) -> Element {
            rsx! {}
        }
    }

    /// `fx()` classes only the widgets it knows, so a dev's renders bare
    /// until the app says otherwise.
    #[gtest]
    fn fx_leaves_a_devs_widget_unclassed() {
        expect_that!(Classes::fx().part::<Notes>(Tag::Div), none());
        expect_that!(Classes::fx().part_class::<Notes>(Tag::Div, true), none());
    }

    #[gtest]
    fn with_part_classes_a_devs_widget() {
        let classes = Classes::fx().with_part::<Notes>(Tag::Textarea, Slot::new("notes-box"));
        expect_that!(
            classes.part_class::<Notes>(Tag::Textarea, false),
            some(eq("notes-box"))
        );
        expect_that!(classes.part::<Notes>(Tag::Div), none());
    }

    /// Two widgets with the same tag are two slots: the key is the widget's
    /// `NAME` as well as the tag.
    #[gtest]
    fn the_same_tag_on_two_widgets_is_two_slots() {
        let classes = Classes::fx().with_part::<Notes>(Tag::Textarea, Slot::new("notes-box"));
        expect_that!(
            classes.part_class::<Textarea>(Tag::Textarea, false),
            some(eq("fx-control fx-textarea"))
        );
    }

    // ── The builders ─────────────────────────────────────────────────────

    #[gtest]
    fn with_part_replaces_one_slot_and_nothing_else() {
        let key = SlotKey::Part(Select::NAME, Tag::Select);
        let changed = Classes::fx().with_part::<Select>(Tag::Select, Slot::new("form-select"));
        expect_that!(base_of(&changed, key), eq(Some(("form-select", None))));
        let mut unchanged = changed.clone();
        unchanged
            .slots
            .insert(key, Slot::new("fx-control fx-select"));
        expect_that!(unchanged, eq(&Classes::fx()));
    }

    #[gtest]
    #[should_panic(expected = "`textarea` has no part `legend` (it has div, label, textarea)")]
    fn with_part_panics_on_a_part_the_widget_lacks_naming_the_ones_it_has() {
        let _ = Classes::fx().with_part::<Textarea>(Tag::Legend, Slot::new("x"));
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

    // ── The lookups ──────────────────────────────────────────────────────

    #[gtest]
    fn part_finds_a_slot_and_bare_has_none() {
        expect_that!(
            Classes::fx().part::<Select>(Tag::Select),
            some(eq(&Slot::new("fx-control fx-select")))
        );
        expect_that!(Classes::bare().part::<Select>(Tag::Select), none());
    }

    #[gtest]
    #[should_panic(expected = "`select` has no part `input` (it has div, label, select)")]
    fn part_panics_on_a_part_the_widget_lacks() {
        let _ = Classes::fx().part::<Select>(Tag::Input);
    }

    /// A hidden input declares no parts, so asking for one is the widget's
    /// bug, not an unclassed part.
    #[gtest]
    #[should_panic(expected = "`hidden` has no part `input` (it has none)")]
    fn a_hidden_input_has_no_part_to_ask_for() {
        let _ = Classes::fx().part::<HiddenInput>(Tag::Input);
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
        let fx = Classes::fx();
        expect_that!(
            fx.part_class::<TextInput>(Tag::Div, false),
            some(eq("fx-form-field"))
        );
        expect_that!(
            fx.part_class::<TextInput>(Tag::Div, true),
            some(eq("fx-form-field fx-invalid"))
        );
        expect_that!(
            fx.part_class::<TextInput>(Tag::Input, true),
            some(eq("fx-control fx-input"))
        );
        expect_that!(
            Classes::bare().part_class::<TextInput>(Tag::Div, true),
            none()
        );
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

    // ── ClassEdit ────────────────────────────────────────────────────────

    #[gtest]
    fn a_class_edit_keys_its_part_by_the_widgets_name_and_changes_nothing_yet() {
        let edit = ClassEdit::part::<Select>(Tag::Select);
        expect_that!(edit.key, eq(SlotKey::Part(Select::NAME, Tag::Select)));
        expect_that!(edit.base, none());
        expect_that!(edit.invalid, none());
    }

    #[gtest]
    #[should_panic(expected = "`select` has no part `legend` (it has div, label, select)")]
    fn a_class_edit_panics_on_a_part_the_widget_lacks() {
        let _ = ClassEdit::part::<Select>(Tag::Legend);
    }

    fn select(classes: &Classes) -> Option<(&str, Option<&str>)> {
        base_of(classes, SlotKey::Part(Select::NAME, Tag::Select))
    }

    fn edited(edit: &ClassEdit) -> Classes {
        Classes::fx().with_edit(edit)
    }

    #[gtest]
    fn replacing_the_base_keeps_the_invalid_class() {
        let start =
            Classes::fx().with_part::<Select>(Tag::Select, Slot::new("a").with_invalid("bad"));
        let classes =
            start.with_edit(&ClassEdit::part::<Select>(Tag::Select).replace_base("form-select"));
        expect_that!(select(&classes), eq(Some(("form-select", Some("bad")))));
    }

    #[gtest]
    fn appending_to_the_base_joins_with_a_space() {
        let classes = edited(&ClassEdit::part::<Select>(Tag::Select).append_base("mine"));
        expect_that!(
            select(&classes),
            eq(Some(("fx-control fx-select mine", None)))
        );
    }

    /// An invalid-only edit is how Bootstrap's `is-invalid` gets onto an
    /// input without restating the input's base.
    #[gtest]
    fn an_invalid_only_edit_leaves_the_base_alone() {
        let classes = edited(&ClassEdit::part::<Select>(Tag::Select).append_invalid("is-invalid"));
        expect_that!(
            select(&classes),
            eq(Some(("fx-control fx-select", Some("is-invalid"))))
        );
    }

    #[gtest]
    fn each_piece_may_be_replaced_or_appended_independently() {
        let wrapper = SlotKey::Part(Select::NAME, Tag::Div);
        let classes = edited(
            &ClassEdit::part::<Select>(Tag::Div)
                .replace_base("mb-3")
                .append_invalid("has-error"),
        );
        expect_that!(
            base_of(&classes, wrapper),
            eq(Some(("mb-3", Some("fx-invalid has-error"))))
        );
    }

    #[gtest]
    fn replacing_the_invalid_class_with_nothing_removes_it() {
        let wrapper = SlotKey::Part(Select::NAME, Tag::Div);
        let classes = edited(&ClassEdit::part::<Select>(Tag::Div).replace_invalid(""));
        expect_that!(
            base_of(&classes, wrapper),
            eq(Some(("fx-form-field", None)))
        );
    }

    /// On `bare()` there is nothing to append to, so the edit's classes are
    /// all there is.
    #[gtest]
    fn an_edit_of_an_absent_slot_starts_from_an_empty_one() {
        let classes = Classes::bare().with_edit(
            &ClassEdit::fixed(FixedKey::ButtonRow)
                .append_base("actions")
                .append_invalid("x"),
        );
        expect_that!(
            base_of(&classes, SlotKey::Fixed(FixedKey::ButtonRow)),
            eq(Some(("actions", Some("x"))))
        );
    }

    #[gtest]
    fn edits_apply_in_order() {
        let classes = Classes::fx()
            .with_edit(&ClassEdit::fixed(FixedKey::Form).replace_base("a"))
            .with_edit(&ClassEdit::fixed(FixedKey::Form).append_base("b"))
            .with_edit(&ClassEdit::fixed(FixedKey::Form).replace_base("c"));
        expect_that!(
            base_of(&classes, SlotKey::Fixed(FixedKey::Form)),
            eq(Some(("c", None)))
        );
    }

    #[gtest]
    fn an_edit_that_changes_nothing_leaves_the_classes_as_they_were() {
        let classes = edited(&ClassEdit::fixed(FixedKey::Form));
        expect_that!(classes, eq(&Classes::fx()));
    }
}
