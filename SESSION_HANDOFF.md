# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 11:38, autorun (prtl2mac-auto-resume-2). T-150 PARTIAL FIX
b8a2d76: intro3's fall splash (env_splash -> water_splash_03 +
Physics.WaterSplash) and the breakable-floor dust (env_ar2explosion ->
grey puff cloud) were not implemented at all; now they are (autopilot
before/after shots only). Motion blur/speed lines and the gun glow through
the wall after sp_a2_intro's fall are still open. Next: T-146.

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
- dist/Prtl2Mac.app = 0.1.0+b8a2d76.20261021T1537Z (T-150 intro3 splash + floor dust; before it T-174 elevator movies at full size and 24 fps; before it T-173 gun hold beam, carry anims, fizzler shake; before it T-153 GLaDOS wake-up white disc: $mod2x particle cards + Remap CP Speed to CP; T-148 frosted glass over portals before it).

## Next action
1. P4: T-146 intro1 half-stuck door (T-150 splash/dust PARTIAL FIX b8a2d76,
   awaiting owner replay, motion blur and gun glow through wall open; T-164/T-151 attempt branches await the owner; T-175 intro1 ride fog NOT REPRODUCED,
   wrong (non-master) fog controller by data, untested fix on attempt/T-175,
   blocked on owner; T-174 elevator videos FIXED
   b250e5f, awaiting owner replay; T-140 portal shake/sound NOT REPRODUCED,
   shake+sound fire in the real intro1 vault path, untested fix on
   attempt/T-140, blocked on owner; T-173 gun hold
   beam, carry anims and fizzler shake FIXED fd1310c, awaiting owner replay,
   gun spark particle still open; T-153 white
   particles FIXED 69426f1, awaiting owner replay, her floating arms and
   the wrong-way view still open in its entry; T-152 STUCK: bumped
   VertexLitGeneric path is a multi-run job, plan in its entry; T-117 shadows NOT CONFIRMED, untested
   fix on attempt/T-117, blocked on owner; T-148 glass FIXED dcc0c0e, awaiting
   owner replay; T-171 first-portal hitch PARTIAL FIX
   d4fd841, awaiting owner replay; T-113 hints FIXED 693f8ce,
   awaiting owner replay; T-147 intro2 black door NOT REPRODUCED,
   blocked on owner; T-149 not reproduced, blocked on owner shots;
   T-144 clipboard FIXED 910da9a, awaiting owner replay; T-112 untested fix
   on attempt/T-112, blocked on owner; T-139 music pieces, T-135 rubble
   open; T-172 blocked; T-195..T-197 awaiting owner replay).
2. Then T-164 (try attempt/T-164), ... /P4. Skip T-116, T-164, T-151 (owner).
MENUS FROZEN: only T-196/T-195/T-197 may touch menu code; no other menu
changes (owner, 2026-10-21). A separate session may port upstream engine
changes (Stink-O/Portal2-rust-engine) into main; merge origin/main often.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
- attempt/T-175 (1f4de9a): maps start on the master env_fog_controller
  (intro1 ride: light fog_intro_ride instead of the dense vault fog). Needs
  the fog inputs (SetColor/SetStartDist/SetEndDist) before merging, else the
  wrecked vault loses its dense fog. Replay the intro1 ride vs Windows.
- attempt/T-140 (e5e751c): view shake follows Source's CalcShake
  (sin(t*freq/fraction), amplitude decay). Replay intro1's vault portal
  opening vs Windows, then merge or delete.
- attempt/T-117 (31262e5): projected-texture shadows use a Source-style
  5x5 binomial PCF (softer penumbra, same cost). Replay intro1's planter
  shadow and an elevator arrival vs Windows, then merge or delete.
- attempt/T-112 (cc81fdb): world-reflecting water (GLaDOS chamber,
  water_glados_01) samples a mirrored half-res camera view with an oblique
  clip plane at the water. Small change at shallow angles; replay
  sp_a1_wakeup's chamber (and check FPS there), then merge or delete.
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
