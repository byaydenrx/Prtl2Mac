#!/bin/sh
# Run a command (usually cargo) and print only what a session needs: errors
# with their location lines, test failures, panics, warning totals and
# summary lines. The full output is kept in a temp file whose path is printed
# last, so it can still be grepped. Exits with the command's own status.
# Usage: sh scripts/quiet.sh cargo test --release -p p2_game
set -u
[ $# -gt 0 ] || { sed -n 2,6p "$0" | sed 's/^# \{0,1\}//'; exit 2; }

log=$(mktemp "${TMPDIR:-/tmp}/quiet.XXXXXX") || exit 1
"$@" >"$log" 2>&1
status=$?

pick='^error|^failures:|^---- |panicked at|^test result:|generated [0-9]+ warnings?|^ *Finished |FAILED|^thread .* has overflowed'
picked=$(grep -E -A4 "$pick" "$log" | grep -v '^--$')
lines=$(printf '%s\n' "$picked" | grep -c .)
if [ "$lines" -gt 0 ]; then
    printf '%s\n' "$picked" | head -150
    [ "$lines" -gt 150 ] && echo "... ($lines lines matched, first 150 shown)"
elif [ $status -ne 0 ]; then
    echo "(no error lines matched; last 30 lines)"
    tail -30 "$log"
fi
echo "exit $status · $(wc -l <"$log" | tr -d ' ') lines in full log: $log"
exit $status
