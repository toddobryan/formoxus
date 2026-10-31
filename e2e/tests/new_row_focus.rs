//! A row the Add button makes takes focus, from the browser's side.
//!
//! Focus is a browser fact: the suite renders to a string and has no
//! `document`, so `use_form`'s focusing effect does nothing there. With Add
//! below the rows the new row appears beside the button anyway, so this is
//! mostly for a screen reader, where focus is what says the row arrived.

use anyhow::Result;
use e2e::Session;
use e2e::focus::expect_focused;
use e2e::roles::{button, textbox};

/// A leaf row's input, by position: its rows' inputs have no names of their
/// own (the list's legend labels them).
fn leaf_row_input(s: &Session, index: i32) -> playwright_rs::Locator {
    s.page.locator("ol.fx-list-rows > li input").nth(index)
}

/// Each new row, not the first one again. A reorderable row starts with its
/// ↑/↓ buttons, and the second row's ↑ is enabled, so landing on the input
/// shows those buttons are skipped (they carry no `name`).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_row_added_with_add_takes_focus() -> Result<()> {
    let s = Session::open("/t/reorder").await?;
    button(&s, "Add").click(None).await?;
    expect_focused(&leaf_row_input(&s, 0)).await?;
    leaf_row_input(&s, 0).fill("a", None).await?;

    button(&s, "Add").click(None).await?;
    expect_focused(&leaf_row_input(&s, 1)).await?;
    s.close().await
}

/// A struct row takes focus at its first field, in each new row.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_struct_row_takes_focus_at_its_first_field() -> Result<()> {
    let s = Session::open("/t/struct-rows").await?;
    button(&s, "Add").click(None).await?;
    expect_focused(&textbox(&s, "Name").nth(0)).await?;
    textbox(&s, "Name").nth(0).fill("Ada", None).await?;

    button(&s, "Add").click(None).await?;
    expect_focused(&textbox(&s, "Name").nth(1)).await?;
    s.close().await
}
