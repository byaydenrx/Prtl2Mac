# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (see WORKFLOW.md). Work top to bottom: the first TODO/IN_PROGRESS task
whose dependencies are DONE. When a task finishes, move it to Done (newest
first) with its evidence and update PROJECT_STATE.md.

"Prove a map" (the standard acceptance for T-002..T-015): from the map's
start, an autopilot script plays the intended solution using movement,
`fire`, `use` and props (no `setpos` past a puzzle step, no `input` on
puzzle entities); the exit door opens from the puzzle's own outputs; the
next map loads. Record the script in `scripts/routes/<map>.txt` and the run
date. Any engine bug found becomes its own task (P1 if it blocks the route).

## Active

## Owner playtest blockers (P1, play order)

### T-102 Portal traversal: cubes and player through portals  (P1, NEEDS_VERIFICATION, owner replay)
- Owner (intro4, intro2 after the 2026-10-19 build): cubes don't go through
  portals; after a cube got through once, a portal stopped working.
- Causes found 2026-10-19 (all reproduced with autopilot first):
  1. A cube in a map portal's hole when the map re-activates it (intro2 does
     on approach) was listed as the portal's wall; the cube moving then
     fizzled the portal ("portal on a moving surface"). Walls are now static
     colliders only (portals.rs `WallBodies`).
  2. Carrying a cube into a portal: the hold point jumped to the far side at
     once, dragging the cube into the wall, and the player's teleport mapped
     a cube already through a second time; it fell out of the world. Now the
     cube is pulled to the hold point's copy on its own side and only carried
     with the player if still at the entry (props.rs).
  3. Floor portals: only the brush the portal sits on was cut out, so a
     second brush layer 6 u below held the player and cubes (intro4). Static
     world in the 80 u tunnel behind a new portal is now cut out too
     (portals.rs `clear_portal_tunnels`).
  4. Exit-side mirroring copied dynamic props (collider child of the body)
     and triggers as static walls behind the entry (p2_transit).
- Evidence (autopilot, no setpos across portals): intro2 carry cube through
  red->blue, turn, carry back blue->red, cube stays in front; intro4 player
  falls through a gun floor portal out of the ceiling portal; two loose cubes
  fall through it; walking into a floor-level wall portal keeps the floor.
  Unit test held_prop_maps_with_the_player_only_once. Workspace tests pass.
- Not yet: the owner's play of intro2-intro4 by hand; thrown cubes.

### T-103 sp_a1_wakeup: breaker lift doesn't carry the player  (P1, NEEDS_VERIFICATION)
- Cause: the lift (func_tracktrain breaker_train, max speed 1) is sped up
  by SetMaxSpeed 52 / 155 from the map; SetMaxSpeed wasn't handled, so it
  crawled at 1 u/s while the shaft animation rose. Now handled (movers.rs).
- Autopilot: basement_breakers_start -> the player rides from -476 to 351,
  path 3 fires basement_elevator_complete_rl.
- Owner: when the pod moves up switching the breakers, you don't move with it. Unplayable.
- Verify: stand on the lift and ride it to the top without setpos.

