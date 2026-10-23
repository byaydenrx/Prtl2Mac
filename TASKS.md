# Tasks

Active work only. Owner holds and the "later" (not blocking) list are in
docs/TASKS_ON_HOLD.md; finished tasks in docs/TASKS_DONE.md. Grep those by
task id, don't read them whole. The owner's raw reports: docs/PLAYTESTS.md.

Statuses: TODO, IN_PROGRESS, BLOCKED, DONE. One entry per task: fold new
reports into it. When a task is fixed, write its cause and evidence in two or
three lines, mark its header "FIXED, awaiting owner replay" and its
priority-order line the same; once the owner confirms, move the entry to
docs/TASKS_DONE.md. Long investigation notes go in docs/STATUS.md.

How fixes are done (owner, 2026-10-22): the owner plays the campaign himself
and reports bugs; sessions fix bugs and never write scripts that play whole
levels. A fix is done when (1) the bug is reproduced, (2) a targeted
autopilot run or unit test shows it gone (before/after), and (3) nothing else
broke: `cargo test --release -p p2_game` plus the crate touched pass, and
every map the change could affect still loads and plays its first seconds.
VERIFIED (PROJECT_STATE.md) means the owner played the map. T-008..T-015
("sweep" a chapter) mean: find and fix what would stop the owner finishing it.

## Priority order (owner, 2026-10-23)
The one source of what's next: sessions and autoruns take the first line not
marked FIXED, DONE or BLOCKED. Crashes and softlocks found on the way go to
the top. Menus, on-hold and "later" tasks are never taken without the owner.
Now (owner playtest 12, playability first):
1. T-230 sp_a2_laser_relays: a laser that has gone through a portal doesn't power the relays (FIXED, awaiting owner replay; was unsolvable)
2. T-229 Turrets don't work: no shooting animation, beams or bullets, knocking or dropping them does nothing, but they still do damage (FIXED, awaiting owner replay: animated, visible shots, tip over when walked into or knocked, Portal 2 voice lines; gun aim still straight ahead)
3. T-231 Lasers aim wrong globally: a laser angles up or down instead of going straight (FIXED, awaiting owner replay: no more auto-aim toward portal centres and relays; owner hit it in column_blocker on the build before the fix)
4. T-233 sp_a2_column_blocker: lasers and turret sight don't pass the grate (laser can't reach or kill the turret; turret can't see or shoot through it) (owner playtest 12b)
5. T-234 Turrets killed by a laser don't blow up (owner playtest 12b)
6. T-235 sp_a2_column_blocker: the light doesn't switch back on in the surprise room (owner playtest 12b)
7. T-236 sp_a2_column_blocker: invisible barrier at the departure elevator (NOT REPRODUCED, needs owner; owner playtest 12c)
8. T-237 sp_a2_column_blocker: Wheatley in the departure elevator doesn't look at you (FIXED, awaiting owner replay: his rail train started 34 u off its rail)
9. T-232 sp_a2_bridge_the_gap: Wheatley glitches around; the bird looks wrong (Wheatley FIXED, awaiting owner replay; bird: needs owner to say what looks wrong)
10. T-223 sp_a2_ricochet: the rotating platform under the cube is invisible and not solid (BLOCKED, needs owner replay: not reproduced on 18676be, likely fixed by T-221)
11. T-217 sp_a2_laser_stairs: floor missing after GLaDOS's floor move; Wheatley seen through the closed wall (BLOCKED, needs owner: floor fixed on main by T-221; Wheatley attempt on local branch attempt/T-217-wheatley)
12. T-218 Performance: 60-70 fps and stutter on an M4 Pro; portals cost a lot; v-sync unclear (owner: better; continue)
13. T-224 Cube physics feel: no tumbling in the air; plate and button hits unreliable
14. T-225 No dissolve ("fizzle") effect on cubes
15. T-226 Pedestal buttons (prop_button): no sound or press animation
16. T-173 Portal gun view model: hold beam, carry anims, fizzler shake
17. T-227 Zoom doesn't work
18. T-228 Chapter HUD icons don't look like Portal 2's
Then the engine gaps common to many maps, and gels:
19. T-212 sp_a3_00 (start of chapter 6): loud glass smash at load (BLOCKED: merge attempt/T-212 for the owner's replay)
20. T-203 Switchable lights frozen: lightmap styles other than 0 ignored (50 of 63 maps)
21. T-199 Particle sprite sheets not decoded: sheet particles draw the whole atlas
22. T-214 All-maps load sweep findings (2026-10-21)
23. T-201 UnlitTwoTexture (TV/monitor screens) and other model shaders on the fallback
24. T-206 Sounds not panned left/right
25. T-200 Particle operators the maps' effects use but p2_fx doesn't simulate
26. T-202 No bloom or colour correction (every map)
27. T-204 No env_cubemap reflections or dynamic lights on props
28. T-211 Gels: check each against Source
29. T-030 Gel chapters: missing buttons in gel rooms, black elevator, cube carry feel
Then the campaign sweeps (the owner's playtests go first):
30. T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug
31. T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)
32. T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)
33. T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)
34. T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)
35. T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)
36. T-015 Sweep chapter 9 (finale1..finale4)
Lower:
37. T-205 Portal gun edge cases
38. T-209 Portal transit only mirrors box-shaped geometry at the exit
39. T-207 Viewmodel gaps
40. T-208 Chell body gaps
41. T-210 Upstream minor gaps
42. T-090 Restart check per verified map
43. T-040 App icon and second-Mac first-run check
44. T-050 Verify merged cloud features in game: Wheatley look-at, gels, projtex, saves

