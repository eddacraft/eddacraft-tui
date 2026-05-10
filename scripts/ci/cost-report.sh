#!/usr/bin/env bash
# Summarise recent GitHub Actions cost signals.
#
# This is intentionally read-only: it queries GitHub Actions metadata through
# `gh`, or reads a captured `gh run list --json ...` payload via --input. The
# first version reports workflow-level minutes and run reasons; pass --jobs to
# fetch per-job timing for the sampled runs.

set -euo pipefail

readonly RUN_FIELDS='workflowName,event,status,conclusion,createdAt,updatedAt,headBranch,databaseId'
readonly MAX_LIMIT=500
readonly MAX_JOB_LIMIT=200

limit=100
input_file=''
include_jobs=false
output='markdown'

usage() {
  cat <<'EOF'
Usage: scripts/ci/cost-report.sh [options]

Options:
  --limit <n>     Number of recent workflow runs to inspect (default: 100)
  --input <file>  Read a saved gh run list JSON payload instead of calling gh
  --jobs          Fetch per-job timings for each sampled run (slower)
  --json          Emit machine-readable JSON summary
  -h, --help      Show this help

Examples:
  scripts/ci/cost-report.sh --limit 200
  scripts/ci/cost-report.sh --limit 50 --jobs
  gh run list --limit 20 --json workflowName,event,status,conclusion,createdAt,updatedAt,headBranch,databaseId > /tmp/runs.json
  scripts/ci/cost-report.sh --input /tmp/runs.json
EOF
}

while (($#)); do
  case "$1" in
    --)
      shift
      ;;
    --limit)
      limit="${2:?--limit requires a value}"
      shift 2
      ;;
    --input)
      input_file="${2:?--input requires a file path}"
      shift 2
      ;;
    --jobs)
      include_jobs=true
      shift
      ;;
    --json)
      output='json'
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! [[ "${limit}" =~ ^[0-9]+$ ]] || [[ "${limit}" == '0' ]]; then
  echo "--limit must be a positive integer" >&2
  exit 2
fi
if ((limit > MAX_LIMIT)); then
  echo "--limit must be <= ${MAX_LIMIT}" >&2
  exit 2
fi
if [[ "${include_jobs}" == true ]] && ((limit > MAX_JOB_LIMIT)); then
  echo "--jobs requires --limit <= ${MAX_JOB_LIMIT}" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

runs_file=$(mktemp)
jobs_file=$(mktemp)
errors_file=$(mktemp)
cleanup() {
  rm -f "${runs_file}" "${jobs_file}" "${errors_file}"
}
trap cleanup EXIT

if [[ -n "${input_file}" ]]; then
  jq '.' "${input_file}" >"${runs_file}"
else
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is required unless --input is provided" >&2
    exit 2
  fi
  gh run list --limit "${limit}" --json "${RUN_FIELDS}" >"${runs_file}"
fi

