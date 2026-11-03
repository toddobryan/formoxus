//! `/t/tags` and `/t/tags-pattern`: a set, from the browser's side
//! (`SETS_PLAN.md` box 10).
//!
//! The suite proves what the set renders and what its edits do to the state
//! and the store. Only a browser proves the things these check: that Enter in
//! the draft adds without submitting the form, where focus lands after Add,
//! Edit, Replace and Cancel, and what `{:#?}` of the submitted set reads.

use anyhow::Result;
use e2e::Session;
use e2e::focus::expect_focused;
use e2e::roles::{button, submit, textbox};
use playwright_rs::{Locator, expect};

const DRAFT: &str = "Add to Tags";

fn draft(s: &Session) -> Locator {
    textbox(s, DRAFT)
}

fn items(s: &Session) -> Locator {
    s.page.locator(".fx-set-item")
}

fn message(s: &Session) -> Locator {
    s.page.locator(".fx-set-message")
}

/// Type `tag` into the draft and click Add, then wait for it to be an item.
async fn add(s: &Session, tag: &str) -> Result<()> {
    draft(s).fill(tag, None).await?;
    button(s, "Add").click(None).await?;
    expect(s.page.locator(format!(".fx-set-item:has-text(\"{tag}\")")))
        .to_have_count(1)
        .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn add_makes_the_draft_an_item_and_says_so() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    expect(message(&s))
        .to_have_text("“rust” was added successfully.")
        .await?;
    expect(draft(&s)).to_have_value("").await?;
    expect_focused(&draft(&s)).await?;
    s.close().await
}

/// Enter in a text box inside a `<form>` would submit it; in the draft it
/// adds instead (`DraftAdder`'s `prevent_default`).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn enter_in_the_draft_adds_and_does_not_submit() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    draft(&s).fill("rust", None).await?;
    draft(&s).press("Enter", None).await?;
    expect(items(&s)).to_have_count(1).await?;
    expect(items(&s)).to_contain_text("rust").await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    expect_focused(&draft(&s)).await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_duplicate_is_refused_with_a_message() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    draft(&s).fill("rust", None).await?;
    button(&s, "Add").click(None).await?;
    expect(s.page.locator(".fx-set-message.fx-invalid"))
        .to_have_text("“rust” is already there.")
        .await?;
    expect(items(&s)).to_have_count(1).await?;
    expect(draft(&s)).to_have_value("rust").await?;
    s.close().await
}

/// Added out of order, submitted sorted: a `BTreeSet`, and the set's own
/// order on the page.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn the_set_is_shown_and_submitted_in_order() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    for tag in ["go", "rust", "dioxus"] {
        add(&s, tag).await?;
    }
    expect(items(&s).nth(0)).to_contain_text("dioxus").await?;
    expect(items(&s).nth(2)).to_contain_text("rust").await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"tags: { "dioxus", "go", "rust", }"#)
        .await?;
    s.close().await
}

/// Text typed into the draft and never added is not in the set (box 7).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn text_left_in_the_draft_is_not_submitted() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    draft(&s).fill("go", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"tags: { "rust", }"#)
        .await?;
    s.close().await
}

/// Edit puts the item into the draft and focus there; Replace replaces it.
/// Named for the item it replaces, so it never shares a name with the test
/// form's own submit button, `save`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn edit_then_save_replaces_the_item() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    button(&s, "Edit “rust”").click(None).await?;
    expect(draft(&s)).to_have_value("rust").await?;
    expect_focused(&draft(&s))
        .await
        .map_err(|e| e.context("after Edit"))?;
    draft(&s).fill("go", None).await?;
    button(&s, "Replace “rust”").click(None).await?;
    expect(message(&s))
        .to_have_text("“rust” was changed to “go” successfully.")
        .await?;
    expect(items(&s)).to_have_count(1).await?;
    expect(items(&s)).to_contain_text("go").await?;
    expect(draft(&s)).to_have_value("").await?;
    expect_focused(&draft(&s))
        .await
        .map_err(|e| e.context("after Replace"))?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn edit_then_cancel_leaves_the_item() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    button(&s, "Edit “rust”").click(None).await?;
    draft(&s).fill("go", None).await?;
    button(&s, "Cancel").click(None).await?;
    expect(items(&s)).to_contain_text("rust").await?;
    expect(draft(&s)).to_have_value("").await?;
    expect(button(&s, "Add")).to_have_count(1).await?;
    expect_focused(&draft(&s)).await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn escape_in_the_draft_cancels_an_edit() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    button(&s, "Edit “rust”").click(None).await?;
    draft(&s).fill("go", None).await?;
    draft(&s).press("Escape", None).await?;
    expect(items(&s)).to_contain_text("rust").await?;
    expect(draft(&s)).to_have_value("").await?;
    expect(button(&s, "Add")).to_have_count(1).await?;
    s.close().await
}

/// The item stays in the set until Replace, so a submit while editing loses
/// nothing (decision E).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_submit_while_editing_keeps_the_item() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    button(&s, "Edit “rust”").click(None).await?;
    draft(&s).fill("go", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"tags: { "rust", }"#)
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn remove_is_named_by_its_item() -> Result<()> {
    let s = Session::open("/t/tags").await?;
    add(&s, "rust").await?;
    button(&s, "Remove “rust”").click(None).await?;
    expect(items(&s)).to_have_count(0).await?;
    expect(message(&s))
        .to_have_text("“rust” was removed successfully.")
        .await?;
    s.close().await
}

/// Box 7's gap, closed by box 14: text left in the draft is not in the set,
/// so the browser must not block a submit over it. The draft's input carries
/// none of its items' constraints, so "Rust" under `pattern: "[a-z]+"` goes
/// through, and the set stays empty.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_draft_breaking_its_pattern_does_not_block_the_submit() -> Result<()> {
    let s = Session::open("/t/tags-pattern").await?;
    draft(&s).fill("Rust", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("tags: {}")
        .await?;
    s.close().await
}

/// The constraint still applies where it belongs: Add refuses "Rust", with
/// formoxus's own error on the draft, and keeps the text to fix.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn add_refuses_a_draft_breaking_its_pattern() -> Result<()> {
    let s = Session::open("/t/tags-pattern").await?;
    draft(&s).fill("Rust", None).await?;
    button(&s, "Add").click(None).await?;
    expect(s.page.locator(".fx-set-adder .fx-field-error"))
        .to_have_count(1)
        .await?;
    expect(items(&s)).to_have_count(0).await?;
    expect(draft(&s)).to_have_value("Rust").await?;
    s.close().await
}
