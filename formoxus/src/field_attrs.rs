use dioxus::core::Attribute;
use formoxus_spec::{
    Attr, AttrKey, AttrValue, Bound, BuiltinWidget, FieldType, RenderedInputType, Step,
};
use indexmap::IndexMap;

use crate::{
    classes::Slot,
    widgets::{WidgetType, errors::errors_id},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldAttrs(IndexMap<AttrKey, AttrValue>);

impl<const N: usize> From<[(AttrKey, AttrValue); N]> for FieldAttrs {
    fn from(arr: [(AttrKey, AttrValue); N]) -> Self {
        Self(IndexMap::from(arr))
    }
}

impl FieldAttrs {
    pub fn get(&self, attr: Attr) -> Option<&AttrValue> {
        self.0.get(&AttrKey::Std(attr))
    }

    pub fn contains(&self, attr: Attr) -> bool {
        self.0.contains_key(&AttrKey::Std(attr))
    }

    /// A copy without `attr`, for a widget that puts it somewhere other than
    /// where its spread lands (`RadioGroup`'s `required`). `shift_remove`, so
    /// the rest keep their order.
    pub fn without(&self, attr: Attr) -> FieldAttrs {
        let mut map = self.0.clone();
        map.shift_remove(&AttrKey::Std(attr));
        FieldAttrs(map)
    }

    /// A copy with `attr` set to `value` unless the author already set it, for
    /// a value formoxus supplies but an author may replace (`VariantSet`'s
    /// `aria-labelledby`). The author's entry wins and keeps its position, so
    /// the attribute is never emitted twice.
    pub fn with_default(&self, attr: Attr, value: AttrValue) -> FieldAttrs {
        let mut map = self.0.clone();
        map.entry(AttrKey::Std(attr)).or_insert(value);
        FieldAttrs(map)
    }

    /// This map's entries, then `later`'s, in their orders. For the field's
    /// own attributes followed by the author's. A key in both takes `later`'s
    /// value and keeps its first position, though the two never share a
    /// validated key, since `with_attrs` refuses one.
    pub fn then(&self, later: &FieldAttrs) -> FieldAttrs {
        let mut map = self.0.clone();
        map.extend(later.0.iter().map(|(k, v)| (*k, v.clone())));
        FieldAttrs(map)
    }

    /// The first attribute here that formoxus validates, if any: what
    /// `FormSpec::with_attrs` refuses, because a field's settings validate
    /// now and attributes only render.
    pub(crate) fn first_validated(&self) -> Option<Attr> {
        self.0.keys().find_map(|key| match key {
            AttrKey::Std(attr) if attr.validated() => Some(*attr),
            _ => None,
        })
    }

    /// Every entry but the classes, as Dioxus attributes. A bound is emitted as
    /// written, so call [`FieldAttrs::resolve_bounds`] first.
    fn to_attributes(&self) -> Vec<Attribute> {
        let mut out = Vec::new();
        for (attr_key, attr_value) in &self.0 {
            if matches!(attr_key, AttrKey::Std(Attr::Class | Attr::ClassPlus)) {
                continue;
            }
            let name = match attr_key {
                AttrKey::Std(attr) => attr.name(),
                AttrKey::NonStd(name) => name,
            };
            let attribute = match attr_value {
                AttrValue::Int(n) => Attribute::new(name, *n, None, false),
                AttrValue::Regex(patt) => Attribute::new(name, *patt, None, false),
                AttrValue::Flag => Attribute::new(name, true, None, false),
                // Both emitted as written. A bound is already in the field's
                // kind (`resolve_bounds`). A step is a pass-through, never
                // converted: `form!` refuses a fractional one on an int field
                // at compile time; a hand-built spec's is emitted anyway, since
                // it validates nothing and so has nothing to panic over.
                AttrValue::Bound(Bound::Int(n)) | AttrValue::Step(Step::By(Bound::Int(n))) => {
                    Attribute::new(name, *n, None, false)
                }
                AttrValue::Bound(Bound::Float(x)) | AttrValue::Step(Step::By(Bound::Float(x))) => {
                    Attribute::new(name, *x, None, false)
                }
                AttrValue::Step(Step::Any) => Attribute::new(name, "any", None, false),
                AttrValue::String(s) => Attribute::new(name, s.clone(), None, false),
                AttrValue::List(vs) => {
                    assert!(matches!(
                        attr_key,
                        AttrKey::Std(Attr::Style | Attr::StylePlus)
                    ));
                    Attribute::new(name, vs.join("; "), None, false)
                }
            };
            out.push(attribute);
        }
        out
    }

    pub fn merge_with_attrs(&self, extras: Vec<Attribute>) -> Vec<Attribute> {
        let mut mapped_attrs: IndexMap<&'static str, Attribute> = self
            .to_attributes()
            .into_iter()
            .map(|a| (a.name, a))
            .collect();

        for attr in extras {
            mapped_attrs.insert(attr.name, attr);
        }

        mapped_attrs.into_values().collect()
    }

    pub fn class(&self, slot: Option<&Slot>, invalid: bool) -> Option<String> {
        let empty = Slot::new("");
        let slot = slot.unwrap_or(&empty);
        let base = if let Some(AttrValue::List(classes)) = self.get(Attr::Class) {
            classes.join(" ")
        } else if let Some(AttrValue::List(classes)) = self.get(Attr::ClassPlus) {
            [&*slot.base, &classes.join(" ")]
                .into_iter()
                .filter(|c| !c.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            return slot.value(invalid);
        };
        Slot {
            base: base.into(),
            invalid: slot.invalid.clone(),
        }
        .value(invalid)
    }

    /// The map a widget actually gets: formoxus's own attributes, which only
    /// exist at render, FIRST, then the field's.
    ///
    /// - `Required` (a flag) when `required`, except on a `Checkbox`.
    ///   formoxus's `required` means PRESENCE, and an unticked box is a
    ///   complete `false`; HTML `required` on a checkbox means "must be
    ///   ticked", which is `required_true`'s job.
    /// - `AriaInvalid` = `"true"` when `invalid`, and otherwise ABSENT, never
    ///   `"false"`: per ARIA, `"false"` asserts "checked, and passed", so an
    ///   untouched form would claim every field had been validated, a screen
    ///   reader would say so, and a stylesheet painting a validated-and-clean
    ///   state would paint it. Telling those apart is issue #7.
    ///
    /// First, so `required` keeps the position the suite's exact-markup tests
    /// expect.
    pub fn with_owned_attrs(
        field_id: &str,
        widget: &WidgetType,
        required: bool,
        invalid: bool,
        attrs: &FieldAttrs,
    ) -> FieldAttrs {
        let mut map: IndexMap<AttrKey, AttrValue> = IndexMap::default();
        if required
            && !matches!(
                widget,
                WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Checkbox))
            )
        {
            map.insert(AttrKey::Std(Attr::Required), AttrValue::Flag);
        }
        if invalid {
            map.insert(
                AttrKey::Std(Attr::AriaInvalid),
                AttrValue::String("true".to_string()),
            );
            map.insert(
                AttrKey::Std(Attr::AriaDescribedby),
                AttrValue::String(errors_id(field_id)),
            );
        }
        map.extend(attrs.0.clone());
        FieldAttrs(map)
    }

    /// This map with every `Bound` in the field's kind: `Bound::Int` on an int
    /// field, `Bound::Float` on any other. `FieldNode` calls it once, before a
    /// widget sees the map, so `to_attributes` emits a bound as written and no
    /// widget, built-in or custom, needs the field's type. Panics on a
    /// fractional bound on an int field, as [`int_bound`] does.
    pub fn resolve_bounds(mut self, field_type: FieldType) -> Self {
        for (key, value) in &mut self.0 {
            if let AttrValue::Bound(bound) = value {
                let name = match key {
                    AttrKey::Std(attr) => attr.name(),
                    AttrKey::NonStd(name) => name,
                };
                *bound = match field_type {
                    FieldType::Int => Bound::Int(int_bound(*bound, name)),
                    _ => Bound::Float(float_bound(*bound)),
                };
            }
        }
        self
    }
}

