# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (WORKFLOW.md). Work the "Priority order" top to bottom; never P3/P4
while a P0/P1 is open. One entry per task: fold new reports into it. When a
task is fixed, give its cause and evidence in two or three lines, move it
to "Awaiting owner replay"; once the owner confirms, move it to Done. Long
investigation notes go in docs/STATUS.md; the owner's raw reports in
docs/PLAYTESTS.md (history only, everything there is tracked here).

How maps get proven (owner, 2026-10-22): the owner plays each chapter
himself and reports bugs; sessions fix bugs, they do not write scripts that
play whole levels. A bug fix is done when (1) the bug is reproduced, (2) a
targeted autopilot run or unit test shows it gone (before/after evidence),
and (3) nothing else broke: `cargo test --release -p p2_game` plus the crate
touched pass, and every map the change could affect still loads and plays
its first seconds normally (short autopilot run per map). Then the owner
replays it. VERIFIED (PROJECT_STATE.md) means the owner played the map.
T-002..T-015 ("prove/sweep" a map) mean: find what blocks the owner from
finishing it, fix that, and hand it to the owner.

Owner playtest sections (playtest 8): 0 intro1, 1 intro2, 2 intro3, 3 intro4,
4 intro5/intro6, 5 intro7 + sp_a1_wakeup + sp_a2_intro, 6 sp_a2_laser_intro,
7 sp_a2_laser_stairs.

## Priority order (2026-10-21, owner: whole game first, then the campaign)
Owner 2026-10-21: every map should load, with textures and animations
right, then fix the biggest problems common to many levels, each gel, then
the full campaign (chapters 2-9), done by the auto sessions. Audit sources:
docs/UPSTREAM_GAPS.md (upstream README gaps), T-214 (all-maps sweep), fxaudit.
ON HOLD (owner): menus (no menu code changes at all); screenshot tasks
T-173, T-174, T-147, T-149, T-116, T-152, T-151, T-154; T-164
skipped; T-120 done enough. Don't work on anything already fixed.
Awaiting the owner: T-112 GLaDOS floor, T-153 leftover particles, T-150
motion blur and gun glow, T-213 PotatOS on the gun, T-198 faith plates, T-215 winref.sh.
This list is the one source of what's next: the handoff's "Next action" names
its first open item, and resume.sh warns when they disagree.
Owner 2026-10-22 (playtest 10): playability first. The owner plays the
campaign himself; fix what stops him, then what he called "necessary now".
Owner's order (2026-10-22 23:20): T-216, T-218, T-145, T-217; everything
else (items 1-8 below and the "Later" list) waits until the owner says.
0a. T-216 sp_a2_catapult_intro: the plate throws the cube too low. FIXED, awaiting replay.
0b. T-218 performance. PARTLY FIXED (culling), awaiting replay; MSAA-at-4K and v-sync-off findings in its entry.
0c. T-145 cubes on buttons: dark cubes and spikes FIXED, awaiting replay; jitter open (cause narrowed, entry).
0d. T-217 sp_a2_laser_stairs: missing floor fixed on attempt/T-217 (regression check left); Wheatley through the wall and floating texture not started.
0e. Then wait for the owner's next playtest (chapter 2 from catapult_intro);
    his reports go before everything below.