## Active tasks (in priority order)

### T-230 sp_a2_laser_relays: a laser that has gone through a portal doesn't power the relays  (P1, FIXED, awaiting owner replay; owner playtest 12)
- 2026-10-23 FIXED. Cause: lasers.rs checked relays only on a beam's last
  leg; a leg that ended by entering a portal (or on the player) skipped
  them. In laser_relays the reflection cube's leg passes catcher_1 into a
  portal, so catcher_1 dropped out the moment the portal opened; the
  branch listener went "mixed" and closed the exit door in the same frame.
  Fix: every leg checks the relays it passes. Evidence (autopilot: spawn
  laser_cube, cube on the beam at -544 -704, blue portal east wall
  448 -704, orange north wall 256 -320): before catcher_1 OnUnpowered,
  onmixed -> exit_door_close_rl; after catcher_1..3 stay powered,
  onalltrue -> exit_door_open_rl, door stays open. Smaller repro: cube leg
  through catcher_2 into a portal: before catcher_2 unpowered, after
  powered. Regression: starting laser state identical on 7 laser maps; 202
  tests pass. The legs in that run also show T-231's tilt (the cube leg
  climbs z 18 -> 58 toward the portal).
- Owner: "the laser goes through multiile of the glass things but after it
  goes through a portal it doesnt so i cant trigger all 3 to finish the
  level". Laser relays (prop_laser_relay) pass the beam on and light up; the
  leg after a portal hits nothing. He noclipped past the level.

### T-229 Turrets don't work: no shooting, can't be knocked over  (P1 by owner, FIXED, awaiting owner replay; owner playtest 12)
- 2026-10-23 FIXED (turrets.rs, map.rs, lasers.rs; p2_fx sim.rs; p2_movement
  controller.rs). Turrets spawn animated (class_animates) and play deploy,
  idlealert, fire (looping while firing or tipped), retract, eye_off /
  idle_eye_off. Every 0.1 s a shot from one of the four muzzles
  (LFT/RT_Gun1/2_muzzle attachments): a combinemuzzle1/2 flash sprite, a
  `weapon_tracers` tracer to where the ray stops (new p2_fx initializer
  "move particles between 2 control points") and on every other shot an
  `impact_*_cheap` effect by surfaceprop. Tipped turrets spray bullets
  where they point for 2.5 s (hit the player only in that line), then shut
  down (TalkDisabled + Die). Walking into a standing turret tips it. Lasers
  burn it for 1 s (TalkStartBurning, then TalkBurned + Die). Voice lines
  added: TalkActive while engaged, TalkAutosearch mid-search, TalkRetire on
  retract, TalkCollide when knocked/hit (velocity jump or contact impulse
  from a moving body), TalkFlung when thrown/launched, TalkDissolved.
- Found on the way: walking into any prop never pushed it (the push code
  looked up the hit collider, not its body; props carry colliders on child
  entities). Fixed via ColliderOf, with a gentle push (accelerates to at
  most 120 u/s) so cubes slide instead of tumbling.
- Evidence (autopilot, turret_intro second_turret, god mode): before the
  turret stayed closed with only its sight line; after deploy pose, flashes
  alternating guns, tracers (screenshot burst). Walked into from the side:
  Idle -> Tipped -> Dead, spray shown. Dropped on its side: TalkTipped,
  TalkDisabled, Die. Dropped upright: TalkCollide, stays alive. Another
  turret dropped on its head: TalkCollide. Picked up: TalkPickup; dropped:
  TalkCollide. fps with 3 turrets firing (laser_vs_turret): 120 = old build
  (full per-bullet impact effects had cost ~25 fps; now cheap ones on every
  other shot). Tests 205 + 30 + 20 + 74 pass.
- Not done: the guns don't aim at the player (aim_yaw/aim_pitch pose
  parameters; they fire straight from the muzzles toward the player), no
  eye glow sprite, carried turrets don't play carry_pose, no bullet decals,
  turrets don't push the player with bullets.
- Owner: "turrets definitely do not work as intended theres no shooting
  animations or anything hitting them or dropping them doesnt trigger
  anything their just not done yet but they do damage as if their
  shooting ... definitely prioritize getting turrets going correctly".
  turret_intro completable; turret_blocker "pretty much unplayable" (done
  with godmode); laser_vs_turret noclipped.
