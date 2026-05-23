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

TARGET_DIRS=(crates/*/src)

FORBIDDEN_PATTERN='(^|[^[:alnum:]_])(std[[:space:]]*::[[:space:]]*)?thread[[:space:]]*::[[:space:]]*(spawn|Builder)([^[:alnum:]_]|$)'
declare -a HITS=()

scan_source() {
  local display_name="$1"

  awk -v forbidden="${FORBIDDEN_PATTERN}" -v display_name="${display_name}" '
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

    /^[[:space:]]*#\[cfg[[:space:]]*\([[:space:]]*test[[:space:]]*\)\]/ {
      pending_test_item = 1;
      next;
    }

    /^[[:space:]]*#\[test\]/ {
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

      if ($0 ~ /;/) {
        pending_test_item = 0;
        skip_depth = 0;
      } else if ($0 ~ /\{/) {
        pending_test_item = 0;
        skip_depth = brace_delta($0);
        if (skip_depth < 0) {
          skip_depth = 0;
        }
      }
      next;
    }

    $0 ~ forbidden {
      print display_name ":" FNR ":" $0;
    }
  '
}

append_hits() {
  local output="$1"

  if [[ -n "${output}" ]]; then
    while IFS= read -r line; do
      HITS+=("${line}")
    done <<< "${output}"
  fi
}

scan_file() {
  local file="$1"
  local output
  output="$(
    # shellcheck disable=SC2094
    scan_source "${file}" < "${file}" || true
  )"

  append_hits "${output}"
}

run_self_test() {
  local output
  output="$(
    scan_source "self_test_mixed.rs" <<'RUST'
#[cfg(test)]
mod tests {
    fn test_spawn_is_ignored() {
        std::thread::spawn(|| {}); // test spawn sentinel
    }
}

#[test]
fn standalone_test_spawn_is_ignored() {
    std::thread::spawn(|| {}); // standalone test spawn sentinel
}

#[cfg(test)]
mod inline_tests { fn inline_test_spawn_is_ignored() { std::thread::spawn(|| {}); } } // inline cfg(test) spawn sentinel

#[test]
fn inline_standalone_test_spawn_is_ignored() { std::thread::spawn(|| {}); } // inline standalone test spawn sentinel

#[cfg(test)]
fn spaced_test_spawn_is_ignored() { std :: thread :: spawn(|| {}); } // spaced test spawn sentinel

#[cfg (test)]
fn spaced_cfg_attribute_spawn_is_ignored() { thread :: spawn(|| {}); } // spaced cfg attribute spawn sentinel

fn production_after_test_is_detected() {
    std::thread::spawn(|| {}); // production spawn sentinel
}

fn production_spaced_path_is_detected() {
    thread :: spawn(|| {}); // spaced production spawn sentinel
    std :: thread :: Builder::new().spawn(|| {}).unwrap(); // spaced builder sentinel
}
RUST
  )"

  if [[ "${output}" == *"test spawn sentinel"* || "${output}" == *"standalone test spawn sentinel"* || "${output}" == *"inline cfg(test) spawn sentinel"* || "${output}" == *"inline standalone test spawn sentinel"* || "${output}" == *"spaced test spawn sentinel"* || "${output}" == *"spaced cfg attribute spawn sentinel"* ]]; then
    echo "[FAIL] self-test scanner reported a test-only thread spawn" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi

  if [[ "${output}" != *"production spawn sentinel"* ]]; then
    echo "[FAIL] self-test scanner missed a production thread spawn after #[cfg(test)]" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi

  if [[ "${output}" != *"spaced production spawn sentinel"* || "${output}" != *"spaced builder sentinel"* ]]; then
    echo "[FAIL] self-test scanner missed a whitespace-formatted production thread path" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi

  echo "[PASS] no-OS-thread scanner self-test passed"
}

if [[ "${1:-}" == "--self-test" ]]; then
  run_self_test
  exit 0
fi

if (( $# > 0 )); then
  echo "usage: $0 [--self-test]" >&2
  exit 2
fi

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
