#!/usr/bin/env bash
# Guardrail for bd-2jpu6.4:
# fail if production engine code reintroduces raw OS-thread spawning.

set -euo pipefail

SCRIPT_PATH="${BASH_SOURCE[0]}"
if [[ "${SCRIPT_PATH}" != /* ]]; then
  SCRIPT_PATH="$(pwd)/${SCRIPT_PATH}"
fi
REPO_ROOT="$(cd "$(dirname "${SCRIPT_PATH}")/.." && pwd)"
cd "${REPO_ROOT}"

TARGET_DIRS=(
  "crates/fsqlite/src"
  "crates/fsqlite-core/src"
  "crates/fsqlite-vfs/src"
  "crates/fsqlite-pager/src"
  "crates/fsqlite-wal/src"
  "crates/fsqlite-mvcc/src"
  "crates/fsqlite-btree/src"
  "crates/fsqlite-vdbe/src"
  "crates/fsqlite-observability/src"
  "crates/fsqlite-cli/src"
)

FORBIDDEN_PATTERN='std::thread::spawn|thread::spawn|std::thread::Builder|thread::Builder'
declare -a HITS=()

scan_file() {
  local file="$1"
  local output
  output="$(
    awk -v forbidden="${FORBIDDEN_PATTERN}" '
      function brace_delta(text, i, ch, delta) {
        delta = 0;
        for (i = 1; i <= length(text); i++) {
          ch = substr(text, i, 1);
          if (ch == "{") {
            delta++;
          } else if (ch == "}") {
            delta--;
          }
        }
        return delta;
      }

      /^[[:space:]]*#\[cfg\([[:space:]]*test[[:space:]]*\)\]/ {
        pending_test_item = 1;
        next;
      }

      skip_depth > 0 {
        skip_depth += brace_delta($0);
        if (skip_depth <= 0) {
          skip_depth = 0;
        }
        next;
      }

      pending_test_item {
        if ($0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/) {
          next;
        }

        skip_depth = brace_delta($0);
        if (skip_depth > 0) {
          pending_test_item = 0;
        } else if ($0 ~ /;/) {
          pending_test_item = 0;
          skip_depth = 0;
        }
        next;
      }

      $0 ~ forbidden {
        print FILENAME ":" FNR ":" $0;
      }
    ' "${file}" || true
  )"

  if [[ -n "${output}" ]]; then
    while IFS= read -r line; do
      HITS+=("${line}")
    done <<< "${output}"
  fi
}

while IFS= read -r -d '' file; do
  scan_file "${file}"
done < <(
  find "${TARGET_DIRS[@]}" \
    -type f \
    -name '*.rs' \
    ! -name '*_tests.rs' \
    ! -path '*/src/bin/*' \
    -print0
)

if (( ${#HITS[@]} > 0 )); then
  echo "[FAIL] raw OS-thread usage detected in production engine code:" >&2
  printf '%s\n' "${HITS[@]}" >&2
  exit 1
fi

echo "[PASS] no raw OS-thread spawn/builders found in production engine crate sources"
