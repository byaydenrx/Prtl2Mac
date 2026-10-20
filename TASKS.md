# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (see WORKFLOW.md). Work top to bottom: the first TODO/IN_PROGRESS task
whose dependencies are DONE. When a task finishes, move it to Done (newest
first) with its evidence and update PROJECT_STATE.md.

"Prove a map" (the standard acceptance for T-002..T-015): from the map's
start, an autopilot script plays the intended solution using movement,
`fire`, `use` and props (no `setpos` past a puzzle step, no `input` on
puzzle entities); the exit door opens from the puzzle's own outputs; the
next map loads. Record the script in `scripts/routes/<map>.txt` and the run
date. Any engine bug found becomes its own task (P1 if it blocks the route).

## Active

## Owner playtest blockers (P1, play order)

### T-102 Portal traversal: cubes and player through portals  (P1, NEEDS_VERIFICATION, owner replay)
- Owner (intro4, intro2 after the 2026-10-19 build): cubes don't go through
  portals; after a cube got through once, a portal stopped working.
- Causes found 2026-10-19 (all reproduced with autopilot first):
  1. A cube in a map portal's hole when the map re-activates it (intro2 does
     on approach) was listed as the portal's wall; the cube moving then
     fizzled the portal ("portal on a moving surface"). Walls are now static
     colliders only (portals.rs `WallBodies`).
  2. Carrying a cube into a portal: the hold point jumped to the far side at
     once, dragging the cube into the wall, and the player's teleport mapped
     a cube already through a second time; it fell out of the world. Now the
     cube is pulled to the hold point's copy on its own side and only carried
     with the player if still at the entry (props.rs).
  3. Floor portals: only the brush the portal sits on was cut out, so a
     second brush layer 6 u below held the player and cubes (intro4). Static
     world in the 80 u tunnel behind a new portal is now cut out too
     (portals.rs `clear_portal_tunnels`).
  4. Exit-side mirroring copied dynamic props (collider child of the body)
     and triggers as static walls behind the entry (p2_transit).
- Evidence (autopilot, no setpos across portals): intro2 carry cube through
  red->blue, turn, carry back blue->red, cube stays in front; intro4 player
  falls through a gun floor portal out of the ceiling portal; two loose cubes
  fall through it; walking into a floor-level wall portal keeps the floor.
  Unit test held_prop_maps_with_the_player_only_once. Workspace tests pass.
- Not yet: the owner's play of intro2-intro4 by hand; thrown cubes.

### T-103 sp_a1_wakeup: breaker lift doesn't carry the player  (P1, TODO)
- Owner: when the pod moves up switching the breakers, you don't move with it. Unplayable.
- Verify: stand on the lift and ride it to the top without setpos.

### T-104 sp_a2_intro: player lands with nowhere to go  (P1, TODO; replaces T-001)
- Owner: the fall is normal but drops you somewhere with nowhere to go; screen blue and glitchy for ~5 s at the start.
- Verify: from map start, the fall, the gun pickup, then leave the pit using only portals and walking, to the exit.

### T-105 Wheatley can't be picked up again once dropped/stuck  (P1, TODO)
- Owner: if he gets stuck and you drop him, you can't pick him up; the original allows drop and pick up.
- Verify: drop and pick up in sp_a1_intro7 and sp_a1_wakeup.

