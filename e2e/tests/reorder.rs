//! `/t/reorder`: a reorderable list's ↑/↓ buttons, from the browser's side.
//!
//! The suite proves the markup (which rows get buttons, which are disabled)
//! and `ListSet::move_row` proves the edit. Only a browser proves the click:
//! that each button sends the move it is named for, and that the order the
//! user sees is the order the model is built in.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{button, submit};
use playwright_rs::expect;

/// The rows' inputs, in the order the page shows them. They have no names of
/// their own (the list's legend is what labels them), so they are found by
/// position, inside the list's `<ol>`.
fn row_input(s: &Session, index: i32) -> playwright_rs::Locator {
    s.page.locator("ol.fx-list-rows > li input").nth(index)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn moving_rows_in_the_browser_reorders_the_model() -> Result<()> {
    let s = Session::open("/t/reorder").await?;
    for (i, song) in ["a", "b", "c"].into_iter().enumerate() {
        button(&s, "Add").click(None).await?;
        row_input(&s, i as i32).fill(song, None).await?;
    }

    // c up past b, then a down past c: a, b, c → a, c, b → c, a, b.
    button(&s, "Move row 3 up").click(None).await?;
    expect(row_input(&s, 1)).to_have_value("c").await?;
    button(&s, "Move row 1 down").click(None).await?;
    expect(row_input(&s, 0)).to_have_value("c").await?;
    expect(row_input(&s, 1)).to_have_value("a").await?;
    expect(row_input(&s, 2)).to_have_value("b").await?;

    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"songs: [ "c", "a", "b", ]"#)
        .await?;
    s.close().await
}

/// Disabled at the ends: the suite checks the attribute, this that a browser
/// honours it, so a click on the first row's ↑ moves nothing.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_end_rows_cannot_move_past_the_ends() -> Result<()> {
    let s = Session::open("/t/reorder").await?;
    for (i, song) in ["a", "b"].into_iter().enumerate() {
        button(&s, "Add").click(None).await?;
        row_input(&s, i as i32).fill(song, None).await?;
    }
    expect(button(&s, "Move row 1 up")).to_be_disabled().await?;
    expect(button(&s, "Move row 2 down"))
        .to_be_disabled()
        .await?;
    expect(button(&s, "Move row 1 down"))
        .to_be_enabled()
        .await?;
    s.close().await
}
