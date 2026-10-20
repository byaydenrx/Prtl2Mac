# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (WORKFLOW.md). Work the "Priority order" top to bottom; never P3/P4
while a P0/P1 is open. One entry per task: fold new reports into it. When a
task is fixed, give its cause and evidence in two or three lines, move it
to "Awaiting owner replay"; once the owner confirms, move it to Done. Long
investigation notes go in docs/STATUS.md; the owner's raw reports in
docs/PLAYTESTS.md (history only, everything there is tracked here).

"Prove a map" (acceptance for T-002..T-015): from the map's start, an
autopilot script plays the intended solution with movement, `fire`, `use`
and props (no `setpos` past a puzzle step, no `input` on puzzle entities);
the exit opens from the puzzle's own outputs; the next map loads. Save the
script as `scripts/routes/<map>.txt`. Engine bugs found become tasks.

Owner playtest sections (playtest 8): 0 intro1, 1 intro2, 2 intro3, 3 intro4,
4 intro5/intro6, 5 intro7 + sp_a1_wakeup + sp_a2_intro, 6 sp_a2_laser_intro,
7 sp_a2_laser_stairs.

## Priority order (2026-10-20)
P1:
1. T-131 sp_a1_intro3 floors and roofs missing (BLOCKED: not reproduced, needs owner screenshots).
P2:
2. (T-114 fixed, awaiting owner replay.)
3. T-120 Elevators is BLOCKED (thumps, trash, VO timing not reproduced; needs owner); T-135 turrets through the exit elevator is BLOCKED (needs owner reference).
4. (T-133 Wheatley float fixed, awaiting owner replay; panel part not reproduced.)
5. (T-134 pod turn and T-163 hatch fixed, awaiting owner replay.)
6. (T-136 wakeup tube portal fixed, awaiting owner replay.)
7. (T-137 portal spacing and T-102 cube exit-side drawing fixed, awaiting owner replay.) T-125 turn-away delay is BLOCKED (map's own timing; not reproduced, needs owner).
8. (T-145 button press and cube lit skins fixed, awaiting owner replay.)
9. (T-144 props fixed, awaiting owner replay.) T-147 black screen as intro2's first door opens.
10. T-139 Audio: loud intro4 glass fixed (503406e, awaiting owner replay); soundscapes, button clicks, music cut-offs still open (PARTIAL, next run skips to T-149).
11. (T-149 chamber progress signs drawn, d314ea7, awaiting owner replay); intro5 screens and lights still open.
12. T-112 Missing/flat textures: GLaDOS chamber floor water fixed (991441d, awaiting owner replay); hallway to GLaDOS, black walk-through wall, intro4 blue wall still open (PARTIAL, next run skips to T-123).
13. (T-123 security cameras knocked off by portal shots, 2bb0015, awaiting owner replay.)
14. (T-160 main menu like Portal 2's, ab64b84, awaiting owner replay.)
P3:
15. (T-161 menu movie and music by act, f6f129e, awaiting owner replay; T-162 logo/item layout and hover flash, b39fff3, awaiting owner replay, panel fades still open.)
16. (T-020 intro1 ride BLOCKED: not reproduced, needs the camera proxy path or an owner recording; T-164 spiky debris BLOCKED: untested fix on attempt/T-164); T-116 intro1 room BLOCKED: not reproduced, needs a Windows screenshot; (T-117 shadows fixed, d1df637, awaiting owner replay); T-115 wake-up BLOCKED: not reproduced, the map has no get-up/lie-down rig in normal play (blackout is a removed template), needs a Windows recording.
17. T-153 GLaDOS wake-up: portal gun drop fixed (c64a60b, awaiting owner replay); particles, floating arms, look direction still open (PARTIAL); T-157 fall transition: 3.6 s black wait cut to 0.25 s (badb7a9, awaiting owner replay), a ~2.5 s frame hitch after the load still open (PARTIAL); T-165 Wheatley rail connector BLOCKED: not reproduced, arm props drawn, needs an owner screenshot.
18. (T-148 portals behind glass fixed, cc04a74, awaiting owner replay); (T-140 env_shake for game-placed portals, eb58eac, awaiting owner replay).
P4:
19. T-151 water, T-155 lasers, T-150 particles, T-152 lighting/textures,
    T-143 clock blink, T-146 half-stuck door, T-154 floating moss, T-166 intro5 wall panel.
Then the Backlog (prove and sweep the campaign).

## Open: P1

### T-131 sp_a1_intro3: floors and roofs missing  (P1, BLOCKED: not reproduced, needs owner screenshots)
- Section 2: the floor right in front of the door leaving the arrival
  elevator room and the roof outside it; the roof after the gun climb until
  the next gun use, then the floor there; the floor entering the exit
  elevator room. Check drawn vs solid (visibility, areaportals, brush entities).
- 2026-10-20 autopilot (HEAD 314cb7a), not reproduced: walking out of the
  arrival elevator (feet -160,-142,1051 -> -160,481,1104; corridor floor
  z 1104, its underside/ceiling ray-hit at 1072, white_wall_tile004b) shows
  floor and roof on both sides of door_0, looking +-40..75 deg both ways;
  `goto portalgun 120` and `goto emitter_orange_2 200` (first gun use)
  floors drawn, overhead is the map's open ruin (foliage, dark void);
  exit room (`goto departure_elevator-source_elevator_door_open_trigger
  200`) stairs, floor and the entry door drawn. No PVS/areaportal culling
  exists in our renderer (all world faces always drawn), so a hole would be
  a missing face or a frustum-culling error; PortalProjection's far plane
  and oblique clip only apply to portal cameras. Ask the owner for
  screenshots with the console `getpos` (or the exact spot); "until the
  next gun use" hints at something tied to portal placement/rendering.

### T-108 A chamber door had an invisible barrier, needed noclip  (P1, NEEDS INFO)
- intro4 doors pass both ways in a walk test. Need the door and map from the owner.

### T-107 Cubes and floor buttons invisible  (P1, CAN'T REPRODUCE)
- Owner screenshot 8 (intro2, cube held above the button): corner tiles
  drawn, red top and ring gone, a black ellipse in the pit, a straight
  diagonal edge cutting the model, like a portal clip plane applied to it.
  Not reproduced (load, changelevel, cube toggled 14 times, BindPose, views
  from above). Next: intro2 portals active, cube carried through onto the
  button. Playtest 8: cubes and buttons drawn.

## Open: P2

### T-120 Elevators  (P2, BLOCKED: needs owner; jitter and car sounds fixed)
- Jitter FIXED 2026-10-20 (c1079f9, see Awaiting owner replay). Still open:
  the VO after intro5's second button started
  halfway down and stretched the ride (T-138); thumping sounds during the
  ride (section 4); the trash rising with the elevator is buggy; one ride
  reached the bottom before the next map loaded.
- 2026-10-20 autorun: VO timing / ride length NOT REPRODUCED (no code change).
  How it works: path_tracks call ReadyForTransition() (sp_elevator_motifs.nut);
  it only leaves once the exit VO's scene fires TransitionReady(), else the
  car rides the whole shaft to the failsafe node (intro5: ~20 s at 200 u/s,
  same logic as the original). Autopilot: intro2 with PuzzleCompleted() leaves
  at the first path_track (~1 s into the ride); intro5 with the cube put into
  the box_filter trigger (64 448 264) starts testchamber03 0.6 s later, so the
  VO is not delayed by our scene/script chain. Differences seen: our new line
  cuts off the still-playing start line (testchamber02) instead of waiting;
  unchecked whether a cube resting on the real button overlaps that trigger
  (owner said the line came only halfway down). Needs an owner replay of
  intro5's second button + `log` timing to go further.
- 2026-10-20 autorun: thumps NOT REPRODUCED. Every world/player sound during
  the intro5 and intro6 arrival and departure rides was logged
  (`RUST_LOG=portal2::audio=debug,p2_fx=debug`): no landing/slam/impact
  sounds; the only impacts are the chamber's robot arms (World.Robot*,
  positional, gain 0.05-0.3 by distance). Found instead that the cars were
  silent: func_tracktrain movesound/stopsound never played. FIXED (5bfaf52,
  see Awaiting owner replay). Open: thumps (owner: which ride, what it
  sounded like), trash rising with the car, move-sound pitch by speed
  (movesoundmin/maxpitch, not done).
- 2026-10-20 autorun: trash NOT REPRODUCED (no code change). The trash is
  intro5's departure_elevator-tube_blockage_01 (models/elevator/
  elevator_rubble.mdl, solid 6, holdanimation 1): blocked_elevator_tube_anim
  (trigger at -64 816 352) plays the car body's "arrive" (car comes DOWN,
  root z 972 -> 110 in ~2 s) and 1.75 s later the rubble's "anim" (11.2 s;
  29 root bones fall from model z 469..1464 to about -90..99, i.e. onto the
  car roof). Autopilot (setpos -64 1150 300, look -40 90; shots at 3.5-15 s)
  shows exactly that: car arrives, rubble drops and rests on the roof. The
  departure ride with the rubble on the roof was not checked (needs the
  puzzle solved). Possible owner-visible issue: the rubble is a held
  prop_dynamic, so when the car leaves upward it passes through the pile
  (same entities in the original; unknown whether Windows hides it).
  BLOCKED: needs the owner to say which ride and what looked wrong, or a
  Windows reference of intro5's departure.

