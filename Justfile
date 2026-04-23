# blocktxt — terminal falling-block puzzle game
# https://github.com/NikolayS/blocktxt-1
#
# Run `just --list` to see all available recipes.

set shell := ["bash", "-uc"]

export CARGO_TERM_COLOR := "always"

# --- build ---

# Compile a debug build
build:
    cargo build

# Compile an optimised release build
release:
    cargo build --release

# Install the binary from this worktree
install:
    cargo install --locked --path .

# --- run ---

# Run the game (pass extra flags after --: `just run -- --seed 42`)
run *args:
    cargo run --release -- {{args}}

# --- test ---

# Run the full test suite
test:
    cargo test

# Run only lib unit tests (fast iteration)
test-quick:
    cargo test --lib

# --- lint / format ---

# Format all crates in-place
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run Clippy with all warnings as errors
clippy:
    cargo clippy --all-targets -- -D warnings

# --- security / compliance ---

# Check dependency licences and advisories
deny:
    cargo deny check

# Scan source files for the prohibited trademark term
naming:
    bash scripts/check-naming.sh

# --- ci ---

# Full local CI gate: fmt-check clippy test deny naming
ci: fmt-check clippy test deny naming

# --- misc ---

# Print the release binary size; warn if > 8 MiB
bench-size: release
    @bin=target/release/blocktxt; \
    actual=$(wc -c < "${bin}" | tr -d ' '); \
    mib=$(( actual / 1048576 )); \
    echo "Binary size: ${actual} bytes (${mib} MiB)"; \
    if (( actual > 8388608 )); then \
      echo "WARNING: binary exceeds 8 MiB release guard" >&2; \
    fi

# Remove build artefacts (including worktree target dirs)
clean:
    cargo clean && rm -rf .worktrees/*/target
