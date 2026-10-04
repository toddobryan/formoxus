use crate::FieldControl;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Owner {
    Formoxus,
    Author,
    Merged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttrType {
    String,
    Int,
    Bound,
    Regex,
    Flag,
    TokenList,
    Declarations,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    Text,
    Int,
    Float,
    Bool,
}

/// Turns a row's `validated:` into a `bool`, accepting exactly the three
/// spellings so that anything else is an error naming the column.
macro_rules! validated_flag {
    (false) => {
        false
    };
    (true) => {
        true
    };
    (true_and_on $also_on:pat) => {
        true
    };
    ($($other:tt)*) => {
        compile_error!(concat!(
            "`validated:` takes `false`, `true` or `true_and_on(...)`, not `",
            stringify!($($other)*),
            "`"
        ))
    };
}

/// The attribute table: one row per attribute, and every fact about it in that
/// row, so the generated accessors cannot disagree.
///
/// The columns, in order:
///
/// - `name`: the HTML attribute name. NOT the `form!` key, which is the
///   variant name in `snake_case` (`MaxLength` ↔ `max_length`).
/// - `owner`: who may set it. `Formoxus` attributes are refused in `form!`.
/// - `type`: the type of the attribute's VALUE, which decides how `form!`
///   parses it.
/// - `for`: the [`FieldType`]s it applies to.
/// - `on`: the [`FieldControl`]s it may be EMITTED on, straight from the spec.
/// - `validated`: whether formoxus's own validation enforces it. `false`;
///   `true` (enforced, and the browser enforces it too wherever it is
///   emitted); or `true_and_on(…)`, which is `true` PLUS these extra controls,
///   where it is accepted and validated by formoxus but not emitted.
///
/// [`Attr::is_allowed`] combines `for`, `on` and `validated` into the rule
/// `form!` applies.
macro_rules! attributes {
    ($(
        $variant:ident {
            name: $name:literal,
            owner: $owner:ident,
            type: $attr_type:ident,
            for: $field_type:pat,
            on: $on:pat,
            validated: $validated:tt $( ( $also_on:pat ) )? $(,)?
        }
    ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Attr {
            $( $variant ),+
        }

        impl Attr {
            pub const ALL: &[Attr] = &[ $( Self::$variant ),+ ];

            pub const fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )+
                }
            }

            pub const fn owner(self) -> Owner {
                match self {
                    $( Self::$variant => Owner::$owner, )+
                }
            }

            pub const fn attr_type(self) -> AttrType {
                match self {
                    $( Self::$variant => AttrType::$attr_type, )+
                }
            }

            #[deny(unused_variables)]
            pub const fn applies_to(self, field_type: FieldType) -> bool {
                use crate::FieldType::*;
                match (self, field_type) {
                    $( (Self::$variant, $field_type) => true, )+
                    _ => false,
                }
            }

            /// Whether the attribute may be EMITTED on this control.
            #[deny(unused_variables)]
            pub const fn is_valid_on(self, field_control: FieldControl) -> bool {
                use crate::FieldControl::*;
                use crate::InputType::*;
                match (self, field_control) {
                    $( (Self::$variant, $on) => true, )+
                    _ => false,
                }
            }

            /// Whether formoxus's own validation enforces it.
            pub const fn validated(self) -> bool {
                match self {
                    $( Self::$variant => validated_flag!($validated $($also_on)?), )+
                }
            }

            /// The controls from `true_and_on(…)`: accepted and validated by
            /// formoxus there, but not emitted, because HTML does not allow it.
            #[deny(unused_variables)]
            pub const fn is_also_validated_on(self, field_control: FieldControl) -> bool {
                use crate::FieldControl::*;
                use crate::InputType::*;
                match (self, field_control) {
                    $( $( (Self::$variant, $also_on) => true, )? )+
                    _ => false,
                }
            }

            pub const fn from_name(name: &str) -> Option<Self> {
                let mut i = 0;
                while i < Self::ALL.len() {
                    let attr = Self::ALL[i];
                    if str_eq(name, attr.name()) {
                        return Some(attr);
                    }
                    i += 1;
                }
                None
            }
        }
    }
}