- Needed (Portal 2 npc_portal_turret_floor): search/alert/fire states with
  the model's sequences, muzzle flashes and tracer beams, bullets that hit
  (impact effects, damage, push on props), the red eye laser sight,
  knocked over / picked up / dropped -> tips over, "dies" (sparks, voice,
  stops firing), lasers kill turrets, voice lines per state.

### T-231 Lasers aim wrong: a laser angles up or down instead of going straight  (P2, FIXED, awaiting owner replay; owner playtest 12)
- 2026-10-23 FIXED. Cause: lasers.rs auto-aimed a beam leaving a portal or a
  reflection cube at portal centres and at a point 32 u up each relay, so a
  beam at cube height (z 18) climbed to a portal's centre (z 58) and left
  the other portal sloping. Portal 2's auto-aim is for catchers
  (server.dll has `sv_laser_cube_autoaim`); a beam goes through a portal
  wherever it meets it and a relay is a column. Now only catcher lenses
  are auto-aim targets. Evidence (laser_relays cube + two portals, T-230
  setup): before legs z 18 -> 58 and 58 -> 25; after 18 -> 18 and 18 -> 18,
  all three relays still powered and the exit opens; starting beams on 7
  laser maps unchanged; 205 tests pass. Not compared against Windows: how
  strongly Portal 2 snaps onto a catcher (kept our 10 degrees).
- Owner: "lasers function a bit weird globally like being angled if they
  look up or down when they should just go straight they just function
  strange". Check emitter/catcher/relay directions and the cube's
  redirection (Portal 2 keeps a reflection cube's beam level).

### T-233 sp_a2_column_blocker: lasers and turret sight don't pass the grate  (P1, TODO; owner playtest 12b)
- Owner: "the laser for the turret doesnt go through the grate in that level
  and the turret cant see you through or shoot you through the grate, also
  the laser doesnt kill the turret through the grate". Grates
  (CONTENTS_GRATE, T-130) stop the player but should pass lasers, turret
  sight lines and bullets. Also: "this level is nit completeable because of
  the lasers being weird and angling, it angles itself above the next
  reflection box" (build 1894bb5, before the T-231 fix: replay first).

### T-234 Turrets killed by a laser don't blow up  (P2, TODO; owner playtest 12b)
- Owner: "when the turret does get killed by the laser it doesnt blow up".
  Portal 2: a burned-out turret explodes into gibs (robot_fx.pcf
  turret_fx_break_gib*, impact_turret_break sparks, explosion sound).

### T-235 sp_a2_column_blocker: the light doesn't switch back on in the surprise room  (P2, TODO; owner playtest 12b)
- Owner: "the light doesnt switch back on in the supries room". Likely the
  switchable-lights gap (T-203: lightmap styles other than 0 ignored).

### T-236 sp_a2_column_blocker: invisible barrier at the departure elevator  (P1, NOT REPRODUCED, needs owner; owner playtest 12c)
- Owner: "that first chamber is completeable but when i get to the elevator
  theres an invisivle barrier stopping me from getting in", noclip got past.
- 2026-10-23 autopilot (build a40afeb + T-232 fix): from past the exit door
  (-850 256 0) walking west: source_elevator_door_open_trigger fires,
  elevator_arrive fires, departure_elevator-elevator_playerclip is killed
  3.5 s later, walking on enters the car, "close" fires and it rides down.
  The clip is meant to block the empty shaft until the car has arrived, so
  the barrier likely means the car hadn't arrived (walk-in trigger at
  -872 256 64 not touched?) or a save load put the clip back. Asked the
  owner whether the car had arrived and whether a save was loaded.

### T-237 sp_a2_column_blocker: Wheatley in the departure elevator doesn't look at you, moves through his rail  (P2, FIXED, awaiting owner replay; owner playtest 12c)
- 2026-10-23: his rail train started at its map origin, 34 u off its rail
  (Windows: on path_1), so he rode beside the rail; fixed by T-232 part 2.
  His eye (+Y) tracked the player within 2 deg during the ride (measured
  old and new), so the aim itself was fine.
- Owner: "wheatlys animation here while im in the elevator is pretty buggy
  like he doesnt look at you and hes moving through a rail".
- Same rig as T-232 (train -> top/bottom func_tank swivels -> @sphere). He
  follows the train here (old and new builds). Checking his aim: his model
  +X is ~89 deg off the player for the whole ride in both builds (eye axis
  not yet confirmed).

