#!/bin/sh
# Reference screenshot from Windows Portal 2 in the CrossOver bottle "Steam"
# (T-215). Usage:
#   sh scripts/winref.sh <map> <out.jpg> [--pos X Y Z] [--ang P Y R]
#       [--cmd "console commands"] [--settle SEC] [--timeout SEC] [--hud]
# Why not a cfg: Portal 2 only allows `wait` once a server runs, so cfg waits
# can't outlast the slow CrossOver load. Instead the game is started with
# -netconport and this script talks to its console over TCP: it polls
# `getpos` until the player exists (map loaded), sends the commands, waits
# --settle seconds, takes a `jpeg` (portal2/screenshots/<map>NNNN.jpg),
# copies it out and quits. WINREF_LOG=<file> saves the console transcript. Steam is shut
# down afterwards; only screenshots this script created are removed.
set -u
MAP=${1:?map}; OUT=${2:?out.jpg}; shift 2
POS=""; ANG=""; CMD=""; SETTLE=6; TIMEOUT=420; HUD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --pos) POS="setpos $2 $3 $4"; shift 4 ;;
    --ang) ANG="setang $2 $3 $4"; shift 4 ;;
    --cmd) CMD="$2"; shift 2 ;;
    --settle) SETTLE="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    --hud) HUD=1; shift ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
done

WINE=/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine
STEAM='C:\Program Files (x86)\Steam\steam.exe'
G="$HOME/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/Portal 2/portal2"
SHOTS="$G/screenshots"
PORT=27999; PW=$(od -An -N6 -tx1 /dev/urandom | tr -d ' \n')
mkdir -p "$SHOTS"
BEFORE=$(ls "$SHOTS")

if [ "$HUD" = 1 ]; then HUDCMD=""; else
  HUDCMD="cl_drawhud 0; crosshair 0; closecaption 0; cc_subtitles 0; hud_quickinfo 0"
fi

"$WINE" --bottle Steam "$STEAM" -silent -applaunch 620 -windowed -w 1280 -h 720 \
  -novid -netconport $PORT -netconpassword "$PW" +map "$MAP" >/dev/null 2>&1 &

python3 -I - "$PORT" "$PW" "$TIMEOUT" "$SETTLE" "$SHOTS" "$OUT" \
  "con_drawnotify 0; con_notifytime 0; sv_cheats 1; $HUDCMD; jpeg_quality 95" "$POS; $ANG; $CMD" <<'EOF'
import os, shutil, socket, sys, time
port, pw, timeout, settle, shots, out, setup, place = sys.argv[1:]
timeout, settle = float(timeout), float(settle)
start = time.time()
def left(): return timeout - (time.time() - start)
s = None
while left() > 0 and s is None:
    try: s = socket.create_connection(("127.0.0.1", int(port)), timeout=3)
    except OSError: time.sleep(3)
if s is None: sys.exit("winref: netcon never came up")
s.settimeout(1.0)
def send(c): s.sendall((c + "\n").encode())
def read(secs):
    buf, end = b"", time.time() + secs
    while time.time() < end:
        try:
            d = s.recv(65536)
            if not d: break
            buf += d
        except socket.timeout: pass
    txt = buf.decode("latin-1", "replace")
    if os.environ.get("WINREF_LOG"):
        with open(os.environ["WINREF_LOG"], "a") as f: f.write(txt)
    return txt
send("PASS " + pw); read(1)
# getpos answers "setpos 0 0 0" before the player spawns; the load is over
# once the client prints "Redownloading all lightmaps" (lightmaps uploaded).
loaded, seen = False, ""
while left() > 0:
    send("getpos")
    seen += read(3)
    if "Redownloading all lightmaps" in seen: loaded = True; break
if not loaded: sys.exit("winref: map never finished loading")
print(f"winref: map loaded after {time.time() - start:.0f} s")
send(setup); send(place); read(settle)
send(place); read(0.5)
old = set(os.listdir(shots))
send("jpeg")
got = None
while left() > 0 and got is None:
    read(1)
    new = sorted(f for f in set(os.listdir(shots)) - old if f.endswith(".jpg"))
    if new: got = new[-1]
if got:
    time.sleep(1); shutil.copy(os.path.join(shots, got), out)
    print(f"winref: {out} ({got})")
send("quit"); read(2)
sys.exit(0 if got else "winref: jpeg never appeared")
EOF
RC=$?
sleep 6
"$WINE" --bottle Steam "$STEAM" -shutdown >/dev/null 2>&1
sleep 5
pkill -f 'Portal 2\\portal2.exe' 2>/dev/null
for f in $(ls "$SHOTS" | grep '\.jpg$'); do
  echo "$BEFORE" | grep -qx "$f" || rm -f "$SHOTS/$f"
done
exit $RC
