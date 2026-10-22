# p2_game status and handoff notes

The `p2_game` crate (binary `portal2`) plays Portal 2's own levels from the
player's install, natively on Apple Silicon (Metal), on top of the Portal 2
Rust Engine crates. No Valve code: behaviour comes from the levels' own
entity wiring, the game's text scenes/scripts/captions, public file formats
and observation. Never use leaked Valve source (the owner has some in
~/Downloads/game — do not open or use it). Studying the owner's own
server.dll/client.dll (Ghidra) to understand behaviour is allowed (owner,
2026-10-21; this replaces the earlier "or decompile Valve binaries"); write
our own code and never commit decompiled code.

## Run

```sh
export PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621"
cargo build --release -p p2_game
./target/release/portal2              # main menu
./target/release/portal2 sp_a1_intro2 # a map directly
```

Scripted tests: `P2_SCRIPT="5 goto box 60; 5.2 lookat box; 5.5 use; 6 shot /tmp/a.png; 7 quit"`
(commands in `crates/p2_game/src/autopilot.rs`), `P2_TRACE=1` for traces,
`RUST_LOG=info,portal2=debug` for every fired output. Scripted runs lock real
input and stay windowed. Screenshots of a covered/background window can come
out black (macOS stops drawing it) — not a game bug.

