#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${1:-.}"
ALLOW_FILE="${2:-autoload/rust/FootballRustEngine.gd}"
PATTERN='ClassDB.instantiate("FootballMatchSimulator")'

cd "$REPO_ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "[FAIL] E_RG_NOT_FOUND"
  echo "ripgrep (rg) not found; install rg or implement a fallback search."
  exit 1
fi

HITS="$(rg -n -F "$PATTERN" -g '*.gd' -g '*.tscn' . || true)"
if [[ -z "$HITS" ]]; then
  echo "[OK] no forbidden instantiation found"
  exit 0
fi

ALLOW_RE="^(\\./)?${ALLOW_FILE}:"
BAD="$(echo "$HITS" | rg -v "$ALLOW_RE" || true)"
if [[ -n "$BAD" ]]; then
  echo "[FAIL] E_SINGLETON_ENGINE_INSTANTIATION_FORBIDDEN"
  echo "FootballMatchSimulator는 \"${ALLOW_FILE}\"에서만 생성 가능. 직접 instantiate 금지."
  echo
  echo "$BAD"
  exit 1
fi

echo "[OK] instantiation only in ${ALLOW_FILE}"
