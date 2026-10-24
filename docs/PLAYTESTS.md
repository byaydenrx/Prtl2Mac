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

## Owner playtest 11 (2026-10-23, build c0cd7f1), chapter 2 end + chapter 3 start, the owner's words
Tracked in TASKS.md (T-220..T-228 new; statuses updated per his notes).

sp_a2_catapult_intro is now verified playable and completeable by me. sp_a2_trust_fling is also verified and playable by me although i had a small issue with a cube not hitting the first faith plate the first time and dropping a cube on a button from high is hit or miss but not impossible. sp_a2_pit_flings is verified and playable by me. sp_a2_fizzler_intro is also verified and playable by me. if i didnt mention any bugs here then theres none that i noticed. starting chapter 3 on sp_a2_sphere_peek which the only bugs i noticed was one of the platforms im meant to stand on sometimes just makes me fall through not always though its like not rendering the wall part itself just the mechanical, also the faith plate that glados messes with should auto launch me if im just standing on it but it only does if i walk off and back on after shes done, and 2 of the moving platforms moves too much it ends up going horizontal rather than angled, but this level is still verified and completeable by me. sp_a2_ricochet some bugs i noticed, the cube thats sitting on a rotating platform the actual platform not the machinery part was invisible so the cube fell through to the floor rather than sittinf on it the cubes still grabable tho, this level is verified and completeable. now sp_a2_bridge_intro has lots of invisible textures and walls just from the beginning room, the animations machinery is missing walls are missing, id mark this unplayable past this point. but you can crouch past the hallway animation and continue further. bridges work surprisingly their super ugly but they do show through portals, that being said even with all the visual glitches i was able to complete and go through the elevator and pass the level because i know the game but i would say unplayable level here because of the visual glitches. now sp_a2_bridge_the_gap also has some missing textures definitely less, wheatly is also a little buggy glitching around and shit and the bird that comes through and you can see, i couldnt pick up one of the cubes that landed on a bridge about half way into beating this level so i spawned another and it crashed my game, if i had to guess tho this level is playable even with the minor visual glitches and crash. 

cubes show normal on buttons now the jitter is still an issue though. zooming in and out doesnt work. fps is smoother. cubes dont rotate on an axis when moving in air like in portal 2. also the holding something animation looks nothing like portal 2, like the electricity looks really bad. cubes dont have the “fizzle” / disintegrating animation when spawning a new one ir when glados does it herself in sp_a2_pit_flings aswell as when going through an emancipation grill. the podium buttons i think their called dont have a sound or animation, the ones that stand up and i have to click to interact with. the chapter hud icons dont look like portal 2s.

also just going through tasks.md in the git, t-212 is not fixed, 213 is fixed, 198 is fixed, the "common to many maps section" hasnt been touched neither has 211. 217 wasnt able to finish but i had it push what it had done so far because my usage was almost out. 219 hasnt been touched, i dont think engine gaps have been touched those are definently big. 218 improved definently, 216 is fixed, 130 is fixed, 215 is done (im pretty sure 215 is just being able to use my crossover install of portal2 if that is what that means then yes it does work ive seen them use it), 198 is fixed, 213 is fixed although i dont think any lighting shows on her or the portal gun noted in engine gaps im pretty sure, 150 is partly fixed, fall smoke and water splash is there, the guns beam is stupid ugly, the motion blur/speed lines are there, some particles dont render and some are just randomly there, the guns glow throuhg a wall is fixed tho. i just realized some of these have "not doe" parts im just reading the title and going off that btw, like for potatos 213 it does say her eye talking animation isnt done which is true. glados wakeup is not fixed definently still an issue although better than it was before, the missing textures noted here are fixed but theres more going throuhg more maps, and then the backlog obviously isnt done.

## Owner playtest 12 (2026-10-23 evening, build c38ce98), chapter 3, the owner's words
Tracked in TASKS.md (T-229..T-232 new; T-145, T-220, T-222 done).

the issues on sp_a2_sphere_ peek are fixed, cube jittering is fixed, visuals on sp_a2_bridge_intro are fixed, wheatly and the bird are still pretty buggy on sp_a2_bridge_the_gap, everything up to and including sp_a2_bridge_the_gap is now playable and im testing further now, turrets definitely do not work as intended theres no shooting animations or anything hitting them or dropping them doesnt trigger anything their just not done yet but they do damage as if their shooting, even with that the level is completeable but definitely prioritize getting turrets going correctly. then sp_a2_laser_relays is where playability ends the laser goes through multiile of the glass things but after it goes through a portal it doesnt so i cant trigger all 3 to finish the level, lasers function a bit weird globally like being angled if they look up or down when they should just go straight they just function strange. gonna noclip past this level to see forward. again sp_a2_turret_blocker is a turret heavy level making it pretty much jnplayable cus turrets just dont function correctly but if you do manage to get the button pressed with the cube it functions and completes normally i just had to use godmode lol. same issues with sp_a2_laser_vs_turret i noclipped past this level. that next level i didnt notice any bugs but i didnt play it all the way through.

