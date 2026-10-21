# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 (owner session). Owner round 7: T-175 run-time fog, T-140 shake,
T-117 shadows confirmed (Done); T-153 gun drop/arms confirmed, some particles
remain; T-164 CPU skinning REVERTED (e88ae41: ride parts shook). Screenshot
tasks (T-173, T-174, T-145, T-147, T-149, T-116) and menus are ON HOLD.

## Where things stand
- dist/Prtl2Mac.app = 0.1.0+250359f.20261021T1841Z (T-164 revert on top of 6e0efbf).
- Awaiting the owner: T-112 GLaDOS floor reflection (merged 2c08d11).

## Next action
1. T-153 remaining particles around GLaDOS at wake-up (owner: fix for sure).
2. T-150 motion blur/speed lines + gun glow through the wall after the
   sp_a2_intro fall (owner: fix for sure).
ON HOLD: menus, screenshot tasks, T-164, T-120. Don't redo fixed tasks.
T-146 door wiggle FIXED (oscillator logic_timer), awaiting owner replay.

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
