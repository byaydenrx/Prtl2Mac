//! Level scripts: every entity with a `vscripts` keyvalue runs its Squirrel
//! files from the install (GLaDOS's and Wheatley's dialogue tables, the
//! transition list, the elevator motifs...) in our interpreter
//! ([`vscript`]). `RunScriptCode` / `CallScriptFunction` inputs run code in
//! an entity's scope, `thinkfunction`s tick, and functions hooked with
//! `ConnectOutput` run when the output fires. The engine calls the scripts
//! make (`EntFire`, `CreateSceneEntity`, `Entities.FindByName`...) are
//! answered here from the entity table.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use p2_assets::GameFsRes;
use p2_core::*;
use vscript::{Host, Key, Value, Vm};

use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents, GameEnt, PLAYER};
use crate::level::{CurrentMap, GameSet, LevelLoaded};
use crate::scenes::Playing;

pub struct VScriptPlugin;

impl Plugin for VScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Scripts>()
            .init_resource::<ScriptedEnts>()
            .add_message::<ConsoleCommand>()
            .add_systems(Update, start_scripts.in_set(GameSet::Logic).before(crate::io::run_io))
            .add_systems(Update, run_scripts.in_set(GameSet::Logic).after(crate::io::run_io));
    }
}

/// Entities whose scripts loaded: their `RunScriptCode` belongs to the
/// script, not to built-in stand-ins.
#[derive(Resource, Default)]
pub struct ScriptedEnts(pub HashSet<usize>);

/// A console command from a script (`SendToConsole`) or a map.
#[derive(Message, Clone, Debug)]
pub struct ConsoleCommand(pub String);

/// The interpreter and its per-level state.
#[derive(Resource, Default)]
pub struct Scripts {
    vm: Option<VmBox>,
    /// (entity, function, next call time).
    thinks: Vec<(usize, String, f32)>,
    /// Scene lengths by file.
    scene_len: HashMap<String, f32>,
    teams: HashMap<usize, i64>,
    /// Functions already reported missing (log once).
    missing: HashSet<String>,
    rng: u32,
}

/// The VM uses `Rc`; it is only ever touched from these systems (one at a
/// time, never shared between threads at once).
struct VmBox(Vm);
unsafe impl Send for VmBox {}
unsafe impl Sync for VmBox {}

/// Handles: entity index + 1 (0 is "no entity"); the `Entities` object.
const ENTITIES: u64 = 1 << 40;
fn handle(i: usize) -> Value {
    Value::Handle(i as u64 + 1)
}
fn ent_of(v: &Value) -> Option<usize> {
    match v {
        Value::Handle(h) if *h > 0 && *h < ENTITIES => Some(*h as usize - 1),
        _ => None,
    }
}

const NATIVES: &[&str] = &[
    "Time", "FrameTime", "GetMapName", "RandomInt", "RandomFloat", "EntFire", "EntFireByHandle", "DoEntFire", "CreateSceneEntity", "SendToConsole",
    "GetPlayer", "IsMultiplayer", "LoopSinglePlayerMaps", "GivePlayerPortalgun", "UpgradePlayerPortalgun", "UpgradePlayerPotatogun",
    "TryDLC1InstalledOrCatch", "UniqueString", "ShowMessage", "ScriptShowHudMessageAll", "GetDeveloperLevel", "Developer", "Msg",
    "RecordAchievementEvent", "IsLocalSplitScreen", "PrecacheMovie", "ScriptIsAlive", "AddThinkToEnt", "GetPlayerSilenced",
    "ScriptSteamShowURL", "SetDucking", "IsPlayerABot", "GetCoopSectionIndex", "GetHighestActiveBranch", "GetNumPlayersConnected",
    "GetMapIndexInPlayOrder", "GetNumMapsPlayed",
];

/// The engine as scripts see it, for one call.
struct GameHost<'a> {
    ents: &'a mut Ents,
    map: &'a str,
    game: &'a GameFsRes,
    playing: &'a Playing,
    scene_len: &'a mut HashMap<String, f32>,
    teams: &'a mut HashMap<usize, i64>,
    thinks: &'a mut Vec<(usize, String, f32)>,
    sounds: Vec<PlayGameSound>,
    console: Vec<String>,
    caller: usize,
    player_pos: Vec3,
    player_fwd: Vec3,
    rng: &'a mut u32,
    frame_time: f32,
}

