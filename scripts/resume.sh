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

section() { # print a "## <name>..." section of a markdown file (heading may say more after <name>)
    awk -v h="## $2" 'index($0,h)==1{on=1;next} /^## /{on=0} on' "$1" | sed '/^$/d'
}

entry() { # print task <id>'s "### <id> ..." entry from the first file that has it
    id=$1; shift
    for f in "$@"; do
        [ -f "$f" ] || continue
        out=$(awk -v h="### $id " 'index($0,h)==1{on=1;print;next} on&&/^##/{exit} on' "$f")
        if [ -n "$out" ]; then echo "  ($f)"; echo "$out" | sed 's/^/  /'; return 0; fi
    done
    echo "  $id: no entry found in $*"
}

echo
echo "== Next action (SESSION_HANDOFF.md)"
if [ -f SESSION_HANDOFF.md ]; then section SESSION_HANDOFF.md "Next action" | sed 's/^/  /'
else echo "  SESSION_HANDOFF.md missing"; fi

echo
echo "== Priority order (TASKS.md)"
if [ -f TASKS.md ]; then
    # The open items only (done/blocked lines are skipped), first 12.
    section TASKS.md "Priority order" | grep -E '^[0-9]+[a-z]?\. ' | grep -viE 'FIXED|DONE|BLOCKED|awaiting' | head -12 | cut -c1-110 | sed 's/^/  /'
    n() { grep -c "^### .*$1" TASKS.md; }
    awaiting=$(grep -ciE '^### T-.*awaiting (owner )?replay' TASKS.md)
    held=$(awk '/^### T-/{c++} END{print c+0}' docs/TASKS_ON_HOLD.md 2>/dev/null); held=${held:-0}
    echo "  counts: $(n IN_PROGRESS) in progress, $(n TODO) todo, $(n BLOCKED) blocked, $awaiting awaiting owner replay, $held on hold or later (docs/TASKS_ON_HOLD.md)"
else echo "  TASKS.md missing"; fi

echo
echo "== Next task's entry"
if [ -f TASKS.md ]; then
    # An IN_PROGRESS task wins; else the priority order's first open item (the
    # first numbered line not marked FIXED/DONE/BLOCKED/awaiting). The priority
    # order is the one source of what's next; the handoff should agree with it.
    active=$(grep -m1 '^### T-[0-9]* .*IN_PROGRESS' TASKS.md | grep -oE 'T-[0-9]+' | head -1)
    top=$(section TASKS.md "Priority order" | grep -E '^[0-9]+[a-z]?\. ' | grep -viE 'FIXED|DONE|BLOCKED|awaiting' | grep -oE 'T-[0-9]+' | head -1)
    said=$( [ -f SESSION_HANDOFF.md ] && section SESSION_HANDOFF.md "Next action" | grep -oE 'T-[0-9]+' | head -1)
    id=${active:-$top}
    if [ -n "$id" ]; then entry "$id" TASKS.md docs/TASKS_ON_HOLD.md docs/TASKS_DONE.md
    else echo "  no open task in the priority order"; fi
    if [ -n "$said" ] && [ "$said" != "$id" ]; then
        why="top of the priority order"; [ -n "$active" ] && why="in progress"
        echo "  NOTE: the handoff's next action names $said, but the next task is $id ($why)."
        echo "  The priority order wins; update the handoff's next action."
    fi
fi

if [ $build = 1 ]; then
    echo
    echo "== Build"
    cargo=$(command -v cargo || echo "$HOME/.cargo/bin/cargo")
    if out=$("$cargo" build --release -q 2>&1); then echo "  build ok"
    else echo "$out" | tail -20; echo "  BUILD FAILED"; fi
fi

echo
echo "== For a NEW session only (this one has already run resume.sh)"
echo "  Resume Prtl2Mac in $root: read CLAUDE.md, run sh scripts/resume.sh once,"
echo "  then do the next task it prints."
