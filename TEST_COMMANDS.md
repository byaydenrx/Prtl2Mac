# Test commands

Paste one of these into Terminal.

Build, test, package (from `~/Documents/Claude/Portal2-wheatley`):

    ~/.cargo/bin/cargo build --release -p p2_game
    ~/.cargo/bin/cargo test --release --workspace
    sh scripts/make_app.sh    # dist/Prtl2Mac.app; build id in Contents/Resources/BUILD.txt

Play the packaged app (finds Portal 2 by itself; launched with `open`, no env vars):

    open ~/Documents/Claude/Portal2-wheatley/dist/Prtl2Mac.app

sp_a1_intro1 ride end (T-101): logs the wall-break chain; the ride starts at 3 s, runs at 4x and quits at 200 s (run 2026-10-19):

    cd ~/Documents/Claude/Portal2-wheatley && NO_COLOR=1 RUST_LOG=warn,portal2::io=debug P2_SCRIPT="3 input @rl_container_ride_second_section trigger; 4 con host_timescale 4; 200 quit" ./target/release/portal2 sp_a1_intro1 2>&1 | grep -i "exit_wall\|modify_collision\|third_section"

Entity I/O logging: the binary's log target is `portal2` (`RUST_LOG=warn,portal2::io=debug`), not `p2_game`.

Main menu (Options now has the new video settings and a Subtitles page):

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2

Blue gel (repulsion): sp_a3_jump_intro paints its floors at the start; walk onto the blue to bounce, hold crouch to stop:

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a3_jump_intro

Orange gel (propulsion): press the sprayer buttons in sp_a3_speed_ramp and run across the orange:

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a3_speed_ramp

Look triggers: in sp_a2_bridge_intro Wheatley's peek fires when you look at him, not just walk past:

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a2_bridge_intro

Level scripts end to end: finish sp_a1_intro2 and ride the elevator; the game's own transition script takes you to sp_a1_intro3:

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a1_intro2

sp_a1_wakeup: you start carrying Wheatley; the gantry door, his lines and the breaker room work. Plugging him into the socket doesn't work yet (see docs/STATUS.md):

    cd ~/Documents/Claude/Portal2-wheatley && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a1_wakeup

The end of sp_a1_wakeup without the socket (GLaDOS wakes, the claw takes you to the incinerator, and sp_a2_intro loads):

    cd ~/Documents/Claude/Portal2-wheatley && P2_SCRIPT="4 setpos 9016 1106 400; 5 input basement_elevator_complete_rl trigger; 150 quit" PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a1_wakeup

Any other map works the same way: put its name at the end (sp_a1_intro1, sp_a1_intro4, sp_a2_laser_intro, ...).
