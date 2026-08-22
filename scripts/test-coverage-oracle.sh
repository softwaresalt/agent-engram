#!/usr/bin/env bash
# Canonical dev-test coverage oracle (Feature 126-F, stash C2413934).
#
# Shell mirror of scripts/test-coverage-oracle.ps1. Computes the required
# test-target set for a diff from .cargo/test-coverage-manifest.toml, compares
# it against the selected set, and reports required / selected / omitted with
# the pass condition `omitted == 0`. Runnable standalone.
#
# Modes: report | select | completeness | run  (see the .ps1 header for detail).
# CLI (identical to the .ps1):
#   --mode <report|select|completeness|run>
#   --changed <comma-separated paths>
#   --selected <comma-separated target names>   (report mode; default = required)
#   --dry-run                                    (run mode)
#   --repo-root <path>  --manifest <path>  --cargo-toml <path>

set -euo pipefail

MODE="report"
CHANGED_RAW=""
SELECTED_RAW=""
SELECTED_PROVIDED=0
DRY_RUN=0
REPO_ROOT=""
MANIFEST_PATH=""
CARGO_TOML_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) MODE="$2"; shift 2 ;;
    --changed) CHANGED_RAW="$2"; shift 2 ;;
    --selected) SELECTED_RAW="$2"; SELECTED_PROVIDED=1; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --repo-root) REPO_ROOT="$2"; shift 2 ;;
    --manifest) MANIFEST_PATH="$2"; shift 2 ;;
    --cargo-toml) CARGO_TOML_PATH="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ -z "$REPO_ROOT" ]] && REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
[[ -z "$MANIFEST_PATH" ]] && MANIFEST_PATH="$REPO_ROOT/.cargo/test-coverage-manifest.toml"
[[ -z "$CARGO_TOML_PATH" ]] && CARGO_TOML_PATH="$REPO_ROOT/Cargo.toml"