### T-135 Turrets fall through the exit elevator  (P2, BLOCKED: needs owner)
- Sections 5 (sp_a2_intro) and 6 (sp_a2_laser_intro).
- 2026-10-20 autorun (HEAD abc5618), cause not confirmed. The turrets are
  `departure_elevator-vacturret_01..04` (prop_dynamic,
  models/props_vac_anim/turret_lowres.mdl, origin = the elevator axis
  -736,1600,-10672 in sp_a2_intro). `@start_elevator_animation_rl` (trigger_once
  at -368,832) sends SetAnimation + SetDefaultAnimation `elevator_0N` (delays
  0.3/0.8/2.7/3). Decoded root-bone track of elevator_01: z +355 -> -87 (0.82 s)
  -> -510 (1.63 s), x/y within +-22, so the model's own data drops the turret
  straight down the shaft from -10317 to -11182, through the waiting car
  (bounds z -10958..-10731; the car sits at path_0 until startmoving()).
  AE 1100 `fireuser1` at cycle 0.459 (1.5 s, already below the car) ->
  OnUser1 -> `tube_object_kill` (kills !activator, only enabled by the
  trigger_once at -736,1104 via `blocked_elevator_tube_anim`). Engine already
  handles SetDefaultAnimation (re-loops the fall) and event 1100.
  Autopilot shots from the corridor/shaft were inconclusive (player placement
  fell into the shaft). Open question for the reference: does Windows Portal 2
  show these turrets dropping past/through the car (then only the looping
  after the kill relay matters), or is the root-bone position meant to be in
  another frame (e.g. a horizontal vac tube above the car)? Needs an owner
  screenshot/clip of this moment in Windows Portal 2 (or depot 624 RE of
  prop_dynamic root-motion handling). No fix attempt branch.

### T-125 sp_a1_intro7: the turn-away check is slow to register  (P2, BLOCKED: not reproduced, needs owner)
- Works (negative FOV cone fixed) but takes a moment (playtests 5, 7, 8).
- 2026-10-20 autorun, NOT REPRODUCED (autopilot log only): the delay we have
  is the map's own. glados_scenetable_include sp_a1_intro7NoWatching02
  ("...turn around for a second?", sphere03 doorwaysequence11.vcd, 3.17 s)
  enables wheatley_dont_watch_aisc only when the line ends (+0.2 s, glados.nut
  PlayNextSceneInternal "fire at end"); the aisc needs RequiredTime 1.5 with
  the target outside a 100 degree cone. Measured: turned away at t=25 (aisc
  already enabled) -> OnConditionsSatisfied at t=26.51, turnaroundnow01.vcd
  10 ms later; turned away early (during line 10) -> fired 1.7 s after line
  11 ended (0.2 + 1.5). So turning during his lines waits for them to finish;
  Source should do the same. No engine cause found, no attempt branch.
  Needs the owner: how long it took and when they turned (during or after
  "could you just turn around"), ideally a Windows timing reference.

### T-147 intro2: the screen goes black right as the first door opens  (P2, TODO)
- Section 1; compare T-119. Screenshot 7 (intro2): a dark shard in a door.

### T-139 Audio  (P2, PARTIAL: glass fixed 503406e, rest TODO)
- No soundscapes/ambience (water drips), no button clicks (sections 1, 5).
  Still open: the engine has no env_soundscape / soundscape script support
  at all (grep finds nothing), so this is a feature to build.
- Music stops abruptly sometimes; stopped at the wakeup stairs (old T-111).
  Still open, not looked at this run.
- Glass break far too loud entering intro4 while still in the elevator
  (old T-118). FIXED 2026-10-20 (503406e, autopilot only): cause was
  `aud_ramp_break_glass` (ambient_generic, flags 33 = everywhere + not
  looped, ScriptedSequence.RampGlassBreak, 150 dB) autostarting at map
  load because only flag 16 suppressed autostart. One-shot (flag 32)
  ambients now wait for PlaySound; 12 of 13 such ambients in the sp_a1/a2
  maps are PlaySound-triggered by their maps (the 13th, bts5
  `grinder_noise` world.grinderlp, now stays silent: replay bts5 too).
  Evidence: dist 89f2bf5 logs `play scriptedsequence.rampglassbreak` at
  intro4 load before the elevator arrives; 503406e does not, and plays it
  when autopilot walks into the ramp trigger (`goto glass_shard`), with the
  pane door opening. Unit test `only_looping_audible_ambients_autostart`.

