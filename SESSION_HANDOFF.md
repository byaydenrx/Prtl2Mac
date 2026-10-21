# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 15:16 autorun: T-150 motion blur FIXED (new motion_blur.rs,
Source-style image-space blur: turning + falling, radial when looking
down), autopilot-checked only, awaiting owner replay. T-150's gun glow
through the wall is still open.

## Where things stand
- dist/Prtl2Mac.app = 0.1.0+7cc1096.20261021T1915Z.
- Awaiting the owner: T-112 GLaDOS floor reflection, T-153 particles,
  T-150 motion blur.
- ON HOLD: menus (no menu code changes), screenshot tasks (T-173, T-174,
  T-145, T-147, T-149, T-116, T-152, T-151, T-154), T-164, T-120.

## Next action (auto sessions; owner away 24-48 h from 2026-10-21 evening)
Follow TASKS.md "Priority order": T-212 sp_a3_00 glass smash at load,
T-213 PotatOS on the gun, T-150 gun glow (motion blur done 7cc1096), T-198 faith plates, then the common
engine gaps (T-203 lights, T-199 sprite sheets, T-214 DecalModulate/
SolidEnergy/broken models, T-201, T-206, T-200, T-202, T-204), T-211 gels,
then the campaign T-002..T-015. Audit docs: docs/UPSTREAM_GAPS.md, T-214.
Tools added: p2_fx examples `pcfdump` (one system) and `fxaudit` (unsupported
operators over map effects). ON HOLD: menus, screenshot tasks, T-164, T-120.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
None: all merged into main for the owner's replay (2026-10-21).

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