impl Attr {
    /// Whether `form!` should accept this attribute on a field of `field_type`
    /// rendered as `field_control`:
    ///
    /// | `for` matches? | `on` matches? | `true_and_on` names it? | result   |
    /// |----------------|---------------|-------------------------|----------|
    /// | no             | —             | —                       | refused  |
    /// | yes            | yes           | —                       | accepted |
    /// | yes            | no            | yes                     | accepted |
    /// | yes            | no            | no                      | refused  |
    ///
    /// Accepted where `on` matches means emitted (and validated if
    /// [`Attr::validated`]); accepted only through `true_and_on` means
    /// validated by formoxus but not emitted. Ownership is a separate check:
    /// a [`Owner::Formoxus`] attribute is refused whatever this says.
    pub const fn is_allowed(self, field_type: FieldType, field_control: FieldControl) -> bool {
        self.applies_to(field_type)
            && (self.is_valid_on(field_control) || self.is_also_validated_on(field_control))
    }
}

attributes! {
    Placeholder {
        name: "placeholder",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(Text | Search | Telephone | Url | Email | Password | Number) | Textarea,
        validated: false,
    },
    MaxLength {
        name: "maxlength",
        owner: Author,
        type: Int,
        for: Text,
        on: Input(Text | Search | Telephone | Url | Email | Password) | Textarea,
        validated: true,
    },
    MinLength {
        name: "minlength",
        owner: Author,
        type: Int,
        for: Text,
        on: Input(Text | Search | Telephone | Url | Email | Password) | Textarea,
        validated: true,
    },
    // No `Textarea`: HTML has no `pattern` on one, and Todd ruled it out
    // (2026-10-02). With plain `true`, that makes it refused there; a regex
    // over multi-line text belongs in a per-field validator.
    Pattern {
        name: "pattern",
        owner: Author,
        type: Regex,
        for: Text,
        on: Input(Text | Search | Telephone | Url | Email | Password),
        validated: true,
    },
    // Valid in HTML only on number, range and the date/time types. A number
    // field rendered as `type="text"` (formoxus's default) is still validated
    // by formoxus; the attribute just is not emitted there.
    Min {
        name: "min",
        owner: Author,
        type: Bound,
        for: Int | Float,
        on: Input(Number | Range | Date | Month | Week | Time | DatetimeLocal),
        validated: true_and_on(Input(Text)),
    },
    Max {
        name: "max",
        owner: Author,
        type: Bound,
        for: Int | Float,
        on: Input(Number | Range | Date | Month | Week | Time | DatetimeLocal),
        validated: true_and_on(Input(Text)),
    },
    // The PRESENCE sense: formoxus emits it on every non-optional field, and
    // `validate` reports "This field is required." The spec allows `required`
    // on a checkbox too, but there it means "must be ticked", which is
    // `RequiredTrue`, so the checkbox is left out here on purpose.
    Required {
        name: "required",
        owner: Formoxus,
        type: Flag,
        for: Text | Int | Float | Bool,
        on: Input(
            Text | Search | Telephone | Url | Email | Password | Date | Month | Week | Time
                | DatetimeLocal | Number | Radio | File
        ) | Textarea
            | Select,
        validated: true,
    },
    // "I agree to the terms" (#6). Emits HTML `required`, but only a checkbox
    // gives that the "must be ticked" meaning; on a bool `select` or radio
    // group formoxus validates it and nothing is emitted.
    RequiredTrue {
        name: "required",
        owner: Author,
        type: Flag,
        for: Bool,
        on: Input(Checkbox),
        validated: true_and_on(Select | Input(Radio)),
    },
    // Owned by formoxus: the store key and the wire path.
    Name {
        name: "name",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned: comes from `widget:`.
    Type {
        name: "type",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_),
        validated: false,
    },
    // Owned: comes from the store. `<textarea>` and `<select>` have no
    // `value` ATTRIBUTE; Dioxus sets it as a DOM property there.
    Value {
        name: "value",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_),
        validated: false,
    },
    // Owned: comes from the store. Radios can carry any field type (a radio
    // group over string choices), so `for:` is not just `Bool`.
    Checked {
        name: "checked",
        owner: Formoxus,
        type: Flag,
        for: Text | Int | Float | Bool,
        on: Input(Checkbox | Radio),
        validated: false,
    },
    // Owned: resolved at the `Form` boundary from whether there are errors.
    // A global ARIA attribute, so valid on every control.
    AriaInvalid {
        name: "aria-invalid",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
}

