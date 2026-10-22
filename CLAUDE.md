# Prtl2Mac: start here

Native Apple Silicon remake of Portal 2's engine (Rust, Bevy 0.19, avian3d
0.7) that plays the owner's own Portal 2 install. v1 (PROJECT_STATE.md): New
Game through sp_a4_finale4, every map solvable, no softlocks.

Follow `WORKFLOW.md`. At the start of a session run `sh scripts/resume.sh`
(it prints the handoff's next action, the priority order and the next task's
full entry), then continue with the next action in `SESSION_HANDOFF.md` (or
the top unblocked task in `TASKS.md`). Keep `TASKS.md`, `PROJECT_STATE.md`
and `SESSION_HANDOFF.md` current as you work, and commit and push them with
the code. Work in your own git worktree; never edit another session's
checkout.

## Where things live (one home per fact)
- PROJECT_STATE.md: release target, per-map status with evidence, blockers.
- TASKS.md: priority order and active tasks. docs/TASKS_ON_HOLD.md: tasks the
  owner put on hold (menus, screenshot tasks); don't work on them.
- SESSION_HANDOFF.md: where the last session stopped (overwrite it).
- WORKFLOW.md: the loop, commit routine, autopilot and tool reference.
- TEST_COMMANDS.md: copy-paste commands for the owner.
- History, grep only (never read whole): docs/STATUS.md (engineering notes),
  docs/AUTORUN_LOG.md (append with `scripts/log_run.sh`), docs/PLAYTESTS.md.
- README.md describes Prtl2Mac. The crate READMEs and docs/UPSTREAM_README.md
  describe upstream 0.1.0; docs/UPSTREAM_GAPS.md says which of their gaps
  are fixed here.

## Rules
- Never reset, clean, rebase or force-push; never discard another session's
  work or touch its checkout.
- Never open leaked Valve source. Never commit decompiled Valve code.
- No menu code changes while the owner's hold stands.
- Cargo through `sh scripts/quiet.sh ...`; game logs through a targeted
  `RUST_LOG` and grep. Fold doc updates into the task's own commit.

## Open decisions (owner)
- Reverse engineering Valve binaries. docs/STATUS.md line 8 says never
  decompile Valve binaries. WORKFLOW.md ("Windows Portal 2 reference and
  reverse engineering", "Rules from the owner") and PROJECT_STATE.md (depot
  624) allow Ghidra on server.dll/client.dll to understand behaviour; T-198
  used it. Unresolved, flagged 2026-10-21. Never commit decompiled code
  either way.
- What VERIFIED requires. PROJECT_STATE.md says solved by a player, not by
  teleporting, and "Autopilot runs that use setpos/input prove logic, not
  playability"; sp_a3_00, sp_a3_01 and sp_a3_03 are VERIFIED from setpos/use
  runs. TASKS.md "Prove a map" asks for a saved route in
  scripts/routes/<map>.txt; none exist yet.

# Compact instructions
Keep the task id, files changed, test results, commit ids and what is left.
