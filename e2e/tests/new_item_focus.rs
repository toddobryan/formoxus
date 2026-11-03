//! An item the Add button makes takes focus, from the browser's side.
//!
//! Focus is a browser fact: the suite renders to a string and has no
//! `document`, so `use_form`'s focusing effect does nothing there. With Add
//! below the items the new item appears beside the button anyway, so this is
//! mostly for a screen reader, where focus is what says the item arrived.

use anyhow::Result;
use e2e::Session;
use e2e::focus::expect_focused;
use e2e::roles::{button, textbox};

/// A leaf item's input, by position: its items' inputs have no names of their
/// own (the list's legend labels them).
fn leaf_item_input(s: &Session, index: i32) -> playwright_rs::Locator {
    s.page.locator("ol.fx-list-items > li input").nth(index)
}

/// Each new item, not the first one again. An item starts with its
/// ↑/↓ buttons, and the second item's ↑ is enabled, so landing on the input
/// shows those buttons are skipped (they carry no `name`).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_item_added_with_add_takes_focus() -> Result<()> {
    let s = Session::open("/t/reorder").await?;
    button(&s, "Add").click(None).await?;
    expect_focused(&leaf_item_input(&s, 0)).await?;
    leaf_item_input(&s, 0).fill("a", None).await?;

    button(&s, "Add").click(None).await?;
    expect_focused(&leaf_item_input(&s, 1)).await?;
    s.close().await
}

/// A struct item takes focus at its first field, in each new item.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_struct_item_takes_focus_at_its_first_field() -> Result<()> {
    let s = Session::open("/t/struct-items").await?;
    button(&s, "Add").click(None).await?;
    expect_focused(&textbox(&s, "Name").nth(0)).await?;
    textbox(&s, "Name").nth(0).fill("Ada", None).await?;

    button(&s, "Add").click(None).await?;
    expect_focused(&textbox(&s, "Name").nth(1)).await?;
    s.close().await
}
