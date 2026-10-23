# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
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
- dist/Prtl2Mac.app = 0.1.0+a8fda9c.20261023T0821Z (T-216, T-218, T-145, T-220 crash fixes).
- Owner to replay: T-220 load the bridge_the_gap autosave, drop a new cube,
  finish the map; try picking up a cube resting on a bridge.
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
TASKS.md "Priority order": T-221
(bridge_intro invisible walls; check T-222/T-223 with it), T-222, T-223,
then finish T-217 (attempt/T-217 regression check), T-145 jitter, then the
feel tasks. The owner plays the campaign himself; no level scripts.

## Uncommitted / in-flight work
None. T-217 work in progress is on branch attempt/T-217 (pushed).
Scratch scripts used this session (not in the repo): Windows netcon/vscript
tracker (method in docs/STATUS.md "Measuring Windows Portal 2
numerically"), frame benchmarks (P2_FRAMELOG, WORKFLOW.md).

## Fix attempts awaiting the owner
- attempt/T-217 (2fe2ac4): laser_stairs floor tiles (pushed; regression
  check left).
- attempt/T-212 (9a9373e): skip frame-0 events of a spawn default sequence
  (sp_a3_00 glass smash at load). Not pushed.

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
