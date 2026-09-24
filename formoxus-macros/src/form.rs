//! `form!` — the parser, and the `FormSpec` chain it expands into.
//!
//! Split by grammar region. This file holds the top-level entries; each
//! submodule owns one nested grammar and the errors that grammar can raise.

mod button;
mod field;
mod spec_path;
mod suggest;
mod widget;

pub(crate) use button::ButtonInfo;
pub(crate) use field::{FieldBody, FieldSpec};
pub(crate) use spec_path::{SpecPath, probe};
pub(crate) use widget::WidgetRef;

use std::collections::HashSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, Path, Result, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream},
    punctuated::Punctuated,
};

pub(crate) fn impl_form(input: TokenStream2) -> TokenStream2 {
    match syn::parse2::<FormSpecInput>(input) {
        Ok(spec) => spec.expand(),
        Err(err) => err.to_compile_error(),
    }
}

/// Every label casing, keyed by a string that IS what that casing does to the
/// words "label case".
///
/// That is the whole trick: `"Label-Case"` does not *describe* Train case, it
/// *demonstrates* it, so nobody has to remember whether "Title Case" means
/// spaces or whether camel is upper or lower. It also means the table is
/// checkable rather than merely documented — `the_key_is_what_the_case_does`
/// in the consumer suite asserts that every string here really is
/// `"label_case".to_case(variant)`, so an edit to either side that breaks the
/// correspondence fails rather than quietly turning a key into a lie.
const LABEL_CASES: [(&str, &str); 11] = [
    ("labelCase", "CamelLower"),
    ("LabelCase", "CamelCapitalized"),
    ("label-case", "KebabLower"),
    ("Label-Case", "KebabCapitalized"),
    ("LABEL-CASE", "KebabAllCaps"),
    ("label_case", "SnakeLower"),
    ("Label_Case", "SnakeCapitalized"),
    ("LABEL_CASE", "SnakeAllCaps"),
    ("Label Case", "Title"),
    ("label case", "Lower"),
    ("LABEL CASE", "AllCaps"),
];

mod kw {
    syn::custom_keyword!(label_case);
    syn::custom_keyword!(browser_validation);
    syn::custom_keyword!(on);
    syn::custom_keyword!(off);
    syn::custom_keyword!(title);
    syn::custom_keyword!(validator);
    syn::custom_keyword!(buttons);
}

#[derive(Debug)]
pub(crate) struct FormSpecInput {
    pub(crate) target: Path,
    pub(crate) entries: Vec<Entry>,
}

#[derive(Debug)]
struct FormSpecMeta {
    model_type: Path,
    title: Option<Expr>,
    use_browser_validation: Option<bool>,
    label_case: Option<Ident>,
    validator: Option<Expr>,
    buttons: Vec<ButtonInfo>,
    field_specs: Vec<FieldSpec>,
}

impl FormSpecMeta {
    fn new(model_type: Path) -> Self {
        Self {
            model_type,
            title: None,
            use_browser_validation: None,
            label_case: None,
            validator: None,
            buttons: Vec::new(),
            field_specs: Vec::new(),
        }
    }
}

