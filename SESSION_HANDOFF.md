# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 (owner session). Owner round 6: T-169, T-171, T-148, T-135,
T-139, T-162 confirmed (Done); menus fixed "for now" and ON HOLD (no menu
work). T-175 finished: run-time fog (fog.rs: SetFogController blend +
SetColor/SetStartDist/SetEndDist, ValveFog override) ac96ab1. Merged for the
owner's replay: attempt/T-175, attempt/T-164, attempt/T-112 (plus T-140 and
T-117 earlier).

## Where things stand
- dist/Prtl2Mac.app = BUILD_ID: every fix awaiting the owner is in it
  (T-175 fog, T-164 debris, T-112 GLaDOS floor, T-140 shake, T-117 shadows,
  T-153 GLaDOS wake-up).
- Owner screenshots coming: T-173 hold beam, T-174 elevator panels, T-145,
  T-147, T-149, T-116.
- No attempt/* branches left.

## Next action
1. Wait for / record the owner's replay of the build above (every item in
   TASKS.md). If T-164 or T-112 are worse, revert their merge commits
   (355882a, 2c08d11) with `git revert -m 1`.
2. Then T-146 intro1 half-stuck door, then the priority order.
MENUS ON HOLD. Merge origin/main often.

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
