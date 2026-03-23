#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

readonly TAG_FORMAT_HINT='<crate>/v<version>'

usage() {
  cat <<'USAGE'
Usage:
  scripts/publish.sh plan
  scripts/publish.sh tag [--execute]

Commands:
  plan             生成发布计划（changed + affected + registry compare）
  tag              为需要发布的 crate 生成独立 tag（默认 dry-run）

Options:
  --execute        仅用于 tag 子命令，真正执行 git tag -a
USAGE
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[error] missing command: $cmd" >&2
    exit 1
  fi
}

index_of_crate() {
  local target="$1"
  local i
  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    if [[ "${CRATES[$i]}" == "$target" ]]; then
      echo "$i"
      return 0
    fi
  done
  return 1
}

fetch_registry_version() {
  local crate="$1"
  local response
  local body
  local status

  response="$(curl -sS -L --connect-timeout 5 --max-time 20 -w $'\n%{http_code}' "https://crates.io/api/v1/crates/${crate}" || true)"
  if [[ -z "$response" ]]; then
    echo "__ERROR_HTTP_000__"
    return 0
  fi

  status="${response##*$'\n'}"
  body="${response%$'\n'*}"

  case "$status" in
    200)
      jq -r '.crate.max_version // empty' <<<"$body"
      ;;
    404)
      echo ""
      ;;
    *)
      echo "__ERROR_HTTP_${status}__"
      ;;
  esac
}

semver_cmp() {
  local a="$1"
  local b="$2"
  local max_version

  if [[ "$a" == "$b" ]]; then
    echo 0
    return 0
  fi

  max_version="$(printf '%s\n%s\n' "$a" "$b" | sort -V | tail -n 1)"
  if [[ "$max_version" == "$a" ]]; then
    echo 1
  else
    echo -1
  fi
}

