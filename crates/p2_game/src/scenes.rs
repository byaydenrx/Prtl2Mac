//! Choreographed scenes (`logic_choreographed_scene`): the game's `.vcd`
//! files, read as text from the install. A scene plays its `speak` events
//! (voice lines) at their times, fires its `OnTrigger<N>` outputs from
//! `firetrigger` events, plays its `sequence` / `gesture` events on their
//! actors (Wheatley's nods, glances, eye pops), and fires `OnCompletion` when
//! its last event ends, which is what moves most scripted sequences along.
use bevy::prelude::*;
use p2_assets::GameFsRes;

use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents, FireOutput};
use crate::level::LevelLoaded;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playing>()
            .add_systems(Update, (scene_inputs, run_scenes, speaking_lights).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneEventKind {
    Speak,
    FireTrigger,
    /// A `sequence` event: the actor plays that animation.
    Sequence,
    /// A `gesture` event: its sequence is layered over the actor's current
    /// animation for the event's time (`anim_props.rs` reads it).
    Gesture,
    /// A `lookat` event: the actor aims at `param` (`!player`, a name).
    LookAt,
    Other,
}

#[derive(Clone, Debug)]
pub struct SceneEvent {
    pub kind: SceneEventKind,
    pub start: f32,
    pub end: f32,
    pub param: String,
    /// The actor it belongs to (lowercase), if any.
    pub actor: Option<String>,
    /// `event_ramp` points: (seconds into the event, weight 0..1).
    pub ramp: Vec<(f32, f32)>,
}

impl SceneEvent {
    /// The event's weight `t` seconds into it: its ramp (implicitly 0 at the
    /// event's start and end unless a point is there), else full weight
    /// faded in and out over `fade` seconds.
    pub fn weight(&self, t: f32, fade: f32) -> f32 {
        let len = (self.end - self.start).max(0.0);
        if t < 0.0 || t > len {
            return 0.0;
        }
        if self.ramp.is_empty() {
            let f = fade.min(len * 0.5);
            if f <= 0.0 {
                return 1.0;
            }
            return (t / f).min((len - t) / f).clamp(0.0, 1.0);
        }
        let mut pts = self.ramp.clone();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        if pts[0].0 > 1e-3 {
            pts.insert(0, (0.0, 0.0));
        }
        if pts[pts.len() - 1].0 < len - 1e-3 {
            pts.push((len, 0.0));
        }
        if t <= pts[0].0 {
            return pts[0].1.clamp(0.0, 1.0);
        }
        for w in pts.windows(2) {
            let ((t0, v0), (t1, v1)) = (w[0], w[1]);
            if t <= t1 {
                let u = if t1 - t0 > 1e-6 { (t - t0) / (t1 - t0) } else { 1.0 };
                return (v0 + (v1 - v0) * u).clamp(0.0, 1.0);
            }
        }
        pts[pts.len() - 1].1.clamp(0.0, 1.0)
    }
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
    let mut actor: Option<(String, usize)> = None;
    // Depth of the open `event_ramp` block.
    let mut ramp_depth: Option<usize> = None;
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
            if ramp_depth == Some(depth) {
                ramp_depth = None;
            }
            if actor.as_ref().is_some_and(|(_, d)| *d == depth) {
                actor = None;
            }
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
        if ramp_depth == Some(depth) {
            if let (Some(ev), Some(t), Some(v)) = (&mut cur, words.next().and_then(|w| w.parse().ok()), words.next().and_then(|w| w.parse().ok())) {
                ev.ramp.push((t, v));
            }
            continue;
        }
        match words.next() {
            Some("event_ramp") if cur.is_some() => {
                // Its block opens on the next line.
                ramp_depth = Some(depth + 1);
            }
            Some("actor") => {
                let name = l["actor".len()..].trim().trim_matches('"').to_ascii_lowercase();
                if !name.is_empty() && !scene.actors.contains(&name) {
                    scene.actors.push(name.clone());
                }
                // Its block opens on the next line.
                actor = Some((name, depth + 1));
            }
            Some("event") if cur.is_none() => {
                let kind = match words.next() {
                    Some("speak") => SceneEventKind::Speak,
                    Some("firetrigger") => SceneEventKind::FireTrigger,
                    Some("sequence") => SceneEventKind::Sequence,
                    Some("gesture") => SceneEventKind::Gesture,
                    Some("lookat") => SceneEventKind::LookAt,
                    Some("stoppoint") | Some("stopplayback") => {
                        // Recorded below once its time is read.
                        SceneEventKind::Other
                    }
                    _ => SceneEventKind::Other,
                };
                let is_stop = l.contains("stoppoint") || l.contains("stopplayback");
                cur = Some(SceneEvent { kind, start: 0.0, end: if is_stop { -1.0 } else { 0.0 }, param: String::new(), actor: actor.as_ref().map(|a| a.0.clone()), ramp: Vec::new() });
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
    /// Events of the running scenes going on `now`: (scene entity, event,
    /// seconds into the event). A paused scene's events hold where they are.
    pub fn active(&self, now: f32) -> impl Iterator<Item = (usize, &SceneEvent, f32)> {
        self.0.iter().flat_map(move |r| {
            let t = r.paused_at.unwrap_or(now) - r.started;
            r.scene.events.iter().filter(move |e| e.start <= t && t <= e.end).map(move |e| (r.ent, e, t - e.start))
        })
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
    mut send: MessageWriter<EntInput>,
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
                SceneEventKind::Sequence => {
                    let Some(actor) = &ev.actor else { continue };
                    for t in actor_ents(&ents, r.ent, actor) {
                        send.write(EntInput { target: t, input: "scenesequence".into(), param: ev.param.clone(), activator: None });
                    }
                }
                // Played continuously by `anim_props.rs` from `Playing::active`.
                SceneEventKind::Gesture | SceneEventKind::LookAt | SceneEventKind::Other => {}
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

/// The entities a scene's actor (or a lookat target) names: `!target3` is
/// the scene's `target3` keyvalue, `!player` the player, else a name.
pub fn actor_ents(ents: &Ents, scene: usize, actor: &str) -> Vec<usize> {
    let name = match actor.strip_prefix("!target") {
        Some(n) => ents.list[scene].get(&format!("target{n}")).unwrap_or("").to_string(),
        None => actor.to_string(),
    };
    ents.find(&name, scene, None)
}

/// Actors' self-illuminated "mouths" (Wheatley's eye: the `LightedMouth`
/// proxy) flicker while one of their `speak` events plays, and stay fully
/// lit otherwise.
fn speaking_lights(ents: Option<Res<Ents>>, playing: Res<Playing>, mut speaking: ResMut<source_assets::SpeakingAmount>) {
    speaking.0.clear();
    let Some(ents) = ents else { return };
    let now = ents.now;
    for r in playing.0.iter().filter(|r| r.paused_at.is_none()) {
        let t = now - r.started;
        for ev in r.scene.events.iter().filter(|e| e.kind == SceneEventKind::Speak && e.start <= t && t < e.end) {
            let Some(actor) = &ev.actor else { continue };
            for i in actor_ents(&ents, r.ent, actor) {
                if let Some(e) = ents.list[i].entity {
                    // A voice's loudness comes and goes with its syllables.
                    let k = ((t * 13.0).sin() * (t * 5.3 + 1.0).sin()).abs();
                    speaking.0.insert(e, 0.55 + 0.45 * k);
                }
            }
        }
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

    #[test]
    fn reads_gestures_with_ramps_and_lookats() {
        let vcd = r#"actor "@sphere"
{
  channel "gestures"
  {
    event gesture "nod"
    {
      time 1.000000 3.000000
      param "sphere_gesture_nod"
      event_ramp
      {
        0.5000 1.0000
        1.5000 1.0000
      }
      absolutetags playback_time
      {
        "apex" 0.400000
      }
    }
    event sequence "pop"
    {
      time 0.000000 0.500000
      param "sphere_eye_pop"
    }
  }
  channel "look"
  {
    event lookat "player"
    {
      time 0.500000 4.000000
      param "!player"
    }
  }
}
"#;
        let s = parse_vcd(vcd);
        assert_eq!(s.events.len(), 3, "{:?}", s.events);
        let g = s.events.iter().find(|e| e.kind == SceneEventKind::Gesture).unwrap();
        assert_eq!(g.param, "sphere_gesture_nod");
        assert_eq!(g.actor.as_deref(), Some("@sphere"));
        assert_eq!(g.ramp, vec![(0.5, 1.0), (1.5, 1.0)]);
        // Ramp: up from 0 at the start, flat, down to 0 at the end.
        assert_eq!(g.weight(0.0, 0.2), 0.0);
        assert!((g.weight(0.25, 0.2) - 0.5).abs() < 1e-5);
        assert_eq!(g.weight(1.0, 0.2), 1.0);
        assert!((g.weight(1.75, 0.2) - 0.5).abs() < 1e-5);
        assert_eq!(g.weight(2.5, 0.2), 0.0);
        let q = s.events.iter().find(|e| e.kind == SceneEventKind::Sequence).unwrap();
        assert!(q.ramp.is_empty() && q.param == "sphere_eye_pop");
        let l = s.events.iter().find(|e| e.kind == SceneEventKind::LookAt).unwrap();
        assert_eq!((l.param.as_str(), l.start, l.end), ("!player", 0.5, 4.0));
        // No ramp: faded in and out.
        assert!((l.weight(0.1, 0.2) - 0.5).abs() < 1e-5);
        assert_eq!(l.weight(2.0, 0.2), 1.0);
        assert!((l.weight(3.4, 0.2) - 0.5).abs() < 1e-4);
        assert_eq!(s.length, 4.0);
    }
}
