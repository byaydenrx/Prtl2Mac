# Map routes

One file per proven map, `<map>.txt`: the autopilot script that solves it.
This is the evidence TASKS.md's "Prove a map" asks for. It shows the map is
solvable; the map only becomes VERIFIED in PROJECT_STATE.md once the owner
has played it. No map has one yet (2026-10-21).

A route plays the intended solution from the map's start with movement,
`fire`, `use` and props: no `setpos` past a puzzle step and no `input` on
puzzle entities. The exit must open from the puzzle's own outputs and the
next map must load. Read the map's intended solution in `docs/maps/<map>.md`
first, if it exists.

Format: the same commands as `P2_SCRIPT` (WORKFLOW.md "Running the game"),
one `<seconds> <command>` per line; `#` starts a comment line.

```
# sp_a2_laser_intro: <one line on the solution>
3 look 0 90
3.5 walk 1.2
5 fire blue
...
40 quit
```

Run one (from the repo root):

```sh
P2_SCRIPT="$(grep -v '^#' scripts/routes/sp_a2_laser_intro.txt | paste -sd ';' -)" \
  perl -e 'alarm 120; exec @ARGV' ./target/release/portal2 sp_a2_laser_intro
```
