//! One browser, on one page, for one test.

use anyhow::Result;
use playwright_rs::{Browser, Page, Playwright, expect};

use crate::config::{base_url, headed, slow_mo};

/// A browser open on one page.
///
/// **Why this is a struct rather than three lines in each test.** [`Playwright`]
/// has a `Drop` impl that shuts the driver down and takes its browsers with it,
/// so the driver and the browser have to outlive the page — write
/// `Playwright::launch().await?.chromium().launch().await?.new_page().await?`
/// and the temporaries drop at the end of the statement, killing the browser you
/// just got a page from. Holding all three in one value is what keeps the page
/// alive.
///
/// It also gives the hydration wait exactly one home. Skipping that wait is the
/// flaky failure this whole harness exists to avoid (see [`Session::open`]), and
/// it is not something to rely on remembering once per test.
///
/// Set `E2E_HEADED=1` to watch it happen in a real window, and `E2E_SLOW_MO` to
/// slow the actions down enough to follow — `just e2e-watch` does both, serially.
///
/// **One page per test, by design.** Every test form has its own URL, so a test
/// navigates once and never again — which is why there is no separate `goto`.
/// Each test gets a fresh browser, so nothing leaks between them either.
pub struct Session {
    _pw: Playwright,
    browser: Browser,
    pub page: Page,
}

impl Session {
    /// Launch a browser, navigate to `path` relative to [`base_url`], and wait
    /// for the client to hydrate.
    ///
    /// **The hydration wait is load-bearing.** The app renders a hidden
    /// `#app-ready` element from a post-mount effect, and only when built with
    /// `--features e2e-testing`. Without waiting for it, a click can land on
    /// server-rendered markup that Dioxus has not attached handlers to yet — and
    /// a submit button in that state triggers a native form GET instead of the
    /// `onsubmit` handler, which looks like a formoxus bug and is not one.
    pub async fn open(path: &str) -> Result<Self> {
        let pw = Playwright::launch().await?;
        // `LaunchOptions` is `#[non_exhaustive]`, so it is built from `default()`
        // rather than with a struct literal.
        let mut opts = playwright_rs::LaunchOptions::default();
        opts.headless = Some(!headed());
        if slow_mo() > 0.0 {
            opts.slow_mo = Some(slow_mo());
        }
        let browser = pw.chromium().launch_with_options(opts).await?;
        let page = browser.new_page().await?;
        page.goto(&format!("{}{}", base_url(), path), None).await?;
        expect(page.locator("#app-ready")).to_have_count(1).await?;
        Ok(Self {
            _pw: pw,
            browser,
            page,
        })
    }

    /// The model a test form last validated, as pretty-printed `Debug`.
    ///
    /// Every test form renders this into `#submitted` — see
    /// `examples/src/test_forms.rs`. Empty before the first successful submit.
    pub async fn submitted(&self) -> Result<String> {
        Ok(self.page.locator("#submitted").inner_text().await?)
    }

    pub async fn close(self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}
