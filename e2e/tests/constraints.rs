//! `/t/pattern`, `/t/pattern-novalidate`, `/t/lengths`, `/t/bounds`.
//!
//! The constraint attributes, from the browser's side. The SSR suite proves they
//! are IN the markup; only these prove a browser acts on them — which is the
//! assumption behind emitting them at all.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{spinbutton, submit, textbox};
use playwright_rs::expect;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_pattern_is_enforced_by_the_browser() -> Result<()> {
    let s = Session::open("/t/pattern").await?;
    textbox(&s, "Code").fill("abc", None).await?;
    submit(&s).click(None).await?;
    // Blocked before the handler, so nothing was recorded.
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_value_matching_the_pattern_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/pattern").await?;
    textbox(&s, "Code").fill("90210", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("code: \"90210\"")
        .await?;
    s.close().await
}

/// **The anchoring, end to end.** HTML wraps a `pattern` as `^(?:…)$` implicitly,
/// and formoxus wraps it Rust-side to match — so a value with five digits
/// EMBEDDED in it must be rejected by both. This is the browser half of
/// `fields.rs`'s `a_pattern_must_match_the_entire_value`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_browser_anchors_the_pattern_too() -> Result<()> {
    let s = Session::open("/t/pattern").await?;
    textbox(&s, "Code").fill("xx90210xx", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

/// With validation off the same bad value reaches formoxus, which reports it —
/// and agrees with the browser about what the pattern means, which is why
/// `regress` is a dependency.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn formoxus_reports_a_bad_pattern_when_the_browser_does_not() -> Result<()> {
    let s = Session::open("/t/pattern-novalidate").await?;
    textbox(&s, "Code").fill("xx90210xx", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator(".field-error"))
        .to_contain_text("regular expression")
        .await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn min_length_is_enforced_by_the_browser() -> Result<()> {
    let s = Session::open("/t/lengths").await?;
    textbox(&s, "Nick").fill("ab", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

/// `maxlength` is enforced as you TYPE rather than at submit, so this checks the
/// value the browser allowed rather than whether submit was blocked.
/// `press_sequentially` is required — `fill` sets the value directly and bypasses
/// the limit.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn max_length_stops_the_browser_accepting_more() -> Result<()> {
    let s = Session::open("/t/lengths").await?;
    let field = textbox(&s, "Nick");
    field.press_sequentially("abcdefghij", None).await?;
    expect(field).to_have_value("abcdef").await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_value_within_the_lengths_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/lengths").await?;
    textbox(&s, "Nick").fill("ada", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("nick: \"ada\"")
        .await?;
    s.close().await
}

/// A number BELOW `min` is blocked. Note the role: `type="number"` is a
/// `spinbutton`, not a `textbox`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_bound_is_enforced_by_the_browser() -> Result<()> {
    let s = Session::open("/t/bounds").await?;
    spinbutton(&s, "Age").fill("5", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_number_within_the_bounds_reaches_the_model() -> Result<()> {
    let s = Session::open("/t/bounds").await?;
    spinbutton(&s, "Age").fill("42", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("age: 42")
        .await?;
    s.close().await
}
