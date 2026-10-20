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
