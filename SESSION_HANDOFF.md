# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-19 evening, sprint session (worktree `Portal2-wheatley`, branch
`wheatley`, pushes to `main`).

## Where things stand
- T-101 (intro1 ride end) fixed in logic: model animation events
  (`FireUser1..4`, sounds) now fire; math_counter OutValue passes its value,
  so `exit_wall_break_case` breaks the wall and opens the blocking doors.
  Autopilot log proves the chain; a player has NOT yet walked off the
  container to sp_a1_intro2. Status NEEDS_VERIFICATION (owner replay).
- Animation events are new for every map: re-check verified maps for new
  sounds/inputs that misbehave (none seen; only intro1 was run).
- `cargo test --release --workspace` passes (~340 tests, 0 failures).
- make_app.sh keeps `dist/Prtl2Mac-previous.app` and writes a build id to
  `Contents/Resources/BUILD.txt`.

## Next action
1. If the owner's replay of intro1 passes, mark T-101 DONE; if not, take their report.
2. T-102 (sp_a1_intro4: player can't walk through portals, cubes don't pass
   portals): core mechanic, blocks most of the campaign. Then T-103..T-106.
Verify with walking/portal runs, not setpos/input. Autopilot screenshots can
come out black when the window is covered; re-take rather than debug.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Disk (2026-10-19)
`~/Documents/Claude` is ~71 GB, ~70 GB of it Rust `target/` dirs:
Portal2-ch6-9 27 GB, Portal2-rust-engine 26 GB, Portal2-ch35 10 GB,
Portal2-wheatley 7.5 GB (in use). Portal 2 itself (12 GB) lives in Steam's
folder, not here. The other checkouts' targets are regenerable but await
the owner's approval (owner asked to leave those checkouts untouched).

## Checkouts and sessions (checked 2026-10-19)
- `Portal2-wheatley` (this one): up to date with origin/main; the sprint works here.
- `Portal2-rust-engine` (branch `main`): stuck mid-merge since 00:45 Oct 19
  (merging origin/main at 11a60b7; props.rs conflicted), 41 behind
  origin/main. Its one local commit 3cf08c2 (laser_stairs) reached main as
  f2d9b71 (same change, slightly reworked). A stash "local animinfo (other
  session)" holds a 15-line animinfo.rs debug addition. Don't touch until
  the owner decides; nothing unique found besides that stash.
- `Portal2-ch35`, `Portal2-ch6-9`: clean, 6 / 8 behind origin/main, nothing unpushed.
- All `origin/cloud/*` branches are ancestors of origin/main.
## Owner manual checks wanted (T-021)
sp_a1_intro1 ride, sp_a1_wakeup socket, sp_a2_intro portal route.

## Gotchas
- Departure elevators only arrive after the player walks through
  `departure_elevator-source_elevator_door_open_trigger`; some exits are
  walk-in `transition_trigger`s whose volume isn't at their origin
  (`info <name>` prints bounds).
- Proven placement sequences (exits only): catapult_intro
  `3 setpos -64 -1512 -400; 9 setpos -64 -2126 -575`; sphere_peek
  `3 setpos -544 1176 500; 9 setpos -544 562 316`; laser_relays
  `3 setpos -320 -1192 52; 9 setpos -320 -1806 -132`; turret_intro
  `3 setpos -336 392 -250`.
- Faith plates with usethresholdcheck=1 only redirect things arriving fast.
