//! `help_text:`, a field's hint: a persistent line after its caption, which
//! its control (or group) is described by. Django's `help_text`, and its
//! default field template's order: caption, hint, control, errors.

use super::{render_to_html, without_form_id};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
use googletest::prelude::*;
use std::collections::BTreeSet;

const STATES: &[(&str, &str)] = &[("KY", "Kentucky"), ("OH", "Ohio")];

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Contact {
    Email { address: String },
    Phone { number: String },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    street: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Account {
    name: String,
    agreed: bool,
    state: String,
    region: String,
    contact: Contact,
    address: Address,
    phones: Vec<String>,
    tags: BTreeSet<String>,
}

fn helped() -> FormSpec<Account> {
    form! {
        Account {
            name => { help_text: "As it appears on your ID." },
            agreed => { help_text: "You can change this later." },
            state => { choices: STATES, help_text: "Where you live." },
            region => { widget: radio_group, choices: STATES, help_text: "Where you work." },
            contact => { help_text: "How we reach you." },
            address => { help_text: "No P.O. boxes." },
            phones => { help_text: "Mobile first." },
            tags => { help_text: "Lowercase only." },
        }
    }
}

#[component]
fn Helped() -> Element {
    let form = use_form(|| empty_form(helped()));
    form.render(using_fns! {})
}

fn html() -> String {
    without_form_id(&render_to_html(Helped))
}

/// The hint as `field_help` renders it, by the field's path.
fn hint(path: &str, text: &str) -> String {
    format!(r#"<div class="fx-help-text" id="FORMID-{path}-help">{text}</div>"#)
}

/// Where `needle` starts, which must be there.
fn at(html: &str, needle: &str) -> usize {
    html.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not in the markup"))
}

// ── Each kind of node ────────────────────────────────────────────────────

/// Caption, hint, control: what the field wants is read before it is
/// answered. The control names the hint.
#[gtest]
fn a_text_inputs_hint_follows_its_caption_and_describes_it() {
    let html = html();
    let caption = at(&html, r#"for="FORMID-name">Name"#);
    let help = at(&html, &hint("name", "As it appears on your ID."));
    let input = at(&html, r#"id="FORMID-name""#);
    expect_that!(caption < help && help < input, eq(true));
    expect_that!(
        html,
        contains_regex(r#"<input [^>]*id="FORMID-name"[^>]*aria-describedby="FORMID-name-help""#)
    );
}

/// A checkbox reads "☐ I agree": its caption follows the control, and the
/// hint follows the caption.
#[gtest]
fn a_checkboxs_hint_follows_its_caption_after_the_box() {
    let html = html();
    let input = at(&html, r#"id="FORMID-agreed""#);
    let caption = at(&html, r#"for="FORMID-agreed">Agreed"#);
    let help = at(&html, &hint("agreed", "You can change this later."));
    expect_that!(input < caption && caption < help, eq(true));
    expect_that!(
        html,
        contains_regex(
            r#"<input [^>]*id="FORMID-agreed"[^>]*aria-describedby="FORMID-agreed-help""#
        )
    );
}

#[gtest]
fn a_selects_hint_comes_before_it_and_describes_it() {
    let html = html();
    let help = at(&html, &hint("state", "Where you live."));
    let select = at(
        &html,
        r#"<select class="fx-control fx-select" id="FORMID-state""#,
    );
    expect_that!(help < select, eq(true));
    expect_that!(
        html,
        contains_regex(
            r#"<select [^>]*id="FORMID-state"[^>]*aria-describedby="FORMID-state-help""#
        )
    );
}

/// The group is described, on its `fieldset`, and the hint sits under the
/// legend, before the first radio.
#[gtest]
fn a_radio_groups_hint_follows_its_legend_and_describes_the_group() {
    let html = html();
    expect_that!(
        html,
        contains_regex(
            r#"<fieldset [^>]*role="radiogroup"[^>]*aria-describedby="FORMID-region-help""#
        )
    );
    let legend = at(&html, "Region");
    let help = at(&html, &hint("region", "Where you work."));
    let first_radio = at(&html, r#"id="FORMID-region-0""#);
    expect_that!(legend < help && help < first_radio, eq(true));
}

/// An enum's legend is its select's caption, so the hint goes under it and
/// describes the select.
#[gtest]
fn an_enums_hint_follows_its_legend_and_describes_its_select() {
    let html = html();
    let legend = at(&html, r#"id="FORMID-contact-legend""#);
    let help = at(&html, &hint("contact", "How we reach you."));
    let select = at(&html, r#"id="FORMID-contact" name="contact""#);
    expect_that!(legend < help && help < select, eq(true));
    expect_that!(
        html,
        contains_regex(
            r#"<select [^>]*id="FORMID-contact"[^>]*aria-describedby="FORMID-contact-help""#
        )
    );
}

/// A struct, a list and a set: a `fieldset` described by its hint, which
/// sits under its legend.
#[gtest]
fn a_groups_hint_follows_its_legend_and_describes_its_fieldset() {
    let html = html();
    for (path, legend, text) in [
        ("address", "Address</legend>", "No P.O. boxes."),
        ("phones", "Phones</legend>", "Mobile first."),
        ("tags", "Tags</legend>", "Lowercase only."),
    ] {
        expect_that!(
            html,
            contains_regex(format!(
                r#"<fieldset [^>]*aria-describedby="FORMID-{path}-help""#
            )),
            "{path}"
        );
        let legend = at(&html, legend);
        let help = at(&html, &hint(path, text));
        expect_that!(legend < help, eq(true), "{path}");
    }
}

// ── With errors, and without a hint ──────────────────────────────────────

#[component]
fn Invalid() -> Element {
    let form = use_form(|| empty_form(helped()));
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// The hint stays once the field is invalid, and the control is described
/// by both, the hint first, as they render.
#[gtest]
fn an_invalid_field_is_described_by_its_hint_then_its_errors() {
    let html = without_form_id(&render_to_html(Invalid));
    expect_that!(
        html,
        contains_regex(
            r#"<input [^>]*id="FORMID-name"[^>]*aria-describedby="FORMID-name-help FORMID-name-errors""#
        )
    );
    let help = at(&html, &hint("name", "As it appears on your ID."));
    let errors = at(&html, r#"id="FORMID-name-errors""#);
    expect_that!(help < errors, eq(true));
}

#[component]
fn Unhelped() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Account>::default()));
    form.render(using_fns! {})
}

/// No hint, no `<div>` and no `aria-describedby`: naming an element that is
/// not rendered is itself a defect.
#[gtest]
fn without_a_hint_nothing_is_rendered_or_described() {
    let html = render_to_html(Unhelped);
    expect_that!(html, not(contains_substring("fx-help-text")));
    expect_that!(html, not(contains_substring("-help")));
    expect_that!(html, not(contains_substring("aria-describedby")));
}

// ── Built by hand, and classed ───────────────────────────────────────────

#[component]
fn HandBuilt() -> Element {
    let form = use_form(|| {
        empty_form(FormSpec::<Account>::default().with_help_text("name", "Built by hand."))
    });
    form.render(using_fns! {})
}

#[gtest]
fn with_help_text_is_what_form_expands_to() {
    let html = without_form_id(&render_to_html(HandBuilt));
    expect_that!(html, contains_substring(hint("name", "Built by hand.")));
}

#[component]
fn ClassedHint() -> Element {
    let form = use_form(|| {
        empty_form(form! {
            Account {
                name => { help_text: "Classed." },
                classes_plus: { help_text: + ["muted"] },
            }
        })
    });
    form.render(using_fns! {})
}

#[component]
fn BareHint() -> Element {
    let form = use_form(|| {
        empty_form(form! {
            Account {
                name => { help_text: "Bare." },
                classes: Classes::bare(),
            }
        })
    });
    form.render(using_fns! {})
}

/// Its own slot, `help_text`, so a theme can style it apart from a caption
/// or an error; `bare()` leaves it unclassed but still there.
#[gtest]
fn the_hint_has_a_class_slot_of_its_own() {
    expect_that!(
        render_to_html(ClassedHint),
        contains_regex(r#"<div class="fx-help-text muted" id="[^"]+-name-help">Classed\.</div>"#)
    );
    expect_that!(
        render_to_html(BareHint),
        contains_regex(r#"<div id="[^"]+-name-help">Bare\.</div>"#)
    );
}