Autopilot extras: `input <target> <input> [param]` sends a map input,
`ray x y z [dx dy dz]` logs what is below a Source point (or along a
direction), `info <name>` (or `ent`) logs an entity's state, place, axes,
bounds and a prop's physics body, `click <Action>` presses a
menu button by its action (`click pause`, `click Options` opens the
flyout, then `click Page(Video)` etc.; `click Toggle(VSync)`,
`click Adjust(Fov,1.0)`, `click Done`; the full list is under "cloud/ui
round 2"; an unknown name logs the buttons on screen). The script clock is real time (menus pause the world).
`P2_TRACE=1` also logs laser legs, catapult launches, trigger_look and gel
bounces; `RUST_LOG=...,portal2=debug` logs portal shot results and
no-portal volumes.

Scripted runs never write the player's settings.cfg/save.txt; set
`P2_DATA_DIR=/some/dir` to give them a test data folder (it also moves the
normal data folder for any run).

Tools: `cargo run -p source_assets --example ents -- MAP [class|targetname|*N]`,
`--example ls -- PREFIX` / `-- --cat PATH` / `-- --fmt PREFIX`, `--example pak -- MAP`,
`--example probe -- MAP x,y,z`, `cargo run -p p2_assets --example animinfo -- MODEL SEQ ATTACH`.

## Works

Menus (main, chapter select, pause, options: volumes, mouse, FOV 75..110,
fullscreen, resolution, v-sync, frame limit, MSAA, anisotropic filtering,
Original/Enhanced preset (stored only), and a Subtitles page with size,
colour and background), autosave/continue, campaign order + chapter
titles from `sp_transition_list.nut`, level loading/exits, entity I/O
(relays, counters, branches, cases, timers, proxies, autos), triggers,
teleports, fades, elevator trains (player locked to the car), doors (incl.
use-to-open + locks), buttons (floor + use, locks), map portals, dynamic cubes
(carry/drop/transit), displacement + entity-prop collision, choreographed
scenes (`.vcd` text) with voice lines and OnTrigger/OnCompletion, captions
(text + compiled VCCD from every game folder), ai_script_conditions,
instructor hints (text only), ambient_generic, menu music, point_viewcontrol
(hold time; animated-rig cameras behind `P2_CINEMATIC=1`), OnAnimationDone,
cube droppers / entity templates (`templates.rs`: point_template and
env_entity_maker ForceSpawn clone the templated entity hierarchy), fizzled
cubes fire OnFizzled, OnStartTouchAll/OnEndTouchAll, per-bone `.phy`
collision posed by the default sequence, faith plates (`trigger_catapult`:
playerspeed = ground speed toward launchtarget, vertical solved for 600
u/s^2), trigger_hurt + death fade + level reload (`health.rs`, also
`player_loadsaved`), trigger_push, lasers (`lasers.rs`: portals,
reflection cubes, catchers, relays, burn), trigger_look (look within
`fieldofview` of the target for `looktime` while inside; OnTimeout),
func_noportal_volume (the gun refuses openings reaching into one), gels
(`paint.rs`: info_paint_sprayer blobs fly under gravity, through portals,
and paint splats; blue bounces on landing/off walls, orange raises top
speed, white makes surfaces portalable, water erases; maps that pre-paint
at spawn do so), personality cores drawn with their sphere model.

Level scripts: `crates/vscript` is our own Squirrel interpreter (lexer,
parser, tree-walker; parses all 82 shipped `.nut` files). `vscripts.rs`
loads every entity's `vscripts`, runs Precache/OnPostSpawn, think
functions, RunScriptCode/CallScriptFunction, ConnectOutput hooks, and
answers EntFire/CreateSceneEntity/Entities.*/handle methods. Entities whose
scripts loaded skip the old hand-written runscriptcode stand-ins.
`P2_NOSCRIPTS=1` turns scripts off. Level transitions run the game's own
`TransitionFromMap()` (TryDLC1InstalledOrCatch throws, so it falls through
to RealTransitionFromMap like single-player Portal 2). GladosPlayVcd plays
lines (GetCurrentScene finds an actor's scene by the VCD's actor names).

Entity values: `vbsp` lowercases the whole entity lump, so we read the raw
lump (`entities::entity_lump`). Values are still lowercased for matching,
except output parameters and `thinkfunction` (Squirrel is case-sensitive);
non-script input parameters are lowercased on delivery. A sweep of all 63
campaign maps (12 s each) shows no script errors except calls the maps
themselves get wrong (`gladosplayvcd(392)` in sp_a2_bts6 is lowercase in
the shipped map).

Owner's feature wishlist: docs/ROADMAP.md.

## Chapter 1 end to end (2026-10-18 evening)

Every chapter 1 map hands off to the next: intro2-intro6 through the
departure elevator (`3 goto departure_elevator-elevator_1`), sp_a1_intro7
through `transition_trigger` (-2208 376 1280, after Wheatley's transition),
sp_a1_wakeup through the incinerator drop.

sp_a1_wakeup, checked with scripted runs:
- Wheatley starts in the player's hands (`ForcePickup`, props.rs) and stays
  there (`SetDropEnabled 0`); the Wheatley-filtered triggers fire: the gantry
  door opens, the breaker room hatch opens, all of Wheatley's lines up to the
  socket play (gantry, "there she is", stairs, jump, catwalk, switch room).
- The stair jump: walk off the north end of the walkway through the gap at
  x≈10928 (a pillar and the railing leave ~45 units; the player is 32 wide).
- **Blocker: plugging Wheatley in.** `basement_breakers_socket_trigger` is a
  6x4x6 trigger parented to `core_receptacle_socket`, which rides
  `breaker_shaft`'s `psphere_plug_attach` (bone `joint2`). As the plug rises,
  `breaker_shaft_open_hatch` tilts that attachment 20° about X. That tilt is
  authored, not a decode error: it is joint2's own local rotation track, and
  `breaker_shaft_going_up` starts from the same pose turned 90° (checked by
  the animation session with `EVALTEST=joint2 animinfo ...`). With it the
  trigger sits at y 1086-1090, 10 units inside the solid face at y=1100,
  out of held Wheatley's reach. Rays show the face is a brush of
  `func_tracktrain *50` (`breaker_train`: 3 brushes, all CONTENTS_SOLID), not
  the shaft prop or its hatch. Still unexplained; leads:
  - whether the trigger's offset should be taken from the socket's map
    transform (8990 1090.84 -655, yaw 90) as now, and which shaft pose is
    current when the logic_auto snaps the socket at map spawn;
  - `core_receptacle_socket_brush` (func_brush on the socket's
    `panel_attach`) and the hatch bones (`hatch_joint`, `hatch_joint1`):
    what they cover in Portal 2, and whether the player and Wheatley are
    meant to reach the face at all;
  - how Source tests a held vphysics object against a trigger (its
    surrounding box, any margin).
  To test past it: `input socket_powered_rl trigger` (what the trigger
  fires), or `input basement_breakers_start trigger`.
- After the socket everything works: breakers, platform ride to GLaDOS
  (`basement_elevator_complete_rl`), the 13-scene wake-up chain
  (GladosPlayVcd 589), the claw pickup (`relay_start_claw_pickup` fired by
  scene A15), the claw camera rides `ghostanim` to the incinerator (the 20 m
  attachment sanity limit is now 120 m), `trigger_weapon_strip` takes the
  gun, `TeleportToView` drops the player over the opened hatch, the
  `linked_portal_door` pair carries them to the drop room, and
  `@transition_from_map` loads sp_a2_intro.
- `linked_portal_door` (portals.rs): map-paired world portals the player and
  props go through; not drawn (the renderer shows only the gun's pair), so
  the hole looks like whatever is below it. Also used by sp_a4_finale2.
- Parenting (movers.rs): plainly parented children now follow a parent that
  moved by any means (an animated attachment), not only movers; children
  riding an attachment themselves are left to viewcontrol.
- Scripted `setpos` brings a held prop along, but big jumps can still drop it
  inside geometry (physics pushes it out, the carry gives up): walk the last
  stretch, or `input @sphere forcepickup` to grab Wheatley again.

## Chapters 6-9 (2026-10-19, worktree session "ch6-9")

Checked map by map in play order with autopilot runs (`setpos`, `use`,
`propto` cubes onto buttons, `info`), so each note says what was proven:
the I/O chain that opens the way works and the next map loads. Routes
between those points (portal shots, gel jumps) were not all flown.

New engine pieces:
- `rotators.rs`: `func_door_rotating`, `func_rot_button`, `func_rotating`
  turn about their origin (they didn't move at all before, so their
  OnFullyOpen / OnIn chains never fired).
- Gel tuning from Portal 2's published cvar defaults (Valve Developer
  Community, "Gel-related console commands"): `bounce_paint_min_speed`
  500 (walk-on bounce ~208 u), `bounce_paint_wall_jump_upward_speed` 275,
  `sv_speed_paint_max` 800 reached at `sv_speed_paint_acceleration` 500.
  Still not modelled: `sv_press_jump_to_bounce` 3 (jump pressed on gel),
  `sv_wall_bounce_trade` 0.73, `sv_wall_jump_help`.
- sp_a3_01 starts without the gun (the transition script's
  `upgrade_portalgun` only upgrades a held gun; a gun lying in a later map
  is the one the player lost).

Per map:
- **sp_a3_00**: the fall plays and sp_a3_01 loads after ~88 s.
- **sp_a3_01**: knockout scene (~18 s, `knockout-viewcontroller`) puts the
  player by the gun; gun pickup; breaker lever (`use` at 3640 4482 -430)
  lights the area; `big_door_button1`/`2` within 4.5 s -> `door_start` ->
  vault door opens (its collision follows the animation); exit door
  button (5698 4500 -491, `use`) -> `transition_trigger` -> sp_a3_03.
- **sp_a3_03**: `door1_button`; lift (`elevator_trigger` +
  `lift_button_up`) rises; `pump_machine_blue_button` turns its lever
  (rotators.rs) -> `pump_machine_relay` -> control room gates swing open;
  `transition_trigger` (12160 1920 -64) -> sp_a3_jump_intro.
- **sp_a3_jump_intro**: blue floors pre-painted at spawn; walking onto blue
  bounces (P2_TRACE logs it). Room 1: pedestal (`use` at -310 745 290)
  drops the cube, which bounces on the gel; cube on the floor button
  (352 544 280) opens `room_1_door`. Room 2: cubes on both floor buttons
  flip `flipping_panel_1/2_tile_1` (rotators.rs). Exit lift (-650 2080
  1580) -> sp_a3_bomb_flings. The intended climb (bounce into a wall
  portal, portal on the high wall) is a portal + gel route, not flown.
  Room 1's gel looks flat (plain blue, unlit).
- The "black elevator": the entrance lift rides up a dark shaft at -8880
  (lit by one `light`) before `startup_lighting_teleport` moves the player
  into the room's copy of the lift; the shaft walls render near black.

## Upstream port (2026-10-21)
Checked github.com/Stink-O/Portal2-rust-engine for changes since our base.
Our first commit 76c4767 is upstream's tag v0.1.0 (shared history, not a
squash). Upstream main is 18230b3; one commit since the base:

- 18230b3 "Relicense to GPL-3.0-only" (2026-10-20): SKIPPED. Licence text
  only (LICENSE, NOTICE, READMEs, `license =` fields in Cargo.toml); no
  code, shader, asset-loading, physics or portal change, so nothing to port.
  Our copy stays on the MIT terms of release 0.1.0, which upstream says
  remain valid; NOTICE now says so. Note for later: any code taken from
  upstream after this commit comes under GPL-3.0-only, which would require
  Prtl2Mac to be GPL-3.0-compatible and publish its source. The commit also
  flags the Valve-derived shader starting points as unsettled; ours is
  crates/p2_render/src/portal_surface.wgsl (from the public SDK 2013
  portal_refract shaders; we have no spritecard.wgsl).

## Next (priority order)

1. **sp_a1_wakeup socket** (above): find why held Wheatley can't reach the
   socket trigger, then carry him up to the opened cabinet and check
   `socket_powered_rl` fires and the platform rises with the player on it.
2. **sp_a1_intro1 wrecked vault / container ride**: the finedebris_part
   models (170 bones) need per-bone playback to look right; the vault is
   dark (env_projectedtexture, see SESSION_HANDOFF.md).
3. Chapter 2 in play order: solve each chamber with autopilot (TASKS.md
   lists what is verified).
4. Instructor hint icons (`hint_icon_onscreen` -> materials/vgui/hud/…),
   refract/glass shaders (vault window), env_tonemap, sounds on prop events.
5. Play on through the chapters with P2_SCRIPT runs, fixing whatever blocks
   progress. Scripts now really run (map RunScriptCode used to reach them
   lowercased), so watch for new behaviour in GLaDOS/Wheatley sequences.
6. Gels, next: measure the real bounce/speed tuning, gel on props (cubes
   with `paintpower`, prop_paint_bomb), splat look (soft edges, lighting),
   trigger_paint_cleanser, paint_sphere (finale). Blobs through portals and
   white gel taking portals are written but not yet tried in a map.
   (Props, cleanser, bombs, paint_sphere, tuning: cloud/gels, below.)
7. trigger_look's line-of-sight flag (128) isn't checked.
8. Missing: npc_wheatley_boss (finale), func_rotating, logic_playerproxy,
   env_global, Wheatley's rail animation (item 2).
9. Settings: with v-sync off and no frame cap, a background test window ran
   at 60 fps (95-103 with a 120/240 cap); check in a focused window.
   Enhanced graphics does nothing yet; model textures ignore the AF setting.

## cloud/aimers branch (untested in game)

Built in the cloud with unit tests only (no game files, no GPU).

What's new:
- `tanks.rs`, **func_tank** (any `func_tank*` class): a tank turns about
  its own origin, relative to its unturned pose (its spawn pose, or where
  its parent / animated attachment puts it this tick):
  - yaw within +-`yawrange` (180 and more = all the way round, the short
    way), pitch within +-`pitchrange`, at `yawrate` / `pitchrate` deg/s;
  - it aims its barrel (`barrel`/`barrely`/`barrelz`) at the target.
  - Inputs: `Activate`, `Deactivate`, `SetTargetEntity` /
    `SetTargetEntityName` (a point_viewproxy, point_viewcontrol or
    `!player` = the main camera, otherwise the entity's origin),
    `SetTargetPosition`, `SetTargetDir`, `ClearTargetEntity` /
    `ClearTarget` (holds its aim). Spawnflag 1 = starts active.
  - Children: `movers.rs` `follow_parents` places a tank child from its
    parent and then adds the tank's own turn (`Tanks::place`), and the
    tank's children follow it the same tick. A tank on an animated
    attachment is turned again after `follow_attachments` each frame.
  - Not saved in save games yet (tanks restart unturned on a load).
- `sprites.rs`, **env_sprite** (and env_glow, env_sprite_oriented):
  - Drawn as camera-facing quads of the sprite material's `$basetexture`.
    Size = texels x `scale` units; colour = `rendercolor` x
    `HDRColorScale` (linear), alpha = `renderamt`.
  - Additive for `rendermode` 3/5/9, blended otherwise. Each sprite is
    pulled `GlowProxySize` units toward the camera so its fixture doesn't
    clip it.
  - Starts on with spawnflag 1, or when it has no name.
  - Inputs: ShowSprite / HideSprite / ToggleSprite, ColorRedValue /
    ColorGreenValue / ColorBlueValue, Color, Alpha, SetScale.
  - Placed from the entity's transform every frame, so parented sprites
    follow their parent.
- **point_spotlight**: a camera-facing additive cone from the lamp along
  its forward, `spotlightlength` long. It is `spotlightwidth` wide at the
  lamp and twice that at the far end (`SPOTLIGHT_SPREAD`, a guess). Its
  texture is a generated gradient fading along the cone and to the
  edges. Takes `rendercolor` x `HDRColorScale`; LightOn / LightOff;
  spawnflag 1 = on.
- **env_beam**: a camera-facing ribbon from `LightningStart` to
  `LightningEnd` (or to itself), `BoltWidth` wide, with its `texture`
  material. TurnOn / TurnOff / Toggle. No noise.

Verify in game:
1. **sp_a1_intro1, Wheatley's rail**:
   - `top_swivel_1` and `bottom_swivel_1` turn toward the player's view
     once activated (`SetTargetEntity camera_1`), quickly (600 / 300
     deg/s).
   - Wheatley (@sphere, under bottom_swivel_1) swings with them and stays
     attached, while the rail (actor_railing_1) moves.
   - If a swivel turns the wrong way (mirror), check `angles_to`'s axis
     change.
   - If it turns about the wrong point, the brush's origin isn't its
     pivot.
   - If the children lag a frame behind the rail, the chain mixes an
     animated attachment (Update) with plain parenting (FixedUpdate).
2. **Other tank users**: sp_a1_intro3, sp_a2_bts2 and others: the turrets
   / cameras built from func_tank face their targets and stop at their
   yaw/pitch limits.
3. **sp_a3_01 glows** (64 env_sprites):
   - right size and colour, additive;
   - not cut by the lamps they sit on;
   - hidden ones appear on ShowSprite.
   - Not done: occlusion fading. Glows show through nothing (they are
     depth-tested), but don't fade behind thin geometry as Source's
     `GlowProxySize` query does.
   - `rendermode 3` (constant screen size) is approximated: world size at
     256 units, growing with distance.
4. **Spotlights**: point_spotlight cones look like Portal 2's soft light
   shafts (length, width, brightness, the far-end width) and switch with
   LightOn/LightOff.
5. **env_beam** ribbons span their two ends with the right texture.
## cloud/glass branch (untested in game)

Glass (`Refract`) and water (`Water`) materials, built without the game
(unit tests, `glass.wgsl` validated with naga against stubs of Bevy's
imports; no Portal 2 assets or GPU in the cloud). Nothing here has been
seen running. Before, both were a faint tinted pane ("shader refract not
translated yet"); they no longer appear in that fallback list.

How it works (`source_assets/src/glass.rs`, `shaders/glass.wgsl`):
- `GlassMaterial` reads Bevy's screen-space transmission texture, so Bevy
  draws it in its transmissive pass after copying the opaque scene once per
  view (Camera3d already requires `ScreenSpaceTransmission`, 1 step). The
  Valve main pass and the projected-light pass are now ordered before that
  pass so the copy includes them. Cost: one colour copy per view with glass
  or water in it, plus the material's own fragment work.
- Refract: scene behind offset by the normal map's xy * `$refractamount`
  (falling off beyond 128 units), `$bluramount` 5-tap blur, `$refracttint`
  and `$refracttinttexture`, `+ $envmap * $envmaptint` when the material has
  its own cubemap (`env_cubemap` is skipped: there are no per-map cubemaps
  yet), `$basetexture` over it by its alpha, lit by the lightmap (brushes)
  or vertex light (models) and the exposure.
- Water: the same refraction, faded into `$fogcolor` between `$fogstart`
  and `$fogend` along the view ray through an assumed 64 units of water
  (scene depth isn't readable in this pass), mixed with a reflection
  (`$envmap`, else the map's sky) by a Schlick Fresnel term times
  `$reflecttint`. Goo is water with a short, murky fog, so it should come out
  nearly opaque green-brown with its normal map scrolling.
- Proxies (TextureScroll on `$bumptransform`, AnimatedTexture on
  `$bumpframe`/`$frame`) run every frame per glass material; the exposure
  follows `ValveExposure` (updated when it moves more than 1%).
- **Estimates** to tune by eye: `REFRACT_SCALE` (0.1, distortion strength
  per unit of `$refractamount`), `WATER_DEPTH` (64 units), default
  `$refractamount` 0.5. They're constants in both `glass.rs` and `glass.wgsl`.
- Not done: real planar water reflections (`_rt_WaterReflection`), the
  underwater view (`$abovewater 0`, fog when the camera is under the
  surface), Bevy distance fog on the glass's own reflection/dirt,
  `LightmappedReflective` and `PortalRefract` (still the pane).

Check in the real game:
1. **sp_a1_intro1 vault window**: through the relaxation vault's window
   the room/facility behind is visible and slightly wavy, not a faint
   flat-coloured pane; the dirt/streaks (if the material has a base texture)
   sit on it. If the distortion is far too strong or invisible, change
   `REFRACT_SCALE`; if the image behind is upside down or shifted, the
   screen UV (`in.position.xy / size`) is the place to look.
2. **sp_a1_intro3 / sp_a1_intro4 elevator glass**: the elevator tube and
   doors show the shaft and chamber behind with a slight distortion; things
   in front of the glass don't smear into it badly (some bleeding at edges is
   expected, Source has it too); no black flashes when the elevator moves.
3. **sp_a2_laser_over_goo**: the goo is a murky, mostly opaque
   green-brown surface whose ripples scroll; looking straight down shows a
   little of what's below near the edges, grazing angles turn it reflective
   (sky colour). If it's see-through like clear water, raise `WATER_DEPTH` or
   check `$fogend`; if it's flat colour with no motion, check that its
   `$normalmap` loaded and its proxies run.
4. **sp_a2_bts maps**: flooded areas and observation windows look like 1-3;
   frame time with large water areas is acceptable.
5. Portal views: glass and water seen through a portal look the same as
   directly.

## cloud/projtex branch (untested in game)

`env_projectedtexture` (shadowed spotlights), built without the game
(unit tests, WGSL validated with naga; no Portal 2 assets or GPU in the
cloud). Nothing here has been seen running. `P2_NOPROJTEX=1` turns every
projected light off for before/after comparisons.

What changed:
- `source_assets/src/projected.rs`: `ProjectedLight` component (colour,
  FOV, near/far, cookie, shadows, lightworld); the light sits at the
  entity's GlobalTransform and shines along its local +X (Source forward),
  local +Y up. Each frame the 8 enabled lights nearest the main camera go
  into one storage buffer; the nearest 4 with shadows get a 1024² shadow
  map. Cookies (`texturename`) are read from the VTF, resampled to 256²
  into a texture array (white if missing, with a warning).
- Light maths (`shaders/projected_common.wgsl`): `albedo * colour * cookie *
  N.L * atten * shadow`, added on top of the baked light and scaled by the
  exposure like it. Colour = lightcolor rgb/255 * intensity/255 *
  brightnessscale. Attenuation: Source flashlight constants as publicly
  described for this entity (constant 0, linear 100, quadratic 0: full
  within 100 units, then 100/d), faded out from 60% of farz to farz.
  **These are estimates**; if lights look too bright/dim or too short,
  change `LINEAR_ATTEN`/`light_color` and `projected_attenuation` together.
- Valve path (`projected_render.rs`): after the Valve main pass, each
  lit opaque/alpha-tested surface inside a light frustum is drawn again
  additively (same depth, small bias toward the camera). Shadow maps are
  rendered from the same Valve meshes (alpha-tested ones cut out) and
  submitted before the frame's passes; 2x2 bilinear PCF with a slope bias.
- Fallback `SourceMaterial`: the same lights in its fragment shader (the
  light buffer, cookies and shadow maps are bound in every material).
- `p2_game/src/projected_lights.rs`: one light per `env_projectedtexture`
  at level load, on if spawnflag 1 is set (`startactive 1` also counts);
  follows its entity's transform every frame (parented lights ride trains
  and attachments through movers/viewcontrol), faces `target` if set,
  colour fades by `colortransitiontime` (taken as a rate: 255*rate per
  second per channel; an estimate). Inputs: TurnOn, TurnOff, LightColor,
  FOV, EnableShadows, SpotlightTexture, SetNearZ, SetFarZ, LightWorld,
  Target.

Not done / known limits: translucent surfaces and decals/overlays get no
projected light (decals would be lit twice); unlit surfaces aren't lit (they
still cast shadows); fallback-material meshes are lit but cast no shadows;
`lightonlytarget` lights everything; no volumetric light shafts.

Check in the real game (compare with `P2_NOPROJTEX=1`):
1. **sp_a1_intro1, wrecked relaxation vault (after the second wake-up)**:
   the room is lit by `light_room_00` (a soft cone with the
   flashlight cookie's pattern, furniture throwing shadows on the walls and
   floor) instead of being far too dark. When Wheatley opens the door,
   `light_door_00` turns on (TurnOn from the map) and lights the doorway.
   Look for: the cookie upside down or mirrored (flip the `vec2(0.5, -0.5)`
   in `projected_light` in projected_common.wgsl), the light pointing the
   wrong way (forward axis), shadow acne (stripes; raise the bias) or
   detached shadows (lower it), surfaces flickering between lit and unlit
   (depth bias of the light pass).
2. **sp_a1_intro1 container ride**: the `light_track_*` lights move with
   the vault/train and sweep across the walls; nothing stays lit behind.
3. **sp_a1_intro4 elevator** (`departure_elevator_shadowed_light`): the
   elevator interior is lit with shadows of the elevator's bars, and the
   light moves with the elevator.
4. **sp_a2_core** (7 lights; finales 4-9): no missing lights (only the 8
   nearest are drawn and 4 have shadows: check nothing pops on and off as you
   walk), frame time acceptable, `RUST_LOG=info` shows no "cookie not found"
   warnings.
5. Brightness overall: lit areas near a light shouldn't burn to white; if
   they do, the colour/attenuation estimate is too strong.

## cloud/proxies branch (untested in game)

Source material proxies, built without the game (unit tests only; no
Portal 2 assets or GPU in the cloud). Nothing here has been seen running.

**Before testing: one line in `io.rs` (not changed on this branch, the
local session owns it).** `Ents::apply` swallows every input to
`env_texturetoggle` (it is in the `"logic_auto" | "logic_script" | ... => true`
list near the end), so `SetTextureIndex` never reaches
`texture_toggle.rs`. Remove `| "env_texturetoggle"` from that list (keep
`info_overlay_accessor`). `prop_indicator_panel` inputs already get through.

What changed:
- `vmt.rs`: every material's `Proxies` block is parsed into
  `MaterialInfo::proxies` (`MaterialProxy`: lowercased name, keys,
  `result_var()`), patch `insert`s included.
- `proxy.rs` (new): proxies compiled once per material (`ProxyProgram`)
  and run every frame over its variables: AnimatedTexture, TextureScroll,
  Sine, LinearRamp, UniformNoise, Add/Subtract/Multiply/Divide, Equals,
  Abs/Frac/Int, Clamp, LessOrEqual, SelectFirstIfNonZero, WrapMinMax,
  Exponential, CurrentTime, EntityRandom, LightedMouth, ToggleTexture.
  Others (PortalOpenAmount, ConveyorScroll, MaterialModify, ...) are skipped
  and leave the VMT's values. `$var[1]` components, literal or variable
  inputs, and `center/scale/rotate/translate` texture transforms work.
  Only surfaces whose material has proxies are updated (PostUpdate).
- Per-entity inputs (pub resources, keyed by the spawned entity
  `MapEntity::entity`, or an overlay surface): `source_assets::TextureFrames`
  (read by ToggleTexture) and `source_assets::SpeakingAmount` (read by
  LightedMouth; an entity not in it counts as 1 = fully lit). Nothing writes
  `SpeakingAmount` yet: to make Wheatley's eye pulse with his voice, set his
  core's entity to the voice loudness (0..1) from voice playback and remove
  it when he stops.
- Valve path (`valve.rs`, `valve_render.rs`): constants are built from
  material variables (`surface_consts`), so proxy results reach the right
  registers ($color/$alpha, $selfillumtint, $envmaptint, $detailtint,
  $detailblendfactor, $basetexturetransform...). Each sampler knows its frame
  variable ($frame, $bumpframe, $detailframe, ...); all frames of a
  multi-frame VTF sit in the texture table, so a frame flip only changes a
  slot word. Proxied surfaces get a `SurfaceOverrides` component rewritten
  every frame.
- Fallback path: a proxied material gets its own `SourceMaterial` per mesh;
  tint, base texture frame and a new base-texture transform follow the
  proxies.
- `vtf.rs`: every animation frame is decoded (`frame_count()`, `frame(i)`).
- `map.rs`/`bsp_raw.rs`: overlays keep their id; overlay surfaces carry
  `SourceOverlay { id }` (what `info_overlay_accessor`'s `overlayid` names).
- `p2_game/src/texture_toggle.rs`: `env_texturetoggle` SetTextureIndex /
  IncrementTextureIndex on its `target` (brushes, props, overlay accessors);
  `prop_indicator_panel` Check/Uncheck sets its `indicatorlights` toggle to
  1/0. Frames reset on level load.
- Static changes every material now sees (closer to Portal 2, but look
  for regressions): a VMT's own `$basetexturetransform` and `$frame` are
  applied on both paths, and `$selfillummask` textures are loaded (model
  self-illumination used the base alpha before).

Check in the real game:
1. **sp_a1_intro7 Wheatley's eye**: it glows (self-illuminated, blue
   light). If not, log which material/shader the eye mesh uses
   (`SOURCE_VALVE_DEBUG=1`, and the "model materials on the fallback" log
   line): the glow only works on VertexLitGeneric/UnlitGeneric (the fallback
   material has no self-illumination), and a proxy name in
   `ProxyProgram::unsupported` would leave it static.
2. **sp_a1_intro2 / sp_a1_intro3 indicator lights**: after the io.rs line
   above, pressing the floor button turns the antlines on the floor/walls and
   the checkmark by the exit door from blue to orange; stepping off (where
   the map unchecks) turns them back. With `RUST_LOG=info,portal2=debug`
   the button's outputs should show `settextureindex` on the toggle. If the
   inputs arrive but nothing changes, check that the toggle's `target`
   finds `info_overlay_accessor`s whose `overlayid` matches an overlay
   (`SourceOverlay`) and that the material is drawn by the Valve path or
   the fallback (both handle it).
3. **Animated screens** (monitors, signs with AnimatedTexture, e.g. the
   relaxation vault TV and chamber info signs): they flip frames at their
   own rate, and nothing static started flickering.
4. **Water**: still looks as before (water is drawn as a tinted pane, so
   its animated/scrolling normal map has nothing to show yet); scrolling
   textures elsewhere (TextureScroll: fans, conveyor-like signs) move.
5. Also: no surface turned a wrong colour or shifted its texture (that would
   be a static `$basetexturetransform`/`$frame` problem), and frame time in a
   chamber with many lights is unchanged.

Not done: `prop_indicator_panel` itself is not drawn (its checkbox/timer
face); nothing feeds `SpeakingAmount` from voice playback yet.

## cloud/anim branch (merged 2026-10-18, partly verified)

Verified locally: test chamber doors animate open/closed (intro2 exit
door); Wheatley is drawn with his handles; chapter 1 exits still hand off.
Not right yet: Wheatley's eye socket is empty (the eye mesh or its
`personality_sphere_lights` selfillum part isn't showing); the intro1
wrecked vault debris (finedebris_part*) looks like cracked concrete
shells with floating blocks instead of the wrecked motel room, and is
very dark.

### Original notes from the cloud session

Model animation playback, built without the game (unit tests only; no
Portal 2 assets or GPU in the cloud). Nothing here has been seen running.

What changed:
- `source_assets`: entity props of the animating classes
  (`map::class_animates`: prop_dynamic(_override/_glow/_ornament),
  prop_testchamber_door, npc_personality_core) whose model has an animation
  with more than one frame (`ModelData::is_animated`) are drawn as rigid
  per-bone parts: each triangle goes to its dominant bone
  (`models::split_by_bone`), vertices in that bone's space, under a
  `BonePart` child of the prop's `AnimatedModel` entity. The part's
  Transform is the bone's model-space transform in world axes
  (`models::bone_transform`), so the Valve shader path and the Bevy-mesh
  path draw them unchanged. Baked lighting uses the default pose's normals.
  Per-bone `.phy` solids are colliders under their bone's part; others stay
  on the prop. Static props, physics props, cubes and turrets are drawn as
  before. Eye meshes (`EyeRefract`) get the iris projected
  (`ModelData::iris_uvs`) and use `$iris` as their base texture.
- `p2_game/src/anim_props.rs`: plays each animated prop's sequence
  (default `DefaultAnim`; personality cores their `ACT_IDLE`; else
  sequence 0), handles SetAnimation, SetAnimationNoReset,
  SetDefaultAnimation, SetPlaybackRate and test chamber door Open/Close
  (`open` then `idleopen`, `close` then `idleclose`), fires OnAnimationDone
  when a non-looping sequence it was asked to play ends (no longer for
  looping ones), then returns to the default (unless `holdanimation 1`).
  The state is `viewcontrol::PropAnims::playing`, which the attachments and
  cameras also read, so attached props follow the drawn pose. Parts are
  only moved when the sampled frame changes.
- `movers.rs`: prop_dynamic SetAnimation no longer hides door/hatch models
  or turns elevator-body collision off.

Check in the real game:
1. **sp_a1_intro1 Wheatley**: the eye is drawn (iris texture, right way
   round, looking out of the sphere's front) and the handles sit where they
   should and idle; he follows his rail (`bottom_swivel_1`) as before. If
   the eye is still missing, see what material/shader the eye mesh uses
   (it may not be an `EyeRefract` eyeball at all).
2. **sp_a1_intro2 elevator doors**: the arrival and departure elevators'
   doors slide open/closed (not vanish), the player can walk out once open
   and is blocked while closed (per-bone `.phy`; if the elevator body has a
   single solid, the doorway stays blocked: say so).
3. **Test chamber doors** (`prop_testchamber_door`): they play `open` and
   `close`. NOTE movers.rs still hides the door 0.35 s after Open and fires
   OnFullyOpen/OnFullyClosed from that timer (not changed on this branch):
   once the animation looks right, remove that hide and fire
   OnFullyOpen/OnFullyClosed from the animation's end instead.
4. **sp_a1_intro4 cube dropper**: the dropper iris
   (`box_dropper-cube_dropper_model`) opens and closes when a cube drops,
   and still hangs straight down (the `intro4_dropper_points_down` test
   checks the baked pose; the parts start in the same default pose).
5. Also look for: parts flying off or turned 90 degrees (a bone-axis or
   quaternion decode problem, compare with the old baked pose), cracks
   where a smoothly skinned model (cables, GLaDOS) bends between two bones
   (rigid parts can't stretch; GPU skinning would be the fix), props that
   jitter when they should be still, OnAnimationDone chains that now fire
   later or never (only non-looping, explicitly set sequences fire),
   GLaDOS's chamber body, and frame time in a map with many animated props.

## cloud/anim (round 2): scene animation (untested in game)

Built in the cloud with unit tests only (no game files, no GPU).

What changed:
- `scenes.rs`: VCD `gesture` events are their own kind (no longer sent as
  `scenesequence`), `lookat` events are read (actor, target `param`,
  time), and `event_ramp` blocks are kept on every event
  (`SceneEvent::ramp`, `SceneEvent::weight`: the ramp with an implied 0 at
  the event's start/end, or a 0.2 s fade in/out when there is no ramp).
  `Playing::active(now)` lists the events going on; `actor_ents` resolves
  `!targetN` / `!player` / names. `sequence` events still go through
  `scenesequence`.
- `anim_props.rs`, `pose_parts`:
  - Poses come from `Animator::evaluate` (blend grid by pose parameters,
    the sequence's auto-layers, extra layers) instead of `sample`.
  - **Crossfade**: when the sequence changes (or the map restarts one),
    the parts blend from the last drawn pose over 0.2 s (ease in/out). The
    default replaying itself (a one-frame idle) doesn't fade.
  - **Gestures**: each active `gesture` event is laid over the base with
    `apply_layer` (delta/post/weights honoured) at weight = its ramp, its
    cycle fitted to the event's length.
  - **Lookat**: the actor's target (`!player` = the player's eye, else the
    named entity) is taken into the prop's model space and turned into
    yaw/pitch (Source convention: yaw positive left of +X, pitch positive
    up), then into the model's aim pose parameters by name
    (`aim_axis`: `*yaw*`/`*rightleft*` = +yaw, `*leftright*` = -yaw,
    `*pitch*` = -pitch (Source pitch is positive down), `*updown*` =
    +pitch), clamped to each parameter's range, weighted by the event
    ramp and smoothed (~0.4 s to settle). If the base sequence
    auto-layers an aim matrix it is driven by those parameters; if not,
    the delta `*aimmatrix*` sequence sharing most name words with the base
    (`sphere_idle_neutral` -> `sphere_aimmatrix_neutral...`) is layered.
  - The drawn pose is stored in `PropAnims::local_pose`; attachments
    (`attachment_transform`) use it, so things parented to an animated
    attachment follow crossfades, gestures and aim.
- `p2_assets` `Animator::grid_params`, `Animator::cycle_at`.

Verify in game (sp_a1_intro1/2/3 with Wheatley):
1. Wheatley's sequence changes (scene `sequence` events, back to
   `sphere_idle_neutral`) blend over ~0.2 s instead of snapping.
2. Gestures (nods, glances) play on top of his idle and fade in/out with
   the scene instead of replacing it; nothing jumps when one ends.
3. During `lookat "!player"` events his eye/body turns to the player and
   follows them as they move, then eases back when the event ends. Check
   the **direction**: if he looks the opposite way in yaw or pitch, flip
   the sign for that parameter name in `aim_axis` (anim_props.rs). Run
   `cargo run -p p2_assets --example animinfo` on
   `models/npcs/personality_sphere/personality_sphere.mdl` to see his pose
   parameter names and ranges; if none matches `aim_axis`'s name rules
   nothing aims (add the name there).
4. Check which sequence is the aim matrix (`aim_layer`) and that it isn't
   also auto-layered (applied twice looks like double turning).
5. Elevator doors, chamber doors, the dropper iris and the rig cameras
   (sp_a1_intro1 wake-up, sp_a1_wakeup claw) still look as before: they
   now go through `evaluate` (blend grids at pose 0 = middle of the grid,
   auto-layers on) and crossfade 0.2 s when their sequence changes.
6. A still map costs nothing: props only re-pose while their frame, a
   fade, a gesture or the aim changes.

## Settings (merged from cloud/settings, tested 2026-10-18)

Tested in the game with scripted menu clicks: Options fits at 1280x720 in
two columns, every `<`/`>`/toggle changes its value and applies live
(FOV 110 widens the view, MSAA off, 8x AF, 120 cap, v-sync off), the
Subtitles page opens and BACK returns to Options then the pause menu, the
user's old settings.cfg loads, and settings.cfg/save.txt are written as
`version 2` (old save files still Continue; unit tests cover both). Not
checked: exclusive fullscreen resolutions, Esc on the Subtitles page.

## cloud/tools branch (merged 2026-10-18 into the Portal 2-style menus)

Merged: developer console (~, off by default; toggle in Keyboard/Mouse),
FPS and movement-stat overlays (Advanced Video: Show frame rate / Show
movement stats; verified in game: 120 fps, steady frame times),
speedrun timer and practice checkpoints, mod store (crates/p2_mods,
`p2mods` CLI) and mounting, community map discovery. Not yet in the new
menus: the Speedrun and Mods option pages and the chapter select
Community tab (their console commands work; tools_tests::options_pages_work
is ignored until they're rebuilt as Portal 2-style pages).

### Original notes from the cloud session

Developer, speedrun and modding tools from docs/ROADMAP.md, built in a cloud
session that can't run the game: `cargo check -p p2_game` and `cargo test`
pass, nothing here has been seen on screen. Every option defaults to off /
the original behaviour, all options live inside Options (no new main or
pause menu buttons), and none of the gameplay files listed as being worked on
locally were touched. New files: `console.rs`, `overlays.rs`, `speedrun.rs`
+ `splits.rs` (pure timing logic), `community.rs`, `mounts.rs`,
`tool_settings.rs`, and the crate `crates/p2_mods`. Small additive edits:
`settings.rs` (one `tools` field, 5 lines), `ui.rs` (rows, a Speedrun page,
a Community tab), `main.rs` (module lines, plugins, mounting),
`autopilot.rs` (a `con` step).

What's in it:
- **Options**: column 1 gets EXTRAS (DEVELOPER CONSOLE, SPEEDRUN >, MODS >),
  column 2 gets SHOW FPS and MOVEMENT STATS under VIDEO. MODS opens a page
  with the active mod profile (< >), every installed mod ON/OFF, UNDO LAST
  CHANGE, the first problems p2_mods reports, and "Restart the game to load
  the changes" when the profile differs from what was mounted (`mods_page.rs`).
  SPEEDRUN opens a page like
  Subtitles: timer on/off, mode (full run / single map), quick restart and
  its key (F6), practice checkpoints and their keys (F7 save, F8 load),
  checkpoint slot 1-5. Keys step through F1-F12, Insert, Delete, Home, End,
  PageUp, PageDown. Stored in settings.cfg as `con_enable`, `cl_showfps`,
  `cl_showpos`, `speedrun_timer`, `speedrun_mode`, `quick_restart(_key)`,
  `practice_mode`, `checkpoint_save_key`, `checkpoint_load_key`,
  `checkpoint_slot`, `community_maps_dir` (the extra community folder; only
  set by editing the file).
- **Console** (` or ~ once enabled; on macOS also IntlBackslash, which I
  expect ISO keyboards to send for the key left of 1 — check): pauses the world, frees the mouse, takes every key.
  History (Up/Down), Tab completion of commands, map names, entity names and
  inputs (with a suggestion list), PageUp/PageDown scroll, Esc or ` closes.
  `cmdlist` lists everything: map, maps, restart, noclip, god, give
  weapon_portalgun, upgrade_portalgun, setpos, setang, getpos (prints the
  feet, so it pastes back into setpos), fov, host_timescale, cl_showfps,
  cl_showpos, sv_cheats, fire/ent_fire, ent_list, practice_save/load/list/
  delete, speedrun_reset, speedrun_splits (PB table, golds, sum of best),
  mods, mod_list, mod_enable, mod_disable, mod_profile, mod_rollback (mod
  changes apply at the next start), exec (`<data>/cfg/<name>.cfg`;
  `autoexec.cfg` runs at startup when the console is enabled), help, find,
  echo, clear, quit. Cheats need
  `sv_cheats 1` with Portal 2's message; noclip and god reset on map load,
  sv_cheats 0 turns them and host_timescale back off.
- **Overlays** (top left): FPS with frame time min/avg/max over the last
  second; movement stats (pos/ang in Source units, speed and horizontal
  speed in u/s, ground yes/no).
- **Speedrun timer** (top right): real time while playing, stopped in menus,
  the pause menu, the console, from a level exit until the next map loads,
  and on the frame after a load. Splits when a map is left (`LevelExit`) and
  when a chapter ends, against the PB (green ahead, red behind, gold for a
  best segment). Files in `<data>/splits/`: `golds.txt` and one file per
  category (`run-<start map>.txt` for full runs, `map-<map>.txt` for single
  maps). Full-run mode keeps one run through restarts/deaths and the next
  campaign map; any other map start (chapter select, `map`) starts a new run;
  single-map mode restarts the clock on every load. A run with `sv_cheats 1`,
  `host_timescale` other than 1 or a loaded checkpoint is marked and never
  saved as a PB or gold.
- **Practice**: checkpoints (feet, velocity, view, crouch via `ForceDuck`) in
  `<data>/practice/<map>.txt`, Source units, named (`practice_save name`) or
  per slot from the keys.
- **Community maps**: `.bsp` files in `<data>/maps/` (and subfolders) plus the
  `community_maps_dir` folder. Chapter select shows CAMPAIGN / COMMUNITY tabs
  only when there are some (12 per page). At startup they are mounted as
  `maps/<name>.bsp` behind every game folder through a folder of symlinks
  (`<data>/.mount/community/maps/`), so they load by name and restart,
  death reload, Continue and trigger bounds work like campaign maps. Maps
  added while the game runs load by path until the next start.
- **Mods** (`crates/p2_mods`, CLI `cargo run -p p2_mods --bin p2mods --
  help`): store in `<data>/mods/`, install from a folder or zip, mod.toml
  manifest (id, name, version, dependencies with semver ranges, optional
  dependencies, conflicts, content folder, engine range), profiles, load
  order, dependency/conflict/cycle checks, file-overlap warnings, a snapshot
  before every profile change and `p2mods rollback`. At startup `mounts.rs`
  mounts the active profile's working mods *ahead of* the install's folders
  (`GameConfig::search_paths`, no change to fs.rs; with no mods and no
  community maps the game mounts exactly what it did before). `mods` in the
  console shows what loaded and why anything was left out.

Tests: `cargo test -p p2_mods` (store, manifests, resolution, profiles,
rollback, zip) and `cargo test -p p2_game` (console parsing/completion/
history, splits and PB/golds, checkpoints, settings, community discovery,
plus `tools_tests.rs`: the console/overlay/speedrun plugins and the real
`UiPlugin` run headless for a few frames with a panicking error handler,
clicking through the new Options rows, Speedrun and Mods pages and the
Community tab). They prove the systems fit together and the logic; they
don't show anything on screen.

Verify in the real game (`P2_SCRIPT` steps can run console lines with
`con ...`; `click` names: see "cloud/ui round 2"):
1. Options still fits at 1280x720 with the new rows; the Speedrun page opens,
   BACK and Esc return to Options; values survive a restart (settings.cfg).
2. Console: opens only when enabled; typing doesn't move the player or open
   the pause menu; Esc closes it without pausing; the mouse is grabbed again
   on close in game and free in menus; the world (and timer) stop while open.
   Check the font (looks for `resource/linux_fonts/dejavusansmono.ttf`,
   falls back to the menu font) and that the panel and suggestion box sit
   over everything.
3. `sv_cheats 1; noclip`: fly with WASD/Space/Ctrl through walls and through
   portals; nothing jitters (p2_movement still runs its move underneath and
   noclip replaces the result each tick). `god` in goo / turret fire.
   `getpos` then `setpos ...;setang ...` lands at the same spot and view.
   `fire @exit_door Open` (any name from Tab), `ent_list prop_`,
   `host_timescale 0.5`, `map sp_a2_laser_intro`, `restart`,
   `give weapon_portalgun` / `upgrade_portalgun` in sp_a1_intro3.
4. SHOW FPS with v-sync on/off and each frame cap: the numbers should match
   the cap (frame times are frame start to frame start).
5. Speedrun: turn the timer on, New Game, finish sp_a1_intro1..intro3; a
   split line per map in the console, chapter 1's split after its last map,
   `<data>/splits/` written; a second run shows deltas and golds; F6 restarts
   the map; F7/F8 save and load a checkpoint (also ducked in a vent) and mark
   the run PRACTICE.
6. Community: copy any .bsp (e.g. a copy of sp_a1_intro3 renamed) to
   `~/Library/Application Support/Portal2Rust/maps/`, open chapter select,
   COMMUNITY tab, start it, die / restart it, quit and Continue.
7. Mods: `p2mods install some_mod_folder --enable` with a mod that replaces
   a material, start the game; the material changes; `p2mods rollback` (or
   Options > Mods > UNDO LAST CHANGE) undoes it on the next start. The Mods
   page fits with 10 mods and a couple of problem lines.

TODO hooks for the local session (in files this branch didn't touch):
- `health.rs`: god mode tops `PlayerHealth::hp` up to 1e9 every frame (works
  because any hit is far below it); an `invulnerable` flag that skips damage
  would be cleaner.
- `p2_movement`: noclip overrides the controller's result after
  `P2Systems::Move`; the controller still pushes props it would touch. A
  "skip this player" marker in the controller would remove that.
- Finale: the run finishes on a `LevelExit` with no next map; the last map
  (sp_a4_finale4) ends with the credits instead, so a full run never
  finishes. Send `LevelExit` (or a new message) where the ending starts.
- `source_assets/src/fs.rs`: loose mounts skip `sound/` folders, so a mod's
  loose sounds are ignored (VPK mods are fine). Indexing `sound/` only for
  mod folders would fix it.

## cloud/ui branch (merged 2026-10-18)

Verified on screen: main menu (logo, PLAY SINGLE PLAYER / OPTIONS / QUIT),
Options flyout, Video and Advanced Video pages. Changed on merge: first
launch is windowed with subtitles on (the owner asked for both).
Difference from Portal 2: its option dialogs are light grey panels with
dark text; ours are dark.

### Original notes from the cloud session

Menus restyled after Portal 2's PC menus, with all of its options. Built
and unit-tested on Linux only (`cargo test -p p2_game -p p2_core`); nothing
here has been seen on screen yet.

What changed:
- Main menu: logo top left, items at the lower left: PLAY SINGLE PLAYER
  (flyout: CONTINUE GAME when there is a save, NEW GAME), OPTIONS (flyout:
  KEYBOARD / MOUSE, CONTROLLER, AUDIO, VIDEO), QUIT. NEW GAME opens the
  chapter dialog (as Source's New Game does); the separate CHAPTER SELECT
  item is gone. Not shown until they exist: co-op, community test
  chambers, extras, load/save game.
- Pause menu: same items as before at the lower left over the dimmed game
  (no "PAUSED" title); OPTIONS is a flyout. Quit and Quit to main menu ask
  first (OK / CANCEL).
- Options pages are dialogs: rows of `label  < value >` (click the value to
  step and wrap) or sliders (click the track to jump), footer buttons:
  DONE keeps, CANCEL puts the page back as it was, Esc = DONE. Video's
  resolution / aspect ratio / display mode wait for APPLY (or DONE).
  - Keyboard/Mouse: Customize keyboard (key list from the game's
    `scripts/kb_act.lst` with its localized labels, defaults from
    `cfg/config_default.cfg`; click a row, press a key/mouse button/wheel,
    Esc cancels; USE DEFAULTS, CLEAR KEY), sensitivity, reverse mouse,
    mouse filter, mouse acceleration, raw input.
  - Controller: game controller, look sensitivity, reverse look, vibration.
  - Audio: game volume, music volume, speaker configuration, captioning
    (Off / Subtitles / Closed captions), language (caption files), and our
    Subtitle style page (size, colour, background).
  - Video: resolution, aspect ratio, display mode, brightness, Advanced
    video: model/texture, effect, shader detail, paged pool memory,
    multicore, anti-aliasing, filtering (bilinear/trilinear/aniso 2-16x),
    v-sync, motion blur, field of view, max frame rate, Original/Enhanced.
- Bindings drive movement: `p2_core::InputBindings` (new) is read by
  `p2_movement::input::read_keys`; with no resource it uses today's keys.
- settings.cfg is version 3: new keys use Source's names (`closecaption`,
  `cc_lang`, `m_rawinput`, `mat_monitorgamma`, `gpu_level`...), bindings
  are `unbindall` + `bind "key" "command"` lines once one is changed.
  Version 1/2 files load (old `cc_subtitles 1` = Subtitles).
- First-launch defaults are Portal 2's: volume and music 100%, fullscreen
  at native resolution, captions off, brightness 2.2, top detail levels,
  motion blur and multicore on (the old file defaults were 80%/60%
  volume, windowed, subtitles on). An existing settings.cfg keeps the
  player's values.

Saved and shown but not used yet (TODO in the code says what should read
each): +attack/+attack2 rebinding (p2_gun reads mouse buttons itself) and
the other non-movement commands, raw input, mouse acceleration, the whole
Controller page, speaker configuration, closed captions for sound effects
(audio.rs would have to send ShowCaption), brightness, bilinear vs
trilinear, motion blur, multicore, the detail levels, effects volume (kept
in the file, no longer on screen).

Check in the real game:
1. Fonts: the log lists `font files in the game` and the `menu font` it
   picked. If Portal 2's menu font isn't picked (or the wrong one is), set
   `P2_MENU_FONT=resource/<file>` to try one and fix `pick_menu_font`.
2. Logo: the log lists `logo textures in the game` and the `menu logo`
   used; without one a text "PORTAL 2" shows. `P2_MENU_LOGO` overrides.
3. Main menu against the real Portal 2: item names and order, case,
   size, colours, hover bar, flyout position; logo size/position.
4. Pause menu over the game, the dimming, the confirm dialogs.
5. Every Options page fits at 1280x720 and fullscreen; the key list
   scrolls with the wheel and keeps its place after a rebind; key labels
   come from the game (not `#Valve_...` tokens).
6. Rebind jump to the wheel or move-forward to another key, play, restart
   the game: the binding sticks and moves the player; USE DEFAULTS
   restores the game's keys (right Ctrl no longer ducks with the game's
   config_default.cfg, which only binds CTRL).
7. Video: change resolution/display mode, nothing happens until APPLY;
   CANCEL after APPLY puts the old mode back.
8. Captioning: Subtitles vs Closed captions text on a scene line (closed
   captions use the closecaption files), language switch reloads captions.
9. Delete settings.cfg (or set `P2_DATA_DIR` to an empty folder): first
   launch is fullscreen, 100% volume, captions off.

## cloud/ui round 2 (untested in game)

The remaining screens on the light Portal 2 theme. Built and tested
headless only (`cargo test -p p2_game`: 77 tests, including
`tools_tests::options_pages_work`, which is no longer ignored and clicks
through every page below).

What changed:
- Options rows: only the row under the mouse shows its ◀ ▶ arrows, on a
  dark bar (`theme::ROW_HOVER`) with white text; the others show just
  label and value. Sliders turn light on the dark bar. Key-list rows use
  the same bar. Hover comes from the row's `RelativeCursorPosition`
  (`hover_rows`), so the arrows and value on it still take the clicks.
- Speedrun and Mods are two more entries in the Options flyout (after
  VIDEO). Main and pause menus have no new buttons.
  - Speedrun: timer, timer mode, quick restart (+ key), practice
    checkpoints (+ save/load keys, slot), and where splits are kept.
  - Mods: profile (◀ ▶), each mod Enabled/Disabled, Undo last change,
    problems in red, "Restart the game..." when the profile changed.
    Footer DONE only: mod changes are written as they're made.
    The list scrolls like the key list.
- New Game: CAMPAIGN / COMMUNITY MAPS tabs (small dark buttons, the
  chosen one darker) when community maps exist; community tiles with
  name and folder, ◀ PAGE n / m ▶ past 9 maps. Opening New Game rescans
  the maps folders, so maps copied in while the game runs show up.
- Quit confirmations use the same dialog (title tab, light panel, dark
  OK / CANCEL), centred over the dimmed menu.
- The pause menu keeps its items over the dimmed game; its Options
  flyout is the light panel with the dark hover bar.

Click names (autopilot `click`, case and spaces ignored):
- Main: `SinglePlayer` then `Continue` / `NewGame` (old `Chapters` still
  works); `Options` (flyout), `Quit` then `ConfirmOk` / `ConfirmCancel`.
- Pause: `pause`, `Resume`, `Restart`, `Options`, `MainMenu` then
  `ConfirmOk`, `Quit`.
- Options flyout: `Page(KeyboardMouse)`, `Page(Controller)`,
  `Page(Audio)`, `Page(Video)`, `Page(Speedrun)` (old `SpeedrunOptions`),
  `Page(Mods)` (old `ModsOptions`). Links: `Page(Keys)`,
  `Page(Subtitles)` (old `SubtitleOptions`), `Page(AdvancedVideo)`.
- Rows: `Adjust(<Setting>,-1.0)` / `Adjust(<Setting>,1.0)` (the arrows,
  hidden but clickable by script), `Toggle(<Setting>)` (the value),
  `Slide(<Setting>)`, `Bind(<n>)`. Settings include `DevConsole`
  (Keyboard/Mouse), `ShowFps`, `ShowPos`, `Fov`, `VSync` (Advanced
  video), `SpeedrunTimer`, `SpeedrunMode`, `QuickRestart`,
  `QuickRestartKey`, `Practice`, `SaveCheckpointKey`,
  `LoadCheckpointKey`, `CheckpointSlot`, `ModProfile`.
- Mods: `ModToggle(<n>)`, `ModRollback`. New Game: `ChapterTab(true)`,
  `ChapterTab(false)`, `CommunityPage(1)`, `CommunityMap(<n>)`,
  `Chapter(<n>)`.
- Footers: `Done`, `Cancel`, `Apply`, `UseDefaults`, `ClearKey`, `Back`.

Check in screenshots:
1. Any Options page with the mouse over a row: that row alone is dark
   with white text and ◀ ▶; moving down moves the bar; nothing flickers
   when the mouse is on the arrows or the value. A click rebuilds the
   page, so the bar may blink off for one frame after each click (the new
   rows only learn they're hovered on the next frame); if that shows,
   carry the hovered row across the rebuild in `rebuild_menu`.
2. A slider row hovered: track and fill still visible on the dark bar.
3. Customize keyboard: hover bar on key rows; the clicked row stays dark.
4. Options flyout lists SPEEDRUN and MODS after VIDEO, on the main menu
   and the pause menu.
5. Speedrun page and Mods page (with and without mods installed; with a
   broken mod for the red problem line) fit at 1280x720.
6. New Game with a `.bsp` in the maps folder: tabs above the grid, the
   community tile, starting it loads the map.
7. Quit and Quit to main menu: the confirm dialog's tab and panel are
   centred together; OK and CANCEL look like the footer buttons.
## cloud/gels branch (untested in game)

Chapters 6-8 gel work, from `main` 709086c, built in a cloud session without
the game: `cargo test -p p2_game` passes (81 + the new paint tests, among
them headless runs of `PaintPlugin` + `PaintPropsPlugin` with a panicking
error handler), nothing has been seen on screen. Behaviour comes from the
entities' documented keyvalues/inputs and published descriptions; every
tuning number is a named constant saying where it comes from or that it is
an estimate. Files: `paint.rs` (edited), new `paint_props.rs`, one entry
added to `props::PICKUP_CLASSES` (`prop_paint_bomb`, so bombs are physics
props, dropper copies included, and fizzlers dissolve them).

What's new:
- **Paint on props**: a blob that hits a prop paints it (blobs no longer
  pass through dynamic bodies). Blue: restitution 0.9 (Max combine) and
  props falling onto blue floors bounce; orange: friction 0.02 (Min
  combine); white: looks only; water (painttype 4) washes. The coat is a
  translucent box (a ball for `cubetype` 3 spheres) over the prop. Map props
  start painted from `paintpower` (0 bounce, 2 speed, 3 portal, 4 none).
- **trigger_paint_cleanser**: while enabled, props whose box is inside it
  lose their paint (Enable/Disable through the usual inputs).
- **prop_paint_bomb**: bursts on the first frame its velocity changes by
  more than 120 u/s (a landing, a wall), paints everything a 160-ray fan
  reaches within 160 u (props included) with its `painttype`, fires
  `OnExploded`, goes away. Held bombs don't burst. `bombtype` isn't used.
- **paint_sphere**: `Paint` input paints everything within `radius`
  (default 128 u) with its `painttype`.
- **info_paint_sprayer**: also `Toggle` and `ChangePaintType <n>`;
  `silent`, `light_position_name`, `render_mode` and the streak keyvalues
  are read by nobody (look/sound only; noted in the code).
- **Bounce tuning**: landing on blue gives back the whole speed of the fall
  (you bounce back to the height you dropped from, every time; at least
  450 u/s = ~170 u when walking on), off the splat's own slope, keeping the
  run along it; crouch still stops it. Blue walls reflect your run and add
  200 u/s up. `P2_TRACE=1` logs `TRACE bounce gel: off at N u/s (to H u)`.
- **Speed gel**: unchanged numbers (800 u/s top, accelerate 1, friction
  0.3), now documented as estimates.
- **White gel and portals**: the gun already accepts gel-covered points
  (`PortalGel`); now a portal standing only on white gel closes when that gel
  is washed off or painted over (portals on surfaces that take portals
  anyway stay).

Verify in the real game (`cl_showpos 1` shows speeds; `ent_list
info_paint_sprayer`, `ent_list prop_paint_bomb`, `ent_list paint` in the
console find the entities; `P2_TRACE=1` logs bounces):
1. sp_a3_jump_intro: the pre-painted blue floors are there at the start;
   walking onto blue bounces you ~170 u (about to a 2-high wall); dropping
   from a ledge onto blue bounces you back to the ledge height again and
   again; holding crouch stops it; running into a blue wall bounces you back
   and a little up. A cube dropped or thrown onto blue jumps; carry a cube
   under a blue sprayer: it turns blue and bounces off floors and walls.
2. sp_a3_speed_ramp: orange floors let you reach the far side with a run
   (check top speed in `cl_showpos`, ~800 u/s; tell us if the ramp jump is
   short or long, then tune `SPEED_GEL_MAX` / `SPEED_GEL_ACCELERATE`);
   an orange cube slides when pushed; letting go of the keys on orange
   keeps you sliding.
3. sp_a3_portal_intro: white gel from the sprayer makes plain walls take
   portals; a portal shot onto white gel works; washing that gel (water, if
   the map has it, or `fire <water sprayer> Start`) or painting it over
   closes that portal; a portal on a normal white wall stays.
4. Any bomb chamber (sp_a3_bomb_flings, sp_a4_...): bombs from the dropper
   burst where they land and paint a few metres around; cubes nearby take
   the colour; a fizzler dissolves a bomb carried through it.
5. A cleanser (trigger_paint_cleanser, e.g. a water curtain in the later
   chambers) strips a painted cube carried through it.
6. Look: coats sit over the cube (not offset) and follow it; spheres get a
   ball.

TODO / not done: blob streaks and drips, the player's screen splashing,
painted props' own surface paint (coat only), dry vs wet bombs, splat look
(still flat discs), measuring the tuning against the real game.



## cloud/saves branch (untested in game)

Portal 2's save system, built without the game (unit tests only). Code:
`saves.rs` (format, writing, loading, chapter progress), with hooks in
`io.rs` (`Ents::hold`, `io_state`/`set_io_state`/`truncate`; `logic_autosave`
inputs now reach `saves.rs`), `movers.rs` (`Movers::save`/`restore`),
`vscripts.rs` (plain globals: `Scripts::save`/`restore`), `templates.rs`
(copies remember their prototype; `spawn_copy`), `props.rs`
(`start_holding`), `health.rs` (death loads the latest save), `ui.rs`
(menus), `console.rs` (`save`/`load`). Not touched: scenes, paint, movies,
exposure, render crates.

Files: `<data dir>/saves/NAME.sav` = one JSON header line (version, map,
time, kind) + one JSON state line. `autosave` (level start or
`logic_autosave`; the previous one moves to `autosave01`), `quick`,
`saveNN` / any `save NAME`. A level-start autosave has no state (loading it
starts the level normally). `save.txt` is still written for older builds.
Chapter unlocks: `<data dir>/progress.cfg` (`sv_unlockedchapters`); the
first run counts the newest save / old save.txt as reached. Scripted runs
(`P2_SCRIPT` without `P2_DATA_DIR`) unlock every chapter and write nothing.

How a load works: `LoadLevel`, then on `LevelLoaded` the I/O is held (no
logic_auto, no outputs, game clock stopped) and the player is put at the
saved spot behind a black fade; after 3 settle frames the entity table is
reset to the map's own and the saved differences applied, run-time entities
re-added (template copies re-spawned from their prototypes), bodies posed
(transform, velocity, hidden, solid, despawned), movers, animations, script
globals, health put back; scenes that were playing start again from their
first line; one frame later the portals (wall colliders found again) and
the held prop.

Verify in game:
- [ ] F6 mid-puzzle ("SAVING..." bottom right, console "Saved quick"),
      move/solve something, F9: back where you were, same view, cubes where
      they were (incl. a dropped cube and one in your hands), doors/lifts in
      the same state, buttons still pressed, both portals open and
      walkable-through, pending timed outputs still happen.
- [ ] No logic_auto re-runs on load (no intro lines replayed, elevators
      don't restart); no visible jump of props after the fade.
- [ ] A map's `logic_autosave` writes `saves/autosave.sav` (sp_a1_intro
      maps have them); die afterwards (goo, turret): you come back at that
      save, not the map start. Die on a map with no mid-map save: the map
      restarts as before.
- [ ] Main menu: PLAY SINGLE PLAYER shows CONTINUE GAME / NEW GAME / LOAD
      GAME once a save exists; Continue loads the newest save.
- [ ] LOAD GAME list (main and pause menu): chapter picture, chapter title,
      chapter name, map, date/time (local), kind; scrolls; BACK and Esc
      close it; a click loads.
- [ ] Pause > SAVE GAME: NEW SAVED GAME makes save01, save02...; clicking
      an existing one overwrites it.
- [ ] Console: `save`, `save name`, `load`, `load name`, `load autosave`.
- [ ] New Game: chapters after the furthest reached are dark tiles with a
      "?" and can't be clicked; reaching a new chapter unlocks it.
- [ ] Hitch: F6 should not stutter (state is copied on the main thread,
      written on a thread); loading costs about a level load.
- [ ] Mid-scene save: the scene restarts from its first line on load
      (`Playing` is private to scenes.rs; resuming mid-line would need it).
Not saved yet: paint (paint.rs), triggers' touch sets / trigger_look
timers (taken from where the player stands during the settle frames, so a
trigger you stand in doesn't fire again; a floor button whose cube comes
back onto it fires OnPressed once more), attachments from SetParentAttachment (viewcontrol), laser /
bridge / funnel / turret internals (they rebuild from entity state),
script locals and class instances.

## cloud/particles branch (untested in game)

Map particle effects, from `main` f1aa706, built in a cloud session without
the game or its `.pcf` files: `cargo test -p p2_fx` (24) and `-p p2_game`
pass, including a headless run of the new plugin with a panicking error
handler; nothing has been seen on screen. Files: `p2_fx` (`pcf.rs` manifest
loading, `sim.rs` operators, `render.rs` loading/culling/stop), new
`p2_game/src/map_particles.rs`, one plugin line in `main.rs`.

What's new:
- **All the game's particle files load** (`particles/particles_manifest.txt`
  at startup, then the three portal files as before), plus the map's own
  `maps/<map>_particles.txt` on each load (dropped again on the next map).
  Not read: particle files packed inside a map's BSP.
- **info_particle_system**: `effect_name`, `start_active`, CP0 at the entity
  with its Source axes (follows `parentname`; `name,attachment` parents use
  just the entity), `cpoint1`..`cpoint63` on the named entities every frame,
  `cpointN_parent` keeps CP N fixed relative to CP M from the moment it
  starts. Inputs `Start` (no-op while already emitting), `Stop` (no new
  particles, live ones finish, no end cap), `StopPlayEndCap`,
  `DestroyImmediately`, `Kill`.
- **Operators**: the common ones map effects use are added to the simulator
  (list in `crates/p2_fx/README.md`, "Map effects"). I couldn't scan the
  campaign's `.pcf` files from the cloud, so the list comes from what Source
  effects generally use; every system logs what it still lacks once:
  `p2_fx: <system> uses unsupported ...`. Please grep a play-through log for
  that line and send the most common ones back for the next round
  (`Collision via traces` and `Position Modify Place On Ground` are the
  likely big ones: they need world traces).
- **env_spark**: bursts every 0.1 s + random(0..`MaxDelay`) while on; count
  and speed grow with `Magnitude`; `TrailLength` 1-3 sets the streak; 64 start
  on, 128 glow (a short flash sprite), 256 silent, 512 along the entity's
  forward; sound `DoSpark` (soundscript). Inputs StartSpark/StopSpark/
  ToggleSpark/SparkOnce. Drawn here (Source's sparks aren't a `.pcf`):
  additive streak quads, yellow to orange, gravity 600.
- **env_steam**: puffs at `Rate`/s moving at `Speed` (+ `SpreadSpeed`
  sideways) for `JetLength`, growing `StartSize`->`EndSize`, `rendercolor`/
  `renderamt`, `rollspeed`; spawnflag 1 unlit, else lit. `InitialState`;
  inputs TurnOn/TurnOff/Toggle and JetLength/Rate/Speed/SpreadSpeed <n>.
  `type 1` (heat shimmer) isn't drawn. A soft round puff texture is made
  in code. No sound (Source's steam has none of its own).
- **Cost**: effects farther than 60 m or behind the camera beyond 10 m pause
  and hide (no simulation, no mesh rebuild); at most 64 `.pcf` map effects
  run at once (nearest first); sparks are capped at 96 and steam puffs at
  120 per emitter.

Verify in the real game (`ent_list info_particle_system`, `ent_list
env_spark`, `ent_list env_steam` in the console list them; `fire <name>
Start`/`Stop` to poke one; `cl_showfps 1` for cost):
1. sp_a1_intro1: the dust/debris effects in the relaxation vault and the
   container ride run (look for floating dust in light beams and falling
   debris); nothing piles up after a minute.
2. sp_a2_bts1: the broken-panel sparks spark at random with a crackle
   (`DoSpark`), streaks fall to the floor; glow flashes where flagged.
3. sp_a2_laser_intro: the chamber's ambient effects (and anything at the
   laser emitter/catcher) start with the map; nothing drawn at the wrong
   place or with wrong orientation (CP0 axes).
4. sp_a3_01: the big-room effects (falling water/dust, steam) run and stop
   when the map stops them; turning away and back resumes them.
5. Effects on moving things (parented) follow them; control points on
   other entities (beams between two points) span the right points.
6. Frame rate with many effects in view stays near what it is without them;
   tell us the worst map.
7. Log: `p2_fx: <system> uses unsupported ...` lines (see above), and any
   `particle system X not found` for a map's `effect_name`.



## Fix notes moved from TASKS.md (2026-10-20)

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

### T-104 sp_a2_intro: the incinerator drop  (P1, DONE: owner playtest 8, the fall plays and lands right)
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

### T-124 sp_a1_intro7: Wheatley doesn't fall off the management rail  (P1, NEEDS_VERIFICATION)
- Owner playtest 4: after the portal behind him (T-122 works), he never drops.
- Cause: the drop is GLaDOS's scene table (sp_a1_intro7OnThree04 fires
  `@sphere ClearParent`, the impact trigger, the detach spark); cores spawn
  kinematic and ClearParent didn't free them. Now ClearParent makes a core
  dynamic unless DisableMotion (or spawnflag 8) froze it.
- Autopilot: sp_a1_intro7_OnThree() -> chain of 4 lines -> Wheatley falls
  from z 1419 to the floor (1239), sphere_impact_trigger fires.
- Owner playtest 5: he falls and can be carried and plugged in (verified).

### T-128 sp_a2_laser_stairs: the staircase doesn't move after the puzzle  (P1, NEEDS_VERIFICATION, owner replay)
- Owner section 7: unplayable past this point.
- Cause: the steps are brushes on circle_shield arms. At spawn each arm plays
  move03_NNdown and holds it; relay_raise_stairs (catcher_1 OnPowered) only
  sends SetPlaybackRate -1 to play the held sequence backwards. A finished
  sequence ignored rate changes, so the arms never moved; only the invisible
  func_door step clips rose (why the earlier autopilot run "climbed" them).
  Now a rate that runs back into a held sequence resumes it
  (Playback::set_rate; unit test negative_rate_replays_a_held_sequence_backwards).
- Autopilot: relay_raise_stairs -> the arms lift the steps into a staircase
  (side screenshots before/after); the 1faf8d2 app shows them flat. Real
  laser route not replayed; the owner reached the catcher.



## Owner-confirmed fixes, playtest 9 (2026-10-20), moved from TASKS.md

#### T-131 sp_a1_intro3: floors and roofs missing  (P1, BLOCKED: not reproduced, needs owner screenshots)
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

#### T-133 sp_a1_intro7: Wheatley floats after the wall panel moves; the panel disappears  (P2, fixed 2542009, autopilot only)
- Cause: DisableMotion (socket_powered_rl) took Wheatley out of the
  player's hands but kept the carry's GravityScale(0); when @plug_eject_rl
  ran EnableMotion/ClearParent he went dynamic with no gravity and hung at
  the socket. Fix: a locked core gets GravityScale(1) back (props.rs
  `locked_core`, unit test). Evidence (autopilot: forcepickup, socket
  powered, eject): before z 1265 at t+9 s (floor 1232); after z 1249 (resting).
- Panel vanishing NOT REPRODUCED: after the open, heavydoor disables and
  heavydoor_ghost is enabled and drawn (info). Owner: screenshot if it recurs.

#### T-134 sp_a1_wakeup: the rising pod's floor turns, the player doesn't  (P2, fixed 5cfe749, autopilot only)
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

#### T-136 sp_a1_wakeup end: a portal shows in the tube before GLaDOS drops the player  (P2, fixed 3f96088, autopilot only)
- Playtests 7 and 8. Cause: relay_incinerator_open links
  incinerator_entry_portal_1 (floor under the incinerator door) to _2 (top
  of the tube); our linked_portal_doors were drawn like gun portals (oval
  flame shader + blue particle rim + hum). Portal 2 draws them as a plain
  window. Fix: SeamlessPortal marker; the surface shader draws only the view
  across the quad, p2_fx skips rim and hum (unit test in p2_fx effects).
  Evidence: autopilot `input relay_incinerator_open trigger` + goto/lookat
  the door: blue oval ring before (dist 8f8c8b1), continuous tube after.
  Owner: replay the drop after GLaDOS's scene (also the finale's doors).

#### T-137 Portals can't be placed as close together as in Portal 2  (P2, fixed 38c09bf, autopilot only)
- Section 5. Cause: a shot overlapping the other portal on the same wall
  failed with Overlap (fizzle); Portal 2 bumps the new portal off it so they
  sit side by side. Fix: rules::bump_off_portals pushes it to the nearest
  free side (1 u gap, still fitted to the surface, within the max slide).
- Evidence (sp_a2_laser_intro, blue at (-35,-256,173), orange aimed 42 u
  beside it): before "Orange shot failed: Overlap"; after "Orange portal
  placed at (30,-256,186)", both visible side by side. Gap size and which
  side wins are our choice, not checked against Windows Portal 2.

#### T-102 Cubes through portals not seamless  (P2, fixed 847a581, autopilot only)
- Sections 1-2 (intro2, intro3). Traversal itself works (owner replays).
- Cause: only the player body had exit-side ghosts; a cube half through a
  portal showed nothing coming out of the linked one. Props now get a mirror
  of their ValveSurface draws through the pair while in the opening
  (prop_ghosts.rs); unit test a_cube_in_the_opening_gets_a_ghost_out_of_the_exit.
- Evidence (intro2, red_1 + blue_1/blue_3 activated, cube propto'd half
  through red_1, view of the blue exit): before no cube at the exit, after
  the cube's half sticks out of the blue portal. Owner: carry/throw cubes
  through portals in intro2-intro4 and check for seams or a pop at the swap.

#### T-123 Portal shots should knock security cameras off  (P2, FIXED 2026-10-20, 2bb0015, autopilot only)
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

#### T-160 Main menu like Portal 2's  (P2, FIXED 2026-10-20, ab64b84, autopilot only)
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

#### T-161 Menu background video and music  (P3, FIXED 2026-10-20, f6f129e, autopilot only)
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

#### T-020 sp_a1_intro1 ride  (P3, BLOCKED: needs owner)
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

#### T-115 intro1: no wake/sleep animation on the bed  (P3, BLOCKED (needs owner))
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

#### T-157 Transition into the sp_a2_intro fall not smooth  (P3, PARTIAL 2026-10-20, badb7a9, autopilot only)
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

#### T-165 sp_a1_intro7: Wheatley's rail connector invisible  (P3, BLOCKED: needs owner)
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

#### T-155 Lasers look poor  (P4, FIXED 2026-10-20, 014ab6e, autopilot only)
- Playtest 8 section 6: "lasers ugly".
- Cause: env_portal_laser legs were drawn as an opaque, hard-edged solid
  cylinder plus an additive red tube, not Portal 2's beam material
  (client.dll C_PortalLaser uses `sprites/purplelaser1.vmt`: UnlitTwoTexture,
  additive, a thin white-pink core with a soft red falloff across u).
- Fix: one camera-facing additive quad per leg, 40 units wide, textured
  with purplelaser1, colour x2.5 overbright, flickering 0.9..1.05 per frame
  (the vmt's UniformNoise proxy). Turret sight lines unchanged.
- Evidence (autopilot shots, sp_a2_laser_relays `3 setpos -420 -460 0;
  3.5 lookat new_laser` and `setpos -600 -700 0`): before a flat solid
  red/white band; after a glowing core with a soft red halo. Width and
  brightness chosen by eye, not compared with Windows Portal 2.
- Not done: the scrolling `$texture2` (effects/laserplane_atmosphere),
  impact sparks (m_bShouldSpark) and emitter/impact glow sprites.

#### T-143 intro1 12:00 clock: the whole sign blinks, only the digits should  (P4, TODO)

#### T-166 intro5: a missing wall panel  (P4, TODO)

#### T-125 sp_a1_intro7: the turn-away check is slow to register  (P2, BLOCKED: not reproduced, needs owner)
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