/// `==` on `str` is a trait method, which const code cannot call.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
#[cfg(test)]
mod tests {
    use super::{Attr, AttrType, FieldType, Owner};
    use crate::{FieldControl, InputType};
    use googletest::prelude::*;

    const INPUT_TYPES: [InputType; 22] = [
        InputType::Text,
        InputType::Password,
        InputType::Hidden,
        InputType::Number,
        InputType::Email,
        InputType::Telephone,
        InputType::Url,
        InputType::Search,
        InputType::Color,
        InputType::Date,
        InputType::Time,
        InputType::DatetimeLocal,
        InputType::Month,
        InputType::Week,
        InputType::Checkbox,
        InputType::Radio,
        InputType::File,
        InputType::Range,
        InputType::Submit,
        InputType::Image,
        InputType::Reset,
        InputType::Button,
    ];

    const FIELD_TYPES: [FieldType; 4] = [
        FieldType::Text,
        FieldType::Int,
        FieldType::Float,
        FieldType::Bool,
    ];

    // ── `on:` against the spec ───────────────────────────────────────────
    //
    // The `<input>` types each attribute applies to, copied from the WHATWG
    // spec's non-normative summary table (pulled 2026-10-02, see
    // `.claude/memory/html_attributes_reference.md`). Written as `type=`
    // KEYWORDS, not variants, so these lists read exactly like the spec and
    // share nothing with the table they check.

    const SPEC_PLACEHOLDER: &[&str] = &[
        "text", "search", "tel", "url", "email", "password", "number",
    ];
    const SPEC_LENGTH_AND_PATTERN: &[&str] = &["text", "search", "tel", "url", "email", "password"];
    const SPEC_MIN_MAX: &[&str] = &[
        "date",
        "month",
        "week",
        "time",
        "datetime-local",
        "number",
        "range",
    ];
    const SPEC_REQUIRED: &[&str] = &[
        "text",
        "search",
        "tel",
        "url",
        "email",
        "password",
        "date",
        "month",
        "week",
        "time",
        "datetime-local",
        "number",
        "checkbox",
        "radio",
        "file",
    ];
    const SPEC_CHECKED: &[&str] = &["checkbox", "radio"];

    /// Every input type: valid exactly when the spec's list names it, except
    /// for the keywords in `excluded`, which the row deliberately leaves out.
    fn matches_spec_on_inputs(attr: Attr, spec: &[&str], excluded: &[&str]) {
        for t in INPUT_TYPES {
            let keyword = t.html_type();
            let expected = spec.contains(&keyword) && !excluded.contains(&keyword);
            expect_that!(
                attr.is_valid_on(FieldControl::Input(t)),
                eq(expected),
                "{attr:?} on input type={keyword}"
            );
        }
    }

    #[gtest]
    fn placeholder_follows_the_spec_on_inputs() {
        matches_spec_on_inputs(Attr::Placeholder, SPEC_PLACEHOLDER, &[]);
    }

    #[gtest]
    fn lengths_and_pattern_follow_the_spec_on_inputs() {
        for attr in [Attr::MinLength, Attr::MaxLength, Attr::Pattern] {
            matches_spec_on_inputs(attr, SPEC_LENGTH_AND_PATTERN, &[]);
        }
    }