### T-232 sp_a2_bridge_the_gap: Wheatley glitches around; the bird looks wrong  (P2, Wheatley FIXED, awaiting owner replay; bird needs owner; owner playtests 11, 12)
- 2026-10-23 FIXED (movers.rs), two causes:
  1. follow_parents moved a child only if its parent had already moved
     that tick, in list order: Wheatley (train -> extension door -> top
     swivel -> bottom swivel -> @sphere) stayed put while his rail train
     moved and jumped ~150 u when it stopped. It now passes over the list
     until no child is left. Evidence (P2_PROPTRACE per-frame trace, window
     scene from start_wheatley_window_scene_relay): before one 148.6 u jump
     at the end of the ride; after he rides along (train and Wheatley
     sampled together every 0.2 s).
  2. Track trains now start on their first path_track (+ height), as
     Source spawns them. Windows (VScript GetOrigin over winref netcon):
     bridge_the_gap rail train (-816,-928,1400) = path_1, ours was at its
     map origin 112 u away; bird (-880,-904,1336) = its path_1; intro7 rail
     train (-296,-416,1592); column_blocker rail train (-1260,192,-1295.75),
     34 u off before; departure elevators 16 u lower than placed (all
     maps). Ours now match all of these. ~40 trains move at load (survey:
     bts3/bts5 panel trains by up to 2600 u, unchecked visually; those maps
     load).
- Regression: departure elevator rides in intro2 and column_blocker behave
  the same as before (arrive, close, ride down); 11 maps with moved trains
  load; 205 tests pass. A func_tank rewrite (absolute angles) was tried and
  reverted: the old aim already pointed Wheatley's eye (+Y) at the player.
- Bird: the scene plays (bird_attack_start_relay, wings flap via
  bird_wing_1/2); screenshots weren't possible while the owner played.
  Needs the owner to say what looks wrong.
- Owner (11): "wheatly is also a little buggy glitching around and shit and
  the bird that comes through and you can see"; (12): "wheatly and the bird
  are still pretty buggy". Was part of T-220.

### T-223 sp_a2_ricochet: the rotating platform under the cube is invisible and not solid  (P1, BLOCKED (needs owner replay: not reproduced on 18676be); playable; owner playtest 11)
- Owner playtest 12: "everything up to and including sp_a2_bridge_the_gap is
  now playable" (ricochet included); the platform itself wasn't mentioned.
- The platform surface (not the machinery) is missing, so the cube falls to
  the floor (still grabbable). Same family as T-221/T-222.
- Autorun 2026-10-23 05:27 NOT REPRODUCED (autopilot only, build 18676be,
  HEAD 43b5854). The platform is `panel_error_arm_03a` (func_brush *54,
  top nature/dirtfloor004c) parented to prop `error_arm_03a`
  (arm64x64_interior_rusty, keyvalue DefaultAnim bindpose; logic_auto
  onmapspawn SetDefaultAnimation error_arm_cube +
  SetParentAttachmentMaintainOffset panel_attach); the second one is
  `panel_error_arm_02b`. `P2_SCRIPT="3 goto juggled_cube 160; 3.5 lookat
  juggled_cube; 9 shot ..."`: the panel draws, turns with the arm, has a
  collider (top z -1264) and `juggled_cube` rests on it at z -1246 for the
  whole run; after `input error_arm_stop_relay Trigger` both panels keep
  their colliders and the cube stays. Likely fixed by T-221 (18676be): before
  it the map-spawn SetDefaultAnimation (here error_arm_cube) was applied
  before the spawn sequence, so the offset was measured against the wrong
  pose. Not checked on a pre-18676be build. Owner: replay ricochet's start
  (cube on the turning platform, platform visible and solid).

### T-217 sp_a2_laser_stairs: floor missing after GLaDOS's floor move; Wheatley seen through the closed wall  (P2, BLOCKED (needs owner); playable; owner playtest 10)
- Missing floor: look down a few seconds after the animation where GLaDOS
  moves the floor, at `setpos 121.78 -354.91 -90.00`.
- Wheatley: in the room where you get the laser cube, he appears for a
  moment and stays visible through the wall after the wall-closing
  animation: `setpos -529.75 327.04 -90.00; setang -5.15 87.45 0.00`.
- A floating texture to the right of that spot.
- Done when all three are gone and the map still plays through.
- 2026-10-23 missing floor REPRODUCED and fixed on branch attempt/T-217
  (2fe2ac4, pushed, not on main yet). Replay without the elevator: autopilot
  `3 input flooranima_go trigger; 8 input floor_collsion_brush enable;
  15 input robo_flooranima_NN disabledraw`, then `info brush_flooranima_NN`.
  Cause: a logic_auto sends each tile SetParentAttachmentMaintainOffset
  panel_attach, then each arm SetDefaultAnimation flooranima_NN_idle. We
  applied and posed the animation before handling the attach (anim_props
  chain ran before viewcontrol::track_animations), so the offset was taken
  against the idle pose; when the arms returned to idleend the tiles sat
  ~80 u low and tilted (tile 11 at (123,-274,-179) instead of (80,-352,-97)).
  Fix: track_animations runs before anim_inputs; follow_attachments and the
  viewcontrol inputs stay after pose_parts. After: all checked tiles at
  their map positions, flat.
- To finish: cargo test; regression on other attachment users (intro1
  container ride and camera proxy, Wheatley's rail in intro1/wakeup,
  sp_a2_intro's portal panel, laser_stairs' panel_error_arm and
  brush_armfollow_move03 tiles); shots of the owner's spot before/after;
  then make_app. Wheatley-through-wall and the floating texture: not started.
