//! THROWAWAY: WIDGETS_PLAN box 10's spike, driving `/t/spike-upload`.
//!
//! Each test prints the server's report; the findings are read off the output
//! (`--no-capture`), not asserted. Run serially, or the RSS numbers mix.

use anyhow::Result;
use e2e::Session;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn file_of(name: &str, bytes: usize) -> PathBuf {
    let dir = std::env::temp_dir().join("fx-spike");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    if std::fs::metadata(&p).map(|m| m.len() as usize).ok() != Some(bytes) {
        let data: Vec<u8> = (0..bytes).map(|i| b'a' + (i % 26) as u8).collect();
        std::fs::write(&p, data).unwrap();
    }
    p
}

async fn run(file_input: &str, file: PathBuf, button: &str) -> Result<()> {
    let s = Session::open("/t/spike-upload").await?;
    s.page
        .locator(file_input)
        .set_input_files(&file, None)
        .await?;
    let start = Instant::now();
    s.page.locator(button).click(None).await?;
    // Wait for the full report, not just its first line.
    let mut text = String::new();
    for _ in 0..600 {
        text = s.page.locator("#report").inner_text().await?;
        if text.contains("rss_peak") || text.contains("CLIENT_ERROR") || text.contains("NO FILE") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    eprintln!(
        "\n===== {button} with {} ({:?}) =====\n{text}",
        file.display(),
        start.elapsed()
    );
    s.close().await
}

macro_rules! spike {
    ($name:ident, $input:literal, $file:literal, $bytes:expr, $button:literal) => {
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "e2e: needs `just e2e` (a served app + a Playwright browser)"]
        async fn $name() -> Result<()> {
            run($input, file_of($file, $bytes), $button).await
        }
    };
}

const SMALL: usize = 100;
const MB3: usize = 3 * 1024 * 1024;
const MB20: usize = 20 * 1024 * 1024;

spike!(
    spike_01_event_before,
    "#file-event-before",
    "small.txt",
    SMALL,
    "#send-event-before"
);
spike!(
    spike_02_event_after,
    "#file-event-after",
    "small.txt",
    SMALL,
    "#send-event-after"
);
spike!(
    spike_03_hand_wire_first,
    "#file-hand",
    "small.txt",
    SMALL,
    "#hand-wire-first"
);
spike!(
    spike_04_hand_file_first,
    "#file-hand",
    "small.txt",
    SMALL,
    "#hand-file-first"
);
spike!(
    spike_05_event_3mb_default_limit,
    "#file-event-before",
    "three.bin",
    MB3,
    "#send-event-before"
);
spike!(
    spike_06_hand_3mb_default_limit,
    "#file-hand",
    "three.bin",
    MB3,
    "#hand-wire-first"
);
spike!(
    spike_07_hand_20mb_raised_stream,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-big"
);
spike!(
    spike_08_hand_20mb_raised_buffer,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-big-buffer"
);
spike!(
    spike_09_hand_20mb_raised_stream_again,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-big"
);
spike!(
    spike_10_hand_20mb_server_stops,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-stop"
);
spike!(
    spike_11_again,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-stop"
);
spike!(
    spike_12_again,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-stop"
);
spike!(
    spike_13_stream,
    "#file-hand",
    "twenty.bin",
    MB20,
    "#hand-big"
);
