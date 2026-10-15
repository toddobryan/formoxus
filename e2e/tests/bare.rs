//! `/t/bare` — a form under `Classes::bare()`.
//!
//! The SSR suite already shows a `bare()` form's first render has no `class`
//! anywhere. What only a browser shows is the render AFTER an interaction:
//! the error list a failed submit adds, and the `aria-invalid` it sets, come
//! from a client-side re-render, which must look the classes up the same way.
//! And a form with no classes must still work, located by accessible name.
//!
//! All `#[ignore]`d — they need a running server and a browser. Run `just e2e`.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{submit, textbox};
use playwright_rs::expect;

/// Every element inside `main` that carries a class, except the harness's
/// own `.note` and `.submitted`: so, everything formoxus rendered.
const FORMOXUS_CLASSED: &str = "main [class]:not(.note):not(.submitted)";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_bare_form_has_no_classes_before_or_after_a_failed_submit() -> Result<()> {
    let s = Session::open("/t/bare").await?;
    expect(textbox(&s, "Name")).to_be_visible().await?;
    expect(s.page.locator(FORMOXUS_CLASSED))
        .to_have_count(0)
        .await?;

    submit(&s).click(None).await?;
    // The re-render happened: formoxus's own error is on the page, and the
    // field is marked invalid, with still no class anywhere.
    expect(s.page.get_by_text("This field is required.", true))
        .to_be_visible()
        .await?;
    expect(textbox(&s, "Name"))
        .to_have_attribute("aria-invalid", "true")
        .await?;
    expect(s.page.locator(FORMOXUS_CLASSED))
        .to_have_count(0)
        .await?;
    expect(s.page.locator("#submitted"))
        .to_have_text("")
        .await?;
    s.close().await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_bare_form_still_submits() -> Result<()> {
    let s = Session::open("/t/bare").await?;
    textbox(&s, "Name").fill("Ada", None).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("name: \"Ada\"")
        .await?;
    s.close().await
}
