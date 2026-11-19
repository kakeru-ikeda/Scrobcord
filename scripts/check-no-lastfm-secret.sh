#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
pat='LASTFM_API_''SECRET'

set +e
matches=$(git grep -n --untracked -e "$pat" -- src-tauri .github 2>&1)
result=$?
set -e

if [[ $result -eq 0 ]]; then
  printf '%s\n' "$matches" >&2
  printf '%s\n' 'LASTFM の secret の参照が残っています。署名は scrobsign で行います' >&2
  exit 1
elif [[ $result -eq 1 ]]; then
  printf '%s\n' 'no LASTFM secret references'
  exit 0
else
  printf '%s\n' "$matches" >&2
  exit "$result"
fi
