# Workflow (how every session works)

Prtl2Mac: a native macOS (Apple Silicon) Rust/Bevy remake of Portal 2 that
plays the owner's own Portal 2 files. Repo `github.com/byaydenrx/Prtl2Mac`,
branch `main`. The owner authorizes committing and pushing to `main`.

## The four documents (git is the source of truth, not chat history)
- `PROJECT_STATE.md`: release target, per-map status with evidence, blockers.
- `TASKS.md`: the backlog. IDs, priority, status, acceptance, evidence.
  Owner-held tasks (menus, screenshot tasks) live in `docs/TASKS_ON_HOLD.md`.
- `SESSION_HANDOFF.md`: exactly where the last session stopped and the next action.
- `WORKFLOW.md`: this file.
`docs/PLAYTESTS.md` keeps the owner's raw playtest reports (history; every
item is a task in TASKS.md). `docs/STATUS.md` is the long engineering log (how systems work, debug notes);
add detail there, keep the four documents short.

## Resume (start of every session)
1. `sh scripts/resume.sh`, once per session (CLAUDE.md already asks for it;
   don't rerun it): git state of every worktree, uncommitted work, the
   handoff's next action, the priority order, task counts, and the full entry
   of the next task (so TASKS.md doesn't need opening to start).
2. Check the handoff against reality: build
   (`sh scripts/quiet.sh ~/.cargo/bin/cargo build --release`) and, if the last
   task touched code, its targeted test. Don't trust a status you haven't seen
   evidence for.
3. Take the next task resume.sh prints: an IN_PROGRESS task, else the first
   open item of TASKS.md's "Priority order". That list is the one source of
   what's next; if the handoff names something else, the list wins and the
   handoff gets fixed. A prompt from the owner overrides both.

## Loop (per task)
1. SELECT: the owner's priority order first (Resume step 3). For anything it
   doesn't list, highest priority first. P0 build/crash/corruption; P1 portals,
   unsolvable puzzles, softlocks, broken progression/transitions; P2 campaign
   scope, restart/checkpoints, controls, assets, packaging; P3 performance and
   macOS usability; P4 polish/refactors/optional features. Never do P3/P4 while a
   known P0/P1 is open. Split anything that can't be verified in one sitting.
2. MARK: set the task IN_PROGRESS in TASKS.md (and commit, no build needed)
   before big work, so another session doesn't take it too.
3. PLAN briefly: objective, files, acceptance, cheapest reliable check.
4. IMPLEMENT: focused change, no unrelated refactors.
5. VERIFY: `cargo test --release -p p2_game` (plus the crate touched) and a real
   game run when behaviour is involved (autopilot below). Read the diff.
   Wrap cargo in `sh scripts/quiet.sh ...` so only errors, failures and
   summaries come back (the full log path is printed).
6. RECORD: task -> DONE with evidence (or NEEDS_VERIFICATION / BLOCKED with the
   reason); update PROJECT_STATE.md and SESSION_HANDOFF.md; commit; push.
   If the task changed code, then run `sh scripts/make_app.sh` so the owner's
   `dist/Prtl2Mac.app` has the fix (the old app is kept as
   Prtl2Mac-previous.app), and put its build id (Contents/Resources/BUILD.txt)
   in the handoff and the run log, with what to replay.
7. CONTINUE with the next task. Don't stop after one task while usage remains.

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE.
Map statuses (PROJECT_STATE.md): NOT STARTED, IN PROGRESS, BLOCKED, VERIFIED.
VERIFIED only when the owner has played the map through (owner, 2026-10-21).
Sessions don't script whole levels (owner, 2026-10-22): the owner plays
them. Autopilot runs are for reproducing one bug, showing the fix, and the
regression check (TASKS.md top).

## Commit and push routine
```
git add <files> && git commit -m "<what and why>"   # end with the Co-Authored-By line
before=$(git rev-parse HEAD)
git fetch -q origin && git merge --no-edit origin/main   # never rebase/force
# Rebuild and retest only if the merge brought in code (your own code was
# already built and tested in VERIFY); docs-only commits skip this.
git diff --quiet "$before" HEAD -- crates Cargo.toml Cargo.lock || {
  sh scripts/quiet.sh ~/.cargo/bin/cargo build --release && sh scripts/quiet.sh ~/.cargo/bin/cargo test --release -p p2_game; }
git push origin HEAD:main
```
Never reset, clean, force-push, or touch another session's checkout. If a
merge conflicts, resolve it keeping both sides' intent, rebuild, retest.
Run one session at a time on this repo (manual or automatic): two sessions
on the same task waste both.

## Working copies
One coding session by default. Each local session works in its own git
worktree (`git worktree add ../Portal2-<name> -b <name> origin/main`), never in
another session's folder. Which checkout is active is recorded in
SESSION_HANDOFF.md "Checkouts" (2026-10-20: `Portal2-wheatley`; the old main
checkout `Portal2-rust-engine` is stuck mid-merge, owner to decide, leave it
alone); `git worktree list` shows them all. At most two extra sessions, only
for independent tasks with separate files.

## Running the game (verification)
- Game files: found automatically (Steam libraries or the depot under
  `~/Library/Application Support/Steam/Steam.AppBundle/.../content/app_620/depot_621`);
  `PORTAL2_DIR=<folder with portal2/>` overrides.
- macOS has no `timeout`: `perl -e 'alarm 60; exec @ARGV' ./target/release/portal2 <map>`.
- Autopilot: `P2_SCRIPT="<t> <cmd>; ..."` runs commands at game seconds:
  `setpos x y z [pitch yaw]`, `look pitch yaw`, `turn`, `walk/back/left/right <s>`,
  `duck`, `jump`, `use`, `fire blue|orange`, `goto|lookat <name> [dist]`,
  `input <target> <input> [param]`, `info <name>` (state, bounds),
  `portalable x y z r`, `ray x y z [dx dy dz]`, `propto <name> x y z`,
  `shot <file.png>`, `log`, `click <Action>`, `hover <Action>`, `key up|down|left|right|enter|tab|escape`, `quit`.
  `RUST_LOG=info,portal2::io=debug` logs every entity output that fires.
- Debug env: P2_NO_INTRO=1 (skip the startup intro movie and loading screen
  of a no-map launch; with the intro the script clock starts at launch),
  P2_LOADING_HOLD=<s> (keep the loading screen up), P2_TRACE, P2_NOCROSSHAIR, P2_HIDE_MAT (hide materials by name
  substring: find which material a bad surface is), P2_LIST_MATS,
  P2_EXPOSURE=1 (fixed exposure), P2_FRAMELOG=1 (fps/min/avg/max each second
  and every frame over 25 ms; keeps an unfocused window from being held to
  60 Hz; with `--size 1920x1080` it measures 3840x2160), SOURCE_VALVE_DEBUG (shader combo per
  material).
- Placing the view: `goto <entity> <dist>` + `lookat <entity>` beats
  guessing `setpos` (setpos z is the feet; outside the map you fall into the
  void). `con map <name>` changes level mid-run. `info <name>` prints state,
  bounds, colliders and face materials.
- Asset tools (`cargo build --release -p source_assets --example <x>`, run
  from target/release/examples with PORTAL2_DIR set): `ents <map> [class|name|--all]`
  (keyvalues and outputs), `ls --cat <path>` (VMTs, scripts), `ls --vtf`,
  `mdlinfo <mdl> [skin]`, `info <map> --near x,y,z r` (world metres: x, z_src, -y_src
  times 0.0254), `probe <map> x,y,z`, `pak <map>`; p2_assets `animinfo <mdl>`
  (sequences; FRAMEANIM/JUMPS/BONES env). BSP lumps can be read directly with
  python (static props: lump 35 `prps`).
- Screenshots (owner's in ~/Documents/Screenshots, autopilot 2560x1440):
  shrink with `sips -Z 1000` before reading.
- Package: `sh scripts/make_app.sh` -> `dist/Prtl2Mac.app`.

## Unattended runs (scheduled task `prtl2mac-auto-resume-2`; the old `prtl2mac-auto-resume` folder is unused)
- One task per fresh session; lock in ~/Library/Application Support/
  Portal2Rust/autopilot-session.lock; one row per run in docs/AUTORUN_LOG.md,
  appended with `sh scripts/log_run.sh <task> <outcome> <commit> <build>`
  (no need to read the log).
- The task's exact instructions are in docs/AUTORUN_PROMPT.md (the owner
  pastes them into the scheduled task; sessions can't edit it).
- Only reproduced, verified fixes reach main. A fix a run could not verify
  goes on branch `attempt/T-<n>`, pushed to origin (not main), noted in the
  task and the handoff. Owner sessions review
  them: `git branch --list 'attempt/*'`, `git log -p wheatley..attempt/T-<n>`;
  merge with `git merge attempt/T-<n>` after a replay, or delete the branch.

## Windows Portal 2 reference and reverse engineering
- Windows Portal 2 is installed in the owner's CrossOver bottle "Steam":
  `~/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/Portal 2`
  (portal2/bin/server.dll, client.dll; same as depot 624). Proven 2026-10-21:
  `/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine --bottle Steam "C:\Program Files (x86)\Steam\steam.exe" -silent -applaunch 620 -windowed -w 1280 -h 720 -novid +map <map> +exec <cfg>`
  starts the game. Reference shots: `sh scripts/winref.sh <map> <out.jpg>
  [--pos X Y Z] [--ang P Y R] [--cmd "..."] [--settle SEC] [--hud]` (T-215).
  It runs the game with -netconport and drives the console over TCP:
  waits for "Redownloading all lightmaps" (load done, ~30 s), hides the
  HUD/crosshair/captions/notify text, setpos/setang, waits --settle (default
  6 s; maps with an arrival elevator need ~25 s), `jpeg`, quit, Steam
  -shutdown, removes its own screenshots. ~1.5 min per shot, 1280x720.
  Our matching shot: `P2_SCRIPT="6 setpos X Y Z P Y; 9 shot f.png; 10 quit"`.
  Note: cfg `wait` is "Unknown command" until a server runs, so cfg-only
  scripts can't wait out the load. Always shut Steam down afterwards:
  `wine --bottle Steam "C:\Program Files (x86)\Steam\steam.exe" -shutdown`.
  Don't change the owner's Steam account or settings.
- Ghidra 12.1.4: `/opt/homebrew/Cellar/ghidra/12.1.4/libexec/support/analyzeHeadless`
  (headless analysis/scripts) for server.dll/client.dll behaviour (entity
  logic, cvar defaults, physics constants). Understand, then write our own
  code; never use leaked source.
  Confirmed by the owner 2026-10-21 (personal project). Never commit
  decompiled code or paste it into docs; write findings in your own words.

## Gotchas (map logic)
- Departure elevators only arrive after the player walks through
  `departure_elevator-source_elevator_door_open_trigger`; some exits are
  walk-in `transition_trigger`s whose volume isn't at their origin
  (`info <name>` prints bounds).
- Proven placement sequences (exits only): catapult_intro
  `3 setpos -64 -1512 -400; 9 setpos -64 -2126 -575`; sphere_peek
  `3 setpos -544 1176 500; 9 setpos -544 562 316`; laser_relays
  `3 setpos -320 -1192 52; 9 setpos -320 -1806 -132`; turret_intro
  `3 setpos -336 392 -250`.
- Faith plates with usethresholdcheck=1 only redirect things arriving fast.

## Cost rules
- Read the handoff and the named files; don't rescan the repository.
- Don't read the long files whole (TASKS.md, docs/TASKS_ON_HOLD.md,
  docs/STATUS.md, docs/AUTORUN_LOG.md, docs/PLAYTESTS.md): grep by task id
  or read the lines you need.
- Cargo output through `sh scripts/quiet.sh`; game logs through a targeted
  `RUST_LOG` and `grep`, never raw.
- Prefer targeted tests and short autopilot runs over full sweeps.
- No subagents/extra sessions unless a task is independent and worth it.
- Model choice and usage limits are set by the owner in the Claude app.

## Rules from the owner
- Never open or use leaked Valve source code. Reverse engineering the owner's
  own install (Ghidra, tools in ~/Documents/Claude/tools) to understand
  behaviour is fine; write our own code. The install's `bin/*.fgd` files
  document every entity (`LC_ALL=C grep -a`).
  (Confirmed 2026-10-21.)
- Never commit decompiled Valve code (owner, 2026-10-21).
- Only touch the project, scratch/temp folders and
  ~/Library/Application Support/Portal2Rust.
- Keep the game as close to the original as possible; new options go inside
  Settings, never as extra main/pause menu buttons.
