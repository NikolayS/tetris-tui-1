#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'

# bump-version.sh <version>
# Updates Cargo.toml version and scaffolds a CHANGELOG.md entry.
# Usage: bash scripts/bump-version.sh 0.1.0

USAGE="Usage: $(basename "${0}") <version>  (e.g. 0.1.0)"

# ── argument validation ───────────────────────────────────────────────────────

if [[ $# -ne 1 ]]; then
  echo "${USAGE}" >&2
  exit 1
fi

new_version="${1}"

if ! [[ "${new_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be semver (e.g. 1.2.3), got: ${new_version}" >&2
  exit 1
fi

# ── helpers ───────────────────────────────────────────────────────────────────

repo_root() {
  git rev-parse --show-toplevel
}

current_version() {
  grep -m1 '^version = ' "$(repo_root)/Cargo.toml" \
    | sed 's/version = "\(.*\)"/\1/'
}

update_cargo_toml() {
  local root="${1}"
  local old_ver="${2}"
  local new_ver="${3}"
  local cargo="${root}/Cargo.toml"
  # Replace only the first occurrence (package version, not dep versions)
  perl -i -0pe \
    "s/(\\[package\\].*?version = \")${old_ver}(\")/"'${1}'"${new_ver}"'${2}/s' \
    "${cargo}"
  echo "Updated ${cargo}: ${old_ver} -> ${new_ver}"
}

scaffold_changelog() {
  local root="${1}"
  local ver="${2}"
  local date
  date="$(date -u +%Y-%m-%d)"
  local changelog="${root}/CHANGELOG.md"
  local entry
  entry="$(printf \
    '## [%s] — %s\n\n### Added\n\n- \n\n### Fixed\n\n- \n\n### Changed\n\n- \n' \
    "${ver}" "${date}")"

  if [[ ! -f "${changelog}" ]]; then
    printf '# Changelog\n\nAll notable changes are documented here.\n\n' \
      > "${changelog}"
    printf '%s\n' "${entry}" >> "${changelog}"
    echo "Created ${changelog}"
    return
  fi

  # Insert after the first heading line (# Changelog ...)
  perl -i -0pe \
    "s/(^# [^\n]+\n+)/"'${1}'"${entry//\//\\/}\n\n/" \
    "${changelog}"
  echo "Scaffolded entry in ${changelog}"
}

# ── main ──────────────────────────────────────────────────────────────────────

main() {
  local root
  root="$(repo_root)"
  local old_ver
  old_ver="$(current_version)"

  if [[ "${old_ver}" == "${new_version}" ]]; then
    echo "Already at ${new_version}; no-op."
    exit 0
  fi

  echo "Bumping ${old_ver} -> ${new_version}"

  update_cargo_toml "${root}" "${old_ver}" "${new_version}"
  scaffold_changelog "${root}" "${new_version}"

  # Regenerate Cargo.lock so it stays consistent
  cargo generate-lockfile --manifest-path "${root}/Cargo.toml" \
    2>/dev/null || true

  echo ""
  echo "Done. Next steps:"
  echo "  1. Fill in the CHANGELOG entry."
  echo "  2. Commit: chore: bump version to ${new_version}"
  echo "  3. Tag on main after the PR merges."
}

main "$@"
