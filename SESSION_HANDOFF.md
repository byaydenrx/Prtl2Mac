# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 16:50 autorun: T-215 DONE: scripts/winref.sh takes reliable
Windows Portal 2 reference shots (netcon console over TCP, waits for the
load to finish). See WORKFLOW.md "Windows Portal 2 reference". First
finding: laser_intro start is much darker in ours than in Windows.
Earlier: T-198 faith plates (b55fc08). Ghidra project of server.dll is not
kept; re-import takes ~10 min (JAVA_HOME=/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home).

## Where things stand
- dist/Prtl2Mac.app = 0.1.0+b55fc08.20261021T2016Z.
- Awaiting the owner: T-112 GLaDOS floor reflection, T-153 particles,
  T-150 motion blur + gun glow, T-213 PotatOS on the gun, T-198 faith plates,
  T-215 winref.sh.
- ON HOLD: menus (no menu code changes), screenshot tasks (T-173, T-174,
  T-145, T-147, T-149, T-116, T-152, T-151, T-154), T-164, T-120.
- Regression-check note: sp_a3_jump_intro's 8 s shot is sometimes washed
  out (mean 164 vs 18) on both the old and new builds; not a regression.

## Next action (auto sessions; owner away 24-48 h from 2026-10-21 evening)
Follow TASKS.md "Priority order": the common engine gaps (T-203 lights, T-199 sprite
sheets, T-214 DecalModulate/SolidEnergy/broken models, T-201, T-206, T-200,
T-202, T-204), T-211 gels, then the campaign T-002..T-015. Audit docs:
docs/UPSTREAM_GAPS.md, T-214. Tools: p2_fx `pcfdump`, `fxaudit`; mdlinfo now
lists bodygroup alternates; scripts/winref.sh for Windows reference shots. ON HOLD: menus, screenshot tasks, T-164, T-120.

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
