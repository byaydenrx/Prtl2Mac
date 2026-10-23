# Owner playtest history

Raw owner reports, kept for reference (moved out of TASKS.md 2026-10-20).
Every item is tracked as a task in TASKS.md; this file is not a to-do list.

## Owner playtest issues (P2)
- T-110 (NEEDS_VERIFICATION) Fix: speech plays on its actor's voice channel, so an actor's new line cuts the previous one (Source CHAN_VOICE); busyactor 2/3 cancel the actor's running scene. Verified by log in intro1 (Wheatley and announcer lines). Different speakers can still overlap, as in Source. Report: jumping/saying "apple" while Wheatley talks, opening the door early; Portal 2 queues or interrupts by scene priority.
- T-111 Music stops at the stairs (wakeup, way to GLaDOS).
- T-112 Missing textures: hallway to GLaDOS, after the jumps (plus a black wall you can walk through), GLaDOS chamber floor drawn as a warm sky, a blue flat wall behind the intro4 elevator.
- T-113 Instructor hints vanish after a few seconds if not done. (Key labels FIXED 2026-10-20: white text on the white key cap, now dark.)
- T-114 intro1: the Wheatley door opens on approach without pressing E.
- T-115 intro1: wake-up animation still wrong; no wake/sleep animation on the bed.
- T-116 intro1: the first room's window is a grey block (should be blinds etc.); the TV screen sits forward and up from the TV; glitchy screen at the start of the ride; odd outside lighting on the ride.
- T-117 Shadows are blocky (shadow-map resolution and filtering).
- T-118 intro4: loud glass-breaking sound (check against the original); the door animation cuts to black.

## Owner playtest 3 (2026-10-19, build 2cf59a9), by map (P2 unless noted)
- intro1: wake-up animation; blocky shadows (T-117); key hints lack labels (E, SPACE: T-113); Wheatley and announcer overlap in the dark room; the moving room jerks the camera and moves the player around; floating parts during the room move; Wheatley moves oddly after the ride and is a black box seen from below during it; 12:00 clock blinks its background too (only the digits should); Chell seen through the first portal holding a gun she hasn't got; no press-down animation on the button; saying "apple" when asked skipped his next line; when the door opens Wheatley's first lines and the announcer talk over each other (neither understandable).
- intro2: cubes don't sit still on buttons (autopilot: a dropped cube settles in 0.5 s with one press/unpress flicker); stairs to the exit elevator missing (floor and sides empty).
- intro3: the water below the falling platform is far too bright orange.
- intro4: glass-break sound on arrival (T-118); a stray screen in front of the stairs to the elevator.
- intro5: a floating grass texture; a missing wall panel.
- intro7: the auto-placed orange portal (portal_b_rm4, tilted 75 deg onto a panel) shows only half (T-126).

## Owner playtest 5 (2026-10-20, build 33bca15), by map (P2 unless noted)
- intro7: the turn-away check worked but felt unreliable (took a while); turning back to pick Wheatley up, he floated attached to nothing; the moving wall that opens disappears once its open animation ends (passable).
- sp_a1_wakeup: missing textures in the hallway to GLaDOS's chamber and her chamber floor drawn as sky (T-112); crouch needed to enter the main breaker room (T-106), which drops Wheatley, who then floats there; he still plugs in, but the breaker platform rises without the player (T-103, P1 blocker); noclip couldn't fly up past it either (player stuck).

## Owner playtest 6 (2026-10-20, build 06cf3e3), by map (P2 unless noted)
- sp_a1_wakeup: the lift rises (T-103 ok) but as the breaker platform turns the player should turn with it (rotating mover doesn't carry/rotate the player); the player can jump out of the hatch (shouldn't be possible) and a stone-brick-looking block appears there; GLaDOS wake-up scene is buggy: white particles all around her that shouldn't be there; the portal gun didn't visibly drop to the floor (check); the player is made to look the wrong way.
- sp_a2_intro (P1 blocker, T-104): ~10 s of blue screen before anything shows; the fall should play as a cutscene but doesn't; the player already has the portal gun (should pick it up again); the player lands in the incinerator instead of on the platform and is stuck on a white screen.

