# Session handoff

Overwrite this file at every stopping point. Keep it short: the next session
reads it first (via `sh scripts/resume.sh`).

## Last update
2026-10-20, after owner playtest 8 (build 1faf8d2). Worktree
`Portal2-wheatley`, branch `wheatley`, pushes to `main`.

## Where things stand
- Repo tidied 2026-10-20: TASKS.md is one entry per task with a "Priority
  order" at the top and an "Awaiting owner replay" list; raw playtests moved
  to docs/PLAYTESTS.md, finished-fix write-ups to docs/STATUS.md; durable
  gotchas moved to WORKFLOW.md; PROJECT_STATE map table updated from
  playtest 8 (owner played chapter 1 to sp_a2_laser_stairs).
- Last code change: T-128 laser_stairs stairs rise (8ac53a2), owner replay
  pending. dist/Prtl2Mac.app = 0.1.0+8ac53a2.
- Depot 624 (Windows server.dll/client.dll) installed as behaviour reference.

## Next action
1. TASKS "Priority order" item 1: T-129 stray screens (one blocks the
   sp_a2_laser_stairs arrival staircase). Then T-130, T-131, T-105, T-132.

## Uncommitted / in-flight work
None outside what this file's commit contains.

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
