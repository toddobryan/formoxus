//! Where the tests point, read from the environment with a default.
//!
//! Just the one knob. apcsp's equivalent also carries a username and password;
//! the gallery has no accounts, so there is nothing else to configure.

/// The running app under test. `just e2e` serves on this address; override with
/// `E2E_BASE_URL` to point at something already running.
pub fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

/// Whether to show the browser instead of running it headless.
///
/// Set `E2E_HEADED=1` to watch the tests drive a real window. Worth pairing with
/// `E2E_SLOW_MO` — headed but full speed is mostly a flicker — and with
/// `--test-threads=1`, or every test opens its own window at once. `just e2e-watch`
/// does all three.
pub fn headed() -> bool {
    matches!(
        std::env::var("E2E_HEADED").as_deref(),
        Ok("1" | "true" | "yes")
    )
}

/// Milliseconds to pause before each Playwright action, from `E2E_SLOW_MO`.
///
/// Playwright's own `slowMo`, so it slows the ACTIONS rather than sleeping
/// between tests. 0 (the default) disables it. Note it multiplies with every
/// action, so a whole suite at 500ms takes minutes — it is for watching one test.
pub fn slow_mo() -> f64 {
    std::env::var("E2E_SLOW_MO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}
