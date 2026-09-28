//! `/t/required-text` and `/t/required-text-novalidate`.
//!
//! Between them these two are the whole `novalidate` story, and the only proof
//! formoxus has that `browser_validation` does what it claims. The SSR suite can
//! show that `novalidate` reaches the `<form>` and that `required` reaches the
//! input; only a browser can show that one neutralizes the other.
//!
//! All `#[ignore]`d — they need a running server and a browser. Run `just e2e`.

use anyhow::Result;
use e2e::Session;
use playwright_rs::expect;
use playwright_rs::protocol::locator::{AriaRole, GetByRoleOptions};

/// The field, located by its ACCESSIBLE NAME rather than its label text.
///
/// This distinction is the whole point of the `aria-hidden` on formoxus's
/// required marker, and it is easy to get wrong: `get_by_label` matches the
/// `<label>`'s TEXT, which still contains the asterisk, while `get_by_role`
/// matches the computed accessible name, which does not. So an exact
/// `get_by_label("Name")` times out and an exact role-and-name match succeeds —
/// verified, not assumed.
fn named(s: &Session, name: &str) -> playwright_rs::Locator {
    s.page.get_by_role(
        AriaRole::Textbox,
        Some({
            // `GetByRoleOptions` is `#[non_exhaustive]`, so it has to be built
            // from `Default` and mutated rather than with a struct literal.
            let mut o = GetByRoleOptions::default();
            o.name = Some(name.to_string());
            o.exact = Some(true);
            o
        }),
    )
}

/// **Validation ON: the browser blocks an empty required field.** `onsubmit`
/// never fires, so nothing is recorded — `#submitted` stays empty. This is the
/// assumption behind emitting constraint attributes at all, and it was untested
/// until now.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn browser_validation_blocks_an_empty_required_field() -> Result<()> {
    let s = Session::open("/t/required-text").await?;
    s.page
        .locator("button[type=\"submit\"]")
        .click(None)
        .await?;
    // Nothing reached the handler.
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    // And formoxus never got to report it either — the browser got there first.
    expect(s.page.locator(".field-error"))
        .to_have_count(0)
        .await?;
    s.close().await
}

/// A value that satisfies the field submits and round-trips into the model.
/// Pretty-printed, so the field sits on its own line.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_filled_required_field_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/required-text").await?;
    named(&s, "Name").fill("Ada", None).await?;
    s.page
        .locator("button[type=\"submit\"]")
        .click(None)
        .await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("name: \"Ada\"")
        .await?;
    s.close().await
}

/// **Validation OFF: the browser lets it through and formoxus reports it.** The
/// mirror image of the first test, and the half the SSR suite cannot reach —
/// formoxus's own error rendering is only reachable once the browser stops
/// pre-empting it.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn novalidate_lets_formoxus_report_the_empty_field() -> Result<()> {
    let s = Session::open("/t/required-text-novalidate").await?;
    s.page
        .locator("button[type=\"submit\"]")
        .click(None)
        .await?;
    expect(s.page.locator(".field-error"))
        .to_contain_text("required")
        .await?;
    // The submit was rejected by formoxus, so no model was recorded.
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

/// `novalidate` is on the `<form>`, not on the field — the `required` attribute
/// is still emitted either way. This is what makes gating unnecessary, so it is
/// worth pinning from the browser's side rather than only from SSR.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_field_is_still_marked_required_under_novalidate() -> Result<()> {
    let s = Session::open("/t/required-text-novalidate").await?;
    expect(s.page.locator("form[novalidate]"))
        .to_have_count(1)
        .await?;
    expect(s.page.locator("input[required]"))
        .to_have_count(1)
        .await?;
    s.close().await
}

/// **The required marker really is hidden from the accessible name.** The SSR
/// suite can only pin the `aria-hidden` attribute; computing a name needs a
/// browser, so this is the test that makes the claim true rather than plausible.
///
/// Both halves matter. The exact role-and-name match proves the asterisk is NOT
/// in the name; the label-text match proves it IS still in the DOM and visible,
/// so sighted users keep the convention.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_required_marker_is_absent_from_the_accessible_name() -> Result<()> {
    let s = Session::open("/t/required-text").await?;
    expect(named(&s, "Name")).to_have_count(1).await?;
    // The asterisk is still rendered — `get_by_label` reads the label's text,
    // which includes it.
    expect(s.page.get_by_label("Name *", true))
        .to_have_count(1)
        .await?;
    s.close().await
}
