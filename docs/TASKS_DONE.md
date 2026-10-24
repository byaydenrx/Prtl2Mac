# Finished tasks

Moved out of TASKS.md (2026-10-23) to keep it short. Entries unchanged;
newest summary list at the bottom. Grep by task id.

### T-239 sp_a2_bts1: Wheatley's breakout doesn't start  (P1, DONE 2026-10-24: owner playtest 14: "bts1 is verified and playable"; cube at the power cut moved to T-241)
- Owner (13b): the cutscene starts but "it teleports me rather than just
  turning the lights off ... i tp to the floor, i also ended up getting
  attached to wheatlys rail and it dropped me in the void and it does this
  everytime"; "wheatlys also pretty buggy during the sequence" (asked what).
- 2026-10-23 FIXED part 2 (triggers.rs, movers.rs). (a) power_loss_teleport
  has a landmark: Source keeps the player's offset from the landmark at the
  destination (the dark chamber is a copy), we put them on the destination
  point. Now landmark teleports keep place, motion and view (turned with
  uselandmarkangles), and move touching props too (spawnflag 8). Evidence:
  crossing at -9776 -900: before at -3712 -1152 0 (the destination), after
  at -4016 -900 (same place in the copy). (b) The elevator lock (movers.rs
  `ride`) took any player within 2.5 m across and 5 m below a train that
  starts moving; Wheatley's rail train runs 192 u over the floor, so a
  player under its start was locked to it and carried along the rail off
  the edge. Now it never takes someone standing on the map's own floor.
  Evidence (player at -3200 -1680 0 when @sphere_start_moving_relay fires):
  before carried to -2949 and killed falling; after stays put. Regression:
  arrival elevator (laser_vs_turret), departure elevator ride
  (column_blocker), sp_a1_intro1 container, bts5 identical to the old build.
  Departure-elevator player teleports have no target (script-set) and are
  unchanged; sp_a3_jump_intro's startup_lighting_teleport is the same kind
  of landmark swap and now keeps place too.
- Owner (13c, build 4e0f7f0): "im still getting picked up by wheatlys rail".
- 2026-10-24 FIXED part 3 (movers.rs). The lock also took a player in the
  air (jumping, dropping down) when the train set off, and it re-arms each
  time Wheatley starts moving again; his walkway (x -2804) runs 92 u from
  the rail, inside the lock's 98 u. Now Passable trains (spawnflag 8: not
  solid, carry nothing in Source) never take the player, by the lock or
  the inside-the-car check. Evidence (player held in the air at
  -3188 -1708 40 as @sphere_start_moving_relay fires, t 21.8): 4e0f7f0
  carried to -2937 and killed falling; new stays and lands. Elevators are
  all solid trains: arrival (laser_vs_turret) and departure (column_blocker)
  rides identical. sp_a3_00 changed: the old build locked the player at
  spawn to potatos_train (passable, 22 u away) and carried them up to
  z 1054 and down to -12308; the owner's Windows game keeps the player at
  0 -4096 0 for 36 s+, as the new build does. Not checked yet: bts5/bts6
  pod rides and the finale tube trains (passable; bts5 is a view-proxy
  tube ride).
- Owner: "at sp_a2_bts1 the wheatly busting you out cutscene or whatever
  doesnt register or start so not playable past this point".
- 2026-10-23 FIXED (triggers.rs). The breakout trigger
  (jailbreak_chamber_lit-jailbreak_trigger, a slab at y -912) starts
  disabled. A trigger_multiple at the dropper mouth (model *96, z 544..576,
  cube filter) enables it when the dropped cube falls through, and one in
  the pit (*94) disables it again if the cube falls in: the cube has to be
  caught on the light bridge. Both have spawnflags 4161 (clients +
  "everything"); our triggers only let props touch flag 8, so the cube
  never armed it. Flag 64 now counts the player, props and NPCs. Evidence
  (autopilot: cube moved through *96 and onto the bridge, player into the
  slab): before the trigger stays disabled and nothing fires; after it is
  enabled and @jailbreak_begin_logic, @sphere_lookat_player_relay and
  jailbreak_peak_logic fire. Other maps with flag-64 triggers (bts3, bts4,
  core, laser_over_goo, jump_intro, speed_ramp, tb_wall_button,
  stop_the_box, finale2, finale3) load and run without a crash or death
  in both builds. The rest of the breakout sequence is not checked yet.


