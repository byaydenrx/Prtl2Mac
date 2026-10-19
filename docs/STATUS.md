# p2_game status and handoff notes

The `p2_game` crate (binary `portal2`) plays Portal 2's own levels from the
player's install, natively on Apple Silicon (Metal), on top of the Portal 2
Rust Engine crates. No Valve code: behaviour comes from the levels' own
entity wiring, the game's text scenes/scripts/captions, public file formats
and observation. Never use leaked Valve source (the owner has some in
~/Downloads/game — do not open or use it) or decompile Valve binaries.

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

## Next (priority order)

1. **sp_a1_wakeup socket** (above): find why held Wheatley can't reach the
   socket trigger, then carry him up to the opened cabinet and check
   `socket_powered_rl` fires and the platform rises with the player on it.
2. **sp_a1_intro1 wrecked vault / container ride**: the finedebris_part
   models (170 bones) need per-bone playback to look right; the vault is
   dark (env_projectedtexture, see HANDOFF.md).
3. Chapter 2 in play order: solve each chamber with autopilot (HANDOFF.md
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
