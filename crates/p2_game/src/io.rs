//! The map's entity I/O: every entity's outputs (`OnPressed`, `OnTrigger`...)
//! queue inputs on named targets after a delay, the way Hammer wires a level.
//!
//! Logic entities (relays, counters, branches, cases, timers, autos and
//! instance proxies) run here. Inputs for everything else are sent on as
//! [`EntInput`] messages for the module that owns that class.
use std::collections::HashMap;

use bevy::prelude::*;
use source_assets::MapInfo;

/// One output connection: `event` fires `input` on `target` after `delay` s.
#[derive(Clone, Debug)]
pub struct Output {
    pub event: String,
    pub target: String,
    pub input: String,
    pub param: String,
    pub delay: f32,
    /// Times left to fire; -1 = unlimited.
    pub times: i32,
}

/// A map entity at run time.
#[derive(Clone, Debug)]
pub struct GameEnt {
    pub class: String,
    /// Lowercased targetname ("" when unnamed).
    pub name: String,
    pub keyvalues: Vec<(String, String)>,
    pub outputs: Vec<Output>,
    /// Spawned render/collision entity, if any.
    pub entity: Option<Entity>,
    pub transform: Transform,
    pub brush_model: Option<usize>,
    /// World-space box of a brush entity (triggers), metres.
    pub bounds: Option<(Vec3, Vec3)>,
    pub enabled: bool,
    pub killed: bool,
    pub spawnflags: u32,
    /// Counter / branch / compare value.
    pub value: f32,
    /// Extra per-class number (timer: next fire time; compare: compare value).
    pub aux: f32,
}

impl GameEnt {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keyvalues.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(source_assets::kv::parse_f32)
    }
}

/// An input that reached an entity whose class is handled elsewhere.
#[derive(Message, Clone, Debug)]
pub struct EntInput {
    pub target: usize,
    /// Lowercased input name.
    pub input: String,
    pub param: String,
    pub activator: Option<usize>,
}

/// Something outside the I/O system fired an output (a trigger, a button).
#[derive(Message, Clone, Debug)]
pub struct FireOutput {
    pub ent: usize,
    /// Output name, any case (`OnPressed`).
    pub event: String,
    pub activator: Option<usize>,
}

#[derive(Clone, Debug)]
struct Pending {
    at: f32,
    target: String,
    input: String,
    param: String,
    caller: usize,
    activator: Option<usize>,
}

/// Index of the player in [`Ents::list`].
pub const PLAYER: usize = 0;

#[derive(Resource, Default)]
pub struct Ents {
    pub list: Vec<GameEnt>,
    by_name: HashMap<String, Vec<usize>>,
    queue: Vec<Pending>,
    /// Seconds since the map spawned (game time: stops while paused).
    pub now: f32,
    started: bool,
    rng: u32,
}