impl FormSpecInput {
    fn expand(self) -> TokenStream2 {
        let mut fsm: FormSpecMeta = FormSpecMeta::new(self.target);
        for e in self.entries {
            match e {
                Entry::Title(expr) => fsm.title = Some(expr),
                Entry::BrowserValidation(value) => fsm.use_browser_validation = Some(value),
                Entry::LabelCase(ident) => fsm.label_case = Some(ident),
                Entry::Validator(expr) => fsm.validator = Some(expr),
                // The whole body, not field by field: adding a key should touch
                // `FieldBody` and nothing else.
                Entry::Field { path, body } => {
                    fsm.field_specs.push(FieldSpec { path, body: *body });
                }
                Entry::Buttons(buttons) => fsm.buttons = buttons,
            }
        }

        let model_type: Path = fsm.model_type;
        let title: Option<TokenStream2> = fsm.title.map(|t| {
            quote! {
                .with_title(&#t)
            }
        });
        let label_case: Option<TokenStream2> = fsm.label_case.map(|c| {
            quote! {
                .with_label_case(::formoxus::label_case::LabelCase::#c)
            }
        });
        let use_browser_validation: Option<TokenStream2> = fsm.use_browser_validation.map(|ubv| {
            quote! {
                .with_use_browser_validation(#ubv)
            }
        });
        let validator: Option<TokenStream2> = fsm.validator.map(|v| {
            quote! {
                .with_validator(#v)
            }
        });
        // Omitted entirely when there are none, so a spec without buttons
        // expands to exactly what it did before they existed.
        let buttons: Option<TokenStream2> = (!fsm.buttons.is_empty()).then(|| {
            let specs = fsm.buttons.iter().map(ButtonInfo::spec_tokens);
            quote! { .with_buttons(::std::vec![ #(#specs),* ]) }
        });
        let fields: Vec<TokenStream2> = fsm
            .field_specs
            .iter()
            .map(|f| {
                let key = f.path.key();
                let label = f
                    .body
                    .label
                    .as_ref()
                    .map(|l| quote! { .with_label(#key, &#l) });
                let widget = f.body.widget.as_ref().map(|c| {
                    let c = c.path();
                    quote! { .with_custom_widget(#key, #c) }
                });
                let choices = f
                    .body
                    .widget
                    .as_ref()
                    .and_then(|w| w.args.choices.as_ref())
                    .map(|c| {
                        quote! { .with_choices(#key, #c) }
                    });
                let constraints = f
                    .body
                    .constraints_tokens()
                    .map(|c| quote! { .with_constraints(#key, #c) });
                quote! { #label #widget #choices #constraints }
            })
            .collect();
        let witnesses: Vec<TokenStream2> = fsm
            .field_specs
            .iter()
            .map(|f| probe(&f.path.segments, quote!(__s), 0))
            .collect();

        quote! {
            {
                #[allow(unused)]
                fn __paths_exist(__s: &#model_type) {
                    #(#witnesses)*
                }

                ::formoxus::form::FormSpec::<#model_type>::new()
                #title
                #label_case
                #use_browser_validation
                #validator
                #buttons
                #(#fields)*
            }
        }
    }
}

#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
struct FormAttribute {
    title: bool,
    browser_validation: bool,
    label_case: bool,
    validator: bool,
    buttons: bool,
    fields: HashSet<String>,
}

impl FormAttribute {
    fn check_duplicate(&mut self, body: &ParseBuffer<'_>, entry: &Entry) -> Result<()> {
        if self.check_seen_and_set(entry) {
            if let Entry::Field { path, .. } = entry {
                let key = path.key();
                Err(syn::Error::new(
                    path.span(),
                    format!("`{key}` is specified twice — merge the two bodies into one"),
                ))
            } else {
                let name = entry.name();
                Err(body.error(format!("`{name}` is given twice")))
            }
        } else {
            Ok(())
        }
    }

    fn check_seen_and_set(&mut self, entry: &Entry) -> bool {
        let previously_seen = match entry {
            Entry::Title(_) => self.title,
            Entry::LabelCase(_) => self.label_case,
            Entry::BrowserValidation(_) => self.browser_validation,
            Entry::Validator(_) => self.validator,
            Entry::Buttons(_) => self.buttons,
            Entry::Field { path, .. } => {
                let key = path.key();
                self.fields.contains(&key)
            }
        };
        if !previously_seen {
            match entry {
                Entry::Title(_) => self.title = true,
                Entry::LabelCase(_) => self.label_case = true,
                Entry::BrowserValidation(_) => self.browser_validation = true,
                Entry::Validator(_) => self.validator = true,
                Entry::Buttons(_) => self.buttons = true,
                Entry::Field { path, .. } => {
                    let key = path.key();
                    let _ = self.fields.insert(key);
                }
            }
        }
        previously_seen
    }
}

impl Parse for FormSpecInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let target: Path = input.parse()?;
        let body;
        braced!(body in input);
        let entries: Vec<Entry> = Punctuated::<Entry, Token![,]>::parse_terminated(&body)?
            .into_iter()
            .collect();
        let mut attrs: FormAttribute = FormAttribute::default();
        for e in &entries {
            attrs.check_duplicate(&body, e)?;
        }
        Ok(FormSpecInput { target, entries })
    }
}

#[derive(Debug)]
pub(crate) enum Entry {
    Title(Expr),
    BrowserValidation(bool),
    /// The resolved `LabelCase` variant, already looked up — a bad string is a
    /// parse error, so nothing downstream has to handle one.
    LabelCase(Ident),
    Validator(Expr),
    /// **Boxed.** `FieldBody` carries up to three `syn::Expr`s, each of which is
    /// ~168 bytes, so inlining it here would make every `Entry` — including a
    /// `BrowserValidation(bool)` — as large as the biggest one.
    Field {
        path: SpecPath,
        body: Box<FieldBody>,
    },
    Buttons(Vec<ButtonInfo>),
}

impl Entry {
    /// `label_case: "Label Case"` — the string is an example of itself.
    fn parse_label_case(input: ParseStream<'_>) -> Result<Self> {
        let _kw: kw::label_case = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let lit: syn::LitStr = input
            .parse()
            .map_err(|_| input.error("`label_case` takes a string literal, e.g. \"Label Case\""))?;
        let written = lit.value();
        let Some((_, variant)) = LABEL_CASES.iter().find(|(k, _)| *k == written) else {
            // Every valid spelling, listed in full. There are only eleven,
            // and each one shows what it does, so the list IS the
            // documentation — better than naming the ones that are close.
            let all = LABEL_CASES
                .iter()
                .map(|(k, _)| format!("{k:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(syn::Error::new_spanned(
                &lit,
                format!("unknown label case {written:?} — write one of: {all}"),
            ));
        };
        Ok(Entry::LabelCase(Ident::new(variant, lit.span())))
    }

    fn parse_title(input: ParseStream<'_>) -> Result<Self> {
        let _title: kw::title = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let expr: Expr = input.parse()?;
        Ok(Entry::Title(expr))
    }

    fn parse_browser_validation(input: ParseStream<'_>) -> Result<Self> {
        let _browser_validation: kw::browser_validation = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let is_on = input.peek(kw::on);
        let is_off = input.peek(kw::off);
        if !is_on && !is_off {
            Err(input.error("Expected `on` or `off` after `browser_validation`"))
        } else if is_on {
            let _on: kw::on = input.parse()?;
            Ok(Entry::BrowserValidation(true))
        } else if is_off {
            let _off: kw::off = input.parse()?;
            Ok(Entry::BrowserValidation(false))
        } else {
            Err(input.error("It should be impossible to get here"))
        }
    }

    fn parse_validator(input: ParseStream<'_>) -> Result<Self> {
        let _validator: kw::validator = input.parse()?;
        let _color: Token![:] = input.parse()?;
        let expr: Expr = input.parse()?;
        Ok(Entry::Validator(expr))
    }

    fn parse_field(input: ParseStream<'_>) -> Result<Self> {
        let path: SpecPath = input.parse()?;
        if !input.peek(syn::token::FatArrow) {
            return Err(input.error("Expected => after a field name"));
        }

        input.parse::<syn::token::FatArrow>()?;
        let body: FieldBody = input.parse()?;
        Ok(Entry::Field {
            path,
            body: Box::new(body),
        })
    }

    /// `buttons: { save: { type: submit, text: "Save" }, … }`
    ///
    /// **Static knowledge only** — which buttons exist, in what order, how each
    /// one renders, and whether it validates first. What a button *does* is
    /// supplied at the render call site instead; `BUTTONS_PLAN.md` §2 records
    /// why that split is forced rather than chosen.
    ///
    /// Source order is display order, so this collects a `Vec` and not a map —
    /// the author's order is the only ordering information there is.
    fn parse_buttons(input: ParseStream<'_>) -> Result<Self> {
        let _buttons: kw::buttons = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a buttons block surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);

        let mut buttons: Vec<ButtonInfo> = Vec::new();
        while !body.is_empty() {
            let name: Ident = body.parse()?;
            let _colon: Token![:] = body.parse()?;
            // Each name becomes one slot in the generated handler struct, so a
            // repeat is a duplicate field rather than anything an author could
            // have meant by it.
            if buttons.iter().any(|b| b.name == name) {
                return Err(syn::Error::new_spanned(
                    &name,
                    format!("`{name}` is specified twice — merge the two bodies into one"),
                ));
            }
            buttons.push(ButtonInfo::parse_body(name, &body)?);
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        if buttons.is_empty() {
            return Err(syn::Error::new(braces.span.join(), "empty buttons block"));
        }
        Ok(Entry::Buttons(buttons))
    }

    fn name(&self) -> String {
        let attr_name: &str = match self {
            Entry::Title(_) => "title",
            Entry::BrowserValidation(_) => "browser_validation",
            Entry::LabelCase(_) => "label_case",
            Entry::Validator(_) => "validator",
            Entry::Field { path, .. } => &path.key(),
            Entry::Buttons(_) => "buttons",
        };
        attr_name.to_string()
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if attribute(input, kw::label_case) {
            Entry::parse_label_case(input)
        } else if attribute(input, kw::title) {
            Entry::parse_title(input)
        } else if attribute(input, kw::browser_validation) {
            Entry::parse_browser_validation(input)
        } else if attribute(input, kw::validator) {
            Entry::parse_validator(input)
        } else if attribute(input, kw::buttons) {
            Entry::parse_buttons(input)
        // `peek_any`, not `peek(Ident)`: a field may be named with a Rust
        // keyword (`r#type`), and a bare keyword is not an `Ident`. The five
        // guards above have already had their say, and each of them needs a
        // following `:`, so nothing an author writes as a field can be
        // swallowed here.
        } else if input.peek(Ident::peek_any) {
            Entry::parse_field(input)
        } else {
            Err(input.error("expected a form attribute or a field specifier"))
        }
    }
}

/// Is this entry the attribute `kw`, rather than a field whose name happens to
/// be that word?
///
/// A `custom_keyword!` is still an `Ident`, so `title` on its own cannot tell
/// the two apart — and a model with a `title` field is not a corner case, it is
/// most of them. The second token settles it: an attribute is `name:` and a
/// field spec is `name =>`, so a following `:` means attribute and anything
/// else means field.
///
/// **Testing for `:` rather than against `=>`.** The `=>` does not have to be
/// the second token — `title.text => …` and `titles[] => …` are both legal
/// paths rooted at a keyword, with `.` and `[` sitting where the `=>` would be.
/// Only `:` is guaranteed to be in this position, and only for an attribute.
fn attribute<K: syn::parse::Peek>(input: ParseStream<'_>, kw: K) -> bool
where
    K::Token: syn::token::Token,
{
    input.peek(kw) && input.peek2(Token![:])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::widget::WidgetKind;
    use super::*;
    use googletest::prelude::*;
    use quote::quote;
    use syn::Result;

    pub(crate) fn parse(tokens: TokenStream2) -> Result<FormSpecInput> {
        syn::parse2::<FormSpecInput>(tokens)
    }

    /// Which kind each entry is, in source order — enough to pin the grammar
    /// without requiring `Debug` on everything below `Entry`.
    pub(crate) fn kinds(spec: &FormSpecInput) -> Vec<&'static str> {
        spec.entries
            .iter()
            .map(|e| match e {
                Entry::Title(_) => "title",
                Entry::BrowserValidation(_) => "browser_validation",
                Entry::LabelCase(_) => "label_case",
                Entry::Validator(_) => "validator",
                Entry::Field { .. } => "field",
                Entry::Buttons(_) => "buttons",
            })
            .collect()
    }

    /// Field entries as `("dotted.path", "widget", "label")`, with `""` for an
    /// absent widget or label. The widget is rendered the way `expand` will
    /// have to, so this also pins that `Input(Password)` keeps both idents.
    pub(crate) fn fields(spec: &FormSpecInput) -> Vec<(String, String, String)> {
        spec.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Field { path, body } => Some((
                    path.key(),
                    match body.widget.as_ref().map(|w| &w.kind) {
                        None => String::new(),
                        Some(WidgetKind::Named(n)) => n.to_string(),
                        Some(WidgetKind::Custom(w)) => {
                            format!("custom({})", quote!(#w))
                        }
                    },
                    match &body.label {
                        None => String::new(),
                        Some(expr) => quote!(#expr).to_string(),
                    },
                )),
                _ => None,
            })
            .collect()
    }

    // ── The two real forms ───────────────────────────────────────────────

    #[gtest]
    fn the_login_form_parses() {
        let spec = parse(quote! {
            LoginForm {
                title: "Sign In",
                password => { widget: password },
            }
        })
        .expect("LoginForm should parse");

        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "password".to_string(),
                "password".to_string(),
                String::new()
            ))]
        );
    }

    #[gtest]
    fn the_change_password_form_parses() {
        // The whole surface at once: a title, a validator, three fields, two of
        // them carrying both keys. If entries were being dropped this would be
        // the test that noticed.
        let spec = parse(quote! {
            ChangePasswordForm {
                title: "Change Password",
                validator: check_new_and_confirm_match,
                current_password => { widget: password },
                new_password => { widget: password, label: "New password" },
                confirm_new_password => { widget: password, label: "Confirm new password" },
            }
        })
        .expect("ChangePasswordForm should parse");

        expect_that!(
            kinds(&spec),
            elements_are![
                eq(&"title"),
                eq(&"validator"),
                eq(&"field"),
                eq(&"field"),
                eq(&"field")
            ]
        );
        expect_that!(fields(&spec).len(), eq(3));
    }

    // ── Shapes ───────────────────────────────────────────────────────────

    #[gtest]
    fn a_bare_widget_needs_no_parens() {
        let spec = parse(quote! { Source { notes => { widget: textarea } } }).unwrap();
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "notes".to_string(),
                "textarea".to_string(),
                String::new()
            ))]
        );
    }

    #[gtest]
    fn a_label_only_entry_is_legal() {
        // Nothing about a label requires a widget — renaming a field is the
        // commonest customization there is.
        let spec = parse(quote! { Source { url => { label: "Homepage" } } }).unwrap();
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "url".to_string(),
                String::new(),
                "\"Homepage\"".to_string()
            ))]
        );
    }

    #[gtest]
    fn a_nested_path_keeps_every_segment() {
        // `expand` needs both halves out of this: the dotted string for the map
        // key and the same idents for `&s.venue.city` in the witness.
        let spec = parse(quote! { Event { venue.city => { label: "City" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("venue.city"));
    }

    #[gtest]
    fn the_target_may_be_a_qualified_path() {
        // A foreign model won't be in scope unqualified, which is the whole
        // second use case — so `Path`, not `Ident`.
        let spec = parse(quote! { models::Source { url => { label: "x" } } }).unwrap();
        let target = &spec.target;
        expect_that!(quote!(#target).to_string(), eq("models :: Source"));
    }

    #[gtest]
    fn trailing_commas_are_allowed_everywhere() {
        let spec = parse(quote! {
            Source {
                title: "Sources",
                notes => { widget: textarea, label: "Notes", },
            }
        })
        .expect("trailing commas in both positions should parse");
        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
    }

    #[gtest]
    fn a_spec_may_be_empty() {
        // `form! { T {} }` is the "formization with no customization" case —
        // every struct gets one, so the empty body has to be legal.
        let spec = parse(quote! { Source {} }).expect("an empty spec should parse");
        expect_that!(kinds(&spec), elements_are![]);
    }

    pub(crate) fn err_of(tokens: TokenStream2) -> String {
        parse(tokens).expect_err("should not parse").to_string()
    }

    // ── Entry errors ──────────────────────────────────────────────────────

    #[gtest]
    fn a_missing_fat_arrow_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes { widget: textarea } } }),
            contains_substring("=>")
        );
    }

    #[gtest]
    fn a_nonsense_entry_is_rejected() {
        expect_that!(err_of(quote! { Source { 42 } }).len(), gt(0));
    }

    // ── One entry per path ───────────────────────────────────────────────
    //
    // Two entries naming the same path would silently MERGE rather than
    // conflict: both go through `fields.entry(path).or_default()`, so a second
    // `label` overwrites the first and leaves the widget in place. Nothing
    // downstream can notice, which is why the parser has to.

    #[gtest]
    fn a_repeated_path_is_rejected() {
        let msg = err_of(quote! {
            Source {
                notes => { label: "Notes" },
                notes => { widget: textarea },
            }
        });
        expect_that!(msg, contains_substring("notes"));
        expect_that!(msg, contains_substring("twice"));
    }

    #[gtest]
    fn a_repeated_nested_path_is_rejected() {
        expect_that!(
            err_of(quote! {
                Event {
                    venue.city => { label: "City" },
                    venue.city => { label: "Town" },
                }
            }),
            contains_substring("venue.city")
        );
    }

    #[gtest]
    fn a_repeated_row_selector_is_rejected() {
        // `[]` is part of the key, so the comparison has to be on the rendered
        // key rather than on the idents alone.
        expect_that!(
            err_of(quote! {
                Trip {
                    venues[].city => { label: "City" },
                    venues[].city => { widget: textarea },
                }
            }),
            contains_substring("venues[].city")
        );
    }

    // ── Keyword-named fields ─────────────────────────────────────────────
    //
    // `title`, `validator` and `buttons` are `custom_keyword!`s and therefore
    // also idents, so a model field with one of those names collides with the
    // attribute of the same name. The second token disambiguates: `:` is an
    // attribute, anything else is a field path.

    #[gtest]
    fn a_field_may_be_named_title() {
        // Not a corner case — most models with prose in them have a `title`.
        let spec = parse(quote! { Article { title => { label: "Headline" } } })
            .expect("a field named `title` should parse as a field");
        expect_that!(kinds(&spec), elements_are![eq(&"field")]);
        expect_that!(fields(&spec)[0].0, eq("title"));
    }

    #[gtest]
    fn a_field_may_be_named_validator_or_buttons() {
        let spec = parse(quote! {
            Weird {
                validator => { label: "Validator" },
                buttons => { widget: textarea },
            }
        })
        .expect("fields named after the other two attributes should parse too");
        expect_that!(kinds(&spec), elements_are![eq(&"field"), eq(&"field")]);
    }

    #[gtest]
    fn an_attribute_and_a_field_of_the_same_name_coexist() {
        // The form is titled "Article" and also HAS a title. Nothing about one
        // should consume the other.
        let spec = parse(quote! {
            Article {
                title: "Article",
                title => { label: "Headline" },
            }
        })
        .expect("a `title:` attribute and a `title` field are different entries");
        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
        expect_that!(fields(&spec)[0].2, eq("\"Headline\""));
    }

    #[gtest]
    fn a_path_rooted_at_a_keyword_is_a_field() {
        // Why the test is for `:` rather than against `=>`: here the second
        // token is `.`, so "not a fat arrow" would have sent this to
        // `parse_title` and failed on the missing colon.
        let spec = parse(quote! { Page { title.text => { widget: textarea } } })
            .expect("a dotted path rooted at a keyword should parse as a field");
        expect_that!(fields(&spec)[0].0, eq("title.text"));
    }

    #[gtest]
    fn a_row_selector_on_a_keyword_name_is_a_field() {
        // Same hazard with `[` in the second position instead of `.`.
        let spec = parse(quote! { Deck { buttons[].text => { widget: text } } })
            .expect("a row selector rooted at a keyword should parse as a field");
        expect_that!(fields(&spec)[0].0, eq("buttons[].text"));
    }

    #[gtest]
    fn a_list_and_its_rows_are_different_paths() {
        // The collision that must NOT fire. `answers` is the `ListSet` (its
        // legend) and `answers[]` is every row; naming both in one spec is the
        // normal way to label a list and give its rows a widget. A dedup on the
        // idents alone would reject this.
        let spec = parse(quote! {
            Quiz {
                answers => { label: "Answers" },
                answers[] => { widget: textarea },
            }
        })
        .expect("a list and its rows are separate targets");
        expect_that!(
            fields(&spec)
                .iter()
                .map(|f| f.0.clone())
                .collect::<Vec<_>>(),
            elements_are![eq("answers"), eq("answers[]")]
        );
    }

    #[gtest]
    fn two_levels_of_the_same_list_are_different_paths() {
        // Likewise `rows[]` vs `rows[].cells[]` — a prefix is not a duplicate.
        let spec = parse(quote! {
            Grid {
                rows[] => { label: "Row" },
                rows[].cells[] => { widget: textarea },
            }
        })
        .expect("a row and a row's field are separate targets");
        expect_that!(fields(&spec).len(), eq(2));
    }

    #[gtest]
    fn a_second_title_is_rejected() {
        expect_that!(
            err_of(quote! { Source { title: "One", title: "Two" } }),
            contains_substring("title")
        );
    }

    #[gtest]
    fn a_second_validator_is_rejected() {
        // The one likelier to be a real mistake: two cross-field checks read as
        // if they would both run, and silently only the second would.
        expect_that!(
            err_of(quote! {
                Source {
                    validator: check_dates,
                    validator: check_urls,
                }
            }),
            contains_substring("validator")
        );
    }
}
