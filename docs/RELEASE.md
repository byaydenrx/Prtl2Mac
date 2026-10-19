# Release checklist (72-hour playable sprint, started 2026-10-19)

Target A (achievable): a native `Prtl2Mac.app` that plays the campaign from
sp_a1_intro1 onward with every map loading and progression never blocked,
chapters 1-3 verified start to finish, later chapters playable with a
known-issues list.
Target B (not claimed): full fidelity with the original campaign (effects,
every scripted scene, all 9 chapters verified).

## Build and package
- [x] `cargo build --release` and `cargo test --release --workspace` pass
      (~340 tests, 0 failures, 2026-10-19).
- [x] `sh scripts/make_app.sh` builds `dist/Prtl2Mac.app`; launched with
      `open` and an empty environment it finds Portal 2 (Steam libraries or a
      Steam-console depot) and opens. A missing game shows a dialog.
- [ ] App icon; first-run check on a second Mac.

## Level status
VERIFIED = evidence its required gameplay works and the next map loads
(autopilot runs unless noted "manual"). IN PROGRESS = loads and exits, puzzle
route not proven. NOT STARTED = untested.

| Map | Status | Evidence / notes |
|---|---|---|
| sp_a1_intro1 | IN PROGRESS | wake-up, exercises, bed, Wheatley door, space to talk, ride scenes all run (autopilot); ride visuals rough; needs a manual play |
| sp_a1_intro2..intro6 | VERIFIED | button -> door chains and exits (autopilot, 2026-10-18) |
| sp_a1_intro7 | VERIFIED | Wheatley pickup, socket, powered relay, scene (autopilot) |
| sp_a1_wakeup | VERIFIED (autopilot) | socket fixed 2026-10-19 (core trigger reach); breakers -> GLaDOS -> claw -> incinerator -> sp_a2_intro; needs a manual play |
| sp_a2_intro | VERIFIED (logic) | after the fall, the pedestal gives the dual gun (blue and orange both place); the exit trigger opens door_0; next map loads. The portal route out of the gun pit wasn't flown: manual check |
| sp_a2_laser_intro | VERIFIED | solved (dbc2f19) |
| sp_a2_laser_stairs | VERIFIED | solved (3cf08c2, rescued into main as f2d9b71) |
| sp_a2_dual_lasers .. sp_a2_fizzler_intro | IN PROGRESS | exits work; faith plates land on target (catapult_intro, trust_fling, pit_flings) |
| sp_a2_sphere_peek, ricochet, bridge_intro | VERIFIED | solved (ch3-5 session) |
| sp_a2_bridge_the_gap .. sp_a2_pull_the_rug | IN PROGRESS | exits work |
| sp_a2_column_blocker .. sp_a2_core (ch4-5) | NOT STARTED | |
| sp_a3_00, sp_a3_01, sp_a3_03 | VERIFIED | I/O chains and transitions (ch6-9 session) |
| sp_a3_jump_intro | IN PROGRESS | buttons/doors proven; gel + portal climb not flown |
| sp_a3_bomb_flings .. sp_a4_finale4 | NOT STARTED | |

## Release blockers (P0/P1)
- [ ] Chapters 2-3 puzzle routes not all proven (see table).
- [ ] Chapters 4-9 untested.
- [ ] sp_a1_intro1 needs a manual playthrough (owner reported ride jitter).

## Sessions and ownership (2026-10-19)
- `Portal2-wheatley` worktree (this session): chapter 1, packaging, release.
- `Portal2-rust-engine` main checkout: chapter 2 session; it has an
  unfinished `git merge` of cloud/particles in progress (props.rs
  conflicted). Its one unpushed commit (3cf08c2) is now on main.
- `Portal2-ch35`, `Portal2-ch6-9` worktrees: chapter sessions (both at their
  usage limit; resume with a message).

## Resume here (next session)
- State: branch `wheatley` worktree = origin/main (all pushed). Tests pass.
- Next task: prove the chapter 2 puzzles after laser_stairs in play order
  (dual_lasers, laser_over_goo, catapult_intro, trust_fling, pit_flings,
  fizzler_intro), then chapter 3's bridge_the_gap..pull_the_rug. Autopilot
  helpers: `info <name>`, `portalable x y z r`, `ray x y z dx dy dz`,
  `goto/lookat <name>`, `fire blue|orange`, `input <ent> <input>`.
- Build the app: `sh scripts/make_app.sh` -> dist/Prtl2Mac.app.
- Manual checks wanted from the owner: sp_a1_intro1 ride, sp_a1_wakeup
  socket, sp_a2_intro portal route.