/// A bound on a float field, as an `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a bound above 2^53 does not survive this widening; form! rejects \
    one at compile time (`field_kind::bound_is_exact`), so only a hand-built \
    spec can still reach here with one"
)]
pub(crate) fn float_bound(bound: Bound) -> f64 {
    match bound {
        Bound::Int(n) => n as f64,
        Bound::Float(x) => x,
    }
}

/// A bound on an integer field, as the `i128` it is compared as.
///
/// A whole float bound is accepted, because `min: 2.0` means what it says
/// and `form!` cannot reject it: it cannot tell `2.0` from `2` when the
/// bound is a named const. A fractional one still panics. `form!` rejects
/// that at compile time (`field_kind::bound_is_whole`), so only a hand-built
/// spec can reach the panic.
pub(crate) fn int_bound(bound: Bound, which: &str) -> i128 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::core::AttributeValue;
    use googletest::prelude::*;

    // ── Helpers ──────────────────────────────────────────────────────────
    //
    // What `to_attributes` emits depends on the attributes AND the field's
    // type (a bound is emitted as the field's kind), so these build the pair.

    /// Table attributes only; the author-attribute tests build `NonStd` keys
    /// themselves.
    fn all(entries: &[(Attr, AttrValue)]) -> FieldAttrs {
        FieldAttrs(
            entries
                .iter()
                .map(|(attr, value)| (AttrKey::Std(*attr), value.clone()))
                .collect(),
        )
    }

    fn holding(field_type: FieldType, attrs: FieldAttrs) -> (FieldAttrs, FieldType) {
        (attrs, field_type)
    }

    fn text(
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<&'static str>,
    ) -> (FieldAttrs, FieldType) {
        let mut entries = Vec::new();
        entries.extend(min_length.map(|n| (Attr::MinLength, AttrValue::Int(n))));
        entries.extend(max_length.map(|n| (Attr::MaxLength, AttrValue::Int(n))));
        entries.extend(pattern.map(|p| (Attr::Pattern, AttrValue::Regex(p))));
        holding(FieldType::Text, all(&entries))
    }

    fn bounds(min: Option<Bound>, max: Option<Bound>) -> FieldAttrs {
        let mut entries = Vec::new();
        entries.extend(min.map(|b| (Attr::Min, AttrValue::Bound(b))));
        entries.extend(max.map(|b| (Attr::Max, AttrValue::Bound(b))));
        all(&entries)
    }

    fn int(min: Option<i128>, max: Option<i128>) -> (FieldAttrs, FieldType) {
        holding(
            FieldType::Int,
            bounds(min.map(Bound::Int), max.map(Bound::Int)),
        )
    }

    fn float(min: Option<f64>, max: Option<f64>) -> (FieldAttrs, FieldType) {
        holding(
            FieldType::Float,
            bounds(min.map(Bound::Float), max.map(Bound::Float)),
        )
    }

    fn boolean(required_true: bool) -> (FieldAttrs, FieldType) {
        if required_true {
            holding(
                FieldType::Bool,
                all(&[(Attr::RequiredTrue, AttrValue::Flag)]),
            )
        } else {
            holding(FieldType::Bool, FieldAttrs::default())
        }
    }

    /// What a widget gets: the bounds resolved, then emitted.
    fn rendered((attrs, field_type): &(FieldAttrs, FieldType)) -> Vec<Attribute> {
        attrs.clone().resolve_bounds(*field_type).to_attributes()
    }

    fn names(pair: &(FieldAttrs, FieldType)) -> Vec<&'static str> {
        rendered(pair).iter().map(|a| a.name).collect()
    }

    /// The value of the one emitted attribute called `name`.
    fn emitted(pair: &(FieldAttrs, FieldType), name: &str) -> AttributeValue {
        let attrs = rendered(pair);
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
    // same `min_length` that `FieldNode`'s check enforces in Rust also has to reach the
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
        let field = holding(
            FieldType::Text,
            all(&[
                (Attr::MinLength, AttrValue::Int(3)),
                (Attr::Pattern, AttrValue::Regex(r"\d{5}")),
                (Attr::MaxLength, AttrValue::Int(10)),
            ]),
        );
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
        let field = holding(FieldType::Text, FieldAttrs(map));
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
        let int_bound_on_float = holding(FieldType::Float, bounds(Some(Bound::Int(0)), None));
        expect_that!(
            emitted(&int_bound_on_float, "min"),
            eq(&AttributeValue::Float(0.0))
        );
        let whole_float_on_int = holding(FieldType::Int, bounds(Some(Bound::Float(2.0)), None));
        expect_that!(
            emitted(&whole_float_on_int, "min"),
            eq(&AttributeValue::Int(2))
        );
    }

    /// `form!` refuses this at compile time, so only a hand-built spec reaches
    /// it, and `FieldNode` resolves before any widget renders.
    #[gtest]
    #[should_panic(expected = "fractional min 2.5")]
    fn resolving_a_fractional_bound_on_an_int_field_panics() {
        let _ = bounds(Some(Bound::Float(2.5)), None).resolve_bounds(FieldType::Int);
    }

    /// Only bounds change: every other entry, and the order, come through.
    #[gtest]
    fn resolving_leaves_everything_but_bounds_alone() {
        let attrs = all(&[
            (Attr::Pattern, AttrValue::Regex("[a-z]+")),
            (Attr::Min, AttrValue::Bound(Bound::Float(2.0))),
            (Attr::Required, AttrValue::Flag),
        ]);
        expect_that!(
            attrs.resolve_bounds(FieldType::Int),
            eq(&all(&[
                (Attr::Pattern, AttrValue::Regex("[a-z]+")),
                (Attr::Min, AttrValue::Bound(Bound::Int(2))),
                (Attr::Required, AttrValue::Flag),
            ]))
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
        .to_attributes()
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
        let field = holding(
            FieldType::Text,
            FieldAttrs(IndexMap::from([
                (
                    AttrKey::NonStd("hx-get"),
                    AttrValue::String("/x".to_string()),
                ),
                (AttrKey::Std(Attr::MaxLength), AttrValue::Int(5)),
                (
                    AttrKey::NonStd("data-id"),
                    AttrValue::String("7".to_string()),
                ),
            ])),
        );
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

    // ── Owned attributes, added at render ────────────────────────────────
    //
    // `required`, `aria-invalid` and `aria-describedby` depend on
    // `ctx.required` and the field's errors, which only exist at render, so
    // `form!`'s map never holds them. `with_owned_attrs` builds the map a
    // widget actually gets.

    /// A field's element id, as `NodeContext::id` builds it. `.into()` at each
    /// call, so these read the same whether the parameter is `String` or `&str`.
    const ID: &str = "aB3xY9kq-name";

    fn keys(attrs: &FieldAttrs) -> Vec<AttrKey> {
        attrs.0.keys().copied().collect()
    }

    fn just_max_length() -> FieldAttrs {
        all(&[(Attr::MaxLength, AttrValue::Int(10))])
    }

    /// **Owned attributes come FIRST**, `required`, then `aria-invalid` and
    /// `aria-describedby`, then the field's own in their order. First is what
    /// keeps `required` where the suite's exact-markup tests already expect it.
    #[gtest]
    fn owned_attributes_come_first_then_the_fields_own() {
        let attrs = FieldAttrs::with_owned_attrs(
            ID,
            &WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Text)),
            true,
            true,
            &just_max_length(),
        );
        expect_that!(
            keys(&attrs),
            elements_are![
                eq(&AttrKey::Std(Attr::Required)),
                eq(&AttrKey::Std(Attr::AriaInvalid)),
                eq(&AttrKey::Std(Attr::AriaDescribedby)),
                eq(&AttrKey::Std(Attr::MaxLength)),
            ]
        );
        expect_that!(attrs.get(Attr::Required), some(eq(&AttrValue::Flag)));
        expect_that!(
            attrs.get(Attr::AriaInvalid),
            some(eq(&AttrValue::String("true".to_string())))
        );
    }

    /// **`aria-describedby` names the error list's id**, `{field id}-errors`,
    /// which is what `FieldErrors` puts on its `<ul>`. Spelled out as a literal
    /// rather than through `errors_id`, so a change to the format has to change
    /// this test too, on purpose.
    #[gtest]
    fn aria_describedby_names_the_error_list() {
        let attrs = FieldAttrs::with_owned_attrs(
            ID,
            &WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Text)),
            false,
            true,
            &FieldAttrs::default(),
        );
        expect_that!(
            attrs.get(Attr::AriaDescribedby),
            some(eq(&AttrValue::String("aB3xY9kq-name-errors".to_string())))
        );
    }

    /// Each is added on its own condition, not both or neither. `aria-invalid`
    /// and `aria-describedby` share one: there is an error list to point at
    /// exactly when the field is invalid, and pointing at a list that is not
    /// rendered is itself a defect.
    #[gtest]
    fn required_and_invalid_are_independent() {
        let input = WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Text));
        let only_required =
            FieldAttrs::with_owned_attrs(ID, &input, true, false, &just_max_length());
        expect_that!(
            keys(&only_required),
            elements_are![
                eq(&AttrKey::Std(Attr::Required)),
                eq(&AttrKey::Std(Attr::MaxLength)),
            ]
        );
        let only_invalid =
            FieldAttrs::with_owned_attrs(ID, &input, false, true, &just_max_length());
        expect_that!(
            keys(&only_invalid),
            elements_are![
                eq(&AttrKey::Std(Attr::AriaInvalid)),
                eq(&AttrKey::Std(Attr::AriaDescribedby)),
                eq(&AttrKey::Std(Attr::MaxLength)),
            ]
        );
    }

    /// Neither applying leaves the field's own attributes exactly as they were.
    #[gtest]
    fn without_either_the_fields_attributes_pass_through_unchanged() {
        let attrs = FieldAttrs::with_owned_attrs(
            ID,
            &WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Text)),
            false,
            false,
            &just_max_length(),
        );
        expect_that!(attrs, eq(&just_max_length()));
    }

    /// `without` drops exactly the one attribute and keeps the rest in order,
    /// which is what lets `RadioGroup` move `required` off its fieldset without
    /// reordering what is left there.
    #[gtest]
    fn without_drops_one_attribute_and_keeps_the_order() {
        let attrs = all(&[
            (Attr::MaxLength, AttrValue::Int(10)),
            (Attr::Required, AttrValue::Flag),
            (Attr::MinLength, AttrValue::Int(2)),
        ]);
        expect_that!(
            keys(&attrs.without(Attr::Required)),
            elements_are![
                eq(&AttrKey::Std(Attr::MaxLength)),
                eq(&AttrKey::Std(Attr::MinLength)),
            ]
        );
    }

    /// `with_default` adds a value the author has not set, at the end, and
    /// leaves one the author HAS set alone, value and position both.
    #[gtest]
    fn with_default_never_overrides_the_author() {
        let added = just_max_length()
            .with_default(Attr::AriaLabelledby, AttrValue::String("x-legend".into()));
        expect_that!(
            keys(&added),
            elements_are![
                eq(&AttrKey::Std(Attr::MaxLength)),
                eq(&AttrKey::Std(Attr::AriaLabelledby)),
            ]
        );
        let authored = all(&[
            (Attr::AriaLabelledby, AttrValue::String("mine".into())),
            (Attr::MaxLength, AttrValue::Int(10)),
        ]);
        expect_that!(
            authored.with_default(Attr::AriaLabelledby, AttrValue::String("x-legend".into())),
            eq(&authored)
        );
    }

    /// **A checkbox never gets `required`.** On a checkbox, HTML `required`
    /// means "must be ticked", which is `required_true`'s job; formoxus's
    /// `required` means PRESENCE, and an unticked box is still a present
    /// `false`. `aria-invalid` and `aria-describedby` are unaffected.
    #[gtest]
    fn a_checkbox_never_gets_required() {
        let attrs = FieldAttrs::with_owned_attrs(
            ID,
            &WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Checkbox)),
            true,
            true,
            &FieldAttrs::default(),
        );
        expect_that!(
            keys(&attrs),
            elements_are![
                eq(&AttrKey::Std(Attr::AriaInvalid)),
                eq(&AttrKey::Std(Attr::AriaDescribedby)),
            ]
        );
    }
}
