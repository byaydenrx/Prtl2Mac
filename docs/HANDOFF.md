# Handoff (latest session update)

Updated 2026-10-18 ~19:45 EDT by the local session ("Portal 2 native Silicon Mac").

## Priorities (owner)
Chapters 1-2 fully playable and finished first, then the next chapters, then
polish. "The definitive Portal 2 for Mac", as close to the original as
possible. New options only inside Settings (no extra main/pause menu buttons).

## Done this session (all on main)
- cloud/ui, cloud/tools, cloud/anim, cloud/proxies, cloud/ui round 2 merged.
- Menus restyled to Portal 2's look (light dialogs, title tabs, hover-only arrows);
  Speedrun/Mods/Community pages in the Options flyout.
- Wheatley: `Animator::load` merges `$includemodel` animation models
  (personality_sphere_animation.mdl has his 320 sequences); cores idle on
  sphere_idle_neutral; his eye glows (LightedMouth) and pulses while he talks
  (scenes.rs `speaking_lights`); VCD `sequence`/`gesture` events play on actors.
- env_texturetoggle inputs now reach texture_toggle.rs (indicator lights).
- Auto-exposure ON by default (exposure.rs): a 64x36 probe camera + GPU readback
  measures the view; TARGET 0.08 matched to reference footage. P2_NOAUTOEXPOSURE
  or P2_EXPOSURE=n to override. P2_TRACE logs it.
- Movie screens (movies.rs): vgui_movie_display plays the game's Bink movies via
  ffmpeg (Homebrew) decoded to ~/Library/Application Support/Portal2Rust/movies;
  the elevator video script (videos/video_splitter.nut) drives them. Screens are
  lifted 8 units in front of their elevator_screen panels.
- Intro1 verified with autopilot through the wake-up exercises, bed, second
  wake-up and Wheatley's knocking/nagging (door does not open early).

## Known problems / next steps (in order)
1. Chapter 2 exits: ALL chapter 1-2 (and 3) maps reach their next map when the
   player follows the real path. The earlier "no exit" results were the sweep's
   fault: departure cars only arrive after the player walks through
   `departure_elevator-source_elevator_door_open_trigger`, and turret_intro exits
   through a walk-in `transition_trigger` whose volume is not at its origin
   (use autopilot `info <name>` to print an entity's bounds). sp_a2_turret_tower
   doesn't exist (ring_around_turrets is the real map). Proven sequences:
   catapult_intro `3 setpos -64 -1512 -400; 9 setpos -64 -2126 -575`,
   sphere_peek `3 setpos -544 1176 500; 9 setpos -544 562 316`,
   laser_relays `3 setpos -320 -1192 52; 9 setpos -320 -1806 -132`,
   turret_intro `3 setpos -336 392 -250`.
   Faith plates checked with scripts/check_faith_plates.py: catapult_intro,
   trust_fling and pit_flings plates land within 6-60 units of their targets;
   plates with usethresholdcheck=1 (pit_flings catapult_1/2, trust_fling
   cube_button_*, wallpad_2) correctly only redirect things arriving at speed.
   NEXT: verify each chamber can actually be SOLVED (exit doors open from the
   puzzle: lasers into catchers, buttons, catapults, fizzlers), map by map in
   play order: sp_a2_intro, laser_intro, laser_stairs, dual_lasers,
   laser_over_goo, catapult_intro, trust_fling, pit_flings, fizzler_intro.
2. Wrecked relaxation vault (intro1) is still dark: it and most elevator areas
   are lit by env_projectedtexture (shadowed spotlights) that we don't render.
   cloud/projtex was asked to implement them; verify and merge.
3. User reports still open: missing buttons in the gel rooms (orange/blue gel),
   black elevator in later (gel) chapters, cube carry feel.
4. refract shader (glass) not translated: drawn as a faint pane.
5. Wheatley look-at during scenes, sequence blending: cloud/anim round 2.

## Cloud sessions (each pushes its own branch and documents checks in STATUS.md)
- cloud/anim: crossfades, gesture layers, VCD lookat (Wheatley aims at player).
- cloud/gels: paint on props, cleansers, paint bombs, bounce/speed tuning.
- cloud/projtex: env_projectedtexture lights with shadows.
- cloud/saves: autosave/quicksave/load, Continue/Load Game menu entries.
- cloud/ui: round 2 done and merged.

## Debug switches added
P2_HIDE_MAT=a,b (skip model+brush materials containing a/b), P2_LIST_MATS,
P2_TRACE_MESH=model, P2_TRACE_BONE=bone, P2_NOCROSSHAIR, P2_PROBE_DUMP=file.ppm,
`ls --vtf path out.ppm`, `animinfo model seq` with BONE=name.

## Test commands
```
cd ~/Documents/Claude/Portal2-rust-engine && git pull && cargo build --release
export PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621"
./target/release/portal2                    # menus
./target/release/portal2 sp_a1_intro1       # wake-up, Wheatley
./target/release/portal2 sp_a1_intro4       # elevator video screens
./target/release/portal2 sp_a1_intro7       # Wheatley's eye
```
Autopilot (scripted run): `P2_SCRIPT="3 lookat @sphere; 4 shot /tmp/x.png; 5 quit" ./target/release/portal2 sp_a1_intro7`
(commands: setpos, look, turn, walk, back, left, right, duck, jump, use,
fire blue|orange, input <target> <input> [param], ray, propto, goto, lookat,
shot, log, set ..., click <Action>, quit).
