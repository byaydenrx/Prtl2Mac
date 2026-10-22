#!/bin/sh
# Append one row to docs/AUTORUN_LOG.md without reading the file.
# Usage: sh scripts/log_run.sh <task> <outcome> <commit> <app build>
#   sh scripts/log_run.sh "T-203 light styles" "FIXED: style lightmaps blend; intro1 before/after shots" 1a2b3c4 "0.1.0+1a2b3c4.20261022T0300Z"
# The outcome starts with FIXED, PARTIAL, ATTEMPT, NOT REPRODUCED, STUCK or
# UNFINISHED (the log's header says what each means). AUTORUN_LOG=<file> overrides the target.
set -u
[ $# -eq 4 ] || { sed -n 2,6p "$0" | sed 's/^# \{0,1\}//'; exit 2; }
case $2 in
    FIXED*|PARTIAL*|ATTEMPT*|"NOT REPRODUCED"*|STUCK*|UNFINISHED*) ;;
    *) echo "outcome must start with FIXED, PARTIAL, ATTEMPT, NOT REPRODUCED, STUCK or UNFINISHED" >&2; exit 2 ;;
esac
root=$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel) || exit 1
log=${AUTORUN_LOG:-$root/docs/AUTORUN_LOG.md}
[ -f "$log" ] || { echo "no log at $log" >&2; exit 1; }

cell() { printf '%s' "$1" | tr '\n|' ' /'; } # a table cell can't hold a newline or a pipe
row="| $(date '+%Y-%m-%d %H:%M') | $(cell "$1") | $(cell "$2") | $(cell "$3") | $(cell "$4") |"
[ -n "$(tail -c1 "$log")" ] && echo >>"$log" # keep the last row on its own line
printf '%s\n' "$row" >>"$log"
echo "appended: $row"
