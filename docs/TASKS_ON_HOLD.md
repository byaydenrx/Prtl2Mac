# Tasks on hold

Moved verbatim from TASKS.md on 2026-10-21 (housekeeping) so sessions stop
reading 46 KB of tasks they must not work on. Statuses and wording are
unchanged. These stay on hold until the owner lifts the hold; when one
comes back, move its entry back into TASKS.md.

Find one entry without reading this file whole:
`awk '/^### T-179 /{on=1} on&&/^##/&&!/^### T-179 /{exit} on' docs/TASKS_ON_HOLD.md`

## On hold: menus (owner 2026-10-21)
Owner round 6: "all menu items are fixed for now, buttons just don't function
but i don't want those being worked on rn". Don't touch menu code until the
owner says so. Kept here for later: dead buttons, T-179 flip polish
("semi fixed, needs work"), T-188 gamepad, T-189 raw mouse/key conflicts.
- Owner notes 2026-10-21 round 4 (build d36e250), not worked yet:
  (1) the screen turns black after each flip animation plays, caused by
  the round-3 fix (T-196); (2) the in-game pause menu is inaccurate; owner
  reference shots in ~/Documents/Screenshots/pausemenu (2704x1756 CrossOver
  window, a different size/aspect from the earlier shots) (T-195);
  (3) grid/box inaccuracies: tile spacing off, some tiles shifted a few
  pixels sideways instead of one flat, even grid (T-197); (4) the paused
  game behind the pause menu should be blurred/bloomed and slightly
  desaturated (T-195).

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

### T-179 Tile-flip transitions  (P2, SEMI FIXED: owner says it needs work; spec 1, 10, 11, 15)
- Owner round 5 (2026-10-21, build b8a2d76): "semi fixed needs work". Waiting
  for what still looks off (a short screen recording of Windows vs ours helps).
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

## On hold: waiting for owner screenshots (owner 2026-10-21 round 7)
"ill send screenshots another time so put those tasks on hold". Not for
autoruns until the screenshots arrive.

### T-173 Portal gun view model: hold beam, carry anims, fizzler shake  (P3, OPEN: hold beam "looks terrible"; fizzler shake owner-confirmed)
- Owner round 5 (2026-10-21): "the gun hold beam is not fixed it looks
  terrible, the fizzler shakes are fixed". A screenshot or clip of ours while
  holding a cube would pin down what is wrong.
- Owner playtest 9: "portal gun still doesn't have that electricity when
  picking things up and doesn't spark/shake when going through the
  emancipation grids" (also T-150's gun beam).
- Cause: nothing drove the gun while carrying or at a grid. p2_fx only had
  the top light / muzzle flash; the viewmodel never played its carry
  sequences (ACT_VM_PICKUP idle_to_carrying, idle_carrying, carrying_to_idle)
  and played `fizzle` only on a failed shot.
- Fix: p2_core `GunHolding` (held object's world centre, kept by p2_game
  props::gun_holding; through a portal the hold point on this side) and
  `GunCleansed` (written on trigger_portal_cleanser touch when the gun can
  fire). p2_fx spawns `portalgun_beam_holding_FP` (portalgun.pcf: CP0 muzzle,
  CP1 object; the end point is rescaled for the 50-degree viewmodel FOV so it
  lands on the object; unit test hold_beam_end_matches_world_projection)
  on the viewmodel layer with the `Weapon_Portalgun.HoldSound` loop, only
  while the gun is shown. p2_view plays the carry sequences and `fizzle` on
  GunCleansed; p2_fx plays `Portal.FizzlerShimmy`.
- Evidence (autopilot only, sp_a1_intro4): `goto section_2_box_2 70; use`:
  after build shows the electric beam from the muzzle to the cube centre and
  the raised carrying pose, log "play Weapon_Portalgun.HoldSound"; before
  build (dist 69426f1) the cube floats with no beam and the idle pose.
  Walking through fizzler_brush: log "play Portal.FizzlerShimmy" and the gun
  mid-`fizzle` in the shot; before: nothing.
