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
1. T-102 seamless cube carry, then T-125 turn-away delay.
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
None.

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
