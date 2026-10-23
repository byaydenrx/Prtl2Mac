# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Autorun (2026-10-23 14:20)
The autorun no longer stops itself or uses a STOP file (docs/AUTORUN.md
"Keep going"). Blocked tasks are recorded and skipped; the owner ends the runs.
T-217, T-222, T-223 wait for the owner's replay (see Fix attempts below).

## Last update
2026-10-23 evening, manual session: T-220 REGRESSION FIXED (af7d368, app
0.1.0+f73361b.20261023T1844Z). The owner reported that a8fda9c "removed the
entire cube from the dropper" in bridge_the_gap (softlock). Bevy's clone
`deny` also denies components that require the denied one, so denying
avian's ColliderTreeProxyKey stripped the Collider from every template
copy (all droppers, all maps). Now only the key is denied. Shown by trace:
copies fell through before; they rest in the dropper after; the save-load
crash repro stays fixed (exit 0); 5 other dropper maps fine; 202 tests pass.
T-221 owner-confirmed, moved to docs/TASKS_DONE.md. T-145 jitter was
paused mid-investigation; findings are in its entry (new P2_PROPTRACE trace).
Autopilot screenshots were all black this session, old build too (capture
problem, not a code change).
Before that:
2026-10-23 05:42 autorun (ATTEMPT, docs only on main): T-217 laser_stairs:
the missing floor is already fixed on main by T-221 (all tiles flat on the
grid after the floor move; attempt/T-217 not needed). Wheatley: Kill didn't
remove entities parented to the target (sphere_bone -> sphere_peek01);
untested fix on local branch attempt/T-217-wheatley (978c28c). Not shown to
fix the owner's report. T-217 BLOCKED (needs owner). Details in TASKS.md.
Before that:
2026-10-23 05:27 autorun (NOT REPRODUCED, docs only): T-223 ricochet
rotating platform: panel_error_arm_03a draws, turns with its arm, is solid
and holds juggled_cube on build 18676be (also after error_arm_stop_relay);
likely fixed by T-221 (spawn sequence before the map-spawn
SetDefaultAnimation). T-223 BLOCKED (needs owner replay). No code changed.
Before that:
2026-10-23 05:19 autorun (NOT REPRODUCED, docs only): T-222 sphere_peek
fall-through platform: catapult_block1..4 floors and the floor_repair_1 arm
panels all end drawn and solid where they belong on build 18676be (likely
the T-221 bindpose fix). All three T-222 items now not reproduced, so T-222
is BLOCKED (needs owner replay). No code changed.
Before that:
2026-10-23 05:07 autorun (NOT REPRODUCED, docs only): T-222 sphere_peek
panel end pose: arm_panel_01/02 end at their Hammer-authored pose (angled
~45 deg, side shot) after arm_spherepeek_01/02, with build 18676be; most
likely the T-221 fix (spawn in DefaultAnim) already fixed it. Owner replay
needed. T-222 stays IN_PROGRESS (catapult2_up bounce Windows check and the
fall-through platform left). No code changed.
Before that:
2026-10-23 05:01 autorun (NOT REPRODUCED, docs only): T-222 sphere_peek
faith plate: a player standing on catapult_up/catapult2_up is launched at
once when GLaDOS re-enables or swaps it, by direct input and in the real
look-at-the-sphere flow. Noted a possible loop after the ceiling lowers
(catapult2_up bounces the player off ceiling_panel every ~1.9 s; check vs
Windows). T-222 stays IN_PROGRESS. No code changed.
Before that:
2026-10-23 04:48 autorun (FIXED, autopilot only): T-221 bridge_intro
invisible walls fixed in 18676be: props spawn in their DefaultAnim keyvalue;
a SetDefaultAnimation at map spawn no longer replaced it, so the floor-panel
brushes' maintained offset is taken in the right pose and they rise to floor
level (was eye level). Details in TASKS.md T-221.
Before that:
2026-10-23 04:38 autorun (UNFINISHED, docs only): T-221 bridge_intro invisible
walls REPRODUCED: the rising floor-panel func_brushes (parented with
SetParentAttachmentMaintainOffset to the arm props) end 64 units too high,
at eye level, so they read as invisible walls and the floor looks missing.
Likely offset taken from the wrong pose (bindpose vs spawn 64_idle). Next
step in TASKS.md T-221 (stays IN_PROGRESS). No code changed.
Before that:
2026-10-23 04:27 autorun (UNFINISHED, docs only): T-221 bridge_intro invisible
walls not reproduced yet in a short pass; arrival area and ceiling repair
arms render. Narrowed the "hallway animation" to the floor-panel rows
(row_[12]_panel_*) raised by floor_panel_up relays; next step written in
TASKS.md T-221 (stays IN_PROGRESS). No code changed.
Before that:
2026-10-23 04:22 autorun (FIXED, autopilot only): T-220 bridge_the_gap crash
fixed in a8fda9c: dropper copies cloned avian's broad-phase proxy key from
the cube prototype; after a save load that freed the prototype's proxy and
left a stale key that panicked on the next drop. Repro now passes; bridge
pickup still not reproduced. Details in TASKS.md T-220.
Before that:
2026-10-23 03:17 autorun (docs only, UNFINISHED): regression check on main
3a2bddb passed (build, tests, 4 map shots; a4_intro shot black twice, then
good on a third run: capture flake). T-220 crash REPRODUCED: load the
owner's bridge_the_gap autosave, drop a cube; avian panics when the cube
is killed ("no element at index 20 in StableVec"). Fresh loads don't crash.
Repro and next step in TASKS.md T-220. No code changed.
Before that:
2026-10-23 03:20 cloud session (docs only): TASKS.md reorganized (active only;
done -> docs/TASKS_DONE.md, later -> docs/TASKS_ON_HOLD.md, all 80 entries
kept); autorun rules in docs/AUTORUN.md (10-minute timebox). Before that:
2026-10-23 03:00 owner playtest 11 recorded (cloud session, docs only): chapter 2
finished, sphere_peek and ricochet played; bridge_intro BLOCKED (T-221),
bridge_the_gap crash (T-220). New T-220..T-228, T-173 back from hold.
Before that:
2026-10-23 owner session, PAUSED by the owner mid T-217. Done: T-216 FIXED
b9ac2b1; T-218 PARTLY FIXED 51c6aa7; T-145 PARTLY FIXED (c005fbc dark
cubes, bf5006a spikes; jitter open). T-217 missing floor: cause found and
fixed on branch attempt/T-217 (2fe2ac4, pushed to origin), regression
check not done; details in TASKS.md T-217.

