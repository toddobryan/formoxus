use formoxus_spec::{BuiltinWidget, FixedKey, SlotKey};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, Result, Token, braced, parse::ParseStream, token};

use super::button::unknown_button_type;
use super::field::class_list;
use super::suggest::edit_distance;
use super::widget::{builtin_tokens, unknown_widget};
use super::{Entry, kw};

/// One `selector: value` line of `classes_plus: { … }`, resolved and checked
/// while parsing, so expanding it cannot fail. Becomes one `ClassEdit`.
#[derive(Debug)]
pub(crate) struct ClassEntry {
    pub(crate) key: SlotKey,
    /// What happens to the slot's base; `None` leaves it as it was.
    pub(crate) base: Option<Change>,
    /// What happens to the slot's invalid class; `None` leaves it as it was.
    pub(crate) invalid: Option<Change>,
}

impl ClassEntry {
    /// The `.with_class_edit(…)` call this entry becomes:
    /// `ClassEdit::part(widget, "part")` or `ClassEdit::fixed(key)`, then one
    /// `replace_*`/`append_*` per piece the entry changes.
    pub(crate) fn tokens(&self) -> TokenStream2 {
        let edit = match self.key {
            SlotKey::Part(widget, part) => {
                let widget = builtin_tokens(widget);
                quote! { ::formoxus::classes::ClassEdit::part(#widget, #part) }
            }
            SlotKey::Fixed(key) => {
                let key = fixed_key_tokens(key);
                quote! { ::formoxus::classes::ClassEdit::fixed(#key) }
            }
        };
        let base = self.base.as_ref().map(|c| c.tokens("base"));
        let invalid = self.invalid.as_ref().map(|c| c.tokens("invalid"));
        quote! { .with_class_edit(#edit #base #invalid) }
    }
}

impl Change {
    /// `.replace_base("…")`, `.append_invalid("…")`, and so on: the
    /// `ClassEdit` method for this change to `piece`.
    fn tokens(&self, piece: &str) -> TokenStream2 {
        let (verb, classes) = match self {
            Change::Replace(classes) => ("replace", classes),
            Change::Append(classes) => ("append", classes),
        };
        let method = format_ident!("{verb}_{piece}");
        quote! { .#method(#classes) }
    }
}

/// `key`, as `::formoxus::classes::FixedKey::…`. Every variant but `Button`
/// is a unit variant, written out by its `Debug` name as `BuiltinWidget`'s
/// are; `Button(t)` prints as `Button(Submit)`, which is not an ident, so it
/// is spelled out.
fn fixed_key_tokens(key: FixedKey) -> TokenStream2 {
    match key {
        FixedKey::Button(ty) => {
            let ty = format_ident!("{ty:?}");
            quote! {
                ::formoxus::classes::FixedKey::Button(::formoxus::buttons::ButtonType::#ty)
            }
        }
        other => {
            let variant = format_ident!("{other:?}");
            quote! { ::formoxus::classes::FixedKey::#variant }
        }
    }
}

/// `classes_plus: { selector: value, … }`: changes to the classes the form
/// inherits, one slot per entry, in source order.
///
/// A selector given twice is an error rather than a second edit: the second
/// would silently undo the first, or half of it, and merging the two into one
/// entry says the same thing readably.
pub(crate) fn parse_classes_plus(input: ParseStream<'_>) -> Result<Entry> {
    input.parse::<kw::classes_plus>()?;
    input.parse::<Token![:]>()?;
    if !input.peek(token::Brace) {
        return Err(input.error("expected a `classes_plus` block surrounded by braces"));
    }
    let body;
    let braces = braced!(body in input);
    let mut entries: Vec<ClassEntry> = Vec::new();
    while !body.is_empty() {
        let (key, written) = parse_slot_key(&body)?;
        if entries.iter().any(|e| e.key == key) {
            return Err(syn::Error::new_spanned(
                written,
                format!(
                    "`{}` is given twice — merge the two entries into one",
                    selector(key)
                ),
            ));
        }
        body.parse::<Token![:]>()?;
        let (base, invalid) = parse_class_value(&body)?;
        entries.push(ClassEntry { key, base, invalid });
        if !body.is_empty() {
            body.parse::<Token![,]>()?;
        }
    }
    if entries.is_empty() {
        return Err(syn::Error::new(
            braces.span.join(),
            "empty `classes_plus` block — drop it",
        ));
    }
    Ok(Entry::ClassesPlus(entries))
}

/// `key` as an author writes it: `select.select`, `button.submit`,
/// `button_row`.
fn selector(key: SlotKey) -> String {
    match key {
        SlotKey::Part(widget, part) => format!("{}.{part}", widget.form_name()),
        SlotKey::Fixed(key) => key.form_name().to_string(),
    }
}

/// A selector: which slot an entry edits, as a `SlotKey`. One of
///
/// - `widget.part` (`select.select`, `text.input`): a part of a built-in
///   widget, by its `form!` name and its `parts()` name;
/// - `button.type` (`button.submit`): one button type's button;
/// - `fixed_key` (`button_row`, `required_marker`): an element no widget
///   renders, by its `FixedKey` name.
///
/// Every name is checked here, with the caret on the token at fault, so a
/// selector that parses names a slot that exists. Also returns the tokens as
/// written, so a later error about the whole selector can underline all of it.
fn parse_slot_key(input: ParseStream<'_>) -> Result<(SlotKey, TokenStream2)> {
    let first: Ident = input.parse()?;
    if !input.peek(Token![.]) {
        return Ok((fixed_key(&first)?, quote! { #first }));
    }
    let dot: Token![.] = input.parse()?;
    let second: Ident = input.parse()?;
    let key = if first == "button" {
        button_key(&second)?
    } else {
        part_key(&first, &second)?
    };
    Ok((key, quote! { #first #dot #second }))
}

/// `button.<type>`, with `type` checked as `buttons:` checks it.
fn button_key(ty: &Ident) -> Result<SlotKey> {
    FixedKey::from_form_name(&format!("button.{ty}"))
        .map(SlotKey::Fixed)
        .ok_or_else(|| syn::Error::new(ty.span(), unknown_button_type(ty)))
}

/// `widget.part`: a widget `widget:` would accept, that renders, and has a
/// part by that name.
fn part_key(widget: &Ident, part: &Ident) -> Result<SlotKey> {
    let Some(builtin) = BuiltinWidget::from_form_name(&widget.to_string()) else {
        return Err(syn::Error::new(
            widget.span(),
            unknown_widget(widget, false),
        ));
    };
    if !builtin.is_rendered() {
        return Err(syn::Error::new(
            widget.span(),
            format!("`{widget}` is not supported yet — nothing renders it, so it has no classes"),
        ));
    }
    SlotKey::part(builtin, &part.to_string()).ok_or_else(|| {
        let parts: Vec<&str> = builtin.parts().iter().map(|p| p.name).collect();
        syn::Error::new(
            part.span(),
            format!(
                "`{widget}` has no part `{part}` — it has {}",
                parts.join(", ")
            ),
        )
    })
}

/// A lone name: a `FixedKey`. Failing that, the message guesses what was
/// meant: a widget written without its part, `button` without its type, or
/// a near miss.
fn fixed_key(name: &Ident) -> Result<SlotKey> {
    let written = name.to_string();
    if let Some(key) = FixedKey::from_form_name(&written) {
        return Ok(SlotKey::Fixed(key));
    }
    let message = if let Some(widget) = BuiltinWidget::from_form_name(&written) {
        let parts: Vec<String> = widget
            .parts()
            .iter()
            .map(|p| format!("`{written}.{}`", p.name))
            .collect();
        format!(
            "`{written}` is a widget, and its classes are set part by part: {}",
            parts.join(", ")
        )
    } else if written == "button" {
        "a button's classes are set by its type, as `button.submit`".to_string()
    } else {
        let fixed = || {
            FixedKey::ALL
                .iter()
                .map(FixedKey::form_name)
                .filter(|n| !n.contains('.'))
        };
        match fixed()
            .filter(|n| edit_distance(&written, n) <= 2)
            .min_by_key(|n| edit_distance(&written, n))
        {
            Some(near) => format!("unknown class selector `{written}` — did you mean `{near}`?"),
            None => format!(
                "unknown class selector `{written}` — expected `widget.part` \
                 (`select.select`), `button.<type>`, or one of {}",
                fixed().collect::<Vec<_>>().join(", ")
            ),
        }
    };
    Err(syn::Error::new(name.span(), message))
}

/// What an entry does to one piece of a slot (its base or its invalid
/// class), mirroring the library's private `Change`. The classes are already
/// normalised and space-joined.
#[derive(Debug, PartialEq)]
pub(crate) enum Change {
    Replace(String),
    Append(String),
}

/// A value, as `(base, invalid)`, each `None` when the entry leaves that piece
/// alone:
///
/// - `[a, b]` / `+ [a, b]`: the base only, replaced or appended;
/// - `{ base: …, invalid: … }`: either piece or both, each written as the
///   short form is, so each is replaced or appended on its own.
fn parse_class_value(input: ParseStream<'_>) -> Result<(Option<Change>, Option<Change>)> {
    if input.peek(Token![+]) || input.peek(token::Bracket) {
        return Ok((Some(parse_change(input)?), None));
    }
    if !input.peek(token::Brace) {
        return Err(
            input.error("expected `[classes]`, `+ [classes]`, or `{ base: …, invalid: … }`")
        );
    }
    let body;
    let braces = braced!(body in input);
    let (mut base, mut invalid) = (None, None);
    while !body.is_empty() {
        let key: Ident = body.parse()?;
        let piece = match key.to_string().as_str() {
            "base" => &mut base,
            "invalid" => &mut invalid,
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown key `{other}` — expected `base` or `invalid`"),
                ));
            }
        };
        if piece.is_some() {
            return Err(syn::Error::new(
                key.span(),
                format!("`{key}` is given twice"),
            ));
        }
        body.parse::<Token![:]>()?;
        *piece = Some(parse_change(&body)?);
        if !body.is_empty() {
            body.parse::<Token![,]>()?;
        }
    }
    if base.is_none() && invalid.is_none() {
        return Err(syn::Error::new(
            braces.span.join(),
            "empty braces change nothing — give `base:`, `invalid:`, or both",
        ));
    }
    Ok((base, invalid))
}

/// `[a, b]` replaces; `+ [a, b]` appends. `[]` is a replace with nothing,
/// which clears the piece; `+ []` would append nothing, so it is refused.
fn parse_change(input: ParseStream<'_>) -> Result<Change> {
    let append = input.parse::<Option<Token![+]>>()?.is_some();
    let empty = append.then_some("`+ []` appends nothing — drop the `+` to clear instead");
    let classes: Vec<String> = class_list(input, empty)?
        .iter()
        .map(syn::LitStr::value)
        .collect();
    let classes = classes.join(" ");
    Ok(if append {
        Change::Append(classes)
    } else {
        Change::Replace(classes)
    })
}

#[cfg(test)]
mod tests {
    use super::{Change, ClassEntry, parse_class_value, parse_slot_key};
    use crate::form::Entry;
    use crate::form::tests::{err_of, parse};
    use formoxus_spec::{BuiltinWidget, ButtonType, FixedKey, RenderedInputType, SlotKey};
    use googletest::prelude::*;
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::parse::Parser;

    fn key(tokens: TokenStream2) -> syn::Result<SlotKey> {
        parse_slot_key.parse2(tokens).map(|(key, _)| key)
    }

    fn key_err(tokens: TokenStream2) -> String {
        key(tokens).expect_err("should not parse").to_string()
    }

    #[gtest]
    fn a_widget_part_is_its_form_name_and_part_name() {
        expect_that!(
            key(quote!(select.select)),
            ok(eq(&SlotKey::Part(BuiltinWidget::Select, "select")))
        );
        expect_that!(
            key(quote!(tel.input)),
            ok(eq(&SlotKey::Part(
                BuiltinWidget::Input(RenderedInputType::Telephone),
                "input"
            )))
        );
    }

    #[gtest]
    fn a_button_is_named_by_its_type_keyword() {
        expect_that!(
            key(quote!(button.submit)),
            ok(eq(&SlotKey::Fixed(FixedKey::Button(ButtonType::Submit))))
        );
    }

    #[gtest]
    fn a_fixed_key_is_its_snake_case_name() {
        expect_that!(
            key(quote!(button_row)),
            ok(eq(&SlotKey::Fixed(FixedKey::ButtonRow)))
        );
        expect_that!(
            key(quote!(required_marker)),
            ok(eq(&SlotKey::Fixed(FixedKey::RequiredMarker)))
        );
    }

    #[gtest]
    fn a_misspelt_widget_gets_a_suggestion_and_no_path_hint() {
        let msg = key_err(quote!(selct.select));
        expect_that!(msg, contains_substring("did you mean `select`"));
        let msg = key_err(quote!(fluorescent.input));
        expect_that!(msg, contains_substring("expected one of"));
        // A selector names a built-in's slots; a dev's widget has none.
        expect_that!(msg, not(contains_substring("path")));
    }

    #[gtest]
    fn a_part_the_widget_lacks_lists_the_ones_it_has() {
        expect_that!(
            key_err(quote!(select.lable)),
            eq("`select` has no part `lable` — it has div, label, select")
        );
    }

    #[gtest]
    fn a_widget_not_rendered_yet_is_refused() {
        expect_that!(
            key_err(quote!(file.input)),
            contains_substring("`file` is not supported yet")
        );
    }

    #[gtest]
    fn a_misspelt_button_type_gets_the_buttons_message() {
        expect_that!(
            key_err(quote!(button.submitt)),
            contains_substring("did you mean `submit`")
        );
    }

    #[gtest]
    fn a_misspelt_fixed_key_gets_a_suggestion() {
        expect_that!(
            key_err(quote!(buton_row)),
            contains_substring("did you mean `button_row`")
        );
    }

    #[gtest]
    fn a_widget_without_its_part_is_told_how_to_name_one() {
        expect_that!(
            key_err(quote!(select)),
            eq(
                "`select` is a widget, and its classes are set part by part: \
                `select.div`, `select.label`, `select.select`"
            )
        );
    }

    #[gtest]
    fn button_without_its_type_is_told_how_to_name_one() {
        expect_that!(
            key_err(quote!(button)),
            contains_substring("as `button.submit`")
        );
    }

    #[gtest]
    fn an_unknown_name_lists_the_fixed_keys_but_not_the_buttons() {
        let msg = key_err(quote!(zzzzzzzz));
        expect_that!(msg, contains_substring("button_row"));
        expect_that!(msg, contains_substring("`button.<type>`"));
        expect_that!(msg, not(contains_substring("button.submit")));
    }

    // ── Values ───────────────────────────────────────────────────────────

    fn value(tokens: TokenStream2) -> syn::Result<(Option<Change>, Option<Change>)> {
        parse_class_value.parse2(tokens)
    }

    fn value_err(tokens: TokenStream2) -> String {
        value(tokens).expect_err("should not parse").to_string()
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "one piece of an expected `(base, invalid)`, where `None` is the other value"
    )]
    fn replace(s: &str) -> Option<Change> {
        Some(Change::Replace(s.to_string()))
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "one piece of an expected `(base, invalid)`, where `None` is the other value"
    )]
    fn append(s: &str) -> Option<Change> {
        Some(Change::Append(s.to_string()))
    }

    /// The field-level list syntax: bare names with `_` → `-`, quoted ones
    /// verbatim, space-joined into one string.
    #[gtest]
    fn the_short_form_replaces_the_base_only() {
        expect_that!(
            value(quote!([form_select, "is-big"])),
            ok(eq(&(replace("form-select is-big"), None)))
        );
    }

    #[gtest]
    fn a_plus_appends_to_the_base() {
        expect_that!(value(quote!(+ [mine])), ok(eq(&(append("mine"), None))));
    }

    #[gtest]
    fn the_braced_form_takes_either_piece_or_both() {
        expect_that!(
            value(quote!({ base: [form_control], invalid: + [is_invalid] })),
            ok(eq(&(replace("form-control"), append("is-invalid"))))
        );
        expect_that!(
            value(quote!({ invalid: [is_invalid] })),
            ok(eq(&(None, replace("is-invalid"))))
        );
        expect_that!(
            value(quote!({ invalid: [x], base: + [y], })),
            ok(eq(&(append("y"), replace("x"))))
        );
    }

    #[gtest]
    fn an_empty_replace_clears_and_an_empty_append_is_refused() {
        expect_that!(value(quote!({ invalid: [] })), ok(eq(&(None, replace("")))));
        expect_that!(value(quote!([])), ok(eq(&(replace(""), None))));
        expect_that!(
            value_err(quote!(+ [])),
            eq("`+ []` appends nothing — drop the `+` to clear instead")
        );
    }

    #[gtest]
    fn the_braced_form_refuses_what_it_does_not_know() {
        expect_that!(
            value_err(quote!({ bse: [x] })),
            eq("unknown key `bse` — expected `base` or `invalid`")
        );
        expect_that!(
            value_err(quote!({ base: [x], base: [y] })),
            eq("`base` is given twice")
        );
        expect_that!(
            value_err(quote!({})),
            eq("empty braces change nothing — give `base:`, `invalid:`, or both")
        );
    }

    /// A string is the other syntax an author might reach for; the message
    /// names the three that work.
    #[gtest]
    fn anything_else_names_the_three_forms() {
        expect_that!(
            value_err(quote!("form-select")),
            eq("expected `[classes]`, `+ [classes]`, or `{ base: …, invalid: … }`")
        );
    }

    #[gtest]
    fn a_class_name_is_checked_as_a_fields_is() {
        expect_that!(
            value_err(quote!([text - center])),
            contains_substring("text-center")
        );
    }

    // ── The block ────────────────────────────────────────────────────────

    /// The entries of the one `classes_plus:` block, as `(key, base,
    /// invalid)`.
    fn block(tokens: TokenStream2) -> Vec<(SlotKey, Option<Change>, Option<Change>)> {
        let spec = parse(tokens).expect("should parse");
        spec.entries
            .into_iter()
            .find_map(|e| match e {
                Entry::ClassesPlus(entries) => Some(entries),
                _ => None,
            })
            .expect("a classes_plus block")
            .into_iter()
            .map(|ClassEntry { key, base, invalid }| (key, base, invalid))
            .collect()
    }

    #[gtest]
    fn a_block_keeps_its_entries_in_source_order() {
        let entries = block(quote! {
            Signup {
                classes_plus: {
                    select.select: [form_select],
                    button_row: + [actions],
                    text.input: { invalid: [is_invalid] },
                }
            }
        });
        expect_that!(
            entries,
            elements_are![
                eq(&(
                    SlotKey::Part(BuiltinWidget::Select, "select"),
                    replace("form-select"),
                    None
                )),
                eq(&(SlotKey::Fixed(FixedKey::ButtonRow), append("actions"), None)),
                eq(&(
                    SlotKey::Part(BuiltinWidget::Input(RenderedInputType::Text), "input"),
                    None,
                    replace("is-invalid")
                )),
            ]
        );
    }

    #[gtest]
    fn a_selector_given_twice_is_refused() {
        expect_that!(
            err_of(quote! {
                Signup {
                    classes_plus: { select.select: [a], select.select: + [b] }
                }
            }),
            eq("`select.select` is given twice — merge the two entries into one")
        );
    }

    #[gtest]
    fn an_empty_block_is_refused() {
        expect_that!(
            err_of(quote! { Signup { classes_plus: {} } }),
            eq("empty `classes_plus` block — drop it")
        );
    }

    #[gtest]
    fn the_block_needs_braces() {
        expect_that!(
            err_of(quote! { Signup { classes_plus: [x] } }),
            contains_substring("expected a `classes_plus` block surrounded by braces")
        );
    }

    #[gtest]
    fn a_block_given_twice_is_refused() {
        expect_that!(
            err_of(quote! {
                Signup {
                    classes_plus: { button_row: [a] },
                    classes_plus: { form: [b] },
                }
            }),
            contains_substring("`classes_plus` is given twice")
        );
    }

    /// The two keys together: `classes:` first, then the edits on top.
    #[gtest]
    fn classes_and_classes_plus_may_both_be_given() {
        let spec = parse(quote! {
            Signup {
                classes: ::formoxus::Classes::bare(),
                classes_plus: { form: [form] },
            }
        })
        .expect("both keys together should parse");
        expect_that!(
            crate::form::tests::kinds(&spec),
            elements_are![eq(&"classes"), eq(&"classes_plus")]
        );
    }

    // ── The expansion ────────────────────────────────────────────────────

    /// The whole expansion of `form!`, as a string, for the `with_class_edit`
    /// calls in it.
    fn expansion(tokens: TokenStream2) -> String {
        parse(tokens).expect("should parse").expand().to_string()
    }

    #[gtest]
    fn a_part_entry_expands_to_a_part_edit_per_piece() {
        expect_that!(
            expansion(quote! {
                Signup {
                    classes_plus: { select.select: { base: [form_select], invalid: + [is_invalid] } }
                }
            }),
            contains_substring(
                ". with_class_edit (:: formoxus :: classes :: ClassEdit :: part \
                 (:: formoxus :: attrs :: BuiltinWidget :: Select , \"select\") \
                 . replace_base (\"form-select\") . append_invalid (\"is-invalid\"))"
            )
        );
    }

    #[gtest]
    fn an_input_part_names_its_input_type() {
        expect_that!(
            expansion(quote! { Signup { classes_plus: { tel.input: + [phone] } } }),
            contains_substring(
                ":: formoxus :: attrs :: BuiltinWidget :: Input \
                 (:: formoxus :: attrs :: RenderedInputType :: Telephone) , \"input\") \
                 . append_base (\"phone\")"
            )
        );
    }

    #[gtest]
    fn a_fixed_entry_and_a_button_entry_expand_to_fixed_edits() {
        let tokens = expansion(quote! {
            Signup { classes_plus: { button_row: [actions], button.submit: [btn, btn_primary] } }
        });
        expect_that!(
            tokens,
            contains_substring(
                "ClassEdit :: fixed (:: formoxus :: classes :: FixedKey :: ButtonRow) \
                 . replace_base (\"actions\")"
            )
        );
        expect_that!(
            tokens,
            contains_substring(
                "ClassEdit :: fixed (:: formoxus :: classes :: FixedKey :: Button \
                 (:: formoxus :: buttons :: ButtonType :: Submit)) \
                 . replace_base (\"btn btn-primary\")"
            )
        );
    }

    #[gtest]
    fn classes_expands_to_with_classes() {
        expect_that!(
            expansion(quote! { Signup { classes: my_classes() } }),
            contains_substring(". with_classes (my_classes ())")
        );
    }

    /// No `classes:` or `classes_plus:`, no calls: a spec without them
    /// expands to what it did before they existed.
    #[gtest]
    fn a_form_without_them_expands_to_neither() {
        let tokens = expansion(quote! { Signup { title: "Sign up" } });
        expect_that!(tokens, not(contains_substring("with_classes")));
        expect_that!(tokens, not(contains_substring("with_class_edit")));
    }
}