impl Ents {
    pub fn from_map(info: &MapInfo, bounds: &HashMap<usize, (Vec3, Vec3)>) -> Self {
        let mut list = vec![GameEnt {
            class: "player".into(),
            name: "!player".into(),
            keyvalues: Vec::new(),
            outputs: Vec::new(),
            entity: None,
            transform: Transform::IDENTITY,
            brush_model: None,
            bounds: None,
            enabled: true,
            killed: false,
            spawnflags: 0,
            value: 0.0,
            aux: 0.0,
        }];
        for e in &info.entities {
            let get = |k: &str| e.keyvalues.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.as_str());
            let outputs = e.keyvalues.iter().filter_map(|(k, v)| parse_output(k, v)).collect();
            let start_disabled = get("startdisabled").is_some_and(|v| v.trim() == "1");
            let mut value = get("startvalue").or(get("initialvalue")).and_then(source_assets::kv::parse_f32).unwrap_or(0.0);
            // Buttons: value 1 = unlocked (spawnflag 2048 starts them locked).
            if matches!(e.classname.as_str(), "func_button" | "func_rot_button" | "prop_button" | "prop_under_button") {
                let flags: u32 = get("spawnflags").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                value = if flags & 2048 != 0 { 0.0 } else { 1.0 };
            }
            list.push(GameEnt {
                class: e.classname.to_ascii_lowercase(),
                name: e.targetname.clone().unwrap_or_default().to_ascii_lowercase(),
                keyvalues: e.keyvalues.clone(),
                outputs,
                entity: e.entity,
                transform: e.transform,
                brush_model: e.brush_model,
                // Brush models are stored relative to their entity's origin.
                bounds: e
                    .brush_model
                    .filter(|m| *m > 0)
                    .and_then(|m| bounds.get(&m).copied())
                    .map(|(lo, hi)| (lo + e.transform.translation, hi + e.transform.translation))
                    .or(e.bounds),
                enabled: !start_disabled,
                killed: false,
                spawnflags: get("spawnflags").and_then(|v| v.trim().parse().ok()).unwrap_or(0),
                value,
                aux: 0.0,
            });
        }
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, e) in list.iter().enumerate() {
            if !e.name.is_empty() {
                by_name.entry(e.name.clone()).or_default().push(i);
            }
        }
        if std::env::var_os("P2_TRACE").is_some() {
            for e in list.iter().filter(|e| e.class.starts_with("trigger_")) {
                info!("TRACE trigger {} {:?} flags {} enabled {} bounds {:.2?}", e.class, e.name, e.spawnflags, e.enabled, e.bounds);
            }
        }
        Ents { list, by_name, queue: Vec::new(), now: 0.0, started: false, rng: 0x2545_f491 }
    }

    /// Entities named `target` (wildcard `*` suffix, `!player`/`!activator`/`!self`/`!caller`).
    pub fn find(&self, target: &str, caller: usize, activator: Option<usize>) -> Vec<usize> {
        let t = target.trim().to_ascii_lowercase();
        match t.as_str() {
            "" => Vec::new(),
            "!player" | "!pvsplayer" | "player" => vec![PLAYER],
            "!activator" => activator.into_iter().collect(),
            "!self" | "!caller" => vec![caller],
            _ => {
                if let Some(prefix) = t.strip_suffix('*') {
                    self.list
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| !e.name.is_empty() && e.name.starts_with(prefix))
                        .map(|(i, _)| i)
                        .collect()
                } else if let Some(v) = self.by_name.get(&t) {
                    v.clone()
                } else {
                    // Some outputs name a class instead of an entity.
                    self.list.iter().enumerate().filter(|(_, e)| e.class == t).map(|(i, _)| i).collect()
                }
            }
        }
    }

    /// Fire every `event` output of `ent`.
    pub fn fire(&mut self, ent: usize, event: &str, activator: Option<usize>) {
        let now = self.now;
        let Some(e) = self.list.get_mut(ent) else { return };
        if e.killed {
            return;
        }
        let mut queued = Vec::new();
        for o in e.outputs.iter_mut().filter(|o| o.event.eq_ignore_ascii_case(event)) {
            if o.times == 0 {
                continue;
            }
            if o.times > 0 {
                o.times -= 1;
            }
            queued.push(Pending {
                at: now + o.delay.max(0.0),
                target: o.target.clone(),
                input: o.input.to_ascii_lowercase(),
                param: o.param.clone(),
                caller: ent,
                activator,
            });
        }
        if !queued.is_empty() {
            debug!("io: {} {:?} fires {event} ({} targets)", e.class, e.name, queued.len());
        }
        self.queue.extend(queued);
    }

    /// Queue an input directly (scripts, the console).
    pub fn send(&mut self, target: &str, input: &str, param: &str, delay: f32) {
        self.queue.push(Pending {
            at: self.now + delay,
            target: target.into(),
            input: input.to_ascii_lowercase(),
            param: param.into(),
            caller: PLAYER,
            activator: Some(PLAYER),
        });
    }

    fn rand(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    /// Run one input on one entity. Returns `true` when handled here.
    fn apply(&mut self, i: usize, input: &str, param: &str, caller: usize, activator: Option<usize>) -> bool {
        let class = self.list[i].class.clone();
        // Inputs every entity understands.
        match input {
            "kill" | "killhierarchy" => {
                self.list[i].killed = true;
                return false; // the owner despawns what it spawned
            }
            "enable" => self.list[i].enabled = true,
            "disable" => self.list[i].enabled = false,
            "toggle" if !class.starts_with("prop_") && !class.starts_with("func_door") => {
                self.list[i].enabled = !self.list[i].enabled
            }
            "fireuser1" | "fireuser2" | "fireuser3" | "fireuser4" => {
                let n = &input[8..];
                self.fire(i, &format!("onuser{n}"), activator);
                return true;
            }
            "addoutput" => {
                self.add_output(i, param);
                return true;
            }
            "cancelpending" => {
                self.queue.retain(|p| p.caller != i);
                return true;
            }
            _ => {}
        }
        match class.as_str() {
            "logic_relay" => {
                if input == "trigger" && self.list[i].enabled {
                    self.fire(i, "ontrigger", activator);
                    if self.list[i].spawnflags & 1 != 0 {
                        self.list[i].killed = true;
                    }
                } else if input == "enablerefire" {
                    self.list[i].enabled = true;
                }
                true
            }
            "func_instance_io_proxy" => {
                if input.starts_with("onproxyrelay") {
                    self.fire(i, input, activator);
                }
                true
            }
            "math_counter" => {
                let (min, max) = (self.list[i].get_f32("min").unwrap_or(0.0), self.list[i].get_f32("max").unwrap_or(0.0));
                let p = source_assets::kv::parse_f32(param).unwrap_or(0.0);
                let old = self.list[i].value;
                let new = match input {
                    "add" => old + p,
                    "subtract" => old - p,
                    "setvalue" | "setvaluenofire" => p,
                    "multiply" => old * p,
                    "divide" if p != 0.0 => old / p,
                    "sethitmax" => {
                        self.list[i].keyvalues.push(("max".into(), param.into()));
                        old
                    }
                    "sethitmin" => {
                        self.list[i].keyvalues.push(("min".into(), param.into()));
                        old
                    }
                    "getvalue" => {
                        self.fire(i, "ongetvalue", activator);
                        old
                    }
                    _ => old,
                };
                let new = if max > min { new.clamp(min, max) } else { new };
                self.list[i].value = new;
                if input != "setvaluenofire" && self.list[i].enabled && new != old {
                    self.fire(i, "outvalue", activator);
                    if max > min && new >= max {
                        self.fire(i, "onhitmax", activator);
                    }
                    if max > min && new <= min {
                        self.fire(i, "onhitmin", activator);
                    }
                    if new != old && old >= max && max > min {
                        self.fire(i, "onchangedfrommax", activator);
                    }
                    if new != old && old <= min && max > min {
                        self.fire(i, "onchangedfrommin", activator);
                    }
                }
                true
            }
            "logic_branch" => {
                let old = self.list[i].value != 0.0;
                let p = source_assets::kv::parse_f32(param).unwrap_or(0.0) != 0.0;
                let (new, test) = match input {
                    "setvalue" => (p, false),
                    "setvaluetest" => (p, true),
                    "toggle" => (!old, false),
                    "toggletest" => (!old, true),
                    "test" => (old, true),
                    _ => (old, false),
                };
                self.list[i].value = new as u8 as f32;
                if test {
                    self.fire(i, if new { "ontrue" } else { "onfalse" }, activator);
                }
                if new != old {
                    self.branch_changed(i, activator);
                }
                true
            }
            "logic_branch_listener" => {
                if input == "test" {
                    self.test_listener(i, activator);
                }
                true
            }
            "logic_compare" => {
                let p = source_assets::kv::parse_f32(param).unwrap_or(0.0);
                let cmp = |e: &GameEnt| e.get_f32("comparevalue").unwrap_or(0.0);
                match input {
                    "setvalue" => self.list[i].value = p,
                    "setvaluecompare" => {
                        self.list[i].value = p;
                        self.compare(i, cmp(&self.list[i].clone()), activator);
                    }
                    "setcomparevalue" => {
                        self.list[i].keyvalues.retain(|(k, _)| k != "comparevalue");
                        self.list[i].keyvalues.push(("comparevalue".into(), param.into()));
                    }
                    "compare" => self.compare(i, cmp(&self.list[i].clone()), activator),
                    _ => {}
                }
                true
            }
            "logic_case" => {
                let cases: Vec<(usize, String)> = (1..=16)
                    .filter_map(|n| self.list[i].get(&format!("case{n:02}")).map(|v| (n, v.to_string())))
                    .collect();
                match input {
                    "invalue" => {
                        if let Some((n, _)) = cases.iter().find(|(_, v)| v.eq_ignore_ascii_case(param)) {
                            self.fire(i, &format!("oncase{n:02}"), activator);
                        } else {
                            self.fire(i, "ondefault", activator);
                        }
                    }
                    "pickrandom" | "pickrandomshuffle" => {
                        let with_outputs: Vec<usize> = (1..=16)
                            .filter(|n| self.list[i].outputs.iter().any(|o| o.event.eq_ignore_ascii_case(&format!("oncase{n:02}"))))
                            .collect();
                        if !with_outputs.is_empty() {
                            let n = with_outputs[self.rand() as usize % with_outputs.len()];
                            self.fire(i, &format!("oncase{n:02}"), activator);
                        }
                    }
                    _ => {}
                }
                true
            }
            "logic_timer" => {
                match input {
                    "refiretime" => {
                        self.list[i].keyvalues.retain(|(k, _)| k != "refiretime");
                        self.list[i].keyvalues.push(("refiretime".into(), param.into()));
                    }
                    "firetimer" => self.fire(i, "ontimer", activator),
                    // Counting starts when the timer is (re)started.
                    "resettimer" | "enable" | "toggle" => self.list[i].aux = self.now,
                    _ => {}
                }
                true
            }
            "func_button" | "func_rot_button" | "prop_button" | "prop_under_button" => {
                match input {
                    "lock" => self.list[i].value = 0.0,
                    "unlock" => self.list[i].value = 1.0,
                    "press" | "pressin" => self.fire(i, "onpressed", activator),
                    "pressout" => self.fire(i, "onout", activator),
                    _ => {}
                }
                true
            }
            "logic_auto" | "logic_script" | "logic_choreographed_scene_disabled" | "info_target" | "info_landmark_entry"
            | "info_landmark_exit" | "env_texturetoggle" | "logic_autosave" | "info_overlay_accessor" => true,
            _ => {
                let _ = caller;
                false
            }
        }
    }

    fn compare(&mut self, i: usize, against: f32, activator: Option<usize>) {
        let v = self.list[i].value;
        if v == against {
            self.fire(i, "onequalto", activator);
        } else {
            self.fire(i, "onnotequalto", activator);
            self.fire(i, if v < against { "onlessthan" } else { "ongreaterthan" }, activator);
        }
    }

    /// A branch changed: every listener naming it re-tests.
    fn branch_changed(&mut self, branch: usize, activator: Option<usize>) {
        let name = self.list[branch].name.clone();
        let listeners: Vec<usize> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.class == "logic_branch_listener"
                    && (1..=16).any(|n| e.get(&format!("branch{n:02}")).is_some_and(|b| b.eq_ignore_ascii_case(&name)))
            })
            .map(|(i, _)| i)
            .collect();
        for l in listeners {
            self.test_listener(l, activator);
        }
    }

    fn test_listener(&mut self, l: usize, activator: Option<usize>) {
        let names: Vec<String> = (1..=16).filter_map(|n| self.list[l].get(&format!("branch{n:02}")).map(str::to_string)).collect();
        let values: Vec<bool> = names
            .iter()
            .flat_map(|n| self.find(n, l, activator))
            .map(|b| self.list[b].value != 0.0)
            .collect();
        if values.is_empty() {
            return;
        }
        let event = if values.iter().all(|v| *v) {
            "onalltrue"
        } else if values.iter().all(|v| !*v) {
            "onallfalse"
        } else {
            "onmixed"
        };
        self.fire(l, event, activator);
    }

    /// `AddOutput "OnTrigger target:input:param:delay:times"` (or a keyvalue).
    fn add_output(&mut self, i: usize, param: &str) {
        let mut parts = param.splitn(2, ' ');
        let (Some(key), Some(rest)) = (parts.next(), parts.next()) else { return };
        let key = key.to_ascii_lowercase();
        let f: Vec<&str> = rest.split(':').collect();
        if key.starts_with("on") && f.len() >= 2 {
            self.list[i].outputs.push(Output {
                event: key,
                target: f[0].into(),
                input: f[1].into(),
                param: f.get(2).copied().unwrap_or("").into(),
                delay: f.get(3).and_then(|d| d.parse().ok()).unwrap_or(0.0),
                times: f.get(4).and_then(|d| d.parse().ok()).unwrap_or(-1),
            });
        } else {
            self.list[i].keyvalues.retain(|(k, _)| *k != key);
            self.list[i].keyvalues.push((key, rest.into()));
        }
    }
}

