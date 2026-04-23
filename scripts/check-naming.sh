#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'

# Reject ANY occurrence of the prohibited trademark term in committed
# text files, regardless of case. There is no legitimate use of the
# word in this repository.
#
# The pattern is stored in a variable so the literal term does not appear
# as a plain string in this file (which is itself scanned by this script).
#
# Note: tests/ is intentionally excluded — it contains the term as
# fixture input to validate the detector itself (see tests/cli.rs).
prohibited="tet""ris"

targets=(README.md Cargo.toml src scripts .github)
[[ -f target/release/blocktxt ]] && targets+=(target/release/blocktxt)

fail=0
for t in "${targets[@]}"; do
  [[ -e "$t" ]] || continue
  if grep -rn -iE "${prohibited}" "$t" 2>/dev/null; then
    echo "::error::prohibited trademark term found in $t" >&2
    fail=1
  fi
done
exit "$fail"
