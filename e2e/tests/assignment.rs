//! `/t/assignment`: apcsp's shape, a list inside an enum inside a list.
//!
//! The suite (`assignment_shape.rs`) proves the edits by path. Only a
//! browser proves that a click on a nested list's ↑ or Add lands on THAT
//! list, when the outer list and its siblings have buttons of the same name.

use anyhow::Result;
use e2e::Session;
use playwright_rs::Locator;
use playwright_rs::expect;
use playwright_rs::protocol::locator::{AriaRole, GetByRoleOptions};

/// The list whose own legend is `legend`. Not one of its ancestors, which
/// contain that legend too.
fn list(s: &Session, legend: &str) -> Locator {
    s.page.locator(format!(
        "fieldset.fx-list:has(> legend:text-is(\"{legend}\"))"
    ))
}

/// One of the list's OWN item buttons (↑, ↓, Remove), by its `aria-label`:
/// never a nested list's, whose items carry buttons of the same names.
fn item_button(s: &Session, legend: &str, label: &str) -> Locator {
    let item = ":scope > ol.fx-list-items > li";
    list(s, legend).locator(format!(
        "{item} > .fx-move-buttons > button[aria-label=\"{label}\"], \
         {item} > button[aria-label=\"{label}\"]"
    ))
}

/// A button inside a list with no list nested in it, by accessible name.
fn leaf_list_button(s: &Session, legend: &str, name: &str) -> Locator {
    let mut o = GetByRoleOptions::default();
    o.name = Some(name.to_string());
    o.exact = Some(true);
    list(s, legend).get_by_role(AriaRole::Button, Some(o))
}

/// The item inputs of the list whose legend is `legend`, in page order: its
/// own items only, not a nested list's.
fn item_input(s: &Session, legend: &str, index: i32) -> Locator {
    list(s, legend)
        .locator(":scope > ol.fx-list-items > li input")
        .nth(index)
}

/// ↓ on the All group's first question, then ↓ on the first GROUP: each
/// click moves only its own list, and the model is built in the order shown.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn each_lists_move_buttons_move_only_that_list() -> Result<()> {
    let s = Session::open("/t/assignment").await?;
    item_button(&s, "Questions", "Move item 1 down")
        .click(None)
        .await?;
    expect(item_input(&s, "Questions", 0))
        .to_have_value("q2")
        .await?;

    item_button(&s, "Groups", "Move item 1 down")
        .click(None)
        .await?;
    s.page
        .locator("button[type=\"submit\"]")
        .click(None)
        .await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("Random {")
        .await?;
    let model = s
        .page
        .locator("#submitted")
        .text_content()
        .await?
        .unwrap_or_default();
    let random = model.find("Random").expect("the Random group is submitted");
    let all = model.find("All").expect("the All group is submitted");
    assert!(random < all, "the groups swapped: {model}");
    let q2 = model.find("\"q2\"").expect("q2 is submitted");
    let q1 = model.find("\"q1\"").expect("q1 is submitted");
    assert!(q2 < q1, "q1 moved below q2 within its group: {model}");
    s.close().await
}

/// Add inside the Random group's pool appends there, and its value reaches
/// the model with the pool's others.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_nested_lists_add_appends_to_that_list() -> Result<()> {
    let s = Session::open("/t/assignment").await?;
    leaf_list_button(&s, "Set", "Add").click(None).await?;
    item_input(&s, "Set", 2).fill("q9", None).await?;
    s.page
        .locator("button[type=\"submit\"]")
        .click(None)
        .await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text(r#"set: [ "q4", "q5", "q9", ]"#)
        .await?;
    s.close().await
}

/// **The variant picker is found by `name`** (#17): its enum's path, here an
/// item of the outer list. Switching it in the browser swaps that group's
/// fields and leaves the other group alone.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_groups_variant_picker_is_found_by_its_name() -> Result<()> {
    let s = Session::open("/t/assignment").await?;
    let first = s.page.locator("select[name=\"groups.#0\"]");
    expect(first.clone()).to_have_value("All").await?;
    expect(s.page.locator("select[name=\"groups.#1\"]"))
        .to_have_value("Random")
        .await?;

    first.select_option("Random", None).await?;
    expect(list(&s, "Set")).to_have_count(2).await?;
    s.close().await
}
