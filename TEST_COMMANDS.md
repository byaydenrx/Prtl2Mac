# Test commands

Paste one of these into Terminal.

Main menu (Options now has the new video settings and a Subtitles page):

    cd ~/Documents/Claude/Portal2-rust-engine && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2

Blue gel (repulsion): sp_a3_jump_intro paints its floors at the start; walk onto the blue to bounce, hold crouch to stop:

    cd ~/Documents/Claude/Portal2-rust-engine && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a3_jump_intro

Orange gel (propulsion): press the sprayer buttons in sp_a3_speed_ramp and run across the orange:

    cd ~/Documents/Claude/Portal2-rust-engine && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a3_speed_ramp

Look triggers: in sp_a2_bridge_intro Wheatley's peek fires when you look at him, not just walk past:

    cd ~/Documents/Claude/Portal2-rust-engine && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a2_bridge_intro

Level scripts end to end: finish sp_a1_intro2 and ride the elevator; the game's own transition script takes you to sp_a1_intro3:

    cd ~/Documents/Claude/Portal2-rust-engine && PORTAL2_DIR="$HOME/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621" ./target/release/portal2 sp_a1_intro2

Any other map works the same way: put its name at the end (sp_a1_intro1, sp_a1_intro4, sp_a2_laser_intro, ...).
