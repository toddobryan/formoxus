//! `/t/buttons` — reset and submit over a seeded form.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{submit, textbox};
use playwright_rs::expect;

/// Reset puts the SEEDED value back, not an empty field. The form is built with
/// `form_for`, so "Ada" is what it started as.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn reset_restores_the_seeded_value() -> Result<()> {
    let s = Session::open("/t/buttons").await?;
    let field = textbox(&s, "Name");
    expect(field.clone()).to_have_value("Ada").await?;

    field.fill("Grace", None).await?;
    expect(field.clone()).to_have_value("Grace").await?;

    s.page.locator("button[type=\"reset\"]").click(None).await?;
    expect(field).to_have_value("Ada").await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn submit_records_the_edited_model() -> Result<()> {
    let s = Session::open("/t/buttons").await?;
    textbox(&s, "Name").fill("Grace", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("name: \"Grace\"")
        .await?;
    s.close().await
}
