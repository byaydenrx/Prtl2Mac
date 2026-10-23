# Finished tasks

Moved out of TASKS.md (2026-10-23) to keep it short. Entries unchanged;
newest summary list at the bottom. Grep by task id.

### T-221 sp_a2_bridge_intro: invisible walls and textures, missing animated machinery from the first room  (P1, DONE 2026-10-23: owner confirmed all walls and textures are back; owner playtest 11: "unplayable")
- Owner: "lots of invisible textures and walls just from the beginning room,
  the animations machinery is missing walls are missing ... you can crouch
  past the hallway animation and continue". He finished it only because he
  knows the game. Bridges work and show through portals ("super ugly").
- First check (T-213 side effect): a598811 changed how MDL body parts are
  read and said "other props' non-default bodygroups were garbage before and
  are now hidden". Animated Aperture panels/arms may rely on bodygroups; diff
  an autopilot shot of this room on a build before a598811 vs now. Also check
  the models that fail to load (T-214) and fallback shaders.
- Likely shares a cause with T-222/T-223 (moving panels drawn without their
  surface) and bridge_the_gap's missing textures: fix the cause once.
- Autorun 2026-10-23 04:23 (autopilot only, build a8fda9c): not reproduced
  yet. Arrival area (elevator at source -736,736; stairs going -y, frames,
  video screens) renders complete from `setpos -736 560 -127`. Ceiling
  repair arms `ceiling_panel_repair1-roboarmm_0[1-5]bendy01` (191..319,
  -416..-480, 384) and their parented func_brush panels draw. The likely
  "hallway animation" is the floor-panel instance rows
  `row_[12]_panel_[2-5]-floor_panel_01_prop` (arm64x64_interior_rusty.mdl at
  -704/-768, -480..-672, -64) with func_brush `..._floor_panel_01_brush`
  (collider present, prop value 0 = not yet raised), raised by relays
  `row_*_panel_*-floor_panel_up`. Also `wall_panel_extend_model`
  (telescope_arm_128, 400,-768,64). `setpos -736 -330 -63` lands in the void,
  so use `goto`/walk from the stairs. Next step: walk up the stairs from the
  elevator (or `input row_1_panel_3-floor_panel_up Trigger`) and shoot the
  panel rows before/after they rise; compare with Windows (scripts/winref.sh)
  at the same spot; then check what fires floor_panel_up (ents outputs).
- Autorun 2026-10-23 04:37 (autopilot only, build a8fda9c): REPRODUCED.
  Walk from `setpos -736 560 -127 10 -90` (`walk 6`): door_52 opens,
  floor_up_relay -> row_1/row_2_up_relay -> each `floor_panel_up` plays
  `64_out_straight` on the arm prop. Arms and frames rise but the floor
  surfaces are missing (the hallway looks like open framework). Cause:
  the parented `..._floor_panel_01_brush` (func_brush *64, top face
  metal/black_floor_metal_001c) starts with its top at source z 0 (floor
  level) and after the rise `info` puts it at z 64 (origin -45 -> 19): it
  ends 64 units too high, i.e. at eye height, seen edge-on = invisible
  walls; the player is stopped at y -432 by them. Map setup per panel:
  prop keyvalue `defaultanim 64_idle`; logic_auto onmapspawn: prop
  `SetDefaultAnimation bindpose`, brush `SetParentAttachmentMaintainOffset
  panel_attach` (both delay 0). So the maintained offset must be measured
  against the attachment in the prop's spawn pose (64_idle = extended, the
  brush's Hammer position), after which bindpose retracts it (brush hidden
  below the floor) and 64_out_straight brings it back to z 0. We seem to
  measure the offset against bindpose/rest (or apply bindpose first), so
  the brush ends +64. Next step: find SetParentAttachmentMaintainOffset in
  crates/p2_game (grep maintainoffset) and check which pose the attachment
  is read from at spawn; make it the current sequence's (64_idle) pose;
  check bindpose actually plays after 64_idle ends (brush below floor
  before the rise). `animinfo` panicked "parse" on
  models/anim_wp/room_transform/arm64x64_interior_rusty.mdl (path given as
  framework/, retry with room_transform/). Likely the same bug for T-222
  (sphere_peek panels) and T-223 (ricochet platform): check after the fix.
  - Autorun 2026-10-23 04:48 FIXED in 18676be (autopilot only, build
    0.1.0+18676be.20261023T0847Z). Cause: on the first frame
    `anim_inputs` stored the map-spawn `SetDefaultAnimation bindpose` before
    `start_default_anims` picked the spawn sequence, so the arms spawned in
    bindpose instead of their keyvalue `DefaultAnim` (64_idle) and
    SetParentAttachmentMaintainOffset measured the brush offset against the
    folded pose. Fix (anim_props.rs `initial_seq`): a prop spawns in its
    DefaultAnim keyvalue, as in Source; SetDefaultAnimation only sets what
    follows. Evidence: row_1_panel_3 brush top before the rise z -55 (hidden
    below the floor), after z 0 (floor level; was +64); the walk from the
    elevator now passes the hallway (was stopped at y -432); p2_game tests
    pass; sphere_peek/ricochet load without errors. Owner: replay
    bridge_intro from the start (hallway floor rises and is walkable, no
    invisible walls); T-222/T-223 not checked yet against this fix.

