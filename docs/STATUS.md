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
`ray x y z` logs what is below a Source point. `P2_TRACE=1` also logs
laser legs and catapult launches.

Tools: `cargo run -p source_assets --example ents -- MAP [class|targetname|*N]`,
`--example ls -- PREFIX` / `-- --cat PATH` / `-- --fmt PREFIX`, `--example pak -- MAP`,
`--example probe -- MAP x,y,z`, `cargo run -p p2_assets --example animinfo -- MODEL SEQ ATTACH`.

## Works

Menus (main, chapter select, options incl. FOV/sensitivity/invert/smoothing/
subtitles/fullscreen, pause), autosave/continue, campaign order + chapter
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
reflection cubes, catchers, relays, burn).

Level scripts: `crates/vscript` is our own Squirrel interpreter (lexer,
parser, tree-walker; parses all 82 shipped `.nut` files). `vscripts.rs`
loads every entity's `vscripts`, runs Precache/OnPostSpawn, think
functions, RunScriptCode/CallScriptFunction, ConnectOutput hooks, and
answers EntFire/CreateSceneEntity/Entities.*/handle methods. Entities whose
scripts loaded skip the old hand-written runscriptcode stand-ins.
`P2_NOSCRIPTS=1` turns scripts off. Known gap: video_splitter calls
GetMapIndexInPlayOrder (defined in the transition script's scope).

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
5. Play on through sp_a1_intro2..intro7 with P2_SCRIPT runs, fixing whatever
   blocks progress (map wiring is mostly standard).
6. Missing puzzle elements, by how many campaign maps use them:
   prop_wall_projector (light bridges, 7 maps), prop_tractor_beam (12),
   npc_portal_turret_floor (16), info_paint_sprayer gels (12),
   npc_personality_core / npc_wheatley_boss (finale), func_rotating,
   logic_playerproxy, env_global, trigger_look (currently fires on touch).

## cloud/settings branch (untested in game)

Settings from docs/ROADMAP.md, all inside the existing Options screen (no new
main/pause menu buttons). Options is now two columns: AUDIO (volumes, a
SUBTITLES row that opens its own page) + MOUSE, and VIDEO. Defaults equal the
old behaviour (FOV 75, v-sync on, unlimited frame rate, MSAA 4x, 16x
anisotropic, native/launch resolution, Original graphics, captions 20px in
their own colours on a 60% box). Only `cargo check` / `cargo test` were run:
no game assets or Mac GPU in the cloud. Verify in the real game:

- [ ] Options fits at 1280x720 and fullscreen; every `<`/`>` and toggle
      changes its value; Esc/BACK from the Subtitles page returns to Options
      (from the pause menu too, with audio still paused).
- [ ] FOV goes 75..110 in steps of 5 and the view widens; viewmodel unchanged.
- [ ] VERTICAL SYNC off uncaps the frame rate live (AutoNoVsync); on is
      Fifo. No tearing/stutter regressions with it on.
- [ ] MAX FRAME RATE 30/60/120/144/165/240/UNLIMITED: frame times are even
      (sleep + spin limiter in `settings.rs`, `Last` schedule); with v-sync
      on, a limit above the display rate changes nothing; CPU use reasonable.
- [ ] RESOLUTION: windowed sizes the window (in points, `resolution.set`);
      DEFAULT goes back to the launch size; the player's own resizing is
      kept. Fullscreen with a size uses the monitor's exclusive video mode
      of that size (macOS exclusive fullscreen is the part most likely to
      misbehave); NATIVE/no matching mode is borderless as before.
- [ ] ANTI-ALIASING OFF/MSAA 4X switches live on the main and viewmodel
      cameras; world (valve_render pipelines are keyed by sample count),
      portals, viewmodel and UI still draw.
- [ ] TEXTURE FILTERING OFF/4X/8X/16X changes world surfaces live
      (`source_assets::valve_render::ValveQuality` rebuilds the sampler
      table). Model textures (`vtf.rs`) still always use 16x: TODO there.
- [ ] GRAPHICS ORIGINAL/ENHANCED only stores the choice and sets
      `ValveQuality::enhanced`; nothing renders differently yet.
- [ ] SUBTITLES page: on/off, text size, colour (ORIGINAL keeps the game's
      per-speaker colours), background opacity; the sample line and live
      captions follow them.
- [ ] settings.cfg now starts with `version 2`; an old file loads with its
      values; unknown lines are kept. save.txt is now `version 2` + `map X`
      (an old map-name-only file still works for Continue; a build from
      before this branch cannot read the new save.txt).
