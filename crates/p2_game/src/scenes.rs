//! Choreographed scenes (`logic_choreographed_scene`): the game's `.vcd`
//! files, read as text from the install. A scene plays its `speak` events
//! (voice lines) at their times, fires its `OnTrigger<N>` outputs from
//! `firetrigger` events, and fires `OnCompletion` when its last event ends,
//! which is what moves most scripted sequences along.
use bevy::prelude::*;
use p2_assets::GameFsRes;

use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents, FireOutput};
use crate::level::LevelLoaded;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playing>()
            .add_systems(Update, (scene_inputs, run_scenes).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneEventKind {
    Speak,
    FireTrigger,
    Other,
}

#[derive(Clone, Debug)]
pub struct SceneEvent {
    pub kind: SceneEventKind,
    pub start: f32,
    pub end: f32,
    pub param: String,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub events: Vec<SceneEvent>,
    /// When the scene is over (last event end, or a `stoppoint`/`stopplayback`).
    pub length: f32,
    /// Its actors' entity names (`actor "@glados"`), lowercase.
    pub actors: Vec<String>,
}

/// Parses a text `.vcd`: events with their `time start end` and `param`.
pub fn parse_vcd(text: &str) -> Scene {
    let mut scene = Scene::default();
    let mut cur: Option<SceneEvent> = None;
    let mut depth_at_event = 0usize;
    let mut depth = 0usize;
    let mut stop: Option<f32> = None;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("//") {
            continue;
        }
        if l == "{" {
            depth += 1;
            continue;
        }
        if l == "}" {
            if let Some(ev) = cur.take() {
                if depth == depth_at_event + 1 {
                    scene.events.push(ev);
                } else {
                    cur = Some(ev);
                }
            }
            depth = depth.saturating_sub(1);
            continue;
        }
        let mut words = l.split_whitespace();
        match words.next() {
            Some("actor") => {
                let name = l["actor".len()..].trim().trim_matches('"').to_ascii_lowercase();
                if !name.is_empty() && !scene.actors.contains(&name) {
                    scene.actors.push(name);
                }
            }
            Some("event") if cur.is_none() => {
                let kind = match words.next() {
                    Some("speak") => SceneEventKind::Speak,
                    Some("firetrigger") => SceneEventKind::FireTrigger,
                    Some("stoppoint") | Some("stopplayback") => {
                        // Recorded below once its time is read.
                        SceneEventKind::Other
                    }
                    _ => SceneEventKind::Other,
                };
                let is_stop = l.contains("stoppoint") || l.contains("stopplayback");
                cur = Some(SceneEvent { kind, start: 0.0, end: if is_stop { -1.0 } else { 0.0 }, param: String::new() });
                depth_at_event = depth;
            }
            Some("time") => {
                if let Some(ev) = &mut cur {
                    let a: f32 = words.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let b: f32 = words.next().and_then(|v| v.parse().ok()).unwrap_or(a);
                    let is_stop = ev.end < 0.0;
                    ev.start = a;
                    ev.end = if b < a { a } else { b };
                    if is_stop {
                        stop = Some(stop.map_or(a, |s: f32| s.max(a)));
                    }
                }
            }
            Some("param") => {
                if let Some(ev) = &mut cur {
                    if ev.param.is_empty() {
                        ev.param = l.split('"').nth(1).unwrap_or("").to_string();
                    }
                }
            }
            _ => {}
        }
    }
    let last = scene.events.iter().map(|e| e.end).fold(0.0f32, f32::max);
    // A stop point ends the scene; otherwise its last event does.
    scene.length = stop.filter(|s| *s > 0.0).unwrap_or(last);
    scene
}

struct Running {
    ent: usize,
    scene: Scene,
    started: f32,
    done: Vec<bool>,
    paused_at: Option<f32>,
}

#[derive(Resource, Default)]
pub struct Playing(Vec<Running>);

impl Playing {
    /// Is scene entity `ent` playing (not paused)?
    pub fn is_playing(&self, ent: usize) -> bool {
        self.0.iter().any(|r| r.ent == ent)
    }
    /// The scene entity actor `name` is in now (the latest started).
    pub fn current_for(&self, name: &str) -> Option<usize> {
        self.0.iter().filter(|r| r.paused_at.is_none() && r.scene.actors.iter().any(|a| a == name)).max_by(|a, b| a.started.total_cmp(&b.started)).map(|r| r.ent)
    }
}