- 2026-10-23 05:40 autorun: missing floor is already fixed on main by
  T-221 (18676be, props spawn in their DefaultAnim): same replay on main
  (no attempt/T-217 code) leaves all 18 brush_flooranima_NN tiles flat at
  z -97 on the grid (tile 11 at (80,-352,-97)); mid-animation tile 11 moves
  (-46 at 5 s, -88 at 6 s) and comes back; shot at the owner's spot shows
  floor. The old attempt/T-217 (commit 2fe2ac4, branch deleted 2026-10-23) is
  not needed; don't bring it back.
  Wheatley: he is npc_personality_core sphere_peek01, parented to
  prop_dynamic sphere_bone. sphere_hide_trigger (trigger_look) plays
  sphere_peek_01, then _end at 3 s, then Kills sphere_bone at 4 s. Source's
  Kill removes entities parented to the target; ours only killed the
  target, so sphere_peek01 stayed alive. Untested fix: local branch
  attempt/T-217-wheatley (978c28c, io.rs: Kill queues Kill on children by
  parentname; cargo test p2_game 202 ok; autopilot: sphere_peek01 killed
  after the kill). Not shown to fix the report: in shots from the owner's
  spot, after sphere_peek_01_end he already leaves the window on old and
  new builds. Needs owner review/replay (where exactly he stays visible).
  The white arm with a cyan light seen through the robot_eyepeek window at
  the owner's spot is not Wheatley; robot_eyepeek_07/08 panels (attached
  with maintain offset at 0.2 s while the arms get close_idle at 0 s) look
  open from spawn: worth checking against Windows (scripts/winref.sh
  sp_a2_laser_stairs --pos -529.75 327.04 -26 --ang -5 87). Floating
  texture: not started.

### T-218 Performance: 60-70 fps and stutter on an M4 Pro; portals cost a lot; v-sync unclear  (P2, PARTLY FIXED: owner 2026-10-23 "fps is smoother"; more to gain, findings below)
- Owner (playtest 10): "60-70fps and its stuttery ... not sure if vsync
  works correctly, portals lower performance". His settings: fullscreen
  3840x2160, v-sync on + triple buffering, 2x MSAA, on a 120 Hz 6K display.
- Measured (P2_FRAMELOG=1, new; `--size 1920x1080` = his pixel count):
  the frame was render-thread bound everywhere (~10.7 ms at any size; even
  640x360 only reached 104 fps), so at 120 Hz v-sync frames alternate 8.3 and
  16.7 ms: 60-80 fps with judder, his "stutter". Chrome trace
  (bevy/trace_chrome): every camera (main, each portal level, the water
  reflection, the exposure probe) built constants for and drew every map
  surface; valve_render had no per-view culling.
- Fix (valve_render.rs prepare_surfaces): each draw's bounding sphere is
  tested against the view's own clip_from_world (the matrix the GPU clips
  with, oblique portal planes included), so a view only draws what is in
  its frustum. Same as projected_render's light culling.
- Evidence, fps before -> after, v-sync on, his settings. 2560x1440:
  catapult_intro room 94 -> 120 (locked), intro6 room 102 -> 120, intro6
  with a portal pair in view 59 -> 69, intro3 120 -> 120, laser_stairs 109
  -> 111. 3840x2160: 80 -> 90, 63 -> 66, 36 -> 40, 90 -> 91, 79 -> 80.
  Shots on 11 views (incl. portal views, water) match the old build within
  run-to-run noise. Tests: p2_game 202, source_assets 72 pass.
