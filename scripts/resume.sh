#!/bin/sh
# Start-of-session summary: git state of every worktree, the handoff's next
# action, the active task, and a prompt to paste into a fresh session.
# Read-only (only `git fetch`). Usage: sh scripts/resume.sh [--no-fetch] [--build]
set -u
root=$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel) || exit 1
cd "$root" || exit 1

fetch=1 build=0
for a in "$@"; do
    case $a in
        --no-fetch) fetch=0 ;;
        --build) build=1 ;;
        -h|--help) sed -n 2,4p "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $a" >&2; exit 2 ;;
    esac
done

[ $fetch = 1 ] && { git fetch -q origin 2>/dev/null || echo "(fetch failed; showing local state)"; }

echo "== Worktrees (vs origin/main)"
git worktree list --porcelain | sed -n 's/^worktree //p' | while read -r wt; do
    [ -d "$wt" ] || { echo "  $wt: missing (prunable)"; continue; }
    br=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null)
    counts=$(git -C "$wt" rev-list --left-right --count origin/main...HEAD 2>/dev/null)
    behind=${counts%%	*}; ahead=${counts##*	}
    dirty=$(git -C "$wt" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
    gd=$(git -C "$wt" rev-parse --absolute-git-dir)
    state=""
    [ -f "$gd/MERGE_HEAD" ] && state=" MERGE-IN-PROGRESS"
    [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ] && state=" REBASE-IN-PROGRESS"
    mark=""; [ "$wt" = "$root" ] && mark=" (this checkout)"
    echo "  $(basename "$wt") [$br]$mark: ${ahead:-?} ahead, ${behind:-?} behind, $dirty changed$state"
done

echo
echo "== This checkout"
git log -1 --format='  HEAD %h %s (%cr)'
git status --short | sed 's/^/  /' | head -20

section() { # print a "## <name>" section of a markdown file
    awk -v h="## $2" '$0==h{on=1;next} /^## /{on=0} on' "$1" | sed '/^$/d'
}

echo
echo "== Next action (SESSION_HANDOFF.md)"
if [ -f SESSION_HANDOFF.md ]; then section SESSION_HANDOFF.md "Next action" | sed 's/^/  /'
else echo "  SESSION_HANDOFF.md missing"; fi

echo
echo "== Active task (TASKS.md)"
if [ -f TASKS.md ]; then
    active=$(grep '^### .*IN_PROGRESS' TASKS.md)
    if [ -n "$active" ]; then echo "$active" | sed 's/^### /  /'
    else
        echo "  none in progress; first TODO (the priority order's top P1):"
        grep -m1 '^### .*TODO)' TASKS.md | sed 's/^### /  /'
    fi
    n() { grep -c "^### .*$1" TASKS.md; }
    awaiting=$(awk '/^## Awaiting owner replay/{on=1;next} /^## /{on=0} on && /^- T-/' TASKS.md | wc -l | tr -d ' ')
    echo "  counts: $(n IN_PROGRESS) in progress, $(n TODO) todo, $(n BLOCKED) blocked, $awaiting awaiting owner replay"
else echo "  TASKS.md missing"; fi

if [ $build = 1 ]; then
    echo
    echo "== Build"
    cargo=$(command -v cargo || echo "$HOME/.cargo/bin/cargo")
    if out=$("$cargo" build --release -q 2>&1); then echo "  build ok"
    else echo "$out" | tail -20; echo "  BUILD FAILED"; fi
fi

echo
echo "== Resume prompt (paste into a new session)"
echo "  Resume Prtl2Mac in $root. Follow WORKFLOW.md: run sh scripts/resume.sh,"
echo "  verify the handoff, then continue the next action in SESSION_HANDOFF.md."