## Owner playtest 12b (2026-10-23 evening, build 1894bb5, before the T-231 fix), the owner's words
Tracked in TASKS.md (T-233..T-235 new; T-231 updated).

sp_a2_pull_the rug is verified and playable. in dp_a2_column_blocker the light doesnt switch back on in the supries room, also the laser for the turret doesnt go through the grate in that level and the turret cant see you through or shoot you through the grate, also the laser doesnt kill the turret through the grate, and when the turret does get killed by the laser it doesnt blow up, this level is nit completeable because of the lasers being weird and angling, it angles itself above the next reflection box making it unplayable past this point.

## Owner playtest 12c (2026-10-23 evening, build a40afeb), the owner's words
Tracked in TASKS.md (T-236, T-237 new).

okay sp_a2_column_blocker that first chamber is completeable but when i get to the elevator theres an invisivle barrier stopping me from getting in and moving forward, noclipping past it into the elevator worked, wheatlys animation here while im in the elevator is pretty buggy like he doesnt look at you and hes moving through a rail just buggy.

## Owner playtest 13 (2026-10-23 night, build 024d846), chapters 3-4, the owner's words
Tracked in TASKS.md (T-238..T-240 new; T-230, T-231 done; T-229 partly).

sp_a2_laser_chaining is verified and playable. sp_a2_triple_laser is also verified and playable there was a weird bug behind one of the portals with an invisible barrier but i got out without noclip and still completed the level like mormal. at sp_a2_bts1 the wheatly busting you out cutscene or whatever doesnt register or start so not playable past this point. going back through chapter 3 levels now to test the ones i didnt verify before, sp_a2_laser_relays is now verified and playable. sp_a2_turret_blocker is verified and playable although turret functions still arent finished. sp_a2_laser_vs_turret is still not playable because the lasers dont shoot through glass like they should.

(Later in the session: "turrets shouldnt be able to shoot through glass btw".)

## Owner playtest 13b (2026-10-23 night, build 9d43b7a), the owner's words
Tracked in TASKS.md (T-234, T-239 updated; T-233, T-236 done).

because the turrets dont explode when they die i cant shoot past them so the line of 3 in laser_vs_turret is still not playable. in bts1 the cutscene now triggers but it teleports me rather than just turning the lights off and turning the bridge off like normal i tp to the floor, i also ended up getting attached to wheatlys rail and it dropped me in the void and it does this everytime i try to replay so unplayable past this point, wheatlys also pretty buggy during the sequence. sp_a2_column_blocker is fixed tho and verified and playable.

## Owner playtest 13c (2026-10-24, build 4e0f7f0), the owner's words
Tracked in TASKS.md T-239.

im still getting picked up by wheatlys rail

## Owner playtest 14 (2026-10-24, build cde979f), the owner's words
Tracked in TASKS.md (T-239 done; T-241..T-243 new; T-203 now P1).

the cube disappears once the lights turn off but it should continue to fall, other than that its perfect bts1 is verified and playable. bts3 i had some issues with turrets and wheatly jittering and teleporting on the rail, wheatly jittering seems to be a bit of a global problem but its not on every map other than that bts2 is confirmed and playable mark it verified. bts3s opening room is pitch black and im stuck and cant get outve it so unplayable past this point.

(Follow-up: "the doors supposed to be stuck and i portal out but theres no lights so i cant see anything its pitch black". Then: "just log everything in the github now and dont continue working on it".)

## Owner notes 15 (2026-10-24): playing the original Portal 2, compared from memory, the owner's words
Not a playtest of our build: the owner played Windows Portal 2 and wrote
down what differs from ours, numbered by loading screens (0 = sp_a1_intro1).
Tracked in TASKS.md (T-244..T-253 new; notes added to T-150, T-151, T-152,
T-153, T-173, T-203, T-206, T-218, T-219, T-224, T-226, T-228, T-229) and
docs/TASKS_ON_HOLD.md (pause screen, T-195). Owner: "just update the repo
dont begin working".

i noticed these playing the original game. i just played as far as i felt, wrote down as i noticed, wasnt playing with the other alongside but these are what i remember. their numbered by how many loading screens i hit.