fn to_source(v: Vec3) -> Value {
    let s = v / shared::SOURCE_UNIT;
    Value::vector(s.x as f64, -s.z as f64, s.y as f64)
}
fn from_source(v: &Value) -> Option<Vec3> {
    match v {
        Value::Vector(r) => {
            let a = *r.borrow();
            Some(shared::source_to_world(Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32)))
        }
        _ => None,
    }
}
fn s(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl GameHost<'_> {
    fn rand(&mut self) -> u32 {
        *self.rng ^= *self.rng << 13;
        *self.rng ^= *self.rng >> 17;
        *self.rng ^= *self.rng << 5;
        *self.rng
    }
    fn origin(&self, i: usize) -> Vec3 {
        if i == PLAYER {
            self.player_pos
        } else {
            self.ents.list[i].transform.translation
        }
    }
    fn scene_length(&mut self, i: usize) -> f32 {
        let file = self.ents.list[i].get("scenefile").unwrap_or("").replace('\\', "/").to_ascii_lowercase();
        if let Some(l) = self.scene_len.get(&file) {
            return *l;
        }
        let l = self.game.read(&file).map(|b| crate::scenes::parse_vcd(&String::from_utf8_lossy(&b)).length).unwrap_or(0.0);
        self.scene_len.insert(file, l);
        l
    }
    /// `Entities.FindBy*`: the first match after `prev`.
    fn find_after(&self, prev: &Value, pred: impl Fn(usize, &GameEnt) -> bool) -> Value {
        let start = ent_of(prev).map_or(0, |i| i + 1);
        for i in start..self.ents.list.len() {
            let e = &self.ents.list[i];
            if !e.killed && pred(i, e) {
                return handle(i);
            }
        }
        Value::Null
    }
}

fn name_matches(e: &GameEnt, i: usize, pat: &str) -> bool {
    let p = pat.to_ascii_lowercase();
    if i == PLAYER {
        return p == "!player" || p == "player";
    }
    match p.strip_suffix('*') {
        Some(prefix) => e.name.starts_with(prefix) && !e.name.is_empty(),
        None => e.name == p,
    }
}

