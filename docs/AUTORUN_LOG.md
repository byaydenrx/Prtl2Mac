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
