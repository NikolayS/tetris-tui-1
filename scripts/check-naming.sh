#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'
# Prohibited product-name strings in README + Cargo.toml + built artifacts.
fail=0
targets=(README.md Cargo.toml)
[[ -f target/release/blocktxt ]] && targets+=(target/release/blocktxt)
for f in "${targets[@]}"; do
  [[ -f "${f}" ]] || continue
  # "Tetris®" in the trademark footer is allowed; bare "Tetris" as a
  # product name is not. The inner grep filters out lines that contain
  # "trademark of" (the standard footer phrasing).
  if grep -nE '(^|[^®a-zA-Z])(Tetris|TETRIS)([^®]|$)' "${f}" \
    | grep -vi 'trademark of'; then
    echo "::error::prohibited 'Tetris' usage in ${f}" >&2
    fail=1
  fi
done
exit "${fail}"