- Still open (findings, not fixed):
  1. 4K is GPU bound and 2x MSAA is the biggest cost there: intro6 room
     66 fps with MSAA, 103 without; motion blur costs nothing. Likely the
     MSAA HDR target being loaded/stored by each of ~8 render passes on a
     tile GPU. Until fixed: Antialiasing off (or 1440p) for 120 fps.
  2. V-sync OFF is capped at exactly 60 fps in a window (focused or not),
     below v-sync ON: wgpu maps it to Metal displaySyncEnabled=false and
     macOS paces such a window at 60. Exclusive fullscreen not checked
     (would switch the owner's display mode). Keep v-sync on.
  3. Each portal recursion level still costs ~1.5 ms CPU (Bevy's per-camera
     passes) and a full-resolution render; rendering only the portal's
     screen rectangle needs the portal and refraction shaders' screen UVs
     reworked (they use the drawing camera's viewport).

### T-224 Cube physics feel: no tumbling in the air; plate and button hits unreliable  (P2, TODO; owner playtest 11)
- Cubes don't rotate/tumble in the air like Portal 2 (T-198 skipped Source's
  random angular impulse on catapult launches; also check angular velocity
  kept through portals and when thrown).
- trust_fling: a cube missed the first faith plate once; dropping a cube on
  a button from high is hit or miss.

### T-225 No dissolve ("fizzle") effect on cubes  (P2, TODO; owner playtest 11)
- Missing when a dropper replaces a cube, when GLaDOS fizzles one
  (sp_a2_pit_flings) and at emancipation grills. Source: the prop dissolves
  (white-out material + particles + sound) before it is removed.

### T-226 Pedestal buttons (prop_button): no sound or press animation  (P2, TODO; owner playtest 11)
- The stand-up buttons you click: no press animation, no sound.

### T-173 Portal gun view model: hold beam, carry anims, fizzler shake  (P3, OPEN: hold beam "looks terrible"; fizzler shake owner-confirmed)
- Owner playtest 11 (2026-10-23): "the holding something animation looks
  nothing like portal 2, like the electricity looks really bad". Compare with
  a Windows reference shot of holding a cube (scripts/winref.sh).
- Owner round 5 (2026-10-21): "the gun hold beam is not fixed it looks
  terrible, the fizzler shakes are fixed". A screenshot or clip of ours while
  holding a cube would pin down what is wrong.
- Owner playtest 9: "portal gun still doesn't have that electricity when
  picking things up and doesn't spark/shake when going through the
  emancipation grids" (also T-150's gun beam).
- Cause: nothing drove the gun while carrying or at a grid. p2_fx only had
  the top light / muzzle flash; the viewmodel never played its carry
  sequences (ACT_VM_PICKUP idle_to_carrying, idle_carrying, carrying_to_idle)
  and played `fizzle` only on a failed shot.
- Fix: p2_core `GunHolding` (held object's world centre, kept by p2_game
  props::gun_holding; through a portal the hold point on this side) and
  `GunCleansed` (written on trigger_portal_cleanser touch when the gun can
  fire). p2_fx spawns `portalgun_beam_holding_FP` (portalgun.pcf: CP0 muzzle,
  CP1 object; the end point is rescaled for the 50-degree viewmodel FOV so it
  lands on the object; unit test hold_beam_end_matches_world_projection)
  on the viewmodel layer with the `Weapon_Portalgun.HoldSound` loop, only
  while the gun is shown. p2_view plays the carry sequences and `fizzle` on
  GunCleansed; p2_fx plays `Portal.FizzlerShimmy`.
- Evidence (autopilot only, sp_a1_intro4): `goto section_2_box_2 70; use`:
  after build shows the electric beam from the muzzle to the cube centre and
  the raised carrying pose, log "play Weapon_Portalgun.HoldSound"; before
  build (dist 69426f1) the cube floats with no beam and the idle pose.
  Walking through fizzler_brush: log "play Portal.FizzlerShimmy" and the gun
  mid-`fizzle` in the shot; before: nothing.
- Not done: a spark particle on the gun at the grid (no Valve effect name
  confirmed for it; the fizzle sequence shows no particle events in
  animinfo); the beam particles don't follow the gun between restarts
  ("Movement Lock to Saved Position Along Path" not implemented; the effect
  restarts often enough that it looked right in the shot). Owner replay:
  does the beam/fizzle feel like Windows Portal 2?

### T-227 Zoom doesn't work  (P3, TODO; owner playtest 11)
- Toggle zoom / zoom in / zoom out do nothing (keys exist in Edit Keys).

### T-228 Chapter HUD icons don't look like Portal 2's  (P3, TODO; owner playtest 11)
- Owner: "the chapter hud icons dont look like portal 2s". Ask which screen
  if unclear (chapter title card, chamber sign icons); compare with Windows.

### T-212 sp_a3_00 (start of chapter 6): loud glass smash at load  (P2, BLOCKED: owner 2026-10-23 still hears it; the fix on attempt/T-212 was never merged: merge it for his replay)
- Owner: "when you booted up chapter 6 the fall a loud glass noise played".
- Reproduced (autopilot log, RUST_LOG=portal2::audio=debug): in the first
  frame after sp_a3_00 loads, `play ScriptedSequence.MoronSmash08
  (moron_glass_hit_08.wav) gain 1.000`, right after the woosh and music. Not
  from the map's 4 ambient_generics (falling_whoosh, music, and the two
  debris sounds with flags 48 = start silent) nor its func_breakables
  (material 1 wood, break later on crash_landing_path_2). Suspected: a
  prop_dynamic's animation sound event (anim_props.rs EVENT_SOUNDS
  1004/1005/5004, AE_CL_PLAYSOUND) at cycle 0 of its default sequence fired
  on spawn; Source only fires events the cycle actually passes. Find the
  prop (log the model when an event sound plays), fix the first-frame rule,
  check other maps for sounds at load.
- 2026-10-21 autorun: source found. The sound is ent 479
  `exit_elevator_frame_model` (models/elevator/elevator_escape.mdl,
  parented to lift_rotator_2 at 0 -4096 416, right above the player start)
  with `defaultanim break` + holdanimation: sequence "break" (0.53 s) has
  `AE_CL_PLAYSOUND ScriptedSequence.MoronSmash08` at cycle 0 (SNDLVL_150dB,
  mixgroup xLoud, so it is full volume anywhere). Our first step from 0
  fires it. Not confirmed what Source does: the client-side event code
  backs up to fire 0th-frame events when it first sees a sequence, and the
  prop is in the player's PVS, so Windows may play it too (it continues
  sp_a2_core's moron-smash). Needs the owner's ear or a Windows check
  (T-215). Untested fix: local branch attempt/T-212 (9a9373e), needs owner
  review/replay: a default (non-input) sequence's frame-0 events are
  skipped on its first step; sp_a3_00 MoronSmash plays 1 -> 0 (autopilot
  log only). Also adds a debug log of animation event sounds.

