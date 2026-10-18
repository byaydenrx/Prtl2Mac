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

## cloud/tools branch (untested in game)

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

Verify in the real game (`P2_SCRIPT` steps can run console lines with
`con ...`; `click` names: `Toggle(DevConsole)`, `Toggle(ShowFps)`,
`Toggle(ShowPos)`, `SpeedrunOptions`, `Toggle(SpeedrunTimer)`, `ModsOptions`,
`ModToggle(0)`, `ModRollback`, `Adjust(ModProfile,1.0)`,
`Adjust(SpeedrunMode,1.0)`, `ChapterTab(true)`, `CommunityMap(0)`):
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
- Seen while reading `ui.rs`: Options opened from the pause menu unpauses the
  world (`pause_world` only pauses on `Screen::Pause`). The timer ignores
  that (it only runs on `Screen::None`), but the level keeps running behind
  Options.
