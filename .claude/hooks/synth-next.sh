#!/usr/bin/env bash
# Synth SessionStart hook.
# Reports the spec-to-PR pipeline state and the single next action, so Claude
# starts each session knowing where work stands. The plan (ordered spec list)
# comes from .synth/tasks.json; live merge/PR state is derived from git + gh.
# Degrades gracefully: missing gh/auth just drops the open-PR section.
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || cd .

TASKS=".synth/tasks.json"
[ -f "$TASKS" ] || exit 0
command -v jq >/dev/null 2>&1 || exit 0

log="$(git log --oneline -n 300 2>/dev/null || true)"
open_prs="$(gh pr list --state open --json number,title,headRefName 2>/dev/null || echo '[]')"

rows=""
next_action=""

# Each spec: FS-NNN \t title \t planned(true|false)
while IFS=$'\t' read -r id title planned; do
  [ -n "$id" ] || continue
  slug="$(printf '%s' "$id" | tr '[:upper:]' '[:lower:]')"   # FS-005 -> fs-005
  spec_file="docs/specs/${id}/spec.md"

  spec_done=false; [ -f "$spec_file" ] && spec_done=true
  impl_done=false; printf '%s' "$log" | grep -qiE "feat\(${id}\)" && impl_done=true

  open_spec_pr="$(printf '%s' "$open_prs" | jq -r --arg s "docs/${slug}" 'map(select(.headRefName|startswith($s+"-") or .==$s)) | .[0] | if . then "#\(.number)" else "" end')"
  open_impl_pr="$(printf '%s' "$open_prs" | jq -r --arg s "synth/${slug}" 'map(select(.headRefName|startswith($s+"-") or .==$s)) | .[0] | if . then "#\(.number)" else "" end')"

  if [ -n "$open_impl_pr" ]; then
    state="impl PR ${open_impl_pr} open — review & merge"
    action="Get implementation PR ${open_impl_pr} for ${id} reviewed and merged before starting new work."
  elif [ "$impl_done" = true ]; then
    state="implemented ✓"
    action=""
  elif [ -n "$open_spec_pr" ]; then
    state="spec PR ${open_spec_pr} open — review & merge, then implement"
    action="Merge spec PR ${open_spec_pr} for ${id}, then implement it."
  elif [ "$spec_done" = true ]; then
    impl_branch="$(grep -oE 'synth/fs-[0-9]+[a-z0-9-]*' "$spec_file" 2>/dev/null | head -1)"
    [ -n "$impl_branch" ] || impl_branch="synth/${slug}-<slug>"
    state="spec merged — ready to implement"
    action="Implement ${id} (${title}). Branch: ${impl_branch}. Verify with bun run build, bun run test, cargo test --manifest-path src-tauri/Cargo.toml; open PR: feat(${id}): ${title}."
  elif [ "$planned" = true ]; then
    state="planned — spec not written"
    action="Write the ${id} spec PR first. Copy docs/templates/feature-spec.md to ${spec_file} on branch docs/${slug}-<slug>; open PR: Add ${id} ${title} spec."
  else
    state="spec file missing"
    action="No spec file for ${id}; write its spec PR or remove it from the backlog."
  fi

  rows="${rows}  ${id}  ${title} — ${state}"$'\n'
  # First spec that still needs action drives "next" (pipeline is sequential).
  if [ -z "$next_action" ] && [ -n "$action" ]; then
    next_action="$action"
  fi
done < <(jq -r '.specs[] | [.id, (.title // ""), (.planned // false | tostring)] | @tsv' "$TASKS")

open_summary="$(printf '%s' "$open_prs" | jq -r 'if length==0 then "none" else (map("\(.title) [\(.headRefName)]") | join("; ")) end')"
[ -n "$open_summary" ] || open_summary="unavailable (gh not authed)"

ctx="Synth spec-to-PR pipeline (computed at session start):
${rows}Open PRs: ${open_summary}

▶ Next: ${next_action:-All tracked specs are implemented and merged. Add the next spec to .synth/tasks.json (planned: true) and write its spec PR.}

Reminder (see CLAUDE.md): a spec ships as a spec PR then an implementation PR; each PR must be merged before the next step, and merged specs are immutable (deviations become amendments)."

jq -cn --arg ctx "$ctx" '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $ctx}}'