- Not done: a spark particle on the gun at the grid (no Valve effect name
  confirmed for it; the fizzle sequence shows no particle events in
  animinfo); the beam particles don't follow the gun between restarts
  ("Movement Lock to Saved Position Along Path" not implemented; the effect
  restarts often enough that it looked right in the shot). Owner replay:
  does the beam/fizzle feel like Windows Portal 2?

### T-174 Elevator-room videos buggy and very low quality  (P3, OPEN: quality better, panel layout wrong)
- Owner round 5 (2026-10-21): "the quality looks a little better but the
  panels are like awkwardly not together and some are in the wrong places".
  Needs owner screenshots (which map, which screens).
- Owner playtest 9: "the videos that play in the elevator rooms are a bit
  buggy and super low quality" (the vgui movie screens around the arrival
  and departure elevators; T-129 fixed their placement).
- Cause: movies.rs decoded every Bink at 192 px wide and 12 fps (the
  elevator movies are 640x400 / 480x640 at 24 fps), so the big panels were
  blurry and choppy. Now decoded at their own size (up to 640 wide) and
  frame rate, kept as YUV 4:2:0 and converted per shown frame; the old
  `*_192_12.rgba` caches are removed. Unit tests for size and colour.
- Evidence (autopilot only): sp_a2_laser_stairs arrival, `lookat
  @arrival_sign_3/_5` at 9-10 s, dist fd1310c vs b250e5f: the Aperture logo
  and "Thermal Discouragement" text go from smeared to sharp. "Buggy" not
  pinned down beyond the choppy 12 fps: owner, say what looks wrong if it
  persists (entry/exit_emergency.bik don't exist in the game files; that
  warning is harmless).

### T-145 (moved back to TASKS.md 2026-10-22: owner wants it fixed now)
### T-147 intro2: the screen goes black right as the first door opens  (DONE 2026-10-22: owner sees no black screen; was: P2, BLOCKED (needs owner): not reproduced)
- Section 1; compare T-119. Screenshot 7 (intro2): a dark shard in a door.
- 2026-10-21 autorun: NOT REPRODUCED. Owner saw it on build 1faf8d2
  (playtest 8, which already had the T-119 door/area-portal fix 6d75b6f);
  not reported again in playtest 9. Autopilot sp_a1_intro2 on the current
  build and on dist 910da9a: the natural path (elevator opens, walk to
  @entry_door-testchamber_door; arrival_elevator-leaving_elevator_trigger
  fires @entry_door_open_rl after 0.5 s) with shots every 0.2-0.5 s while
  the door opens and the player walks through, plus off-axis looks
  (pitch +10/-20, yaw +-25, looking back) mid-opening: the door, its
  frame and the chamber behind stay drawn in every frame, no black frame
  (looking back shows the dark stairwell wall, lit normally). The exit
  door was not checked by walking. No cause found, so no attempt branch.