Later, owner says not blocking (keep for the polish pass): intro1 sky
red-black (T-175 area) and spiky ride debris (T-164); intro1 window (T-116);
water looks poor in intro3/wakeup (T-151, T-112); fog-blue seen through walls
outside rooms on every map (T-112's "blue wall"); intro5 floating vine
(T-154); intro7 turn-away a couple of seconds slow (T-125, docs/STATUS.md);
wakeup leftover particles (T-153); water footsteps (T-219).
1. T-212 loud glass smash at sp_a3_00 load (owner report). BLOCKED: attempt/T-212.
2. T-213 PotatOS missing from the portal gun. FIXED a598811, awaiting replay.
2b. T-215 Windows reference capture tool. DONE (scripts/winref.sh), awaiting replay.
4. T-198 aerial faith plate physics. FIXED b55fc08, awaiting replay (threshold-check part open in its entry).
5. Common to many maps: T-203 switchable lights (50/63 maps), T-199
   particle sprite sheets (every map), T-214 DecalModulate (29 maps) and
   SolidEnergy (51 maps) shaders + the 4 models that fail to load, T-201
   monitors, T-206 sound panning, T-200 particle operators, T-202 bloom/colour
   correction, T-204 cubemaps.
6. T-211 each gel against Source (+ render_blobs).
7. Campaign in map order: T-002..T-008, T-010..T-015 (prove each map; every
   blocking engine bug becomes its own task first).
8. Lower: T-205 gun edge cases, T-209 transit geometry, T-207, T-208, T-210.
Then the Backlog (prove and sweep the campaign).

## On hold (entries in docs/TASKS_ON_HOLD.md)
Owner holds of 2026-10-21, moved out of this file unchanged. Do not work on
them until the owner lifts the hold; search that file by task id.
- Menus (no menu code changes at all): T-193 T-194 T-177 T-178 T-179 T-180
  T-181 T-183 T-184 T-185 T-187 T-188 T-189 T-190 T-191.
- Waiting for owner screenshots: T-173 T-174 T-147 T-149 T-116.
- Skip for now / wait for screenshots: T-164 T-152 T-151 T-154 T-120.

## Open: P0
(none)

## Open: P1

### T-145 Cubes go dark and jitter on buttons, every chapter  (P1 by owner, PARTLY FIXED: dark cubes c005fbc and button spikes bf5006a awaiting replay; jitter open, cause narrowed)
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

### T-219 No footstep or jump sounds in water (intro3)  (P3, TODO; owner playtest 10)
- Splash on the fall works; walking or jumping in the water is silent.

## Open: engine gaps (2026-10-21 audit: upstream READMEs + all-maps sweep; docs/UPSTREAM_GAPS.md)

### T-212 sp_a3_00 (start of chapter 6): loud glass smash at load  (P2, BLOCKED (needs owner); owner 2026-10-21)
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

### T-201 UnlitTwoTexture (TV/monitor screens) and other model shaders on the fallback  (P3, TODO; upstream gap + sweep)
- models/props_motel/tv_screen and other monitors use UnlitTwoTexture
  (two textures multiplied, often a scrolling scanline); we draw them on the
  fallback (22 materials over ~20 maps in the 2026-10-21 sweep, T-214).

### T-202 No bloom or colour correction (every map)  (P3, TODO; upstream gap)
- Portal 2 adds bloom (engine_post) and a colour-correction LUT
  (color_correction entities, .raw LUTs in materials/correction) on top of
  auto-exposure; we have auto-exposure only (exposure.rs). Bloom makes
  lights, screens, portals and the gun glow; the LUTs tint whole areas
  (e.g. old Aperture). Related to T-152's look but not blocked on its
  screenshots.

### T-203 Switchable lights frozen: lightmap styles other than 0 ignored (50 of 63 maps)  (P2, TODO; upstream gap)
- Lightmaps use style 0 only (geometry.rs; P2_ALLSTYLES sums every style as
  a debug view). Named `light`/`light_spot` entities (50 of 63 maps have
  some) switch with TurnOn/TurnOff/Toggle and their light lives in extra
  lightmap styles: chambers lighting up as you enter, power-ups (intro1),
  lights going out (Wheatley, chapter 3/6 old Aperture, finale). Keep each
  face's style lightmaps and a per-style brightness (style index from the
  light's `style` keyvalue, inputs toggle it; patterns for flickering
  styles), blend in the lightmap shader. Unit test with a two-style face.

### T-204 No env_cubemap reflections or dynamic lights on props  (P3, TODO; upstream gap)
- Materials with `$envmap env_cubemap` (glossy floors, metal, glass, the
  portal gun) get no reflection: the BSP's built cubemaps (pakfile
  materials/maps/<map>/c<x>_<y>_<z>.vtf, the cubemap lump) aren't loaded or
  assigned to the nearest surface. Props also ignore dynamic local lights
  (light_dynamic, named lights). Also: displacement lightmap UVs are an
  approximation. Overlaps T-152 (on hold); this part needs no screenshots.

