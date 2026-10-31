//! `/t/adder`: a custom adder, from the browser's side.
//!
//! The suite proves the adder renders in place of Add and that its `AddRow`
//! reaches `to_wire`. Only a browser proves the click lands, and what focus
//! does: a blank row takes it, but a filled row leaves it on the adder, so a
//! user picking several songs can keep clicking.

use anyhow::Result;
use e2e::Session;
use e2e::focus::expect_focused;
use e2e::roles::{button, submit};
use playwright_rs::expect;

/// The rows' inputs, in the order the page shows them, found by position:
/// they have no names of their own (the list's legend labels them).
fn row_input(s: &Session, index: i32) -> playwright_rs::Locator {
    s.page.locator(".fx-list-row input").nth(index)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_picked_song_arrives_as_a_filled_row() -> Result<()> {
    let s = Session::open("/t/adder").await?;
    expect(s.page.locator(".fx-button-add-row"))
        .to_have_count(0)
        .await?;
    button(&s, "Add Bravo").click(None).await?;
    button(&s, "Add Alpha").click(None).await?;
    expect(row_input(&s, 0)).to_have_value("Bravo").await?;
    expect(row_input(&s, 1)).to_have_value("Alpha").await?;

    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"songs: [ "Bravo", "Alpha", ]"#)
        .await?;
    s.close().await
}

/// `use_form` focuses only a blank row. Checked once the row is on the page,
/// since the focusing effect runs right after that render.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_filled_row_leaves_focus_on_the_adder() -> Result<()> {
    let s = Session::open("/t/adder").await?;
    button(&s, "Add Charlie").click(None).await?;
    expect(row_input(&s, 0)).to_have_value("Charlie").await?;
    expect_focused(&button(&s, "Add Charlie")).await?;
    s.close().await
}
