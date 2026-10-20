# Autonomous run log

What the unattended sessions did while the owner was away, newest last. One
line per run: when, task, outcome, commit, app build. Everything here was
checked by autopilot only; the owner's replay is the real verification
(see TASKS.md "Awaiting owner replay").

Outcomes: FIXED (pushed, app rebuilt), PARTIAL (some of it fixed),
ATTEMPT (not reproduced; a best-guess fix sits on the local, unpushed
branch `attempt/T-<n>` for the owner's next session to review),
NOT REPRODUCED / STUCK (findings written into the task, moved on),
UNFINISHED (stopped mid-task; the next run continues it).

| When (local) | Task | Outcome | Commit | App build |
|---|---|---|---|---|
| 2026-10-20 05:00 | T-129 stray elevator screens (laser_stairs staircase) | FIXED: movie panels were drawn mirrored, covering the ring's exit | da914bf | 0.1.0+da914bf |
| 2026-10-20 05:10 | T-130 portal shots through handrails / cubes | PARTIAL: handrails fixed (grate models); cube case not reproduced | 0454ed8 | 0.1.0+0454ed8.20261020T0914Z |
| 2026-10-20 ~05:30 | T-131 intro3 missing floors/roofs | NOT REPRODUCED: blocked on owner screenshots + `getpos` (see TASKS.md) | 1a5f58d | unchanged |
| 2026-10-20 05:40 | T-105 held objects dropped at walls | PARTIAL: wall drop fixed (hold point pulled in front of walls); Wheatley re-pickup and camera overlap split to T-167 | fd9c086 | 0.1.0+fd9c086.20261020T0938Z |
| 2026-10-20 05:47 | T-167 Wheatley re-pickup; held prop at a wall around the camera | PARTIAL: re-pickup fixed (cores pickable until DisablePickup; wakeup never sends EnablePickup); camera overlap still open | 636fc3f | 0.1.0+636fc3f.20261020T0945Z |
| 2026-10-20 05:57 | T-167 held prop at a wall surrounds the camera | FIXED: player now stopped by the held prop at walls (ray along view, prop depth + 17 u); intro2 before/after autopilot shots | 342b5b1 | 0.1.0+342b5b1.20261020T0956Z |
| 2026-10-20 06:09 | T-132 Fizzler field not drawn | FIXED: visible trigger_portal_cleansers now draw (were skipped as triggers); SolidEnergy approximated as tinted green-mask additive; disable hides it; intro2 before/after autopilot shots | 99392c0 | 0.1.0+99392c0.20261020T1008Z |
| 2026-10-20 06:19 | T-114 intro1 Wheatley's door and the announcer | FIXED: ambient_generic Volume input was ignored, so the courtesy announcer stayed at full volume over Wheatley (map ducks it to 3, then 0 on door open); door-on-approach and some overlap are original behaviour; before/after autopilot logs | 463757d | 0.1.0+463757d.20261020T1017Z |
| 2026-10-20 10:35 | T-135 Turrets fall through the exit elevator | NOT REPRODUCED: vacturret elevator_0N root-bone track drops them straight down the shaft through the waiting car (model data; engine handles SetDefaultAnimation and AE 1100); can't confirm vs Windows reference, autopilot shots inconclusive; BLOCKED, no attempt branch | abc5618 | unchanged |
| 2026-10-20 06:40 | T-120 Elevators (ride jitter) | PARTIAL: jitter FIXED, movers were drawn at tick positions under the interpolated view (12.5 cm eye-vs-car swing per tick on intro3 arrival, now ~4 mm); VO timing/thumps/trash still open | c1079f9 | 0.1.0+c1079f9.20261020T1038Z |
| 2026-10-20 06:49 | T-120 Elevators (VO timing, ride length) | NOT REPRODUCED: exit VO fires TransitionReady and the car leaves at the next path_track (intro2 autopilot); intro5's completion VO starts 0.6 s after the cube enters its trigger; long intro5 ride without the VO is the original failsafe logic; no code change, no attempt branch | (this commit) | unchanged |
| 2026-10-20 07:02 | T-120 Elevators (thumps, car sounds) | PARTIAL: thumps NOT REPRODUCED (all ride sounds logged on intro5/intro6, none thump-like); FIXED silent cars, func_tracktrain movesound/stopsound now play (autopilot log before/after) | 5bfaf52 | 0.1.0+5bfaf52.20261020T1100Z |
| 2026-10-20 07:12 | T-133 intro7 Wheatley floats / panel vanishes | FIXED (float): DisableMotion kept the carry's zero gravity, so the ejected Wheatley hung at the socket (autopilot z 1265 before, 1249 on the floor after); panel vanishing NOT REPRODUCED (ghost brush enabled and drawn) | 2542009 | 0.1.0+2542009.20261020T1109Z |
| 2026-10-20 07:20 | T-134 sp_a1_wakeup pod: player doesn't turn | FIXED: func_tracktrain orientationtype 2 never rotated; trains now blend path_track yaws and turn riders + view (autopilot yaw 0 before, 0 -> -90 after at the lift's start) | 5cfe749 | 0.1.0+5cfe749.20261020T1119Z |
| 2026-10-20 07:32 | T-163 sp_a1_wakeup pod: jump out of the hatch | FIXED: startdisabled brushes were never built, so the hatch's player-clip blocker never became solid; now built hidden + collider-disabled (autopilot: escaped to (9133 1017 428) before, stays on lift after); brick block not seen | 8f8c8b1 | 0.1.0+8f8c8b1.20261020T1131Z |
| 2026-10-20 07:38 | T-136 sp_a1_wakeup tube portal before the drop | FIXED: linked_portal_doors were drawn as gun portals (oval + blue particle rim + hum); now SeamlessPortal, view only (autopilot: blue ring at incinerator_entry_portal_1 before, continuous tube after) | 3f96088 | 0.1.0+3f96088.20261020T1137Z |
| 2026-10-20 07:45 | T-120 Elevators: trash rising with the car | NOT REPRODUCED: trash is intro5 tube_blockage_01 (elevator_rubble.mdl); autopilot shows it fall onto the arriving car's roof as its animation intends; T-120 marked BLOCKED (needs owner), next T-137 | 2f5d345 | unchanged |
