# Faith plate check: stand on each trigger_catapult, see how close the player gets to its launch target.
# Usage (from the repo, after cargo build --release): python3 scripts/check_faith_plates.py sp_a2_catapult_intro ...
import os, re, subprocess, sys
REPO = "/Users/aydensnider/Documents/Claude/Portal2-rust-engine"
env = dict(os.environ, PORTAL2_DIR=os.path.expanduser("~/Library/Application Support/Steam/Steam.AppBundle/Steam/Contents/MacOS/steamapps/content/app_620/depot_621"))
ANSI = re.compile(r"\x1b\[[0-9;]*m")
def run(m, script, secs):
    e = dict(env, P2_SCRIPT=script)
    out = subprocess.run(["perl", "-e", f"alarm {secs}; exec @ARGV", "./target/release/portal2", m], cwd=REPO, env=e, capture_output=True, text=True); out = out.stdout + out.stderr
    return ANSI.sub("", out)
def v3(s): return [float(x) for x in s.split(",")]
for m in sys.argv[1:]:
    ents = subprocess.run(["./target/release/examples/ents", m, "--all"], cwd=REPO, env=env, capture_output=True, text=True).stdout
    cats = []
    cur = None
    for line in ents.splitlines():
        if not line.startswith(" "):
            cur = None
            mm = re.match(r'trigger_catapult Some\("([^"]+)"\)', line)
            if mm: cur = {"name": mm.group(1)}; cats.append(cur)
        elif cur is not None:
            k, _, v = line.strip().partition(" = ")
            cur[k] = v
    cats = [c for c in cats if c.get("launchtarget") and c.get("startdisabled") != "1" and float(c.get("playerspeed", "0")) > 0]
    names = " ".join(set([c["name"] for c in cats] + [c["launchtarget"] for c in cats]))
    info = run(m, "3 " + "; 3 ".join("info " + n for n in names.split()) + "; 3.2 quit", 60)
    pos, bounds = {}, {}
    for line in info.splitlines():
        mm = re.search(r'"([^"]+)" enabled .* at source Vec3\(([-\d.]+), ([-\d.]+), ([-\d.]+)\) bounds (Some\(\(Vec3\(([-\d.]+), ([-\d.]+), ([-\d.]+)\), Vec3\(([-\d.]+), ([-\d.]+), ([-\d.]+)\)\)\)|None)', line)
        if mm:
            pos[mm.group(1)] = [float(mm.group(i)) for i in (2, 3, 4)]
            if mm.group(6): bounds[mm.group(1)] = [float(mm.group(i)) for i in range(6, 12)]
    for c in cats:
        b = bounds.get(c["name"]); t = pos.get(c["launchtarget"].lower())
        if not b or not t:
            print(f"{m} {c['name']}: missing bounds/target"); continue
        cx, cy = (b[0] + b[3]) / 2, (b[1] + b[4]) / 2
        cz = min(b[2], b[5]) + 2
        script = f"3 setpos {cx:.0f} {cy:.0f} {cz:.0f}; " + "; ".join(f"{3.2 + 0.2 * k:.1f} log" for k in range(25)) + "; 8.5 quit"
        out = run(m, script, 40)
        feet = [[float(x) for x in mm.groups()] for mm in re.finditer(r"feet source \(([-\d.]+) ([-\d.]+) ([-\d.]+)\)", out)]
        if not feet:
            print(f"{m} {c['name']}: no positions"); continue
        best = min(feet, key=lambda f: sum((f[i] - t[i]) ** 2 for i in range(3)))
        d = sum((best[i] - t[i]) ** 2 for i in range(3)) ** 0.5
        print(f"{m} {c['name']} speed {c['playerspeed']}: closest {d:.0f} units to {c['launchtarget']} (end {feet[-1][0]:.0f} {feet[-1][1]:.0f} {feet[-1][2]:.0f}, target {t[0]:.0f} {t[1]:.0f} {t[2]:.0f})")
