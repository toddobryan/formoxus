use crate::{Element, FieldControl, FieldType, InputType, RenderedInputType, attrs::str_eq};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinWidget {
    Input(RenderedInputType),
    Textarea,
    Select,
    RadioGroup,
    // not rendered yet
    SelectMultiple,
    CheckboxMultiple,
    File,
}

/// A labelled widget's parts, label before the control: `div`, `label`,
/// then `control`.
///
/// Inside `const { }` because `&[…]` is promoted to `'static` only when every
/// element is a literal; a call, even to a `const fn` like
/// `Part::from_element`, is not. The block evaluates it at compile time.
macro_rules! labelled {
    ($control:expr) => {
        const {
            &[
                Part::from_element(Element::Div),
                Part::from_element(Element::Label),
                Part::from_element(Element::Control($control)),
            ]
        }
    };
}

/// A group widget's parts: `fieldset`, `legend`, then per choice a `div`
/// holding `control` before its `label`. `const { }` for the reason
/// [`labelled!`] gives.
macro_rules! grouped {
    ($control:expr) => {
        const {
            &[
                Part::from_element(Element::Fieldset),
                Part::from_element(Element::Legend),
                Part::from_element(Element::Div),
                Part::from_element(Element::Control($control)),
                Part::from_element(Element::Label),
            ]
        }
    };
}

impl BuiltinWidget {
    /// Every built-in widget, rendered or not: one `Input` per
    /// `RenderedInputType`, then the rest.
    pub const ALL: &[Self] = &[
        Self::Input(RenderedInputType::Text),
        Self::Input(RenderedInputType::Password),
        Self::Input(RenderedInputType::Hidden),
        Self::Input(RenderedInputType::Number),
        Self::Input(RenderedInputType::Email),
        Self::Input(RenderedInputType::Telephone),
        Self::Input(RenderedInputType::Url),
        Self::Input(RenderedInputType::Search),
        Self::Input(RenderedInputType::Color),
        Self::Input(RenderedInputType::Date),
        Self::Input(RenderedInputType::Time),
        Self::Input(RenderedInputType::DatetimeLocal),
        Self::Input(RenderedInputType::Month),
        Self::Input(RenderedInputType::Week),
        Self::Input(RenderedInputType::Checkbox),
        Self::Textarea,
        Self::Select,
        Self::RadioGroup,
        Self::SelectMultiple,
        Self::CheckboxMultiple,
        Self::File,
    ];

    /// The widget an author names after `widget:` in `form!`, or `None` if
    /// `name` is not one. The inverse of [`Self::form_name`].
    ///
    /// Replaces `widget_tokens`/`widget_variant` in
    /// `formoxus-macros/src/form/widget.rs`. A `const fn` cannot `match` on a
    /// `&str`, so this loops over every widget comparing `form_name`s with
    /// `str_eq`, as `Attr::from_variant_name` does. That needs [`Self::ALL`],
    /// which also replaces the macro's `WIDGET_NAMES` for the "unknown widget"
    /// suggestions.
    pub const fn from_form_name(name: &str) -> Option<Self> {
        let mut i = 0;
        while i < Self::ALL.len() {
            let widget = Self::ALL[i];
            if str_eq(name, widget.form_name()) {
                return Some(widget);
            }
            i += 1;
        }
        None
    }