### T-104 sp_a2_intro: the incinerator drop  (P1, NEEDS_VERIFICATION)
- Intended (owner + video https://www.youtube.com/watch?v=ddV1ArV_GHc): a
  free-fall cutscene facing down through the pipes, no crosshair, no gun,
  no mouse look, landing on a platform; then walk to the gun pedestal.
- Map: trigger at the spawn -> relay_ghostanim: camera_ghostanim (flags 36:
  freeze player) rides ghostanim attach_1 through incinerator_drop (21 s);
  OnAnimationDone -> TeleportToView.
- Fixed 2026-10-20 (four causes):
  1. Quaternion48S decoded wrongly: the flag bits are (first word's top bit
     << 1 | second's), index n leaves out slot (n+3)%4 (w, x, y, z), the
     stored three follow it (Facepunch's Source 1 importer, sbox-public
     MdlFile.ReferencePose.cs). The root of incinerator_drop turned 90 deg
     about X instead of Z (path sideways out of the map = the "blue
     screen"); per-frame rotations jumped. Unit test quaternion48s_layout;
     ghostanim sequences now turn smoothly (incinerator_drop worst 4.8 deg
     per frame, was 78).
  2. A 120 m limit on rig cameras dropped the camera 4.7 s into the 445 m fall.
  3. Freeze-player cameras now hold the player (they free-fell into the pit
     trigger: white fade, death).
  4. The level starts without the gun; the view model hides during cameras.
- Autopilot: the player stays at the top for the 21 s, then stands at
  (-3232, 568, -10792) at the bottom; the room's landing triggers fire.
  Not verified by eye (screenshots of cutscenes come out black).
- Quaternion48S change affects every frame-animation rotation (intro1
  wake-up camera, Wheatley, GLaDOS): owner should recheck those.

### T-105 Wheatley can't be picked up again once dropped/stuck  (P1, TODO)
- Owner: if he gets stuck and you drop him, you can't pick him up; the original allows drop and pick up.
- Verify: drop and pick up in sp_a1_intro7 and sp_a1_wakeup.

### T-106 sp_a1_wakeup: crouch needed to enter the breaker room  (P1, NEEDS_VERIFICATION)
- Two causes: a full-height step up onto the lift's 4 u lip lifted the head
  into the lintel (lower steps are tried now), and the raised door leaves
  exactly 72 u, which the movement skin made impassable (hulls are now
  2 * SKIN shorter, as Source lets an exact fit through). Autopilot: walks
  in standing. Regression: intro2 carry and intro4 doors unchanged.
- Owner: you have to crouch to get in (then Wheatley drops and auto-plugs since he's close).

### T-107 Cubes and floor buttons invisible (they work)  (P1, CAN'T REPRODUCE)
- Owner screenshot 8 (2026-10-20, intro2 button room, cube held above the
  button): the button's corner tiles drawn, its red top and ring gone, a
  black ellipse in the pit and a straight diagonal edge cutting the model,
  like a portal clip plane or tunnel cut-out applied to it (portal_blue_1 is
  ~200 u away). Not reproduced: load, changelevel from intro1, cube dropped
  and lifted 14 times, BindPose forced, views from above all draw it whole.
  Next: try with portal_blue_1/portal_red_1 active and the cube carried
  through a portal onto the button.
- Owner playtest of 0.1.0+4858bee (intro2..intro6): cubes and buttons can't be seen but collide and press.
- 2026-10-19: screenshots show them drawn in intro2 and intro4, held, after a
  portal, after a level change, and from the packaged app run like Finder.
  Owner later saw them drawn too. Need the map and moment if it happens again.
### T-108 A chamber door had an invisible barrier, needed noclip  (P1, NEEDS INFO)
- Door walk test (every prop_testchamber_door in intro4, opened by its own
  relay, walked through both ways): door_2 and the middle door pass; door_0
  and door_1 close again because the test teleports into the map's
  close-behind-you triggers (correct). Need which door/map from the owner.
- Door changes in T-119 keep the open door drawn, so the doorway shows what blocks it.
### T-119 Door open animation cuts to black; a black plane you can walk through  (P2, NEEDS_VERIFICATION)
- Cause: an opening prop_testchamber_door was hidden 0.35 s in (from before
  doors animated), showing its func_areaportalwindow brush, which we drew
  always. Fix: doors stay drawn (their open sequence and idleopen play) and
  only lose collision; area-portal window brushes are not drawn (Source draws
  them only for distance fades, none in door instances). Screenshots of
  intro4 door_2 opening show the room beyond, no black plane.
### T-120 Elevators: the trash rising with the elevator is buggy; one ride went to the bottom before the next map loaded  (P2, TODO)

### T-121 Portal under a resting cube: the cube doesn't fall in (intro4, intro6)  (P1, NEEDS_VERIFICATION)
- Owner playtest 3 (2026-10-19). Shots already pass through cubes; the cube
  kept the floor contact it had before the portal opened (pair filters only
  run when contacts start). Fix: props in a portal hole drop those contacts
  every step (props.rs `modify_contacts`). Autopilot intro4: a cube resting
  on the floor falls through a portal shot under it and out of the ceiling.
### T-122 func_portal_detector not implemented: intro7 can't progress  (P1, NEEDS_VERIFICATION)
- Owner: a portal behind Wheatley didn't register. Detectors now fire
  OnStart/EndTouchPortal (+1/2, Linked, BothLinked) as portals open or leave
  their brush (portals.rs). Autopilot intro7: a portal in Wheatley's room
  fires portal_detector -> look_at_me_shortcircuit_relay and the script.
### T-123 Portal shots should pass through cameras (and knock them off) and other non-solid props  (P2, TODO)
- Owner intro6: can't shoot a portal under a security camera; in Portal 2 the camera falls off.

### T-124 sp_a1_intro7: Wheatley doesn't fall off the management rail  (P1, NEEDS_VERIFICATION)
- Owner playtest 4: after the portal behind him (T-122 works), he never drops.
- Cause: the drop is GLaDOS's scene table (sp_a1_intro7OnThree04 fires
  `@sphere ClearParent`, the impact trigger, the detach spark); cores spawn
  kinematic and ClearParent didn't free them. Now ClearParent makes a core
  dynamic unless DisableMotion (or spawnflag 8) froze it.
- Autopilot: sp_a1_intro7_OnThree() -> chain of 4 lines -> Wheatley falls
  from z 1419 to the floor (1239), sphere_impact_trigger fires.
- Owner playtest 5: he falls and can be carried and plugged in (verified).
### T-125 sp_a1_intro7: turning away from Wheatley doesn't register  (P1, NEEDS_VERIFICATION)
- ai_script_conditions treated PlayerTargetFOV -100 ("target NOT within
  100 deg") as a 50 deg cone toward it: it fired while facing him and never
  after turning. ai.rs in_fov now handles negative cones and the 2D cone
  (unit test negative_fov_means_looking_away). Autopilot: enabled while
  facing him it stays quiet; it fires 1.5 s after turning away.
### T-126 Tilted map portal drawn half (intro7 portal_b_rm4)  (P2, NEEDS_VERIFICATION)
- The panel's drawn face sits within ~1 u of the portal plane but crosses
  it; the portal quad stood off 0.16 u. Now 1 u (p2_render quad_standoff),
  like a depth bias. Screenshots: whole portal from two angles; an intro2
  wall portal and walking through it look as before.

## Owner playtest issues (P2)
- T-110 (NEEDS_VERIFICATION) Fix: speech plays on its actor's voice channel, so an actor's new line cuts the previous one (Source CHAN_VOICE); busyactor 2/3 cancel the actor's running scene. Verified by log in intro1 (Wheatley and announcer lines). Different speakers can still overlap, as in Source. Report: jumping/saying "apple" while Wheatley talks, opening the door early; Portal 2 queues or interrupts by scene priority.
- T-111 Music stops at the stairs (wakeup, way to GLaDOS).
- T-112 Missing textures: hallway to GLaDOS, after the jumps (plus a black wall you can walk through), GLaDOS chamber floor drawn as a warm sky, a blue flat wall behind the intro4 elevator.
- T-113 Instructor hints vanish after a few seconds if not done. (Key labels FIXED 2026-10-20: white text on the white key cap, now dark.)
- T-114 intro1: the Wheatley door opens on approach without pressing E.
- T-115 intro1: wake-up animation still wrong; no wake/sleep animation on the bed.
- T-116 intro1: the first room's window is a grey block (should be blinds etc.); the TV screen sits forward and up from the TV; glitchy screen at the start of the ride; odd outside lighting on the ride.
- T-117 Shadows are blocky (shadow-map resolution and filtering).
- T-118 intro4: loud glass-breaking sound (check against the original); the door animation cuts to black.

## Owner playtest 3 (2026-10-19, build 2cf59a9), by map (P2 unless noted)
- intro1: wake-up animation; blocky shadows (T-117); key hints lack labels (E, SPACE: T-113); Wheatley and announcer overlap in the dark room; the moving room jerks the camera and moves the player around; floating parts during the room move; Wheatley moves oddly after the ride and is a black box seen from below during it; 12:00 clock blinks its background too (only the digits should); Chell seen through the first portal holding a gun she hasn't got; no press-down animation on the button; saying "apple" when asked skipped his next line; when the door opens Wheatley's first lines and the announcer talk over each other (neither understandable).
- intro2: cubes don't sit still on buttons (autopilot: a dropped cube settles in 0.5 s with one press/unpress flicker); stairs to the exit elevator missing (floor and sides empty).
- intro3: the water below the falling platform is far too bright orange.
- intro4: glass-break sound on arrival (T-118); a stray screen in front of the stairs to the elevator.
- intro5: a floating grass texture; a missing wall panel.
- intro7: the auto-placed orange portal (portal_b_rm4, tilted 75 deg onto a panel) shows only half (T-126).

## Owner playtest 5 (2026-10-20, build 33bca15), by map (P2 unless noted)
- intro7: the turn-away check worked but felt unreliable (took a while); turning back to pick Wheatley up, he floated attached to nothing; the moving wall that opens disappears once its open animation ends (passable).
- sp_a1_wakeup: missing textures in the hallway to GLaDOS's chamber and her chamber floor drawn as sky (T-112); crouch needed to enter the main breaker room (T-106), which drops Wheatley, who then floats there; he still plugs in, but the breaker platform rises without the player (T-103, P1 blocker); noclip couldn't fly up past it either (player stuck).

## Owner playtest 6 (2026-10-20, build 06cf3e3), by map (P2 unless noted)
- sp_a1_wakeup: the lift rises (T-103 ok) but as the breaker platform turns the player should turn with it (rotating mover doesn't carry/rotate the player); the player can jump out of the hatch (shouldn't be possible) and a stone-brick-looking block appears there; GLaDOS wake-up scene is buggy: white particles all around her that shouldn't be there; the portal gun didn't visibly drop to the floor (check); the player is made to look the wrong way.
- sp_a2_intro (P1 blocker, T-104): ~10 s of blue screen before anything shows; the fall should play as a cutscene but doesn't; the player already has the portal gun (should pick it up again); the player lands in the incinerator instead of on the platform and is stuck on a white screen.

## Owner playtest 7 (2026-10-20, build b2f0080), by map (0 = intro1 ... 6 = intro7, then wakeup, sp_a2_intro)
- REGRESSION (P0, NEEDS_VERIFICATION): "weird objects" everywhere (elevators, elevator rooms, chamber entrances, moving stairs, attached to GLaDOS) and floor buttons disappearing from intro4 on. Owner screenshots 2026-10-20 12-16 (intro3) explained, see "Owner screenshots 2026-10-20" below: untextured slabs (fixed), static props left out (fixed), elevator-tube rubble (real map content, tube_blockage). Owner to recheck the other maps on the new build.
- sp_a2_intro fall: the crushing arms in the shaft look wrong (owner screenshot 2026-10-20: spiky dark shapes around the central arm).
- intro1: intro animations now right; Wheatley's first lines and the announcer still overlap; the box ride is less buggy but walls vanish and reappear before breaking off; Wheatley above is a black box; the whole 12:00 clock blinks (only the digits should); through portals Chell is seen holding a portal gun the player doesn't have; cubes twitch on buttons and buttons have no press animation.
- intro2: cubes through portals not seamless; exit stairs still missing (gap with no floor or sides).
- intro3: water after the fall still wrong; some textures flat white/empty before it; stray objects in the elevator room (one inside the elevator, one behind the door).
- intro4: stray objects in/after the elevator, in the level and next chamber; chamber doors look wrong; buttons gone; a stray object after the glass breaks underneath.
- intro5: stray objects in the elevator room and moving ones at the chamber entrance and moving stairs; a voice line starts late; the elevator descends far too long.
- intro6: stray objects; a cube still didn't fall into a portal (noclip needed).
- intro7: Wheatley's rail connector invisible; the turn-away check still slow; walking into the breaker room drops Wheatley (he auto-plugs).
- sp_a1_wakeup: the player rises with the breaker platform but doesn't turn with it; GLaDOS wake-up still has white particles and stray objects attached to her; floor texture missing in her chamber; a portal shows where she drops the player that shouldn't.
- sp_a2_intro: fall cutscene right and lands in the right place; the player gets a portal gun back on landing (FIXED: the map gives the gun by a console command, which the start-without-gun rule didn't count); the dual gun pickup works; after GLaDOS moves a wall to be portaled, it won't take a portal (FIXED, see T-127).
### T-127 sp_a2_intro: the panel GLaDOS moves into place won't take a portal  (P1, NEEDS_VERIFICATION)
- The panel (collapsing_rubble_instance-robo_flopintoplace_04panel, white tile, tilted down at the pit) is a "never solid" func_brush on a robot arm; it had no collider, and a nodraw/clip world brush sits flush with its face. Now: never-solid parented func_brushes keep their portalable faces as sensor colliders only portal shots stop at (movement and physics ignore sensors); the gun takes a portalable surface flush with a non-portalable one; attached entities' transforms are written only when they move (otherwise a portal on them fizzled as "on a moving surface").
- Autopilot: after panel_lowering_relay, an orange portal placed on the panel stays.

## Owner screenshots 2026-10-20 00:19-00:28 (~/Documents/Screenshots), by number
1. intro1 window a grey block (T-116) - TODO.
2. Blocky shadow of the hanging planter (T-117) - TODO.
3. Key hints without the key - FIXED (dark text on the key cap), autopilot screenshot shows E and SPACE.
4. intro1 ride: Wheatley a black box above - TODO.
5. intro1 ride: spiky pieces (wall destruction / docking station debris) - TODO, check skinning: animated props are drawn as rigid per-bone triangles (models.rs split_by_bone), soft-weighted debris would tear.
6. Chell seen through a portal holding a gun before having one - FIXED, NEEDS_VERIFICATION by eye: nogun_* sequences and no world gun while no gun can fire.
7. A dark shard in a door (intro2) - TODO, not looked at.
8. Invisible button, buggy held cube (intro2) - see T-107.
9. intro2 exit stairs invisible, a gap - FIXED: static props with flag 0x4 were skipped (769 of intro1's 1080 props too); stairs drawn and walked down to the elevator (autopilot).
10. intro3 flat lavender-grey slabs around the pedestal - FIXED: VertexLitGeneric $normalmapalphaenvmapmask now masks the envmap with the bump alpha (black_floor_metal_004b rubble). Bump detail itself still missing (bumped VLG not driven).
11. intro3 water bright tan - FIXED: $lightmapwaterfog lights the fog; flowing water's normals in world space at $flow_bumpstrength; no sky reflection on fogged maps (sky_white turned it white). Now dark murky brown.
12-16. intro3 "weird objects": the flat slabs (10), the previously missing static props, and the elevator-tube rubble (tube_blockage_*, real map content) - owner to recheck. Shot 15's tilted panel and 14's beams not yet matched to an entity.

## Backlog

### T-002 Prove sp_a2_dual_lasers  (P1, TODO)
- Outcome: both laser catchers powered by the player's redirection, door opens, next map.
- Files: likely lasers.rs, portals.rs, autopilot.rs; scripts/routes/.
- Depends on: none. Verify: autopilot route run + log of catcher outputs.

### T-003 Prove sp_a2_laser_over_goo  (P1, TODO)
### T-004 Prove sp_a2_catapult_intro  (P1, TODO)
### T-005 Prove sp_a2_trust_fling  (P1, TODO)
### T-006 Prove sp_a2_pit_flings  (P1, TODO)
### T-007 Prove sp_a2_fizzler_intro  (P1, TODO)
- Each: "Prove a map" acceptance above. Faith plates already land on target
  (scripts/check_faith_plates.py), so T-004..T-006 test the player's route.

### T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug  (P1, TODO)
- Split one task per map when started.

### T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)  (P1, TODO)
### T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)  (P1, TODO)
### T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)  (P1, TODO)
### T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)  (P1, TODO)
### T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)  (P1, TODO)
### T-015 Sweep chapter 9 (finale1..finale4)  (P1, TODO)
- Sweep = per map: load, logic runs, exit reachable, next map loads (fills
  L/O/N columns in PROJECT_STATE.md). Proving each puzzle follows as
  per-map tasks.

### T-090 Restart check per verified map  (P2, TODO)
- Outcome: on each VERIFIED map, dying and loading the autosave/quick save leaves the puzzle solvable.
- Verify: autopilot `click`/console load after progress, then the route again.

### T-020 sp_a1_intro1 ride: glitch frames and wall destruction  (P2, TODO)
- Outcome: no frames outside the container; walls break away (actor_wall_destruction_*).
- Verify: frame dump during the ride (autopilot `shot` every 0.2 s) and owner check.

### T-021 Owner manual checks  (P2, NEEDS_VERIFICATION, owner)
- sp_a1_intro1 ride, sp_a1_wakeup socket, sp_a2_intro portal route.

### T-030 Owner reports: missing buttons in gel rooms, black elevator in gel chapters, cube carry feel  (P2, TODO)
- Reproduce in sp_a3_* maps during T-012/T-013 first.

### T-040 App icon and second-Mac first-run check  (P3, TODO)
### T-050 Verify merged cloud features in game: anim round 2 (Wheatley look-at), gels checklist, projtex, saves  (P3, TODO)
- Their checklists are in docs/STATUS.md.

## Done
- 2026-10-19 T-101 sp_a1_intro1 ride end: the wall breaks, the player walks on, the elevator to sp_a1_intro2 works (owner playtest of build 0.1.0+921edff). Cause: model animation events and math_counter values (921edff).
- 2026-10-19 make_app.sh keeps dist/Prtl2Mac-previous.app and stamps a build id (BUILD.txt, CFBundleVersion).
- 2026-10-19 Workflow docs (WORKFLOW.md, TASKS.md, PROJECT_STATE.md, SESSION_HANDOFF.md, CLAUDE.md, scripts/resume.sh).
- 2026-10-19 sp_a2_intro logic: gun pickup, both portals, exit door, next map (autopilot) - 01c83ec.
- 2026-10-19 sp_a1_wakeup socket: CORE_TRIGGER_REACH lets the held core plug in (autopilot).
- 2026-10-19 Packaging: dist/Prtl2Mac.app finds the install with no env vars.
- 2026-10-18 Chapter 1 intro2..intro7 verified (autopilot).