### T-233 sp_a2_column_blocker: lasers and turret sight don't pass the grate  (P1, DONE 2026-10-23: owner playtest 13b: "sp_a2_column_blocker is fixed tho and verified and playable")
- 2026-10-23 FIXED with T-238 (one cause). Grate colliders (CONTENTS_GRATE,
  no SOLID/WINDOW; brushes and `$contents grate` models) are on their own
  collision layer (plugin.rs GRATE_LAYER); lasers and turret sight mask it
  out, bullets too. Evidence (autopilot, player at -192 600 0 across the
  pit from the turret behind grate brush 600 at y 1027): before the
  turret stays Idle; after Deploying -> Firing and the player dies in
  ~2.5 s. Lasers use the same mask, shown through glass in T-238.
- Owner: "the laser for the turret doesnt go through the grate in that level
  and the turret cant see you through or shoot you through the grate, also
  the laser doesnt kill the turret through the grate". Grates
  (CONTENTS_GRATE, T-130) stop the player but should pass lasers, turret
  sight lines and bullets. Also: "this level is nit completeable because of
  the lasers being weird and angling, it angles itself above the next
  reflection box" (build 1894bb5, before the T-231 fix: replay first).


### T-236 sp_a2_column_blocker: invisible barrier at the departure elevator  (P1, DONE 2026-10-23: owner verified column_blocker playable in playtest 13b; never reproduced)
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


### T-230 sp_a2_laser_relays: a laser that has gone through a portal doesn't power the relays  (P1, DONE 2026-10-23: owner playtest 13: "sp_a2_laser_relays is now verified and playable")
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


### T-231 Lasers aim wrong: a laser angles up or down instead of going straight  (P2, DONE 2026-10-23: owner verified laser_chaining, triple_laser and laser_relays in playtest 13)
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


### T-220 sp_a2_bridge_the_gap: game crashed after spawning a new cube; a cube on a bridge couldn't be picked up  (P0, DONE 2026-10-23: owner played bridge_the_gap through on c38ce98; Wheatley and the bird moved to T-232)
- Owner: "i couldnt pick up one of the cubes that landed on a bridge about
  half way into beating this level so i spawned another and it crashed my
  game". Reproduce both: a cube resting on a hard light bridge (pickup ray /
  use cone vs the bridge's collider, or the cube asleep on a kinematic
  body), and the crash when the dropper replaces it (compare T-168, a Kill
  reaching a template prototype; panic message in the terminal log).
- Also seen in this map (P2): Wheatley glitches around; the bird that flies
  through is visible when it shouldn't be (or looks wrong); some missing
  textures (fewer than bridge_intro, likely the same cause as T-221).
