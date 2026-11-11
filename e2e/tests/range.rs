//! `/t/range`: `widget: range` from the browser's side (`WIDGETS_PLAN.md` box 3).
//!
//! The SSR suite proves where formoxus STARTS an empty range. Only a browser can
//! prove that is where the browser draws one (trap 1): given `value=""`, a range
//! shows HTML's default, and the form has to hold exactly that, or the user sees
//! one value and submits another. Playwright cannot `fill` a range, so a value is
//! changed with the keyboard, as a user without a mouse would.

use anyhow::Result;
use e2e::Session;
use e2e::roles::{slider, submit};
use playwright_rs::expect;

/// Each formoxus slider (by label) beside its bare reference (by id) and the
/// field's name in the submitted model.
const SLIDERS: [(&str, &str, &str); 4] = [
    ("Plain", "#ref-plain", "plain"),
    ("By three", "#ref-by_three", "by_three"),
    ("By four", "#ref-by_four", "by_four"),
    ("Tie", "#ref-tie", "tie"),
];

/// **An untouched range starts where the browser draws an empty one,** and that
/// is the value submitted. The reference inputs have no value, so theirs is
/// HTML's default; the assertion is that formoxus computed the same, for the
/// plain midpoint, a midpoint off the step either way, and a tie.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn an_untouched_range_starts_where_the_browser_draws_an_empty_one() -> Result<()> {
    let s = Session::open("/t/range").await?;
    let mut drawn = Vec::new();
    for (label, reference, _) in SLIDERS {
        let browsers = s.page.locator(reference).input_value(None).await?;
        let formoxus = slider(&s, label).input_value(None).await?;
        assert_eq!(
            formoxus, browsers,
            "{label}: formoxus starts at {formoxus}, the browser draws {browsers}"
        );
        drawn.push(browsers);
    }
    submit(&s).click(None).await?;
    for ((_, _, field), value) in SLIDERS.iter().zip(&drawn) {
        expect(s.page.locator("#submitted"))
            .to_contain_text(&format!("{field}: {value},"))
            .await?;
    }
    s.close().await
}

/// A keyboard step moves the slider, its `<output>` follows, and the moved value
/// reaches the model.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_moved_range_shows_and_submits_its_new_value() -> Result<()> {
    let s = Session::open("/t/range").await?;
    let plain = slider(&s, "Plain");
    plain.focus().await?;
    plain.press("ArrowRight", None).await?;
    expect(s.page.locator(r#"output[for$="-plain"]"#))
        .to_have_text("6")
        .await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#submitted"))
        .to_contain_text("plain: 6,")
        .await?;
    s.close().await
}
