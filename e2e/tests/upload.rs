//! A file upload end to end (`WIDGETS_PLAN.md` box 13): `/t/upload`,
//! `/t/upload-unlimited`, `/t/upload-server-limit` and `/t/upload-big`.
//!
//! The suite builds the multipart request by hand (`accept_multipart.rs`) and
//! renders the widget to a string (`file_input.rs`). Only a browser shows that
//! a real `<input type="file">` pick reaches `to_multipart`, that the
//! browser's own `FormData` carries the bytes to the server function, that the
//! server stores them whole, and what the user sees when the request is past
//! the server's body limit: the one trap box 11 left unmeasured.
//!
//! Each page sends to a server function that reports what it stored, read
//! back from the temporary file, in `#report`.
//!
//! All `#[ignore]`d. Run `just e2e`.

use std::path::PathBuf;

use anyhow::Result;
use e2e::Session;
use e2e::roles::submit;
use playwright_rs::expect;

/// A file of `bytes` bytes named `name`, in a directory of this test file's
/// own, made once. `a`-to-`z` over and over, so its start is predictable.
fn file_of(name: &str, bytes: usize) -> PathBuf {
    let dir = std::env::temp_dir().join("formoxus-e2e-upload");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let path = dir.join(name);
    if std::fs::metadata(&path).map(|m| m.len() as usize).ok() != Some(bytes) {
        let data: Vec<u8> = (0..bytes).map(|i| b'a' + (i % 26) as u8).collect();
        std::fs::write(&path, data).expect("a scratch file");
    }
    path
}

async fn pick(s: &Session, file: &PathBuf) -> Result<()> {
    s.page
        .locator("input[type=\"file\"]")
        .set_input_files(file, None)
        .await?;
    Ok(())
}

// ── upload: 1 kB, .txt ───────────────────────────────────────────────────

/// **A picked file reaches the server whole.** Name, size and bytes are the
/// server's own report, read back from the file it stored.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_picked_file_is_stored_whole_on_the_server() -> Result<()> {
    let s = Session::open("/t/upload").await?;
    pick(&s, &file_of("essay.txt", 500)).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report"))
        .to_have_text("received essay.txt, 500 bytes, 500 on disk: abcdefghijklmnopqrst")
        .await?;
    s.close().await
}

/// **A file over `max_size` is refused at the pick, and never sent.** The
/// error is the field's; the server is never asked, so `#report` stays empty.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_file_over_max_size_is_refused_before_sending() -> Result<()> {
    let s = Session::open("/t/upload").await?;
    pick(&s, &file_of("long.txt", 2_000)).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text("This file is too large. The largest allowed is 1 kB.")
        .await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report")).to_have_text("").await?;
    s.close().await
}

/// **A type `accept` doesn't list is refused at the pick, and never sent.**
/// `set_input_files` ignores `accept`, as a user choosing "All files" in the
/// picker does, so this is the check behind the attribute.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_type_accept_doesnt_list_is_refused_before_sending() -> Result<()> {
    let s = Session::open("/t/upload").await?;
    pick(&s, &file_of("essay.pdf", 100)).await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text("This file type isn't allowed. Allowed: .txt.")
        .await?;
    expect(s.page.locator("input[type=\"file\"]"))
        .to_have_attribute("accept", ".txt")
        .await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report")).to_have_text("").await?;
    s.close().await
}

// ── upload-unlimited: only the 2 MB body limit ───────────────────────────

/// **Past the body limit, the user still gets an answer** (trap 12, and the
/// trap h.8 left to box 13): the form-level "too large to send together",
/// absorbed from the server's reply, and nothing stored.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn past_the_body_limit_the_form_says_so() -> Result<()> {
    let s = Session::open("/t/upload-unlimited").await?;
    pick(&s, &file_of("big.txt", 3 * 1024 * 1024)).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report"))
        .to_have_text("rejected")
        .await?;
    expect(s.page.locator(".fx-form-error"))
        .to_have_text("These files are too large to send together.")
        .await?;
    s.close().await
}

// ── upload-server-limit: 1 kB on the server only ─────────────────────────

/// **The server's own `max_size` refusal still reaches the user** (box 10
/// (d)): the server stops storing at 1 kB, reads the rest of the 20 MB file
/// anyway, and only then answers, with the field's error. 20 MB, as in box
/// 10's spike, so the browser is still sending when the server has its
/// answer: a smaller file is all sent first, and passes without the drain.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_file_the_server_refuses_partway_still_gets_an_answer() -> Result<()> {
    let s = Session::open("/t/upload-server-limit").await?;
    pick(&s, &file_of("huge.txt", 20 * 1024 * 1024)).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report"))
        .to_have_text("rejected")
        .await?;
    expect(s.page.locator(".fx-field-error"))
        .to_have_text("This file is too large. The largest allowed is 1 kB.")
        .await?;
    s.close().await
}

// ── upload-big: the limit raised to 8 MB ─────────────────────────────────

/// **With the limit raised on the server function, a 3 MB file arrives
/// whole**, the fix `accept_multipart`'s docs give for the test above.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
async fn a_raised_body_limit_lets_a_large_file_through() -> Result<()> {
    let s = Session::open("/t/upload-big").await?;
    pick(&s, &file_of("big.txt", 3 * 1024 * 1024)).await?;
    submit(&s).click(None).await?;
    expect(s.page.locator("#report"))
        .to_have_text("received big.txt, 3145728 bytes, 3145728 on disk: abcdefghijklmnopqrst")
        .await?;
    s.close().await
}