- Needs the owner: does it still happen on the current build; if so a
  screenshot or a short video, and whether it is the whole screen or a
  shape in the door (screenshot 7's dark shard).

### T-149 Chamber signs blank; screens and lights wrong  (DONE 2026-10-22: owner: screens and lights fine everywhere; was: P2, BLOCKED (needs owner): signs drawn; intro5 screens/lights and big number not reproduced)
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
- Run 2026-10-21 09:26 (autorun, NOT REPRODUCED, no code change):
  - Big number: autopilot sp_a2_laser_over_goo (`input
    instanceauto69-info_sign-info_panel setactive; goto ... 320; lookat;
    look -30 0`) shows a big "04" over "04/22", matching
    sp_lightboard_icons.txt (level_number 4, total 22); signs.rs is
    unchanged since d314ea7 (before the owner's build 014ab6e) and stamps
    the same digit cells for the big number and the label, so a big "01"
    with "04/22" could not be produced. The owner's shot 2.45.36 PM is not
    on disk. Owner: resend it (or a Windows shot of that sign) if the
    sign still looks wrong.
  - intro5 screens: the elevator vgui_movie_displays get SetMovie
    media\exercises_vert.bik from videos/video_splitter.nut (the map's
    entry for sp_a1_intro5), split across the panels with custom UVs, as
    Windows does. The "entry_emergency.bik / exit_emergency.bik could not
    be decoded" warnings come from the video masters' moviefilename
    keyvalue: those files are not in the depot and the script's
    OVERRIDE_VIDEOS is 0, so Windows never plays them either (harmless).
    Video quality/bugginess is T-174.
  - intro5 lights: chamber indicator dots (env_texturetoggle overlays),
    button glow and room lights look plausible in autopilot shots; no
    defect could be identified without a reference. Owner: a screenshot
    of what looks wrong in intro5/intro6 ("screens and lights") is needed.
  - No fix attempt branch (no plausible cause to try).

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

## On hold (owner 2026-10-21 round 7): skip for now / wait for screenshots
T-164 skip for now; T-152, T-151, T-154 wait for screenshots; T-120's ride
jitter is fixed (owner), its VO timing overlaps T-172 (confirmed fixed).

### T-164 Spiky, torn animated debris  (P3, OPEN: CPU-skinning attempt REVERTED e88ae41)
- Owner round 7 (2026-10-21, build 6e0efbf): the CPU skinning made moving
  things on the intro1 ride shake; reverted at the owner's request (code in
  c34006e). Next try: check the skinned positions per frame (bone transforms
  from BonePart vs the rigid parts' interpolation) before redoing it.
- 2026-10-21: merged untested at the owner's request so the replay has it.
- Owner round 5 (2026-10-21): still saw spikes on the intro1 ride, but in
  build b8a2d76, which does not contain attempt/T-164. Autopilot couldn't
  show the ride animating from outside (`@rl_container_ride Trigger` +
  setpos: the container stays put in both builds), so no before/after.
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

### T-152 Lighting and texture quality below the original  (P3, STUCK 2026-10-21: bumped-model path too large for one run; plan below)
- Owner playtest 9 (2026-10-20, build 014ab6e): "lighting/textures being low quality is still an issue". Raised to P3.
- Scrap pieces in the intro3 fall; turrets in tubes (wakeup); bumped
  VertexLitGeneric (models with $bumpmap) still drawn without bump.
- Autorun 2026-10-21 (STUCK, no code change): no owner shot pins one
  surface, so the only concrete item is the bumped VertexLitGeneric path.
  Texture filtering is already 16x anisotropic with mips (vtf.rs,
  valve_render.rs), so that's not the cause. Findings for the next attempt
  (`examples/shaders vertexlit_and_unlit_generic_bump_ps20b|vs20 --statics`):
  ps20b has 32768 static x 80 dynamic combos (544 stored); bumped statics use
  BumpmapSampler@s3, cAmbientCube@c5 (6 regs), cLightInfo@c13,
  g_ShaderControls@c12, cLightScale@c30, g_DiffuseModulation@c1. vs20 is 64 x
  24 (24 stored), reads a TANGENT input (Input6), has cLightInfo@c27,
  g_bLightEnabled@b0, cModel@c58, and NO COLOR1 static-light input. So a
  bumped prop needs (1) VVD tangents in ModelMesh (model_vertices writes a
  dummy tangent today), (2) a per-prop ambient cube (6 colours, from the same
  leaf ambient samples that bake COLOR1 now) plus the nearest local lights
  packed into cLightInfo, (3) the static/dynamic combo digits mapped like
  vertex_lit_generic's (NUM_LIGHTS in the dynamic index), (4) a new
  ConstRecipe. Keep the current non-bump path as the fallback when any step
  fails. Next run: start with (1)+(2) on an attempt branch, check the intro3
  rubble and the wakeup tube turrets before/after.

### T-151 Water like Portal 2's (reflections, murk), intro3 and sp_a2_laser_intro  (P4, OPEN: attempt dropped 2026-10-21)
- 2026-10-21 check: attempt/T-151 (2df932d) turned the intro3 water bright
  glossy white in a dark room (reflects the raw white sky; Source fogs the
  sky to the fog colour on fogged maps). Owner: close it; branch deleted
  (recreate with `git branch attempt/T-151 2df932d`).
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

### T-154 Floating vine/moss/grass texture by the centre wall (intro5)  (P4, TODO: owner says not fixed)
- Owner playtest 9 (2026-10-20, build 014ab6e): not fixed.

### T-120 Elevators  (DONE 2026-10-22: owner: late voice lines fixed, every elevator works; was: P2, OPEN: VO timing only; jitter, car sounds, thumps owner-confirmed)
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
