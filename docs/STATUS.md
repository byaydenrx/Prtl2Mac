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
menu button by its action (`click pause`, `click Options`,
`click Toggle(VSync)`, `click Adjust(Fov,5.0)`; an unknown name logs the
buttons on screen). The script clock is real time (menus pause the world).
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

## Settings (merged from cloud/settings, tested 2026-10-18)

Tested in the game with scripted menu clicks: Options fits at 1280x720 in
two columns, every `<`/`>`/toggle changes its value and applies live
(FOV 110 widens the view, MSAA off, 8x AF, 120 cap, v-sync off), the
Subtitles page opens and BACK returns to Options then the pause menu, the
user's old settings.cfg loads, and settings.cfg/save.txt are written as
`version 2` (old save files still Continue; unit tests cover both). Not
checked: exclusive fullscreen resolutions, Esc on the Subtitles page.
