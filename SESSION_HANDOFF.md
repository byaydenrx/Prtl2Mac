# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-19 night, sprint session (worktree `Portal2-wheatley`, branch
`wheatley`, pushes to `main`).

## Where things stand
- T-101 DONE: owner played sp_a1_intro1 through the ride to sp_a1_intro2
  (build 0.1.0+921edff).
- T-102 (portal traversal) fixed in autopilot, NEEDS_VERIFICATION by the
  owner: four causes (cube listed as a portal wall then fizzling it; held
  cube dragged/double-mapped; floor portals left the lower brush layer
  solid; dynamic props mirrored as static walls). Details and evidence in
  TASKS.md T-102.
- `cargo test --release --workspace` passes (0 failures).
- Owner's earlier bug list is all in TASKS.md (T-102..T-106, T-110..T-118).

## Next action
1. Owner replays sp_a1_intro2..intro4 with portals and cubes (T-102).
2. T-103 (wakeup breaker lift doesn't carry the player), then T-105
   (Wheatley re-pickup), T-106 (crouch into the breaker room), T-104
   (sp_a2_intro). Reproduce with autopilot first; verify by walking.
- Debug aids added: `RUST_LOG=p2_movement=trace` logs the ground entity;
  `p2_transit=trace` logs mirrored exit geometry; `mdlinfo` prints `.phy` pieces.

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