    #[gtest]
    fn min_and_max_follow_the_spec_on_inputs() {
        for attr in [Attr::Min, Attr::Max] {
            matches_spec_on_inputs(attr, SPEC_MIN_MAX, &[]);
        }
    }

    /// The spec allows `required` on a checkbox, where it means "must be
    /// ticked". That sense is `RequiredTrue`, so the presence row leaves the
    /// checkbox out, and `RequiredTrue` is valid on the checkbox ONLY.
    #[gtest]
    fn the_two_requireds_split_the_specs_list_at_the_checkbox() {
        matches_spec_on_inputs(Attr::Required, SPEC_REQUIRED, &["checkbox"]);
        matches_spec_on_inputs(Attr::RequiredTrue, &["checkbox"], &[]);
    }

    #[gtest]
    fn checked_follows_the_spec_on_inputs() {
        matches_spec_on_inputs(Attr::Checked, SPEC_CHECKED, &[]);
    }

    /// `name`, `type`, `value` and `aria-invalid` apply to every input type.
    #[gtest]
    fn the_owned_input_attributes_are_valid_on_every_input_type() {
        for attr in [Attr::Name, Attr::Type, Attr::Value, Attr::AriaInvalid] {
            matches_spec_on_inputs(attr, &INPUT_TYPES.map(InputType::html_type), &[]);
        }
    }

    // ── `on:` for the other controls ─────────────────────────────────────

    /// The non-`<input>` controls, per the spec's element definitions: which
    /// of `<textarea>`, `<select>` and `<fieldset>` each attribute may appear
    /// on.
    #[gtest]
    fn each_attribute_is_valid_on_the_right_non_input_controls() {
        use FieldControl::{Fieldset, Select, Textarea};
        let cases: &[(Attr, &[FieldControl])] = &[
            (Attr::Placeholder, &[Textarea]),
            (Attr::MaxLength, &[Textarea]),
            (Attr::MinLength, &[Textarea]),
            (Attr::Pattern, &[]),
            (Attr::Min, &[]),
            (Attr::Max, &[]),
            (Attr::Required, &[Textarea, Select]),
            (Attr::RequiredTrue, &[]),
            (Attr::Name, &[Textarea, Select, Fieldset]),
            (Attr::Type, &[]),
            (Attr::Value, &[]),
            (Attr::Checked, &[]),
            (Attr::AriaInvalid, &[Textarea, Select, Fieldset]),
        ];
        expect_that!(cases.len(), eq(Attr::ALL.len()), "every row has a case");
        for (attr, valid) in cases {
            for control in [Textarea, Select, Fieldset] {
                expect_that!(
                    attr.is_valid_on(control),
                    eq(valid.contains(&control)),
                    "{attr:?} on {control:?}"
                );
            }
        }
    }

    // ── `for:` ───────────────────────────────────────────────────────────

    #[gtest]
    fn each_attribute_applies_to_the_right_field_types() {
        use FieldType::{Bool, Float, Int, Text};
        let any: &[FieldType] = &[Text, Int, Float, Bool];
        let cases: &[(Attr, &[FieldType])] = &[
            (Attr::Placeholder, &[Text, Int, Float]),
            (Attr::MaxLength, &[Text]),
            (Attr::MinLength, &[Text]),
            (Attr::Pattern, &[Text]),
            (Attr::Min, &[Int, Float]),
            (Attr::Max, &[Int, Float]),
            (Attr::Required, any),
            (Attr::RequiredTrue, &[Bool]),
            (Attr::Name, any),
            (Attr::Type, any),
            (Attr::Value, any),
            (Attr::Checked, any),
            (Attr::AriaInvalid, any),
        ];
        expect_that!(cases.len(), eq(Attr::ALL.len()), "every row has a case");
        for (attr, applies) in cases {
            for field_type in FIELD_TYPES {
                expect_that!(
                    attr.applies_to(field_type),
                    eq(applies.contains(&field_type)),
                    "{attr:?} for {field_type:?}"
                );
            }
        }
    }