/// `"target\x1binput\x1bparam\x1bdelay\x1btimes"` (Portal 2) or the comma form.
fn parse_output(key: &str, value: &str) -> Option<Output> {
    if !key.starts_with("on") && !key.starts_with("out") && !key.starts_with("pressed") && !key.starts_with("unpressed") {
        return None;
    }
    let fields: Vec<&str> = if value.contains('\u{1b}') { value.split('\u{1b}').collect() } else { value.split(',').collect() };
    if fields.len() < 4 {
        return None;
    }
    Some(Output {
        event: key.to_string(),
        target: fields[0].trim().to_string(),
        input: fields[1].trim().to_string(),
        param: fields[2].to_string(),
        delay: fields[3].trim().parse().unwrap_or(0.0),
        times: fields.get(4).and_then(|t| t.trim().parse().ok()).unwrap_or(-1),
    })
}

/// Fires each logic_auto once the map is up, runs timers, then delivers due
/// inputs (in time order, including ones queued while delivering).
pub fn run_io(
    time: Res<Time>,
    ents: Option<ResMut<Ents>>,
    mut fired: MessageReader<FireOutput>,
    mut out: MessageWriter<EntInput>,
) {
    let Some(mut ents) = ents else {
        fired.clear();
        return;
    };
    ents.now += time.delta_secs();
    if !ents.started {
        ents.started = true;
        let autos: Vec<usize> = ents.list.iter().enumerate().filter(|(_, e)| e.class == "logic_auto").map(|(i, _)| i).collect();
        for a in autos {
            ents.fire(a, "onmapspawn", None);
            ents.fire(a, "onnewgame", None);
        }
        // Timers start counting from spawn.
        let now = ents.now;
        for e in ents.list.iter_mut().filter(|e| e.class == "logic_timer") {
            e.aux = now;
        }
    }
    for f in fired.read() {
        ents.fire(f.ent, &f.event, f.activator);
    }
    // Timers.
    let now = ents.now;
    let timers: Vec<usize> = ents
        .list
        .iter()
        .enumerate()
        .filter(|(_, e)| e.class == "logic_timer" && e.enabled && !e.killed)
        .map(|(i, _)| i)
        .collect();
    for t in timers {
        let e = &ents.list[t];
        let random = e.get("userandomtime").is_some_and(|v| v.trim() == "1");
        let period = if random {
            let lo = e.get_f32("lowerrandombound").unwrap_or(1.0);
            let hi = e.get_f32("upperrandombound").unwrap_or(lo);
            lo + (hi - lo) * 0.5
        } else {
            e.get_f32("refiretime").unwrap_or(0.0)
        };
        if period > 0.0 && now - e.aux >= period {
            ents.list[t].aux = now;
            ents.fire(t, "ontimer", None);
        }
    }
    // Deliver.
    for _ in 0..64 {
        let now = ents.now;
        let mut due: Vec<Pending> = Vec::new();
        ents.queue.retain(|p| {
            if p.at <= now {
                due.push(p.clone());
                false
            } else {
                true
            }
        });
        if due.is_empty() {
            break;
        }
        due.sort_by(|a, b| a.at.total_cmp(&b.at));
        for p in due {
            let targets = ents.find(&p.target, p.caller, p.activator);
            if targets.is_empty() {
                trace!("io: no target {:?} for {}", p.target, p.input);
            }
            for t in targets {
                if ents.list[t].killed && p.input != "kill" {
                    continue;
                }
                if !ents.apply(t, &p.input, &p.param, p.caller, p.activator) {
                    out.write(EntInput { target: t, input: p.input.clone(), param: p.param.clone(), activator: p.activator });
                }
            }
        }
    }
}
