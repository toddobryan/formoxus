//! Live validation in a real browser (`ERRORS_PLAN.md` 8d): `/t/live-text`,
//! `/t/live-click`, `/t/live-server-error`, `/t/live-select`, `/t/live-off`,
//! `/t/live-custom`.
//!
//! The suite (`tests/suite/live_checks.rs`) fires each event by hand. These
//! prove what only a browser can: that Tab really sends the `blur` the check
//! waits for, that a keystroke's re-render keeps the caret where the user is
//! typing, and that a click on Submit still lands when an error appears above
//! it on the way.
//!
//! All `#[ignore]`d. Run `just e2e`.

use anyhow::Result;
use e2e::Session;
use e2e::focus::expect_focused;
use e2e::roles::{combobox, submit, textbox};
use playwright_rs::expect;
use playwright_rs::protocol::ClickOptions;

const TOO_SHORT: &str = "length must be at least 3";

// ── live-text ────────────────────────────────────────────────────────────

/// **A: nothing while typing a first time,** however wrong the value.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn nothing_is_shown_while_typing_a_first_time() -> Result<()> {
    let s = Session::open("/t/live-text").await?;
    let name = textbox(&s, "Name");
    name.press_sequentially("A", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    expect(s.page.locator("[aria-invalid]"))
        .to_have_count(0)
        .await?;
    s.close().await
}

/// **Leaving the field shows its error**, and Tab really is a blur.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn leaving_a_field_typed_into_shows_its_error() -> Result<()> {
    let s = Session::open("/t/live-text").await?;
    let name = textbox(&s, "Name");
    name.press_sequentially("A", None).await?;
    name.press("Tab", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text(TOO_SHORT)
        .await?;
    expect(name)
        .to_have_attribute("aria-invalid", "true")
        .await?;
    s.close().await
}

/// **The error goes on the keystroke that fixes it,** not on the next blur,
/// and the field then says it was checked and passed.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_error_goes_on_the_keystroke_that_fixes_it() -> Result<()> {
    let s = Session::open("/t/live-text").await?;
    let name = textbox(&s, "Name");
    name.press_sequentially("A", None).await?;
    name.press("Tab", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(1)
        .await?;

    name.click(None).await?;
    name.press("End", None).await?;
    name.press_sequentially("d", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text(TOO_SHORT)
        .await?;
    name.press_sequentially("a", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    expect(name)
        .to_have_attribute("aria-invalid", "false")
        .await?;
    s.close().await
}

/// **E: tabbing through an empty required field says nothing.**
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn tabbing_through_without_typing_says_nothing() -> Result<()> {
    let s = Session::open("/t/live-text").await?;
    let name = textbox(&s, "Name");
    name.click(None).await?;
    name.press("Tab", None).await?;
    expect_focused(&textbox(&s, "City")).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    s.close().await
}

/// **An error appearing does not take focus,** and one going does not lose
/// the caret. The error renders after Tab has moved focus on, so City must
/// still have it. Then, back in Name, the keystroke that clears the error
/// re-renders the field; the next keystroke must still land in it, at the end.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn focus_and_the_caret_survive_an_error_coming_and_going() -> Result<()> {
    let s = Session::open("/t/live-text").await?;
    let name = textbox(&s, "Name");
    name.press_sequentially("Ad", None).await?;
    name.press("Tab", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(1)
        .await?;
    expect_focused(&textbox(&s, "City")).await?;

    name.click(None).await?;
    name.press("End", None).await?;
    name.press_sequentially("a", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    // No click: the caret has to have survived the re-render.
    s.page.keyboard().press("m", None).await?;
    expect(name.clone()).to_have_value("Adam").await?;
    expect_focused(&name).await?;
    s.close().await
}

// ── live-click (trap 1) ──────────────────────────────────────────────────

/// The control for the next test: the same slow click with nothing typed, so
/// no error appears on the way and nothing moves. One `submit` arrives.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_slow_click_on_submit_lands_when_nothing_moves() -> Result<()> {
    let s = Session::open("/t/live-click").await?;
    submit(&s)
        .click(Some(ClickOptions::builder().delay(150.0).build()))
        .await?;
    expect(s.page.locator("#submits")).to_have_text("1").await?;
    s.close().await
}

/// **Trap 1 bites (found 2026-11-09), pinned until it is decided.** Submit
/// clicked at a human pace, 150 ms between press and release, with a bad
/// value in the field just above it. The press blurs the field, its error
/// renders between the field and the button, and the button moves down by
/// one line (29.6 px, measured, against a 38.8 px button: the same element,
/// not a replaced one). The release lands above it, so no `click` fires and
/// no `submit` happens. The user clicks again. Playwright's own instant click
/// misses too: its press and release are separate round trips, and the
/// render lands between them.
///
/// `#submits` is the evidence. The control above, the same click with
/// nothing moving, counts one. When this is fixed (one fix is reserving the
/// error line's space; `ERRORS_PLAN.md` box 6's trap 1 has the options), this
/// fails: rename it without `_yet` and assert `"1"`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_click_on_submit_misses_while_an_error_appears_above_it_yet() -> Result<()> {
    let s = Session::open("/t/live-click").await?;
    textbox(&s, "Name").press_sequentially("A", None).await?;
    submit(&s)
        .click(Some(ClickOptions::builder().delay(150.0).build()))
        .await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text(TOO_SHORT)
        .await?;
    expect(s.page.locator("#submits")).to_have_text("0").await?;
    s.close().await
}

// ── live-server-error (E) ────────────────────────────────────────────────

/// **E: a click in and out leaves a server's error alone.** Nothing was typed,
/// so nothing is checked.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_servers_error_survives_a_click_in_and_out() -> Result<()> {
    let s = Session::open("/t/live-server-error").await?;
    let errors = s.page.locator(".fx-field-error");
    expect(errors.clone())
        .to_have_text("That name is taken.")
        .await?;
    let name = textbox(&s, "Name");
    name.click(None).await?;
    name.press("Tab", None).await?;
    expect_focused(&textbox(&s, "City")).await?;
    expect(errors).to_have_text("That name is taken.").await?;
    s.close().await
}

