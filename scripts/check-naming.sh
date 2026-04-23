#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'
# Stub: grep prohibited product-naming strings from README + Cargo.toml.
# Issue #4 will extend this to release artifacts.
fail=0
for f in README.md Cargo.toml; do
  [[ -f "$f" ]] || continue
  # Allow "Tetris®" in trademark footer; flag bare "Tetris" otherwise
  if grep -nE '(^|[^®a-zA-Z])Tetris([^®]|$)' "$f" \
    | grep -vi "trademark of"; then
    echo "::error::prohibited 'Tetris' usage in $f" >&2
    fail=1
  fi
done
exit "$fail"