- 2026-10-23 autorun: CRASH REPRODUCED (autopilot only). Back up the owner's
  saves first (cp -Rp "~/Library/Application Support/Portal2Rust/saves",
  save.txt, progress.cfg), then
  `P2_SCRIPT="1 con load autosave; 9 input cube_dropper-cube_dropper_relay trigger; 21 quit" ./target/release/portal2 sp_a2_bridge_the_gap`
  with the owner's 02:16 autosave: the dropped cube falls into
  trigger_kill_boxes and the game panics in avian
  `update_solver_body_aabbs`: "no element at index 20 in
  StableVec::index_mut" (avian3d-0.7.0 data_structures/stable_vec.rs:334).
  Exit 101. Same steps on a fresh map load, or after `con map` reload, do
  NOT crash, and re-dropping 3 times in a row is fine. So it's specific to
  a save load: the restored cube (#1281, restored from the save rather
  than spawned by the template) probably has a stale collider-tree proxy
  key or a body restored without avian re-registering it, and killing it
  (or the dropper's respawn) hits a freed slot.
  Not reproduced: the bridge pickup (cube placed on the projector3 bridge
  with propto, goto+lookat+use picks it up fine).
  Next step: read crates/p2_game/src/saves.rs prop restore (how props get
  re-created or moved on load) and avian's ColliderTreeProxyKey handling;
  check whether `load` despawns and respawns physics entities in the same
  frame, or reinserts Collider/RigidBody on existing entities. A minimal
  repro: load the save, then `input cube_dropper-cube_dropper_box kill`.
  - 2026-10-23 04:22 autorun: CRASH FIXED a8fda9c (autopilot only), build
    0.1.0+a8fda9c.20261023T0821Z. Cause: templates::spawn_copy clones the
    prototype with linked_cloning, which copied avian's
    ColliderTreeProxyKey. After a save load the cube prototype's (disabled)
    collider still held a live key; when a restored copy's collider was
    inserted, avian's add_to_tree_on "removed the old proxy", i.e. freed the
    prototype's proxy, and the prototype kept a stale key whose slot the
    copy reused. Killing that copy freed it again and the next drop's AABB
    update hit the freed slot. Found with a temporary per-frame check of
    every ColliderTreeProxyKey against ColliderTrees (stale key right
    after "save restored"). Fix: deny ColliderTreeProxyKey when cloning, so
    each copy starts with the placeholder key and gets its own proxy.
    Evidence: the repro above exited 101 before, 0 after, with no stale keys;
    after load, 3 drops in a row fine; fresh map, 3 drops fine; 202 unit
    tests pass; owner's saves left unchanged.
    Not explained: why the prototype's key was live after the load (the
    fix makes it harmless). Also seen: after this save load the new cube
    falls to z~641 and is killed (as the old one was), while on a fresh map
    it stops near z~1430; may be the save's world state (bridge off).
    Owner replay: load the bridge_the_gap autosave, drop a new cube, finish
    the map; also try picking up a cube lying on a bridge (not reproduced:
    propto onto the projector3 bridge + use picked it up).

- 2026-10-23 evening, owner: the a8fda9c build "removed the entire cube
  from the dropper", so the map can't be finished (softlock). Cause: Bevy's
  clone `deny` also denies every component that requires the denied one,
  and avian makes Collider require ColliderTreeProxyKey, so denying the key
  stripped the Collider from every template copy (all cube droppers, every
  map). Reproduced (fresh bridge_the_gap, P2_PROPTRACE): copies fell from
  z 1904 through the dropper and floor in free fall and were killed below.
  Fix (templates.rs): deny the key inside `without_required_by_components`,
  so the copy keeps its Collider and gets the placeholder key. After: the
  copy lands in the dropper (z 1430) with a live collider; the save-load
  crash repro (autosave, two drops) exits 0, no panic, saves unchanged;
  fizzler_intro, laser_stairs, catapult_intro, ricochet and jump_intro
  droppers all leave live, solid copies on floors; 202 tests pass.
  Screenshots came out black for every build this session (capture
  problem, old build too), so evidence is from traces and entity state.

### T-222 sp_a2_sphere_peek: platform you fall through; tampered faith plate; panels rotate too far  (P1, DONE 2026-10-23: owner: "the issues on sp_a2_sphere_peek are fixed")
- A platform you stand on sometimes lets you fall through; "its like not
  rendering the wall part itself just the mechanical" (see T-221 lead).
- The faith plate GLaDOS tampers with should launch a player already standing
  on it once she's done; it only launches after stepping off and on again
  (trigger_catapult on Enable / touch state: Source re-tests touching
  entities when a trigger is enabled).
- Two moving platforms move too far: they end horizontal instead of angled
  (animation end pose, pose parameters, or rotation clamping).
