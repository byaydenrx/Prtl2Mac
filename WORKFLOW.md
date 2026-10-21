# Workflow (how every session works)

Prtl2Mac: a native macOS (Apple Silicon) Rust/Bevy remake of Portal 2 that
plays the owner's own Portal 2 files. Repo `github.com/byaydenrx/Prtl2Mac`,
branch `main`. The owner authorizes committing and pushing to `main`.

## The four documents (git is the source of truth, not chat history)
- `PROJECT_STATE.md`: release target, per-map status with evidence, blockers.
- `TASKS.md`: the backlog. IDs, priority, status, acceptance, evidence.
- `SESSION_HANDOFF.md`: exactly where the last session stopped and the next action.
- `WORKFLOW.md`: this file.
`docs/PLAYTESTS.md` keeps the owner's raw playtest reports (history; every
item is a task in TASKS.md). `docs/STATUS.md` is the long engineering log (how systems work, debug notes);
add detail there, keep the four documents short.

## Resume (start of every session)
1. `sh scripts/resume.sh` (or read its output if someone ran it): git state of
   every worktree, uncommitted work, the handoff's next action, the top task.
2. Check the handoff against reality: build (`~/.cargo/bin/cargo build --release`)
   and, if the last task touched code, its targeted test. Don't trust a status
   you haven't seen evidence for.
3. Take the handoff's next action, or the top unblocked task in TASKS.md.

## Loop (per task)
1. SELECT: highest priority first. P0 build/crash/corruption; P1 portals,
   unsolvable puzzles, softlocks, broken progression/transitions; P2 campaign
   scope, restart/checkpoints, controls, assets, packaging; P3 performance and
   macOS usability; P4 polish/refactors/optional features. Never do P3/P4 while a
   known P0/P1 is open. Split anything that can't be verified in one sitting.
2. MARK: set the task IN_PROGRESS in TASKS.md (and commit) before big work.
3. PLAN briefly: objective, files, acceptance, cheapest reliable check.
4. IMPLEMENT: focused change, no unrelated refactors.
5. VERIFY: `cargo test --release -p p2_game` (plus the crate touched) and a real
   game run when behaviour is involved (autopilot below). Read the diff.
6. RECORD: task -> DONE with evidence (or NEEDS_VERIFICATION / BLOCKED with the
   reason); update PROJECT_STATE.md and SESSION_HANDOFF.md; commit; push.
7. CONTINUE with the next task. Don't stop after one task while usage remains.

Statuses: TODO, IN_PROGRESS, BLOCKED, NEEDS_VERIFICATION, DONE.
Map statuses (PROJECT_STATE.md): NOT STARTED, IN PROGRESS, BLOCKED, VERIFIED.
VERIFIED only with evidence (autopilot log lines, screenshots, or the owner's
manual play). Say which.

## Commit and push routine
```
git add <files> && git commit -m "<what and why>"   # end with the Co-Authored-By line
git fetch -q origin && git merge --no-edit origin/main   # never rebase/force
~/.cargo/bin/cargo build --release && ~/.cargo/bin/cargo test --release -p p2_game
git push origin HEAD:main
```
Never reset, clean, force-push, or touch another session's checkout. If a
merge conflicts, resolve it keeping both sides' intent, rebuild, retest.

## Working copies
One coding session by default. Each local session works in its own git
worktree (`git worktree add ../Portal2-<name> -b <name> origin/main`), never in
another session's folder. `../Portal2-rust-engine` is the main checkout; check
`git worktree list` and SESSION_HANDOFF.md for who owns what. At most two extra
sessions, only for independent tasks with separate files.

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
  `shot <file.png>`, `log`, `click <Action>`, `hover <Action>`, `quit`.
  `RUST_LOG=info,portal2::io=debug` logs every entity output that fires.
- Debug env: P2_TRACE, P2_NOCROSSHAIR, P2_HIDE_MAT (hide materials by name
  substring: find which material a bad surface is), P2_LIST_MATS,
  P2_EXPOSURE=1 (fixed exposure), SOURCE_VALVE_DEBUG (shader combo per
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

## Unattended runs (scheduled task `prtl2mac-auto-resume`)
- One task per fresh session; lock in ~/Library/Application Support/
  Portal2Rust/autopilot-session.lock; one row per run in docs/AUTORUN_LOG.md.
- Only reproduced, verified fixes reach main. A bug a run could not
  reproduce gets a best-guess fix on a local, never-pushed branch
  `attempt/T-<n>`, noted in the task and the handoff. Owner sessions review
  them: `git branch --list 'attempt/*'`, `git log -p wheatley..attempt/T-<n>`;
  merge with `git merge attempt/T-<n>` after a replay, or delete the branch.

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
- Prefer targeted tests and short autopilot runs over full sweeps.
- No subagents/extra sessions unless a task is independent and worth it.
- Model choice and usage limits are set by the owner in the Claude app.

## Rules from the owner
- Never open or use leaked Valve source code. Reverse engineering the owner's
  own install (Ghidra, tools in ~/Documents/Claude/tools) to understand
  behaviour is fine; write our own code. The install's `bin/*.fgd` files
  document every entity (`LC_ALL=C grep -a`).
- Only touch the project, scratch/temp folders and
  ~/Library/Application Support/Portal2Rust.
- Keep the game as close to the original as possible; new options go inside
  Settings, never as extra main/pause menu buttons.
