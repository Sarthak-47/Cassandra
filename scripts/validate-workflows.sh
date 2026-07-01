#!/usr/bin/env bash
# Validates every GitHub Actions workflow file: static lint via actionlint,
# then a syntax/job-graph sanity check via `act -l` for the fixture event
# payloads under .github/act/. Assumes `actionlint` and `act` are already on
# PATH (the workflow-validation CI job installs both before calling this).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "==> actionlint"
actionlint

echo "==> act: workflow/job graph sanity check per fixture event"
shopt -s nullglob
fixtures=(.github/act/*.json)
shopt -u nullglob

if [ ${#fixtures[@]} -eq 0 ]; then
    echo "No fixtures under .github/act/ — skipping act checks."
    exit 0
fi

for fixture in "${fixtures[@]}"; do
    echo "  - ${fixture}"
    act -n -e "$fixture" --list >/dev/null
done

echo "All workflow files validated."