### T-203 Switchable lights frozen: lightmap styles other than 0 ignored (50 of 63 maps)  (P2, TODO; upstream gap)
- Lightmaps use style 0 only (geometry.rs; P2_ALLSTYLES sums every style as
  a debug view). Named `light`/`light_spot` entities (50 of 63 maps have
  some) switch with TurnOn/TurnOff/Toggle and their light lives in extra
  lightmap styles: chambers lighting up as you enter, power-ups (intro1),
  lights going out (Wheatley, chapter 3/6 old Aperture, finale). Keep each
  face's style lightmaps and a per-style brightness (style index from the
  light's `style` keyvalue, inputs toggle it; patterns for flickering
  styles), blend in the lightmap shader. Unit test with a two-style face.

### T-199 Particle sprite sheets not decoded: sheet particles draw the whole atlas  (P2, TODO; upstream gap)
- p2_fx render.rs `SheetGrids` is empty ("the game's sheet layouts are not
  decoded yet"), so every SpriteCard material with a sequence sheet (smoke,
  sparks, splashes, droplets, dust, fire) shows its whole texture atlas per
  particle instead of one frame. Source stores the sheet in the VTF's
  resource entry tag 0x10 0 0 ("sheet": sequences, frames, per-frame
  rectangles and durations). Decode it in vtf.rs, feed per-sequence frame
  rects to the particle renderer (`sequence`, `animation rate`,
  `lifetime from sequence`), unit test against a real VTF. Likely also part
  of T-153's glittery spray and T-150's particle looks. Every map.

### T-214 All-maps load sweep findings (2026-10-21)  (P2, TODO)
- All 63 single-player maps load (autopilot `8 shot; 9 quit` each, build
  bd169f8): no panics, no load errors. Shots at arrival look right on most.