fn scene_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut playing: ResMut<Playing>,
    mut fire: MessageWriter<FireOutput>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        playing.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let now = ents.now;
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "logic_choreographed_scene" && e.class != "scripted_scene" {
            continue;
        }
        match msg.input.as_str() {
            "start" => {
                let path = e.get("scenefile").unwrap_or("").replace('\\', "/").to_ascii_lowercase();
                let Some(text) = game.read(&path).map(|b| String::from_utf8_lossy(&b).into_owned()) else {
                    warn!("portal2: scene {path} not found");
                    // Still let the level move on.
                    fire.write(FireOutput { ent: msg.target, event: "OnStart".into(), activator: msg.activator });
                    fire.write(FireOutput { ent: msg.target, event: "OnCompletion".into(), activator: msg.activator });
                    continue;
                };
                let scene = parse_vcd(&text);
                debug!("portal2: scene {path}: {:.2} s, {} events", scene.length, scene.events.len());
                playing.0.retain(|r| r.ent != msg.target);
                let n = scene.events.len();
                playing.0.push(Running { ent: msg.target, scene, started: now, done: vec![false; n], paused_at: None });
                fire.write(FireOutput { ent: msg.target, event: "OnStart".into(), activator: msg.activator });
            }
            "cancel" | "kill" => {
                if playing.0.iter().any(|r| r.ent == msg.target) {
                    fire.write(FireOutput { ent: msg.target, event: "OnCanceled".into(), activator: msg.activator });
                }
                playing.0.retain(|r| r.ent != msg.target);
            }
            "pause" => {
                for r in playing.0.iter_mut().filter(|r| r.ent == msg.target) {
                    r.paused_at.get_or_insert(now);
                }
            }
            "resume" => {
                for r in playing.0.iter_mut().filter(|r| r.ent == msg.target) {
                    if let Some(p) = r.paused_at.take() {
                        r.started += now - p;
                    }
                }
            }
            _ => {}
        }
    }
}

fn run_scenes(
    ents: Option<Res<Ents>>,
    mut playing: ResMut<Playing>,
    mut fire: MessageWriter<FireOutput>,
    mut play: MessageWriter<PlayGameSound>,
    mut caption: MessageWriter<crate::captions::ShowCaption>,
) {
    let Some(ents) = ents else { return };
    let now = ents.now;
    let mut finished = Vec::new();
    for (ri, r) in playing.0.iter_mut().enumerate() {
        if r.paused_at.is_some() {
            continue;
        }
        let t = now - r.started;
        for (i, ev) in r.scene.events.iter().enumerate() {
            if r.done[i] || ev.start > t {
                continue;
            }
            r.done[i] = true;
            match ev.kind {
                SceneEventKind::Speak => {
                    play.write(PlayGameSound { sound: ev.param.clone(), at: None, looped: false, owner: None, music: false, volume: 1.0 });
                    caption.write(crate::captions::ShowCaption { sound: ev.param.clone(), seconds: ev.end - ev.start });
                }
                SceneEventKind::FireTrigger => {
                    let n = ev.param.trim().parse::<u32>().unwrap_or(1);
                    fire.write(FireOutput { ent: r.ent, event: format!("OnTrigger{n}"), activator: None });
                }
                SceneEventKind::Other => {}
            }
        }
        if t >= r.scene.length {
            finished.push(ri);
            fire.write(FireOutput { ent: r.ent, event: "OnCompletion".into(), activator: None });
        }
    }
    for ri in finished.into_iter().rev() {
        playing.0.remove(ri);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_speak_and_trigger_events() {
        let vcd = r#"// Choreo version 1
event stoppoint "end"
{
  time 4.000000 -1.000000
  param "noaction"
}
actor "@sphere"
{
  channel "audio"
  {
    event speak "sphere03.a"
    {
      time 0.500000 1.75000
      param "sphere03.a"
      fixedlength
    }
  }
  channel "Trigger"
  {
    event firetrigger "go"
    {
      time 2.000000 -1.000000
      param "3"
    }
  }
}
"#;
        let s = parse_vcd(vcd);
        let speak: Vec<_> = s.events.iter().filter(|e| e.kind == SceneEventKind::Speak).collect();
        assert_eq!(speak.len(), 1);
        assert_eq!(speak[0].param, "sphere03.a");
        assert!((speak[0].start - 0.5).abs() < 1e-6);
        let trig: Vec<_> = s.events.iter().filter(|e| e.kind == SceneEventKind::FireTrigger).collect();
        assert_eq!(trig[0].param, "3");
        assert!((s.length - 4.0).abs() < 1e-6, "length {}", s.length);
    }
}