### T-106 sp_a1_wakeup: crouch needed to enter the breaker room  (P1, TODO)
- Owner: you have to crouch to get in (then Wheatley drops and auto-plugs since he's close).

## Owner playtest issues (P2)
- T-110 Voice lines overlap: jumping/saying "apple" while Wheatley talks, opening the door early; Portal 2 queues or interrupts by scene priority.
- T-111 Music stops at the stairs (wakeup, way to GLaDOS).
- T-112 Missing textures: hallway to GLaDOS, after the jumps (plus a black wall you can walk through), GLaDOS chamber floor drawn as a warm sky, a blue flat wall behind the intro4 elevator.
- T-113 Instructor hints vanish after a few seconds if not done; the key icons have no label ("E", "SPACE").
- T-114 intro1: the Wheatley door opens on approach without pressing E.
- T-115 intro1: wake-up animation still wrong; no wake/sleep animation on the bed.
- T-116 intro1: the first room's window is a grey block (should be blinds etc.); the TV screen sits forward and up from the TV; glitchy screen at the start of the ride; odd outside lighting on the ride.
- T-117 Shadows are blocky (shadow-map resolution and filtering).
- T-118 intro4: loud glass-breaking sound (check against the original); the door animation cuts to black.

## Backlog

### T-002 Prove sp_a2_dual_lasers  (P1, TODO)
- Outcome: both laser catchers powered by the player's redirection, door opens, next map.
- Files: likely lasers.rs, portals.rs, autopilot.rs; scripts/routes/.
- Depends on: none. Verify: autopilot route run + log of catcher outputs.

### T-003 Prove sp_a2_laser_over_goo  (P1, TODO)
### T-004 Prove sp_a2_catapult_intro  (P1, TODO)
### T-005 Prove sp_a2_trust_fling  (P1, TODO)
### T-006 Prove sp_a2_pit_flings  (P1, TODO)
### T-007 Prove sp_a2_fizzler_intro  (P1, TODO)
- Each: "Prove a map" acceptance above. Faith plates already land on target
  (scripts/check_faith_plates.py), so T-004..T-006 test the player's route.

### T-008 Prove chapter 3 rest: bridge_the_gap, turret_intro, laser_relays, turret_blocker, laser_vs_turret, pull_the_rug  (P1, TODO)
- Split one task per map when started.

### T-010 Sweep chapter 4 (column_blocker, laser_chaining, triple_laser, bts1, bts2)  (P1, TODO)
### T-011 Sweep chapter 5 (bts3, bts4, bts5, bts6, core)  (P1, TODO)
### T-012 Sweep chapter 6 rest (jump_intro gel climb, bomb_flings, crazy_box, transition01)  (P1, TODO)
### T-013 Sweep chapter 7 (speed_ramp, speed_flings, portal_intro, end)  (P1, TODO)
### T-014 Sweep chapter 8 (sp_a4_intro .. jump_polarity)  (P1, TODO)
### T-015 Sweep chapter 9 (finale1..finale4)  (P1, TODO)
- Sweep = per map: load, logic runs, exit reachable, next map loads (fills
  L/O/N columns in PROJECT_STATE.md). Proving each puzzle follows as
  per-map tasks.

### T-090 Restart check per verified map  (P2, TODO)
- Outcome: on each VERIFIED map, dying and loading the autosave/quick save leaves the puzzle solvable.
- Verify: autopilot `click`/console load after progress, then the route again.

### T-020 sp_a1_intro1 ride: glitch frames and wall destruction  (P2, TODO)
- Outcome: no frames outside the container; walls break away (actor_wall_destruction_*).
- Verify: frame dump during the ride (autopilot `shot` every 0.2 s) and owner check.

### T-021 Owner manual checks  (P2, NEEDS_VERIFICATION, owner)
- sp_a1_intro1 ride, sp_a1_wakeup socket, sp_a2_intro portal route.

### T-030 Owner reports: missing buttons in gel rooms, black elevator in gel chapters, cube carry feel  (P2, TODO)
- Reproduce in sp_a3_* maps during T-012/T-013 first.

### T-040 App icon and second-Mac first-run check  (P3, TODO)
### T-050 Verify merged cloud features in game: anim round 2 (Wheatley look-at), gels checklist, projtex, saves  (P3, TODO)
- Their checklists are in docs/STATUS.md.

## Done
- 2026-10-19 T-101 sp_a1_intro1 ride end: the wall breaks, the player walks on, the elevator to sp_a1_intro2 works (owner playtest of build 0.1.0+921edff). Cause: model animation events and math_counter values (921edff).
- 2026-10-19 make_app.sh keeps dist/Prtl2Mac-previous.app and stamps a build id (BUILD.txt, CFBundleVersion).
- 2026-10-19 Workflow docs (WORKFLOW.md, TASKS.md, PROJECT_STATE.md, SESSION_HANDOFF.md, CLAUDE.md, scripts/resume.sh).
- 2026-10-19 sp_a2_intro logic: gun pickup, both portals, exit door, next map (autopilot) - 01c83ec.
- 2026-10-19 sp_a1_wakeup socket: CORE_TRIGGER_REACH lets the held core plug in (autopilot).
- 2026-10-19 Packaging: dist/Prtl2Mac.app finds the install with no env vars.
- 2026-10-18 Chapter 1 intro2..intro7 verified (autopilot).