impl Host for GameHost<'_> {
    fn call(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        let a = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let f = |i: usize| args.get(i).and_then(Value::as_f64).unwrap_or(0.0);
        Some(Ok(match name {
            "Time" => Value::Float(self.ents.now as f64),
            "FrameTime" => Value::Float(self.frame_time as f64),
            "GetMapName" => Value::str(self.map),
            "RandomInt" => {
                let (lo, hi) = (f(0) as i64, f(1) as i64);
                let r = self.rand() as i64;
                Value::Int(if hi > lo { lo + r % (hi - lo + 1) } else { lo })
            }
            "RandomFloat" => {
                let r = (self.rand() % 1_000_000) as f64 / 1_000_000.0;
                Value::Float(f(0) + (f(1) - f(0)) * r)
            }
            "EntFire" | "DoEntFire" => {
                let activator = ent_of(&a(4));
                let caller = ent_of(&a(5)).unwrap_or(self.caller);
                self.ents.send_from(&s(&a(0)), &s(&a(1)), &s(&a(2)), f(3) as f32, caller, activator);
                Value::Null
            }
            "EntFireByHandle" => {
                if let Some(i) = ent_of(&a(0)) {
                    let activator = ent_of(&a(4));
                    let caller = ent_of(&a(5)).unwrap_or(self.caller);
                    self.ents.send_from(&format!("#{i}"), &s(&a(1)), &s(&a(2)), f(3) as f32, caller, activator);
                }
                Value::Null
            }
            "CreateSceneEntity" => {
                let file = s(&a(0));
                let i = self.ents.add(GameEnt::logic("logic_choreographed_scene", "", vec![("scenefile".into(), file)]));
                handle(i)
            }
            "SendToConsole" => {
                self.console.push(s(&a(0)));
                Value::Null
            }
            "GetPlayer" => handle(PLAYER),
            "IsMultiplayer" | "LoopSinglePlayerMaps" | "IsLocalSplitScreen" | "GetPlayerSilenced" | "IsPlayerABot" => Value::Bool(false),
            "GivePlayerPortalgun" => {
                self.console.push("give weapon_portalgun".into());
                Value::Null
            }
            "UpgradePlayerPortalgun" => {
                self.console.push("upgrade_portalgun".into());
                Value::Null
            }
            "UpgradePlayerPotatogun" => {
                self.console.push("upgrade_potatogun".into());
                Value::Null
            }
            // Throws unless the map has the DLC's stats controller (only
            // community maps do); TransitionFromMap then changes level itself.
            "TryDLC1InstalledOrCatch" => return Some(Err("DLC1 level-end stats are not used here".into())),
            "ShowMessage" | "ScriptShowHudMessageAll" | "Msg" | "RecordAchievementEvent" | "PrecacheMovie" | "SetDucking"
            | "ScriptSteamShowURL" => Value::Null,
            "GetDeveloperLevel" | "Developer" | "GetCoopSectionIndex" | "GetHighestActiveBranch" => Value::Int(0),
            "GetNumPlayersConnected" => Value::Int(1),
            // Community-map play order; -2 means "not a community map".
            "GetMapIndexInPlayOrder" => Value::Int(-2),
            "GetNumMapsPlayed" => Value::Int(0),
            "ScriptIsAlive" => Value::Bool(ent_of(&a(0)).is_some_and(|i| !self.ents.list[i].killed)),
            "UniqueString" => Value::Str(format!("u{}_{}", self.rand(), s(&a(0))).into()),
            "AddThinkToEnt" => {
                if let Some(i) = ent_of(&a(0)) {
                    self.thinks.retain(|t| t.0 != i);
                    let fname = s(&a(1));
                    if !fname.is_empty() {
                        self.thinks.push((i, fname, self.ents.now));
                    }
                }
                Value::Null
            }
            _ => return None,
        }))
    }

    fn method(&mut self, h: u64, name: &str, args: &[Value]) -> Result<Value, String> {
        let a = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let f = |i: usize| args.get(i).and_then(Value::as_f64).unwrap_or(0.0);
        if h == ENTITIES {
            let origin = from_source(&a(1));
            let radius = (f(2) as f32) * shared::SOURCE_UNIT;
            return Ok(match name {
                "First" => self.find_after(&Value::Null, |_, _| true),
                "Next" => self.find_after(&a(0), |_, _| true),
                "FindByName" => {
                    let n = s(&a(1));
                    self.find_after(&a(0), |i, e| name_matches(e, i, &n))
                }
                "FindByClassname" => {
                    let c = s(&a(1)).to_ascii_lowercase();
                    self.find_after(&a(0), |_, e| e.class == c)
                }
                "FindByClassnameWithin" => {
                    let c = s(&a(1)).to_ascii_lowercase();
                    let (o, r) = (from_source(&a(2)).unwrap_or(Vec3::ZERO), f(3) as f32 * shared::SOURCE_UNIT);
                    let player = self.player_pos;
                    self.find_after(&a(0), |i, e| e.class == c && (if i == PLAYER { player } else { e.transform.translation }).distance(o) <= r)
                }
                "FindInSphere" => {
                    let (o, r) = (from_source(&a(1)).unwrap_or(Vec3::ZERO), f(2) as f32 * shared::SOURCE_UNIT);
                    let player = self.player_pos;
                    self.find_after(&a(0), |i, e| (if i == PLAYER { player } else { e.transform.translation }).distance(o) <= r)
                }
                "FindByNameNearest" | "FindByClassnameNearest" => {
                    let key = s(&a(0)).to_ascii_lowercase();
                    let o = origin.unwrap_or(Vec3::ZERO);
                    let mut best: Option<(usize, f32)> = None;
                    for (i, e) in self.ents.list.iter().enumerate() {
                        let hit = if name == "FindByNameNearest" { name_matches(e, i, &key) } else { e.class == key };
                        let d = self.origin(i).distance(o);
                        if hit && !e.killed && (radius <= 0.0 || d <= radius) && best.is_none_or(|b| d < b.1) {
                            best = Some((i, d));
                        }
                    }
                    best.map_or(Value::Null, |b| handle(b.0))
                }
                "FindByTarget" => {
                    let t = s(&a(1)).to_ascii_lowercase();
                    self.find_after(&a(0), |_, e| e.get("target").is_some_and(|v| v.eq_ignore_ascii_case(&t)))
                }
                "FindByModel" => {
                    let m = s(&a(1)).to_ascii_lowercase();
                    self.find_after(&a(0), |_, e| e.get("model").is_some_and(|v| v.eq_ignore_ascii_case(&m)))
                }
                "CreateByClassname" => {
                    let c = s(&a(0));
                    handle(self.ents.add(GameEnt::logic(&c, "", Vec::new())))
                }
                _ => return Err(format!("Entities has no method {name}")),
            });
        }
        let Some(i) = ent_of(&Value::Handle(h)).filter(|i| *i < self.ents.list.len()) else {
            return Err(format!("{name} on an invalid entity handle"));
        };
        Ok(match name {
            "IsValid" => Value::Bool(!self.ents.list[i].killed),
            "GetName" => Value::str(if i == PLAYER { "" } else { &self.ents.list[i].name }),
            "GetPreTemplateName" => Value::str(&self.ents.list[i].name),
            "GetClassname" => Value::str(&self.ents.list[i].class),
            "GetModelName" => Value::str(self.ents.list[i].get("model").unwrap_or("")),
            "entindex" => Value::Int(i as i64),
            "GetOrigin" | "GetLocalOrigin" | "GetCenter" | "EyePosition" => to_source(self.origin(i)),
            "SetOrigin" | "SetAbsOrigin" => {
                if let Some(p) = from_source(&a(0)) {
                    self.ents.list[i].transform.translation = p;
                }
                Value::Null
            }
            "GetForwardVector" => {
                let fwd = if i == PLAYER { self.player_fwd } else { self.ents.list[i].transform.rotation * Vec3::X };
                let v = Vec3::new(fwd.x, -fwd.z, fwd.y);
                Value::vector(v.x as f64, v.y as f64, v.z as f64)
            }
            "GetAngles" => {
                let (y, x, z) = self.ents.list[i].transform.rotation.to_euler(EulerRot::YXZ);
                Value::vector(-x.to_degrees() as f64, y.to_degrees() as f64, z.to_degrees() as f64)
            }
            "GetVelocity" | "GetBoundingMins" | "GetBoundingMaxs" => Value::vector(0.0, 0.0, 0.0),
            "SetVelocity" | "SetAngles" | "SetForwardVector" | "SetModel" | "SetSize" | "SetOwner" | "SetHealth" | "SetMaxHealth" | "StopSound"
            | "PrecacheSoundScript" | "PrecacheScriptSound" | "SetGravity" | "SetFriction" | "__KeyValueFromVector" => Value::Null,
            "GetHealth" | "GetMaxHealth" => Value::Int(100),
            "IsNoclipping" => Value::Bool(false),
            "GetTeam" => Value::Int(*self.teams.get(&i).unwrap_or(&0)),
            "SetTeam" => {
                self.teams.insert(i, f(0) as i64);
                Value::Null
            }
            "Destroy" => {
                self.ents.send_from(&format!("#{i}"), "kill", "", 0.0, self.caller, None);
                Value::Null
            }
            "EmitSound" => {
                let at = Some(self.origin(i)).filter(|_| i != PLAYER);
                self.sounds.push(PlayGameSound { sound: s(&a(0)), at, looped: false, owner: None, music: false, volume: 1.0 });
                Value::Null
            }
            "ConnectOutput" => {
                let (out, func) = (s(&a(0)), s(&a(1)));
                if !self.ents.script_hooks.iter().any(|h| h.0 == i && h.1.eq_ignore_ascii_case(&out) && h.2 == func) {
                    self.ents.script_hooks.push((i, out, func));
                }
                Value::Null
            }
            "DisconnectOutput" => {
                let (out, func) = (s(&a(0)), s(&a(1)));
                self.ents.script_hooks.retain(|h| !(h.0 == i && h.1.eq_ignore_ascii_case(&out) && h.2 == func));
                Value::Null
            }
            "IsPlayingBack" => Value::Bool(self.playing.is_playing(i)),
            "GetCurrentScene" => self.playing.current_for(&self.ents.list[i].name).map_or(Value::Null, handle),
            "EstimateLength" => Value::Float(self.scene_length(i) as f64),
            "__KeyValueFromString" | "__KeyValueFromInt" | "__KeyValueFromFloat" => {
                let (k, v) = (s(&a(0)).to_ascii_lowercase(), s(&a(1)));
                let e = &mut self.ents.list[i];
                e.keyvalues.retain(|(kk, _)| *kk != k);
                e.keyvalues.push((k, v));
                Value::Bool(true)
            }
            "GetMoveParent" | "GetRootMoveParent" | "FirstMoveChild" | "NextMovePeer" | "GetOwner" => {
                let parent = self.ents.list[i].get("parentname").map(str::to_string);
                match (name, parent) {
                    ("GetMoveParent" | "GetRootMoveParent", Some(p)) => self.ents.find(&p, i, None).first().map_or(Value::Null, |p| handle(*p)),
                    _ => Value::Null,
                }
            }
            _ => return Err(format!("entity method {name} is not available")),
        })
    }

    fn read_script(&mut self, name: &str) -> Option<String> {
        let n = name.trim_start_matches('/').replace('\\', "/");
        let n = n.strip_suffix(".nut").unwrap_or(&n).to_string();
        let b = self.game.read(&format!("scripts/vscripts/{n}.nut"))?;
        Some(String::from_utf8_lossy(&b).into_owned())
    }

    fn print(&mut self, text: &str) {
        let t = text.trim_end();
        if !t.is_empty() {
            debug!("vscript: {t}");
        }
    }
}