    // ── The other columns ────────────────────────────────────────────────

    #[gtest]
    fn formoxus_owns_exactly_the_attributes_it_sets_itself() {
        let owned: Vec<Attr> = Attr::ALL
            .iter()
            .copied()
            .filter(|a| a.owner() == Owner::Formoxus)
            .collect();
        expect_that!(
            owned,
            unordered_elements_are![
                eq(&Attr::Required),
                eq(&Attr::Name),
                eq(&Attr::Type),
                eq(&Attr::Value),
                eq(&Attr::Checked),
                eq(&Attr::AriaInvalid),
            ]
        );
    }

    /// Exactly the attributes `validate` enforces today: the four constraint
    /// keys, the presence check and #6's must-be-true.
    #[gtest]
    fn formoxus_validates_exactly_todays_constraints() {
        let validated: Vec<Attr> = Attr::ALL
            .iter()
            .copied()
            .filter(|a| a.validated())
            .collect();
        expect_that!(
            validated,
            unordered_elements_are![
                eq(&Attr::MaxLength),
                eq(&Attr::MinLength),
                eq(&Attr::Pattern),
                eq(&Attr::Min),
                eq(&Attr::Max),
                eq(&Attr::Required),
                eq(&Attr::RequiredTrue),
            ]
        );
    }

    // ── `true_and_on(…)` ─────────────────────────────────────────────────

    /// Every control formoxus can be asked about.
    fn all_controls() -> Vec<FieldControl> {
        INPUT_TYPES
            .iter()
            .copied()
            .map(FieldControl::Input)
            .chain([
                FieldControl::Textarea,
                FieldControl::Select,
                FieldControl::Fieldset,
            ])
            .collect()
    }

    /// The extra controls each row validates on without emitting. Only a
    /// number rendered as text (formoxus's default for `Int`/`Float`) and a
    /// bool rendered as a `select` or radios need one.
    #[gtest]
    fn only_three_rows_are_also_validated_off_their_on_controls() {
        let cases: &[(Attr, &[FieldControl])] = &[
            (Attr::Min, &[FieldControl::Input(InputType::Text)]),
            (Attr::Max, &[FieldControl::Input(InputType::Text)]),
            (
                Attr::RequiredTrue,
                &[FieldControl::Select, FieldControl::Input(InputType::Radio)],
            ),
        ];
        for attr in Attr::ALL {
            let expected: &[FieldControl] = cases
                .iter()
                .find(|(a, _)| a == attr)
                .map_or(&[], |(_, controls)| controls);
            for control in all_controls() {
                expect_that!(
                    attr.is_also_validated_on(control),
                    eq(expected.contains(&control)),
                    "{attr:?} also on {control:?}"
                );
            }
        }
    }

    /// An extra control that is also in `on:` would be redundant, and would
    /// hide the fact that the attribute IS emitted there.
    #[gtest]
    fn no_extra_control_is_also_an_on_control() {
        for attr in Attr::ALL {
            for control in all_controls() {
                expect_that!(
                    attr.is_also_validated_on(control) && attr.is_valid_on(control),
                    eq(false),
                    "{attr:?} on {control:?}"
                );
            }
        }
    }

    // ── The acceptance rule ──────────────────────────────────────────────