if [[ "${include_jobs}" == true ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "--jobs requires gh" >&2
    exit 2
  fi

  : >"${jobs_file}"
  : >"${errors_file}"
  jq -r '.[].databaseId' "${runs_file}" | while IFS= read -r run_id; do
    [[ -z "${run_id}" ]] && continue
    if ! [[ "${run_id}" =~ ^[0-9]+$ ]]; then
      jq -nc --arg runId "${run_id}" --arg error 'invalid run id' '{runId: $runId, error: $error}' >>"${errors_file}"
      continue
    fi

    run_payload_file=$(mktemp)
    run_error_file=$(mktemp)
    if ! gh run view "${run_id}" --json 'workflowName,event,headBranch,databaseId,jobs' >"${run_payload_file}" 2>"${run_error_file}"; then
      jq -nc --arg runId "${run_id}" --rawfile error "${run_error_file}" '{runId: $runId, error: $error}' >>"${errors_file}"
      rm -f "${run_payload_file}" "${run_error_file}"
      continue
    fi

    if ! jq -c '.jobs[] as $job | {
        runId: .databaseId,
        workflowName: .workflowName,
        event: .event,
        headBranch: .headBranch,
        name: $job.name,
        conclusion: ($job.conclusion // ""),
        status: ($job.status // ""),
        startedAt: $job.startedAt,
        completedAt: $job.completedAt,
        minutes: if $job.startedAt == null or ($job.startedAt | startswith("0001-")) then 0 else (((if $job.completedAt == null or ($job.completedAt | startswith("0001-")) then $job.startedAt else $job.completedAt end | fromdateiso8601) - ($job.startedAt | fromdateiso8601)) / 60) end
      }' "${run_payload_file}" >>"${jobs_file}" 2>"${run_error_file}"; then
      jq -nc --arg runId "${run_id}" --rawfile error "${run_error_file}" '{runId: $runId, error: $error}' >>"${errors_file}"
    fi
    rm -f "${run_payload_file}" "${run_error_file}"
  done
fi

summary_json=$(jq -n \
  --slurpfile runs "${runs_file}" \
  --rawfile jobs "${jobs_file}" \
  --rawfile errors "${errors_file}" '
  def minutes($start; $end):
    (((($end // $start) | fromdateiso8601) - ($start | fromdateiso8601)) / 60);

  def conclusion_key:
    if .conclusion == null or .conclusion == "" then "in_progress" else .conclusion end;

  def status_counts($items):
    reduce $items[] as $item ({}; .[$item.conclusionKey] = ((.[$item.conclusionKey] // 0) + 1));

  ($runs[0] // []) as $runItems |
  ($runItems | map(. + {
    conclusionKey: conclusion_key,
    minutes: minutes(.createdAt; .updatedAt)
  })) as $normalisedRuns |
  ($jobs | split("\n") | map(select(length > 0) | fromjson)) as $jobItems |
  ($errors | split("\n") | map(select(length > 0) | fromjson)) as $errorItems |
  {
    generatedAt: (now | todateiso8601),
    measurementModel: {
      workflowMinutes: "elapsed wall-clock minutes from GitHub run metadata",
      jobMinutes: "summed job wall-clock minutes when --jobs is used",
      notMeasured: ["runner cost multipliers", "path/risk classes", "matrix spend", "coverage spend", "security spend"]
    },
    sampledRuns: ($normalisedRuns | length),
    workflowTotals: (
      $normalisedRuns
      | group_by(.workflowName)
      | map({
          workflow: .[0].workflowName,
          runs: length,
          minutes: (map(.minutes) | add // 0),
          conclusions: status_counts(.),
          events: (group_by(.event) | map({event: .[0].event, runs: length}))
        })
      | sort_by(-.minutes)
    ),
    eventTotals: (
      $normalisedRuns
      | group_by(.event)
      | map({event: .[0].event, runs: length, minutes: (map(.minutes) | add // 0)})
      | sort_by(-.minutes)
    ),
    branchTotals: (
      $normalisedRuns
      | group_by(.headBranch)
      | map({branch: (.[0].headBranch // ""), runs: length, minutes: (map(.minutes) | add // 0)})
      | sort_by(-.minutes)
    ),
    jobTotals: (
      $jobItems
      | group_by(.workflowName, .name)
      | map({
          workflow: .[0].workflowName,
          job: .[0].name,
          runs: length,
          minutes: (map(.minutes) | add // 0),
          failures: (map(select(.conclusion == "failure")) | length),
          cancellations: (map(select(.conclusion == "cancelled")) | length)
        })
      | sort_by(-.minutes)
    ),
    omittedRuns: $errorItems
  }')

if [[ "${output}" == 'json' ]]; then
  jq '.' <<<"${summary_json}"
  exit 0
fi

jq -r '
  def md: tostring | gsub("[\r\n]"; " ") | gsub("\\|"; "\\\\|") | gsub("`"; "\\`");
  "# CI Cost Report",
  "",
  "Generated: `" + .generatedAt + "`",
  "Sampled runs: `" + (.sampledRuns | tostring) + "`",
  "",
  "> Workflow, event, and branch totals are elapsed wall-clock minutes from GitHub run metadata. Job totals, when requested, are summed job wall-clock durations. Runner cost multipliers, path/risk classes, matrix spend, coverage spend, and security spend are target-state dimensions not yet measured by this baseline report.",
  "",
  "## Workflow Elapsed Minutes",
  "",
  "| Workflow | Runs | Elapsed Minutes | Conclusions | Events |",
  "| --- | ---: | ---: | --- | --- |",
  (.workflowTotals[] | "| `" + (.workflow | md) + "` | " + (.runs | tostring) + " | " + (.minutes | tonumber | . * 10 | round / 10 | tostring) + " | " + (.conclusions | to_entries | map((.key | md) + ": " + (.value | tostring)) | join(", ")) + " | " + (.events | map((.event | md) + ": " + (.runs | tostring)) | join(", ")) + " |"),
  "",
  "## Event Elapsed Minutes",
  "",
  "| Event | Runs | Elapsed Minutes |",
  "| --- | ---: | ---: |",
  (.eventTotals[] | "| `" + (.event | md) + "` | " + (.runs | tostring) + " | " + (.minutes | tonumber | . * 10 | round / 10 | tostring) + " |"),
  "",
  "## Branch Elapsed Minutes",
  "",
  "| Branch | Runs | Elapsed Minutes |",
  "| --- | ---: | ---: |",
  (.branchTotals[0:20][] | "| `" + (.branch | md) + "` | " + (.runs | tostring) + " | " + (.minutes | tonumber | if . < 0.05 and . > -0.05 then 0 else . end | . * 10 | round / 10 | tostring) + " |")
' <<<"${summary_json}"

if [[ "${include_jobs}" == true ]]; then
  jq -r '
    def md: tostring | gsub("[\r\n]"; " ") | gsub("\\|"; "\\\\|") | gsub("`"; "\\`");
    "",
    "## Job Minutes",
    "",
    "| Workflow | Job | Runs | Minutes | Failures | Cancelled |",
    "| --- | --- | ---: | ---: | ---: | ---: |",
    (.jobTotals[0:30][] | "| `" + (.workflow | md) + "` | `" + (.job | md) + "` | " + (.runs | tostring) + " | " + (.minutes | tonumber | if . < 0.05 and . > -0.05 then 0 else . end | . * 10 | round / 10 | tostring) + " | " + (.failures | tostring) + " | " + (.cancellations | tostring) + " |")
  ' <<<"${summary_json}"
fi

jq -r '
  if (.omittedRuns | length) > 0 then
    "",
    "## Omitted Runs",
    "",
    "| Run ID | Error |",
    "| --- | --- |",
    (.omittedRuns[] | "| `" + (.runId | tostring) + "` | " + (.error | tostring | gsub("[\r\n|]"; " ")) + " |")
  else empty end
' <<<"${summary_json}"