    /// The name an author writes after `widget:` in `form!`.
    ///
    /// HTML's spelling where HTML has one, with `_` for a hyphen:
    /// `Input(Telephone)` → `"tel"`, `Input(DatetimeLocal)` →
    /// `"datetime_local"`. Formoxus's own name otherwise: `RadioGroup` →
    /// `"radio_group"`. The left column of the macro's `widgets!` table.
    pub const fn form_name(&self) -> &'static str {
        match self {
            Self::Input(RenderedInputType::Text) => "text",
            Self::Input(RenderedInputType::Password) => "password",
            Self::Input(RenderedInputType::Hidden) => "hidden",
            Self::Input(RenderedInputType::Number) => "number",
            Self::Input(RenderedInputType::Email) => "email",
            Self::Input(RenderedInputType::Telephone) => "tel",
            Self::Input(RenderedInputType::Url) => "url",
            Self::Input(RenderedInputType::Search) => "search",
            Self::Input(RenderedInputType::Color) => "color",
            Self::Input(RenderedInputType::Date) => "date",
            Self::Input(RenderedInputType::Time) => "time",
            Self::Input(RenderedInputType::DatetimeLocal) => "datetime_local",
            Self::Input(RenderedInputType::Month) => "month",
            Self::Input(RenderedInputType::Week) => "week",
            Self::Input(RenderedInputType::Checkbox) => "checkbox",
            Self::Textarea => "textarea",
            Self::Select => "select",
            Self::SelectMultiple => "select_multiple",
            Self::CheckboxMultiple => "checkbox_multiple",
            Self::RadioGroup => "radio_group",
            Self::File => "file",
        }
    }

    /// The elements this widget renders, in document order, each named by
    /// its element:
    ///
    /// - `Input(Hidden)`: `input`
    /// - `Input(Checkbox)`: `div`, `input`, `label` (box first)
    /// - any other `Input`: `div`, `label`, `input`
    /// - `Textarea`: `div`, `label`, `textarea`
    /// - `Select`: `div`, `label`, `select`
    /// - `RadioGroup`: `fieldset`, `legend`, `div`, `input`, `label`
    ///
    /// The not-yet widgets get the parts they are planned to have:
    /// `SelectMultiple` and `File` like `Select`, `CheckboxMultiple` like
    /// `RadioGroup` with checkboxes.
    ///
    /// Each text-like `Input` has an arm of its own, with no `Input(_)`
    /// wildcard, because its `input` part carries its own `InputType` and a
    /// `const { }` block cannot read `self`. A new `RenderedInputType` stops
    /// this compiling until it gets one.
    pub const fn parts(&self) -> &'static [Part] {
        use RenderedInputType as R;
        match self {
            Self::Input(R::Hidden) => {
                const {
                    &[Part::from_element(Element::Control(FieldControl::Input(
                        InputType::Hidden,
                    )))]
                }
            }
            Self::Input(R::Checkbox) => {
                const {
                    &[
                        Part::from_element(Element::Div),
                        Part::from_element(Element::Control(FieldControl::Input(
                            InputType::Checkbox,
                        ))),
                        Part::from_element(Element::Label),
                    ]
                }
            }
            Self::Input(R::Text) => labelled!(FieldControl::Input(InputType::Text)),
            Self::Input(R::Password) => labelled!(FieldControl::Input(InputType::Password)),
            Self::Input(R::Number) => labelled!(FieldControl::Input(InputType::Number)),
            Self::Input(R::Email) => labelled!(FieldControl::Input(InputType::Email)),
            Self::Input(R::Telephone) => labelled!(FieldControl::Input(InputType::Telephone)),
            Self::Input(R::Url) => labelled!(FieldControl::Input(InputType::Url)),
            Self::Input(R::Search) => labelled!(FieldControl::Input(InputType::Search)),
            Self::Input(R::Color) => labelled!(FieldControl::Input(InputType::Color)),
            Self::Input(R::Date) => labelled!(FieldControl::Input(InputType::Date)),
            Self::Input(R::Time) => labelled!(FieldControl::Input(InputType::Time)),
            Self::Input(R::DatetimeLocal) => {
                labelled!(FieldControl::Input(InputType::DatetimeLocal))
            }
            Self::Input(R::Month) => labelled!(FieldControl::Input(InputType::Month)),
            Self::Input(R::Week) => labelled!(FieldControl::Input(InputType::Week)),
            Self::Textarea => labelled!(FieldControl::Textarea),
            Self::Select | Self::SelectMultiple => labelled!(FieldControl::Select),
            Self::File => labelled!(FieldControl::Input(InputType::File)),
            Self::RadioGroup => grouped!(FieldControl::Input(InputType::Radio)),
            Self::CheckboxMultiple => grouped!(FieldControl::Input(InputType::Checkbox)),
        }
    }

    /// The one part that IS the form control, which validated attributes
    /// (`required`, `min`, `pattern`, …) are checked against and emitted on.
    ///
    /// `Input(t)` → `Input(t.to_input_type())`; `Textarea`; `Select`;
    /// `RadioGroup` → `Input(Radio)` (P2-a: its validated attributes go to
    /// the radios, not the fieldset). Not-yet: `SelectMultiple` → `Select`,
    /// `CheckboxMultiple` → `Input(Checkbox)`, `File` → `Input(File)`.
    pub const fn form_control(&self) -> FieldControl {
        match self {
            BuiltinWidget::Input(t) => FieldControl::Input(t.to_input_type()),
            BuiltinWidget::Textarea => FieldControl::Textarea,
            BuiltinWidget::Select | BuiltinWidget::SelectMultiple => FieldControl::Select,
            BuiltinWidget::RadioGroup => FieldControl::Input(InputType::Radio),
            BuiltinWidget::CheckboxMultiple => FieldControl::Input(InputType::Checkbox),
            BuiltinWidget::File => FieldControl::Input(InputType::File),
        }
    }

    /// Whether `widget: x { choices: … }` is allowed at all: `Select`,
    /// `RadioGroup`, `SelectMultiple`, `CheckboxMultiple`. Replaces the
    /// macro's `CHOOSERS`. Whether choices are REQUIRED is [`Self::renders`]'s
    /// question, not this one's.
    pub const fn takes_choices(&self) -> bool {
        matches!(
            self,
            BuiltinWidget::Select
                | BuiltinWidget::RadioGroup
                | BuiltinWidget::SelectMultiple
                | BuiltinWidget::CheckboxMultiple
        )
    }

    /// Whether this widget can render an `Option<_>` field. `false` for
    /// `RadioGroup` (a picked radio cannot be un-picked) and `Input(Checkbox)`
    /// (two states, and an `Option<bool>` has three); `select` is the way out
    /// for both. Replaces the `refusal` match in `WidgetRef::checks`.
    pub const fn accepts_optional(&self) -> bool {
        !matches!(
            self,
            BuiltinWidget::Input(RenderedInputType::Checkbox) | BuiltinWidget::RadioGroup
        )
    }

    /// Whether this widget can render a single-value field of `field_type`,
    /// given whether `form!` gave it `choices`. Optionality is
    /// [`Self::accepts_optional`]'s job, not this one's.
    ///
    /// - `Input(Checkbox)`: `Bool` only
    /// - any other `Input`: `Text`, `Int`, `Float`, never `Bool`
    /// - `Textarea`: `Text` only
    /// - `Select`, `RadioGroup`: `Bool` always (its choices derive), anything
    ///   else only with `has_choices`
    /// - not-yet widgets: `false`
    ///
    /// Replaces `field_kind::WidgetClass` and `renders`'s inner match.
    /// `field_kind::renders` keeps its early `true` for an enum, a struct or
    /// an out-of-reach newtype, maps the shape to a `FieldType`, and defers
    /// here; the shape side stays in `formoxus`, which has facet.
    pub const fn renders(&self, field_type: FieldType, has_choices: bool) -> bool {
        match self {
            BuiltinWidget::Input(RenderedInputType::Checkbox) => {
                matches!(field_type, FieldType::Bool)
            }
            BuiltinWidget::Input(_) => matches!(
                field_type,
                FieldType::Text | FieldType::Int | FieldType::Float
            ),
            BuiltinWidget::Textarea => matches!(field_type, FieldType::Text),
            BuiltinWidget::Select | BuiltinWidget::RadioGroup => {
                matches!(field_type, FieldType::Bool) || has_choices
            }
            BuiltinWidget::SelectMultiple
            | BuiltinWidget::CheckboxMultiple
            | BuiltinWidget::File => false,
        }
    }

    /// The widget a field gets when `form!` names none.
    ///
    /// `Text`, `Int`, `Float` → `Input(Text)`, optional or not (not `Number`:
    /// `type="number"` hands back `""` for a half-typed value). `Bool` →
    /// `Input(Checkbox)`; an optional `Bool` → `Select`, whose blank option is
    /// the third state. Replaces `FormField::default_widget`.
    pub const fn default_for(field_type: FieldType, optional: bool) -> Self {
        match field_type {
            FieldType::Text | FieldType::Int | FieldType::Float => {
                BuiltinWidget::Input(RenderedInputType::Text)
            }
            FieldType::Bool if optional => BuiltinWidget::Select,
            FieldType::Bool => BuiltinWidget::Input(RenderedInputType::Checkbox),
        }
    }

    /// Whether anything renders this widget yet. `false` for
    /// `SelectMultiple`, `CheckboxMultiple` and `File`, which `form!` rejects
    /// while parsing so that the field does not silently vanish. Replaces the
    /// macro's `Rule::NotYet`.
    pub const fn is_rendered(&self) -> bool {
        !matches!(
            self,
            BuiltinWidget::CheckboxMultiple | BuiltinWidget::SelectMultiple | BuiltinWidget::File
        )
    }
}

