//! `/t/checkbox-multiple` and `/t/select-multiple` (`WIDGETS_PLAN.md` part 2).
//!
//! What the suite's rendered HTML cannot prove: that a real tick or a real
//! multi-select change lands as the set's edits, that a box locked at
//! `max_items` cannot be ticked, that `min_items` shows at once on the
//! untick, and that a multi-select's `required` blocks an empty submit.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{checkbox, listbox, submit};
use playwright_rs::expect;

// ── checkbox-multiple ────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn ticking_boxes_puts_their_days_in_the_set() -> Result<()> {
    let s = Session::open("/t/checkbox-multiple").await?;
    checkbox(&s, "Wed").check(None).await?;
    checkbox(&s, "Mon").check(None).await?;
    submit(&s).click(None).await?;
    // In declaration order, whatever order they were ticked in.
    expect(s.page.locator("#submitted"))
        .to_contain_text("days: { Mon, Wed, }")
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn unticking_a_box_takes_its_day_out() -> Result<()> {
    let s = Session::open("/t/checkbox-multiple").await?;
    checkbox(&s, "Mon").check(None).await?;
    checkbox(&s, "Tue").check(None).await?;
    checkbox(&s, "Mon").uncheck(None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("days: { Tue, }")
        .await?;
    s.close().await
}

/// At `max_items` the unticked boxes lock and the ticked ones stay live, so
/// unticking one frees the rest.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn at_max_items_the_unticked_box_locks_until_one_is_unticked() -> Result<()> {
    let s = Session::open("/t/checkbox-multiple").await?;
    checkbox(&s, "Mon").check(None).await?;
    checkbox(&s, "Tue").check(None).await?;
    expect(checkbox(&s, "Wed")).to_be_disabled().await?;
    expect(checkbox(&s, "Mon")).to_be_enabled().await?;
    checkbox(&s, "Mon").uncheck(None).await?;
    expect(checkbox(&s, "Wed")).to_be_enabled().await?;
    s.close().await
}

/// Unticking the last box under `min_items: 1` says so at once, with no
/// submit: the live re-check every Add and Remove gets. And no box carries
/// `required`, which on a checkbox would demand THAT box.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn unticking_the_last_box_says_so_at_once() -> Result<()> {
    let s = Session::open("/t/checkbox-multiple").await?;
    expect(s.page.locator("input[type=\"checkbox\"][required]"))
        .to_have_count(0)
        .await?;
    checkbox(&s, "Tue").check(None).await?;
    checkbox(&s, "Tue").uncheck(None).await?;
    expect(s.page.locator(".fx-field-errors"))
        .to_contain_text("Add at least 1 item.")
        .await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

// ── select-multiple ──────────────────────────────────────────────────────

/// One change selecting two options is one event the widget diffs into two
/// adds, and the model holds the codes, not the names.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn selecting_several_options_puts_their_codes_in_the_set() -> Result<()> {
    let s = Session::open("/t/select-multiple").await?;
    listbox(&s, "Cities")
        .select_option_multiple(&["rome", "paris"], None)
        .await?;
    submit(&s).click(None).await?;
    let out = s.page.locator("#submitted");
    expect(out.clone())
        .to_contain_text("cities: { \"paris\", \"rome\", }")
        .await?;
    expect(out).not().to_contain_text("Paris").await?;
    s.close().await
}

/// Two out and one in, as ONE change: the removes go highest index first,
/// or the second would miss.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn one_change_can_swap_several_options() -> Result<()> {
    let s = Session::open("/t/select-multiple").await?;
    let cities = listbox(&s, "Cities");
    cities
        .select_option_multiple(&["paris", "rome"], None)
        .await?;
    cities.select_option_multiple(&["oslo"], None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("cities: { \"oslo\", }")
        .await?;
    s.close().await
}

/// `min_items: 1` is the multi-select's `required`, which the browser
/// enforces: nothing selected, nothing submitted.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_empty_required_multi_select_is_blocked() -> Result<()> {
    let s = Session::open("/t/select-multiple").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}