### T-205 Portal gun edge cases  (P3, TODO; upstream gaps)
- Not done (p2_gun README): shooting with the eye behind/inside a portal
  (out of the far side); refusing a floor portal under the standing player;
  re-testing portal support every 0.1 s so a portal fizzles when its surface
  disappears (only listed movers do it now); mobile portals on moving
  panels; several linked portal pairs at once (unverified; single player
  normally has one gun pair plus map portals).

### T-206 Sounds not panned left/right  (P3, TODO; upstream gap)
- All sounds use distance gain only (p2_fx sounds, p2_game audio): no stereo
  position, so you can't hear where turrets, Wheatley or GLaDOS's panels
  are. Add Source-like panning (and the soundlevel falloff check) with a
  listener on the camera; through-portal sound is a later step.

### T-207 Viewmodel gaps  (P4, TODO; upstream gaps)
- Gun lit by a fixed neutral ambient instead of the room's light; no
  clipping/offset when close to a portal (portal_viewmodel_offset);
  cl_mouselook_roll_compensation not done.

### T-208 Chell body gaps  (P4, TODO; upstream gaps)
- No head look, no turn-in-place animation (legs snap past 45 deg), no foot
  IK, body casts no shadow. Seen through portals.

### T-209 Portal transit only mirrors box-shaped geometry at the exit  (P3, TODO; upstream gap)
- p2_transit copies only cuboid colliders as remote geometry; trimesh
  (displacements, props) near an exit portal isn't mirrored, so walking
  through a portal next to such geometry can snag or let you clip. Check
  with a portal on/near displacement terrain (old Aperture, chapter 3/6).

### T-210 Upstream minor gaps  (P4, TODO)
- Portal rim faint/thin and projectile stream dim next to Portal 2's; no
  sound/burst for shots that hit nothing (sky); kinematic bodies block
  shots; mouse sensitivity vs a real config (m_yaw 0.022 x 3); intermittent
  shutdown crash (p2_view); 2D skybox face orientation unverified.

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

## Open: P2

## Open: P3

## Open: P4

## Awaiting owner replay (fixed, autopilot evidence)

### T-218 Performance: 60-70 fps and stutter on an M4 Pro; portals cost a lot; v-sync unclear  (P2, PARTLY FIXED, awaiting owner replay; findings below)
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

### T-216 sp_a2_catapult_intro: the faith plate throws the cube too low; blocks chapter 2  (P1, FIXED, awaiting owner replay)
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

### T-215 Windows reference capture tool (CrossOver)  (P1, DONE, awaiting owner replay; tooling)
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


### T-198 Aerial faith plates: physics wrong (chapter 2)  (P1, FIXED b55fc08, awaiting owner replay)
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

### T-213 PotatOS never shows on the portal gun  (P2, FIXED a598811, awaiting owner replay)
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

### T-150 Particles missing  (P4, FIXED, awaiting owner replay: motion blur 7cc1096, gun glow through the wall 8d48dab; intro3 splash/dust owner-confirmed "for now")
- 2026-10-21 autorun, gun glow through the wall: FIXED 8d48dab. Cause: the
  gun's `info_particle_system` particle_gun_charge (electrical_arc_01,
  started by timer_gun_particles for 0.5 s every 1-3 s) has a child
  electrical_arc_01_cp0_glow drawn with particle_glow_05_add_15ob_noz
  (`$ignorez`, no depth test); Source hides it behind the world with the
  renderer's `Visibility Proxy Input Control Point Number` (an occlusion
  query at CP0), which p2_fx ignored. (The env_sprite spr_gun_glow is depth
  tested and was not it.) Now each frame a line of sight from the main
  camera to the proxy point (static/kinematic colliders; props, the player,
  player clips and grates don't block; the last max(radius, 2) units
  ignored) gives visibility 0/1, mapped through `Visibility input min/max`
  onto `Visibility Alpha Scale min/max` as particle alpha. Line of sight,
  not a partial pixel count; `Visibility Radius Scale` not done. Unit test
  visibility_proxy_fades_hidden_glows. Evidence (autopilot only): landing
  spot (-3232 568 -10792) `lookat portalgun`, 24 shots 0.2 s apart: dist
  7cc1096 shows the blue glow over the crushing arms in 2/24, 8d48dab in
  0/24; close up (`setpos -1125 449 -11076`, lookat portalgun) the arcs and
  glow still flash in both. First try was blocked by the invisible
  portalgun_playerclip brush, hence the clip/grate exclusion.

