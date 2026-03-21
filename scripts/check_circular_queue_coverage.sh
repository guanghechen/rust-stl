#!/usr/bin/env bash
set -euo pipefail

THRESHOLD="${1:-98}"
TARGET_SUFFIX="${2:-/crates/queue/src/circular_queue.rs}"
TARGET_NAME="$(basename "${TARGET_SUFFIX}")"
REPORT_DIR="target/llvm-cov"
LCOV_PATH="${REPORT_DIR}/lcov.info"

mkdir -p "${REPORT_DIR}"

cargo llvm-cov --workspace --lcov --output-path "${LCOV_PATH}" >/dev/null

read -r total_lines hit_lines < <(
  awk -v target_suffix="${TARGET_SUFFIX}" '
    BEGIN {
      in_target = 0;
      total = 0;
      hit = 0;
    }

    /^SF:/ {
      in_target = (substr($0, length($0) - length(target_suffix) + 1) == target_suffix);
      next;
    }

    /^end_of_record$/ {
      if (in_target) {
        found = 1;
      }
      in_target = 0;
      next;
    }

    in_target && /^DA:/ {
      split($0, a, ":");
      split(a[2], b, ",");
      total += 1;
      if (b[2] > 0) {
        hit += 1;
      }
    }

    END {
      if (!found && total == 0) {
        print "0 0";
        exit 3;
      }
      print total, hit;
    }
  ' "${LCOV_PATH}"
)

if [[ "${total_lines}" == "0" ]]; then
  echo "error: no coverage data found for ${TARGET_SUFFIX}"
  exit 1
fi

coverage="$(awk -v hit="${hit_lines}" -v total="${total_lines}" 'BEGIN { printf "%.2f", (hit / total) * 100 }')"

printf '%s line coverage: %s%% (%s/%s)\n' "${TARGET_NAME}" "${coverage}" "${hit_lines}" "${total_lines}"
printf 'required threshold: %s%%\n' "${THRESHOLD}"

if ! awk -v cov="${coverage}" -v threshold="${THRESHOLD}" 'BEGIN { exit (cov + 1e-9 >= threshold ? 0 : 1) }'; then
  echo "error: coverage threshold not met"
  exit 1
fi
