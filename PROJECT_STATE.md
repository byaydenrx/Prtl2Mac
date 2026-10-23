# Project state

Last updated 2026-10-23 (owner playtest 11: chapter 2 finished, chapter 3
started). Keep this current; evidence goes in TASKS.md and
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

Today we can honestly claim (playtest 10, 2026-10-22, build 0634859): the
owner played all of chapter 1 and chapter 2 through sp_a2_laser_over_goo;
sp_a2_catapult_intro was blocked (T-216, fixed since, awaiting replay). Before that, playtest 8 (2026-10-20): intro6
needed noclip (T-130) and laser_stairs' stairs didn't rise (T-128, fixed
since, awaiting replay). Many P2-P4 issues remain (TASKS.md). Autopilot runs
that use `setpos`/`input` prove logic, not playability.

## Reference install (2026-10-20)
- Installed: Steam depot 621 (Portal 2 common content). It holds every map,
  model, material, sound, the menu videos (media/menu_act01-05.bik) and the
  menu layouts (resource/ui/basemodui/*.res). This content is the same on
  Windows and Mac, so a Windows download adds no assets.
- Depot 624 "Portal 2 client binaries" installed 2026-10-20 (owner):
  depot_624/portal2/bin/server.dll (game logic), client.dll (client effects,
  UI), matchmaking.dll; Windows PE32 (x86). Engine/shader DLLs (engine.dll,
  materialsystem, shaderapidx9) are in another depot, not installed. The
  game still mounts depot_621 for content. Use: reverse engineering to
  understand behaviour, own code only (WORKFLOW owner rules; confirmed by the
  owner 2026-10-21; never commit decompiled code).

## Build and package
- [x] `cargo build --release`, `cargo test --release --workspace` pass
      (344 tests, 0 failures, 2026-10-20).
- [x] `sh scripts/make_app.sh` builds `dist/Prtl2Mac.app` (ad-hoc signed,
      build id in Contents/Resources/BUILD.txt, previous app kept as
      dist/Prtl2Mac-previous.app);
      launched with `open` and no environment it finds the install and runs.
      A missing install shows a dialog.
- [ ] App icon; first-run check on a second Mac.

## Map status
Columns: L load, O objective logic, S solvable (route played with mechanics),
N next map loads, R restart works. `y` proven, `-` not checked, `x` broken.
Status: VERIFIED = the owner played the map through (owner's rule,
2026-10-21) and L O S N are y (R may be `-` until T-090); autopilot runs and
autopilot runs are supporting evidence only, so a map Claude
solved stays IN PROGRESS until the owner plays it. IN PROGRESS = some
proven; NOT STARTED = nothing proven; BLOCKED = a known `x`.

| Ch | Map | L | O | S | N | R | Status | Evidence / notes |
|---|---|---|---|---|---|---|---|---|
| 1 | sp_a1_intro1 | y | y | y | y | y | VERIFIED | owner replayed (playtest 10, 2026-10-22); later: sky tint, ride debris, window |
| 1 | sp_a1_intro2, intro3, intro5 | y | y | y | y | y | VERIFIED | owner replayed (playtest 10); cubes on buttons T-145 (jitter fixed 2026-10-23, owner to replay); intro5 vine T-154 |
| 1 | sp_a1_intro6 | y | y | y | y | - | VERIFIED | owner replayed after the T-130 fix (0634859), 2026-10-22 |
| 1 | sp_a1_intro4 | y | y | y | y | y | VERIFIED | owner replayed (playtest 10) |
| 1 | sp_a1_intro7 | y | y | y | y | y | VERIFIED | owner replayed (playtest 10); turn-away slow (T-125) |
| 1 | sp_a1_wakeup | y | y | y | y | y | VERIFIED | owner replayed (playtest 10); floor water look, leftover particles (later) |
| 2 | sp_a2_intro | y | y | y | y | y | VERIFIED | owner replayed (playtest 10) |
| 2 | sp_a2_laser_intro | y | y | y | y | y | VERIFIED | owner played through (playtest 10) |
| 2 | sp_a2_laser_stairs | y | y | y | y | y | VERIFIED | owner played through (playtest 10); T-217 floor, Wheatley through wall, floating texture |
| 2 | sp_a2_dual_lasers | y | y | y | y | y | VERIFIED | owner played through (playtest 10) |
| 2 | sp_a2_laser_over_goo | y | y | y | y | y | VERIFIED | owner: played "the level after" dual_lasers (playtest 10) |
| 2 | sp_a2_catapult_intro | y | y | y | y | y | VERIFIED | owner played through after T-216 (playtest 11) |
| 2 | sp_a2_trust_fling | y | y | y | y | y | VERIFIED | owner played through (playtest 11); cube missed a plate once, button drops hit or miss (T-224) |
| 2 | sp_a2_pit_flings | y | y | y | y | y | VERIFIED | owner played through (playtest 11); no dissolve effect (T-225) |
| 2 | sp_a2_fizzler_intro | y | y | y | y | y | VERIFIED | owner played through (playtest 11) |
| 3 | sp_a2_sphere_peek, ricochet | y | y | y | y | y | VERIFIED | owner played through (playtest 11); bugs T-222, T-223 |
| 3 | sp_a2_bridge_intro | y | y | y | y | y | VERIFIED | owner playtest 12: playable, visuals fixed (T-221) |
| 3 | sp_a2_bridge_the_gap | y | y | y | y | y | VERIFIED | owner playtest 12: playable; Wheatley fixed since (T-232), bird open |
| 3 | sp_a2_turret_intro | y | y | y | y | y | VERIFIED | owner playtest 12: completable; turrets don't shoot or tip over but do damage (T-229) |
| 3 | sp_a2_laser_relays | y | y | y | y | y | VERIFIED | owner playtest 13: verified and playable (T-230) |
| 3 | sp_a2_turret_blocker | y | y | y | y | y | VERIFIED | owner playtest 13: verified and playable; "turret functions still arent finished" (T-229) |
| 3 | sp_a2_laser_vs_turret | y | y | x | y | - | IN PROGRESS | owner playtest 13: lasers didn't pass glass (T-238, fixed since); owner to replay |
| 3 | sp_a2_pull_the_rug | y | y | y | y | y | VERIFIED | owner playtest 12b: verified and playable |
| 4 | sp_a2_column_blocker | y | - | x | y | - | BLOCKED | owner playtest 12b: laser angled over the next cube (T-231, fixed since), grate blocks lasers/turret (T-233), turret doesn't explode (T-234), light (T-235) |
| 4 | sp_a2_laser_chaining, triple_laser | y | y | y | y | y | VERIFIED | owner playtest 13: verified and playable; triple_laser invisible barrier behind a portal (T-240, not blocking) |
| 4 | sp_a2_bts1 | y | y | x | y | - | IN PROGRESS | owner playtest 13: Wheatley's breakout didn't start (T-239, fixed since); owner to replay |
| 4 | sp_a2_bts2 | - | - | - | - | - | NOT STARTED | T-010 |
| 5 | sp_a2_bts3..bts6, sp_a2_core | - | - | - | - | - | NOT STARTED | T-011 |
| 6 | sp_a3_00, sp_a3_01, sp_a3_03 | y | y | y | y | - | IN PROGRESS | I/O chains and transitions by autopilot with setpos/use (ch6-9 session); owner hasn't played them (was VERIFIED before the 2026-10-21 rule) |
| 6 | sp_a3_jump_intro | y | y | - | - | - | IN PROGRESS | buttons/doors proven; gel + portal climb not flown (T-012) |
| 6 | sp_a3_bomb_flings, crazy_box, transition01 | - | - | - | - | - | NOT STARTED | T-012 |
| 7 | sp_a3_speed_ramp, speed_flings, portal_intro, end | - | - | - | - | - | NOT STARTED | T-013 |
| 8 | sp_a4_intro .. sp_a4_jump_polarity (11 maps) | - | - | - | - | - | NOT STARTED | T-014 |
| 9 | sp_a4_finale1..finale4 | - | - | - | - | - | NOT STARTED | T-015 |

## Release blockers (open P0/P1)
- T-239 bts1 breakout and T-238 laser_vs_turret glass fixed; owner to replay.
- T-229: turrets mostly working (turret_blocker verified); owner to say what is still missing.
- T-211 gels (P1) before chapters 6-8; gel isn't kept in saves (saves.rs).
- (The 2026-10-20 list here is closed: T-129 and T-132 fixed by autoruns,
  T-131 owner-confirmed, T-105 carried on as T-167/T-170 (T-170
  owner-confirmed); see TASKS.md Done and docs/AUTORUN_LOG.md.)
- Most chapter 2-3 puzzles unproven (T-002..T-008).
- Chapters 4-9 not swept (T-010..T-015).
- Restart/reload never checked per map (T-090).

## Systems present (merged into main)
Portals, cubes/buttons, lasers, bridges, funnels, faith plates, turrets,
gels, fizzlers, saves (autosave, quick save/load, death reloads), scenes and
captions, view controls/proxies, movies, auto-exposure, projected lights,
particles, glass, settings/menus, model animation events (FireUser/sounds). The cloud/* feature branches were merged into main and deleted (tips in SESSION_HANDOFF.md).
