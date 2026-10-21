# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-21 (owner session). Owner round 5 recorded (docs/PLAYTESTS.md,
TASKS.md): 14 tasks confirmed and moved to Done. Attempt branches checked
by autopilot before/after: T-140 shake and T-117 shadows merged (be91da3);
T-175 and T-112 kept open; T-151 dropped; T-164 not judged (never in an
owner build). Reopened: T-173 hold beam ("looks terrible"), T-174 panel
layout, T-179 flips ("semi fixed").

## Where things stand
- dist/Prtl2Mac.app = BUILD_ID (T-140 + T-117 merged on top of b8a2d76).
- Owner is sending screenshots for: T-174 panels, T-173 hold beam, T-179
  flips, T-145, T-147, T-149, T-116.
- Still awaiting the owner: T-169, T-171, T-148, T-153, T-140, T-117, and the
  menu spec rows not yet named (T-177, T-178, T-180, T-181, T-183..T-185,
  T-187..T-191, T-193, T-194).
- Upstream checked 2026-10-21: only a GPL-3.0 relicense, not ported.

## Next action
1. T-175: implement env_fog_controller SetColor/SetStartDist/SetEndDist
   (+ foglerptime), merge attempt/T-175, verify intro1 ride and vault.
2. T-173 hold beam / T-174 panels / T-179 flips once the owner's screenshots
   arrive (record every item in TASKS.md first).
3. Then T-146 intro1 half-stuck door, the rest of the priority order.
MENUS FROZEN except T-179. Merge origin/main often.

## Uncommitted / in-flight work
None outside what this file's commit contains.

## Fix attempts awaiting the owner
- attempt/T-175 (1f4de9a): maps start on the master env_fog_controller
  (intro1 ride: light fog_intro_ride instead of the dense vault fog). Needs
  the fog inputs (SetColor/SetStartDist/SetEndDist) before merging, else the
  wrecked vault loses its dense fog. Replay the intro1 ride vs Windows.
- attempt/T-112 (cc81fdb): world-reflecting water (GLaDOS chamber,
  water_glados_01) samples a mirrored half-res camera view with an oblique
  clip plane at the water. Small change at shallow angles; replay
  sp_a1_wakeup's chamber (and check FPS there), then merge or delete.
- attempt/T-164 (c34006e): CPU skinning for soft-weighted animated meshes
  (intro1 ride debris). Review `git log -p wheatley..attempt/T-164`, replay
  the intro1 ride, then `git merge attempt/T-164` or delete the branch.

## Checkouts (2026-10-20)
- `Portal2-wheatley` (this one): level with origin/main; work happens here.
- `Portal2-rust-engine`: stuck mid-merge since Oct 19 (owner to decide; its
  one commit reached main as f2d9b71; a stash holds a 15-line animinfo
  debug addition). Leave it alone.
- `Portal2-ch35`, `Portal2-ch6-9`: clean, behind main, nothing unpushed.
- Disk: the other checkouts' target/ dirs (63 GB) were deleted 2026-10-20
  with the owner's OK; ~/Documents/Claude is ~9 GB (this checkout's target
  kept). Those checkouts rebuild from scratch if ever used again.
- GitHub branches: the 11 `cloud/*` branches (all merged into main) were
  deleted 2026-10-20. Tips, to recreate one with `git push origin
  <sha>:refs/heads/cloud/<name>`: aimers 0f09c0f, anim 1bb3e03, gels ac6b4a3,
  glass 3d62059, particles cf344e9, projtex 91503a8, proxies 6e173d1,
  saves ef01090, settings 353a151, tools a1053a1, ui 5fd9de8.