/// One element a widget renders, named by that element (`"label"`,
/// `"input"`), so that class slots and per-part attributes can address it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Part {
    pub name: &'static str,
    pub element: Element,
}

impl Part {
    const fn from_element(element: Element) -> Self {
        Part {
            name: element.tag(),
            element,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuiltinWidget, Part};
    use crate::{Element, FieldType, RenderedInputType};
    use googletest::prelude::*;

    const FIELD_TYPES: [FieldType; 4] = [
        FieldType::Text,
        FieldType::Int,
        FieldType::Float,
        FieldType::Bool,
    ];

    /// `ALL` has to name every widget, or every test below checks a subset.
    /// The `match` has no wildcard, so a new variant (of `BuiltinWidget` or of
    /// `RenderedInputType`) stops this from COMPILING until it is listed here,
    /// which is the prompt to add it to `ALL` too. With 21 entries and no
    /// duplicates, `ALL` then holds each of the 21 exactly once.
    #[gtest]
    fn all_lists_every_widget_once() {
        use RenderedInputType as R;
        for w in BuiltinWidget::ALL {
            match w {
                BuiltinWidget::Input(
                    R::Text
                    | R::Password
                    | R::Hidden
                    | R::Number
                    | R::Email
                    | R::Telephone
                    | R::Url
                    | R::Search
                    | R::Color
                    | R::Date
                    | R::Time
                    | R::DatetimeLocal
                    | R::Month
                    | R::Week
                    | R::Checkbox,
                )
                | BuiltinWidget::Textarea
                | BuiltinWidget::Select
                | BuiltinWidget::RadioGroup
                | BuiltinWidget::SelectMultiple
                | BuiltinWidget::CheckboxMultiple
                | BuiltinWidget::File => {}
            }
        }
        let mut seen = BuiltinWidget::ALL.to_vec();
        seen.sort_by_key(|w| format!("{w:?}"));
        seen.dedup();
        expect_that!(seen.len(), eq(21));
        expect_that!(BuiltinWidget::ALL.len(), eq(21));
    }

    /// Also proves the names are unique: `from_form_name` returns the FIRST
    /// widget with a name, so a second widget sharing it would get the first
    /// one back.
    #[gtest]
    fn from_form_name_inverts_form_name() {
        for &w in BuiltinWidget::ALL {
            expect_that!(BuiltinWidget::from_form_name(w.form_name()), some(eq(w)));
        }
    }

    #[gtest]
    fn from_form_name_refuses_what_is_not_a_form_name() {
        expect_that!(BuiltinWidget::from_form_name(""), none());
        expect_that!(BuiltinWidget::from_form_name("textbox"), none());
        // HTML's spellings, where they differ from `form!`'s.
        expect_that!(BuiltinWidget::from_form_name("datetime-local"), none());
        expect_that!(BuiltinWidget::from_form_name("telephone"), none());
        // The variant name is not the form name.
        expect_that!(BuiltinWidget::from_form_name("RadioGroup"), none());
    }

    /// An `<input>` widget is named by its `type=`, with `_` for the hyphen an
    /// ident cannot hold, so `form!`'s `tel` and HTML's `tel` cannot drift.
    #[gtest]
    fn an_input_widget_is_named_by_its_html_type() {
        for &w in BuiltinWidget::ALL {
            if let BuiltinWidget::Input(t) = w {
                expect_that!(
                    w.form_name(),
                    eq(&t.to_input_type().html_type().replace('-', "_")),
                    "{w:?}"
                );
            }
        }
    }

    fn controls(w: BuiltinWidget) -> Vec<Part> {
        w.parts()
            .iter()
            .copied()
            .filter(|p| matches!(p.element, Element::Control(_)))
            .collect()
    }

    /// Validated attributes are checked against `form_control()` and emitted
    /// on the part that is it, so there must be exactly one such part, and it
    /// must agree with `form_control()`.
    #[gtest]
    fn every_widget_has_exactly_one_form_control_part_and_it_is_form_control() {
        for &w in BuiltinWidget::ALL {
            expect_that!(
                controls(w),
                elements_are![field!(
                    &Part.element,
                    eq(Element::Control(w.form_control()))
                )],
                "{w:?}"
            );
        }
    }

    /// Parts are addressed by name (class slots, per-part attributes), so a
    /// name used twice in one widget would address two elements.
    #[gtest]
    fn part_names_are_unique_within_a_widget() {
        for &w in BuiltinWidget::ALL {
            let mut names: Vec<&str> = w.parts().iter().map(|p| p.name).collect();
            names.sort_unstable();
            names.dedup();
            expect_that!(names.len(), eq(w.parts().len()), "{w:?}");
        }
    }

    /// P1: every field has an accessible name from a `<label>`, except a
    /// hidden input, which nobody sees. A radio group's `<legend>` names the
    /// group; each radio still has its own label.
    #[gtest]
    fn every_widget_but_hidden_has_a_label_part() {
        for &w in BuiltinWidget::ALL {
            let has_label = w.parts().iter().any(|p| p.element == Element::Label);
            expect_that!(
                has_label,
                eq(w != BuiltinWidget::Input(RenderedInputType::Hidden)),
                "{w:?}"
            );
        }
    }

    /// Pins the table `FormField::default_widget` held before it moved here.
    #[gtest]
    fn default_for_is_text_for_scalars_and_checkbox_or_select_for_bools() {
        for optional in [false, true] {
            for t in [FieldType::Text, FieldType::Int, FieldType::Float] {
                expect_that!(
                    BuiltinWidget::default_for(t, optional),
                    eq(BuiltinWidget::Input(RenderedInputType::Text)),
                    "{t:?}, optional: {optional}"
                );
            }
        }
        expect_that!(
            BuiltinWidget::default_for(FieldType::Bool, false),
            eq(BuiltinWidget::Input(RenderedInputType::Checkbox))
        );
        expect_that!(
            BuiltinWidget::default_for(FieldType::Bool, true),
            eq(BuiltinWidget::Select)
        );
    }

    /// A field with no `widget:` must still render: its default is rendered,
    /// renders its type without `choices` (nobody gave any), and takes an
    /// `Option` when the field is one.
    #[gtest]
    fn every_default_renders_its_field() {
        for t in FIELD_TYPES {
            for optional in [false, true] {
                let w = BuiltinWidget::default_for(t, optional);
                let context = format!("{t:?}, optional: {optional} → {w:?}");
                expect_that!(w.is_rendered(), eq(true), "{context}");
                expect_that!(w.renders(t, false), eq(true), "{context}");
                if optional {
                    expect_that!(w.accepts_optional(), eq(true), "{context}");
                }
            }
        }
    }

    /// A widget that is not rendered yet must not claim any field, or the
    /// `const _` check would pass a pair that `ScalarWidget` cannot draw.
    /// And every rendered widget renders SOMETHING, or it would be dead.
    #[gtest]
    fn a_widget_renders_some_field_exactly_when_it_is_rendered() {
        for &w in BuiltinWidget::ALL {
            let renders_any = FIELD_TYPES
                .iter()
                .any(|&t| w.renders(t, false) || w.renders(t, true));
            expect_that!(renders_any, eq(w.is_rendered()), "{w:?}");
        }
    }

    /// `choices` can only change the answer for a widget that takes them;
    /// for any other, `form!` refuses them before `renders` is asked.
    #[gtest]
    fn choices_matter_only_to_a_widget_that_takes_them() {
        for &w in BuiltinWidget::ALL {
            if !w.takes_choices() {
                for t in FIELD_TYPES {
                    expect_that!(w.renders(t, true), eq(w.renders(t, false)), "{w:?}, {t:?}");
                }
            }
        }
    }

    /// Pins the rules `field_kind::renders` held before they moved here.
    #[gtest]
    fn renders_matches_the_rules_it_replaced() {
        use FieldType::{Bool, Float, Int, Text};
        let text = BuiltinWidget::Input(RenderedInputType::Text);
        let hidden = BuiltinWidget::Input(RenderedInputType::Hidden);
        let checkbox = BuiltinWidget::Input(RenderedInputType::Checkbox);
        for t in [Text, Int, Float] {
            expect_that!(text.renders(t, false), eq(true), "{t:?}");
            expect_that!(checkbox.renders(t, false), eq(false), "{t:?}");
        }
        expect_that!(text.renders(Bool, false), eq(false));
        expect_that!(hidden.renders(Bool, false), eq(false));
        expect_that!(checkbox.renders(Bool, false), eq(true));

        expect_that!(BuiltinWidget::Textarea.renders(Text, false), eq(true));
        for t in [Int, Float, Bool] {
            expect_that!(
                BuiltinWidget::Textarea.renders(t, false),
                eq(false),
                "{t:?}"
            );
        }

        for w in [BuiltinWidget::Select, BuiltinWidget::RadioGroup] {
            expect_that!(w.renders(Bool, false), eq(true), "{w:?}");
            for t in [Text, Int, Float] {
                expect_that!(w.renders(t, false), eq(false), "{w:?}, {t:?}");
                expect_that!(w.renders(t, true), eq(true), "{w:?}, {t:?}");
            }
        }
    }

    /// The two widgets that cannot say "no answer", and nothing else.
    #[gtest]
    fn only_checkbox_and_radio_group_refuse_an_optional_field() {
        let refusing: Vec<BuiltinWidget> = BuiltinWidget::ALL
            .iter()
            .copied()
            .filter(|w| !w.accepts_optional())
            .collect();
        expect_that!(
            refusing,
            unordered_elements_are![
                eq(&BuiltinWidget::Input(RenderedInputType::Checkbox)),
                eq(&BuiltinWidget::RadioGroup)
            ]
        );
    }
}
