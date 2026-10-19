# Tasks

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE. Priorities
P0-P4 (see WORKFLOW.md). Work top to bottom: the first TODO/IN_PROGRESS task
whose dependencies are DONE. When a task finishes, move it to Done (newest
first) with its evidence and update PROJECT_STATE.md.

"Prove a map" (the standard acceptance for T-001..T-015): from the map's
start, an autopilot script plays the intended solution using movement,
`fire`, `use` and props (no `setpos` past a puzzle step, no `input` on
puzzle entities); the exit door opens from the puzzle's own outputs; the
next map loads. Record the script in `scripts/routes/<map>.txt` and the run
date. Any engine bug found becomes its own task (P1 if it blocks the route).

## Active

### T-001 Fly sp_a2_intro's portal route out of the gun pit  (P1, IN_PROGRESS)
- Outcome: after picking up the gun, the player leaves the pit only via portals and reaches the exit.
- Verify: autopilot route (fire/walk only) to the exit trigger.
- Files: scripts/routes/sp_a2_intro.txt; engine files if a bug turns up.
- Depends on: none.

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
- 2026-10-19 Workflow docs (WORKFLOW.md, TASKS.md, PROJECT_STATE.md, SESSION_HANDOFF.md, CLAUDE.md, scripts/resume.sh).
- 2026-10-19 sp_a2_intro logic: gun pickup, both portals, exit door, next map (autopilot) - 01c83ec.
- 2026-10-19 sp_a1_wakeup socket: CORE_TRIGGER_REACH lets the held core plug in (autopilot).
- 2026-10-19 Packaging: dist/Prtl2Mac.app finds the install with no env vars.
- 2026-10-18 Chapter 1 intro2..intro7 verified (autopilot).
