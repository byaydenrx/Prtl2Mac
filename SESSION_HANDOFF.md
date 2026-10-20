# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-20, autonomous runs (owner away, no playtests). Scheduled task
`prtl2mac-auto-resume` starts a fresh session hourly; each run does one task
(lock: ~/Library/Application Support/Portal2Rust/autopilot-session.lock).

## Where things stand
- Repo tidied 2026-10-20: TASKS.md is one entry per task with a "Priority
  order" at the top and an "Awaiting owner replay" list; raw playtests moved
  to docs/PLAYTESTS.md, finished-fix write-ups to docs/STATUS.md; durable
  gotchas moved to WORKFLOW.md; PROJECT_STATE map table updated from
  playtest 8 (owner played chapter 1 to sp_a2_laser_stairs).
- T-130 handrails fixed (grate models pass shots). T-129 fixed (movie screens mirrored; ring covered its exit). Owner replay
  pending, as for T-128. T-105 wall drop fixed (fd9c086, autopilot only).
  T-167 Wheatley re-pickup fixed (636fc3f, autopilot only, wakeup); held
  cube no longer covers the camera at walls (342b5b1, autopilot only, intro2).
  T-132 fizzler field drawn (99392c0, autopilot only, intro2 exit; startdisabled
  cleansers still unmeshed, see its entry).
  T-114 intro1 announcer now ducks under Wheatley (ambient_generic Volume
  input, 463757d, autopilot only). T-135 investigated 2026-10-20, not
  confirmed, BLOCKED (no code change). T-120 ride jitter fixed (movers drawn
  between ticks, c1079f9, autopilot trace only; P2_RIDE_TRACE logs it).
  T-120 elevator cars now play their move/stop sounds (5bfaf52, autopilot
  log only). T-133 Wheatley no longer floats after the socket ejects him
  (gravity restored on DisableMotion, 2542009, autopilot only).
  T-134 wakeup lift now turns the player with it (orientationtype 2 trains
  blend path yaws, 5cfe749, autopilot only).
  T-163 wakeup hatch: start-disabled brushes now get a disabled collider
  and hidden mesh, so basement_breakers_upper_blocker seals the hatch
  (8f8c8b1, autopilot only; all maps' startdisabled func_brushes affected).
  T-136 wakeup incinerator drop: linked_portal_doors now draw seamless (no
  oval/rim/hum; SeamlessPortal marker, 3f96088, autopilot shots only).
  dist/Prtl2Mac.app = 0.1.0+3f96088.20261020T1137Z.
- Depot 624 (Windows server.dll/client.dll) installed as behaviour reference.

## Next action
1. T-116 intro1 room details (next in the owner's order: T-116, T-117,
   T-115, T-153, T-157, ...).
   T-164 spiky debris BLOCKED (needs owner) 2026-10-20: cause confirmed by
   weights (intro1 finedebris rebar/rust/crumple meshes are soft-skinned,
   the rigid split tears them) but not visually reproduced (intro camera
   keeps the view). Untested fix on local branch attempt/T-164 (c34006e).
   sp_a2_intro arms are rigid: separate cause, still open in its entry.
   dist build unchanged: 0.1.0+b39fff3.20261020T1546Z.
   T-020 intro1 ride BLOCKED (needs owner) 2026-10-20: not reproduced; the
   ride is seen through point_viewproxy camera_1 (enabled by
   relay_start_second_cryo_wakeup), which relay shortcuts skip. No code
   change, no attempt branch; see its entry for the next repro step.
   dist build unchanged: 0.1.0+b39fff3.20261020T1546Z.
   T-162 PARTIAL 2026-10-20 (b39fff3): menu logo and item column now at
   Portal 2's basemodui_scheme.res / mainmenu.res positions; the clicked
   item no longer flashes unlit when the menu rebuilds. Awaiting owner
   replay (autopilot shots only); panel fade/slide timings still open.
   dist build: 0.1.0+b39fff3.20261020T1546Z.
   T-161 FIXED 2026-10-20 (f6f129e): menus out of the game play the save's
   act movie media/menu_act0N.bik (ffmpeg stream, still as fallback), music
   picks portal2_background0N by act. Awaiting owner replay (autopilot
   shots/log only, music not listened to).
   dist build: 0.1.0+f6f129e.20261020T1535Z.
   T-160 FIXED 2026-10-20 (ab64b84): main menu is Portal 2's six items,
   Single Player flyout has Challenge Mode, New Game is a vertical chapter
   list with the picked chapter's picture. Co-op, Extras, Challenge Mode do
   nothing yet. Awaiting owner replay (autopilot shots only).
   dist build: 0.1.0+ab64b84.20261020T1524Z.
   T-123 FIXED 2026-10-20 (2bb0015): security cameras tear off and fall as
   physics props on a portal shot through them, a portal opened over their
   wall, or the Ragdoll input. Awaiting owner replay (autopilot intro6
   only; laser_stairs not run; no wall decal or sound yet).
   dist build: 0.1.0+2bb0015.20261020T1520Z.
   T-112 PARTIAL 2026-10-20 (991441d): the wakeup GLaDOS chamber floor
   (shallow water_glados_01, $fogend 30) fogged through an assumed 64 units
   and drew as a flat yellow sheet; map water now fogs by its measured depth
   per vertex (32-unit grid), floor visible through the shallows. Awaiting
   owner replay (autopilot shots only; also glance at intro3's water).
   Hallway to GLaDOS, black walk-through wall, intro4 blue wall still open.
   dist build: 0.1.0+991441d.20261020T1511Z.
   T-149 PARTIAL 2026-10-20 (d314ea7): chamber progress signs
   (sp_progress_sign vgui_screens) are now drawn from sp_lightboard_icons.txt
   (number, NN/TT, bar, icons), awaiting owner replay (autopilot shots only;
   layout by eye, no flicker/dirt); intro5 screens and lights still open.
   dist build: 0.1.0+d314ea7.20261020T1458Z.
   T-139 audio PARTIAL 2026-10-20 (503406e): intro4's ramp glass break no
   longer plays at load (one-shot ambient_generics wait for PlaySound),
   awaiting owner replay (autopilot-checked only); soundscapes (no support
   at all yet), button clicks and music cut-offs still open.
   dist build: 0.1.0+503406e.20261020T1449Z.
   T-147 intro2 black screen as the first door opens (TODO, after the list).
   T-144 props fixed 2026-10-20 (89f2bf5: physics props' colliders now
   weigh the prop's 40 kg, so their inertia matches; the intro1 radio and
   clipboard stay put instead of sinking through the vault floor; every
   physics prop tumbles less, intro2's spawn cube now lies flat), awaiting
   owner replay (autopilot-checked only).
   dist build: 0.1.0+89f2bf5.20261020T1444Z.
   T-145 cube lit skins fixed
   2026-10-20 (2e8df1a: cubes on a pressed button switch to metal_box's lit
   skin and back), button press fixed (feb5072); both awaiting owner replay
   (autopilot-checked only).
   dist build: 0.1.0+2e8df1a.20261020T1236Z.
   T-125 turn-away delay is BLOCKED (needs owner) 2026-10-20: not reproduced;
   the aisc fires 1.5 s (RequiredTime) after turning and is only enabled
   0.2 s after Wheatley's "turn around" line ends, as the scene table says.
   dist build unchanged: 0.1.0+847a581.20261020T1213Z.
   T-102 cubes through portals fixed 2026-10-20 (847a581: props half in a
   portal are now drawn coming out of the exit), awaiting owner replay.
   T-137 portal spacing fixed 2026-10-20 (38c09bf, shots overlapping the
   other portal now bump beside it), awaiting owner replay.
   T-120 Elevators is now BLOCKED (needs owner): thumps, trash and VO timing
   all NOT REPRODUCED by autopilot (see its entry). Trash checked
   2026-10-20 (autorun): it is intro5's departure_elevator-tube_blockage_01
   (elevator_rubble.mdl, seq "anim" 11.2 s), started 1.75 s after the car's
   "arrive"; autopilot shows the car come down and the rubble fall onto its
   roof, as the model's animation intends. Needs the owner to say which
   ride and what looked wrong (or a Windows reference). Small leftover:
   movesound pitch by speed (movesoundmin/maxpitch).
   T-136 tube portal, T-134 pod turn and T-163 hatch done 2026-10-20,
   awaiting owner replay (also replay bts3/bts5/a3_end blend trains, which
   now turn, and any map with startdisabled func_brushes).
   T-135 (vac-tube turrets through the exit elevator) is BLOCKED: the model's
   own elevator_0N animation drops them down the shaft through the car;
   needs a Windows Portal 2 reference of that moment (see its entry).
   dist build: 0.1.0+847a581.20261020T1213Z.
2. T-131 (intro3 floors/roofs) is BLOCKED: not reproduced by autopilot;
   needs owner screenshots plus console `getpos`.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
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
