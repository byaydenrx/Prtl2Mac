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

## Priority order (2026-10-20, after owner playtest 9)
P0:
1. (none; T-168 box-button crash owner-confirmed fixed.)
P1:
2. (none; T-169 fixed a1cd026, awaiting owner replay.)
P2:
3. T-162 OWNER PRIORITY: the UI as close to Portal 2's as possible, one
   screen per run until all match (done: main menu, SP dialog, New Game,
   Load/Save Game, Options dialog, Audio; left: Video (+ advanced),
   Keyboard/Mouse, Controller, pause menu, HUD/crosshair/hints/
   subtitles, loading screens); T-162's entry lists done/left screens. Any new crash or
   softlock still goes first.
4. T-170 carrying regressed (pushed back at walls, can't push a cube up a wall).
5. T-145 lit cubes turn black, spikes under buttons, cubes jitter on buttons.
6. T-172 voice lines start late (stretches elevator rides; T-120's VO part).
7. T-135 things fall through elevators (turrets, rubble).
8. T-139 music not playing with ambience; soundscapes.
9. T-112 GLaDOS chamber floor still murky brown (with T-151 water).
10. T-144 clipboard should turn to face the player.
11. T-149 intro5 screens/lights; sign big number check.
12. T-147 intro2 black screen at the first door; T-113 hints vanish.
P3:
13. T-171 FPS drops looking at portals.
14. T-148 portals behind glass drawn on top (reopened).
15. T-117 shadows still blocky.
16. T-152 lighting/texture quality.
17. T-153 GLaDOS wake-up particles; T-173 gun hold beam and fizzler spark.
18. T-140 game-placed portal shake/sound (reopened).
19. T-174 elevator videos; T-175 intro1 ride outside fog/sky; T-164 debris
    (try attempt/T-164); T-116 window (owner screenshots coming).
P4:
20. T-151 water (try attempt/T-151), T-150 particles, T-146 half-stuck door,
    T-154 floating moss.
Then the Backlog (prove and sweep the campaign).

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

### T-170 Carrying regressed: pushed back at walls, can't push a cube up a wall, not smooth  (P2, TODO)
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

### T-145 Buttons don't animate; cubes don't change colour on a button  (P2, REOPENED: lit cubes turn black, spikes under buttons, jitter)
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

### T-162 Menu animations, logo alignment, styling  (P2, REOPENED: owner priority, style the UI like Portal 2)
- Owner playtest 9 (2026-10-20, build 014ab6e): T-160/T-161 confirmed, but "the ui not being styled like portal 2 ... is big for me, please get that as close as possible". Raised to P2, right after softlocks (owner, 2026-10-20). Screens done: main menu (e6e5e91), Single Player (8737034), New Game (3ca8329), Load/Save Game (47757a0), Options dialog (2a6921d), Audio (802802d), Video (5d16315), see below. Left: the Settings panels Advanced Video, Keyboard/Mouse, Controller, pause menu, HUD/crosshair/hints/subtitles, loading screens. Covers the main, pause, options, new game and load menus (fonts, colours, panels, transitions) against the install's resource/ui/basemodui/*.res and basemodui_scheme.res.
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