- Fallback shaders across maps (materials / maps): DecalModulate 129/29
  (grime/dirt decals, sign numbers: missing or wrong), SolidEnergy 78/51
  (fizzler and energy fields drawn on the fallback), UnlitTwoTexture 22/~20
  (T-201), "missing" model shader 11/10 and brush 3/3 (materials whose VMT
  isn't found: check which), Black 5, WindowImposter 2/1, PortalRefract
  (model) 2/1. Do DecalModulate and SolidEnergy first: they are in most maps.
- Models that fail to load: props_destruction/toxintank_intact, _a*,
  _implode ("vtx: Trying to read past the end"), effects/urban_puddle_model*
  ("unreadable vtx"): fix the VTX reader for these.
- Gel blobs: renderer `render_blobs` unsupported (T-211), constraint
  "Constrain distance to path between two control points" (blob_curve*).
- Looks wrong at arrival: sp_a3_jump_intro all white-grey at 8 s (looking
  ahead; fine after turning: check fog/what's in front), sp_a5_credits plain
  white with the gun out (the credits map needs its own handling: no gun,
  credits text/song, the turret opera). Arrival-elevator movie panels flat
  teal/blue on many maps belong to T-174 (on hold).

### T-201 UnlitTwoTexture (TV/monitor screens) and other model shaders on the fallback  (P3, TODO; upstream gap + sweep)
- models/props_motel/tv_screen and other monitors use UnlitTwoTexture
  (two textures multiplied, often a scrolling scanline); we draw them on the
  fallback (22 materials over ~20 maps in the 2026-10-21 sweep, T-214).

### T-206 Sounds not panned left/right  (P3, TODO; upstream gap)
- All sounds use distance gain only (p2_fx sounds, p2_game audio): no stereo
  position, so you can't hear where turrets, Wheatley or GLaDOS's panels
  are. Add Source-like panning (and the soundlevel falloff check) with a
  listener on the camera; through-portal sound is a later step.

### T-200 Particle operators the maps' effects use but p2_fx doesn't simulate  (P3, TODO; upstream gap + audit)
- `fxaudit` (p2_fx example) over every map's info_particle_system effects
  (2026-10-21; maps / systems): Collision via traces 13/16 (sparks, confetti,
  arcs fall through floors), Set Control Point to Impact Point 9/4, Remap
  Percentage Between Two Control Points to Scalar 8/2, Remap Initial Scalar
  6/15 (fire jets, arc glows), Lifetime Pre-Age Noise 4/3, Lifetime from Time
  to Impact 4/2, Movement Maintain Position Along Path 4/2, Constrain
  distance to path between two control points 3/5 (gel blob curves),
  Position From Parent Particles 3/1, Force based on distance from plane,
  lifetime from sequence 1/8 (env_fire_*), Cull when crossing plane, Remap
  Velocity to Vector, Ramp Scalar Spline Simple, Position from Parent Cache.
  Upstream also lists: Lerp Initial Scalar, `$depthblend` soft particles,
  operator time offset/scale, Refract `$bluramount`. Take them in that order.
- Not bugs: cleanser_edge_1, dust_ceiling_rumble_256line,
  portal_2_badvolume, waterleak_pipe_1_traildrops_1 are referenced by maps
  but exist in none of the game's 78 .pcf files (Source skips them too).

### T-202 No bloom or colour correction (every map)  (P3, TODO; upstream gap)
- Portal 2 adds bloom (engine_post) and a colour-correction LUT
  (color_correction entities, .raw LUTs in materials/correction) on top of
  auto-exposure; we have auto-exposure only (exposure.rs). Bloom makes
  lights, screens, portals and the gun glow; the LUTs tint whole areas
  (e.g. old Aperture). Related to T-152's look but not blocked on its
  screenshots.

### T-204 No env_cubemap reflections or dynamic lights on props  (P3, TODO; upstream gap)
- Materials with `$envmap env_cubemap` (glossy floors, metal, glass, the
  portal gun) get no reflection: the BSP's built cubemaps (pakfile
  materials/maps/<map>/c<x>_<y>_<z>.vtf, the cubemap lump) aren't loaded or
  assigned to the nearest surface. Props also ignore dynamic local lights
  (light_dynamic, named lights). Also: displacement lightmap UVs are an
  approximation. Overlaps T-152 (on hold); this part needs no screenshots.

### T-211 Gels: check each against Source  (P1, TODO; campaign chapter 6+)
- paint.rs / paint_props.rs implement blue (bounce), orange (speed), white
  (portal) and water (erase), gel on props, paint_sphere, prop_paint_bomb,
  trigger_paint_cleanser, but the tuning is estimated ("not measured
  against the game yet"). Check each against Windows Portal 2 behaviour
  (server.dll cvars bounce_paint_*, speed_paint_*, paint_* defaults):
  bounce height from a fall and off walls, speed-gel top speed/accel/slide,
  speed+bounce combos, white gel portal placement and when it's washed off,
  cubes/turrets on each gel, blob arcs and splat size, the player's view
  getting splashed (not done), gel through portals. Prove sp_a3_jump_intro,
  sp_a3_speed_ramp, sp_a3_portal_intro first.

### T-030 Gel chapters: missing buttons in gel rooms, black elevator, cube carry feel  (P2, TODO)

### T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug  (P1, TODO)

### T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)  (P1, TODO)

### T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)  (P1, TODO)

### T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)  (P1, TODO)

### T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)  (P1, TODO)

### T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)  (P1, TODO)

### T-015 Sweep chapter 9 (finale1..finale4)  (P1, TODO)
- Sweep = per map: load, logic runs, exit reachable, next map loads (L/O/N
  columns in PROJECT_STATE.md); proving each puzzle follows per map.

### T-205 Portal gun edge cases  (P3, TODO; upstream gaps)
- Not done (p2_gun README): shooting with the eye behind/inside a portal
  (out of the far side); refusing a floor portal under the standing player;
  re-testing portal support every 0.1 s so a portal fizzles when its surface
  disappears (only listed movers do it now); mobile portals on moving
  panels; several linked portal pairs at once (unverified; single player
  normally has one gun pair plus map portals).

### T-209 Portal transit only mirrors box-shaped geometry at the exit  (P3, TODO; upstream gap)
- p2_transit copies only cuboid colliders as remote geometry; trimesh
  (displacements, props) near an exit portal isn't mirrored, so walking
  through a portal next to such geometry can snag or let you clip. Check
  with a portal on/near displacement terrain (old Aperture, chapter 3/6).

### T-207 Viewmodel gaps  (P4, TODO; upstream gaps)
- Gun lit by a fixed neutral ambient instead of the room's light; no
  clipping/offset when close to a portal (portal_viewmodel_offset);
  cl_mouselook_roll_compensation not done.

### T-208 Chell body gaps  (P4, TODO; upstream gaps)
- No head look, no turn-in-place animation (legs snap past 45 deg), no foot
  IK, body casts no shadow. Seen through portals.

### T-210 Upstream minor gaps  (P4, TODO)
- Portal rim faint/thin and projectile stream dim next to Portal 2's; no
  sound/burst for shots that hit nothing (sky); kinematic bodies block
  shots; mouse sensitivity vs a real config (m_yaw 0.022 x 3); intermittent
  shutdown crash (p2_view); 2D skybox face orientation unverified.

### T-090 Restart check per verified map  (P2, TODO)

### T-040 App icon and second-Mac first-run check  (P3, TODO)

### T-050 Verify merged cloud features in game: Wheatley look-at, gels, projtex, saves  (P3, TODO)
- Checklists in docs/STATUS.md.

## Awaiting owner replay
(none; when a task is fixed its header says so and it stays above in place)
