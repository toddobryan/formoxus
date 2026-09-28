//! `/t/checkbox`, `/t/select`, `/t/radio-group`, `/t/textarea`.
//!
//! One widget each, proving the thing SSR cannot: that a real interaction lands
//! and the value survives the round trip through `onsubmit`.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{checkbox, combobox, radio, submit, textbox};
use playwright_rs::expect;

// ── checkbox ─────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn ticking_a_checkbox_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/checkbox").await?;
    checkbox(&s, "Agreed").check(None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("agreed: true")
        .await?;
    s.close().await
}

/// **Unticked is a complete answer.** A required `bool` means "we need an
/// answer", and `false` is one — so this submits rather than being blocked. That
/// is deliberate today; issue #6 is about being able to demand a tick, and if it
/// lands this test is the one that should change.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_unticked_checkbox_still_submits() -> Result<()> {
    let s = Session::open("/t/checkbox").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("agreed: false")
        .await?;
    s.close().await
}

// ── select ───────────────────────────────────────────────────────────────

/// The stored value is the code and the display is the name, so a round trip has
/// to come back with `"AK"` and never `"Alaska"`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn choosing_from_a_select_reaches_the_model_as_its_value() -> Result<()> {
    let s = Session::open("/t/select").await?;
    combobox(&s, "State").select_option("AK", None).await?;
    submit(&s).click(None).await?;
    let out = s.page.locator("#submitted");
    expect(out.clone()).to_contain_text("state: \"AK\"").await?;
    expect(out).not().to_contain_text("Alaska").await?;
    s.close().await
}

/// A required chooser starts on an unselectable placeholder, so submitting
/// without choosing is blocked by the browser.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_unanswered_required_select_is_blocked() -> Result<()> {
    let s = Session::open("/t/select").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

// ── radio-group ──────────────────────────────────────────────────────────

/// **Nothing is pre-selected.** A library that checked the first radio would make
/// a required group unable to fail its own required check, and would submit an
/// answer the user never gave.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn no_radio_is_checked_until_one_is_clicked() -> Result<()> {
    let s = Session::open("/t/radio-group").await?;
    expect(s.page.locator("input[type=\"radio\"]:checked"))
        .to_have_count(0)
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn clicking_a_radio_reaches_the_model_as_its_value() -> Result<()> {
    let s = Session::open("/t/radio-group").await?;
    radio(&s, "Arizona").check(None).await?;
    submit(&s).click(None).await?;
    let out = s.page.locator("#submitted");
    expect(out.clone()).to_contain_text("state: \"AZ\"").await?;
    expect(out).not().to_contain_text("Arizona").await?;
    s.close().await
}

/// `required` on a radio applies to the whole GROUP, which is the only thing
/// standing in for the `--none--` a `<select>` offers — and the reason an
/// optional field is refused outright.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_unanswered_radio_group_is_blocked() -> Result<()> {
    let s = Session::open("/t/radio-group").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

// ── textarea ─────────────────────────────────────────────────────────────

/// A multi-line value, which is the case `fields.rs`'s JavaScript-newline tests
/// were written for — seen from the browser rather than from Rust.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_multiline_value_round_trips_through_a_textarea() -> Result<()> {
    let s = Session::open("/t/textarea").await?;
    textbox(&s, "About").fill("one\ntwo", None).await?;
    submit(&s).click(None).await?;
    // `{:#?}` escapes the newline, so the model shows it as `\n`.
    expect(s.page.locator("#submitted"))
        .to_contain_text("one\\ntwo")
        .await?;
    s.close().await
}
