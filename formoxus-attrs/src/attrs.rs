use crate::{Bound, FieldControl};

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
    List,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttrKey {
    Std(Attr),
    NonStd(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    String(String),
    Int(usize),
    Bound(Bound),
    Regex(&'static str),
    Flag,
    List(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    Text,
    Int,
    Float,
    Bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldTypeWithOptional {
    field_type: FieldType,
    is_optional: bool,
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

            pub const fn variant_name(&self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($variant), )+
                }
            }

            pub const fn from_variant_name(variant_name: &str) -> Option<Self> {
                let mut i = 0;
                while i < Self::ALL.len() {
                    let attr = Self::ALL[i];
                    if str_eq(variant_name, attr.variant_name()) {
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
    Accesskey {
        name: "accesskey",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Alpha {
        name: "alpha",
        owner: Author,
        type: Flag,
        for: Text,
        on: Input(Color),
        validated: false,
    },
    Autocapitalize {
        name: "autocapitalize",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Autocomplete {
        name: "autocomplete",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(Hidden | Text | Search | Telephone | Url | Email | Password | Date | Month | Week | Time | DatetimeLocal | Number | Range | Color) | Textarea | Select,
        validated: false,
    },
    Autocorrect {
        name: "autocorrect",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Autofocus {
        name: "autofocus",
        owner: Author,
        type: Flag,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
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
    // REPLACES formoxus's own classes (`fx-control` included); `ClassPlus` appends.
    // Only one of the two per field, which `form!` checks. Both emit HTML `class`,
    // resolved into the widget's ONE `class` value, never spread as a second one.
    Class {
        name: "class",
        owner: Merged,
        type: TokenList,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    ClassPlus {
        name: "class",
        owner: Merged,
        type: TokenList,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Colorspace {
        name: "colorspace",
        owner: Author,
        type: String,
        for: Text,
        on: Input(Color),
        validated: false,
    },
    Cols {
        name: "cols",
        owner: Author,
        type: Int,
        for: Text,
        on: Textarea,
        validated: false,
    },
    Dir {
        name: "dir",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Submits an EXTRA `name.dir` pair, which `Submission` ignores as an unknown path.
    Dirname {
        name: "dirname",
        owner: Author,
        type: String,
        for: Text,
        on: Input(Hidden | Text | Search | Telephone | Url | Email | Password | Submit) | Textarea,
        validated: false,
    },
    // A disabled control is NOT submitted, so the server sees it as empty, and a
    // non-optional field then fails "This field is required."
    Disabled {
        name: "disabled",
        owner: Author,
        type: Flag,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Enterkeyhint {
        name: "enterkeyhint",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned: it would associate the control with a DIFFERENT `<form>`.
    Form {
        name: "form",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned, reserved until issue #9 settles formoxus's id scheme (`<label for>`,
    // `aria-describedby`), so an author's id cannot collide with it.
    Id {
        name: "id",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Which keyboard a phone shows: the right fix for a number rendered as
    // `type="text"`, which formoxus does by default.
    Inputmode {
        name: "inputmode",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Lang {
        name: "lang",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // The id of a `<datalist>` the author renders; formoxus renders none itself.
    List {
        name: "list",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(Text | Search | Telephone | Url | Email | Date | Month | Week | Time | DatetimeLocal | Number | Range | Color),
        validated: false,
    },
    Max {
        name: "max",
        owner: Author,
        type: Bound,
        for: Int | Float,
        on: Input(Number | Range | Date | Month | Week | Time | DatetimeLocal),
        validated: true_and_on(Input(Text)),
    },
    MaxLength {
        name: "maxlength",
        owner: Author,
        type: Int,
        for: Text,
        on: Input(Text | Search | Telephone | Url | Email | Password) | Textarea,
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
    MinLength {
        name: "minlength",
        owner: Author,
        type: Int,
        for: Text,
        on: Input(Text | Search | Telephone | Url | Email | Password) | Textarea,
        validated: true,
    },
    // Owned: a leaf holds ONE string, so a multi-value control has nowhere to go.
    Multiple {
        name: "multiple",
        owner: Formoxus,
        type: Flag,
        for: Text | Int | Float | Bool,
        on: Input(Email | File) | Select,
        validated: false,
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
    Placeholder {
        name: "placeholder",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(Text | Search | Telephone | Url | Email | Password | Number) | Textarea,
        validated: false,
    },
    // The browser skips its own validation on a read-only control; formoxus still
    // validates the value it submits.
    Readonly {
        name: "readonly",
        owner: Author,
        type: Flag,
        for: Text | Int | Float,
        on: Input(Text | Search | Telephone | Url | Email | Password | Date | Month | Week | Time | DatetimeLocal | Number) | Textarea,
        validated: false,
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
    // Overrides the control's native role; ARIA, though not `aria-*`.
    Role {
        name: "role",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Rows {
        name: "rows",
        owner: Author,
        type: Int,
        for: Text,
        on: Textarea,
        validated: false,
    },
    Size {
        name: "size",
        owner: Author,
        type: Int,
        for: Text | Int | Float | Bool,
        on: Input(Text | Search | Telephone | Url | Email | Password) | Select,
        validated: false,
    },
    Spellcheck {
        name: "spellcheck",
        owner: Author,
        type: String,
        for: Text | Int | Float,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Like `Class`: `Style` replaces, `StylePlus` appends, only one per field.
    Style {
        name: "style",
        owner: Merged,
        type: Declarations,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    StylePlus {
        name: "style",
        owner: Merged,
        type: Declarations,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // `String`, not `Int`: `-1` is the common value, and `AttrType::Int` is unsigned.
    Tabindex {
        name: "tabindex",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Title {
        name: "title",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    Translate {
        name: "translate",
        owner: Author,
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
    Wrap {
        name: "wrap",
        owner: Author,
        type: String,
        for: Text,
        on: Textarea,
        validated: false,
    },
    Writingsuggestions {
        name: "writingsuggestions",
        owner: Author,
        type: String,
        for: Text,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaActivedescendant {
        name: "aria-activedescendant",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaAtomic {
        name: "aria-atomic",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaAutocomplete {
        name: "aria-autocomplete",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaBraillelabel {
        name: "aria-braillelabel",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaBrailleroledescription {
        name: "aria-brailleroledescription",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaBusy {
        name: "aria-busy",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned: formoxus holds the checkbox's value, so an author's `aria-checked`
    // could only contradict the real `checked`.
    AriaChecked {
        name: "aria-checked",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaColcount {
        name: "aria-colcount",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaColindex {
        name: "aria-colindex",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaColindextext {
        name: "aria-colindextext",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaColspan {
        name: "aria-colspan",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaControls {
        name: "aria-controls",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaCurrent {
        name: "aria-current",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned from the start: issue #9 will point it at the field's messages,
    // so taking it later would be a breaking change.
    AriaDescribedby {
        name: "aria-describedby",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaDescription {
        name: "aria-description",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaDetails {
        name: "aria-details",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaDisabled {
        name: "aria-disabled",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaErrormessage {
        name: "aria-errormessage",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaExpanded {
        name: "aria-expanded",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaFlowto {
        name: "aria-flowto",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaHaspopup {
        name: "aria-haspopup",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaHidden {
        name: "aria-hidden",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
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
    AriaKeyshortcuts {
        name: "aria-keyshortcuts",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaLabel {
        name: "aria-label",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaLabelledby {
        name: "aria-labelledby",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaLevel {
        name: "aria-level",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaLive {
        name: "aria-live",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaModal {
        name: "aria-modal",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaMultiline {
        name: "aria-multiline",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaMultiselectable {
        name: "aria-multiselectable",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaOrientation {
        name: "aria-orientation",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaOwns {
        name: "aria-owns",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaPlaceholder {
        name: "aria-placeholder",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaPosinset {
        name: "aria-posinset",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaPressed {
        name: "aria-pressed",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaReadonly {
        name: "aria-readonly",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRelevant {
        name: "aria-relevant",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    // Owned: formoxus decides requiredness and emits HTML `required`; an
    // author's `aria-required` could only contradict it.
    AriaRequired {
        name: "aria-required",
        owner: Formoxus,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRoledescription {
        name: "aria-roledescription",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRowcount {
        name: "aria-rowcount",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRowindex {
        name: "aria-rowindex",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRowindextext {
        name: "aria-rowindextext",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaRowspan {
        name: "aria-rowspan",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaSelected {
        name: "aria-selected",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaSetsize {
        name: "aria-setsize",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaSort {
        name: "aria-sort",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaValuemax {
        name: "aria-valuemax",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaValuemin {
        name: "aria-valuemin",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaValuenow {
        name: "aria-valuenow",
        owner: Author,
        type: String,
        for: Text | Int | Float | Bool,
        on: Input(_) | Textarea | Select | Fieldset,
        validated: false,
    },
    AriaValuetext {
        name: "aria-valuetext",
        owner: Author,
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
    // Where each attribute may be emitted, restated from the WHATWG spec
    // (pulled 2026-10-02, see `.claude/memory/html_attributes_reference.md`)
    // independently of the table: the `<input>` types as `type=` KEYWORDS,
    // exactly as the spec's summary table prints them, and the other controls
    // from each element's own attribute list.

    const TEXT_LIKE: &[&str] = &["text", "search", "tel", "url", "email", "password"];
    const DATE_LIKE: &[&str] = &["date", "month", "week", "time", "datetime-local"];

    /// What the spec says about one attribute: the input types it applies to,
    /// and which of `<textarea>`, `<select>` and `<fieldset>` it may appear on.
    struct Expected {
        inputs: Vec<&'static str>,
        others: Vec<FieldControl>,
    }

    fn every_input_keyword() -> Vec<&'static str> {
        INPUT_TYPES.iter().map(|t| t.html_type()).collect()
    }

    fn keywords(groups: &[&[&'static str]]) -> Vec<&'static str> {
        groups.concat()
    }

    /// The spec's answer for `attr`, or `None` if this test has no statement
    /// for it, which `every_row_has_a_spec_expectation` turns into a failure.
    fn expected(attr: Attr) -> Option<Expected> {
        use FieldControl::{Fieldset, Select, Textarea};
        let (inputs, others): (Vec<&'static str>, Vec<FieldControl>) = match attr {
            // Global attributes, and the form-control attributes every control
            // (fieldset included) has: everything.
            _ if attr.name().starts_with("aria-") => {
                (every_input_keyword(), vec![Textarea, Select, Fieldset])
            }
            Attr::Accesskey
            | Attr::Autocapitalize
            | Attr::Autocorrect
            | Attr::Autofocus
            | Attr::Class
            | Attr::ClassPlus
            | Attr::Dir
            | Attr::Disabled
            | Attr::Enterkeyhint
            | Attr::Form
            | Attr::Id
            | Attr::Inputmode
            | Attr::Lang
            | Attr::Name
            | Attr::Role
            | Attr::Spellcheck
            | Attr::Style
            | Attr::StylePlus
            | Attr::Tabindex
            | Attr::Title
            | Attr::Translate
            | Attr::Writingsuggestions => (every_input_keyword(), vec![Textarea, Select, Fieldset]),
            Attr::Type | Attr::Value => (every_input_keyword(), vec![]),
            Attr::Placeholder => (keywords(&[TEXT_LIKE, &["number"]]), vec![Textarea]),
            Attr::MinLength | Attr::MaxLength => (TEXT_LIKE.to_vec(), vec![Textarea]),
            // No `<textarea>`: HTML has no `pattern` on one.
            Attr::Pattern => (TEXT_LIKE.to_vec(), vec![]),
            Attr::Min | Attr::Max => (keywords(&[DATE_LIKE, &["number", "range"]]), vec![]),
            // The spec also lists `checkbox` for `required`. That sense is
            // `RequiredTrue`, so the presence row leaves it out on purpose.
            Attr::Required => (
                keywords(&[TEXT_LIKE, DATE_LIKE, &["number", "radio", "file"]]),
                vec![Textarea, Select],
            ),
            Attr::RequiredTrue => (vec!["checkbox"], vec![]),
            Attr::Checked => (vec!["checkbox", "radio"], vec![]),
            Attr::Autocomplete => (
                keywords(&[
                    &["hidden"],
                    TEXT_LIKE,
                    DATE_LIKE,
                    &["number", "range", "color"],
                ]),
                vec![Textarea, Select],
            ),
            Attr::Dirname => (
                keywords(&[&["hidden"], TEXT_LIKE, &["submit"]]),
                vec![Textarea],
            ),
            Attr::List => (
                keywords(&[
                    &["text", "search", "tel", "url", "email"],
                    DATE_LIKE,
                    &["number", "range", "color"],
                ]),
                vec![],
            ),
            Attr::Readonly => (
                keywords(&[TEXT_LIKE, DATE_LIKE, &["number"]]),
                vec![Textarea],
            ),
            Attr::Size => (TEXT_LIKE.to_vec(), vec![Select]),
            Attr::Multiple => (vec!["email", "file"], vec![Select]),
            Attr::Alpha | Attr::Colorspace => (vec!["color"], vec![]),
            Attr::Cols | Attr::Rows | Attr::Wrap => (vec![], vec![Textarea]),
            _ => return None,
        };
        Some(Expected { inputs, others })
    }

    /// A new row cannot skip the spec check: it needs an `expected` arm.
    #[gtest]
    fn every_row_has_a_spec_expectation() {
        for &attr in Attr::ALL {
            expect_that!(expected(attr).is_some(), eq(true), "{attr:?}");
        }
    }

    /// Every row, on all 22 input types and on the three other controls.
    #[gtest]
    fn every_row_is_valid_exactly_where_the_spec_says() {
        for &attr in Attr::ALL {
            let Some(Expected { inputs, others }) = expected(attr) else {
                continue;
            };
            for t in INPUT_TYPES {
                let keyword = t.html_type();
                expect_that!(
                    attr.is_valid_on(FieldControl::Input(t)),
                    eq(inputs.contains(&keyword)),
                    "{attr:?} on input type={keyword}"
                );
            }
            for control in [
                FieldControl::Textarea,
                FieldControl::Select,
                FieldControl::Fieldset,
            ] {
                expect_that!(
                    attr.is_valid_on(control),
                    eq(others.contains(&control)),
                    "{attr:?} on {control:?}"
                );
            }
        }
    }

    // ── `for:` ───────────────────────────────────────────────────────────

    /// Which field types each attribute applies to, by group. Anything not in
    /// a narrower group applies to every field type.
    #[gtest]
    fn each_attribute_applies_to_the_right_field_types() {
        use FieldType::{Bool, Float, Int, Text};
        let text_only = [
            Attr::MaxLength,
            Attr::MinLength,
            Attr::Pattern,
            Attr::Alpha,
            Attr::Colorspace,
            Attr::Cols,
            Attr::Rows,
            Attr::Wrap,
            Attr::Dirname,
            Attr::Writingsuggestions,
        ];
        // Attributes about typing text, which a number rendered as text shares.
        let text_and_numbers = [
            Attr::Placeholder,
            Attr::List,
            Attr::Readonly,
            Attr::Inputmode,
            Attr::Enterkeyhint,
            Attr::Autocapitalize,
            Attr::Autocorrect,
            Attr::Spellcheck,
        ];
        let numbers = [Attr::Min, Attr::Max];
        let bools = [Attr::RequiredTrue];
        for &attr in Attr::ALL {
            let applies: &[FieldType] = if text_only.contains(&attr) {
                &[Text]
            } else if text_and_numbers.contains(&attr) {
                &[Text, Int, Float]
            } else if numbers.contains(&attr) {
                &[Int, Float]
            } else if bools.contains(&attr) {
                &[Bool]
            } else {
                &[Text, Int, Float, Bool]
            };
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
                eq(&Attr::AriaChecked),
                eq(&Attr::AriaDescribedby),
                eq(&Attr::AriaInvalid),
                eq(&Attr::AriaRequired),
                eq(&Attr::Checked),
                eq(&Attr::Form),
                eq(&Attr::Id),
                eq(&Attr::Multiple),
                eq(&Attr::Name),
                eq(&Attr::Required),
                eq(&Attr::Type),
                eq(&Attr::Value),
            ]
        );
    }

    /// `class` and `style` are the attributes formoxus sets AND an author may
    /// change: replaced by `Class`/`Style`, appended to by the `Plus` rows.
    #[gtest]
    fn only_class_and_style_are_merged() {
        let merged: Vec<Attr> = Attr::ALL
            .iter()
            .copied()
            .filter(|a| a.owner() == Owner::Merged)
            .collect();
        expect_that!(
            merged,
            unordered_elements_are![
                eq(&Attr::Class),
                eq(&Attr::ClassPlus),
                eq(&Attr::Style),
                eq(&Attr::StylePlus),
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
            (Attr::Autofocus, AttrType::Flag),
            (Attr::Cols, AttrType::Int),
            (Attr::Tabindex, AttrType::String),
            (Attr::Class, AttrType::TokenList),
            (Attr::ClassPlus, AttrType::TokenList),
            (Attr::Style, AttrType::Declarations),
            (Attr::StylePlus, AttrType::Declarations),
            (Attr::AriaLabel, AttrType::String),
        ];
        for (attr, attr_type) in cases {
            expect_that!(attr.attr_type(), eq(attr_type), "{attr:?}");
        }
    }

    // ── Names ────────────────────────────────────────────────────────────

    /// HTML names are unique except where two rows emit the same attribute
    /// with different meanings: `required` (presence and must-be-true), and
    /// `class`/`style` (replace and append).
    #[gtest]
    fn only_required_class_and_style_share_an_html_name() {
        let mut names: Vec<&str> = Attr::ALL.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        let shared: Vec<&str> = names
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        expect_that!(
            shared,
            elements_are![eq(&"class"), eq(&"required"), eq(&"style")]
        );
    }

    #[gtest]
    fn every_html_name_finds_a_row_with_that_name() {
        for attr in Attr::ALL {
            let found = Attr::from_name(attr.name()).map(Attr::name);
            expect_that!(found, some(eq(attr.name())), "{attr:?}");
        }
        // A name only a quoted, pass-through key would carry.
        expect_that!(Attr::from_name("hx-get"), none());
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