### T-113 Instructor hints vanish after a few seconds if not done  (P2, TODO)
- Key labels fixed (owner confirmed).

## Open: P3


### T-020 sp_a1_intro1 ride  (P3, BLOCKED: needs owner)
- Walls flash away and back before breaking off; Wheatley a black box above
  (screenshot 4); the room jerks the camera and moves the player; floating
  parts; Wheatley moves oddly after the ride; glitchy screen at the start.
- Verify: `shot` every 0.2 s during the ride, then the owner.
- 2026-10-20 autorun, NOT REPRODUCED (no code change, no attempt branch).
  Tried shortcuts: `input crane_initial_startup_relay trigger` at 21 s, and
  `input crane_second_startup_relay enable/trigger` (second section, anim2;
  actor_container_01/03/05/07 OnAnimationDone -> kill fire ~30 s later).
  Shots every 0.5-1 s (yaw 65/155/245) show only the static bedroom
  (player feet stay at source -8709 1690 28); no wall flash or break-off seen.
  Reason: what the player sees during the ride is the point_viewproxy
  `camera_1` -> `camera_proxy` (parented to actor_container_master
  attachment vstattachment, offsettype 2, fake acceleration/tilt), enabled
  only by `@camera_proxy_enable_rl` from `relay_start_second_cryo_wakeup`,
  which these shortcuts skip. Next try: trigger relay_start_second_cryo_wakeup
  (or play the full intro: look up/down/painting/space prompts) and shoot
  through the proxy. Suspects for "jerks the camera / moves the player" and
  "walls flash": viewcontrol.rs proxy acceleration/tilt, and actors'
  SetAnimation anim2 -> kill timing vs the proxy-enabled container_master
  being disabled. Owner: a short screen recording of the ride would help.

### T-164 Spiky, torn animated debris  (P3, BLOCKED: needs owner)
- intro1 ride (screenshot 5), sp_a2_intro shaft arms (playtest 7). Animated
  props draw as rigid per-bone triangles (models.rs split_by_bone); soft
  weighted meshes tear. Check the weights, then skin them.
- 2026-10-20 autorun: weights checked (mdlinfo now prints "soft verts" and
  "mixed tris" per mesh). intro1 ride: the container_ride finedebris_part1/3/
  5/9/11/12/13 rebars, rust, crumplebeam/crumplecorner, bed02 and frame2
  meshes blend 2-3 bones (e.g. part12 crumplebeamcol 1257/3265 soft verts,
  1846 of 3080 tris mixed), so the rigid split tears them: confirmed cause
  by data. sp_a2_intro: every model is rigid (arm64x64_interior_rusty,
  crusher, armrubble_* all 0 soft verts), so the shaft-arm spikes are a
  different problem, still open (check bone poses/interpolation there).
- Not visually reproduced: in sp_a1_intro1 autopilot (`input
  actor_container_05 SetAnimation anim2`, `input camera_intro Disable`,
  `goto`/`lookat`) the view stays on the bed-room intro camera, so no
  before/after shot of the debris.