## Where things stand
- dist/Prtl2Mac.app = 0.1.0+f73361b.20261023T1844Z (T-220 regression fix on top of 18676be).
- Owner to replay: T-222 sphere_peek: the two arm platforms should end
  angled, the GLaDOS faith plate should launch you while standing on it, and
  no platform should drop you (none reproduced by autopilot on this build;
  if one still happens, say where).
- Owner to replay: T-217 laser_stairs floor after GLaDOS's floor move
  (fixed by T-221); say where Wheatley stays visible through the wall.
- Owner to replay: T-223 ricochet: the turning platform under the cube at
  the start is visible and solid (not reproduced by autopilot on 18676be).
- Owner to replay first: T-220 bridge_the_gap: the dropper's cube should
  sit in the dropper and drop normally; load the autosave, drop a new cube,
  finish the map; try picking up a cube resting on a bridge. Any map with
  a cube dropper was affected by the a8fda9c build, so droppers elsewhere
  should work again too.
- Owner to replay: T-216 catapult_intro (grab the bouncing cube at eye
  height from the far plate, finish the map). T-218 fps with cl_showfps 1
  (4K: try Antialiasing off; keep v-sync on). T-145: cubes from droppers
  (fizzler_intro, laser_stairs) should look grey, not black; pressed
  buttons should have no spikes; the jitter is not fixed yet.
- Awaiting the owner: T-112 GLaDOS floor reflection,
  T-153 particles, T-150 motion blur + gun glow, T-213 PotatOS on the gun,
  T-198 faith plates, T-215 winref.sh.
- ON HOLD: menus (no menu code changes), screenshot tasks (T-173, T-174,
  T-116, T-152, T-151, T-154), T-164. T-145 is active again; T-147, T-149,
  T-120 confirmed done (playtest 10).
- Regression-check note: sp_a3_jump_intro's 8 s shot is sometimes washed
  out (mean 164 vs 18) on both the old and new builds; not a regression.

## Next action (manual and auto sessions)
TASKS.md "Priority order": T-145 jitter (findings and next steps in its entry), then the feel tasks. T-217, T-222
and T-223 are BLOCKED on owner replay
(optional: Windows check of the catapult2_up ceiling bounce). The owner plays the campaign himself; no level scripts.

## Uncommitted / in-flight work
None.
Scratch scripts used this session (not in the repo): Windows netcon/vscript
tracker (method in docs/STATUS.md "Measuring Windows Portal 2
numerically"), frame benchmarks (P2_FRAMELOG, WORKFLOW.md).

## Fix attempts awaiting the owner
- attempt/T-217 (origin) was deleted 2026-10-23: superseded by T-221 on main.
  Its commit 2fe2ac4 is recorded in TASKS.md T-217.
- On the owner's Mac only (not in this repo, not pushed), to delete or merge
  after a replay: attempt/T-217-wheatley (978c28c, Wheatley's Kill on
  children), attempt/T-212 (9a9373e, frame-0 spawn events). Older attempt/
  branches from before 2026-10-23 may also be there (T-164, T-151, T-112).

## Checkouts (2026-10-20)
- `Portal2-wheatley` (this one): level with origin/main; work happens here.
- `Portal2-rust-engine`: stuck mid-merge since Oct 19 (owner to decide; its
  one commit reached main as f2d9b71; a stash holds a 15-line animinfo
  debug addition). Leave it alone.
- `Portal2-ch35`, `Portal2-ch6-9`: clean, behind main, nothing unpushed.
- Disk: the other checkouts' target/ dirs (63 GB) were deleted 2026-10-20
  with the owner's OK; ~/Documents/Claude is ~9 GB (this checkout's target
  kept). Those checkouts rebuild from scratch if ever used again.
- GitHub branches: the 11 `cloud/*` branches (all merged into main) were
  deleted 2026-10-20. Tips, to recreate one with `git push origin
  <sha>:refs/heads/cloud/<name>`: aimers 0f09c0f, anim 1bb3e03, gels ac6b4a3,
  glass 3d62059, particles cf344e9, projtex 91503a8, proxies 6e173d1,
  saves ef01090, settings 353a151, tools a1053a1, ui 5fd9de8.
