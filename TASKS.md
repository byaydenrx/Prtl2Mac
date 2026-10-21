# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (WORKFLOW.md). Work the "Priority order" top to bottom; never P3/P4
while a P0/P1 is open. One entry per task: fold new reports into it. When a
task is fixed, give its cause and evidence in two or three lines, move it
to "Awaiting owner replay"; once the owner confirms, move it to Done. Long
investigation notes go in docs/STATUS.md; the owner's raw reports in
docs/PLAYTESTS.md (history only, everything there is tracked here).

"Prove a map" (acceptance for T-002..T-015): from the map's start, an
autopilot script plays the intended solution with movement, `fire`, `use`
and props (no `setpos` past a puzzle step, no `input` on puzzle entities);
the exit opens from the puzzle's own outputs; the next map loads. Save the
script as `scripts/routes/<map>.txt`. Engine bugs found become tasks.

Owner playtest sections (playtest 8): 0 intro1, 1 intro2, 2 intro3, 3 intro4,
4 intro5/intro6, 5 intro7 + sp_a1_wakeup + sp_a2_intro, 6 sp_a2_laser_intro,
7 sp_a2_laser_stairs.

## Priority order (2026-10-21, owner menu/intro spec first)
MENUS FROZEN (owner, 2026-10-21): the menus/UI are right as they are. Only
T-196, T-195 and T-197 may change menu code, touching just what each needs;
every other menu task (T-162, T-177..T-194, including their "open pieces"
and the dead-control list) is on hold, not for autoruns.
NEXT UP (owner round 4, 2026-10-21): T-196 black screen after flips
(FIXED f8c9be3, awaiting owner replay), then T-195 pause menu as Windows (logo, no title tab, DONE,
blurred/desaturated game behind), then T-197 even tile grid. Then T-172
onward (no more menu work).
OWNER PRIORITY (2026-10-21): the menu/intro spec (docs/MENU_SPEC.md) comes
before every other task. Owner notes after build fb0090a (12 items,
listed under the spec heading) come first: T-179 flip like the video,
T-193 scrolling/arrows, T-194 Options list, T-178/T-185/T-187/T-189/T-177
tweaks, T-192 spacing. Status 2026-10-21 (build cec807a+): all of
T-177..T-192 built, autopilot-checked only, awaiting the owner's replay.
Open pieces: T-188 gamepad input (none at all yet), T-189 raw mouse
input / key conflicts / keyboard scrolling of Edit Keys, T-187 detail
levels not driving the renderer, T-179 flip look vs Windows (no video
reference), T-192 real fullscreen switching, T-182 Windows comparison.
Then the list below.
P0:
1. (none; T-168 box-button crash owner-confirmed fixed.)
P1:
2. (none; T-176 fixed 2bf3d20 and T-169 fixed a1cd026, awaiting owner replay.)
P2:
   (T-162 UI styling: every screen done 2026-10-20, awaiting owner replay.)
   (T-170 carrying at walls fixed 9f1ed5a, awaiting owner replay.)
   (T-145 lit cubes black still open, cause not found; companion/rusted lit skins merged cc8d8e0; spikes/jitter still open.)
3. T-172 voice lines start late (stretches elevator rides; T-120's VO part).
4. T-135 things fall through elevators (turrets, rubble).
5. T-139 music not playing with ambience; soundscapes.
6. T-112 GLaDOS chamber floor still murky brown (with T-151 water).
7. T-144 clipboard should turn to face the player.
8. T-149 intro5 screens/lights; sign big number check.
9. T-147 intro2 black screen at the first door; T-113 hints vanish.
P3:
10. T-171 FPS drops looking at portals.
11. T-148 portals behind glass drawn on top (reopened).
12. T-117 shadows still blocky.
13. T-152 lighting/texture quality.
14. T-153 GLaDOS wake-up particles; T-173 gun hold beam and fizzler spark.
15. T-140 game-placed portal shake/sound (reopened).
16. T-174 elevator videos; T-175 intro1 ride outside fog/sky; T-164 debris
    (try attempt/T-164); T-116 window (owner screenshots coming).
P4:
17. T-151 water (try attempt/T-151), T-150 particles, T-146 half-stuck door,
    T-154 floating moss.
Then the Backlog (prove and sweep the campaign).

## Owner menu/intro spec (2026-10-21): T-177..T-192, before everything else
- Owner notes 2026-10-21 round 4 (build d36e250), not worked yet:
  (1) the screen turns black after each flip animation plays, caused by
  the round-3 fix (T-196); (2) the in-game pause menu is inaccurate; owner
  reference shots in ~/Documents/Screenshots/pausemenu (2704x1756 CrossOver
  window, a different size/aspect from the earlier shots) (T-195);
  (3) grid/box inaccuracies: tile spacing off, some tiles shifted a few
  pixels sideways instead of one flat, even grid (T-197); (4) the paused
  game behind the pause menu should be blurred/bloomed and slightly
  desaturated (T-195).

### T-196 Screen goes black after every tile flip  (P1, FIXED f8c9be3, awaiting owner replay; owner round 4 item 1)
- FIXED f8c9be3 (build 0.1.0+f8c9be3.20261021T1212Z), autopilot-checked
  only. Cause: when a flip's time was up, run_flip despawned the overlay
  in the same frame it asked rebuild_menu (which runs before it, so the
  next frame) to build the menu again on the window; for that one frame
  the menu was still drawn off screen and the overlay was gone, so the
  dialog and menu items (at startup also the logo/backdrop: the whole
  screen) went black. Fix: the overlay stays until the rebuilt menu is on
  the window (FlipView.rebuilding; flip_end() decides, unit test
  flip_end_keeps_the_overlay_till_the_menu_is_back). Evidence: shots every
  12 ms across the Options flip's end: before (d36e250 dist) one frame
  with the dialog area at brightness 20 vs 173 (menu gone), after none.
  Checked after the fix, same as before: Esc mid-flip lands on the main
  menu, Options -> Keyboard/Mouse -> Esc -> Options list, Single Player
  and back; windowed and fullscreen (temporary local hack, not kept).
  The startup full-screen flip's end frame wasn't caught by the burst in
  either build (frames too sparse while loading); same code path.
  Owner: please replay a few flips (startup, Options, pages, Back).
- Regression from d36e250 (round-3 fix in ui.rs run_flip): when a flip
  ends, the overlay is despawned and MenuState is marked changed so
  rebuild_menu respawns the roots on the window. Owner sees black after
  every flip. Suspects: the rebuild's roots get tagged UiTargetCamera
  again because `active` (now < until || pending_screen) is still true on
  that frame, or pending_screen never clears, or the off-screen camera
  stays active/default; the old roots were drawn off screen and the
  despawn/rebuild ordering leaves nothing on the window. Autopilot shots
  after flips looked fine, so reproduce with the real app path (intro on,
  window focused) and with dialog flips (Options, Single Player, back).
  Fix so menus are always on the window once a flip ends; unit/autopilot
  check several flips in a row, Esc mid-flip, flip into a page then Back.

### T-195 In-game pause menu as Windows  (P2, TODO; owner round 4 items 2, 4)
- Reference: ~/Documents/Screenshots/pausemenu (2 shots: the pause menu
  and Options from it). In game Portal 2 shows the PORTAL 2 logo top left
  (ours shows none in game); the pause dialog has no title tab, a flat
  5 x 4 tile body with RETURN TO GAME, SAVE GAME, LOAD LAST SAVE,
  OPTIONS, EXIT TO MAIN MENU, and a DONE footer button; Options from
  pause is the normal Options dialog with the logo. The game view behind
  is blurred with a bloom/glow and slightly desaturated (ours: a flat dark
  shade). Measure positions against the shots (note their size differs
  from the earlier ones: scale by the 16:9 game area inside the window).
  Keep: no tile flips in pause menus (round 3).

### T-197 Tile grid not even  (P2, TODO; owner round 4 item 3)
- The dialog tiles' spacing is off and some tiles sit a few pixels to the
  side, so the grid isn't one flat even grid. Likely tile_frame_sized's
  pulled-in last column/row (x = width - 50 overlaps) and rounding of
  units (Vh) to pixels per tile; build each frame from whole-pixel tile
  positions on one grid (compute pixel size once, place by integer
  multiples), and check every dialog (Options, Audio, Advanced Video,
  Edit Keys, confirmations) against the owner's Windows shots.

- Owner notes 2026-10-21 round 3 (build ac6a02d, "looking great"): (1) drop
  the startup loading screen: straight from the "powered by Source" screen
  to the main menu (T-177); (2) the full-screen tile flip into the main
  menu is still the old animation: use the same turn as the menus, full
  screen (T-179); (3) no tile animation in the in-game (pause) menus
  (T-179); (4) "Chapter 1 - The Courtesy Call": a dash between chapter and
  name on Load/Save Game and New Game (T-182/T-183).
- Owner notes 2026-10-21 (after build fb0090a), the owner's words, filed:
  (1) tile animation should match https://www.youtube.com/watch?v=qtm2U-M-lHA
  (first 2-3 s), with its sound, and be basically instant, not waiting
  for the tiles to finish and then the menu fading in (T-179);
  (2) scrolling breaks: scrolling past the end keeps scrolling "behind the
  scenes", so you must scroll back the same amount before it moves up
  (repro: Edit Keys, scroll far down, then up) (T-193);
  (3) scrollbar arrows and the slider default arrow render very pixely
  (T-193); (4) the arrows shown when hovering an option render as empty
  "missing glyph" boxes (T-193);
  (5) Options list should be only Audio, Video, Keyboard/Mouse, Controller,
  Mods: remove Speedrun, merge the subtitle options into Audio, grey out
  Mods and make it unhoverable (T-194);
  (6) spacing still feels off; it should scale correctly to different
  aspect ratios and resolutions (T-192);
  (7) move Robot Enrichment down a little; its text aligned with the rest
  of the menu, the icon to the left of it (T-178);
  (8) grey out Setup Microphone, Enable Voice and Push To Talk Key for now
  (T-185); (9) remove Graphics Original/Enhanced from Advanced Video
  (T-187); (10) Edit Keys/Buttons' hover bar is a different colour from
  the rest of the menu (T-189); (11) hide the cursor during the intro logos
  and loading screens, back with the menu (T-177); (12) list every control
  that isn't greyed out but doesn't work or can't be tested (answered in
  chat, kept in T-194).

### T-193 Scrolling and arrow glyphs  (P2, FIXED, awaiting owner replay; owner notes 2026-10-21 items 2-4)
- 2026-10-21 FIXED: the wheel position is clamped to the list's real
  scroll range (it kept counting past the end); every arrow (row ◀ ▶,
  slider marker, scrollbar ends) is now a smooth anti-aliased triangle
  image (the menu fonts have no triangle glyphs -> boxes; the marker was
  stacked rectangles). Autopilot shot: Audio row arrows, slider markers.
- Wheel scrolling past a list's end keeps counting; arrows (scrollbar,
  slider marker) pixely; hover arrows draw as missing-glyph boxes.

### T-194 Options list and dead controls  (P2, FIXED, awaiting owner replay; owner notes 2026-10-21 items 5, 8, 9, 12)
- 2026-10-21 FIXED: Options lists Audio, Video, Keyboard/Mouse,
  Controller and a greyed, unclickable MODS; Speedrun is off the list (its
  settings still read from settings.cfg); Subtitle Size/Colour/Background
  are the last Audio rows (Audio scrolls; Use Defaults resets them too);
  Enable Voice, Push To Talk Key and SETUP MICROPHONE greyed; Graphics
  (Original/Enhanced) removed from Advanced Video. Dead/untestable
  controls listed to the owner in chat 2026-10-21.