- Untested fix: local branch attempt/T-164 (c34006e), needs owner
  review/replay. Soft meshes (vertexlit/unlitgeneric) stay whole under the
  model and are linear-blend skinned on the CPU from their BonePart
  transforms when those move (plugin.rs skin_soft_meshes, models.rs
  skin_vertices with a unit test); valve_render re-uploads the vertex
  buffer when the SkinnedVertices generation changes. Replay the intro1
  ride's wall breaking and the room transform after `git merge
  attempt/T-164`.

### T-116 intro1 room details  (P3, BLOCKED (needs owner))
- The window is a grey block, should be blinds (screenshot 1); the TV
  screen sits forward and up from the TV; odd outside lighting on the ride.
- Autorun 2026-10-20, NOT REPRODUCED (autopilot only, no code change, no
  attempt branch):
  - Window: the start room's window (x ~ -8460, behind the plant) is world
    brushes in glass/glasswindow_frosted_break001a (and
    glass_underground_dirty01a): LightmappedGeneric, $translucent, a dirty
    frosted texture with vertical streaks, no blinds in it. P2_HIDE_MAT=glass
    shows only plain bright white behind it (no model or brush with blinds
    there; static props in the room are furniture04, closet_curtain01,
    lab_pod_frame_end_01, container_lightcover). So we draw what the map
    has; the glass may just read darker than on Windows (the VMT's
    `"srgb?$alpha" "0.5"` conditional is not applied by us; unconfirmed).
    The ride container's slatted look (models/container_ride/windowrails,
    finedebris_part7) is a different room. Needs a Windows screenshot of
    the same view to say what "blinds" should look like.
  - TV screen: `tvscreen` (tvscreen01.mdl) belongs to the ride's
    actor_container_07 and gets SetParentAttachmentMaintainOffset
    screenattachment from @rl_container_ride. Autopilot shots 1.5 s and
    9 s into the ride show it on the TV; the start room's TV is part of
    hotel_container_furniture models. Needs the owner's exact moment/view.
  - Outside lighting on the ride: not checked this run (ride is seen
    through point_viewproxy camera_1, see T-020).

### T-115 intro1: no wake/sleep animation on the bed  (P3, BLOCKED (needs owner))
- 2026-10-20 autorun: NOT REPRODUCED, no code change, no attempt branch.
  Autopilot (sp_a1_intro1; shots at 0.5-14 s, then return_to_bed_button
  unlock+press at 16 s, shots to 31 s): first wake-up plays camera_intro on
  ghostanim "sp_a1_intro" (13.4 s: ceiling through the white fade, sits up,
  looks round, TeleportToView at 12 s, continuous handover). Bed press:
  cryo_fade_out (3 s to white, 5 s hold, 2 s back), relay_start_second_cryo_
  wakeup at +5 s teleports the player to @knockout_teleport_1 (bed, yaw 10);
  no camera motion after that.
- Why no get-up/lie-down here: the only other rig is blackout
  (models/blackin.mdl, seq exit1 5.33 s, camera blackout_viewcontroller on
  vehicle_driver_eyes), but both are members of point_template
  blackout_template (spawnflags 2, i.e. templates removed at load, as Source
  does and we do); only @debug_skip_to_moving_container ForceSpawns it, and
  blackout's own OnAnimationDone kills it. So relay_start_cryo_sequence's
  Enable/SetAnimation hit nothing in normal play, on both wake-ups. Nothing in
  the map animates lying down on the bed (the press only fades to white).
  blackin.mdl's material models/blackout/blackout is missing from the game
  files too.
- Needs the owner: a Windows recording/screenshots of the wake-up (which
  moment looks wrong) and of what happens after using the bed.


### T-165 sp_a1_intro7: Wheatley's rail connector invisible  (P3, BLOCKED: needs owner)
- 2026-10-20 autorun: NOT REPRODUCED. Wheatley's rig is spherebot_train_1_chassis_1
  (func_tracktrain *2) -> spherebot_1_top_swivel_1 (func_tank *3, yaw) ->
  spherebot_1_bottom_swivel_1 (func_tank *4, pitch) -> @sphere, plus two nameless
  prop_dynamics at -370 -416 1592: wheatley_arm_base.mdl (parent chassis) and
  wheatley_arm_main.mdl (parent top swivel), both bind-pose only, material
  models/anim_wp/arm_64x64, both load (no warning). Autopilot shots
  (setpos -300 -330 1560 10 -130 and -330 -330 1640 35 -135) with and without
  P2_HIDE_MAT=arm_64x64 show the arm pieces drawn around Wheatley under the
  orange-striped beam; a thin arm sliver shows at the beam's top edge (arm_main
  spans 15-97 u from its origin at z 1608-1632, mostly inside/behind the beam).
  Unconfirmed lead: the top swivel is already yawed ~167 deg at t=4 (before any
  SetTargetEntity), which swings arm_main with it. No code change, no attempt
  branch. Needs an owner screenshot (or Windows reference) showing which part
  is missing and from where.

## Open: P4

### T-151 Water like Portal 2's (reflections, murk), intro3 and sp_a2_laser_intro  (P4, TODO)
### T-155 Lasers look poor  (P4, TODO)
### T-150 Particles missing  (P4, TODO)
- Fall smoke and water splash (+ sounds) in intro3; the gun's beam when
  holding things; motion blur/speed lines; the gun's glow seen through the
  wall after the sp_a2_intro fall.
### T-152 Lighting and texture quality below the original  (P4, TODO)
- Scrap pieces in the intro3 fall; turrets in tubes (wakeup); bumped
  VertexLitGeneric (models with $bumpmap) still drawn without bump.
### T-143 intro1 12:00 clock: the whole sign blinks, only the digits should  (P4, TODO)
### T-146 intro1: the first cube-button door should get half stuck  (P4, TODO)
### T-154 Floating vine/moss/grass texture by the centre wall (intro5)  (P4, TODO)
### T-166 intro5: a missing wall panel  (P4, TODO)

## Awaiting owner replay (fixed, autopilot evidence)

### T-140 Game-placed portals: no screen shake or sound  (P3, FIXED 2026-10-20, eb58eac, autopilot only)
- The first orange portal should shake the screen a little and play its sound.
- Cause: env_shake was not implemented at all. intro1's
  portal_red_0_activate_rl fires `shake_portal_spawn_room1 StartShake`
  (amplitude 4, frequency 2.5, 1 s, radius 500) 0.8 s after Trigger, with
  the portal's SetActivatedState. The sound already played: map portals send
  PortalPlaced, so Portal.open_red plays (log, before and after build).
- Fix: new p2_game shakes.rs (StartShake/StopShake/Amplitude/Frequency,
  linear radius falloff, spawnflag 1 global, 32 no view shake) feeding
  p2_core ScreenShakes; p2_view offsets and rolls the main camera (random
  direction re-picked `frequency` times a second, fading squared). Not done:
  physics/rope pushes (flags 8/16), the on-ground check. Unit tests:
  amplitude_fades_with_distance, shake_fades_out.
- Evidence (autopilot only): `goto portal_red_0 200; input
  portal_red_0_activate_rl Trigger`: "env_shake start, amplitude 2.34 at
  207 u", then RUST_LOG=p2_view=trace shows camera offsets peaking ~1.9 u
  and decaying to 0 over the second; before build: no shake. Owner replay:
  does it feel like Windows Portal 2 (strength, speed)?

### T-148 Portals behind glass  (P3, FIXED 2026-10-20, cc04a74, autopilot only)
- Owner section 1 (intro2): portals behind glass look on top of the glass;
  behind broken glass not drawn at all.
- Cause: glass/water (Refract) read Bevy's transmission copy, taken once
  before the transmissive pass, and live portal surfaces were drawn in that
  same pass. Broken pane (glass_fracture_128x128_b, its Refract mesh sorted
  after the portal) refracted a copy of the bare wall over the portal;
  glass sorted before a portal was overdrawn by it.
- Fix: live surfaces sort first (depth_bias) and p2_render's
  portal_surface_pass draws them after the Valve scene passes, before the
  copy; Bevy's second draw fails a strict depth test. Unit test
  portal_prefix_len.
- Evidence (autopilot, sp_a1_intro2, portal_blue_1/red_1 activated,
  `setpos -450 -60 -60; lookat portal_blue_1`): before, no portal behind the
  cracked pane; after, the portal shows through it with cracks and
  refraction over it. A view through the intact pane (setpos -100 -40 -60)
  looks the same before and after; the "on top of normal glass" case was not
  isolated by autopilot, owner replay needed.

### T-157 Transition into the sp_a2_intro fall not smooth  (P3, PARTIAL 2026-10-20, badb7a9, autopilot only)
- Owner playtest section 5: the fall works and lands right, but the
  transition into it isn't smooth.
- Reproduced (autopilot `input @transition_from_map trigger` in
  sp_a1_wakeup): the map's exit_fade goes black in 0.3 s, then level_exit
  waited a fixed 3.5 s, so the screen stayed black 3.6 s before
  sp_a2_intro loaded (Source changes level as soon as the fade is black).
- Fixed (badb7a9): when the map already holds an opaque stay-out fade,
  the next map loads 0.1 s after it is black (unit test
  level_exit_after_map_fade_loads_once_black). Autopilot after: "leaving"
  -> "sp_a2_intro loaded" 0.25 s (was 3.65 s). Elevator exits unchanged.
- Still open: right after the load (only when arriving from wakeup, not
  when starting sp_a2_intro directly) frames stall ~2.5 s wall clock (was
  5-8 s before this fix) while the game clock barely moves; `sample` shows
  threads mostly waiting, prepare_surfaces hot. Suspect: the valve_render
  GPU mesh cache (PreparedSurfaces.meshes / mesh_index, keyed by Arc
  pointers) is never cleared between maps, so the old map's buffers stay
  and a reused pointer could even pick a stale mesh. Not confirmed.
- dist build 0.1.0+badb7a9.20261020T1656Z.

### T-153 GLaDOS wake-up  (P3, PARTIAL 2026-10-20, c64a60b, autopilot only)
- White particles hide her; her arms float unattached when grabbing Chell
  and Wheatley; the portal gun didn't visibly drop; the player is made to
  look the wrong way (playtests 6-8).
- Gun drop FIXED (c64a60b): glados_camera.nut fires `gun_shooter Shoot` at
  6 s, and env_shooter wasn't implemented. The loader now spawns its
  shootmodel hidden; Shoot throws m_igibs dynamic copies along its facing
  (m_flvelocity 60, gib life 4 s). Autopilot `input gun_shooter Shoot`:
  before (dist d1df637) nothing appears; after, w_portalgun
  falls, lands ~3.5 s later and is removed at 4 s. Not seen inside the
  full wake-up sequence (autopilot only, not owner-checked).
- Still open: white particles hiding her, arms floating unattached, the
  player looking the wrong way.

### T-117 Blocky shadows  (P3, FIXED 2026-10-20, d1df637, autopilot only)
- Cause: env_projectedtexture shadows (light_room_00 in sp_a1_intro1: 110 deg
  FOV) used a 1024 map and one 2x2 bilinear compare, so the hanging
  planter's shadow stepped per texel and its chains broke into dashes
  (owner screenshot 2).
- Fix: 2048 shadow maps (projected.rs SHADOW_SIZE) and a 3x3 (1 2 1)
  Gaussian of bilinear compares (projected_common.wgsl), like Source's
  flashlight PCF.
- Evidence (autopilot, sp_a1_intro1, `2 input camera_intro disable; 4 setpos
  -8640 1720 10 -18 180; 5 shot`): before (dist 0.1.0+b39fff3) stair-stepped
  pot outline and dashed chains; after smooth outline, solid chains, no
  acne on the walls. Replay: look at the planter shadow above the closet.

### T-162 Menu animations, logo alignment, styling  (P3, PARTIAL 2026-10-20, b39fff3, autopilot only)
- Panel/flyout transitions flip oddly; the Portal 2 logo is misaligned.
- Cause: the logo and item column were placed by eye (logo 6%/9%, 34% wide;
  items 6% left, 12% from the bottom, 2 px apart). Now from the install:
  basemodui_scheme.res Logo.X/Y 75,35, 240x60 units; mainmenu.res items at
  x 88, 20 tall every 30 units, last at y 378 (480-unit layout, Vh-scaled;
  pause menu uses the same column). Also every menu change rebuilt the
  buttons, which only learn they are hovered a frame later, so the clicked
  item flashed unlit for a frame; carry_hover now keeps them lit.
- Evidence: autopilot main menu + `click SinglePlayer` shots, before (dist
  89f2bf5-era) vs after b39fff3; Quit row centre at 80.8% of the height as
  the .res gives. The one-frame flash was found by reading the code, not
  captured in a shot. Still open: Portal 2's panel fade/slide timings are
  not implemented (no Windows reference of what the owner saw as "flip");
  the main/pause font size was not checked against the scheme.

### T-161 Menu background video and music  (P3, FIXED 2026-10-20, f6f129e, autopilot only)
- Cause: the menu only showed the act's still (background0N_widescreen),
  and the music indexed the five portal2_background0N tracks by chapter
  number (chapter 3 played track 3, not act 2's).
- Fix: Main/New Game/Options play media/menu_act0N.bik for the save's act
  (ffmpeg streams it looped in real time, newest frame only; still shown
  until frames arrive or without ffmpeg; stopped in the game). Music uses
  the same act (unit test `menu_act_by_chapter`).
- Evidence (autopilot only): dist 89f2bf5 shows the static still; new build
  logs "menu movie media/menu_act02.bik" (save on laser_over_goo) and shots
  at 4 s and 6 s show different movie frames; ffmpeg exits after Continue
  and on quit, never starts on a map launch. Music change not listened to.
  Owner: check it loops smoothly and the act/track match Windows.

### T-160 Main menu like Portal 2's  (P2, FIXED 2026-10-20, ab64b84, autopilot only)
- Top level: Play Single Player, Play Cooperative Game, Community Test
  Chambers, Options, Extras, Quit (unbuilt ones may do nothing yet).
- Play Single Player: Continue Game, New Game, Load Game, Challenge Mode.
- New Game: a vertical chapter list, not the gallery.
- Layouts and art are in the install: resource/ui/basemodui/*.res
  (mainmenu, campaignflyout, chapter dialogs), materials/vgui.
- 2026-10-20 (ab64b84): the main menu had only Play Single Player,
  Options, Quit, and New Game was a 3-column picture grid. Now: Play Single
  Player, Play Cooperative Game, Community Test Chambers, Options, Extras,
  Quit (Co-op and Extras do nothing yet; Community Test Chambers opens New
  Game's community tab when there are community maps, else nothing). The
  Single Player flyout is Continue Game, New Game, Load Game, Challenge Mode
  (Continue/Load only with a save, as before; Challenge Mode does nothing
  yet). New Game is a vertical chapter list (CHAPTER N: TITLE, locked ones
  greyed) with the picked chapter's picture and title on the right, as in
  newgamedialog.res; click picks, a second click or START NEW GAME starts.
  Community tab unchanged (still the grid). Unit test
  main_menu_items_are_portal_2s.
- Evidence: autopilot `click SinglePlayer`, `click NewGame`,
  `click SelectChapter(1)` shots (before: dist 89f2bf5-era build showed the
  3-item menu and tile grid). Not owner-checked; layout by eye, not the
  .res pixel positions, and no Windows side-by-side.

### T-123 Portal shots should knock security cameras off  (P2, FIXED 2026-10-20, 2bb0015, autopilot only)
- intro6 and sp_a2_laser_stairs: in Portal 2 the camera falls off.
- Cause: npc_security_camera was only a static model; nothing handled its
  `Ragdoll` input (server.dll CNPC_SecurityCamera::InputRagdoll) or portal
  shots. Now (crates/p2_game/src/cameras.rs) a camera tears off and becomes
  a dynamic physics prop (insert_prop_body, shared with props.rs) on
  `Ragdoll`, when a portal shot's path passes within 20 u of its body, or
  when a portal opens over its wall (opening + 16 u). Unit tests cover both
  checks.
- Evidence (autopilot, sp_a1_intro6, `3 setpos 464 -420 0; 5 lookat 458
  -270 40; 6.5 fire blue`): dist 89f2bf5 shot fails, camera stays on the
  wall; new build logs "security camera 563 detached" and later shots show
  the wall bare and the camera lying on the floor below. laser_stairs not
  run. Not done: the wall decal left behind (decals_subrect
  "SecurityCamera.Detachment"), whether the torn-off camera can be carried
  (it can here, like any prop), detach sound, saves keeping it detached.

### T-112 Missing or flat textures  (P2, PARTIAL 2026-10-20, 991441d, autopilot only)
- GLaDOS chamber floor brown and flat like the old intro3 water (section 5);
  hallway to GLaDOS; after the jumps a black wall you can walk through; a
  blue flat wall behind the intro4 elevator.
- GLaDOS chamber floor FIXED (991441d), awaiting owner replay. Cause: the
  floor is shallow water (nature/water_glados_01, surface z 396 over a
  displacement floor at z ~388-393, $fogend 30, $fogcolor 90 70 20,
  $lightmapwaterfog). Our water shader fogged through an assumed 64 units
  of water, so with $fogend 30 it was always fully fogged: one flat
  lightmap-lit yellow-brown sheet. Source fogs by the real depth behind the
  surface. Now map water is cut into a 32-unit grid at load
  (glass::subdivide) and each vertex carries its measured depth to the
  world geometry below (glass::WaterFloor, vertex colour alpha,
  FLAG_WATER_DEPTH; ground just above a vertex, as under banks, = depth 0).
  Unit tests: subdivision area/edge length/winding, depth below.
  P2_LIST_MATS also logs each water mesh's mean depth (wakeup 13.4 units,
  intro3 6.9 units).
- Evidence (autopilot, sp_a1_wakeup, `3 setpos 9060 1120 440 55 20`):
  dist 89f2bf5 shows a solid yellow sheet; new build shows the dark floor
  through the shallows fading to murky olive with distance (pixels ~45,39,15,
  near the floor's ~55,50,37). Not compared with Windows; the water's real
  reflection (_rt_WaterReflection, $reflectamount 0.4) is still missing, so
  it reads darker/flatter than the game. intro3's water (same path, fogend
  60, ~7 units deep) now also shows more floor when looked down at:
  owner replay should glance at both.
- Still open: the hallway to GLaDOS's chamber, the black walk-through wall
  after the jumps (where?), the blue flat wall behind the intro4 elevator.

### T-149 Chamber signs blank; screens and lights wrong  (P2, PARTIAL 2026-10-20, d314ea7, autopilot only)
- The white chamber-number screens ("02") were blank in every level:
  they are `vgui_screen`s with panelname `sp_progress_sign`, which we never
  drew (only the dark sign frame model showed). FIXED: new
  crates/p2_game/src/signs.rs composes the board per map from
  scripts/vscripts/transitions/sp_lightboard_icons.txt (level_number,
  total_levels, ten icons, "1" = used) and the art
  vgui/screens/vgui_coop_progress_board (+_numbers digit sheet 6x2,
  P2_lightboard_vgui icon sheet 5x5): big two-digit number between the top
  rules, "NN/TT" and a per-chamber progress bar (played ones dark) in the
  middle band, the ten icons in two rows of five (used ones dark, others
  faint). Shown on the prefab's SetActive, hidden on SetInactive (spawnflag
  1 starts it on). Unit tests: file parsing, lit vs faint icons, bar.
- Evidence (autopilot, intro2, `input info_sign-info_panel setactive; goto
  info_sign-info_panel 320; look -38 -12`): old build (dist 503406e) shows
  an empty dark frame; new build shows "01", "01/19", the first bar segment
  dark and the cube_button icon dark with the rest faint.
- Approximations (no Windows reference shot): exact positions/sizes of the
  number, label, bar and icons are by eye from the board's three rules;
  the icon sheet's cell order was matched to names by picture (it differs
  from the file's comment list); the startup flicker sequences
  (lightboard_startup_sequences.txt) and dirt overlay ("dirt") are not
  drawn. Owner: compare one sign with Windows Portal 2.
- Still open: intro5's screens and lights wrong (playtest section 4), not
  looked at this run.

### T-144 Missing props: intro1 portal room clipboard and movable clock  (P2, FIXED 2026-10-20, 89f2bf5, autopilot only)
- Cause: physics props had Mass(40) but their rotational inertia came from
  the collider at avian's default density (1 kg/m3). The radio's hull weighs
  37 g at that density, so it got about 1/1000 of a 40 kg box's inertia: contacts
  spun it, it rocked from spawn and sank through the vault floor at ~1.8 s.
  The unnamed bts_clipboard on the table fell too. Fix: the prop's colliders
  get the density that makes them weigh 40 kg (unit test
  prop_colliders_weigh_the_prop_mass).
- Evidence (autopilot, sp_a1_intro1): dist 2e8df1a radio z 2705 -> 2691 at
  1.8 s -> 2180 at 3 s; clipboard z 2639 at 0.5 s. New build: radio at
  (-1208 4324 2705) upright at 0.5/1.6/3/6 s, clipboard at 2727 on the table
  at 1 and 6 s. Side effect: intro2's spawn cube now settles flat on a face
  (z -43) instead of leaning at 24 deg (z -39); every prop now has a 40 kg
  box's inertia (cubes tumble less). "Movable clock" still not identified
  (the clocks are prop_dynamic and draw); owner to say if anything is still
  missing.
### T-145 Buttons don't animate; cubes don't change colour on a button  (P2, fixed feb5072 + 2e8df1a, autopilot only)
- Section 0: no press-down when used or when a cube lands; cubes kept their
  off skin; cubes twitched on buttons (press/unpress flicker as they settle).
- Press + twitch (feb5072): buttons were a baked pose; now animated
  (down/idledown/up and variants), release only after 0.15 s clear.
- Cube skins (2e8df1a): skin was baked in at spawn. Cubes whose model has a
  lit skin (metal_box 0/1 -> 2/3; reflection/antique have none) spawn both
  skins' draws in SkinVariant groups; floor_buttons sets ModelSkin lit while
  the cube rests on a pressed button. Unit test cube_lit_skins.
- Evidence (autopilot, sp_a1_intro2, `propto box` onto button_1-button): old
  build cube ring stays blue on the pressed button, new build ring lit
  yellow; moved off the button, ring blue again. Not owner-verified.
- Not done: spheres (mp_ball skin 1 is orange, unconfirmed as the lit skin).

### T-102 Cubes through portals not seamless  (P2, fixed 847a581, autopilot only)
- Sections 1-2 (intro2, intro3). Traversal itself works (owner replays).
- Cause: only the player body had exit-side ghosts; a cube half through a
  portal showed nothing coming out of the linked one. Props now get a mirror
  of their ValveSurface draws through the pair while in the opening
  (prop_ghosts.rs); unit test a_cube_in_the_opening_gets_a_ghost_out_of_the_exit.
- Evidence (intro2, red_1 + blue_1/blue_3 activated, cube propto'd half
  through red_1, view of the blue exit): before no cube at the exit, after
  the cube's half sticks out of the blue portal. Owner: carry/throw cubes
  through portals in intro2-intro4 and check for seams or a pop at the swap.

### T-137 Portals can't be placed as close together as in Portal 2  (P2, fixed 38c09bf, autopilot only)
- Section 5. Cause: a shot overlapping the other portal on the same wall
  failed with Overlap (fizzle); Portal 2 bumps the new portal off it so they
  sit side by side. Fix: rules::bump_off_portals pushes it to the nearest
  free side (1 u gap, still fitted to the surface, within the max slide).
- Evidence (sp_a2_laser_intro, blue at (-35,-256,173), orange aimed 42 u
  beside it): before "Orange shot failed: Overlap"; after "Orange portal
  placed at (30,-256,186)", both visible side by side. Gap size and which
  side wins are our choice, not checked against Windows Portal 2.
### T-136 sp_a1_wakeup end: a portal shows in the tube before GLaDOS drops the player  (P2, fixed 3f96088, autopilot only)
- Playtests 7 and 8. Cause: relay_incinerator_open links
  incinerator_entry_portal_1 (floor under the incinerator door) to _2 (top
  of the tube); our linked_portal_doors were drawn like gun portals (oval
  flame shader + blue particle rim + hum). Portal 2 draws them as a plain
  window. Fix: SeamlessPortal marker; the surface shader draws only the view
  across the quad, p2_fx skips rim and hum (unit test in p2_fx effects).
  Evidence: autopilot `input relay_incinerator_open trigger` + goto/lookat
  the door: blue oval ring before (dist 8f8c8b1), continuous tube after.
  Owner: replay the drop after GLaDOS's scene (also the finale's doors).

### T-163 sp_a1_wakeup pod: the player can jump out of the hatch; a stone-brick block appears there  (P2, fixed 8f8c8b1, autopilot only)
- Playtest 6. Cause: brush entities with startdisabled 1 were never built
  (no mesh, no collider), so an Enable had nothing to switch on. The hatch is
  sealed by basement_breakers_upper_blocker (toolsplayerclip ring, z 392-576,
  enabled by the socket relay). Fix: build them hidden with ColliderDisabled
  (Always Solid func_brushes keep collision); applies to every map.
- Evidence (autopilot: setpos on the lift, enable blocker, trigger
  basement_breakers_start, walk + jump outward at the top): before (dist
  5cfe749) feet reached (9133 1017 428) outside the hatch; after, the
  blocker has colliders and feet stay on the lift (8988 1046 351).
- The "stone-brick block" was not seen by autopilot (the blocker is player
  clip, never drawn); owner replay should say if it is still there.
### T-134 sp_a1_wakeup: the rising pod's floor turns, the player doesn't  (P2, fixed 5cfe749, autopilot only)
- Playtests 6, 7, 8. Cause: our func_tracktrain never rotated. breaker_train
  has orientationtype 2 (linear blend) and its path_tracks carry yaws
  (path_1 0, path_4 270, path_2 0, path_3 135), so in Source it turns 90
  degrees right as it sets off (with the floor prop's breaker_shaft_rotate_90)
  and again while rising. Fix: blend trains' yaw from node to node (yaw-only
  nodes; nodes that pitch/roll are left alone) and turn riders/standers
  about the train origin, view yaw included.
- Evidence (autopilot, setpos onto the lift + `input basement_breakers_start
  trigger`): before (dist 2542009) yaw 0 and feet (9016 1088) through 7.5-12 s;
  after yaw 0 -> -63 -> -90 between 7.5 and 9 s, feet orbiting to (8975 1048),
  back to yaw 0 at path_2 while rising. Not owner-checked.
- Side effect to replay: blend-oriented trains with yaw-only nodes in
  sp_a2_bts3/bts5 (panel tracks) and sp_a3_end (pumproom lift) now turn too.

### T-133 sp_a1_intro7: Wheatley floats after the wall panel moves; the panel disappears  (P2, fixed 2542009, autopilot only)
- Cause: DisableMotion (socket_powered_rl) took Wheatley out of the
  player's hands but kept the carry's GravityScale(0); when @plug_eject_rl
  ran EnableMotion/ClearParent he went dynamic with no gravity and hung at
  the socket. Fix: a locked core gets GravityScale(1) back (props.rs
  `locked_core`, unit test). Evidence (autopilot: forcepickup, socket
  powered, eject): before z 1265 at t+9 s (floor 1232); after z 1249 (resting).
- Panel vanishing NOT REPRODUCED: after the open, heavydoor disables and
  heavydoor_ghost is enabled and drawn (info). Owner: screenshot if it recurs.

- T-120 elevator car sounds (5bfaf52): func_tracktrain movesound and
  stopsound were never played, so rides were silent. Now the move sound
  starts with the train and follows it (World.ElevatorDepartLp looped,
  World.ElevatorArrive once), the stop sound plays where it stops. intro6
  autopilot log: before (dist c1079f9) no elevator sounds; after,
  elevatorarrive at spawn, elevatorarrivestop as the arrival car stops,
  elevatordepartlp at gain 1.0 through the departure ride. Autopilot only.
- T-120 elevator ride jitter (c1079f9): trains and their parented children
  were drawn at 60 Hz tick positions while the view is interpolated, so the
  eye swung against the car every tick. Movers now draw between ticks
  (MoverLerp; tick value restored in FixedFirst). P2_RIDE_TRACE on the
  intro3 arrival ride: eye-minus-car height 1.374..1.499 m (12.5 cm) before,
  1.499..1.503 m after; car end state identical in shots. Autopilot only.
- T-114 intro1 announcer over Wheatley: ambient_generic ignored the
  `Volume` input. The map turns opening_courtesy_vcd down to 3 as the first
  knock plays (8 s after the second wakeup) and to 0 when the vault door
  opens; ours stayed at full volume over Wheatley. Fixed (463757d), unit
  test ambient_volume_input_scales_0_to_10. Autopilot only (intro1,
  `input @debug_skip_to_second_stasis trigger`, log portal2::audio=debug):
  before no volume change; after "volume 0.3" at the knock, "volume 0" on
  door open. Some overlap is original: Windows P2 also starts "Hello?" over
  the quieted announcer. The door opening on approach is also original
  (door_open_trigger, enabled 10 s after wakeup via door_hud_hint_trigger;
  autopilot: closed at 6 s and 9 s, opens on reaching the hint trigger). The
  "apple" skip was handled earlier by the per-actor voice channel.
- T-132 Fizzler field not drawn: every trigger_* brush was skipped, so
  visible trigger_portal_cleansers never drew their effects/fizzler faces
  (SolidEnergy, a green-only ripple mask, wasn't translated either). Now
  (99392c0) visible cleansers draw, SolidEnergy is unlit additive with the
  mask tinted by a quarter of $FLOW_VORTEX_COLOR (no flow animation yet),
  Enable/Disable shows/hides it. Autopilot only (intro2 exit, `setpos -320
  740 -60 0 -90`): before no field, after a blue field, hidden on
  FireUser1, back on FireUser2; unit test visible_cleansers_draw_their_field.
  Not checked: fizzling through it (logic unchanged); startdisabled
  cleansers still get no mesh (check at sp_a2_fizzler_intro, T-007).
- T-167 (rest) A held cube at a wall surrounded the camera: the player
  ignores the held prop's colliders, so walking on pulled the hold point
  back onto the eye. Now (342b5b1), as in Portal 2 where the held prop is
  solid to the player, a ray along the view stops the player 17 u plus the
  prop's depth from a wall (skipped into portals). intro2 box walked into
  two walls: before (dist) box centre at the eye (36 398 28, feet 36 400),
  view = inside ring of its face; after the player stops ~41-49 u back and
  the whole cube shows. Autopilot only; walking into a portal while holding
  and Wheatley in intro7 not checked.
- T-167 (part) Wheatley can't be picked up again after a drop: cores were
  pickable only after an EnablePickup input, which sp_a1_wakeup never sends
  (he starts force-picked), so once dropped (or stuck-dropped) he was lost.
  Cores now stay pickable until DisablePickup (intro7's map-spawn disable
  still applies). wakeup: drop enabled via input, `use` drop, goto + lookat,
  `use`: before (dist fd9c086) the ray hit him and he stayed on the floor
  (z 401); after (636fc3f) he is held facing the player at z 441. Only
  autopilot-checked, wakeup only.
- T-105 Held props dropped when carried into a wall: the hold point went
  inside the wall, the prop stayed >64u from it and was dropped after 0.5 s.
  Now a ray from the eye pulls the hold point in front of static geometry
  (not through portals). intro2 cube walked into three walls: before
  (dist build) it fell to the floor (z -46) and stayed; after (fd9c086) it
  stays held at the wall and returns to its hold distance when backing
  off. Only the cube was autopilot-checked, not Wheatley in intro7.
- T-130 Portal shots pass handrails: models compiled `$contents grate`
  (props_bts/handrail_*) now let portal shots through like grate brushes.
  intro5 rail at (380,-330): before, the shot stopped at the rail; after, it
  reaches the wall behind. Loose cubes already pass shots (dynamic bodies
  are skipped); not reproduced in intro6, where the dropped cube landed on
  non-portalable dirt floor at (1947,381) tilted, ~25 u up a wall. Owner:
  replay intro6 and say where the cube was when the shot failed.
- T-129 Stray elevator screens: movie panels were drawn extending to the
  entity's right instead of its left, so the ring covered its own exit gap
  (laser_stairs: @arrival_sign_1/_24 across the stairs and leaving trigger).
  Autopilot before/after shots of the laser_stairs arrival; intro4's exit room
  (same cause, playtest 3) still needs the owner's replay.
- T-128 sp_a2_laser_stairs stairs rise: held sequences replay on a negative
  SetPlaybackRate (8ac53a2; screenshots before/after).
- T-103 sp_a1_wakeup breaker lift carries the player (SetMaxSpeed); owner saw it rise in playtest 6.
- T-106 Walk into the breaker room standing (hull fits an exact 72 u).
- T-121 A cube resting where a portal opens falls in (contacts dropped in the hole).
- T-122 func_portal_detector fires (intro7 progresses).
- T-126 Tilted map portal drawn whole (quad stands 1 u off).
- T-127 The panel GLaDOS swings into place takes a portal (sensor colliders).
- T-119 Doors stay drawn while opening; no area-portal black plane.
- T-110 An actor's new line cuts its previous one (voice channel).
- T-021 Owner checks: intro1 ride, wakeup socket, sp_a2_intro portal route.

## Backlog

### T-002 Prove sp_a2_dual_lasers  (P1, TODO)
- Both catchers powered by the player's redirection, door opens, next map.
### T-003 Prove sp_a2_laser_over_goo  (P1, TODO)
### T-004 Prove sp_a2_catapult_intro  (P1, TODO)
### T-005 Prove sp_a2_trust_fling  (P1, TODO)
### T-006 Prove sp_a2_pit_flings  (P1, TODO)
### T-007 Prove sp_a2_fizzler_intro  (P1, TODO)
- Faith plates land on target (scripts/check_faith_plates.py); T-004..T-006 test the player's route.
### T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug  (P1, TODO)
### T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)  (P1, TODO)
### T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)  (P1, TODO)
### T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)  (P1, TODO)
### T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)  (P1, TODO)
### T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)  (P1, TODO)
### T-015 Sweep chapter 9 (finale1..finale4)  (P1, TODO)
- Sweep = per map: load, logic runs, exit reachable, next map loads (L/O/N
  columns in PROJECT_STATE.md); proving each puzzle follows per map.
### T-090 Restart check per verified map  (P2, TODO)
### T-030 Gel chapters: missing buttons in gel rooms, black elevator, cube carry feel  (P2, TODO)
### T-040 App icon and second-Mac first-run check  (P3, TODO)
### T-050 Verify merged cloud features in game: Wheatley look-at, gels, projtex, saves  (P3, TODO)
- Checklists in docs/STATUS.md.

## Done (newest first; details in git log and docs/STATUS.md)
- 2026-10-20 Owner-confirmed in playtest 8: key labels on hints; Chell carries
  no gun before having one; no more "weird objects" (P0 regression), cubes and
  buttons drawn; T-104 sp_a2_intro fall plays and lands right.
- 2026-10-20 Static props with flag 0x4 drawn (intro2 exit stairs, much
  scenery); intro3 flat rubble (normal-alpha envmap mask); intro3 water lit
  fog and no white sky reflection (look still T-151).
- 2026-10-20 T-124 Wheatley falls off the management rail (owner playtest 5).
- 2026-10-19 T-101 sp_a1_intro1 ride end: wall breaks, elevator to intro2 (owner, 921edff).
- 2026-10-19 Packaging: dist/Prtl2Mac.app with build id, previous app kept, finds the install.
- 2026-10-19 Workflow docs and scripts/resume.sh.
- 2026-10-19 sp_a2_intro logic: gun pickup, portals, exit, next map (autopilot, 01c83ec).
- 2026-10-19 sp_a1_wakeup socket: the held core plugs in (autopilot).
- 2026-10-18 Chapter 1 intro2..intro7 logic verified (autopilot).
