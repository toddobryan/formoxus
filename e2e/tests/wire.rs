//! `/t/enum-submit`: issue #13 seen from a real browser.
//!
//! The suite proves the wire with `Harness`, which fires synthetic events at a
//! `VirtualDom`. Only a browser proves that the variant `<select>`'s real
//! `onchange` lands in the form's state, and that what `to_wire` then sends is
//! what a server rebuilds. The test form runs `Submission::accept` in the page,
//! so `#accepted` is the server's answer to the same payload.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{combobox, submit, textbox};
use playwright_rs::expect;

/// Box 23: a variant chosen and filled in the browser reaches the server.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_variant_chosen_in_the_browser_reaches_the_server() -> Result<()> {
    let s = Session::open("/t/enum-submit").await?;
    combobox(&s, "Footprint")
        .select_option("Circle", None)
        .await?;
    textbox(&s, "Radius").fill("2.5", None).await?;
    submit(&s).click(None).await?;

    let accepted = s.page.locator("#accepted");
    expect(accepted.clone()).to_contain_text("Circle").await?;
    expect(accepted).to_contain_text("radius: 2.5").await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("radius: 2.5")
        .await?;
    s.close().await
}

/// Box 21, decision B: distributing the variant already chosen is a no-op, so
/// validating again keeps both the choice and what was typed into its field.
/// Then the same after SWITCHING variants, where a stale choice in the store
/// (if anything read it) would send the old variant.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn validating_twice_keeps_the_choice_and_what_was_typed() -> Result<()> {
    let s = Session::open("/t/enum-submit").await?;
    let footprint = combobox(&s, "Footprint");
    let submits = s.page.locator("#submits");
    let accepted = s.page.locator("#accepted");

    footprint.select_option("Circle", None).await?;
    textbox(&s, "Radius").fill("2.5", None).await?;
    submit(&s).click(None).await?;
    expect(submits.clone()).to_have_text("1").await?;
    submit(&s).click(None).await?;
    expect(submits.clone()).to_have_text("2").await?;

    expect(footprint.clone()).to_have_value("Circle").await?;
    expect(textbox(&s, "Radius")).to_have_value("2.5").await?;
    expect(accepted.clone())
        .to_contain_text("radius: 2.5")
        .await?;

    footprint.select_option("Square", None).await?;
    textbox(&s, "Side").fill("4", None).await?;
    submit(&s).click(None).await?;
    expect(submits.clone()).to_have_text("3").await?;
    submit(&s).click(None).await?;
    expect(submits).to_have_text("4").await?;

    expect(footprint).to_have_value("Square").await?;
    expect(textbox(&s, "Side")).to_have_value("4").await?;
    expect(accepted.clone()).to_contain_text("Square").await?;
    expect(accepted).to_contain_text("side: 4.0").await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("side: 4.0")
        .await?;
    s.close().await
}
