# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 00:30, owner session. Autorun `prtl2mac-auto-resume` PAUSED by
the owner: they are playtesting the whole game several times and taking
Windows Portal 2 (CrossOver) comparison shots for UI, textures and
lighting; weekly usage limit until Sunday. Merged attempt/T-145 (companion/
rusted lit skins) at the owner's request. Next session: file the owner's
playtest 10 report and comparisons in TASKS.md first.

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
- T-162 all screens DONE (last: loading screens 0bc1dd1, new loading.rs:
  act art, spinner, dot bar; level loads wait 2 frames so it is drawn),
  autopilot shots only. Moved to Awaiting owner replay.
- T-170 carrying at walls FIXED (9f1ed5a), autopilot intro2 only: the held
  prop blocks the player from its own position (pinned by a wall, along
  the wall's normal) instead of T-167's view ray; looking up pushes the
  cube over the head up the wall. Moved to Awaiting owner replay.
- T-145 BLOCKED (needs owner): the lit cube on a pressed button renders
  dark (autopilot intro2 reproduced). Texture, shader combo, baked light
  and sRGB cache ruled out; suspect the render path of SkinVariant draws
  spawned Hidden. Next check written in the T-145 entry. Spikes under the
  button and jitter not looked at yet.
- T-145 skin table (cc8d8e0) merged 2026-10-21, untested in game.
- dist/Prtl2Mac.app = see the make_app line in the latest commit message.

## Next action
1. T-172 late VO (stretches elevator rides). Then T-135, T-139, T-112,
   T-144, T-149, T-147, T-113, then P3/P4. New crashes/softlocks still
   first. T-145's dark-lit-cube cause, spikes and jitter remain (see its
   entry). Skip T-116 (owner screenshots coming), T-164, T-151 (attempt
   branches await the owner).

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