- Not greyed but not working yet (2026-10-21): Speaker Configuration
  (mix is always stereo), Sound Quality, Puzzlemaker Sounds, Splitscreen
  Direction, Multicore Rendering, Shader/Effect/Model detail, Paged Pool,
  Filtering Bilinear vs Trilinear (anisotropic levels work), Motion Blur,
  Brightness (stored, renderer ignores gamma), Raw Mouse Input,
  Controller; Edit Keys rows with no game action: Toggle Zoom, Zoom
  In/Out, the co-op five, View Leaderboard, Take Screenshot, Quit Game,
  Swap Splitscreen, Turn Left/Right, Look Up/Down, Strafe and Keyboard
  Look modifiers; Toggle Developer Console and Pause stay on ` and Esc
  whatever is bound. Untested here: exclusive Fullscreen / Windowed (No
  Border) switching and non-native resolutions on a real display.
- Options: Audio, Video, Keyboard/Mouse, Controller, Mods (greyed); no
  Speedrun; subtitle options inside Audio; voice rows greyed; Graphics
  row removed; list what still doesn't work.

The owner's spec, verbatim: docs/MENU_SPEC.md (section numbers below are
its sections). The owner can send Windows screenshots for any screen.
Reference: Windows Portal 2 (install .res files, client.dll in depot 624,
https://interfaceingame.com/games/portal-2/). Supersedes T-162's open notes.

### T-177 Startup intros missing  (P2, FIXED, awaiting owner replay; spec 1, 15)
- 2026-10-21 round 3: no startup loading screen: the menu's map loads
  during the fade after "powered by Source", then the main menu flips in
  (log: "intro done; main menu" at ~17.9 s; shots).
- 2026-10-21 owner note: the pointer is hidden during the intro movie and
  any loading screen and comes back with the menu.
- 2026-10-21 FIXED (autopilot shots + log; new intro.rs). A normal launch
  plays startupvids.txt's media/valve.bik with its own audio (ffmpeg),
  then the Portal 2 loading screen while the menu map loads, fading into
  the main menu. The install's portal2_dlc2/media/valve.bik (17.5 s,
  what Portal 2 itself plays: Valve guy, then "powered by Source" and the
  legal text) wins over the base one by the game's newest-first lookup.
  Any key/click/Esc skips; no movie or no ffmpeg opens the menu at once.
  P2_NO_INTRO=1 (and map/--shot runs) skip it. Shots at 4 s (Valve), 13 s
  (Source), 18 s (loading screen over the menu), 24 s (menu). Not checked:
  skipping by key (autopilot can't press keys), timing vs Windows.
- Play in order, with their music/audio, timing and transitions: Valve logo
  (the head turn), "Powered by Source" screen, the initial Portal 2 loading
  screen, then the main menu. No skip, overlap or repeat.

### T-178 Main menu layout  (P2, FIXED, awaiting owner replay; spec 1, 11, 16)
- 2026-10-21 owner note: Robot Enrichment moved down 7 units (top 386)
  with its label on the items' text edge (x 100) and the icon left of it.
- 2026-10-21 FIXED (autopilot shots only). Measured a Windows 1080p main
  menu (interfaceingame.com) against ours: font size, 30-unit pitch and
  text x already matched; our column sat 30 units low (placed from
  mainmenu.res, whose PC order starts with two hidden community buttons),
  now Play Single Player's bar at 198 and Quit's at 348 (text tops 204.7
  and 354.7 vs 204.4 and 354.7 on Windows). Focus bar now fixed 281 x 25
  units from x 87 on every item, grey ~77 over the backdrop (Windows ~75;
  was alpha 3/255, nearly invisible, sized to the label). Co-op,
  Community, Extras greyed (60 grey, not buttons, so no hover/click; their
  actions removed); Robot Enrichment added under Quit (store_button atom
  icon 17 units at 89,379, label in the 18-tall Cn font), greyed. Logo
  moved 0.7 units so the P lines up with the text (x 100). The logo now
  also shows over every Options page from the main menu (Options pages
  never drew it). Challenge Mode in Single Player greyed too. New
  autopilot `hover <Action>` for hover shots. Keyboard navigation is not
  built at all yet (T-184).
- Robot Enrichment section and its icon missing (greyed out).
- Gap between the Portal 2 logo and PLAY SINGLE PLAYER about one text line
  too big; check spacing and font size against the original.
- Align the logo and the menu items with each other (left edges).
- The logo stays visible on every menu (Options etc.), same position.
- Hover highlight: same size on every item (now it changes colour and
  shrinks to the label); match the original size, position and colour.
- Robot Enrichment + icon, Play Cooperative Campaign, Extras, Community Test
  Chambers: original positions, greyed, not hoverable or clickable, skipped
  by mouse and keyboard.

### T-179 Tile-flip transitions  (P2, FIXED (approximation), needs owner comparison; spec 1, 10, 11, 15)
- 2026-10-21 round 3: the full-screen flip into the main menu (startup,
  back from a level) is now the same turn as the dialogs, each tile a
  slice of the whole menu (backdrop, logo, items), sweeping from the top
  left (0.025 s apart, ~0.8 s), started once the menu's map is in. Pause
  menus (and their Options, save/load, confirmations) open at once. Fixed
  on the way: after a flip the menu is rebuilt on the window (taking the
  off-screen target away left it invisible; screens went black).
- 2026-10-21 REDONE from the owner's Windows recording (frame by frame,
  30 fps; audio cross-correlated with the install's sounds): each 50-unit
  tile turns about its upright axis (squeezes sideways), black back first,
  then its face, which already shows its piece of the dialog (text and
  all); tiles start at the title tab and sweep down/right with a little
  scatter, ~0.14 s each, the Single Player dialog done ~0.43 s after the
  click; no fade afterwards. Sounds: menu_accept on the click, then
  ui_menu_flip_single_01/_02 alternating every ~0.07 s while tiles turn.
  Built by drawing the new dialog off screen (a UI camera into an image)
  and showing slices of it on the turning tiles; the real dialog takes
  over when the last tile lands. Autopilot shot sequence matches the
  recording's look; the logo no longer flips (own layer).
- 2026-10-21 built (autopilot shot sequences only; no Windows video to
  match against, so direction and stagger are a best reading of Portal
  2's look). Every tiled dialog (Single Player, Options and each page,
  Video -> Advanced Video, New/Load Game, quit/discard confirmations)
  turns its 50-unit tiles in about their horizontal middle, one after
  another from the top left (0.20 s each = client.dll's
  ui_transition_time default, 0.03 s apart), then shows its title, rows
  and footer; rebuilding the same dialog doesn't replay it. Entering the
  main menu (startup, or back from a level) turns the whole screen over
  as black tiles from the top left. Sounds: ui_menu_flip_single_01 on a
  dialog, _02 on a confirmation or the whole screen. Owner: compare with
  Windows (direction, speed, closing flips: dialogs don't flip out yet).
- Fullscreen tile flip from the intro screen to the main menu; tile flip
  opening Options, Extras, Community, Play Single Player, Play Co-op; flips
  between subsections (Video -> Advanced Video); dialogs (quit, discard).
  Match direction, timing and look, not a fade/slide.

### T-180 Menu sounds  (P2, FIXED except tile-flip sounds (T-179), awaiting owner replay; spec 1, 11, 15)
- 2026-10-21 FIXED (log-checked only, not listened to). The menus had no
  sounds. client.dll names them: ui/menu_focus.wav when the mouse comes
  onto an item or Options row, ui/menu_accept.wav when one is chosen,
  ui/menu_back.wav for Back/Cancel/Esc. Focus is keyed by the item (a menu
  rebuilt under the mouse stays quiet), rows by their place. New `ui`
  sounds play while the game is paused, at the effects volume. Autopilot
  log: one focus per hover, one accept per click. The flip sounds
  (ui_menu_flip_single_01/02) come with the tile flips (T-179).
- Original hover, click/select, menu/section switch sounds, at the right
  moments, once per interaction.

### T-181 Play Single Player dialog  (P2, FIXED, awaiting owner replay; spec 2, 13, 16)
- 2026-10-21: rows Continue Game, New Game, Load Game, Challenge Mode
  (now greyed, was clickable doing nothing), Developer Commentary (greyed).
  Continue loads the newest save (saves list, else the old save.txt) and
  is greyed without one; the menu movie/background follow that save's
  chapter (menu_act). Not replayed by the owner.
- Rows: Continue Game, New Game, Load Game, Challenge Mode, Developer
  Commentary (last two greyed, not hoverable/clickable).
- Continue loads the most recent valid save at its place and state; the
  menu background video matches that save's chapter; no saves: graceful.

### T-182 New Game chapter list  (P2, UNCHANGED (built under T-162), needs owner check; spec 2, 13)
- 2026-10-21: already as specified from T-162 (list on the left, the
  picked chapter's picture with "CHAPTER N" + bold name beneath, locked
  chapters grey, second click / Start New Game starts that chapter); now
  also flips in (T-179). No Windows New Game shot to compare against yet:
  owner, send one if anything differs.
- Played chapters hover/select properly; each entry shows its screenshot
  with the chapter title beneath (save-menu layout); titles bold; selecting
  starts that chapter.

### T-183 Load Game list  (P2, FIXED, awaiting owner replay; spec 2, 13)
- 2026-10-21 round 3: "CHAPTER 1 - THE COURTESY CALL" (dash) here and on
  New Game.
- 2026-10-21 FIXED (unit test + autopilot shots in a test data folder):
  rows read "Wednesday, Oct 21 3:54 AM" (local time from the save's own
  timestamp; was "OCT 21 2026 00:24"), bold, mixed case as the Windows
  Save Game shot; newest first; "Auto Save" over the picture only for
  autosaves ("Quick Save" for quick saves, nothing on named saves);
  footer Load/Save and Back as Windows writes them; Save Game's new row
  "New Saved Game Slot". Every save now gets its own picture (400 x 225
  JPEG of the game view, saves/<name>.jpg; an autosave's 1.5 s into the
  level, a pause-menu save's as play resumes); saves from before show
  their chapter picture instead. Empty list says so; unreadable saves are
  left out. Chapter line: chapter in Cn + name in BoldCn as before.
- Saves newest first; each with screenshot, save title, chapter name (title
  and chapter bold); date like "Saturday, Jun 27 7:32 PM" from the save's
  real timestamp; "Auto Save" in the screenshot's top-right only for
  autosaves; loads the selected save; all fields from the same save.
- Edge cases: no saves, only autosaves, mixed, missing screenshot, a save
  that fails to load, near-identical timestamps. Never invent data.

### T-184 Options framework: pending/applied, footer, dialogs, navigation  (P2, TODO; spec 3, 11, 12, 14)
- 2026-10-21 pending/applied DONE (unit tests + autopilot shots): Options
  pages edit a working copy; the real settings change (and settings.cfg is
  written) only on Apply, which then returns to the parent page. Footer:
  Done (Audio, Keyboard/Mouse, Controller...) or Back + Use Defaults
  (Video, Advanced Video) until something changes, then Back, Apply, Use
  Defaults. Back/Esc with changes opens "Changes have been made. Discard
  current changes?" (Discard leaves and drops them, Cancel stays with
  them); the page hides under it. Use Defaults resets only that page's
  settings (pending), asking "Reset settings to their recommended
  values?" on the video pages. Edit Keys hands its changes back to
  Keyboard/Mouse; a key row's first click selects, the second (or Edit
  Key) waits for a key. Cancel buttons removed (Portal 2 has none here).
  Subtitle style moved from Audio to its own Options row (SUBTITLES) so
  Audio matches Portal 2's rows. Not done: keyboard navigation/focus,
  confirmation tile flips (T-179).
- 2026-10-21 keyboard navigation DONE (autopilot `key` runs): Up/Down/
  Tab move through the items in screen order (greyed ones are not
  reachable), Enter/Space chooses, Left/Right step a value or slider, Esc
  backs out (discard prompt with changes). The pointer moves onto the
  item, so hover look, sound and Advanced Video's description follow.
  Not done: keys don't scroll a long list (Edit Keys) past what's shown.
  Subtitles and Speedrun pages now use the tiled frame too.
- 2026-10-21 footer buttons DONE (autopilot shots): every tiled dialog's
  footer is now FooterPanel's button: the install's vgui/btn_bg_alt face
  (idle), btn_bg_over (hovered, brighter), btn_bg_click (blue, while held),
  1 px border 130/180, BoldCn 15 black, 10 units padding, 12 apart, at
  y 417; upper case on dialogs, as typed on confirmations. Footer buttons
  now act on release (vgui), so the blue face shows while held.
- Settings functional; shown values = active or pending.
- Use Defaults appears beside Apply and Back once anything changed
  (Advanced Video too), restores defaults; hidden when nothing changed.
- Footer buttons look like the original: brighter on hover/focus, blue
  when clicked; sizes, spacing, focus, transitions.
- Pending vs applied kept apart; persist across sections, reopening and
  restarts; bad/missing settings file -> defaults, no crash.
- Back goes to the right parent (Advanced Video -> Video, key bindings ->
  Keyboard/Mouse); discard dialog "Changes have been made. Discard current
  changes?" Discard/Cancel (Cancel keeps pending); dialogs capture input.
- Shared components for buttons, sliders, highlights, dialogs. Menus don't
  jump or resize when rows change; keyboard focus visible and ordered.

### T-185 Audio settings  (P2, ROWS DONE, awaiting owner replay; spec 4)
- 2026-10-21 rows DONE (unit test + autopilot shot vs the owner's Windows
  shot): Master Volume, Music Volume, Speaker Configuration, Sound Quality
  (Low/High), Puzzlemaker Sounds, Closed Captioning (None/Subtitles
  Only/Full Captions), Closed Captioning Language (Current Game Language
  + each language; captions load it), Enable Voice (Open Microphone/Push
  To Talk/Disabled), Push To Talk Key (greyed unless Push To Talk; shows
  the +voicerecord key, default C; opens Edit Keys on that row), SETUP
  MICROPHONE (bold; asks Steam to open its voice settings). Saved in
  settings.cfg. Honest gaps: Sound Quality and Puzzlemaker Sounds are
  stored only (no puzzle maker; one mixer quality); voice settings have no
  voice chat to drive in single player; Edit Keys does not yet scroll to
  the row (T-189).
- Rows: Sound Quality (Low/High), Puzzle Maker Sounds (Disabled/Enabled),
  Closed Captioning (renamed from Captioning: None/Subtitles Only/Full
  Captions), Closed Captioning Language, Enable Voice (Open Microphone/Push
  to Talk/Disabled), Push to Talk Key (greyed unless Push to Talk; default
  C; opens Edit Keys scrolled to and highlighting that binding; shows the
  new key on return), Setup Microphone (always bold; opens the Steam
  overlay where supported).
- Settings must work (caption level, language where supported); Steam/voice
  features fail gracefully.

### T-186 Video settings  (P2, ROWS DONE, display switching not replayed; spec 5, 14)
- 2026-10-21 rows DONE (unit test + autopilot shot): Aspect Ratio
  (Widescreen 16:9 / 16:10 / Normal 4:3 as the Windows shot spells
  them), Resolution (the display's own modes; native marked "(Native)"),
  Display Mode (Windowed (No Border) = borderless over the screen,
  Fullscreen = exclusive at the chosen size or the display's mode, Run in
  a Window), Laptop Power Savings (caps at 30 fps), Splitscreen Direction
  (Default/Vertical; stored, no splitscreen), ADVANCED VIDEO (bold).
  Brightness only shows in Fullscreen, as in Portal 2. Apply applies the
  display change. Mode switching was not tried on a real display (the
  autopilot stays windowed): owner replay needed.
- Rows: Aspect Ratio (16:9 / 16:10 / 4:3, default from the display),
  Resolution (the display's real modes, native marked "(Native)" and
  default), Display Mode (Windowed (No Border)/Fullscreen/Run in a Window,
  really switching, safe fallback), Laptop Power Savings (Enabled/Disabled),
  Splitscreen Direction (Default (Horizontal)/Vertical), Advanced Video
  (always bold, opens its section).
- UI stays correctly scaled after changes; values show the active config.

### T-187 Advanced Video settings  (P2, FIXED except per-setting render effects, awaiting owner replay; spec 6)
- 2026-10-21 FIXED (unit test + autopilot shot): the eight Portal 2 rows
  in order with the spec's values (Anti-Aliasing None/2x/4x MSAA -> the
  cameras' MSAA; Filtering Bilinear..Anisotropic 16X; Wait For Vertical
  Sync Disabled / Enabled (Double Buffered) / Enabled (Triple Buffered) ->
  present mode + frame latency 2/3; Multicore; Shader/Effect/Model detail;
  Paged Pool), then ours (Motion Blur, FOV, Max Frame Rate, Graphics, Show
  Frame Rate/Movement Stats). The frame stays 11 x 5 (was grown to fit 14
  rows, covering the logo); rows from 25 scroll with the wheel and a
  ScrollBar (black nob, arrows). The description pane now shows the
  install's own text (basemodui PORTAL2_VideoOptions_*_Info). Apply/Back/
  discard as T-184. Gaps: shader/effect/model/paged-pool detail and
  multicore are stored but the renderer draws everything at one level.
- Exact order: Anti-Aliasing Mode (None/2x MSAA/4x MSAA), Filtering Mode
  (Bilinear/Trilinear/Anisotropic 2x/4x/8x/16x), Wait for Vertical Sync
  (Disabled/Enabled (Double Buffered)/Enabled (Triple Buffered)), Multicore
  Rendering (Enabled/Disabled), Shader Detail (Low/Medium/High/Very High),
  Effect Detail (Low/Medium/High), Model / Texture Detail (Low/Medium/High),
  Paged Pool Memory Available (Low/Medium/High); project extras after.
- Scrolls inside the window (not a bigger window); scrolling doesn't break
  selection or the footer.
- Apply applies and returns to Video; Back with changes -> discard dialog
  (Discard leaves and drops, Cancel stays and keeps). Map each to the
  closest real renderer setting, nothing purely decorative.

### T-188 Controller settings  (P2, ROW DONE, input not wired; spec 7)
- 2026-10-21 the page has only Controller Enabled/Disabled (saved). The
  game still reads no gamepad at all, so the setting has nothing to
  switch yet: gamepad input is its own job.
- Only "Controller" Enabled/Disabled; really toggles controller input;
  persists.

### T-189 Keyboard/Mouse and Edit Keys  (P2, MOSTLY FIXED, awaiting owner replay; spec 8)
- 2026-10-21 owner note: Edit Keys' hover/picked bar is now the same
  black cursor bar with light text as every other row.
- 2026-10-21 (unit tests + autopilot shots vs the owner's Windows shots):
  Keyboard/Mouse rows as spec (EDIT KEYS/BUTTONS bold, Reverse Mouse,
  Mouse Sensitivity, Raw Mouse Input, Mouse Acceleration, Acceleration
  Amount, Allow Developer Console). Mouse Acceleration + Amount now bend
  the look (counts x speed^(exponent-1), unit test). Edit Keys/Buttons is
  a tiled 8 x 5 scrolling list: section headings (MOVEMENT, ACTIONS,
  CO-OP, MISCELLANEOUS, MISCELLANEOUS KEYBOARD...) with KEY/BUTTON over
  the key column and a rule, one key per action (labels from the
  install's kb_act.lst + text; View Leaderboard / Swap Splitscreen Control
  filled in), light grey picked/hovered row, ScrollBar; footer Back, Edit
  Key, Clear Key, Use Defaults; Esc cancels a pending key. Audio's Push
  To Talk Key opens it scrolled to and picking Push To Talk. Gaps: Raw
  Mouse Input is stored only (macOS input here is always raw); a key
  already used elsewhere is not flagged; controller buttons can't be
  bound (no gamepad input yet).
- Rows: Edit Keys/Buttons (always bold), Reverse Mouse, Mouse Sensitivity
  (slider), Raw Mouse Input, Mouse Acceleration, Acceleration Amount
  (slider), Allow Developer Console (gates the console). All functional.
- Edit Keys: section headings left (Movement, Actions, Co-op,
  Miscellaneous, Miscellaneous Movement) with "Key/Button" column right.
  Movement: Move Forward, Move Back, Move Left (Strafe), Move Right
  (Strafe), Jump, Duck. Actions: Fire Blue Portal, Fire Orange Portal, Use
  Item (Buttons, Machines, ...), Toggle Zoom, Zoom In, Zoom Out. Co-op:
  Partner Remote View, Communication Menu, Gesture Menu, Push to Talk Key,
  Chat Message. Miscellaneous: View Leaderboard, Take Screenshot, Quick
  Save, Quick Load, Pause Game, Quit Game, Toggle Developer Console, Swap
  Splitscreen Control. Miscellaneous Movement: Turn Left, Turn Right, Look
  Up, Look Down, Strafe Modifier, Keyboard Look Modifier.
- Footer: Back, Edit Key, Clear Key, Use Defaults. Rebinding updates the
  real action, handles conflicts, can be cancelled, persists over restart;
  deep link scrolls to and highlights a binding.

### T-190 Sliders  (P2, FIXED, awaiting owner replay; spec 9)
- 2026-10-21 FIXED (autopilot shots vs the owner's Keyboard/Mouse shot):
  one shared tiled slider: 130 x 6 track (150 grey) filled black (white
  on the hovered row), the small grey arrow above it, no side arrows,
  label 2.5 units higher. On Windows the arrow marks the value the dialog
  opened with (Sensitivity's sits at the saved 3; Acceleration's stays
  put while the fill moves), so ours marks the value the page opened
  with, which is the default until you change it. Sensitivity range now
  keyboardmouse.res's 1..20.
- Small arrow above the slider at its default value; no side arrows; one
  shared slider for every use; values in range, mouse and keyboard.

### T-191 Quit confirmation dialog  (P2, FIXED, awaiting owner replay; spec 10)
- 2026-10-21 FIXED (autopilot shots only). Was a plain grey box ("QUIT",
  OK/CANCEL) over the menu. Now GenericConfirmation as measured on a
  Windows 4K shot: tiled frame 320 units wide at x 100, 2-tile title tab
  "Portal 2" (BoldCn ~28, not upper case), body 81.5 units, the install's
  vgui/hud/icon_info icon, "Are you sure you want to quit?" in the 20-tall
  Cn font, Quit / Cancel footer at y 417; the menu column hides under it
  and a blocking layer keeps clicks off the menu. Exit to Main Menu uses
  the same dialog ("Exit To Main Menu?" / "Any unsaved progress will be
  lost.", the game's strings). Not done: the tile-flip animation (T-179),
  default keyboard focus on Quit (Windows draws it lit).
- Main menu Quit opens dialog: title "Portal 2", "Are you sure you want to
  quit?", Quit / Cancel; same look and tile flip as the discard dialog;
  doesn't trigger the menu behind it.

### T-192 Menu scaling and display changes  (P2, PARTIAL: 16:9, 16:10, 4:3 windows checked; fullscreen switching needs owner replay; spec 14)
- 2026-10-21 owner note "spacing still feels off; should scale to aspect
  ratios and resolutions": not yet pinned down. Everything is laid out in
  480-unit .res space scaled by height (as Source does), 4:3 shifts left;
  the owner's Windows shots are CrossOver windows letterboxed to 16:9.
  Needs an owner screenshot pair of what looks off (resolution/aspect).
- 2026-10-21 (autopilot --size 1280x720, 1280x800, 1024x768): the menu
  scales with the window height in .res units; on non-widescreen screens
  everything now moves 25 units left as the .res files do without
  $WIN32WIDE (dialogs at 75, items at 63, logo at 50), so the 11-tile
  Advanced Video fits 4:3 (it ran off the right edge). Not checked:
  real fullscreen / borderless switches on a display, 5:4, very small
  windows.
- Layout correct at native/non-native resolutions, fullscreen, borderless,
  windowed and each aspect ratio; no clipping or overlap; scrolling
  sections usable at small sizes.

## Open: P0
(none)

## Open: P1

### T-108 A chamber door had an invisible barrier, needed noclip  (P1, NEEDS INFO)
- intro4 doors pass both ways in a walk test. Need the door and map from the owner.

### T-107 Cubes and floor buttons invisible  (P1, CAN'T REPRODUCE)
- Owner screenshot 8 (intro2, cube held above the button): corner tiles
  drawn, red top and ring gone, a black ellipse in the pit, a straight
  diagonal edge cutting the model, like a portal clip plane applied to it.
  Not reproduced (load, changelevel, cube toggled 14 times, BindPose, views
  from above). Next: intro2 portals active, cube carried through onto the
  button. Playtest 8: cubes and buttons drawn.

## Open: P2

### T-145 Buttons don't animate; cubes don't change colour on a button  (P2, BLOCKED (needs owner): lit cubes dark, cause not found; spikes and jitter not looked at)
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

### T-172 Voice lines start late  (P2, TODO)
- Owner playtest 9: "sometimes voicelines activate late and when I'm going
  through the elevator, making the elevator ride go on longer" (global).
  Elevator rides wait for the exit VO's TransitionReady (T-120), so late VO
  stretches them. Measure scene start vs its trigger (logic_choreographed_scene
  Start, glados.nut queue, sound decode/load latency) on several maps.

### T-120 Elevators  (P2, OPEN: VO timing only; jitter, car sounds, thumps owner-confirmed)
- Owner playtest 9 (2026-10-20, build 014ab6e): jittery rides FIXED, thumps FIXED; VO timing NOT fixed (voice lines start late, also while riding, which stretches the ride: see T-172); rubble/turrets still fall through the elevator (T-135).
- Jitter FIXED 2026-10-20 (c1079f9, see Awaiting owner replay). Still open:
  the VO after intro5's second button started
  halfway down and stretched the ride (T-138); thumping sounds during the
  ride (section 4); the trash rising with the elevator is buggy; one ride
  reached the bottom before the next map loaded.
- 2026-10-20 autorun: VO timing / ride length NOT REPRODUCED (no code change).
  How it works: path_tracks call ReadyForTransition() (sp_elevator_motifs.nut);
  it only leaves once the exit VO's scene fires TransitionReady(), else the
  car rides the whole shaft to the failsafe node (intro5: ~20 s at 200 u/s,
  same logic as the original). Autopilot: intro2 with PuzzleCompleted() leaves
  at the first path_track (~1 s into the ride); intro5 with the cube put into
  the box_filter trigger (64 448 264) starts testchamber03 0.6 s later, so the
  VO is not delayed by our scene/script chain. Differences seen: our new line
  cuts off the still-playing start line (testchamber02) instead of waiting;
  unchecked whether a cube resting on the real button overlaps that trigger
  (owner said the line came only halfway down). Needs an owner replay of
  intro5's second button + `log` timing to go further.
- 2026-10-20 autorun: thumps NOT REPRODUCED. Every world/player sound during
  the intro5 and intro6 arrival and departure rides was logged
  (`RUST_LOG=portal2::audio=debug,p2_fx=debug`): no landing/slam/impact
  sounds; the only impacts are the chamber's robot arms (World.Robot*,
  positional, gain 0.05-0.3 by distance). Found instead that the cars were
  silent: func_tracktrain movesound/stopsound never played. FIXED (5bfaf52,
  see Awaiting owner replay). Open: thumps (owner: which ride, what it
  sounded like), trash rising with the car, move-sound pitch by speed
  (movesoundmin/maxpitch, not done).
- 2026-10-20 autorun: trash NOT REPRODUCED (no code change). The trash is
  intro5's departure_elevator-tube_blockage_01 (models/elevator/
  elevator_rubble.mdl, solid 6, holdanimation 1): blocked_elevator_tube_anim
  (trigger at -64 816 352) plays the car body's "arrive" (car comes DOWN,
  root z 972 -> 110 in ~2 s) and 1.75 s later the rubble's "anim" (11.2 s;
  29 root bones fall from model z 469..1464 to about -90..99, i.e. onto the
  car roof). Autopilot (setpos -64 1150 300, look -40 90; shots at 3.5-15 s)
  shows exactly that: car arrives, rubble drops and rests on the roof. The
  departure ride with the rubble on the roof was not checked (needs the
  puzzle solved). Possible owner-visible issue: the rubble is a held
  prop_dynamic, so when the car leaves upward it passes through the pile
  (same entities in the original; unknown whether Windows hides it).
  BLOCKED: needs the owner to say which ride and what looked wrong, or a
  Windows reference of intro5's departure.

### T-135 Turrets fall through the exit elevator  (P2, OPEN: owner confirms, still happens)
- Owner playtest 9 (2026-10-20, build 014ab6e): "things still falling through elevators"; rubble and turrets both fall through (T-120's trash is the same bug).
- Sections 5 (sp_a2_intro) and 6 (sp_a2_laser_intro).
- 2026-10-20 autorun (HEAD abc5618), cause not confirmed. The turrets are
  `departure_elevator-vacturret_01..04` (prop_dynamic,
  models/props_vac_anim/turret_lowres.mdl, origin = the elevator axis
  -736,1600,-10672 in sp_a2_intro). `@start_elevator_animation_rl` (trigger_once
  at -368,832) sends SetAnimation + SetDefaultAnimation `elevator_0N` (delays
  0.3/0.8/2.7/3). Decoded root-bone track of elevator_01: z +355 -> -87 (0.82 s)
  -> -510 (1.63 s), x/y within +-22, so the model's own data drops the turret
  straight down the shaft from -10317 to -11182, through the waiting car
  (bounds z -10958..-10731; the car sits at path_0 until startmoving()).
  AE 1100 `fireuser1` at cycle 0.459 (1.5 s, already below the car) ->
  OnUser1 -> `tube_object_kill` (kills !activator, only enabled by the
  trigger_once at -736,1104 via `blocked_elevator_tube_anim`). Engine already
  handles SetDefaultAnimation (re-loops the fall) and event 1100.
  Autopilot shots from the corridor/shaft were inconclusive (player placement
  fell into the shaft). Open question for the reference: does Windows Portal 2
  show these turrets dropping past/through the car (then only the looping
  after the kill relay matters), or is the root-bone position meant to be in
  another frame (e.g. a horizontal vac tube above the car)? Needs an owner
  screenshot/clip of this moment in Windows Portal 2 (or depot 624 RE of
  prop_dynamic root-motion handling). No fix attempt branch.

### T-139 Audio  (P2, PARTIAL: glass fixed; music/ambience and soundscapes open)
- Owner playtest 9 (2026-10-20, build 014ab6e): glass fixed; "some music doesn't play along with ambient sounds"; voice lines late (T-172).
- No soundscapes/ambience (water drips), no button clicks (sections 1, 5).
  Still open: the engine has no env_soundscape / soundscape script support
  at all (grep finds nothing), so this is a feature to build.
- Music stops abruptly sometimes; stopped at the wakeup stairs (old T-111).
  Still open, not looked at this run.
- Glass break far too loud entering intro4 while still in the elevator
  (old T-118). FIXED 2026-10-20 (503406e, autopilot only): cause was
  `aud_ramp_break_glass` (ambient_generic, flags 33 = everywhere + not
  looped, ScriptedSequence.RampGlassBreak, 150 dB) autostarting at map
  load because only flag 16 suppressed autostart. One-shot (flag 32)
  ambients now wait for PlaySound; 12 of 13 such ambients in the sp_a1/a2
  maps are PlaySound-triggered by their maps (the 13th, bts5
  `grinder_noise` world.grinderlp, now stays silent: replay bts5 too).
  Evidence: dist 89f2bf5 logs `play scriptedsequence.rampglassbreak` at
  intro4 load before the elevator arrives; 503406e does not, and plays it
  when autopilot walks into the ramp trigger (`goto glass_shard`), with the
  pane door opening. Unit test `only_looping_audible_ambients_autostart`.

### T-112 Missing or flat textures  (P2, PARTIAL: GLaDOS floor better but still murky brown)
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

### T-144 Missing props: intro1 portal room clipboard and movable clock  (P2, PARTIAL: props fixed; clipboard should turn to face the player)
- Owner playtest 9 (2026-10-20, build 014ab6e): fixed, "but the clipboard prop doesn't auto face you so you can read it" (Portal 2 turns some props to a preferred carry angle when picked up; check the model's physgun/carry keyvalues).
- Cause: physics props had Mass(40) but their rotational inertia came from
  the collider at avian's default density (1 kg/m3). The radio's hull weighs
  37 g at that density, so it got about 1/1000 of a 40 kg box's inertia: contacts
  spun it, it rocked from spawn and sank through the vault floor at ~1.8 s.
  The unnamed bts_clipboard on the table fell too. Fix: the prop's colliders
  get the density that makes them weigh 40 kg (unit test
  prop_colliders_weigh_the_prop_mass).
- Evidence (autopilot, sp_a1_intro1): dist 2e8df1a radio z 2705 -> 2691 at
  1.8 s -> 2180 at 3 s; clipboard z 2639 at 0.5 s. New build: radio at
  (-1208 4324 2705) upright at 0.5/1.6/3/6 s, clipboard at 2727 on the table
  at 1 and 6 s. Side effect: intro2's spawn cube now settles flat on a face
  (z -43) instead of leaning at 24 deg (z -39); every prop now has a 40 kg
  box's inertia (cubes tumble less). "Movable clock" still not identified
  (the clocks are prop_dynamic and draw); owner to say if anything is still
  missing.

### T-149 Chamber signs blank; screens and lights wrong  (P2, PARTIAL: signs drawn; elevator videos and intro5 screens open)
- Owner playtest 9 (2026-10-20, build 014ab6e): signs fixed; "the videos that play in the elevator rooms are a bit buggy and super low quality" (now T-174). Owner screenshot 2026-10-20 2.45.36 PM (sp_a2_laser_over_goo) shows "04/22" with a big "01": check the big number against Windows.
- The white chamber-number screens ("02") were blank in every level:
  they are `vgui_screen`s with panelname `sp_progress_sign`, which we never
  drew (only the dark sign frame model showed). FIXED: new
  crates/p2_game/src/signs.rs composes the board per map from
  scripts/vscripts/transitions/sp_lightboard_icons.txt (level_number,
  total_levels, ten icons, "1" = used) and the art
  vgui/screens/vgui_coop_progress_board (+_numbers digit sheet 6x2,
  P2_lightboard_vgui icon sheet 5x5): big two-digit number between the top
  rules, "NN/TT" and a per-chamber progress bar (played ones dark) in the
  middle band, the ten icons in two rows of five (used ones dark, others
  faint). Shown on the prefab's SetActive, hidden on SetInactive (spawnflag
  1 starts it on). Unit tests: file parsing, lit vs faint icons, bar.
- Evidence (autopilot, intro2, `input info_sign-info_panel setactive; goto
  info_sign-info_panel 320; look -38 -12`): old build (dist 503406e) shows
  an empty dark frame; new build shows "01", "01/19", the first bar segment
  dark and the cube_button icon dark with the rest faint.
- Approximations (no Windows reference shot): exact positions/sizes of the
  number, label, bar and icons are by eye from the board's three rules;
  the icon sheet's cell order was matched to names by picture (it differs
  from the file's comment list); the startup flicker sequences
  (lightboard_startup_sequences.txt) and dirt overlay ("dirt") are not
  drawn. Owner: compare one sign with Windows Portal 2.
- Still open: intro5's screens and lights wrong (playtest section 4), not
  looked at this run.

### T-147 intro2: the screen goes black right as the first door opens  (P2, TODO)
- Section 1; compare T-119. Screenshot 7 (intro2): a dark shard in a door.

### T-113 Instructor hints vanish after a few seconds if not done  (P2, TODO)
- Key labels fixed (owner confirmed).

## Open: P3

### T-171 FPS drops, especially looking at portals  (P3, TODO)
- Owner playtest 9: "I do get frequent fps drops especially looking at
  portals". Measure frame time with and without a portal in view (portal
  view renders, recursion depth, shadow maps per portal view).

### T-148 Portals behind glass  (P3, REOPENED: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed: "portals still look like they're above glass when they're behind it".
- Owner section 1 (intro2): portals behind glass look on top of the glass;
  behind broken glass not drawn at all.
- Cause: glass/water (Refract) read Bevy's transmission copy, taken once
  before the transmissive pass, and live portal surfaces were drawn in that
  same pass. Broken pane (glass_fracture_128x128_b, its Refract mesh sorted
  after the portal) refracted a copy of the bare wall over the portal;
  glass sorted before a portal was overdrawn by it.
- Fix: live surfaces sort first (depth_bias) and p2_render's
  portal_surface_pass draws them after the Valve scene passes, before the
  copy; Bevy's second draw fails a strict depth test. Unit test
  portal_prefix_len.
- Evidence (autopilot, sp_a1_intro2, portal_blue_1/red_1 activated,
  `setpos -450 -60 -60; lookat portal_blue_1`): before, no portal behind the
  cracked pane; after, the portal shows through it with cracks and
  refraction over it. A view through the intact pane (setpos -100 -40 -60)
  looks the same before and after; the "on top of normal glass" case was not
  isolated by autopilot, owner replay needed.

### T-117 Blocky shadows  (P3, REOPENED: better but still blocky)
- Owner playtest 9 (2026-10-20, build 014ab6e): "shadows look better but still blocky".
- Cause: env_projectedtexture shadows (light_room_00 in sp_a1_intro1: 110 deg
  FOV) used a 1024 map and one 2x2 bilinear compare, so the hanging
  planter's shadow stepped per texel and its chains broke into dashes
  (owner screenshot 2).
- Fix: 2048 shadow maps (projected.rs SHADOW_SIZE) and a 3x3 (1 2 1)
  Gaussian of bilinear compares (projected_common.wgsl), like Source's
  flashlight PCF.
- Evidence (autopilot, sp_a1_intro1, `2 input camera_intro disable; 4 setpos
  -8640 1720 10 -18 180; 5 shot`): before (dist 0.1.0+b39fff3) stair-stepped
  pot outline and dashed chains; after smooth outline, solid chains, no
  acne on the walls. Replay: look at the planter shadow above the closet.

### T-152 Lighting and texture quality below the original  (P3, TODO: owner says still an issue)
- Owner playtest 9 (2026-10-20, build 014ab6e): "lighting/textures being low quality is still an issue". Raised to P3.
- Scrap pieces in the intro3 fall; turrets in tubes (wakeup); bumped
  VertexLitGeneric (models with $bumpmap) still drawn without bump.

### T-153 GLaDOS wake-up  (P3, PARTIAL: gun drop fixed; white particles still hide her)
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
- Still open: white particles hiding her, arms floating unattached, the
  player looking the wrong way.

### T-173 Portal gun view model: no hold beam, no fizzler spark/shake  (P3, TODO)
- Owner playtest 9: "portal gun still doesn't have that electricity when
  picking things up and doesn't spark/shake when going through the
  emancipation grids" (also T-150's gun beam).

### T-140 Game-placed portals: no screen shake or sound  (P3, REOPENED: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed (no shake/sound felt on game-placed portals).
- The first orange portal should shake the screen a little and play its sound.
- Cause: env_shake was not implemented at all. intro1's
  portal_red_0_activate_rl fires `shake_portal_spawn_room1 StartShake`
  (amplitude 4, frequency 2.5, 1 s, radius 500) 0.8 s after Trigger, with
  the portal's SetActivatedState. The sound already played: map portals send
  PortalPlaced, so Portal.open_red plays (log, before and after build).
- Fix: new p2_game shakes.rs (StartShake/StopShake/Amplitude/Frequency,
  linear radius falloff, spawnflag 1 global, 32 no view shake) feeding
  p2_core ScreenShakes; p2_view offsets and rolls the main camera (random
  direction re-picked `frequency` times a second, fading squared). Not done:
  physics/rope pushes (flags 8/16), the on-ground check. Unit tests:
  amplitude_fades_with_distance, shake_fades_out.
- Evidence (autopilot only): `goto portal_red_0 200; input
  portal_red_0_activate_rl Trigger`: "env_shake start, amplitude 2.34 at
  207 u", then RUST_LOG=p2_view=trace shows camera offsets peaking ~1.9 u
  and decaying to 0 over the second; before build: no shake. Owner replay:
  does it feel like Windows Portal 2 (strength, speed)?

### T-174 Elevator-room videos buggy and very low quality  (P3, TODO)
- Owner playtest 9: "the videos that play in the elevator rooms are a bit
  buggy and super low quality" (the vgui movie screens around the arrival
  and departure elevators; T-129 fixed their placement).

### T-175 sp_a1_intro1 ride: the big room outside the container has no fog in parts, black sky  (P3, TODO)
- Owner playtest 9: "the big room outside of the moving box has some issues
  in the wakeup sequence, like parts not being foggy and the sky being just
  black above" (compare T-116's "odd outside lighting on the ride").

### T-164 Spiky, torn animated debris  (P3, OPEN: owner says not fixed (attempt/T-164 never merged))
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed; the fix on attempt/T-164 was not in the build played. Also possibly the "spikes under the button" (T-145).
- intro1 ride (screenshot 5), sp_a2_intro shaft arms (playtest 7). Animated
  props draw as rigid per-bone triangles (models.rs split_by_bone); soft
  weighted meshes tear. Check the weights, then skin them.
- 2026-10-20 autorun: weights checked (mdlinfo now prints "soft verts" and
  "mixed tris" per mesh). intro1 ride: the container_ride finedebris_part1/3/
  5/9/11/12/13 rebars, rust, crumplebeam/crumplecorner, bed02 and frame2
  meshes blend 2-3 bones (e.g. part12 crumplebeamcol 1257/3265 soft verts,
  1846 of 3080 tris mixed), so the rigid split tears them: confirmed cause
  by data. sp_a2_intro: every model is rigid (arm64x64_interior_rusty,
  crusher, armrubble_* all 0 soft verts), so the shaft-arm spikes are a
  different problem, still open (check bone poses/interpolation there).
- Not visually reproduced: in sp_a1_intro1 autopilot (`input
  actor_container_05 SetAnimation anim2`, `input camera_intro Disable`,
  `goto`/`lookat`) the view stays on the bed-room intro camera, so no
  before/after shot of the debris.
- Untested fix: local branch attempt/T-164 (c34006e), needs owner
  review/replay. Soft meshes (vertexlit/unlitgeneric) stay whole under the
  model and are linear-blend skinned on the CPU from their BonePart
  transforms when those move (plugin.rs skin_soft_meshes, models.rs
  skin_vertices with a unit test); valve_render re-uploads the vertex
  buffer when the SkinnedVertices generation changes. Replay the intro1
  ride's wall breaking and the room transform after `git merge
  attempt/T-164`.

### T-116 intro1 room details  (P3, BLOCKED: owner will send comparison screenshots)
- Owner playtest 9 (2026-10-20, build 014ab6e): "that window does look different in the main game, I'll show comparison screenshots".
- The window is a grey block, should be blinds (screenshot 1); the TV
  screen sits forward and up from the TV; odd outside lighting on the ride.
- Autorun 2026-10-20, NOT REPRODUCED (autopilot only, no code change, no
  attempt branch):
  - Window: the start room's window (x ~ -8460, behind the plant) is world
    brushes in glass/glasswindow_frosted_break001a (and
    glass_underground_dirty01a): LightmappedGeneric, $translucent, a dirty
    frosted texture with vertical streaks, no blinds in it. P2_HIDE_MAT=glass
    shows only plain bright white behind it (no model or brush with blinds
    there; static props in the room are furniture04, closet_curtain01,
    lab_pod_frame_end_01, container_lightcover). So we draw what the map
    has; the glass may just read darker than on Windows (the VMT's
    `"srgb?$alpha" "0.5"` conditional is not applied by us; unconfirmed).
    The ride container's slatted look (models/container_ride/windowrails,
    finedebris_part7) is a different room. Needs a Windows screenshot of
    the same view to say what "blinds" should look like.
  - TV screen: `tvscreen` (tvscreen01.mdl) belongs to the ride's
    actor_container_07 and gets SetParentAttachmentMaintainOffset
    screenattachment from @rl_container_ride. Autopilot shots 1.5 s and
    9 s into the ride show it on the TV; the start room's TV is part of
    hotel_container_furniture models. Needs the owner's exact moment/view.
  - Outside lighting on the ride: not checked this run (ride is seen
    through point_viewproxy camera_1, see T-020).

## Open: P4

### T-151 Water like Portal 2's (reflections, murk), intro3 and sp_a2_laser_intro  (P4, OPEN: owner says not fixed (attempt/T-151 never merged))
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed; the fix on attempt/T-151 was not in the build played.
- Owner (playtests 8 and 6): "should look like water with reflections";
  "water ugly, not murky with reflections".
- Seen 2026-10-20 (autorun, dist eb58eac, `goto fall_splash 250`): intro3
  water is a flat, ripple-less brown sheet; no reflection at all, because
  plugin.rs drops the sky reflection on fogged maps (earlier fix: the white
  sky turned the water white).
- Material (both maps share it): `nature/water_a1_intro3` /
  `water_a2_laser_intro`, `$reflecttexture _rt_WaterReflection`,
  `$reflectskyboxonly 1`, `$reflectonlymarkedentities 1`, `$reflecttint 190`,
  `$reflectamount 0.4`, flowmap normals, `$lightmapwaterfog`, fog 90 70 20
  over 0-60, `$pseudotranslucent 1`, `$waterblendfactor 0.75`. Both maps'
  sky is sky_white (pure white UnlitGeneric), no sky_camera. water_ps20b's
  combos without RefractSampler (pseudotranslucent) still sample
  ReflectSampler plus lightmap, and every combo has g_PixelFogParams (the
  map's fog over the surface).
- Suspected cause: no reflection, and no map distance fog on glass/water
  (glass.wgsl returned its colour without Bevy's fog). The previous "white
  water" came from multiplying the reflected sky by the HDR exposure; Source
  samples an LDR reflection target, so the sky saturates at 1 before the tint.
- Untested fix: local branch attempt/T-151 (2df932d; cbfa31d + test stub),
  needs owner review/replay. Sky reflection back on fogged maps, clamped to
  1 then times `$reflecttint`, Fresnel-mixed with the lit fog; map distance
  fog applied to glass/water. Autopilot shots: intro3 water now grey with
  visible flowing ripples at grazing angles (before: flat brown). Whether
  Portal 2's looks that pale is unknown without a Windows screenshot of the
  intro3 pool (or laser_intro). Not done: `$pseudotranslucent` alpha blend
  (`$waterblendfactor`), `$reflectamount` screen-space offset.

### T-150 Particles missing  (P4, TODO: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed.
- Fall smoke and water splash (+ sounds) in intro3; the gun's beam when
  holding things; motion blur/speed lines; the gun's glow seen through the
  wall after the sp_a2_intro fall.

### T-146 intro1: the first cube-button door should get half stuck  (P4, TODO: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed.

### T-154 Floating vine/moss/grass texture by the centre wall (intro5)  (P4, TODO: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed.

## Awaiting owner replay (fixed, autopilot evidence)

### T-170 Carrying regressed: pushed back at walls, can't push a cube up a wall, not smooth  (P2, FIXED 9f1ed5a, awaiting owner replay)
- Owner playtest 9: "trying to move a cube up on a wall or something stops
  me from moving forward, it's buggy, and carrying them around in general
  isn't too smooth"; "something about what fixed dropping cubes when pushing
  them in the wall made carrying stuff around a bit buggy and push me back
  when close to walls".
- Suspects: T-167's player stop (342b5b1: a ray along the view stops the
  player 17 u + the prop's depth from a wall) and T-105's hold point pulled
  in front of walls (fd9c086). In Portal 2 the held prop is solid to the
  player but slides along walls; aiming at a wall or floor should not freeze
  movement.
- History (the fixes that caused it):
  - T-167 (rest) A held cube at a wall surrounded the camera: the player
    ignores the held prop's colliders, so walking on pulled the hold point
    back onto the eye. Now (342b5b1), as in Portal 2 where the held prop is
    solid to the player, a ray along the view stops the player 17 u plus the
    prop's depth from a wall (skipped into portals). intro2 box walked into
    two walls: before (dist) box centre at the eye (36 398 28, feet 36 400),
    view = inside ring of its face; after the player stops ~41-49 u back and
    the whole cube shows. Autopilot only; walking into a portal while holding
    and Wheatley in intro7 not checked.
  - T-167 (part) Wheatley can't be picked up again after a drop: cores were
    pickable only after an EnablePickup input, which sp_a1_wakeup never sends
    (he starts force-picked), so once dropped (or stuck-dropped) he was lost.
    Cores now stay pickable until DisablePickup (intro7's map-spawn disable
    still applies). wakeup: drop enabled via input, `use` drop, goto + lookat,
    `use`: before (dist fd9c086) the ray hit him and he stayed on the floor
    (z 401); after (636fc3f) he is held facing the player at z 441. Only
    autopilot-checked, wakeup only.
  - T-105 Held props dropped when carried into a wall: the hold point went
    inside the wall, the prop stayed >64u from it and was dropped after 0.5 s.
    Now a ray from the eye pulls the hold point in front of static geometry
    (not through portals). intro2 cube walked into three walls: before
    (dist build) it fell to the floor (z -46) and stayed; after (fd9c086) it
    stays held at the wall and returns to its hold distance when backing
    off. Only the cube was autopilot-checked, not Wheatley in intro7.
- FIXED 2026-10-20 (9f1ed5a). Cause: T-167's stop used a level ray along
  the view against any wall (17 u + 2x the prop's depth) instead of the
  prop: looking up to push a cube up a wall the player froze ~60 u out, and
  turning toward a wall within ~65 u backed them off at 60 u/s. Now the
  prop itself blocks the player, like Portal 2's solid held prop: only when
  level with the hull, touching it along the view and pinned by a wall
  (shape cast of the prop), and only along that wall's normal (slides along
  it). Carry keeps the hold point out of the hull: looking up it goes over
  the head (up the wall), otherwise out in front. intro2 box, before (dist
  0bc1dd1) / after: walk into the wall level: stops 340 / 349 (whole cube
  visible, T-167 kept); looking up 55 deg: frozen at 340 / walks to the wall
  (400), cube over the head at z 27; back to the wall then turn to face it:
  pushed 62 u / eased out only because the cube doesn't fit (16 u room);
  looking down 65 deg on open floor: 126 / 127 u in 1.5 s; cube pressed to
  the glass then walking at 160 deg: slides 150 u / 156 u (an intermediate
  build that blocked along the view froze here). Autopilot only; intro3
  (cubes unnamed, dropper) and Wheatley in intro7 not checked.


### T-162 Menu animations, logo alignment, styling  (P2, ALL SCREENS DONE 0bc1dd1, awaiting owner replay)
- Owner playtest 9 (2026-10-20, build 014ab6e): T-160/T-161 confirmed, but "the ui not being styled like portal 2 ... is big for me, please get that as close as possible". Raised to P2, right after softlocks (owner, 2026-10-20). Screens done: main menu (e6e5e91), Single Player (8737034), New Game (3ca8329), Load/Save Game (47757a0), Options dialog (2a6921d), Audio (802802d), Video (5d16315), Advanced Video (4ca377c), Keyboard/Mouse (9b0053b), Controller (903118b), pause menu (21f6353), HUD crosshair + chapter title (b417bba), instructor hints (9576999), subtitles (f475223), loading screens (0bc1dd1), see below. Left: none (the 0.3 s dialog fades and other "Not done" notes below remain). Covers the main, pause, options, new game and load menus (fonts, colours, panels, transitions) against the install's resource/ui/basemodui/*.res and basemodui_scheme.res.
- Visual reference (owner, 2026-10-20): https://interfaceingame.com/games/portal-2/
  has screenshots of the real Portal 2 UI: start screen, main menu, options,
  video, keyboard/mouse, edit keys, extras, loading, chapter title, save
  game, quit dialog, tutorial hints (create portal, pick up) and the test
  chamber evaluation. Compare each screen against it (built-in browser
  `get_page_text`/screenshots) alongside the install's .res files.
- Panel/flyout transitions flip oddly; the Portal 2 logo is misaligned.
- Cause: the logo and item column were placed by eye (logo 6%/9%, 34% wide;
  items 6% left, 12% from the bottom, 2 px apart). Now from the install:
  basemodui_scheme.res Logo.X/Y 75,35, 240x60 units; mainmenu.res items at
  x 88, 20 tall every 30 units, last at y 378 (480-unit layout, Vh-scaled;
  pause menu uses the same column). Also every menu change rebuilt the
  buttons, which only learn they are hovered a frame later, so the clicked
  item flashed unlit for a frame; carry_hover now keeps them lit.
- Evidence: autopilot main menu + `click SinglePlayer` shots, before (dist
  89f2bf5-era) vs after b39fff3; Quit row centre at 80.8% of the height as
  the .res gives. The one-frame flash was found by reading the code, not
  captured in a shot. Still open: Portal 2's panel fade/slide timings are
  not implemented (no Windows reference of what the owner saw as "flip");
  the main/pause font size was not checked against the scheme.
- 2026-10-20 autorun, main menu DONE (e6e5e91, autopilot shots only, not
  owner-checked). basemodui_scheme.res: MainMenuButton font MainMenuItem =
  UniversLTStd-Cn tall 22 (PC), TextColor 120 120 120, FocusColor white,
  CursorColor 255 255 255 15, TextInsetX 10. We used DIN Alternate Bold at
  24 px in near-white. The real face ships as platform/vgui/fonts/
  universc.vfont (CustomFontFiles); its encoding was worked out by known
  plaintext (XOR, key = previous encoded byte + 0xA7; salt + "VFONT1"
  trailer) and decode_vfont now loads it (unit test). univercb (Bold Cn)
  and univercl (Light Cn) decode the same way for later screens. Item em =
  22 / 1.265 units (vgui tall is the Windows cell, winAscent+winDescent).
  The cursor bar's alpha is 3/255 because Bevy blends in linear light (15
  gave grey 72 over 17; vgui's gamma blend gives ~31). The item whose
  flyout is open stays lit. Evidence: before/after main menu and
  `click SinglePlayer` shots (dist a1cd026 vs e6e5e91). Note: one harmless
  "font size <= 0" warning on the first frame (Vh size before the window
  has a size). Flyout itself (light grey box) is the next screen.
- 2026-10-20 autorun, Single Player DONE (8737034, autopilot shots only,
  not owner-checked). In Portal 2 SoloPlay opens singleplayer.res, a
  CBaseModFrame dialog (dialogstyle 1), not a flyout: the menu column
  hides, the dialog is pinned 100 units left / 75 from the bottom
  (Dialog.PinFromLeft/PinFromBottom), body "wide 5 tall 4" in 50-unit
  tiles (Dialog.TileWidth) under a one-tile title tab, drawn from
  materials/vgui/menu_tiles/ui_tile_128_*; title DialogTitle (BoldCn 33,
  black, offset 10,9), rows DefaultButton every 25 units from 25 (upper
  case, black; HybridButton focus 240 on cursor 0 0 0 240; disabled 140
  grey), Back below (FooterPanel.OffsetY 12). Measured against the
  reference gallery's Co-op Mode shot (coopmode.res, same frame): every
  position matched within a unit. Continue/Load are greyed without a save,
  Developer Commentary is greyed (not built). Evidence: autopilot `click
  SinglePlayer` shots, before (dist e6e5e91, grey flyout) vs after.
  Not done: the 0.3 s Frame.TransitionEffectTime fade, keyboard focus
  starting on the first row, the Options flyout (still grey).
- 2026-10-20 autorun, New Game DONE (3ca8329, autopilot shots only, not
  owner-checked). newgamedialog.res: tiled frame wide 9 tall 4 (shared
  tile_frame() with Single Player); ChapterList GenericPanelList at 0,25,
  200 x 150 (PC), newgame_chapteritem rows 25 tall, label inset 10, so 6
  rows show and the list scrolls (mouse wheel, ScrollList); picked row on
  the black cursor bar, locked chapters grey; ChapterImage 212,15 225 x 152
  scaleImage (stretched); ChapterText (ChapterLabel) 212,170: chapter in
  NewGameChapter (UniversLTStd-Cn 18) + name in NewGameChapterName (BoldCn
  18). Guess: rows read "CHAPTER N" (the .res/scheme don't say what the
  list label holds; the full name would overflow 200 units). Footer: Start
  New Game, (Community Maps when there are any), Back. Evidence: autopilot
  `click SinglePlayer; click NewGame` and `click SelectChapter(2)` shots,
  before (dist 8737034: grey panel, full-name list) vs after. Not done: a
  drawn scrollbar (ScrollBar.Wide 12, nob black), Back returns to the main
  menu rather than the Single Player dialog, double-click to start, the
  0.3 s fade; the Community tab keeps its old grid.
- 2026-10-20 autorun, Load Game DONE (47757a0, autopilot shots only, not
  owner-checked). Portal 2 uses saveloadgamedialog.res for both Load and
  Save Game (loadgamedialog.res is an unused L4D leftover): same 9 x 4
  tiled frame as New Game (tile_frame), SaveGameList at 0,25, 200 x 150,
  savegameitem rows 25 tall, label inset 10, scrolling; SaveGameImage
  212,15 225 x 152 stretched; AutoSaveLabel 210,9 223 wide, east aligned,
  NewGameChapterName (BoldCn 18) white alpha 200 for auto/quick saves;
  SaveGameInfo 212,170. First click picks a row, a second loads/saves;
  footer Load Game (or Save Game) + Back; Load starts on the newest save,
  Save on a New Save row; Back from the main menu returns to the Single
  Player dialog. Guesses: rows show the save time (the .res doesn't say
  what the label holds) and SaveGameInfo shows the chapter; the picture is
  the chapter's (we save no screenshots). Evidence: autopilot `click
  SinglePlayer; click LoadGame`, `click PickSave(Some(1))`, `click Back`
  shots, before (dist 3ca8329: grey list panel) vs after. Not done: save
  screenshots for the picture, Delete, the cloud icon, the 0.3 s fade.
- 2026-10-20 autorun, Options dialog DONE (2a6921d, autopilot shots only,
  not owner-checked). options.res (PC) is a tiled CBaseModFrame dialog
  (dialogstyle 1, body 5 x 3), not a flyout: rows Audio, Video, Keyboard/
  Mouse, Controller (DefaultButton every 25 units from 25), replacing the
  menu column on both the main and pause menus. Our own Speedrun and Mods
  pages follow as rows 5-6 (body grows to 4 tiles to hold them). Leaving a
  page (Done/Cancel/Esc) returns to the dialog, Back/Esc closes it.
  Evidence: autopilot `click Options` shots, before (dist 47757a0: grey
  flyout box) vs after; `click Page(Audio); click Done` lands back on the
  dialog; pause (`click pause; click Options; click Back`) in intro2.
  Not done: the Settings panels themselves (next), the 0.3 s fade.
- 2026-10-20 autorun, Audio DONE (802802d, autopilot shots only, not
  owner-checked). audio.res is a tiled CBaseModFrame dialog, body 7 x 5
  (tall 6 for localized audio), controls every 25 units from 0 (it packs
  10 rows, so unlike Video/Keyboard it starts at 0): SldGameVolume,
  SldMusicVolume (LeftDialogButton + SliderControl), DrpSpeakerConfiguration,
  DrpSoundQuality, DrpPuzzlemakerSounds, DrpCaptioning, DrpCaptionLanguage,
  DrpLanguage, voice, microphone (DialogListButton). We show ours in that
  order packed from 0 (no sound quality/puzzlemaker/voice/mic rows: not
  built) and our Subtitle style link last. Label at TextInsetX 10, value
  or slider at the right edge (SliderControl.InsetX -10), DialogButton font
  = UniversLTStd-LightCn tall 18 weight 700 on PC HiDef, drawn with the Cn
  face (Light Cn + synthetic bold is close to Cn); black on the tiles;
  SliderControl track 150 grey, fill black, no number; hovered row on the
  HybridButton cursor bar (black 240, text 240, fill white). Footer Done/
  Cancel under the frame. Evidence: autopilot `click Options; click
  Page(Audio)` shots, before (dist 2a6921d: grey panel with % numbers) vs
  after; `click Page(Subtitles); click Done` returns to Audio. The hover
  bar colours were NOT autopilot-checked (autopilot has no mouse-move
  command). Not done: the Subtitles sub-page (ours) keeps the old panel,
  labels are ours, not the localized "#L4D360UI_AudioOptions_*" strings.
- 2026-10-20 autorun, Video DONE (5d16315, autopilot shots only, not
  owner-checked). video.res: tiled CBaseModFrame dialog, body 7 x 4,
  controls every 25 units from 25: SldBrightness (hidden when windowed in
  Portal 2), DrpAspectRatio, DrpResolution, DrpDisplayMode,
  DrpPowerSavingsMode, DrpSplitScreenDirection, BtnAdvanced (DefaultButton,
  ShowAdvanced). Ours in that order on the shared tile_frame/
  tile_option_row (no power savings/split screen: not built); Brightness
  kept in a window since ours works there; footer Apply/Done/Cancel (Apply
  is ours) and the old "APPLY to use..." hint is gone. Unit test pins the
  order. Evidence: autopilot `click Options; click Page(Video)` shots,
  before (dist 802802d: grey panel, Resolution first, 40% number) vs after.
  Labels not localized (the strings are not in the loose resource/*.txt;
  probably in a VPK). Next: advancedvideo.res, body 11 x 5 (WIN32WIDE),
  rows 350 wide every 25 from 25 (Antialias, Filtering, VSync, Queued
  mode, Shader, CPU/effect, Model/texture, Paged pool) and a description
  pane at x 360 (LblDescriptionTitle y 25, LblDescription y 60, 180 wide);
  ours has 14 rows, so it needs a scroll or a taller body.
- 2026-10-20 autorun, Advanced Video DONE (4ca377c, autopilot shots only,
  not owner-checked). advancedvideo.res order now: Anti-aliasing,
  Filtering, VSync, Multicore (= DrpQueuedMode), Shader, Effect (=
  DrpCPUDetail), Model/texture, Paged pool (Portal 2 hides it on $OSX; kept
  as on Windows), then ours: Motion blur, FOV, Max frame rate, Graphics,
  Show frame rate, Show movement stats. Tiled frame 11 wide; 14 rows need
  a 7-tile body packed from 0 (as Audio), rows 350 units wide; description
  pane at 360: LblDescriptionTitle y 25 (VideoSettingsDescriptionTitle =
  BoldCn 24) and LblDescription y 60, 180 x 180 wrapped
  (VideoSettingsDescription = LightCn 17 w700, drawn Cn), black; filled from
  the hovered row (describe_rows), starting on the first. Description texts
  are our own (Valve's strings are not in the loose files). Unit test pins
  the order. Evidence: autopilot `click Options; click Page(Video); click
  Page(AdvancedVideo)` shots, before (dist 5d16315: grey panel) vs after.
  Hover updating the pane NOT autopilot-checked (no mouse-move command).
- 2026-10-20 autorun, Keyboard/Mouse DONE (9b0053b, autopilot shots only,
  not owner-checked). keyboardmouse.res (!$OSX): tiled dialog, body 7 x 5,
  rows every 25 from 25: BtnEditBindings (DefaultButton), DrpMouseYInvert,
  SldMouseSensitivity, DrpRawMouse, DrpMouseAcceleration,
  SldMouseAcceleration, DrpDeveloperConsole. Ours in that order (no
  acceleration-amount slider: m_customaccel_exponent not built), our Mouse
  Filter last. Labels are Portal 2's English strings, which ARE readable
  with `ls --cat resource/basemodui_english.txt | iconv -f UTF-16` (also
  gameui_english.txt): "EDIT KEYS/BUTTONS" (shown "Edit keys/buttons"),
  "Reverse Mouse", "Mouse Sensitivity", "Raw Mouse Input", "Mouse
  Acceleration", "Allow Developer Console"; title KEYBOARD/MOUSE
  (L4D360UI_KeyboardMouse). Unit test pins the order. Evidence: autopilot
  `click Options; click Page(KeyboardMouse)` shots, before (dist 4ca377c:
  grey panel, Customize keyboard first, sensitivity number) vs after;
  `click Page(Keys); click Done` opens the key list and returns. Not done:
  the key list itself (Portal 2's keybindings dialog) keeps the old grey
  panel; earlier pages' labels could now be localized from those files.
  Next: controller.res.
- 2026-10-20 autorun, Controller DONE (903118b, autopilot shots only, not
  owner-checked). Portal 2's PC page is controlleroptions.res (controller.res
  is an unused L4D leftover): tiled dialog body 7 x 5, rows every 25 from
  25: DrpController, BtnEditButtons ("Button Layout"), BtnEditSticks
  ("Stick Layout"), SldVertSens/SldHorizSens ("Vertical/Horizontal
  Sensitivity"), DrpVerticalLookType/DrpHorizontalLookType (Normal/
  Inverted), DrpVibration, SldAcceleration, BtnDefaults ("RESET TO
  DEFAULTS"). Ours in that order with those labels: Controller, Look
  Sensitivity (we have one joy_sensitivity, not pitch/yaw), Vertical Look
  Type, Vibration; the rest are not built (note: no gamepad input reads
  these settings yet). Title tabs got a 5-unit margin (title_tab_tiles):
  CONTROLLER sat exactly on the 3-tile boundary and clipped its R; other
  titles keep their tile counts (unit test). Evidence: autopilot `click
  Options; click Page(Controller)` shots, before (dist 2bf3d20: grey panel,
  "Game controller", "Reverse look", 1.0 number) vs after; `click Done`
  returns to the Options dialog. Next: the pause menu (ingamemainmenu.res).
- 2026-10-20 autorun, pause menu DONE (21f6353, autopilot shots only, not
  owner-checked). ingamemainmenu.res is a tiled CBaseModFrame dialog
  (dialogstyle 1), body 5 x 4, DefaultButton rows every 25 from 25; in
  single player the visible rows are BtnReturnToGame, BtnSaveGame,
  BtnLoadLastSave, BtnOptions, BtnExitToMainMenu ("RETURN TO GAME", "SAVE
  GAME", "LOAD LAST SAVE", "OPTIONS", "EXIT TO MAIN MENU" from
  basemodui_english.txt). Restart Level, Skip, Leaderboards, Rate map and
  the puzzle maker rows only show in challenge/community/puzzle maker maps,
  and Portal 2's pause menu has no Quit Game, so our old Load Game, Restart
  Level and Quit Game items are gone (Load Last Save loads the newest save,
  greyed without one; Load Game stays on the Single Player dialog).
  No footer (Return to Game/Esc close it). Guess: the title tab is blank,
  since neither the .res, the localization files nor client.dll strings give
  the frame a title (no pause shot in the reference gallery); owner, say if
  Portal 2 shows text there. Unit test pins the rows. Evidence: autopilot
  intro2 `click pause`, `click Options; click Back` shots, before (dist
  903118b: grey item column, 7 items) vs after (tiled dialog, Back returns
  to it). Next: in-game HUD/crosshair, then instructor hints, subtitles,
  loading screens.
- 2026-10-20 autorun, HUD DONE (b417bba, autopilot shots only, not
  owner-checked). hudlayout.res: in single player the only always-on HUD
  is the crosshair (HUDQuickInfo/HudPortalCrosshair), already built from
  the reverse-engineered layout (crosshair.rs: dots 10 px apart, brackets
  6,10 px off centre, alpha 150; checked in an autopilot shot, unchanged).
  The visible mismatch was the chapter title card: the maps'
  @chapter_title_text / @chapter_subtitle_text game_texts (x -1, y .55 /
  .6, effect 2 scan-out, fadein .06/char, fxtime .5 highlight from color2
  205 to white, hold 5, fadeout .5) are drawn by client.dll's CHudMessage
  in InGameChapterTitle (UniversLTStd-Cn 19) and InGameChapterSubtitle
  (UniversLTStd-BoldCn 34) from basemodui_scheme.res, i.e. a small
  "CHAPTER 2" above a big bold "THE COLD BOOT". Ours was a generic bold 54
  px "Chapter 2" over a small regular name with a 1 s fade. Now as the game
  (upper case, per-character scan-out, unit test). Evidence: autopilot
  sp_a2_laser_intro shots, before (dist 21f6353) vs after. Not checked:
  the scan-out mid-way in a shot (unit test only). Next: instructor hints
  (hudlayout has none; instructor fonts InstructorTitle in
  basemodui_scheme.res, scripts/instructor_*), then subtitles
  (HudCloseCaption c-250, y 276, 500x136, BgAlpha 128, CloseCaption_*
  fonts), loading screens.
- 2026-10-20 autorun, instructor hints DONE (9576999, autopilot shots only,
  not owner-checked). Reference: the gallery's "Pick Up" and "Create Blue
  Portal" shots: the icon (key cap / mouse tile from
  scripts/instructor_textures.txt) about 32 units square and a white
  caption to its right, no background box. InstructorTitle =
  UniversLTStd-BoldCn tall 18 (PC; Cn on $OSX), InstructorKeyBindings
  BoldCn 12 for the key on the cap (basemodui_scheme.res). Ours had a
  black 55% box, DIN bold 20 px, all hints stacked at 22% from the bottom.
  Now: hint_static 0 floats over hint_target (+hint_icon_offset), kept on
  screen; static hints centre under the crosshair at 2/3 of the height
  (guess, measured from the Create Blue Portal shot), stacked. Also fixed:
  the hint list was marked changed every frame (timeout retain), so the
  rows were rebuilt every frame. Captions are the game's upper-case
  strings (Valve_Hint_Look_Up "LOOK UP"). Unit test (hint_anchor).
  Evidence: autopilot sp_a1_intro1 `input hint_press_to_crouch showhint;
  input hint_press_to_use_bed showhint` shots, before (dist b417bba: boxed
  rows at the bottom) vs after (LOOK UP under the crosshair, SLEEP on the
  bed button at the screen edge; looking away it drops under the crosshair).
  Not done: Portal 2's off-screen arrow (we clamp to the edge or fall back
  under the crosshair), hint fade in/out, instructor_lessons.txt lessons
  (game-driven hints like Pick Up / Create Blue Portal are not built).
  Next: subtitles (hudlayout HudCloseCaption), then loading screens.
- 2026-10-20 autorun, subtitles DONE (f475223, autopilot shots only, not
  owner-checked). hudlayout.res HudCloseCaption: panel c-250, y 276,
  500 x 136 units, BgAlpha 128. Ours was a box hugging the text at 8% from
  the bottom, 60% max width, centred, in the generic regular font. Now a
  500-unit box centred with its bottom at 412 units (68 from the bottom),
  lines stacked up from the bottom, clipped at 136 tall, left-aligned (the
  install's caption files never use <center>), 12-unit inset, 6-unit
  rounded corners, CloseCaption_Normal = UniversLTStd-Cn tall 20 (= our
  MEDIUM size; the size setting is now vgui tall units). Background default
  128/255, converted to the linear-blend alpha that looks the same (0.78).
  Unit test. Evidence: sp_a1_intro2 announcer line at 9 s, before (dist
  9576999: centred, wide DIN-like text) vs after (box measured 585 px of a
  1000x562 shot = 500 units). Not done: GrowTime 0.25 / item fade in 0.15
  / fade out 0.3 animations; the inset and corner size are our estimate
  (not in the .res). Next: loading screens.
- 2026-10-20 autorun, loading screens DONE (0bc1dd1, autopilot shots only,
  not owner-checked). Before: no loading screen at all; the old map's last
  frame froze during the load. loadingprogress.res has only hidden anchors;
  client.dll fills it: background vgui/loading_screens/loadingscreen_a%d
  (act = the number after sp_a; sp_a3_00/01 use act 2; sp_a5_credits and
  other maps loadingscreen_default_*), "%s_%d%s" for each _1.._4 whose .vmt
  exists, "_widescreen" on wide screens, stretched full screen; Portal 2
  cycles them (ui_loadingscreen_autotransition_time 5, transition_time 1,
  fadein 1); WorkingAnim r75,10 65 x 65 = vgui/spinner (8 frames); dots
  vgui/loadbar_dots (16 px cells white/blue/orange/empty) ending at the
  LoadingProgess anchor r11,r11: 15 dots every 11 units, ~8 units across,
  lit = blue, unlit = faint white (measured from the gallery's 1920x1080
  loading shot). Ours (loading.rs): same art/spinner/dots/positions;
  load_level waits 2 frames on non-menu loads so the screen is drawn before
  the blocking load, and it goes 3 frames after (unlit bar before, full
  after). Guesses: which of the act's pictures shows (per-map hash; our
  loads are one blocking frame, so no slideshow/fades), spinner 10 fps.
  P2_LOADING_HOLD=<s> keeps the screen up for shots. Unit tests (act and
  art choice). Evidence: autopilot `portal2 sp_a1_intro2` with
  P2_LOADING_HOLD: act 1 art, spinner top-right, dot row bottom-right as in
  the reference, gone after the hold; `con map sp_a2_laser_intro` logs
  "loading screen for sp_a2_laser_intro" (act 2 art not captured in a shot:
  the autopilot clock restarts per level). Main-menu loads (startup
  background, Exit to Main Menu) show no loading screen.

### T-176 Can't catch or grab objects in the air  (P1, FIXED 2bf3d20, awaiting owner replay)
- Owner, 2026-10-20 (after playtest 9): "you can't catch falling things or
  grab things midair, like the faithplate level i couldnt continue because
  i couldnt grab the cube and wheatly i couldnt catch when he fell earlier
  in the playthrough". Faith plate level: probably sp_a2_catapult_intro
  (the owner had reached sp_a2_laser_over_goo); Wheatley falling: sp_a1_intro7
  (after the wall panel) or sp_a1_wakeup.
- Portal 2 lets +use pick up a cube or Wheatley in flight. Check the use
  trace/filter and the pickup conditions for moving or airborne bodies
  (speed limits, sleeping/contact requirements, the use-cone range against a
  fast target), and that the held-object code accepts a body with velocity.
  Related: T-105/T-170 (carrying), T-004 (prove catapult_intro).
- Verify: autopilot `use` on a cube mid-flight off a faith plate and on a
  dropped Wheatley, before/after.
- Cause: use_key only picked up a prop hit exactly by one thin ray from the
  eye. A cube flying off a faith plate or Wheatley dropping crosses that
  line for a frame or two, so in practice they could not be grabbed (a
  falling cube WAS taken when the autopilot aimed exactly at it; the
  pickup code itself accepts moving bodies).
- Fix (2bf3d20): like Source's FindUseEntity, when the ray hits no prop,
  take the prop nearest the view line inside a ~26 degree cone whose near
  side is within use range, not behind the surface the ray hit, and with a
  clear line of sight (props.rs cone_pick, unit test). Buttons still win
  (their search runs first).
- Evidence (autopilot only): sp_a1_intro2, cube 55 u away, view turned ~20
  degrees off it: old build ray hits nothing, cube not taken; new build
  cone picks it (cos 0.937, sight clear) and the next use drops it. Ground
  carry still follows the player after walking (cube 39 u in front).
  Not replayed on catapult_intro's real throw or Wheatley's drop in intro7
  (scripted drops kept hitting the arrival elevator's walls).


### T-169 sp_a2_laser_over_goo: GLaDOS never moves the walls, the way on is blocked  (P1, FIXED a1cd026, awaiting owner replay)
- Owner playtest 9: "there's a point where glados never moves the walls so
  I can't move past here" (entry corridor, sign 04/22; owner noclipped).
- Cause: the corridor repair trigger (model *88, angles 0 90 9) was placed
  unrotated, beside the corridor, so `corridor_repair` never fired; also
  Kill on a func_door was ignored, so blocking_door_2 (kills itself when
  open) left its collider in the corridor. Fix: trigger boxes and brush
  touch tests follow the entity angles; Kill despawns func_door/movelinear.
- Evidence (autopilot only, walking from the elevator): before, feet stop at
  (4000,-2256), walls never move; after, corridor_repair + repair_wall_1..3
  fire, the corridor opens, the player walks to the landing (3632,-1616,32)
  and entry_landing_open fires. Unit test trigger_box_turns_with_its_angles.

### T-163 sp_a1_wakeup pod: the player can jump out of the hatch; a stone-brick block appears there  (P2, fixed 8f8c8b1, autopilot only)
- Playtest 6. Cause: brush entities with startdisabled 1 were never built
  (no mesh, no collider), so an Enable had nothing to switch on. The hatch is
  sealed by basement_breakers_upper_blocker (toolsplayerclip ring, z 392-576,
  enabled by the socket relay). Fix: build them hidden with ColliderDisabled
  (Always Solid func_brushes keep collision); applies to every map.
- Evidence (autopilot: setpos on the lift, enable blocker, trigger
  basement_breakers_start, walk + jump outward at the top): before (dist
  5cfe749) feet reached (9133 1017 428) outside the hatch; after, the
  blocker has colliders and feet stay on the lift (8988 1046 351).
- The "stone-brick block" was not seen by autopilot (the blocker is player
  clip, never drawn); owner replay should say if it is still there.

- T-132 Fizzler field not drawn: every trigger_* brush was skipped, so
  visible trigger_portal_cleansers never drew their effects/fizzler faces
  (SolidEnergy, a green-only ripple mask, wasn't translated either). Now
  (99392c0) visible cleansers draw, SolidEnergy is unlit additive with the
  mask tinted by a quarter of $FLOW_VORTEX_COLOR (no flow animation yet),
  Enable/Disable shows/hides it. Autopilot only (intro2 exit, `setpos -320
  740 -60 0 -90`): before no field, after a blue field, hidden on
  FireUser1, back on FireUser2; unit test visible_cleansers_draw_their_field.
  Not checked: fizzling through it (logic unchanged); startdisabled
  cleansers still get no mesh (check at sp_a2_fizzler_intro, T-007).
- T-130 Portal shots pass handrails: models compiled `$contents grate`
  (props_bts/handrail_*) now let portal shots through like grate brushes.
  intro5 rail at (380,-330): before, the shot stopped at the rail; after, it
  reaches the wall behind. Loose cubes already pass shots (dynamic bodies
  are skipped); not reproduced in intro6, where the dropped cube landed on
  non-portalable dirt floor at (1947,381) tilted, ~25 u up a wall. Owner:
  replay intro6 and say where the cube was when the shot failed.
- T-129 Stray elevator screens: movie panels were drawn extending to the
  entity's right instead of its left, so the ring covered its own exit gap
  (laser_stairs: @arrival_sign_1/_24 across the stairs and leaving trigger).
  Autopilot before/after shots of the laser_stairs arrival; intro4's exit room
  (same cause, playtest 3) still needs the owner's replay.
- T-128 sp_a2_laser_stairs stairs rise: held sequences replay on a negative
  SetPlaybackRate (8ac53a2; screenshots before/after).
- T-103 sp_a1_wakeup breaker lift carries the player (SetMaxSpeed); owner saw it rise in playtest 6.
- T-106 Walk into the breaker room standing (hull fits an exact 72 u).
- T-121 A cube resting where a portal opens falls in (contacts dropped in the hole).
- T-122 func_portal_detector fires (intro7 progresses).
- T-126 Tilted map portal drawn whole (quad stands 1 u off).
- T-127 The panel GLaDOS swings into place takes a portal (sensor colliders).
- T-119 Doors stay drawn while opening; no area-portal black plane.
- T-110 An actor's new line cuts its previous one (voice channel).
- T-021 Owner checks: intro1 ride, wakeup socket, sp_a2_intro portal route.

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