- 2026-10-23 05:00 autorun, faith plate NOT REPRODUCED (autopilot, build
  18676be): the plate is catapult_up (disabled by disable_catapult_rl on each
  sphere look, re-enabled by GLaDOS's FailureOne03/FailureTwo03 scene
  `fires` -> @trigger_this_to_fix_catapult); after FailureThree,
  @trigger_this_to_fix_ceiling -> catapult_switch_relay (+9 s) swaps to
  catapult2_up. Standing still on the plate, every path launched at once:
  direct inputs (fix relay, switch relay) and the real flow (goto plate,
  lookat @sphere x3: fix fired twice, launched twice, then ceiling, then
  catapult2_up launched). The trigger code already re-tests on Enable (a
  disabled trigger drops out of `touching`, so enable = new StartTouch).
  Owner: which plate/moment exactly (before or after the ceiling lowers)?
  Maybe the player stood off the 8-unit trigger (bounds y -4.47..-4.27 m,
  feet -4.67 when centred) e.g. on the plate rim.
- Seen while testing (not the owner's report, check against Windows with
  scripts/winref.sh): after the ceiling closes, catapult2_up (target
  helper_catapult2_target straight above, 500 u/s) throws the player into
  ceiling_panel (apex feet ~9.3 m) and back onto the plate every ~1.9 s,
  forever. Next step: confirm what Windows does there (the lowered ceiling
  may be meant to stop you, or catapult2 should fling at 45 deg), then the
  fall-through platform and the panel end pose items.
- 2026-10-23 05:06 autorun, panel end pose NOT REPRODUCED (autopilot, build
  18676be, so with the T-221 fix): the two moving platforms are arm_panel_01
  / arm_panel_02 (func_brush, SetParentAttachmentMaintainOffset to
  robo_spherepeek_meea_01 / meeb_01; DefaultAnim spherepeek_0N_idleend,
  logic_auto sets default _idle; relays arm_spherepeek_01/02 play
  spherepeek_0N then default _idleend). Before the relay the panel sits
  vertical in the wall (rotated 45 deg from its brush); after
  `input arm_spherepeek_01 trigger` + 10 s both panels end with identity
  rotation at their compiled brush origins (-800 1841 145 / -751 2400 145),
  i.e. the Hammer-authored pose, and a side shot shows panel 01 sticking out
  angled ~45 deg, not horizontal. Most likely the owner's playtest-11 report
  was the pre-18676be bug (offsets measured in bindpose). Owner: replay; if
  still wrong, say which platform. Left: Windows check of the catapult2_up
  bounce, the fall-through platform.
