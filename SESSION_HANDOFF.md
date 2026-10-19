# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-19, Wheatley session (worktree `Portal2-wheatley`, branch `wheatley`,
pushes to `main`).

## Where things stand
- main builds; ~340 workspace tests pass (2026-10-19).
- Workflow set up: WORKFLOW.md, TASKS.md, PROJECT_STATE.md, this file,
  CLAUDE.md, scripts/resume.sh.
- Current task: **T-101** (intro1 ride end). The owner playtest (2026-10-19) found chapter 1 blockers T-101..T-106.

## Next action
Fix the owner-playtest blockers in play order: T-101 (intro1 ride end: wall
doesn't break, invisible barrier), T-102 (portal traversal for the player
and cubes), T-103 (wakeup breaker lift), T-104 (sp_a2_intro), T-105, T-106.
Verify with walking/portal runs, not setpos/input, then ask the owner to replay.
Autopilot screenshots sometimes come out fully black when the game window
is covered; the owner sees it rendering fine. Re-take the shot rather than
debugging the renderer.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Checkouts and sessions
- `Portal2-wheatley` (this session): chapter 1-2 proof, packaging, workflow.
- `Portal2-rust-engine`: main checkout of the chapter 2 session; it had an
  unfinished `git merge` of cloud/particles (props.rs conflicted). Don't
  touch it; that session resolves it.
- `Portal2-ch35`, `Portal2-ch6-9`: chapter sessions, clean, nothing unpushed.

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
