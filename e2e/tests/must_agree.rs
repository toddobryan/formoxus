//! `/t/must-agree` and `/t/must-agree-novalidate`: `required_true` on a bool
//! (issue #6), the "I agree to the terms" checkbox.
//!
//! The SSR suite pins that the `required` attribute and the ` *` marker render,
//! and `submissions.rs` pins the server-side rejection. Only a browser can show
//! that the attribute actually blocks an unticked submit, that formoxus's own
//! message is reachable once the browser stands aside, and that the marker
//! stays out of the accessible name.
//!
//! All `#[ignore]`d — they need a running server and a browser. Run `just e2e`.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{checkbox, submit};
use playwright_rs::expect;

// ── validation ON ────────────────────────────────────────────────────────

/// **The browser blocks an unticked box.** `onsubmit` never fires, so nothing
/// is recorded, and formoxus never gets to report it either.
///
/// Asserting EMPTY is vacuous if the click never landed. The next test clicks
/// the same button on the same form and gets a model, which is what makes this
/// one mean something.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_browser_blocks_an_unticked_must_agree_box() -> Result<()> {
    let s = Session::open("/t/must-agree").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_ticked_must_agree_box_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/must-agree").await?;
    checkbox(&s, "Agreed").check(None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("agreed: true")
        .await?;
    s.close().await
}

/// **The marker is visible but not part of the accessible name**, as on every
/// other widget. The exact role-and-name match proves the asterisk is NOT in
/// the name; the label-text match proves it IS still on the page.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_must_agree_marker_is_absent_from_the_accessible_name() -> Result<()> {
    let s = Session::open("/t/must-agree").await?;
    expect(checkbox(&s, "Agreed")).to_have_count(1).await?;
    expect(s.page.get_by_label("Agreed *", true))
        .to_have_count(1)
        .await?;
    s.close().await
}

// ── validation OFF ───────────────────────────────────────────────────────

/// **Validation OFF: formoxus rejects the unticked box itself.** This is the
/// check a request that never went through a browser also meets.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn novalidate_lets_formoxus_reject_an_unticked_box() -> Result<()> {
    let s = Session::open("/t/must-agree-novalidate").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_contain_text("must be true")
        .await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

/// **Ticked and then unticked is rejected the same way.** An untouched box is
/// `Empty` while a ticked-then-unticked one holds a value, so the two take
/// different routes into `validate`. This is the only test that drives the
/// second route through a real `onchange`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn ticking_then_unticking_is_rejected_the_same_way() -> Result<()> {
    let s = Session::open("/t/must-agree-novalidate").await?;
    let agreed = checkbox(&s, "Agreed");
    agreed.check(None).await?;
    agreed.uncheck(None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_contain_text("must be true")
        .await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}
