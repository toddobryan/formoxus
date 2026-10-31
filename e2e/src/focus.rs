//! Asserting focus, which `playwright-rs` (0.15.1 through 0.20.1 at least) cannot do for most locators.
//!
//! Its `to_be_focused` and `is_focused` hand the locator's WHOLE selector to
//! `document.querySelectorAll`, so anything that is not plain CSS (a role
//! lookup, or any `nth()`) fails as an invalid selector instead of being
//! checked. `Locator::evaluate` resolves the locator the Playwright way, so
//! asking the element itself works for every locator.

use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use playwright_rs::Locator;

/// Wait until `locator`'s element has focus. Polls, like an `expect`, because
/// focus moves in an effect that runs after the render the click caused.
pub async fn expect_focused(locator: &Locator) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let focused: bool = locator
            .evaluate("el => el === document.activeElement", None::<()>)
            .await?;
        if focused {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("expected {} to be focused", locator.selector());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
