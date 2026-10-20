# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-20 17:27, autorun (prtl2mac-auto-resume). T-162 Controller screen done and pushed.

## Where things stand
- T-176 FIXED (2bf3d20), autopilot-checked only: use only took a prop hit
  exactly by a thin ray, so flying cubes / falling Wheatley were near
  impossible to grab. Now falls back to the prop nearest the crosshair in a
  ~26 degree cone, in reach and line of sight. Not replayed on the real
  faith-plate throw or Wheatley's drop.
- T-169 FIXED (a1cd026), autopilot-checked only: sp_a2_laser_over_goo's
  entry corridor repair trigger was rotated (angles 0 90 9) and placed
  unrotated, so the walls never moved; Kill on a func_door was ignored
  (blocking_door_2 stayed solid). Trigger boxes now follow entity angles,
  Kill despawns doors. Player walks from the elevator to the chamber landing.
- T-168 FIXED (98b74ad), owner-confirmed.
- T-162 main menu screen DONE (e6e5e91), autopilot shots only: menu font
  is now the install's own UniversLTStd-Cn (decoded from
  vgui/fonts/universc.vfont), MainMenuButton colours/size/inset from
  basemodui_scheme.res, flyout owner stays lit. T-162 stays open.
- T-162 Options dialog DONE (2a6921d), Controller DONE (903118b), autopilot shots only.
- dist/Prtl2Mac.app = 0.1.0+f475223.20261020T2204Z.

## Next action
1. T-162 next screen: loading screens (menus, Settings pages, pause menu,
   HUD crosshair, chapter title, instructor hints and subtitles are done;
   see resource/ui/basemodui/loadingprogress.res and the reference
   gallery's loading shot). That is the last T-162 screen; one screen per
   run, new crashes/softlocks still first.
   Then T-170 carry regression, T-145 cubes black/spikes/jitter, T-172 late
   VO, T-135, T-139, T-112, T-144, T-149, T-147, T-113, then P3/P4.
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
