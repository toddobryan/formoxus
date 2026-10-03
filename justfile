# Serve the example gallery at http://localhost:8080 with hot reload.
serve:
    dx serve -p formoxus-examples

# Same, with dx's own log going to a file — useful when the terminal is busy.
serve-log:
    dx serve -p formoxus-examples --log-to-file /tmp/dx-formoxus.log

# Which (value kind, widget) pairs actually render. See
# .claude/memory/widget_table_and_choice.md for what the number means.
matrix:
    cargo run -p formoxus-examples --bin widget_matrix

# Everything CI runs, in CI's order, so a red build is reproducible locally.
ci: fmt clippy test docs msrv

fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# --workspace, not a bare `cargo test`: the examples crate is a member but not
# a DEFAULT member, and building it is the point of it being in the workspace.
test:
    cargo test --workspace

docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# The floor we claim in Cargo.toml and the README. Published crates only, and
# `check` not `test`: dev-dependencies are ours, not a consumer's.
msrv:
    cargo +1.90 check -p formoxus -p formoxus-attrs -p formoxus-macros

# Both builds the gallery has to satisfy — the browser one and the host one.
check-web:
    cargo check -p formoxus-examples --target wasm32-unknown-unknown --features web

check-server:
    cargo check -p formoxus-examples --features server

# Run the browser tests against a throwaway server, then kill it (and its
# children). Deliberately NOT part of `just ci`: it needs a browser, which CI
# does not have. Install one once with `npx playwright@1.60.0 install chromium`.
#
# `--features e2e-testing` is what renders the hidden `#app-ready` element every
# test waits for. Without it the tests hang on a marker that never appears.
#
# Filter to one test: `just e2e blocks_an_empty_required_field`.
e2e *args:
    #!/usr/bin/env bash
    set -euo pipefail
    # 1. Build in the FOREGROUND first, so a compile error fails fast with the
    #    real message and a non-zero exit. `dx serve` cannot do this — it is a
    #    watcher and stays up on error, which would leave us polling a server
    #    that is never going to arrive.
    echo "Building (--features e2e-testing)..."
    dx build -p formoxus-examples --features e2e-testing --force-sequential
    # 2. Serve the built app in the background. Only runtime failures remain.
    log="$(mktemp -t dx-e2e.XXXXXX.log)"
    echo "Serving in the background (logs: $log)"
    setsid dx serve -p formoxus-examples --features e2e-testing --force-sequential \
        >"$log" 2>&1 &
    server=$!
    # Kill the whole process GROUP on exit: dx spawns the server binary as a
    # child, and killing only dx would orphan it holding port 8080.
    trap 'kill -TERM -"$server" 2>/dev/null || kill "$server" 2>/dev/null || true' EXIT
    echo "Waiting for http://127.0.0.1:8080 ..."
    ready=
    for _ in $(seq 1 60); do
        # -f so curl fails on a 5xx: a server that came up broken is not ready.
        if curl -sf -o /dev/null http://127.0.0.1:8080; then ready=1; break; fi
        if ! kill -0 "$server" 2>/dev/null; then echo "dx serve exited early:"; tail -40 "$log"; exit 1; fi
        # Catches a failure dx logs but keeps running through, which the exit
        # check above would never see.
        if grep -qiE 'panicked|exited with error|could not compile|Build failed' "$log"; then
            echo "Server failed to start — last 40 lines of $log:"; tail -40 "$log"; exit 1
        fi
        sleep 2
    done
    [ -n "$ready" ] || { echo "server never came up — see $log"; tail -40 "$log"; exit 1; }
    cargo test -p e2e {{args}} -- --ignored

# Watch the browser tests run in a real window, one at a time and slowed down.
#
# Three things have to change together: headed, so there is a window; slowed, or
# it is a flicker; and SERIAL, or every test opens its own window at once.
# Slow-mo multiplies with every action, so this is for watching ONE test — pass a
# filter. `just e2e-watch` with no filter runs all of them and takes a while.
#
# Override the pace with E2E_SLOW_MO (milliseconds per action).
e2e-watch *args:
    # RUST_TEST_THREADS rather than `-- --test-threads=1`: the recipe below puts
    # `{{{{args}}}}` BEFORE its own `--`, so a runner flag there would be read by
    # cargo instead of by the test binary.
    E2E_HEADED=1 E2E_SLOW_MO="${E2E_SLOW_MO:-150}" RUST_TEST_THREADS=1 just e2e {{args}}

# Point git at the repo's committed hooks. Run once per clone.
#
# `.git/hooks/` is not part of the repository, so a hook left there exists on one
# machine only — which is how an unformatted commit reached main. `core.hooksPath`
# makes git read `.githooks/` instead, and that IS committed.
hooks:
    git config core.hooksPath .githooks
    @echo "core.hooksPath -> .githooks (pre-commit: cargo fmt --all --check)"

# Format in place — what the pre-commit hook tells you to run.
fmt-fix:
    cargo fmt --all