norm() { local p="${1//\\//}"; p="${p#"${p%%[![:space:]]*}"}"; p="${p%"${p##*[![:space:]]}"}"; printf '%s' "$p"; }

# ── Load declared [[test]] targets ──────────────────────────────────────────
declare -a TARGET_NAMES=()
declare -A TARGET_PATH=()
declare -A TARGET_FEATURES=()
while IFS=$'\t' read -r tname tpath tfeats; do
  [[ -z "$tname" ]] && continue
  TARGET_NAMES+=("$tname")
  TARGET_PATH["$tname"]="$tpath"
  TARGET_FEATURES["$tname"]="$tfeats"
done < <(sed 's/\r$//' "$CARGO_TOML_PATH" | awk '
  function emit() { if (name != "") print name "\t" path "\t" feats; name=""; path=""; feats="" }
  /^\[\[test\]\]/ { emit(); intest=1; next }
  /^\[\[/ { if (intest) { emit(); intest=0 } }
  {
    if (!intest) next
    if ($0 ~ /^name[[:space:]]*=[[:space:]]*"/) { l=$0; sub(/^name[[:space:]]*=[[:space:]]*"/,"",l); sub(/".*/,"",l); name=l; next }
    if ($0 ~ /^path[[:space:]]*=[[:space:]]*"/) { l=$0; sub(/^path[[:space:]]*=[[:space:]]*"/,"",l); sub(/".*/,"",l); path=l; next }
    if ($0 ~ /^required-features[[:space:]]*=[[:space:]]*\[/) { tmp=$0; feats=""; while (match(tmp, /"[^"]+"/)) { g=substr(tmp, RSTART+1, RLENGTH-2); feats=(feats==""?g:feats","g); tmp=substr(tmp, RSTART+RLENGTH) } next }
  }
  END { emit() }
')

# ── Load manifest settings ──────────────────────────────────────────────────
MAX_CONCURRENT="$(sed 's/\r$//' "$MANIFEST_PATH" | grep -oE '^[[:space:]]*max_concurrent_test_binaries[[:space:]]*=[[:space:]]*[0-9]+' | grep -oE '[0-9]+' | head -n1 || true)"
[[ -z "${MAX_CONCURRENT:-}" ]] && MAX_CONCURRENT=8
TEST_THREADS="$(sed 's/\r$//' "$MANIFEST_PATH" | grep -oE '^[[:space:]]*test_threads[[:space:]]*=[[:space:]]*[0-9]+' | grep -oE '[0-9]+' | head -n1 || true)"
[[ -z "${TEST_THREADS:-}" ]] && TEST_THREADS=4
# Environment overrides (set by cargo [env] when invoked via `cargo dev-test`).
[[ -n "${ENGRAM_DEVTEST_MAX_BINARIES:-}" ]] && MAX_CONCURRENT="$ENGRAM_DEVTEST_MAX_BINARIES"
[[ -n "${ENGRAM_DEVTEST_TEST_THREADS:-}" ]] && TEST_THREADS="$ENGRAM_DEVTEST_TEST_THREADS"

# ── Load surfaces as "path<TAB>glob,glob,..." records ───────────────────────
declare -a SURFACE_PATH=()
declare -a SURFACE_GLOBS=()
while IFS=$'\t' read -r spath sglobs; do
  [[ -z "$spath" ]] && continue
  SURFACE_PATH+=("$spath")
  SURFACE_GLOBS+=("$sglobs")
done < <(sed 's/\r$//' "$MANIFEST_PATH" | awk '
  function flush() { if (spath != "") printf "%s\t%s\n", spath, globs; spath=""; globs=""; ing=0 }
  function collect(s,  g) { while (match(s, /"[^"]+"/)) { g=substr(s, RSTART+1, RLENGTH-2); globs = (globs=="" ? g : globs "," g); s=substr(s, RSTART+RLENGTH) } }
  BEGIN { spath=""; globs=""; ing=0; insurf=0 }
  /^\[\[surface\]\]/ { flush(); insurf=1; next }
  /^\[\[/ { if (insurf) { flush(); insurf=0 } }
  {
    if (!insurf) next
    if ($0 ~ /^[[:space:]]*path[[:space:]]*=[[:space:]]*"/) { l=$0; sub(/^[[:space:]]*path[[:space:]]*=[[:space:]]*"/,"",l); sub(/".*/,"",l); spath=l; next }
    if ($0 ~ /targets[[:space:]]*=[[:space:]]*\[/) { tmp=$0; sub(/.*\[/,"",tmp); collect(tmp); ing = ($0 ~ /\]/) ? 0 : 1; next }
    if (ing) { collect($0); if ($0 ~ /\]/) ing=0; next }
  }
  END { flush() }
')

# ── Glob expansion against declared target names ────────────────────────────
expand_glob() {
  local glob="$1" n
  if [[ "$glob" == "*" ]]; then
    printf '%s\n' "${TARGET_NAMES[@]}"
  elif [[ "$glob" == *"*" ]]; then
    local prefix="${glob%\*}"
    for n in "${TARGET_NAMES[@]}"; do [[ "$n" == "$prefix"* ]] && printf '%s\n' "$n"; done
  else
    for n in "${TARGET_NAMES[@]}"; do [[ "$n" == "$glob" ]] && printf '%s\n' "$n"; done
  fi
}

# ── Resolve required targets + unmapped source surfaces ─────────────────────
# Populates REQUIRED (assoc set) and UNMAPPED (array).
resolve_diff() {
  declare -gA REQUIRED=()
  declare -ga UNMAPPED=()
  local f raw n i g t matched
  for raw in "$@"; do
    f="$(norm "$raw")"
    [[ -z "$f" ]] && continue
    matched=0
    for n in "${TARGET_NAMES[@]}"; do
      if [[ "${TARGET_PATH[$n]}" == "$f" ]]; then REQUIRED["$n"]=1; matched=1; fi
    done
    for i in "${!SURFACE_PATH[@]}"; do
      local sp="${SURFACE_PATH[$i]}"
      if [[ "$f" == "$sp" || "$f" == "$sp"* ]]; then
        IFS=',' read -ra globs <<< "${SURFACE_GLOBS[$i]}"
        for g in "${globs[@]}"; do
          while IFS= read -r t; do [[ -n "$t" ]] && REQUIRED["$t"]=1; done < <(expand_glob "$g")
        done
        matched=1
      fi
    done
    if [[ "$matched" -eq 0 && "$f" == src/* ]]; then UNMAPPED+=("$f"); fi
  done
}

git_changed() {
  { git -C "$REPO_ROOT" diff --name-only 2>/dev/null || true; \
    git -C "$REPO_ROOT" diff --name-only --cached 2>/dev/null || true; \
    git -C "$REPO_ROOT" diff --name-only origin/main...HEAD 2>/dev/null || true; \
    git -C "$REPO_ROOT" ls-files --others --exclude-standard 2>/dev/null || true; } | sort -u
}

# ── Compute changed set ─────────────────────────────────────────────────────
declare -a CHANGED=()
if [[ -z "$CHANGED_RAW" ]]; then
  if [[ "$MODE" != "completeness" ]]; then
    # Fail closed: an unresolvable base ref or a missing merge base means the
    # diff is indeterminate, not empty. Treating it as empty would report PASS
    # while omitting every required target (e.g. a shallow checkout where
    # origin/main...HEAD has no merge base). Require an explicit --changed then.
    if ! git -C "$REPO_ROOT" rev-parse --verify --quiet origin/main >/dev/null 2>&1; then
      echo "MODE=$MODE"
      echo "STATUS=FAIL"
      echo "REASON=cannot-resolve-base-ref-origin/main"
      echo "coverage oracle: cannot resolve base ref origin/main to compute the diff; pass --changed explicitly" >&2
      exit 3
    fi
    if ! git -C "$REPO_ROOT" merge-base origin/main HEAD >/dev/null 2>&1; then
      echo "MODE=$MODE"
      echo "STATUS=FAIL"
      echo "REASON=no-merge-base-with-origin/main"
      echo "coverage oracle: no merge base between origin/main and HEAD (shallow checkout?); pass --changed explicitly" >&2
      exit 3
    fi
    while IFS= read -r l; do [[ -n "$l" ]] && CHANGED+=("$l"); done < <(git_changed)
  fi
else
  IFS=',' read -ra parts <<< "$CHANGED_RAW"
  for p in "${parts[@]}"; do p="$(norm "$p")"; [[ -n "$p" ]] && CHANGED+=("$p"); done
fi

case "$MODE" in
  completeness)
    declare -A MAPPED=()
    for i in "${!SURFACE_PATH[@]}"; do
      sp="${SURFACE_PATH[$i]}"
      [[ "$sp" == src/* || "$sp" == crates/* ]] || continue
      IFS=',' read -ra globs <<< "${SURFACE_GLOBS[$i]}"
      for g in "${globs[@]}"; do while IFS= read -r t; do [[ -n "$t" ]] && MAPPED["$t"]=1; done < <(expand_glob "$g"); done
    done
    unmapped_targets=()
    for n in "${TARGET_NAMES[@]}"; do [[ -z "${MAPPED[$n]:-}" ]] && unmapped_targets+=("$n"); done
    modules=()
    while IFS= read -r e; do modules+=("src/$e"); done < <(
      for entry in "$REPO_ROOT"/src/*; do
        base="$(basename "$entry")"
        if [[ -d "$entry" ]]; then echo "$base"; elif [[ "$entry" == *.rs ]]; then echo "$base"; fi
      done)
    unmapped_modules=()
    for mod in "${modules[@]}"; do
      covered=0
      for sp in "${SURFACE_PATH[@]}"; do
        if [[ "$sp" == "src/$( basename "$mod" )" || "$sp" == "src/$( basename "$mod" )/" || "$sp" == "$mod/"* ]]; then covered=1; break; fi
      done
      [[ "$covered" -eq 0 ]] && unmapped_modules+=("$mod")
    done
    status="PASS"; { [[ ${#unmapped_targets[@]} -gt 0 || ${#unmapped_modules[@]} -gt 0 ]] && status="FAIL"; } || true
    echo "MODE=completeness"
    echo "TARGET_COUNT=${#TARGET_NAMES[@]}"
    echo "MODULE_COUNT=${#modules[@]}"
    echo "UNMAPPED_TARGETS_COUNT=${#unmapped_targets[@]}"
    echo "UNMAPPED_TARGETS=$(IFS=,; echo "${unmapped_targets[*]:-}")"
    echo "UNMAPPED_MODULES_COUNT=${#unmapped_modules[@]}"
    echo "UNMAPPED_MODULES=$(IFS=,; echo "${unmapped_modules[*]:-}")"
    echo "STATUS=$status"
    [[ "$status" == "PASS" ]] && exit 0 || exit 1
    ;;
  select)
    resolve_diff "${CHANGED[@]}"
    if [[ ${#REQUIRED[@]} -gt 0 ]]; then mapfile -t req < <(printf '%s\n' "${!REQUIRED[@]}" | sort); else req=(); fi
    echo "MODE=select"
    echo "REQUIRED_COUNT=${#req[@]}"
    for t in "${req[@]}"; do echo "TARGET=$t"; done
    if [[ ${#UNMAPPED[@]} -gt 0 ]]; then
      echo "UNMAPPED_COUNT=${#UNMAPPED[@]}"
      for u in "${UNMAPPED[@]}"; do echo "UNMAPPED=$u"; done
      echo "STATUS=FAIL"; exit 1
    fi
    echo "STATUS=PASS"; exit 0
    ;;
  report)
    resolve_diff "${CHANGED[@]}"
    if [[ ${#REQUIRED[@]} -gt 0 ]]; then mapfile -t req < <(printf '%s\n' "${!REQUIRED[@]}" | sort); else req=(); fi
    declare -A SELSET=()
    if [[ "$SELECTED_PROVIDED" -eq 1 ]]; then
      IFS=',' read -ra sels <<< "$SELECTED_RAW"
      for s in "${sels[@]}"; do s="$(norm "$s")"; [[ -n "$s" ]] && SELSET["$s"]=1; done
    else
      for s in "${req[@]}"; do SELSET["$s"]=1; done
    fi
    omitted=()
    for t in "${req[@]}"; do [[ -z "${SELSET[$t]:-}" ]] && omitted+=("$t"); done
    status="PASS"; { [[ ${#omitted[@]} -gt 0 || ${#UNMAPPED[@]} -gt 0 ]] && status="FAIL"; } || true
    echo "MODE=report"
    echo "REQUIRED_COUNT=${#req[@]}"
    echo "SELECTED_COUNT=${#SELSET[@]}"
    echo "OMITTED_COUNT=${#omitted[@]}"
    echo "OMITTED=$(IFS=,; echo "${omitted[*]:-}")"
    echo "UNMAPPED_COUNT=${#UNMAPPED[@]}"
    echo "UNMAPPED=$(IFS=,; echo "${UNMAPPED[*]:-}")"
    echo "STATUS=$status"
    [[ "$status" == "PASS" ]] && exit 0 || exit 1
    ;;
  run)
    resolve_diff "${CHANGED[@]}"
    if [[ ${#REQUIRED[@]} -gt 0 ]]; then mapfile -t req < <(printf '%s\n' "${!REQUIRED[@]}" | sort); else req=(); fi
    cap=$MAX_CONCURRENT; [[ "$cap" -lt 1 ]] && cap=1
    count=${#req[@]}
    peak=$(( count < cap ? count : cap )); [[ "$count" -eq 0 ]] && peak=0
    if [[ "$count" -eq 0 ]]; then batch_count=0; else batch_count=$(( (count + cap - 1) / cap )); fi
    if [[ ${#UNMAPPED[@]} -gt 0 ]]; then
      echo "MODE=run"; echo "REQUIRED_COUNT=$count"
      echo "UNMAPPED_COUNT=${#UNMAPPED[@]}"; echo "UNMAPPED=$(IFS=,; echo "${UNMAPPED[*]:-}")"
      echo "STATUS=FAIL"; exit 1
    fi
    echo "MODE=run"
    echo "REQUIRED_COUNT=$count"
    echo "MAX_CONCURRENT_CAP=$cap"
    echo "TEST_THREADS=$TEST_THREADS"
    echo "PEAK_CONCURRENT=$peak"
    echo "BATCH_COUNT=$batch_count"
    # The colocated library unit tests (`cargo test --lib`) run whenever any
    # target is required, matching the canonical `cargo dev-test` gate.
    run_lib=0; [[ "$count" -gt 0 ]] && run_lib=1
    echo "LIB_INCLUDED=$run_lib"
    if [[ "$DRY_RUN" -eq 1 ]]; then echo "DRY_RUN=1"; echo "STATUS=PASS"; exit 0; fi
    # Real bounded execution: fixed batches of at most $cap binaries. Each target
    # runs with its declared required-features so feature-gated targets actually
    # build and run instead of being silently skipped by cargo. Batching avoids
    # `wait -n` (absent in Bash 3.2) and the double-reap that miscounted failures.
    observed_peak=0; failed=0; idx=0
    if [[ "$run_lib" -eq 1 ]]; then
      ( cd "$REPO_ROOT" && cargo test --lib -- --test-threads="$TEST_THREADS" >/dev/null 2>&1 ) || failed=$((failed+1))
    fi
    while [[ $idx -lt $count ]]; do
      bpids=(); end=$(( idx + cap )); [[ $end -gt $count ]] && end=$count
      for (( j=idx; j<end; j++ )); do
        tname="${req[$j]}"; tfeat="${TARGET_FEATURES[$tname]:-}"
        if [[ -n "$tfeat" ]]; then
          ( cd "$REPO_ROOT" && cargo test --test "$tname" --features "$tfeat" -- --test-threads="$TEST_THREADS" >/dev/null 2>&1 ) &
        else
          ( cd "$REPO_ROOT" && cargo test --test "$tname" -- --test-threads="$TEST_THREADS" >/dev/null 2>&1 ) &
        fi
        bpids+=("$!")
      done
      [[ ${#bpids[@]} -gt $observed_peak ]] && observed_peak=${#bpids[@]}
      for pid in "${bpids[@]}"; do if ! wait "$pid"; then failed=$((failed+1)); fi; done
      idx=$end
    done
    echo "OBSERVED_PEAK_CONCURRENT=$observed_peak"
    echo "FAILED_TARGETS=$failed"
    [[ "$failed" -eq 0 ]] && { echo "STATUS=PASS"; exit 0; } || { echo "STATUS=FAIL"; exit 1; }
    ;;
  *) echo "unknown mode: $MODE" >&2; exit 2 ;;
esac
