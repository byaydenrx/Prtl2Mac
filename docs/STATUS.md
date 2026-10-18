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
`ray x y z` logs what is below a Source point, `click <Action>` presses a
menu button by its action (`click pause`, `click Options` (opens the
flyout), `click Page(AdvancedVideo)`, `click Toggle(VSync)`,
`click Adjust(Fov,1.0)`, `click Done`, `click SinglePlayer` then
`click Continue`/`click NewGame`; Quit and Quit to main menu need
`click ConfirmOk`; an unknown name logs the buttons on screen). The script clock is real time (menus pause the world).
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

Menus (main, chapter select, pause, Portal 2's options with the extras;
see the cloud/ui section below), autosave/continue, campaign order + chapter
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

## Next (priority order)

1. **sp_a1_intro1 wake-up camera** (`viewcontrol.rs`): ghostanim `sp_a1_intro`
   attach_1 now decodes smoothly (`animinfo`), but which bone axis is the
   view and whether the 48S quaternion sign is right isn't confirmed. Start
   should lie on the bed looking at the ceiling, end standing facing the room
   (+X, like info_player_start). Enable with P2_CINEMATIC=1 when right.
2. **Wheatley in sp_a1_intro1**: `@sphere` (npc_personality_core) is parented to
   `bottom_swivel_1`; his rail/arm props animate. Needs: drawing animated
   props (skinned playback, `p2_assets::anim::Animator`), parenting children
   to animated attachments (SetParentAttachment) each frame, npc model drawn.
   His knock/nag lines: `sphere_door_knock_nag_timer` -> `sphere_door_knock_case`
   -> knock relays (check their ambient_generic messages). Many @glados
   `runscriptcode` calls name functions that don't exist in the shipped
   scripts (e.g. `sp_a1_intro1_open_door_sequence`) — harmless no-ops.
3. **Container ride**: `actor_container_master` (prop_dynamic_override) animates
   the vault through the facility; vault brushes/props are parented to its
   attachments. Same need as 2 plus carrying the player inside.
4. Instructor hint icons (`hint_icon_onscreen` -> materials/vgui/hud/…),
   refract/glass shaders (vault window), env_tonemap, sounds on prop events.
5. Play on through the chapters with P2_SCRIPT runs, fixing whatever blocks
   progress. Scripts now really run (map RunScriptCode used to reach them
   lowercased), so watch for new behaviour in GLaDOS/Wheatley sequences.
6. Gels, next: measure the real bounce/speed tuning, gel on props (cubes
   with `paintpower`, prop_paint_bomb), splat look (soft edges, lighting),
   trigger_paint_cleanser, paint_sphere (finale). Blobs through portals and
   white gel taking portals are written but not yet tried in a map.
7. trigger_look's line-of-sight flag (128) isn't checked.
8. Missing: npc_wheatley_boss (finale), func_rotating, logic_playerproxy,
   env_global, Wheatley's rail animation (item 2).
9. Settings: with v-sync off and no frame cap, a background test window ran
   at 60 fps (95-103 with a 120/240 cap); check in a focused window.
   Enhanced graphics does nothing yet; model textures ignore the AF setting.

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

## Settings (merged from cloud/settings, tested 2026-10-18)

Tested in the game with scripted menu clicks: Options fits at 1280x720 in
two columns, every `<`/`>`/toggle changes its value and applies live
(FOV 110 widens the view, MSAA off, 8x AF, 120 cap, v-sync off), the
Subtitles page opens and BACK returns to Options then the pause menu, the
user's old settings.cfg loads, and settings.cfg/save.txt are written as
`version 2` (old save files still Continue; unit tests cover both). Not
checked: exclusive fullscreen resolutions, Esc on the Subtitles page.

## cloud/ui branch (untested in game)

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
