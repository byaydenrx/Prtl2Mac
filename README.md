<p align="center">
  <img src="docs/images/ferris-portals.png" alt="Ferris the Rust crab holding a portal gun, between a blue and an orange portal" width="360">
</p>

# Prtl2Mac

**Portal 2's single-player campaign, running natively on Apple Silicon Macs, from your own copy of the game.**
Valve's Mac build of Portal 2 is 32-bit and stopped running at macOS Catalina. Prtl2Mac is a Rust/[Bevy](https://bevyengine.org)
engine that reads the original game's maps, models, materials, compiled shaders, sounds, scripts and scenes from your
Steam install and plays them as a native arm64 Metal app.

> [!IMPORTANT]
> **You must own Portal 2.** Nothing from the game is in this repository; everything is read from your own install at
> run time. Without the game the app shows a dialog and exits.

> [!WARNING]
> **Work in progress.** All 63 single-player maps load. Chapter 1 has been played through end to end (except a
> noclip in sp_a1_intro6), and chapter 2 up to sp_a2_laser_stairs. Most later chambers are not yet proven solvable.
> Per-map status, with evidence, is in [PROJECT_STATE.md](PROJECT_STATE.md).

## Requirements

- An Apple Silicon Mac on macOS 13 or later.
- Portal 2's content installed through Steam. The app searches your Steam libraries and the depot folder macOS Steam
  downloads into; set `PORTAL2_DIR` to the folder that contains `portal2/` if it is somewhere else.
- [Rust](https://rustup.rs) (stable) to build it.
- Optional: `ffmpeg` from Homebrew, for the intro, menu and elevator-screen movies (they are skipped without it).

## Build and run

```sh
cargo build --release -p p2_game
./target/release/portal2               # main menu
./target/release/portal2 sp_a1_intro1  # start a map directly
sh scripts/make_app.sh                 # dist/Prtl2Mac.app (ad-hoc signed); open it like any app
```

More commands, including the scripted autopilot runs used for testing, are in [TEST_COMMANDS.md](TEST_COMMANDS.md).

## How it works

| Layer | Crates |
|---|---|
| The game: levels, entity I/O, puzzle entities, scripts, scenes, saves, menus | `p2_game` (binary `portal2`), `p2_mods` |
| Portals, portal gun, player, Chell, particles and sounds (from upstream 0.1.0) | `p2_portals` and the `p2_*` crates it groups |
| Reading the game's files; Portal 2's own D3D9 shaders translated to WGSL; level scripts | `source_assets`, `d3d9_*`, `vscript`, `p2_assets` |

Behaviour comes from the levels' own entity wiring and scripts, public file formats, observation of the original
game and study of its binaries; no Valve code is in this repository. Code comments cite behaviour notes under `research/specs/`, which are not in this repository.

## Working on it

The project is developed mostly by Claude Code sessions. Start with [CLAUDE.md](CLAUDE.md) and [WORKFLOW.md](WORKFLOW.md);
the backlog is [TASKS.md](TASKS.md) (owner-held tasks in [docs/TASKS_ON_HOLD.md](docs/TASKS_ON_HOLD.md)), and
[SESSION_HANDOFF.md](SESSION_HANDOFF.md) says where the last session stopped.

## Credits and licence

Prtl2Mac is built from [Portal 2 Rust Engine](https://github.com/Stink-O/Portal2-rust-engine) release 0.1.0 by
Stink-O, MIT licensed; upstream's own README is kept in [docs/UPSTREAM_README.md](docs/UPSTREAM_README.md). Prtl2Mac is
MIT licensed too: see [LICENSE](LICENSE). Third-party code and its licences are listed in [NOTICE](NOTICE): the
`d3d9_*` crates come from [IW4L](https://github.com/vladtrc/iw4L) (Apache-2.0). The mascot is
[Ferris](https://rustacean.net) by Karen Rustad Tölva (public domain), with a portal gun.

An independent fan project, not affiliated with or endorsed by Valve. Portal 2 is Valve's, and you need your own copy.