0
in chapter 1 the ui saying “chapter 1” doesn’t pop up when you wakeup in our version but it should. the ui popping up sounds for like “speak” dont play. wheatly doesnt fully look at you more to the side of you in the opening scenes in our version which is wrong he should look directly at you. the music volumes either are different or dont play like when the alarms going off and theres the evilish music. the windows still different in the opening room. performance is way better even at max settings 4k via translation layers like crossover. theres no like motion blur/speed lines when wheatlys smashing into containers in the moving portion. the lighting in this part is sooo much different from our version so much better in the og. textures also look worse in our version. the smoke particles dont play after wheatly takes the wall down. buttons should emit some light and look dirty almost you can compare in screenshots you take since you have access to the real portal 2. the blur when pausing should like fade in and out along with the desaturation, also the desaturation when pausing doesnt desaturate enough in our version. in our version we cant move around in elevators but you should be able to.

1
screens should be dirty and like flicker on rather than just snap on in our version. also the fog behind the elevator rooms is too much you cant see anything behind it. the pedestal buttons in our version dont have the click animation or the sound. portals between the 2 bars like in the portals introduction dont have the little lights on the side. cubes dont make drop sounds in our version, and they should also pickup at a set distance from the player and auto rotate. the sound of things falling on the exit elevator is much louder than it should be in our version. 

2
our ambient sounds are either the wrong volume or just non existent. the doors dont have their opening sounds or closing sounds in our version. the portal gun in our version doesnt have the animations and also lighting doesnt effect it. the water looks like shit in our version and doesnt have the sounds of jumping and walking in it. in our version you cant see your portals through walls like the original. the screens in elevator rooms dont have seams so it looks like their just clipped weird in our version. 

3
the shake screen effect when portals open from the game is different and worse on our version than the original. tv screens (not the chamber screens or elevator rooms screens but the screens in backrooms and shit) dont have their scanline animation, the elevator and chamber screens also dont have scanlines. 

4
sounds dont change depending on distance or the way your looking on our version but it should. the game just looks and sounds and feels way better and smoother on the original than our version.

5
no notes that havent been said above.

6
the screen shake effect in our version doesnt play for every portal the game opens. wheatlys dialogue should play all the way through until he says “people with brain damage are the real heros” even if you already get to the other side of the room with him. the portal opening animation in our version is non existent. the orange portal in the room where we catch wheatly should close after we get to the other side. wheatly doesnt back up a bit in our version after saying “thats really high”. in our version you cant shoot the portal gun even when holding props or wheatly but you shouldnt be able to. the turret when “following the rail” laser doesnt flicker on and off in our version.

7
the way you hold props and the way they move when being pushed on walls and with you is much different in our version compared to the og, i noticed in the escape hatch while holding wheatly. also in the escape hatch wheatly in our version auto plugs in but you should have to put him there and after he plugs in the light doesnt switch on and the sound effect doesnt play. the particles and wires in glados wakeup and her whole animation is much different. the see throughness and reflection of water is much different also noticed in glados chamber while she picked us up. the drop hole where she drops us looks like it has a rectangle in the center of ours but its normal in the og.

8
the fall animation fog and particles is much different and worse in ours. theres some cubes in our version that when their falling while navigating the incinerator room have weird white boxes around them. the color of the lights on the portal gun should change to the color of the last portal you shot. 

side note: the loading screens are annoying, remove them completely in our version.

9 (start of chapter 2)
the chapter ui is too low and doesnt play the animation in our version. particles not playing seems to be a global issue. again lasers look much worse in our version and you can also see the seam in them between portals. same thing with ambient sound i think its a global issue because it sounds much better in the og. after going through the emancipation grill the portal guns lights should turn off and your portals should disappear. 

10
grabbing props through portals in our version is finnicky. laser cubes look like shit in our version you cant see the laser in them. 

11
no notes

12
cameras in our version dont follow you. 

13
also noticing shooting the emancipation grills makes it flash a little lighter for a second in the og and not ours. tge cube falling rotates a little in the og and not ours. 

14
flickering lights and their sounds just dont play in our version. the dying animation in our version is also different. 

15
i dont think wheatly showing himself through the wall in this chamber plays in our version.

16
no notes

17 (the start of chapter 3)
the beeping of the faith plate is much louder in our version and doesnt change by how far you get, and the blue color should flash ti orange while beeping syncing with the beeps. 

18
lasers dont emit the fog/loght around them. the music while flying in air doesnt play.

19
the jumping and walking on bridges and their sounds dont play in our version. bridges also just look like shit.

20
glass should have like a textured look to it which is likely why the bird looks weird in the wheatly cutscene. buttons in our version should change color with the cube on it.

21
turrets in our version dont open up just from being picked up and they also dont freakout and start shooting then shutting down after throwing them. turrets in our version also cant see through portals. their bullets in our version dont leave bullet holes on glass and walls. they also dont do the freakout then shutdown when being hit with props like cubes
