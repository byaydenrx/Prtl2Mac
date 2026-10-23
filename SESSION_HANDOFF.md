# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-23 owner session (owner's order T-216, T-218, T-145, T-217):
T-216 FIXED b9ac2b1 (cube throw matches Windows). T-218 PARTLY FIXED
51c6aa7 (per-view culling of Valve surfaces; MSAA-at-4K and v-sync-off
findings in its entry). Working on T-145 next.

## Where things stand
- dist/Prtl2Mac.app = 0.1.0+51c6aa7.20261023T0427Z (T-216 + T-218).
- Owner to replay: T-216 sp_a2_catapult_intro: ride the far plate across
  and grab the cube at the top of its bounce over the middle plate (now at
  eye height, as in Portal 2), then finish the map. T-218: play with the fps
  counter on (cl_showfps 1); expect higher fps, most of all below 4K; at 4K
  try Antialiasing off; keep v-sync on (off caps a window at 60).
- Awaiting the owner: T-112 GLaDOS floor reflection,
  T-153 particles, T-150 motion blur + gun glow, T-213 PotatOS on the gun,
  T-198 faith plates, T-215 winref.sh.
- ON HOLD: menus (no menu code changes), screenshot tasks (T-173, T-174,
  T-116, T-152, T-151, T-154), T-164. T-145 is active again; T-147, T-149,
  T-120 confirmed done (playtest 10).
- Regression-check note: sp_a3_jump_intro's 8 s shot is sometimes washed
  out (mean 164 vs 18) on both the old and new builds; not a regression.

## Next action (manual and auto sessions)
Owner's order: T-216 and T-218 done (awaiting replay); next T-145 (cubes on
buttons), then T-217 (laser_stairs). Nothing else until the
owner says. The owner plays the campaign himself: fix what he reports, no
level scripts.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
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
