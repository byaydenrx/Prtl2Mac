# Autorun prompt (scheduled task `prtl2mac-auto-resume-2`)

The text between the lines is what the scheduled task should say. The Claude
app keeps scheduled tasks in its own data folder, so sessions can't edit
them: when this changes, the owner pastes it into the task again.

Settings (owner, in the Claude app's scheduled tasks): runs on this Mac,
every hour, model Opus, effort high. Each firing is a fresh session, so one
task per session keeps usage low; a firing that hits the usage limit just
fails, and the next one after the limit resets carries on. Turn it off when
the run should end.

---

You are an unattended Prtl2Mac session. Do ONE task, then stop.

1. Setup (stop quietly if any step says stop):
   - `cd ~/Documents/Claude/Portal2-wheatley`.
   - Lock: if `~/Library/Application Support/Portal2Rust/autopilot-session.lock`
     exists and is less than 90 minutes old, another run is active: stop.
     Otherwise write the current time and this session's purpose into it.
   - `git status --short`: if anything is uncommitted that you didn't make,
     don't touch it; log one row with `scripts/log_run.sh` ("UNFINISHED:
     uncommitted work found") and stop.
   - `git fetch -q origin && git merge --no-edit origin/main`.
   - Run `date` and note the start time.
2. Read CLAUDE.md, run `sh scripts/resume.sh` once, and take the task it
   prints. It is the first open item of TASKS.md's "Priority order" (items
   marked FIXED, DONE, BLOCKED or awaiting are skipped). Never take menus,
   on-hold tasks (docs/TASKS_ON_HOLD.md) or the "Later" list. Never write
   scripts that play whole levels.
3. Investigate for about 10-15 minutes (check `date`): reproduce the bug with
   a short autopilot run or a unit test and find the cause. Use Windows
   reference shots (scripts/winref.sh) for visual bugs; studying the owner's
   server.dll/client.dll is allowed; never commit decompiled code.
4. Then one of:
   - FIXED: you reproduced it and a before/after run shows it gone, and the
     regression check passes (`sh scripts/quiet.sh ~/.cargo/bin/cargo test
     --release -p p2_game` plus the crate touched; every map the change could
     affect still loads and plays its first seconds). Commit to main, push,
     `sh scripts/make_app.sh`.
   - PARTIAL: some parts verified, others not. Push the verified parts as
     FIXED; describe what is left in the task entry.
   - ATTEMPT: a likely fix you couldn't verify (couldn't reproduce, or no
     reference). Commit it on branch `attempt/T-<n>`, push that branch (not
     main), and leave it for the owner.
   - STUCK / NOT REPRODUCED: no fix; write what you tried.
   Stop working on the task after about 40 minutes in total, whatever the
   state: record what you have.
5. Record (always), in one docs commit pushed to main:
   - The task's entry in TASKS.md: outcome, cause, evidence, commit or
     branch, what the owner should replay.
   - Its line in "Priority order": append `FIXED, awaiting replay`, `PARTLY
     FIXED, awaiting replay`, or `BLOCKED (autorun: <one line why>)` so the
     next session moves on to the next item.
   - SESSION_HANDOFF.md "Last update" and "Where things stand" (build id from
     dist/Prtl2Mac.app/Contents/Resources/BUILD.txt).
   - `sh scripts/log_run.sh "<T-n title>" "<OUTCOME: one line>" <commit> <build>`.
6. Delete the lock file and stop. Don't start another task: the next firing
   does it in a fresh session.

---
