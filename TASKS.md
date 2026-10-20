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
2. T-114 intro1: Wheatley's door waits for the announcer.
3. T-120 Elevators (jitter, VO timing, thumps) and T-135 turrets falling through them.
4. T-133 intro7 Wheatley floats after the wall panel; panel vanishes.
5. T-134 sp_a1_wakeup pod: the player doesn't turn with the floor; T-163 hatch.
6. T-136 sp_a1_wakeup end: a portal in the tube before the drop.
7. T-137 Portal spacing; T-102 seamless cube carry; T-125 turn-away delay.
8. T-145 Buttons don't animate, cubes don't light up.
9. T-144 Missing props; T-147 black screen as intro2's first door opens.
10. T-139 Ambient audio, button sounds, music cut-offs, loud glass.
11. T-149 Chamber signs blank; screens and lights.
12. T-112 Missing/flat textures (GLaDOS chamber floor, hallways).
13. T-123 Security cameras knocked off by portal shots.
14. T-160 Main menu like Portal 2's.
P3:
15. T-161 Menu video and music; T-162 menu animation, logo, styling.
16. T-020 intro1 ride; T-164 spiky debris; T-116 intro1 room; T-117 shadows; T-115 wake-up.
17. T-153 GLaDOS wake-up; T-157 smoother fall transition; T-165 Wheatley rail connector.
18. T-148 Portals behind glass; T-140 shake and sound for game-placed portals.
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

### T-114 intro1: Wheatley's door and the announcer  (P2, TODO)
- Wheatley's first lines overlap the announcer; the door opens on approach.
  Owner: the door must not open until the announcer has finished. Also:
  saying "apple" when asked skipped his next line (playtest 3).

### T-120 Elevators  (P2, TODO)
- Rides jittery (section 1); the VO after intro5's second button started
  halfway down and stretched the ride (T-138); thumping sounds during the
  ride (section 4); the trash rising with the elevator is buggy; one ride
  reached the bottom before the next map loaded.

### T-135 Turrets fall through the exit elevator  (P2, TODO)
- Sections 5 (sp_a2_intro) and 6 (sp_a2_laser_intro).

### T-133 sp_a1_intro7: Wheatley floats after the wall panel moves; the panel disappears  (P2, TODO)
- Playtests 5 and 8. He should fall to the floor; the moving wall vanishes
  once its open animation ends (passable).

### T-134 sp_a1_wakeup: the rising pod's floor turns, the player doesn't  (P2, TODO)
- Playtests 6, 7, 8: rotating movers must turn and carry what stands on them.

### T-163 sp_a1_wakeup pod: the player can jump out of the hatch; a stone-brick block appears there  (P2, TODO)
- Playtest 6.

### T-136 sp_a1_wakeup end: a portal shows in the tube before GLaDOS drops the player  (P2, TODO)
- Playtests 7 and 8.

### T-137 Portals can't be placed as close together as in Portal 2  (P2, TODO)
- Section 5. Reference: server.dll placement rules.

### T-102 Cubes through portals not seamless  (P2, TODO)
- Sections 1-2 (intro2, intro3). Traversal itself works (owner replays).

### T-125 sp_a1_intro7: the turn-away check is slow to register  (P2, TODO)
- Works (negative FOV cone fixed) but takes a moment (playtests 5, 7, 8).
  Compare ai_script_conditions timing (MinTimeout, think rate) with Source.

### T-145 Buttons don't animate; cubes don't change colour on a button  (P2, TODO)
- Section 0: no press-down when used or when a cube lands; cubes keep their
  off skin. Cubes twitch on buttons (one press/unpress flicker as they settle).

### T-144 Missing props: intro1 portal room clipboard and movable clock  (P2, TODO)

### T-147 intro2: the screen goes black right as the first door opens  (P2, TODO)
- Section 1; compare T-119. Screenshot 7 (intro2): a dark shard in a door.

### T-139 Audio  (P2, TODO)
- No soundscapes/ambience (water drips), no button clicks (sections 1, 5).
- Music stops abruptly sometimes; stopped at the wakeup stairs (old T-111).
- Glass break far too loud entering intro4 while still in the elevator (old T-118).

### T-149 Chamber signs blank; screens and lights wrong  (P2, TODO)
- The white chamber-number screens ("02") are blank in every level;
  intro5's screens and lights wrong (section 4).

### T-112 Missing or flat textures  (P2, TODO)
- GLaDOS chamber floor brown and flat like the old intro3 water (section 5);
  hallway to GLaDOS; after the jumps a black wall you can walk through; a
  blue flat wall behind the intro4 elevator.

### T-123 Portal shots should knock security cameras off  (P2, TODO)
- intro6 and sp_a2_laser_stairs: in Portal 2 the camera falls off.

### T-160 Main menu like Portal 2's  (P2, TODO)
- Top level: Play Single Player, Play Cooperative Game, Community Test
  Chambers, Options, Extras, Quit (unbuilt ones may do nothing yet).
- Play Single Player: Continue Game, New Game, Load Game, Challenge Mode.
- New Game: a vertical chapter list, not the gallery.
- Layouts and art are in the install: resource/ui/basemodui/*.res
  (mainmenu, campaignflyout, chapter dialogs), materials/vgui.

### T-113 Instructor hints vanish after a few seconds if not done  (P2, TODO)
- Key labels fixed (owner confirmed).

## Open: P3

### T-161 Menu background video and music  (P3, TODO)
- media/menu_act01..05.bik by campaign progress instead of a still (ffmpeg
  at /opt/homebrew/bin); menu music.

### T-162 Menu animations, logo alignment, styling  (P3, TODO)
- Panel/flyout transitions flip oddly; the Portal 2 logo is misaligned.

### T-020 sp_a1_intro1 ride  (P3, TODO)
- Walls flash away and back before breaking off; Wheatley a black box above
  (screenshot 4); the room jerks the camera and moves the player; floating
  parts; Wheatley moves oddly after the ride; glitchy screen at the start.
- Verify: `shot` every 0.2 s during the ride, then the owner.

### T-164 Spiky, torn animated debris  (P3, TODO)
- intro1 ride (screenshot 5), sp_a2_intro shaft arms (playtest 7). Animated
  props draw as rigid per-bone triangles (models.rs split_by_bone); soft
  weighted meshes tear. Check the weights, then skin them.

### T-116 intro1 room details  (P3, TODO)
- The window is a grey block, should be blinds (screenshot 1); the TV
  screen sits forward and up from the TV; odd outside lighting on the ride.

### T-117 Blocky shadows  (P3, TODO)
- Shadow-map resolution and filtering (screenshot 2).

### T-115 intro1: no wake/sleep animation on the bed  (P3, TODO)

### T-153 GLaDOS wake-up  (P3, TODO)
- White particles hide her; her arms float unattached when grabbing Chell
  and Wheatley; the portal gun didn't visibly drop; the player is made to
  look the wrong way (playtests 6-8).

### T-157 Transition into the sp_a2_intro fall not smooth  (P3, TODO)

### T-165 sp_a1_intro7: Wheatley's rail connector invisible  (P3, TODO)

### T-148 Portals behind glass  (P3, TODO)
- Drawn as if on top of the glass; behind broken glass not at all (section 1).

### T-140 Game-placed portals: no screen shake or sound  (P3, TODO)
- The first orange portal should shake the screen a little and play its sound.

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