/// Load every entity's scripts when a level comes up (before its autos fire).
#[allow(clippy::too_many_arguments)]
fn start_scripts(
    mut loaded: MessageReader<LevelLoaded>,
    mut scripts: ResMut<Scripts>,
    mut scripted: ResMut<ScriptedEnts>,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    game: Res<GameFsRes>,
    playing: Res<Playing>,
    mut play: MessageWriter<PlayGameSound>,
    mut console: MessageWriter<ConsoleCommand>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    let (Some(mut ents), Some(map)) = (ents, map) else { return };
    if map.menu || std::env::var_os("P2_NOSCRIPTS").is_some() {
        scripts.vm = None;
        scripted.0.clear();
        return;
    }
    let t0 = std::time::Instant::now();
    let mut vm = Vm::new(NATIVES);
    vm.root.borrow_mut().map.insert(Key::str("Entities"), Value::Handle(ENTITIES));
    vm.root.borrow_mut().map.insert(Key::str("player"), handle(PLAYER));
    let sc = &mut *scripts;
    sc.thinks.clear();
    sc.teams.clear();
    sc.rng = 0x1234_5678;
    scripted.0.clear();
    let with_scripts: Vec<(usize, Vec<String>, Option<String>)> = ents
        .list
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let files: Vec<String> = e.get("vscripts")?.split_whitespace().map(str::to_string).collect();
            Some((i, files, e.get("thinkfunction").filter(|t| !t.is_empty()).map(str::to_string)))
        })
        .collect();
    let mut host = GameHost {
        ents: &mut ents,
        map: &map.info.name,
        game: &game,
        playing: &playing,
        scene_len: &mut sc.scene_len,
        teams: &mut sc.teams,
        thinks: &mut sc.thinks,
        sounds: Vec::new(),
        console: Vec::new(),
        caller: PLAYER,
        player_pos: Vec3::ZERO,
        player_fwd: Vec3::NEG_Z,
        rng: &mut sc.rng,
        frame_time: 1.0 / 60.0,
    };
    // mapspawn.nut runs in the root table first.
    let root = vm.root.clone();
    if let Err(e) = vm.run_file(root, "mapspawn", &mut host) {
        debug!("vscript: mapspawn: {e}");
    }
    let mut ok = Vec::new();
    for (i, files, think) in &with_scripts {
        let scope = vm.scope(*i as u64 + 1);
        scope.borrow_mut().map.insert(Key::str("self"), handle(*i));
        host.caller = *i;
        let mut good = true;
        for f in files {
            if let Err(e) = vm.run_file(scope.clone(), f.trim_end_matches(".nut"), &mut host) {
                warn!("vscript: {} {:?} {f}: {e}", host.ents.list[*i].class, host.ents.list[*i].name);
                good = false;
            }
        }
        if good {
            ok.push(*i);
        }
        if let Some(t) = think {
            host.thinks.push((*i, t.clone(), 0.0));
        }
    }
    // After every entity has spawned: Precache, then OnPostSpawn.
    for &i in &ok {
        host.caller = i;
        let scope = vm.scope(i as u64 + 1);
        for f in ["Precache", "OnPostSpawn"] {
            if let Err(e) = vm.call_in(scope.clone(), f, &[], &mut host) {
                warn!("vscript: {:?} {f}: {e}", host.ents.list[i].name);
            }
        }
    }
    for s in host.sounds.drain(..) {
        play.write(s);
    }
    for c in host.console.drain(..) {
        console.write(ConsoleCommand(c));
    }
    info!(
        "portal2: {} scripted entities ({} loaded) in {:.0} ms, {} entities now",
        with_scripts.len(),
        ok.len(),
        t0.elapsed().as_secs_f32() * 1000.0,
        ents.list.len()
    );
    scripted.0 = ok.into_iter().collect();
    scripts.vm = Some(VmBox(vm));
}

