#!/usr/bin/env bash
# ADR integrity check: no duplicate numbers, no orphans between files and
# the DECISION-LOG index, and a printout of the next-available ADR number.
#
# Exit 0 on success, 1 on any integrity violation. Stdout is the report;
# stderr is reserved for runtime errors.

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
decisions_dir="${repo_root}/plans/decisions"
log_file="${decisions_dir}/DECISION-LOG.md"

if [[ ! -d "${decisions_dir}" ]]; then
  echo "error: ${decisions_dir} not found" >&2
  exit 2
fi
if [[ ! -f "${log_file}" ]]; then
  echo "error: ${log_file} not found" >&2
  exit 2
fi

# All ADR files: NNN-*.md or NNN<letter>-*.md. Tolerate an empty/no-match
# decisions directory — `grep` returns 1 on zero matches, and a bare pipeline
# would trip `set -o pipefail`. Wrap in `|| true` and rely on the empty-array
# guards below.
mapfile -t adr_files < <(cd "${decisions_dir}" && ls | grep -E '^[0-9]{3}[a-z]?-.*\.md$' 2>/dev/null | sort || true)

# Extract the leading number from each filename. printf on an empty array
# emits a single empty line; filter it out so file_numbers is genuinely empty
# when adr_files is empty.
if [[ ${#adr_files[@]} -gt 0 ]]; then
  mapfile -t file_numbers < <(printf '%s\n' "${adr_files[@]}" | sed -E 's/^([0-9]{3})[a-z]?-.*/\1/')
else
  file_numbers=()
fi

# Extract every ADR reference from DECISION-LOG: looks for [NNN](NNN<letter>?-*.md).
# Capture both the label and the linked filename so a mismatched target
# ([001](999-missing.md)) cannot pass as a valid reference to ADR 001.
# Same tolerance: grep returns 1 if the log has no references yet.
mapfile -t log_links < <(grep -oE '\[[0-9]{3}[a-z]?\]\([0-9]{3}[a-z]?-[^)]+\)' "${log_file}" 2>/dev/null \
  | sort -u || true)

failed=0

# 1. Duplicate file numbers
duplicates="$(printf '%s\n' "${file_numbers[@]}" | sort | uniq -d || true)"
if [[ -n "${duplicates}" ]]; then
  echo "FAIL: duplicate ADR numbers on disk:"
  while IFS= read -r num; do
    printf '  ADR-%s:\n' "${num}"
    (cd "${decisions_dir}" && ls "${num}"*-*.md 2>/dev/null) | sed 's/^/    /'
  done <<<"${duplicates}"
  failed=1
fi

# 1b. Label/target mismatch and missing link targets in DECISION-LOG
declare -a log_refs=()
declare -A seen_log_refs=()
for link in "${log_links[@]+"${log_links[@]}"}"; do
  [[ -z "${link}" ]] && continue
  label="$(sed -E 's/\[([0-9]{3}[a-z]?)\].*/\1/' <<<"${link}")"
  target="$(sed -E 's/\[[0-9]{3}[a-z]?\]\(([^)]+)\)/\1/' <<<"${link}")"
  target_id="$(sed -E 's/^([0-9]{3}[a-z]?)-.*/\1/' <<<"${target}")"
  if [[ "${label}" != "${target_id}" ]]; then
    echo "FAIL: DECISION-LOG label/target mismatch: ${link} (label ADR-${label}, target id ADR-${target_id})"
    failed=1
  fi
  if [[ ! -f "${decisions_dir}/${target}" ]]; then
    echo "FAIL: DECISION-LOG link target missing: ${link} → ${target}"
    failed=1
  fi
  if [[ -z "${seen_log_refs[${label}]+x}" ]]; then
    log_refs+=("${label}")
    seen_log_refs["${label}"]=1
  fi
done
if [[ ${#log_refs[@]} -gt 0 ]]; then
  mapfile -t log_refs < <(printf '%s\n' "${log_refs[@]}" | sort -u)
fi

# 2. Files not indexed in DECISION-LOG
mapfile -t file_ids < <(printf '%s\n' "${adr_files[@]}" | sed -E 's/^([0-9]{3}[a-z]?)-.*/\1/' | sort -u)
mapfile -t missing_from_log < <(comm -23 <(printf '%s\n' "${file_ids[@]}") <(printf '%s\n' "${log_refs[@]+"${log_refs[@]}"}"))
if [[ ${#missing_from_log[@]} -gt 0 && -n "${missing_from_log[0]}" ]]; then
  echo "FAIL: ADR files not referenced in DECISION-LOG.md:"
  printf '  ADR-%s\n' "${missing_from_log[@]}"
  failed=1
fi

# 3. DECISION-LOG entries with no corresponding file (by label id)
mapfile -t missing_files < <(comm -13 <(printf '%s\n' "${file_ids[@]}") <(printf '%s\n' "${log_refs[@]+"${log_refs[@]}"}"))
if [[ ${#missing_files[@]} -gt 0 && -n "${missing_files[0]}" ]]; then
  echo "FAIL: DECISION-LOG references with no ADR file:"
  printf '  ADR-%s\n' "${missing_files[@]}"
  failed=1
fi

# 4. Compute next available number. A bare slot is "occupied" if any file
#    starts with that number, including suffix variants (011a occupies 011
#    because reusing the bare number alongside a variant is confusing).
if [[ ${#adr_files[@]} -gt 0 ]]; then
  mapfile -t occupied_numbers < <(printf '%s\n' "${adr_files[@]}" \
    | sed -E 's/^([0-9]{3}).*/\1/' | sort -u)
else
  occupied_numbers=()
fi

if [[ ${#occupied_numbers[@]} -gt 0 ]]; then
  max_num=$(printf '%s\n' "${occupied_numbers[@]}" | tail -n1)
  next_num=""
  for ((i=0; i<=10#${max_num}+1; i++)); do
    candidate=$(printf '%03d' "${i}")
    if ! printf '%s\n' "${occupied_numbers[@]}" | grep -qx "${candidate}"; then
      next_num="${candidate}"
      break
    fi
  done
else
  next_num="000"
fi

if [[ "${failed}" -eq 0 ]]; then
  echo "OK: ${#adr_files[@]} ADR files; ${#log_refs[@]} indexed in DECISION-LOG; no duplicates, no orphans."
fi
echo "next available ADR number: ${next_num}"

exit "${failed}"