- 2026-10-21 autorun, motion blur/speed lines: FIXED (see commit "T-150:
  Source-style motion blur"). Cause: no motion blur pass existed
  (`mat_motion_blur_enabled` was a stored setting only). Windows Portal 2
  has no speed-line particle (client.dll has none); the streaks are its
  image-space `dev/motion_blur` pass. New `motion_blur.rs`: one blur vector
  per frame from the view's turn (half-frame shutter, teleports skipped) plus
  falling blur from `mat_motion_blur_falling_min/max` (8/20, read as m/s):
  vertical when looking level, radial streaks from the screen centre when
  looking down/up; max 4% of the screen; crosshair and gun stay sharp
  (their own cameras); Options' Motion Blur turns it off. Our reading of
  the convars/units, not disassembled. Evidence (autopilot only):
  sp_a2_catapult_intro `setpos -400 -800 440 85 0`, shot 1.5 s into the
  fall: dist d1fe5a4 sharp, new build radial streaks; pitch 0: vertical
  streaks. Not checked in the real sp_a2_intro shaft (its opening freezes
  the player, setpos is ignored there).
- Still open: the gun's glow seen through the wall after the sp_a2_intro
  fall (next).
- Owner round 5 (2026-10-21): splash and dust fixed for now.
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed.
- 2026-10-21 autorun, intro3 fall: FIXED b8a2d76. Cause: the two entities
  behind it had no code at all: `env_splash` "fall_splash" (input Splash
  from the trigger at the pool) and `env_ar2explosion` "breakable_floor_dust"
  (input Explode when the floor gives way by the gun pedestal). Now
  Splash plays `water_splash_01/02/03` by scale (16 -> 03) pointing up,
  with `Physics.WaterSplash` (client.dll's watersplash/FX_GunshotSplash
  names; the scale thresholds 2/4 are our choice), and Explode throws a
  one-shot cloud of 48 grey puffs (~3.5 s) drawn like env_steam. The
  splash's spawnflag 1 (find the water below) isn't traced; the entity sits
  on the water. Evidence (autopilot only): `goto/lookat fall_splash; input
  fall_splash splash` and the same for breakable_floor_dust explode, dist
  d0822f2 shows nothing, b8a2d76 the splash spray and the dust ball. Not
  checked through the real fall path (the floor break is scripted).
- Still open: the gun's beam while holding (done in T-173), motion
  blur/speed lines, the gun's glow seen through the wall after the
  sp_a2_intro fall.
- Fall smoke and water splash (+ sounds) in intro3; the gun's beam when
  holding things; motion blur/speed lines; the gun's glow seen through the
  wall after the sp_a2_intro fall.

### T-153 GLaDOS wake-up  (P3, FIXED, awaiting owner replay: leftover particles)
- 2026-10-21: the leftover white spray was glados_head_drag_droplets (800/s
  water_splash droplets on a ring around her head; bisected by
  DestroyImmediately on each fx: only fx_glados_head_01 removes it). Its
  `Position Modify Place On Ground` (include water) was unsupported, so the
  droplets sprayed in mid-air around her head instead of on the pool. Now
  p2_fx traces once per frame below each such system's CP0 (static world via
  avian, or a WorldReflectWater plane above it) and the initializer drops
  particles onto it within `max trace length`. Unit test
  place_on_ground_drops_particles_within_reach. Autopilot sp_a1_wakeup
  (`input glados_start Trigger`, shots 9/13/17/21 s, dist de1d09b vs new):
  the spray is smaller, sits at the water, gone by 17 s (before: mid-air
  through 21 s). New tool: p2_fx example `pcfdump <file.pcf> <system>`.
