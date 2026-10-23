# Autorun (scheduled task `prtl2mac-auto-resume-2`)

The scheduled task holds only the short starter at the bottom of this file;
the rules are here so a repo change updates every later run. Sessions can't
edit the scheduled task itself (the Claude app keeps it in its own data
folder): if the starter changes, the owner pastes it again.

Prtl2Mac (native Rust/Bevy Portal 2 remake for Apple Silicon) lives in
~/Documents/Claude/Portal2-wheatley (git worktree, branch `wheatley`; commit
there and push to `main`). The owner may be away. Each run does ONE short
piece of work, then stops: a new run is attempted every 10 minutes and skips
while another holds the lock. If the usage limit is hit mid-task, the next
run after it resets picks the work up from git state and the handoff.

## Owner direction (2026-10-23)
- Playability first. The owner plays the campaign himself and reports bugs
  (docs/PLAYTESTS.md, filed into TASKS.md). Sessions fix bugs; never write
  scripts that play whole levels.
- Follow TASKS.md "Priority order" top to bottom. Crashes and softlocks you
  find go first (new task, next free T-number).
- Never touch: menus (crates/p2_game/src/ui.rs menus/dialogs/flips/Options,
  options.rs, intro.rs, loading.rs, the menu parts of settings.rs,
  docs/MENU_SPEC.md); anything in docs/TASKS_ON_HOLD.md (owner holds and the
  "later" list); in-game HUD/captions/hints unless the task is about them. If
  a fix would need one of these, record it in the task and move on.
- Studying the owner's own server.dll/client.dll (Ghidra) and Windows Portal 2
  (scripts/winref.sh) to understand behaviour is allowed; write our own code;
  never commit decompiled code; never use leaked source.

## Stop rules (owner: "I don't want to come back to lots of regression or broken stuff")
- If `~/Library/Application Support/Portal2Rust/autopilot-STOP` exists: do
  nothing (the starter checks this).
- Stop for good when ANY is true: (a) no line in the priority order is left
  that isn't FIXED, DONE, BLOCKED or already attempted; (b) the last 5 rows of
  docs/AUTORUN_LOG.md are all STUCK / NOT REPRODUCED / ATTEMPT / UNFINISHED
  (no FIXED or PARTIAL): the runs are thrashing. To stop for good: write the date and
  reason into the STOP file, disable scheduled task prtl2mac-auto-resume-2 if
  a scheduled-tasks tool is available, put the reason at the top of
  SESSION_HANDOFF.md, commit and push the docs, remove the lock.

## The run
1. Setup is in the starter (stop file, lock, branch, sync). If `git status`
   shows uncommitted changes or unpushed commits on `wheatley`, they are a
   previous run's unfinished work: read the diff, finish it (or revert only
   if clearly broken), test, commit, push. That is this run's work.
2. `sh scripts/resume.sh` once. Take the task it prints: an IN_PROGRESS task,
   else the first open line of the priority order. Read only that task's
   entry (resume.sh prints it), WORKFLOW.md "Running the game" for autopilot
   commands and tools, and files the task names. Don't rescan the repo or read
   docs/STATUS.md unless the task needs it. Mark the task IN_PROGRESS in its
   header.
3. Timebox: about 10 minutes of work (check `date` at the start and as you
   go; at most ~15 minutes or ~25 tool calls). A task that needs more spans
   several runs: commit a working, tested step, write in the entry what is
   done and the exact next step, keep it IN_PROGRESS. The next run continues
   it.
4. Outcome:
   - FIXED (reproduced, before/after evidence, regression check for what you
     changed): commit on `wheatley`, `git fetch -q origin && git merge
     --no-edit origin/main`, rebuild and retest only if the merge brought in
     code, `git push origin HEAD:main`, `sh scripts/make_app.sh` (note the
     build id). Header and priority line: "FIXED, awaiting owner replay",
     with cause, evidence and what the owner should replay.
   - PARTIAL: a tested step committed as above; the rest written in the entry.
   - ATTEMPT (not reproduced, or the cause can't be confirmed): don't change
     `wheatley`/main code. Best guess on a local branch only: `git switch -c
     attempt/T-<n>` from `wheatley`, build and tests pass, commit, `git switch
     wheatley` (clean tree). Never push or merge it, don't rebuild dist with
     it. Entry: what you tried, suspected cause, "Untested fix: local branch
     attempt/T-<n> (<commit>), needs owner review/replay"; header and priority
     line BLOCKED (needs owner).
   - STUCK / NOT REPRODUCED: no fix; the entry says what you tried; BLOCKED
     with the reason if there is nothing left to try.
   - UNFINISHED: only when nothing could be committed (ran out of time or
     usage); the entry says where you stopped.
5. Record (always), one docs commit on `wheatley`, pushed to main:
   SESSION_HANDOFF.md (Last update, Where things stand with the dist build id,
   Next action, attempt branches under "Fix attempts awaiting the owner") and
   `sh scripts/log_run.sh "<T-n title>" "<OUTCOME: one line what and why>" <commit> <build or unchanged>`.
6. End on `wheatley` with a clean tree, delete the lock, stop. Don't start
   another task.

Rules: never claim owner verification; say what was only autopilot-checked.
Don't touch other checkouts (Portal2-rust-engine is mid-merge). No subagents.
Avoid regressions: only reproduced, verified fixes reach main.

## Starter (paste this into the scheduled task)

```text
Prtl2Mac autorun. First: if "$HOME/Library/Application Support/Portal2Rust/autopilot-STOP" exists, do nothing else (if a scheduled-tasks tool is available, disable task prtl2mac-auto-resume-2) and stop.
Lock: LOCK="$HOME/Library/Application Support/Portal2Rust/autopilot-session.lock". If it exists and is less than 90 minutes old (find "$LOCK" -mmin -90 prints it), another run is working: stop now without doing anything else. Otherwise write the current date into it; delete it when you finish, also if you stop early.
Then: cd ~/Documents/Claude/Portal2-wheatley, make sure you are on branch wheatley (git switch wheatley), run git fetch -q origin && git merge --no-edit origin/main, then read docs/AUTORUN.md and follow it exactly. It has the stop rules, which task to take, the 10-minute timebox and how to record the run.
```