- 2026-10-23 05:18 autorun, fall-through platform NOT REPRODUCED (autopilot,
  build 18676be): checked every arm-carried brush you can stand on.
  catapult_block1..4_floor (telescope arms, catapult_block_relay from the
  entry trigger_once: rise, ClearParent at 2.75 s, arms killed at 3 s): all
  four floors survive, drawn and solid at z -191 next to the
  catapult_block_floor door. floor_repair_1 roboarmk_01/03/05/07 panels
  (2-unit func_brushes, SetParentAttachmentMaintainOffset panel_attach):
  held up at spawn, after floor_replace_relay all four land at their
  compiled pose (z -193, y 1536), drawn and solid; a shot shows a normal
  tiled floor. landing_01 clipsforbigplatform/tilesonbigplatform are a
  vertical wall (not a floor) and stay put. Most likely the same pre-18676be
  bindpose-offset bug as T-221 (panels ending in the wrong place: "only the
  mechanical part" drawn). Owner: replay on the current build; if you fall
  through again, say where (near the faith plates, the floor GLaDOS
  repairs, or elsewhere).
- Status: all three owner items not reproduced on 18676be -> BLOCKED (needs
  owner replay). Optional, not the owner's report: Windows check of the
  catapult2_up ceiling bounce (scripts/winref.sh).

### T-145 Cubes go dark and jitter on buttons, every chapter  (P1 by owner, DONE 2026-10-23: owner: "cube jittering is fixed"; dark cubes and spikes confirmed earlier)
- 2026-10-23 evening, jitter FIXED. Cause: avian's contact generation
  (parry 0.27, convex_polyhedron.rs local_support_feature) clips only the
  first four vertices of a face. The floor button pad (portal_button and
  both damaged variants, .phy piece 9 on the top bone) is a 14-sided prism,
  so a cube on it found one contact point on a sliver at the pad's edge,
  sank ~2 u into the rest (centre z -35.9 vs Windows -33.47) and rocked on
  that point forever (15 Hz wobble, sliding ~10 u/s). Found with
  P2_PROPTRACE contacts (one point, pen 2.1 u, 10x the floor's impulse;
  the pad collider itself was still and correctly placed, ray-mapped) and a
  scratch probe of avian contact_manifolds (cube vs pad hull: always 1
  point; vs a cuboid: 4-6). Fix (source_assets collision.rs contact_hulls,
  used for every model .phy hull in plugin.rs spawn_prop_collider): a hull
  with a face of more than 4 corners is cut into exact convex pieces along
  face diagonals (cut choice scored by the biggest face it leaves, at most
  15 cuts; corners in the middle of a straight edge dropped first). Pad: 8
  pieces, rim pieces 2-4, the cube's box untouched.
  Evidence (same binary, split on/off): intro2 button_1, six drops: before
  3 of 6 still rocking/sliding 3-6 s after landing (max 16.6 u/s, 1.35
  rad/s), after all 6 still (<= 0.3 u/s, 0.01 rad/s), flat on the tilted
  pad; laser_stairs exit button: rattling drop (9.5 u/s) now still.
  Regression: player walks over button_1 straight and diagonally identical
  with/without; fps and load time unchanged (bts4 329 hulls split, intro2
  283; 120 fps both); 16 maps ch1-9 load clean; unit test
  many_sided_hulls_are_cut_into_quad_faced_pieces (pieces fill the prism
  exactly); tests 202 + 74 pass.
  Not changed: brush hulls (world geometry) go through the same parry code
  and a brush face with more than 4 corners would act the same; not
  measured. Possibly related: T-224's unreliable plate and button hits.
  Off-centre drops rest leaning on the rim (tilt up to ~7 deg): that's the
  rim's geometry, not compared against Windows.
- 2026-10-23 dark cubes, cause: a moving prop's vertex light was baked once
  where it spawned and kept forever. Dropper and template cubes spawn in
  the dropper's tube, where the ambient light is 0.000, so they were black
  everywhere; cubes spawned in dark corners (intro2's "box", 0.017) stayed
  dark. Not the lit skin: swapping which skin is the hidden variant gave the
  same dark cube (the earlier hidden-variant lead was wrong). Source lights
  moving models from where they are now. Fix (source_assets plugin.rs
  RelightOnMove/relight_moved_props): cubes, physics props, turrets and
  paint bombs redo their vertex light (same maths as the spawn) after
  moving 8 u or turning 0.15 rad, at most every 0.1 s; template copies keep
  it (components are Clone). valve_render rewrites a relit draw's vertex
  buffer in place instead of adding buffers.
- Evidence: fizzler_intro and laser_stairs dropper cubes black -> lit;
  catapult_intro dropper cube on the floor 84 -> 110 (floor 34 both);
  intro5/intro6 cubes slightly brighter. 20-map load check clean, fps
  unchanged, tests 202 + 72 pass.
- 2026-10-23 spikes, cause: animated models are drawn as rigid per-bone
  parts and each triangle went to one bone. portal_button(_damaged01/02)
  has 80 triangles with corners pinned to the base and to the top; pressed,
  the top drops 7.3 u (13.64 -> 6.35) and those triangles stayed up with
  the base, poking through the red pad. Fix (models.rs split_by_bone,
  plugin.rs StretchSurface): such triangles go to a stretch part whose
  vertices each follow their own bone and are moved when bone parts move,
  as skinning stretches them. Also used by the faith plate, Wheatley,
  GLaDOS pieces, a chamber door, the box dropper, an arm and intro1's
  container-ride debris (possibly T-164's "spiky debris"; not checked).
  Evidence: intro2 button pressed, spikes before, none after; released pose
  normal; plate arm mid-launch, Wheatley and GLaDOS shots unchanged; unit
  test pinned_mixed_triangles_stretch.
- 2026-10-23 jitter, reproduced (autopilot, intro2, `propto box` dropped
  from ~70 u onto button_1): Windows' cube rests perfectly still (z -33.47,
  vscript 0.25 s samples). Ours, depending on where it lands: (a) rocks
  ~5-7 deg for 1-3 s after landing, or (b) keeps rattling indefinitely, up
  vector flipping every frame and slowly turning (spin ~1.1 rad/s sustained),
  e.g. drop at (-610,284,35) resting near (-598,286). Same cube on plain
  floor: perfectly still. Ruled out: the press (no pad animation: still
  rattles), SweptCcd, restitution, damping (a settle damping of 6 once
  nearly still for 0.2 s stops (a) in 3 of 5 drops but not (b); not kept).
  Cause is in the contact between the cube box and the button's compound
  of ten small convex .phy pieces (rim pieces 0..9.8 u high, pad -3.8..0).
  Next: draw/inspect those hulls where the cube's corners rest, try avian
  contact settings (speculative margin, contact softness, substeps) on this
  case, and check whether VPhysics merges or simplifies those pieces.
- Left: under intro2's button lamp the relit cube's top is a saturated
  pinkish white where Windows shows grey, and its sides stay darker than
  Windows: our ambient for models (leaf-average samples, world-light term)
  isn't calibrated against Source. Inverse-square sample weighting was tried
  and overshot ~10x; not kept.
- Owner playtest 10 (2026-10-22): "cubes still show as black and are
  jittery/buggy moving on buttons, still playable but a necessary now fix,
  global, not on all buttons". Find which buttons/cubes (class, skin, model)
  show it and which don't; the dark lit skin and the settle jitter may be
  separate causes. Leads already in this entry.
- 2026-10-20 19:50 autorun: dark lit cube REPRODUCED (autopilot sp_a1_intro2,
  `propto box -607 288 -20` onto button_1-button, dist build 9f1ed5a): on the
  pressed button the cube body renders dark grey (yellow ring fine); off the
  button it is light grey. P2_HIDE_MAT=metal_box_skin002 hides the whole cube,
  so only the lit-skin draw shows, and it is the one that is dark. Forcing lit
  skin 4 instead of 2 is just as dark, so it is not the texture. Ruled out:
  texture brightness (skin000/002/004 avg ~0.6, decoded with `ls --vtf`),
  shader combo (both `vs 0 ps 36`), lighting (one ambient-cube closure, baked
  into both skins' vertices at spawn), sRGB cache (both srgb=true), skin
  switch (apply_model_skins only flips Visibility). Not yet checked: the
  render side for draws under a SkinVariant entity spawned Hidden
  (valve_render prepare/upload of draws first seen while hidden, e.g. light
  stream or constants), and whether the dark look is the variant draw being
  drawn without its static-light stream. Next step: spawn the cube with
  base skin 2 (no variant) and compare; if bright, the bug is the
  hidden-at-spawn variant path.
- Also found: cube_lit_skin's table was wrong for non-standard cubes:
  metal_box skin textures are 0 standard, 1 companion, 2 standard lit,
  3 rusted, 4 companion lit, 5 rusted lit (we lit companion 1 as 3 = rusted
  blue, rusted 3 not at all). Fix cc8d8e0 (was attempt/T-145) MERGED
  2026-10-21 at the owner's request; not seen in game (no companion or
  rusted cube on a button was checked). The dark standard lit cube is NOT
  fixed by it.
- Spikes under the button and jitter on buttons: not looked at this run.
- Owner playtest 9 (2026-10-20, build 014ab6e): button press works, but "cubes turn black when they're activated and yellow" and "it adds weird spikes under the button almost like there's grass under it"; "buttons are still juttery/buggy when [cubes are] put on buttons".
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