    /// One case per row of `is_allowed`'s table, on the attributes that made
    /// the rule necessary.
    #[gtest]
    fn is_allowed_follows_the_rule() {
        use FieldControl::{Select, Textarea};
        let text_input = FieldControl::Input(InputType::Text);
        let number_input = FieldControl::Input(InputType::Number);
        let checkbox = FieldControl::Input(InputType::Checkbox);
        let cases = [
            // `for` does not match: refused.
            (Attr::Min, FieldType::Text, number_input, false),
            (Attr::Placeholder, FieldType::Bool, checkbox, false),
            (Attr::RequiredTrue, FieldType::Text, checkbox, false),
            // `on` matches: accepted (and emitted).
            (Attr::Min, FieldType::Int, number_input, true),
            (
                Attr::Pattern,
                FieldType::Text,
                FieldControl::Input(InputType::Email),
                true,
            ),
            (Attr::Placeholder, FieldType::Int, text_input, true),
            (Attr::RequiredTrue, FieldType::Bool, checkbox, true),
            // Only `true_and_on` names it: accepted, validated, not emitted.
            (Attr::Min, FieldType::Int, text_input, true),
            (Attr::Max, FieldType::Float, text_input, true),
            (Attr::RequiredTrue, FieldType::Bool, Select, true),
            // Neither: refused.
            (Attr::Pattern, FieldType::Text, Textarea, false),
            (Attr::Min, FieldType::Int, Textarea, false),
            (Attr::Placeholder, FieldType::Text, Select, false),
        ];
        for (attr, field_type, control, expected) in cases {
            expect_that!(
                attr.is_allowed(field_type, control),
                eq(expected),
                "{attr:?} for {field_type:?} on {control:?}"
            );
        }
    }

    /// The case `validated:` was redesigned for (2026-10-03): `min` on a
    /// number rendered as text is accepted, `pattern` on a textarea is not,
    /// although neither attribute can be emitted there.
    #[gtest]
    fn min_on_a_text_input_is_allowed_but_pattern_on_a_textarea_is_not() {
        let text_input = FieldControl::Input(InputType::Text);
        expect_that!(Attr::Min.is_valid_on(text_input), eq(false));
        expect_that!(Attr::Min.is_allowed(FieldType::Int, text_input), eq(true));
        expect_that!(Attr::Pattern.is_valid_on(FieldControl::Textarea), eq(false));
        expect_that!(
            Attr::Pattern.is_allowed(FieldType::Text, FieldControl::Textarea),
            eq(false)
        );
    }

    #[gtest]
    fn each_attribute_takes_the_right_type_of_value() {
        let cases = [
            (Attr::Placeholder, AttrType::String),
            (Attr::MaxLength, AttrType::Int),
            (Attr::MinLength, AttrType::Int),
            (Attr::Pattern, AttrType::Regex),
            (Attr::Min, AttrType::Bound),
            (Attr::Max, AttrType::Bound),
            (Attr::Required, AttrType::Flag),
            (Attr::RequiredTrue, AttrType::Flag),
            (Attr::Checked, AttrType::Flag),
        ];
        for (attr, attr_type) in cases {
            expect_that!(attr.attr_type(), eq(attr_type), "{attr:?}");
        }
    }

    // ── Names ────────────────────────────────────────────────────────────

    /// HTML names are unique except `required`, which two rows emit: the
    /// presence sense and #6's must-be-true.
    #[gtest]
    fn only_required_shares_an_html_name() {
        let mut names: Vec<&str> = Attr::ALL.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        let shared: Vec<&str> = names
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        expect_that!(shared, elements_are![eq(&"required")]);
    }

    #[gtest]
    fn every_html_name_finds_a_row_with_that_name() {
        for attr in Attr::ALL {
            let found = Attr::from_name(attr.name()).map(Attr::name);
            expect_that!(found, some(eq(attr.name())), "{attr:?}");
        }
        expect_that!(Attr::from_name("rows"), none());
    }

    /// With two rows sharing `required`, name lookup can only return one: the
    /// first, the presence row. Pinned so the change is visible when lookup
    /// moves to the `snake_case` variant name (design decision 3), which tells
    /// them apart.
    #[gtest]
    fn looking_up_required_by_name_finds_the_presence_row() {
        expect_that!(Attr::from_name("required"), some(eq(Attr::Required)));
    }
}