- Owner round 7 (2026-10-21, build 6e0efbf): "still has some particles
  around it but its better, portal gun visibly drops, her arms are
  connected, and the rest is good there".
- Owner playtest 9 (2026-10-20, build 014ab6e): "glados awakening animation is still covered by lots of white particles".
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
- White particles FIXED (69426f1, 2026-10-21, autopilot only, build
  0.1.0+69426f1.20261021T1447Z): gladox_fx_start starts glados_head/chest/
  neck_drag (scene_fx.pcf) on her attachments. Their wake-ring children use
  particle/wake/ring_wake_mod.vmt, a `$mod2x` SpriteCard ($overbrightfactor
  3, $addself 2.5) that we drew as a plain sprite: a huge white disc (radius
  up to 192 u). SpriteCard `$mod2x` now blends as 2*src*dst (grey = no
  change). Also implemented `Remap CP Speed to CP` and emit_continuously's
  `emission count scale control point` (only once that CP is set), so the
  drag dust/wakes emit only while she moves (unit test
  drag_effect_emits_only_while_its_control_point_moves). Autopilot
  sp_a1_wakeup `3 setpos 8950 1070 380; 4 input glados_start Trigger;
  16 lookat fx_chest_drag_01`: before (dist dcc0c0e) a white disc covers
  her body; after, her body is visible, no white cloud through t=35. Killing
  the three drag effects in the old build also removed the cloud
  (confirms the source). `Position Modify Place On Ground` is still
  unsupported (wakes sit at the attachment instead of on the water).
- Still open: arms floating unattached, the player looking the wrong way.



### T-112 Missing or flat textures  (P2, attempt MERGED 2c08d11 for owner testing)
- 2026-10-21: merged at the owner's request (autopilot: floor changes 1-2/255).
- 2026-10-21 check (attempt merged onto wheatley, sp_a1_wakeup `setpos 9060
  1120 440 15 20`): floor pixels change by 1-2/255 vs dist b8a2d76, still the
  yellow-brown sheet. Owner: keep the branch open. Next suspects: $refracttint
  {151 135 34} gamma, the fog colour.
- Owner playtest 9 (2026-10-20, build 014ab6e): "glados chamber floor is still just a weird murky brown ... it does look better than before". Likely needs the water reflection (see T-151, attempt/T-151).
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
- Autorun 2026-10-21: reproduced the look (sp_a1_wakeup `setpos 9060 1120
  440 15 20`: flat yellow-olive sheet), cause not confirmable without a
  Windows shot. Suspected: water_glados_01 has `$reflecttexture
  _rt_WaterReflection` WITHOUT `$reflectskyboxonly`, so Source reflects the
  whole chamber (a mirrored scene render); we reflect nothing there (fogged
  map: no sky either, and baked env_cubemaps aren't loaded).
  Untested fix: local branch attempt/T-112 (cc81fdb), needs owner
  review/replay. A half-size camera mirrored in the nearest such water plane
  below the eye, oblique near plane at the water (p2_render's
  PortalProjection), sampled x-flipped by glass.wgsl and Fresnel-mixed
  (FLAG_WORLD_REFLECT); unit test for the mirror/clip plane. Autopilot: a
  reflection-only debug shot shows the mirrored chamber; with the normal
  blend the change is small at shallow pitches (Fresnel ~0.1), so the
  floor still reads yellow-brown. Costs: one extra half-res scene render on
  maps with such water; that water is drawn in the main view only, so it
  is missing when seen through portals. intro3/laser_intro water
  (`$reflectskyboxonly 1`) is untouched (that is attempt/T-151).
  If the owner still sees brown, the next suspects are `$refracttint
  {151 135 34}` (not gamma-converted here) and the fog colour.

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