/// Inputs that run script code, hooked outputs, think functions.
#[allow(clippy::too_many_arguments)]
fn run_scripts(
    time: Res<Time>,
    mut inputs: MessageReader<EntInput>,
    mut scripts: ResMut<Scripts>,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    game: Res<GameFsRes>,
    playing: Res<Playing>,
    players: Query<(&Transform, &ViewAngles), With<Player>>,
    mut play: MessageWriter<PlayGameSound>,
    mut console: MessageWriter<ConsoleCommand>,
) {
    let (Some(mut ents), Some(map)) = (ents, map) else {
        inputs.clear();
        return;
    };
    let sc = &mut *scripts;
    let Some(VmBox(vm)) = sc.vm.as_mut() else {
        inputs.clear();
        return;
    };
    let (player_pos, player_fwd) = players.single().map(|(t, v)| (t.translation, v.forward())).unwrap_or((Vec3::ZERO, Vec3::NEG_Z));
    // What to run this frame: (entity, kind, text, activator).
    let mut work: Vec<(usize, &'static str, String, Option<usize>)> = Vec::new();
    for msg in inputs.read() {
        match msg.input.as_str() {
            "runscriptcode" => work.push((msg.target, "code", msg.param.clone(), msg.activator)),
            "callscriptfunction" => work.push((msg.target, "call", msg.param.clone(), msg.activator)),
            "runscriptfile" => work.push((msg.target, "file", msg.param.clone(), msg.activator)),
            _ => {}
        }
    }
    for (i, func, activator, caller) in std::mem::take(&mut ents.script_calls) {
        work.push((i, "hook", func, activator.or(Some(caller))));
    }
    let now = ents.now;
    let due: Vec<usize> = sc.thinks.iter().enumerate().filter(|(_, t)| t.2 <= now).map(|(k, _)| k).collect();
    let mut think_work = Vec::new();
    for k in due {
        let (i, f, _) = sc.thinks[k].clone();
        sc.thinks[k].2 = now + 0.1;
        think_work.push((k, i, f));
    }
    if work.is_empty() && think_work.is_empty() {
        return;
    }
    let mut host = GameHost {
        ents: &mut ents,
        map: &map.info.name,
        game: &game,
        playing: &playing,
        scene_len: &mut sc.scene_len,
        teams: &mut sc.teams,
        thinks: &mut sc.thinks,
        sounds: Vec::new(),
        console: Vec::new(),
        caller: PLAYER,
        player_pos,
        player_fwd,
        rng: &mut sc.rng,
        frame_time: time.delta_secs(),
    };
    for (i, kind, text, activator) in work {
        host.caller = i;
        let scope = vm.scope(i as u64 + 1);
        {
            let mut sb = scope.borrow_mut();
            sb.map.entry(Key::str("self")).or_insert(handle(i));
            sb.map.insert(Key::str("activator"), activator.map_or(Value::Null, handle));
            sb.map.insert(Key::str("caller"), activator.map_or(Value::Null, handle));
        }
        let r = match kind {
            "code" => vm.run_code(scope, &text, &mut host).map(|_| ()),
            "file" => vm.run_file(scope, text.trim_end_matches(".nut"), &mut host),
            "hook" => {
                // Scene callbacks look up the scene that fired as `owninginstance`.
                vm.root.borrow_mut().map.insert(Key::str("owninginstance"), handle(i));
                vm.call_in(scope, &text, &[], &mut host).map(|_| ())
            }
            _ => match vm.call_in(scope, text.trim(), &[], &mut host) {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(format!("no function {text}")),
                Err(e) => Err(e),
            },
        };
        if let Err(e) = r {
            let key = format!("{}:{text}", host.ents.list[i].name);
            if sc.missing.insert(key) {
                warn!("vscript: {} {:?} {kind} {text:?}: {e}", host.ents.list[i].class, host.ents.list[i].name);
            }
        }
    }
    for (k, i, f) in think_work {
        host.caller = i;
        let scope = vm.scope(i as u64 + 1);
        match vm.call_in(scope, &f, &[], &mut host) {
            Ok(Some(Value::Float(next))) if next > 0.0 => {
                if let Some(t) = host.thinks.get_mut(k) {
                    t.2 = now + next as f32;
                }
            }
            Ok(_) => {}
            Err(e) => {
                if sc.missing.insert(format!("think:{i}:{f}")) {
                    warn!("vscript: think {f} on {:?}: {e}", host.ents.list[i].name);
                }
            }
        }
    }
    for s in host.sounds.drain(..) {
        play.write(s);
    }
    for c in host.console.drain(..) {
        console.write(ConsoleCommand(c));
    }
}
