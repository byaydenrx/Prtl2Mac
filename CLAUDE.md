# Prtl2Mac: start here

Native Apple Silicon remake of Portal 2's engine (Rust, Bevy 0.19, avian3d
0.7) that plays the owner's own Portal 2 install. v1 (PROJECT_STATE.md): New
Game through sp_a4_finale4, every map solvable, no softlocks.

Follow `WORKFLOW.md`. At the start of a session run `sh scripts/resume.sh`
once (it prints the handoff, the priority order and the next task's full
entry), then do the next task it prints. TASKS.md's "Priority order" is the
one source of what's next; a prompt from the owner overrides it, and if the
handoff disagrees, fix the handoff. Keep `TASKS.md`, `PROJECT_STATE.md`
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
- Never open leaked Valve source. Never commit decompiled Valve code
  (studying the binaries is fine: see Owner decisions).
- No menu code changes while the owner's hold stands.
- Cargo through `sh scripts/quiet.sh ...`; game logs through a targeted
  `RUST_LOG` and grep. Fold doc updates into the task's own commit.

## Owner decisions (2026-10-21)
- Studying the owner's own Portal 2 binaries (server.dll, client.dll; Ghidra)
  to understand behaviour is allowed: it's a personal project. Write our own
  code from what you learn; never commit decompiled code or paste it into
  docs. Leaked Valve source stays off limits.
- A map is VERIFIED only when the owner has played it through. A saved
  autopilot route (scripts/routes/<map>.txt) is how a session proves a map
  is solvable (TASKS.md "Prove a map"); the map then stays IN PROGRESS and
  goes to the owner to play.

# Compact instructions
Keep the task id, files changed, test results, commit ids and what is left.
