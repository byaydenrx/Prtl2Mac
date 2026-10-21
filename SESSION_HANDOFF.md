# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 08:52, autorun (prtl2mac-auto-resume-2). T-135 PARTIAL: the
departure elevators' vac-tube turrets were never killed (anim event 1100
fired with no activator, so tube_object_kill's `!activator | kill` missed)
and kept falling through the car; fixed 41e3839 (autopilot log only).
Rubble open (needs owner). Next: T-139.

## Where things stand
- Upstream checked 2026-10-21: no code changes since our base v0.1.0, only a
  GPL-3.0 relicense (18230b3), not ported. Code taken from upstream from now
  on would be GPL-3.0-only (docs/STATUS.md "Upstream port").
- Menu/intro spec T-177..T-192 done 2026-10-21 (see each entry): main
  menu layout + greyed items + Robot Enrichment (T-178), quit dialog
  (T-191), menu sounds (T-180), Options pending/Apply/Back/Use Defaults/
  discard + keyboard navigation (T-184), Audio/Video/Advanced/Controller/
  Keyboard-Mouse/Edit Keys rows (T-185..T-189), sliders (T-190), Load Game
  dates/pictures (T-183), tile flips (T-179), startup intro (T-177, built
  by a subagent, merged), 4:3 layout (T-192). Open pieces listed at the
  top of TASKS.md's priority order.
- Earlier fixes still awaiting the owner: T-176, T-169, T-162, T-170.
- dist/Prtl2Mac.app = 0.1.0+efe0631.20261021T1257Z (T-139 soundscapes; T-135 vac-tube turrets before it).

## Next action
1. T-112 GLaDOS chamber floor murky brown (T-139 soundscapes fixed
   efe0631, music pieces still open; T-135 rubble open; T-172 blocked;
   T-195..T-197 awaiting owner replay).
2. Then T-144, T-149, T-147, T-113,
   then P3/P4. Skip T-116, T-164, T-151 (owner).
MENUS FROZEN: only T-196/T-195/T-197 may touch menu code; no other menu
changes (owner, 2026-10-21). A separate session may port upstream engine
changes (Stink-O/Portal2-rust-engine) into main; merge origin/main often.

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