collect_workspace() {
  local metadata_json
  local name
  local manifest_path
  local version
  local deps_csv
  local rel_manifest
  local rel_path
  local i
  local dep
  local dep_index

  metadata_json="$(cargo metadata --format-version 1 --no-deps)"

  while IFS=$'\t' read -r name manifest_path version deps_csv; do
    [[ -z "$name" ]] && continue

    rel_manifest="${manifest_path#${ROOT_DIR}/}"
    rel_path="${rel_manifest%/Cargo.toml}"

    CRATES+=("$name")
    PATHS+=("$rel_path")
    VERSIONS+=("$version")
    DEPS+=("$deps_csv")
    REVERSE_DEPS+=("")
    LAST_TAGS+=("")
    CHANGED+=("0")
    AFFECTED+=("0")
    REMOTE_VERSIONS+=("")
    DECISIONS+=("")
  done < <(
    jq -r '
      .packages[]
      | [
          .name,
          .manifest_path,
          .version,
          ([.dependencies[]? | select(.path != null) | .name] | join(","))
        ]
      | @tsv
    ' <<<"$metadata_json"
  )

  if [[ ${#CRATES[@]} -eq 0 ]]; then
    echo "[error] no workspace crates found" >&2
    exit 1
  fi

  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    if [[ -z "${DEPS[$i]}" ]]; then
      continue
    fi

    IFS=',' read -r -a dep_names <<<"${DEPS[$i]}"
    for dep in "${dep_names[@]}"; do
      [[ -z "$dep" ]] && continue
      if ! dep_index="$(index_of_crate "$dep")"; then
        continue
      fi

      if [[ -z "${REVERSE_DEPS[$dep_index]}" ]]; then
        REVERSE_DEPS[$dep_index]="${CRATES[$i]}"
      else
        REVERSE_DEPS[$dep_index]="${REVERSE_DEPS[$dep_index]},${CRATES[$i]}"
      fi
    done
  done
}

detect_changed_and_affected() {
  local i
  local last_tag
  local queue=()
  local q_index
  local current
  local current_index
  local dep_index
  local dependent

  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    last_tag="$(git tag --list "${CRATES[$i]}/v*" --sort=-v:refname | head -n 1)"
    LAST_TAGS[$i]="$last_tag"

    if [[ -n "$last_tag" ]]; then
      if git diff --quiet "${last_tag}..HEAD" -- "${PATHS[$i]}"; then
        CHANGED[$i]="0"
      else
        CHANGED[$i]="1"
      fi
    else
      CHANGED[$i]="1"
    fi

    if [[ "${CHANGED[$i]}" == "1" ]]; then
      AFFECTED[$i]="1"
      queue+=("${CRATES[$i]}")
    fi
  done

  q_index=0
  while [[ $q_index -lt ${#queue[@]} ]]; do
    current="${queue[$q_index]}"
    q_index=$((q_index + 1))

    current_index="$(index_of_crate "$current")"
    if [[ -z "${REVERSE_DEPS[$current_index]}" ]]; then
      continue
    fi

    IFS=',' read -r -a dependents <<<"${REVERSE_DEPS[$current_index]}"
    for dependent in "${dependents[@]}"; do
      [[ -z "$dependent" ]] && continue
      dep_index="$(index_of_crate "$dependent")"
      if [[ "${AFFECTED[$dep_index]}" == "1" ]]; then
        continue
      fi
      AFFECTED[$dep_index]="1"
      queue+=("$dependent")
    done
  done
}

compare_registry_versions() {
  local i
  local remote
  local cmp

  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    remote="$(fetch_registry_version "${CRATES[$i]}")"
    REMOTE_VERSIONS[$i]="$remote"

    if [[ "$remote" == __ERROR_HTTP_* ]]; then
      DECISIONS[$i]="check_failed"
      continue
    fi

    if [[ -z "$remote" ]]; then
      DECISIONS[$i]="to_publish_unpublished"
      continue
    fi

    cmp="$(semver_cmp "${VERSIONS[$i]}" "$remote")"
    if [[ "$cmp" == "1" ]]; then
      if [[ "${AFFECTED[$i]}" == "1" ]]; then
        DECISIONS[$i]="to_publish"
      else
        DECISIONS[$i]="to_publish_version_gap"
      fi
    elif [[ "$cmp" == "0" ]]; then
      if [[ "${AFFECTED[$i]}" == "1" ]]; then
        DECISIONS[$i]="blocked_version_not_bumped"
      else
        DECISIONS[$i]="skip"
      fi
    else
      DECISIONS[$i]="blocked_local_lt_registry"
    fi
  done
}

print_list() {
  local title="$1"
  shift
  local values=("$@")

  if [[ ${#values[@]} -eq 0 ]]; then
    echo "- ${title}: -"
    return 0
  fi

  local joined=""
  local item
  for item in "${values[@]}"; do
    if [[ -z "$joined" ]]; then
      joined="$item"
    else
      joined="${joined}, ${item}"
    fi
  done
  echo "- ${title}: ${joined}"
}

print_plan() {
  local i
  local last_tag
  local remote
  local changed=()
  local affected=()
  local to_publish=()
  local blocked=()

  echo "[publish-plan]"
  echo "- workspace: ${ROOT_DIR}"
  echo "- tag_format: ${TAG_FORMAT_HINT}"
  echo

  printf '%-20s %-10s %-12s %-26s %-8s %-9s %s\n' \
    'crate' 'local' 'registry' 'last_tag' 'changed' 'affected' 'decision'
  printf '%-20s %-10s %-12s %-26s %-8s %-9s %s\n' \
    '-----' '-----' '--------' '--------' '-------' '--------' '--------'

  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    last_tag="${LAST_TAGS[$i]:--}"
    remote="${REMOTE_VERSIONS[$i]:--}"
    if [[ -z "$remote" ]]; then
      remote='(none)'
    fi

    printf '%-20s %-10s %-12s %-26s %-8s %-9s %s\n' \
      "${CRATES[$i]}" \
      "${VERSIONS[$i]}" \
      "$remote" \
      "$last_tag" \
      "${CHANGED[$i]}" \
      "${AFFECTED[$i]}" \
      "${DECISIONS[$i]}"

    if [[ "${CHANGED[$i]}" == "1" ]]; then
      changed+=("${CRATES[$i]}")
    fi
    if [[ "${AFFECTED[$i]}" == "1" ]]; then
      affected+=("${CRATES[$i]}")
    fi
    case "${DECISIONS[$i]}" in
      to_publish|to_publish_unpublished|to_publish_version_gap)
        to_publish+=("${CRATES[$i]}")
        ;;
      blocked_version_not_bumped|blocked_local_lt_registry|check_failed)
        blocked+=("${CRATES[$i]}")
        ;;
    esac
  done

  echo
  print_list 'changed_crates' "${changed[@]}"
  print_list 'affected_crates' "${affected[@]}"
  print_list 'to_publish' "${to_publish[@]}"
  print_list 'blocked' "${blocked[@]}"

  if [[ ${#blocked[@]} -gt 0 ]]; then
    echo
    echo '[hint] blocked 存在时请先解决，再执行发布。'
  fi
}

run_tag() {
  local execute="0"
  local i
  local tag_name
  local tag_message
  local created=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --execute)
        execute="1"
        ;;
      -h|--help)
        usage
        return 0
        ;;
      *)
        echo "[error] unknown option for tag: $1" >&2
        return 1
        ;;
    esac
    shift
  done

  echo "[tag-plan] mode=$([[ "$execute" == "1" ]] && echo execute || echo dry-run)"
  echo "- tag_format: ${TAG_FORMAT_HINT}"

  for ((i = 0; i < ${#CRATES[@]}; i++)); do
    case "${DECISIONS[$i]}" in
      to_publish|to_publish_unpublished|to_publish_version_gap)
        tag_name="${CRATES[$i]}/v${VERSIONS[$i]}"
        tag_message="release(${CRATES[$i]}): v${VERSIONS[$i]}"

        if git rev-parse -q --verify "refs/tags/${tag_name}" >/dev/null 2>&1; then
          echo "- skip existing tag: ${tag_name}"
          continue
        fi

        echo "- git tag -a ${tag_name} -m \"${tag_message}\""
        if [[ "$execute" == "1" ]]; then
          git tag -a "$tag_name" -m "$tag_message"
          created=$((created + 1))
        fi
        ;;
    esac
  done

  if [[ "$execute" == "1" ]]; then
    echo "- created_tags: ${created}"
  else
    echo '- dry-run only, add --execute to create tags.'
  fi
}

main() {
  local cmd="${1:-plan}"

  require_cmd cargo
  require_cmd jq
  require_cmd curl
  require_cmd git
  require_cmd sort

  collect_workspace
  detect_changed_and_affected
  compare_registry_versions

  case "$cmd" in
    plan)
      if [[ $# -gt 1 ]]; then
        echo "[error] plan does not accept extra args" >&2
        exit 1
      fi
      print_plan
      ;;
    tag)
      shift
      run_tag "$@"
      ;;
    -h|--help|help)
      usage
      ;;
    *)
      echo "[error] unknown command: $cmd" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"
