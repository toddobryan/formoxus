# Serve the example gallery at http://localhost:8080 with hot reload.
serve:
    dx serve -p formoxus-examples

# Same, with dx's own log going to a file — useful when the terminal is busy.
serve-log:
    dx serve -p formoxus-examples --log-to-file /tmp/dx-formoxus.log

# Which (value kind, control) pairs actually render. See
# .claude/memory/control_table_and_choice.md for what the number means.
matrix:
    cargo run -p formoxus-examples --bin control_matrix

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
    cargo +1.90 check -p formoxus -p formoxus-macros

# Both builds the gallery has to satisfy — the browser one and the host one.
check-web:
    cargo check -p formoxus-examples --target wasm32-unknown-unknown --features web

check-server:
    cargo check -p formoxus-examples --features server