### T-216 sp_a2_catapult_intro: the faith plate throws the cube too low; blocks chapter 2  (P1, DONE 2026-10-23: owner finished catapult_intro)
- Owner (playtest 10): catching works, "the cube doesn't go high enough up".
- Cause: props had avian `LinearDamping(0.05)`, which bled ~6% of the cube's
  speed per second of flight. The launch itself was right (850 u/s straight
  up, catapult2c). Windows Portal 2 reference (netcon + vscript printing the
  cube's z every 50 ms, docs/STATUS.md): the cube peaks at z 190 on the first
  throw, 204 after; ours peaked at 165-171, level with the flying player's
  feet (feet 152 crossing the plate on the 2a flight, same in both games)
  instead of his eyes. Fix (props.rs): no linear damping on props.
- Evidence: peaks old 165/171 -> new 188, 196-200 (Windows 190, 204); bounce
  period 2.82 s (Windows 2.81 s); autopilot ride on plate 2a + use at the
  apex catches the cube and carries it to the far plate. Regression: p2_game
  tests 202 pass; check_faith_plates (catapult_intro, trust_fling,
  pit_flings) same as before within sampling; 20 maps (chapter 1-2, pit_flings,
  fizzler_intro, sp_a3_jump_intro, sp_a4_intro, sp_a4_tb_catch) load with no
  panics and every named cube at rest from 4 s to 9 s.
- Replay: sp_a2_catapult_intro: ride the far plate across the room and grab
  the cube at the top of its bounce over the middle plate; it should be at
  eye height. Then finish the map.

### T-130 sp_a1_intro6: the resting cube didn't fall through a portal opened under it  (P1, DONE 2026-10-22: owner replayed and verified intro6)
- Owner playtest 8 (section 4) and 2026-10-22: the cube sits in a pit under
  a glass box; the only shot is straight down through the round hole on top.
  The portal opens under the cube, but the cube doesn't go through it.
- Cause: props.rs `PortalHooks::modify_contacts` (drops a prop's contact with
  the wall behind a linked portal) was never called. avian 0.7 sets a
  collider proxy's MODIFY_CONTACTS flag only when the proxy is created;
  `hook_prop_colliders` inserted `ActiveCollisionHooks` a step after
  `insert_prop_body` re-inserted the colliders, and avian's insert observer
  then updates only FILTER_PAIRS. Cubes that arrive later still worked
  (new pairs go through `filter_pairs`); a cube resting since map load kept
  its floor contact. Fix: the hooks go in with the re-inserted collider.
- Not the cause: the shot passes the cube (dynamic bodies are skipped); the
  cube is not in the placement probes. Diagonal shots onto the 128 u panel
  fail DoesNotFit, but the owner confirmed the hole above is the only shot.
- Evidence (autopilot, `2.5 input room_1_fling_portal_activate_rl trigger;
  4 setpos 256 192 110 89 90; 4.1 fire blue`, both portals linked): before,
  the cube stays at (256,192,18) for 6 s, contact with floor brush 927 still
  touching while it is in `PassingThrough`; after, 0.5 s later it is at
  (-346,0,366) out of the orange wall portal, then rests at (-187,19,18).
  `cargo test --release -p p2_game` 202 passed.
- Left: a full route (scripts/routes/sp_a1_intro6.txt). Owner chose to
  replay intro6 themselves instead (2026-10-22). Layout notes for a route:
  fling pit floor (-96..96,-128..64) at z=-384, orange portal at
  (-386,0,383), upper level z=128 with button_1 at (352,-160); the cube
  lands on the z=0 ledge west of the pit.

### T-215 Windows reference capture tool (CrossOver)  (P1, DONE 2026-10-23: owner confirmed the CrossOver reference tool works)
- Done 2026-10-21: `scripts/winref.sh` (usage in WORKFLOW.md). Cause of the
  old loading-screen shots: Portal 2 rejects `wait` ("Unknown command")
  until a server runs, so the cfg ran straight through during the load.
  The script uses -netconport (random -netconpassword) instead and sends
  commands once the console prints "Redownloading all lightmaps".
  Autopilot-checked on sp_a2_laser_intro: load in 25-33 s, post-load shots
  with --pos/--ang applied, HUD/notify hidden, Steam shut down, no stray
  screenshots. Elevator maps need --settle ~25 (the player rides in first).
- First finding (not yet a task fix): at the laser_intro start our view is
  much darker than Windows' (walls, floor; T-202/T-203 territory).

### T-198 Aerial faith plates: physics wrong (chapter 2)  (P1, DONE 2026-10-23: owner confirmed (threshold-check detail still open in the entry))
- Owner 2026-10-21: "the fling things aerial plates ... have something wrong
  with the physics". Cause (server.dll CTriggerCatapult, Ghidra 2026-10-21):
  our launch-by-target used the horizontal distance for the flight time and
  aimed the player's feet at the target; Source uses the straight-line 3D
  distance / speed (v = d/t + up g t/2, sv_gravity 600) and aims the player's
  feet 32 u below the target. useExactVelocity (6 plates: trust_fling 4,
  bridge_the_gap 2) was ignored: Source keeps the exact speed and solves the
  angle (exactVelocityChoiceType 1 high, 2 low, 0 the arc nearer the incoming
  velocity; no solution -> the non-exact launch). A straight-up directional
  plate throws the player at 1.5x playerSpeed. Air control is suppressed after
  a launch by target for AirCtrlSupressionTime (<= 0 -> 0.25 s), and after a
  directional launch only with DirectionSuppressAirControl; we had none.
- Fix b55fc08: catapult_velocity + CatapultAim (triggers.rs), new
  p2_core::AirControlSuppressed ticked down by the move (no air wish while > 0).
  Unit tests: flight time, -32 u player aim, exact arcs/choice/fallback,
  straight-up 1.5x.
- Evidence (autopilot only): check_faith_plates sweep (catapult_intro,
  trust_fling) closest approach before -> after: 6->11 (feet now 32 u under
  the target, as Source), 51->40, 15->12, 60->38, 24->12, 58->52, 26->17,
  21->30; catapult_intro launch 602 -> 586 u/s up, mid-flight shot normal.
  Not compared against Windows Portal 2 yet (scripts/winref.sh now works, T-215).
- Not done: Source's threshold check (usethresholdcheck) compares the
  incoming speed along the launch direction with playerSpeed/physicsSpeed
  (lower/upper fraction) and has entryAngleTolerance / onlyVelocityCheck; ours
  compares |v_in| / |launch|. trust_fling wallpad_2_catapult still misses its
  target by ~1650 u in the sweep (threshold plate, setpos entry, maybe fine).
  Props: Source adds a random angular impulse (applyAngularImpulse); skipped.
  scripts/check_faith_plates.py: REPO points at Portal2-rust-engine and the
  info regex misses the new "fwd/up" fields (fixed copy used in scratch).

### T-213 PotatOS never shows on the portal gun  (P2, DONE 2026-10-23: owner confirmed; her talking/eye animation and lighting on the gun and her not done (T-207))
- Owner: "glados as a potato on the portal gun doesn't show in chapters
  after you have her". Cause: v_portalgun.mdl and w_portalgun.mdl do carry
  her as bodygroup 1 ("potatos_vmodel", model 1), but vmdl reads Portal 2
  (v49) mstudiomodel_t with the wrong stride, so every model after a body
  part's first was dropped; and nothing tracked "PotatOS on the gun". Fix:
  models.rs reads body parts from the raw MDL (148/116-byte structs, as the
  eyeball reader does); body 0 stays in `meshes`, other choices go to
  `ModelData::body_alternates`. `PortalGun::potatos`: set by
  logic_playerproxy AddPotatosToPortalgun (sp_a3_transition01 pickup),
  cleared by RemovePotatosFromPortalgun (sp_a4_finale4 socket), set at map
  load from FIRST_MAP_WITH_POTATO_GUN (sp_a3_speed_ramp) through
  sp_a4_finale4, kept in saves. Both gun models show the bodygroup.
- Evidence (autopilot only): sp_a3_speed_ramp start, before build bare gun,
  after build PotatOS clamped on the prongs; sp_a3_transition01 bare until
  `input potatos_playerproxy addpotatostoportalgun`, then shown;
  sp_a2_laser_intro bare. Unit test potatos_rides_from_speed_ramp_through_finale4.
- Not done: her talking animation/eye light on the gun (v_portalgun has no
  separate sequences for it; check client.dll if the owner wants it).
  Side effect to watch: other props' non-default bodygroups were garbage
  before and are now hidden (Source draws body 0).

### T-002 Prove sp_a2_dual_lasers  (DONE 2026-10-23: owner played it through)
- Both catchers powered by the player's redirection, door opens, next map.

### T-003 Prove sp_a2_laser_over_goo  (DONE 2026-10-23: owner played it through)

### T-004 Prove sp_a2_catapult_intro  (DONE 2026-10-23: owner played it through)

### T-005 Prove sp_a2_trust_fling  (DONE 2026-10-23: owner played it through)

### T-006 Prove sp_a2_pit_flings  (DONE 2026-10-23: owner played it through)

### T-007 Prove sp_a2_fizzler_intro  (DONE 2026-10-23: owner played it through)
- Faith plates land on target (scripts/check_faith_plates.py); T-004..T-006 test the player's route.

## Done list (newest first; details in git log and docs/STATUS.md)
- 2026-10-23 Owner playtest 11: sp_a2_catapult_intro, trust_fling, pit_flings, fizzler_intro, sphere_peek, ricochet played through (T-002..T-007 done); T-216, T-198, T-213, T-215 confirmed; cubes no longer dark on buttons (T-145 part).
- 2026-10-22 Owner playtest 10 (build 0634859) confirmed: T-114 intro1 door waits; T-143 clock digits; T-175 fog outside the ride (sky still dark: later); T-147 no black screen; T-148 portals behind glass; T-131 intro3 floor/roof; T-150 intro3 smoke + splash and sp_a2_intro gun glow; T-139/T-118 no loud glass at intro4; T-120 voice-line timing and all elevators; T-149 screens and lights; T-133 Wheatley falls; T-153 gun drops (some particles remain); T-128 laser_stairs playable; dying and saving work; no softlocks seen.
- 2026-10-22 T-130 intro6 resting cube falls through a portal opened under it (0634859): owner verified; sp_a1_intro6 VERIFIED, chapter 1 fully verified pending the owner's full-chapter replay.
- 2026-10-21 T-146 intro1 exit door wiggles half stuck (de1d09b): owner confirmed ("i just saw the door fixed"). Oscillator logic_timers fire OnTimerHigh/Low in turn.
- 2026-10-21 Owner-confirmed (round 7, build 6e0efbf): T-175 sp_a1_intro1 ride: the big room outside the container has no fog in parts, black sky; T-140 Game-placed portals: no screen shake or sound; T-117 Blocky shadows (run-time fog "looks better", portal shake good, shadows good). T-153: gun drop and arms confirmed.
- 2026-10-21 Owner-confirmed (round 6, build ef0144c): T-169 sp_a2_laser_over_goo: GLaDOS never moves the walls, the way on is blocked; T-171 FPS drops, especially looking at portals; T-148 Portals behind glass; T-135 Turrets fall through the exit elevator; T-139 Audio; T-162 Menu animations, logo alignment, styling. Menus: "all menu items are fixed for now, buttons just don't function" (on hold, see "On hold: menus").
- 2026-10-21 Owner-confirmed after replaying build b8a2d76 (owner notes 2026-10-21 round 5, docs/PLAYTESTS.md): T-196 Screen goes black after every tile flip; T-195 In-game pause menu as Windows; T-197 Tile grid not even; T-182 New Game chapter list; T-186 Video settings; T-192 Menu scaling and display changes; T-176 Can't catch or grab objects in the air; T-170 Carrying regressed: pushed back at walls, can't push a cube up a wall, not smooth; T-163 sp_a1_wakeup pod: the player can jump out of the hatch; a stone-brick block appears there; T-113 Instructor hints vanish after a few seconds if not done; T-144 Missing props: intro1 portal room clipboard and movable clock; T-172 Voice lines start late; T-108 A chamber door had an invisible barrier, needed noclip; T-107 Cubes and floor buttons invisible. Also confirmed there: T-135's vac-tube turrets, T-173's fizzler shake, T-150's intro3 splash/dust, T-139's soundscapes (those entries stay open for their other parts). Details in git log.
- 2026-10-21 Upstream check (Stink-O/Portal2-rust-engine): only change since
  our base v0.1.0 is 18230b3, a relicense to GPL-3.0-only with no code.
  Nothing ported; NOTICE records that we stay on 0.1.0's MIT terms. See
  docs/STATUS.md "Upstream port (2026-10-21)".
- 2026-10-20 T-168 sp_a2_laser_over_goo box-button crash (98b74ad): owner
  confirmed. A Kill by name reached a hidden template prototype; prototypes
  now take no inputs.
- 2026-10-20 Owner-confirmed in playtest 9 (build 014ab6e): T-131 intro3
  floors/roofs; T-114 intro1 announcer; T-120 ride jitter, car sounds and
  thumps; T-133 Wheatley float and panel; T-134 pod turn; T-136 wakeup
  tube portal; T-137 portal spacing; T-102 cubes through portals; T-123
  security cameras; T-160 main menu items; T-161 menu movie/music; T-020
  intro1 ride; T-115 intro1 wake/sleep; T-157 fall transition; T-165
  intro7 rail connector; T-155 lasers; T-143 clock blink; T-166 intro5
  wall panel. T-125 closed: the owner no longer notices the turn-away
  delay. Write-ups moved to docs/STATUS.md.
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
