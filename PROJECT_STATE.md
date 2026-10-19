# Project state

Last updated 2026-10-19. Keep this current; evidence goes in TASKS.md and
docs/STATUS.md.

## Release target (honest definition)
**Playable release ("v1")**: a native `Prtl2Mac.app` that, given the owner's
own Portal 2 install, plays the single-player campaign from New Game through
sp_a4_finale4 where, for every map:
1. **Load**: the map loads without crash or missing-world geometry.
2. **Objective**: its scripted sequence/puzzle logic runs (doors, buttons,
   lasers, bridges, funnels, plates, gels, scenes needed to progress).
3. **Solvable**: the puzzle can be solved with the intended mechanics
   (portals, cubes, lasers...) by a player, not only by teleporting.
4. **Portals**: the portal surfaces the solution needs accept portals and
   the player and objects pass through.
5. **No softlock** on the intended route.
6. **Next**: completing it loads the next map (or the chapter's end).
7. **Restart**: dying, reloading the autosave or restarting the level
   puts the map back in a playable state.

Not part of v1 (tracked as P3/P4): full visual fidelity (projected-texture
lighting, every particle, glass refraction), every non-blocking scripted
scene, co-op, commentary, multiplayer, Windows/Linux.

Today we can honestly claim: maps load and their entity logic runs, and most
reach their exit when the player is placed on the route. The owner's
playtest (2026-10-19) found chapter 1 blocked for a real player (ride end,
portal traversal, breaker lift), so nothing is playable end to end yet.
Autopilot runs that use `setpos`/`input` prove logic, not playability; S
needs a run with movement, portals and props only, or the owner's play.

## Build and package
- [x] `cargo build --release`, `cargo test --release --workspace` pass
      (~340 tests, 0 failures, 2026-10-19).
- [x] `sh scripts/make_app.sh` builds `dist/Prtl2Mac.app` (ad-hoc signed,
      build id in Contents/Resources/BUILD.txt, previous app kept as
      dist/Prtl2Mac-previous.app);
      launched with `open` and no environment it finds the install and runs.
      A missing install shows a dialog.
- [ ] App icon; first-run check on a second Mac.

## Map status
Columns: L load, O objective logic, S solvable (route played with mechanics),
N next map loads, R restart works. `y` proven, `-` not checked, `x` broken.
Status: VERIFIED = L O S N y (R may be `-` until T-090); IN PROGRESS = some
proven; NOT STARTED = nothing proven; BLOCKED = a known `x`.

| Ch | Map | L | O | S | N | R | Status | Evidence / notes |
|---|---|---|---|---|---|---|---|---|
| 1 | sp_a1_intro1 | y | y | - | - | - | IN PROGRESS | ride-end wall break/barrier fixed in logic (animation events, T-101), awaiting owner replay; many P2 issues (T-110..T-117) |
| 1 | sp_a1_intro2, intro3, intro5, intro6 | y | y | - | y | - | IN PROGRESS | button->door chains and exits (autopilot logic only); portal traversal broken (T-102) |
| 1 | sp_a1_intro4 | y | y | x | y | - | BLOCKED | owner playtest: can't walk through portals, cubes don't pass portals (T-102) |
| 1 | sp_a1_intro7 | y | y | - | y | - | IN PROGRESS | Wheatley pickup, socket, relay, scene (autopilot logic); Wheatley re-pickup broken (T-105) |
| 1 | sp_a1_wakeup | y | y | x | y | - | BLOCKED | owner playtest: breaker lift doesn't carry the player (T-103); crouch needed to enter (T-106); missing textures, music stops |
| 2 | sp_a2_intro | y | y | x | y | - | BLOCKED | owner playtest: lands with nowhere to go; blue screen at start (T-104) |
| 2 | sp_a2_laser_intro | y | y | y | y | - | VERIFIED | solved dbc2f19 |
| 2 | sp_a2_laser_stairs | y | y | y | y | - | VERIFIED | solved 3cf08c2 (f2d9b71) |
| 2 | sp_a2_dual_lasers | y | - | - | y | - | IN PROGRESS | exit reached by placement only (T-002) |
| 2 | sp_a2_laser_over_goo | y | - | - | y | - | IN PROGRESS | T-003 |
| 2 | sp_a2_catapult_intro | y | - | - | y | - | IN PROGRESS | plates land on target (check_faith_plates.py); T-004 |
| 2 | sp_a2_trust_fling | y | - | - | y | - | IN PROGRESS | plates land on target; T-005 |
| 2 | sp_a2_pit_flings | y | - | - | y | - | IN PROGRESS | plates land on target; T-006 |
| 2 | sp_a2_fizzler_intro | y | - | - | y | - | IN PROGRESS | T-007 |
| 3 | sp_a2_sphere_peek, ricochet, bridge_intro | y | y | y | y | - | VERIFIED | solved (ch3-5 session) |
| 3 | sp_a2_bridge_the_gap | y | - | - | y | - | IN PROGRESS | T-008 |
| 3 | sp_a2_turret_intro | y | - | - | y | - | IN PROGRESS | walk-in transition_trigger works; T-008 |
| 3 | sp_a2_laser_relays | y | - | - | y | - | IN PROGRESS | T-008 |
| 3 | sp_a2_turret_blocker, laser_vs_turret, pull_the_rug | y | - | - | y | - | IN PROGRESS | T-008 |
| 4 | sp_a2_column_blocker, laser_chaining, triple_laser, bts1, bts2 | - | - | - | - | - | NOT STARTED | T-010 |
| 5 | sp_a2_bts3..bts6, sp_a2_core | - | - | - | - | - | NOT STARTED | T-011 |
| 6 | sp_a3_00, sp_a3_01, sp_a3_03 | y | y | y | y | - | VERIFIED | I/O chains and transitions (ch6-9 session) |
| 6 | sp_a3_jump_intro | y | y | - | - | - | IN PROGRESS | buttons/doors proven; gel + portal climb not flown (T-012) |
| 6 | sp_a3_bomb_flings, crazy_box, transition01 | - | - | - | - | - | NOT STARTED | T-012 |
| 7 | sp_a3_speed_ramp, speed_flings, portal_intro, end | - | - | - | - | - | NOT STARTED | T-013 |
| 8 | sp_a4_intro .. sp_a4_jump_polarity (11 maps) | - | - | - | - | - | NOT STARTED | T-014 |
| 9 | sp_a4_finale1..finale4 | - | - | - | - | - | NOT STARTED | T-015 |

## Release blockers (open P0/P1)
- Owner playtest blockers T-102..T-106 (chapter 1 and sp_a2_intro); T-101 fixed, awaiting replay.
- Most chapter 2-3 puzzles unproven (T-001..T-008).
- Chapters 4-9 not swept (T-010..T-015).
- Restart/reload never checked per map (T-090).

## Systems present (merged into main)
Portals, cubes/buttons, lasers, bridges, funnels, faith plates, turrets,
gels, fizzlers, saves (autosave, quick save/load, death reloads), scenes and
captions, view controls/proxies, movies, auto-exposure, projected lights,
particles, glass, settings/menus, model animation events (FireUser/sounds). All `origin/cloud/*` branches are merged.
