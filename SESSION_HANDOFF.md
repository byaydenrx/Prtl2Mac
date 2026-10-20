# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-20 15:15, owner session. The owner sent playtest 9 (build 014ab6e):
all of it is recorded in TASKS.md (new T-168..T-175, reopened T-145, T-148,
T-140, T-117, T-162; confirmed fixes moved to Done, write-ups to
docs/STATUS.md; raw words in docs/PLAYTESTS.md). The owner is away again;
scheduled task `prtl2mac-auto-resume` re-enabled with the new order (every
10 min, one task per run, lock in ~/Library/Application Support/Portal2Rust/
autopilot-session.lock).

## Where things stand
- T-168 FIXED (98b74ad): pressing sp_a2_laser_over_goo's box button
  crashed (Kill reached a template prototype, ForceSpawn cloned a despawned
  entity). Autopilot only; awaiting owner replay.
- dist/Prtl2Mac.app = 0.1.0+98b74ad.20261020T1911Z.
- The T-150 autorun at 13:51 hit the usage limit and left a stale lock and a
  scratch example (both removed); T-150 was not started.

## Next action
1. T-169 sp_a2_laser_over_goo: GLaDOS never moves the walls at the entry
   (P1 softlock; owner screenshot 2026-10-20 2.45.36 PM, sign 04/22).
   Then the TASKS.md "Priority order": T-170 carry regression, T-145 cubes
   black/spikes/jitter, T-162 UI styled like Portal 2 (owner priority),
   T-172 late VO, T-135, T-139, T-112, T-144, T-149, T-147, T-113, then P3/P4.
   Skip T-116 (owner screenshots coming), T-164 and T-151 (attempt branches
   await the owner).

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
- attempt/T-151 (2df932d): water reflects the (LDR-clamped) sky on fogged
  maps and glass/water get the map's distance fog. intro3 water goes from
  flat brown to grey with flowing ripples; compare with Windows Portal 2's
  intro3 pool / sp_a2_laser_intro, then merge or delete.
- attempt/T-164 (c34006e): CPU skinning for soft-weighted animated meshes
  (intro1 ride debris). Review `git log -p wheatley..attempt/T-164`, replay
  the intro1 ride, then `git merge attempt/T-164` or delete the branch.

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