## Owner playtest 7 (2026-10-20, build b2f0080), by map (0 = intro1 ... 6 = intro7, then wakeup, sp_a2_intro)
- REGRESSION (P0, DONE: owner playtest 8 sees no weird objects, cubes and buttons drawn): "weird objects" everywhere (elevators, elevator rooms, chamber entrances, moving stairs, attached to GLaDOS) and floor buttons disappearing from intro4 on. Owner screenshots 2026-10-20 12-16 (intro3) explained, see "Owner screenshots 2026-10-20" below: untextured slabs (fixed), static props left out (fixed), elevator-tube rubble (real map content, tube_blockage). Owner to recheck the other maps on the new build.
- sp_a2_intro fall: the crushing arms in the shaft look wrong (owner screenshot 2026-10-20: spiky dark shapes around the central arm).
- intro1: intro animations now right; Wheatley's first lines and the announcer still overlap; the box ride is less buggy but walls vanish and reappear before breaking off; Wheatley above is a black box; the whole 12:00 clock blinks (only the digits should); through portals Chell is seen holding a portal gun the player doesn't have; cubes twitch on buttons and buttons have no press animation.
- intro2: cubes through portals not seamless; exit stairs still missing (gap with no floor or sides).
- intro3: water after the fall still wrong; some textures flat white/empty before it; stray objects in the elevator room (one inside the elevator, one behind the door).
- intro4: stray objects in/after the elevator, in the level and next chamber; chamber doors look wrong; buttons gone; a stray object after the glass breaks underneath.
- intro5: stray objects in the elevator room and moving ones at the chamber entrance and moving stairs; a voice line starts late; the elevator descends far too long.
- intro6: stray objects; a cube still didn't fall into a portal (noclip needed).
- intro7: Wheatley's rail connector invisible; the turn-away check still slow; walking into the breaker room drops Wheatley (he auto-plugs).
- sp_a1_wakeup: the player rises with the breaker platform but doesn't turn with it; GLaDOS wake-up still has white particles and stray objects attached to her; floor texture missing in her chamber; a portal shows where she drops the player that shouldn't.
- sp_a2_intro: fall cutscene right and lands in the right place; the player gets a portal gun back on landing (FIXED: the map gives the gun by a console command, which the start-without-gun rule didn't count); the dual gun pickup works; after GLaDOS moves a wall to be portaled, it won't take a portal (FIXED, see T-127).

## Owner screenshots 2026-10-20 00:19-00:28 (~/Documents/Screenshots), by number
1. intro1 window a grey block (T-116) - TODO.
2. Blocky shadow of the hanging planter (T-117) - TODO.
3. Key hints without the key - FIXED, owner confirmed (playtest 8).
4. intro1 ride: Wheatley a black box above - TODO.
5. intro1 ride: spiky pieces (wall destruction / docking station debris) - TODO, check skinning: animated props are drawn as rigid per-bone triangles (models.rs split_by_bone), soft-weighted debris would tear.
6. Chell seen through a portal holding a gun before having one - FIXED, owner confirmed (playtest 8): nogun_* sequences and no world gun while no gun can fire.
7. A dark shard in a door (intro2) - TODO, not looked at.
8. Invisible button, buggy held cube (intro2) - see T-107.
9. intro2 exit stairs invisible, a gap - FIXED: static props with flag 0x4 were skipped (769 of intro1's 1080 props too); stairs drawn and walked down to the elevator (autopilot).
10. intro3 flat lavender-grey slabs around the pedestal - FIXED: VertexLitGeneric $normalmapalphaenvmapmask now masks the envmap with the bump alpha (black_floor_metal_004b rubble). Bump detail itself still missing (bumped VLG not driven).
11. intro3 water bright tan - FIXED: $lightmapwaterfog lights the fog; flowing water's normals in world space at $flow_bumpstrength; no sky reflection on fogged maps (sky_white turned it white). Now dark murky brown.
12-16. intro3 "weird objects": the flat slabs (10), the previously missing static props, and the elevator-tube rubble (tube_blockage_*, real map content) - owner to recheck. Shot 15's tilted panel and 14's beams not yet matched to an entity.

## Owner playtest 8 (2026-10-20, build 1faf8d2), by elevator count
Sections: 0 intro1, 1 intro2, 2 intro3, 3 intro4, 4 intro5/intro6, 5 intro7 +
sp_a1_wakeup + sp_a2_intro, 6 sp_a2_laser_intro, 7 sp_a2_laser_stairs (from
the content; many issues are global).
Confirmed fixed by the owner: key labels (T-113 part), Chell without the gun
before getting it, cubes/buttons drawn and no more "weird objects" (P0
regression). Not reported again: intro2 exit stairs.

### Owner playtest 8, the owner's words (2026-10-20, build 1faf8d2)
Sections by elevators taken: 0 none (intro1), 1, 2, ... Many bugs are global.

UI: animations between clicking on panel sections flipping out; bg music doesn't play; bg is a static image rather than the video; some main-menu options missing; Portal logo not aligned. First menu should be "Play Single Player, Play Cooperative Game, Community Test Chambers, Options, Extra, Quit" (some may do nothing for now). Play Single Player: "Continue Game, New Game, Load Game, Challenge Mode"; New Game a vertical menu, not the gallery. Elements look off; maybe take the UI from the Windows version.

0: blocky shadows and the window not fixed; keys fixed; Wheatley's first sequence still overlaps the announcer (door shouldn't open until that's over); a flash of the walls going away and coming back before the walls start breaking in the moving sequence; Wheatley still a black box; weird spiky elements in the moving sequence; Chell shows without the portal gun (good); the whole 12:00 clock flashes including the background, not just the text; props missing in the room where the portal first opens (the clipboard and the movable clock); buttons don't move when clicked or a cube placed; cubes don't change colour when activated; the incandescent particle field doesn't show or work (never has); the first cube-button door should get half stuck and doesn't.

1: the screen renders black right at the first opening of the door; ambient sounds and button clicks don't play; the screen should shake a little and play a sound when the game places a portal (the first orange portal); a portal behind normal glass shows but not behind broken glass; portals behind glass look on top of the glass; elevator rides jittery.

2: floor gone right in front of the door exiting the first elevator room, and the roof right outside that door; lighting strange on some scrap pieces where you fall; bringing cubes through portals still not seamless; the white chamber-number screens ("02") are blank in all levels; no smoke when the fall starts, no splash or sounds hitting the water; the water should look like water with reflections; the roof after grabbing the gun and moving up is missing until the next gun use, then the floor is missing there; the floor is missing entering the elevator room.

3: a super loud glass-breaking sound as you load in while still in the elevator; buttons and cubes render, no more weird objects; a random screen at the entrance of the exit elevator room.

4: can't shoot portals through handrails; a weird vine/moss texture by the wall in the centre of the chamber; the voice line didn't start when pressing the second button but halfway down the elevator, extending the ride; weird hitting sounds going down the elevator; screens and lights here not rendering correctly; couldn't get the cube down the portal without noclip (should be able to place a portal under the cube by shooting through it); the gun's electric particles don't show when picking things up; no motion blur/speed lines moving fast.

5: no water drips or ambient sounds; lighting and textures look worse than the original; the turn-around sequence still takes a little extra time; Wheatley still floats after the wall panel moves instead of falling, and the panel disappears after moving; turrets in tubes lit wrong; "her chamber" floor brown and flat like the water was; walking into the main breaker room with Wheatley pressed against the wall he falls rather than comes closer (also cubes, anything held); the floor rotates going up in the escape pod and the player doesn't; GLaDOS awakening still buggy, can't see it for white particles; her arms float unattached when grabbing Chell and Wheatley; a portal still shows in the tube right before she drops us; the transition into the fall isn't smooth but the fall works and lands in the right place; the gun's particles show through the wall after falling; music stops abruptly sometimes; portals can't be placed as close together as in the real game; turrets fall through the exit elevator.

6: lasers ugly; water ugly, not murky with reflections; things fall through the exit elevator like in 5.

7: a screen in the first elevator room blocks the staircase you have to go through; can't shoot portals at cameras to break them; the staircase after the puzzle doesn't move (unplayable).

## Owner playtest 9 (2026-10-20, build 014ab6e), the owner's words
Tracked in TASKS.md (new T-168..T-175; reopened T-145, T-148, T-140, T-117,
T-162; confirmed fixes moved to Done).

GitHub notes: T-131 is fixed, T-114 is fixed, in T-120 jittery elevators are fixed, vo timing is not, thumps are fixed and rubble/turrets still fall through the elevator (T-135), T-133 is fixed including the panel part, T-134 is fixed, T-136 is fixed, T-137 and T-102 are fixed, I'm not too sure what T-125 is, T-145 is fixed with issues listed below, T-144 is fixed but the clipboard prop doesn't auto face you so you can read it, T-139 is fixed with issues listed below, T-149 is fixed with issues listed below, T-112 is fixed with issues below, T-123 is fixed, T-160 T-161 T-162 are fixed with issues below, T-117 is fixed-ish issues listed below, T-164 isn't fixed, T-020 is fine and T-115 is also fine, T-153 is fixed with issues listed below, T-157 is fixed, T-165 is fixed, T-148 is not fixed, T-140 is not fixed, T-151 is not fixed, T-155 is fixed, T-150 T-152 are not fixed, T-143 T-166 are fixed, T-146 T-154 are not fixed.

Playtest notes: shadows look better but still blocky; that window does look different in the main game (comparison screenshots to come); cubes turn black when they're activated and yellow and it adds weird spikes under the button almost like there's grass under it; portals still look like they're above glass when they're behind it; sometimes voicelines activate late, also when going through the elevator, making the elevator ride go on longer; the portal gun still doesn't have that electricity when picking things up and doesn't spark/shake when going through the emancipation grids; buttons are still juttery/buggy when [cubes are] put on buttons; trying to move a cube up on a wall stops me from moving forward, it's buggy, and carrying them around in general isn't too smooth; frequent fps drops especially looking at portals; GLaDOS chamber floor is still a weird murky brown (better than before); something about what fixed dropping cubes when pushing them into the wall made carrying a bit buggy and pushes me back when close to walls; GLaDOS awakening animation is still covered by lots of white particles; things still falling through elevators; the videos that play in the elevator rooms are a bit buggy and super low quality; there's a point where GLaDOS never moves the walls so I can't move past (screenshot 2026-10-20 2.45.36 PM, sp_a2_laser_over_goo); after noclipping through and continuing to the chamber ahead, clicking the button to drop the box crashed the game.

Other notes: most of these are global and not all of them; lighting/textures being low quality is still an issue; some music doesn't play along with ambient sounds; the big room outside the moving box (intro1) has issues in the wake-up sequence, parts not foggy and the sky just black above; the UI not being styled like Portal 2 is big, get it as close as possible.

## Owner note 2026-10-20 (after playtest 9)
- Can't catch falling things or grab things mid-air: the faith plate level's cube couldn't be grabbed (couldn't continue); Wheatley couldn't be caught when he fell earlier (T-176).

## Owner menu/intro spec (2026-10-21)
Full text in docs/MENU_SPEC.md (startup intros, main menu, Single Player,
New/Load Game, Options/Audio/Video/Advanced Video/Controller/Keyboard-Mouse,
Edit Keys, sliders, quit dialog, navigation, persistence, scaling). Tracked
as T-177..T-192; the owner wants these fixed before any other task.

## Owner notes 2026-10-21 (build fb0090a), menus
Tracked in TASKS.md under the menu spec heading (T-177, T-178, T-179,
T-185, T-187, T-189, T-192, T-193, T-194): flips like the linked video and
instant; scroll overshoot; pixely/missing arrows; Options list trimmed
(Mods greyed, subtitles into Audio, no Speedrun); spacing per aspect;
Robot Enrichment lower and aligned; voice rows greyed; no Graphics row;
Edit Keys hover colour; no cursor during intro; list dead controls.

## Owner notes 2026-10-21 round 4 (build d36e250), menus
Black screen after every flip (regression from the round-3 fix), pause
menu inaccurate (shots in ~/Documents/Screenshots/pausemenu; blur/bloom +
desaturated game behind), tile grid spacing/offsets uneven. Tracked as
T-196, T-195, T-197.

## Owner round 5 (2026-10-21, build b8a2d76), the owner's words
Tracked in TASKS.md (confirmed fixes moved to Done; T-173, T-174, T-179 reopened).

merge t-140, merge t-117, keep t-175 open, keep t-112 open, close t-151, just replayed that intro ride and still saw the spikes idk if it was merged on the app tho so idk for t-164. 196 fixed, 195 fixed, 197 fixed, 179 semi fixed needs work, 182 fixed, 186 and 192 fixed. 176 fixed, im not sure what the laser over goo walls is so idk for 169, 170 fixed, 163 fixed, 174 kind of fixed the quality looks a little better but the panels are like awkwardly not together and some are in the wrong places, the gun hold beam is not fixed it looks terrible, the fizzler shakes are fixed. the first portal stutter idk what that is explain it, i havent tested glados, splash and dust is fixed for now, soundscapes are fixed for now. 113 fixed, clipboard and clock fixed, turrets falling through elevators fixed. 172 is fixed, 108/107 are fixed and ill send screenshots for whats left.

## Owner round 6 (2026-10-21, build ef0144c), the owner's words
Tracked in TASKS.md (T-169, T-171, T-148, T-135, T-139, T-162 to Done; menus on hold).

169 is fixed, 171 is fixed, 148 is fixed, 135 is fixed, 139 is fixed, all menu items are fixed for now buttons just dont function but i dont want those being worked on rn. merge everything else you need me to test rn into my .app build and tell me what im looking for

## Owner round 7 (2026-10-21, build 6e0efbf), the owner's words
Tracked in TASKS.md (T-175/T-140/T-117 to Done; T-164 reverted; screenshot tasks on hold).

fog looks better, torn debries whatever fix you implemented revert it it made moving things start shaking in the ride, portal shake is good, shadows are good, glados wakeup still has some particles around it but its better, portal gun visibly drops, her arms are connected, and the rest is good there. ill send screenshots another time so put those tasks on hold

## Owner note 2026-10-21 (round 7 follow-up), the owner's words
dont work on anything thats already been fixed though ... 164 is the torn spiky shit skip that for now i said, fix 153 for sure, ride jitter in 120s been fixed, 152 has to wait for screenshots, 150 for sure fix, 151 has to wait for screenshots, 154 has to wait for screenshots.

## Owner note 2026-10-21 (campaign), the owner's words
i just saw the door fixed, and continue to get the rest of the game runnning rather than small fixes after these 2, get chapters 2-9 fully playable. chapter 2 is mostly there but the fling things ariel plates i think their called have something wrong with the physics idk what it is, aiming for a full campaign, have the auto sessions do it so your no longer working in this one

## Owner notes 2026-10-21 (before leaving 24-48 h), the owner's words
look at this repo https://github.com/Stink-O/Portal2-rust-engine in the crates theres different sections ... theres a readme with known gaps in some of them, this is the engine we built off of so check these known gaps and triple check if we have a fix for it if not add it to the tasks list ... also all maps should load maybe not fully correctly but they should be able to be loaded, get textures and all animations going correctly and then focus on the biggest most common things between each level, what bugs appear most, each gel, and so on.
when you booted up chapter 6 the fall a loud glass noise played btw (T-212)
also from what im noticing the glados as a potato on the portal gun doesnt show in chapters after you have her (T-213)

## Owner playtest 10 (2026-10-22, build 0634859), chapter 1 replay + chapter 2, the owner's words
Tracked in TASKS.md (T-216..T-219 new; T-145 back from hold; confirmations in Done).

intro1: wheatlys door doesnt open early, there is fog outside, the skys still black more like a red black tint but thats a later issue, still spiky debtis but also a later issue, both those issues dont stop playability, the 12:00 only digits blink correctly, the window by the bed stilk doesnt render like the actual game but also a later issue that doesnt affect playability.

cubes stilm show as black and are jittery/ buggy moving on buttons, still playable but a necessary now fix this is global not just intro2 i see it across every level but it doesnt show on all buttons idk.

intro2: no black screen, noted cubes on buttons above, and portals behind glass is fixed perfectly.

intro3: floor and roof are there so fixed, there is smoke from where i fall and there is a splash but no walking or jumping in water sound effects and water still doesnt look good.

intro4: no loud glass sound on arrival, and the flat blue is there yes but it looks more like just the fog and its there on every level not just this one and its outside of the room so it may just be that you can see through that wall yk.

intro5: the late voicelines was a global issue thats fixed now, screens and lights are fine this was also global but fixed now, and that floating vine/moss texture is still there.

intro6: verified

intro7:turning around still takes a couple extra seconds, and wheatly does fall after the panel moves so thats fixed.

ap_a1_wakeup: wheatly does get carried correctly, the floor is still murky brown but thats beachse there is supposed to be water it just looks like the water from the previous level and it looks better than before the portal gun does visibly drop and there are still some random particles.

sp_a2_intro: portal gun effects no longer show through the wall so fixed.

also verified playable: sp_a2_laser_intro,

sp_a2_laser_stairs is playable but theres a bug with missing floor after the animation of glados moving the floor happens at setpod 121.78 -354.91 -90.00 if you look down a few seconds after the animation, and after tping to that room where you get the laser box and wheatly shows for a second he continues to show through the wall after the wall closing animation plays setpos -529.75 327.04 -90.00;setang -5.15 87.45 0.00, and another floating texture to the right of the wheatly bug but fully playable.

sp_a2_dual_lasers verified playable, aswell as the level after that i forgot to write it down lol.

sp_a2_catapult_intro is where playability stops, the catapult physics or something is incorrect here, im not sure if its the cube or person that moves incorrectly but im pretty sure the cubes physics are wrong, i cant push it midair it goes below me rather than at my chest or wherever usually so i cant grab it and move forward.

dying and saving works as expected, i didnt see any softlocks. every elevator worked as expected, performance is pretty poor for my m4 pro macbook pro 24gb ram sitting at 60-70fps and its stuttery and im not sure if vsync works correctly, portals lower performance, but it is definitely playable.