// ── live-select ──────────────────────────────────────────────────────────

/// **A select's change is checked at once,** with focus still on it: a
/// chooser has no typing to wait out.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_selects_change_is_checked_at_once() -> Result<()> {
    let s = Session::open("/t/live-select").await?;
    let state = combobox(&s, "State");
    state.select_option("AK", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text("We do not ship to Alaska.")
        .await?;
    state.select_option("AL", None).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    expect(state)
        .to_have_attribute("aria-invalid", "false")
        .await?;
    s.close().await
}

// ── live-off ─────────────────────────────────────────────────────────────

/// **The knob off leaves every check to submit.** The control: the same steps
/// as `leaving_a_field_typed_into_shows_its_error`, and then Submit, which
/// does show it.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn with_it_off_only_submit_shows_an_error() -> Result<()> {
    let s = Session::open("/t/live-off").await?;
    let name = textbox(&s, "Name");
    name.press_sequentially("A", None).await?;
    name.press("Tab", None).await?;
    expect_focused(&textbox(&s, "City")).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_count(0)
        .await?;
    submit(&s).click(None).await?;
    expect(errors_of(&s, "name"))
        .to_have_text(TOO_SHORT)
        .await?;
    s.close().await
}

// ── live-custom ──────────────────────────────────────────────────────────

/// The error list a dev widget renders under its field, by the field's path:
/// its id is `{form id}-{path}-errors`, and the form id is random.
fn errors_of(s: &Session, path: &str) -> playwright_rs::Locator {
    s.page
        .locator(format!("[id$=\"-{path}-errors\"] .fx-field-error"))
}

/// **A dev widget using the handle with both handlers checks on leaving,**
/// like a built-in.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_dev_widget_with_the_handle_checks_on_leaving() -> Result<()> {
    let s = Session::open("/t/live-custom").await?;
    let handled = textbox(&s, "Handled");
    handled.press_sequentially("A", None).await?;
    handled.press("Tab", None).await?;
    expect(errors_of(&s, "handled"))
        .to_have_text("Too short.")
        .await?;
    s.close().await
}

/// **Without `onblur`, a dev widget is checked on submit, then on input.**
/// Leaving it says nothing, since no blur reaches the form. Submit validates
/// it like any field, and once it shows an error, `on_input` alone rechecks
/// it on the keystroke that fixes it.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_dev_widget_without_onblur_is_checked_on_submit_then_on_input() -> Result<()> {
    let s = Session::open("/t/live-custom").await?;
    let input_only = textbox(&s, "Input Only");
    input_only.press_sequentially("A", None).await?;
    input_only.press("Tab", None).await?;
    expect_focused(&submit(&s)).await?;
    expect(errors_of(&s, "input_only")).to_have_count(0).await?;

    submit(&s).click(None).await?;
    expect(errors_of(&s, "input_only"))
        .to_have_text("Too short.")
        .await?;

    input_only.click(None).await?;
    input_only.press("End", None).await?;
    input_only.press_sequentially("bc", None).await?;
    expect(errors_of(&s, "input_only")).to_have_count(0).await?;
    s.close().await
}
