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
Now (owner playtest 11, playability first):
1. T-220 sp_a2_bridge_the_gap: game crashed after spawning a new cube; a cube on a bridge couldn't be picked up (P0 crash, FIXED, awaiting owner replay: crash a8fda9c; bridge pickup not reproduced)
2. T-221 sp_a2_bridge_intro: invisible walls and textures, missing animated machinery from the first room (unplayable; check T-222 and T-223 with it: likely one cause, see entry)
3. T-222 sp_a2_sphere_peek: platform you fall through; tampered faith plate; panels rotate too far
4. T-223 sp_a2_ricochet: the rotating platform under the cube is invisible and not solid
5. T-217 sp_a2_laser_stairs: floor missing after GLaDOS's floor move; Wheatley seen through the closed wall (floor fix waits on attempt/T-217's regression check)
6. T-145 Cubes go dark and jitter on buttons, every chapter (jitter still open)
7. T-218 Performance: 60-70 fps and stutter on an M4 Pro; portals cost a lot; v-sync unclear (owner: better; continue)
8. T-224 Cube physics feel: no tumbling in the air; plate and button hits unreliable
9. T-225 No dissolve ("fizzle") effect on cubes
10. T-226 Pedestal buttons (prop_button): no sound or press animation
11. T-173 Portal gun view model: hold beam, carry anims, fizzler shake
12. T-227 Zoom doesn't work
13. T-228 Chapter HUD icons don't look like Portal 2's
Then the engine gaps common to many maps, and gels:
14. T-212 sp_a3_00 (start of chapter 6): loud glass smash at load (BLOCKED: merge attempt/T-212 for the owner's replay)
15. T-203 Switchable lights frozen: lightmap styles other than 0 ignored (50 of 63 maps)
16. T-199 Particle sprite sheets not decoded: sheet particles draw the whole atlas
17. T-214 All-maps load sweep findings (2026-10-21)
18. T-201 UnlitTwoTexture (TV/monitor screens) and other model shaders on the fallback
19. T-206 Sounds not panned left/right
20. T-200 Particle operators the maps' effects use but p2_fx doesn't simulate
21. T-202 No bloom or colour correction (every map)
22. T-204 No env_cubemap reflections or dynamic lights on props
23. T-211 Gels: check each against Source
24. T-030 Gel chapters: missing buttons in gel rooms, black elevator, cube carry feel
Then the campaign sweeps (the owner's playtests go first):
25. T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug
26. T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)
27. T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)
28. T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)
29. T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)
30. T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)
31. T-015 Sweep chapter 9 (finale1..finale4)
Lower:
32. T-205 Portal gun edge cases
33. T-209 Portal transit only mirrors box-shaped geometry at the exit
34. T-207 Viewmodel gaps
35. T-208 Chell body gaps
36. T-210 Upstream minor gaps
37. T-090 Restart check per verified map
38. T-040 App icon and second-Mac first-run check
39. T-050 Verify merged cloud features in game: Wheatley look-at, gels, projtex, saves

## Active tasks (in priority order)

### T-220 sp_a2_bridge_the_gap: game crashed after spawning a new cube; a cube on a bridge couldn't be picked up  (P0 crash, FIXED, awaiting owner replay; owner playtest 11)
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

### T-221 sp_a2_bridge_intro: invisible walls and textures, missing animated machinery from the first room  (P1, IN_PROGRESS; owner playtest 11: "unplayable")
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

### T-222 sp_a2_sphere_peek: platform you fall through; tampered faith plate; panels rotate too far  (P1, TODO; playable; owner playtest 11)
- A platform you stand on sometimes lets you fall through; "its like not
  rendering the wall part itself just the mechanical" (see T-221 lead).
- The faith plate GLaDOS tampers with should launch a player already standing
  on it once she's done; it only launches after stepping off and on again
  (trigger_catapult on Enable / touch state: Source re-tests touching
  entities when a trigger is enabled).
- Two moving platforms move too far: they end horizontal instead of angled
  (animation end pose, pose parameters, or rotation clamping).

### T-223 sp_a2_ricochet: the rotating platform under the cube is invisible and not solid  (P1, TODO; playable; owner playtest 11)
- The platform surface (not the machinery) is missing, so the cube falls to
  the floor (still grabbable). Same family as T-221/T-222.

### T-217 sp_a2_laser_stairs: floor missing after GLaDOS's floor move; Wheatley seen through the closed wall  (P2, TODO; playable; owner playtest 10)
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
  then merge attempt/T-217 into main, make_app. Wheatley-through-wall and
  the floating texture: not started.

### T-145 Cubes go dark and jitter on buttons, every chapter  (P1 by owner, PARTLY FIXED: dark cubes and spikes owner-confirmed 2026-10-23; jitter open)
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
