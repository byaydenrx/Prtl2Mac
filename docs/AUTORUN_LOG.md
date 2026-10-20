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
