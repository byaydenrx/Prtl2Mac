//! Soundscapes: the room ambience (insects, water drips, machine hum) the
//! maps pick with `env_soundscape` speakers from the install's
//! `scripts/soundscapes_*.txt`. Like Source, the active soundscape is the
//! closest enabled speaker whose radius holds the listener and that the
//! listener can see; it stays on after the listener walks out of range
//! until another one takes over, crossfading over its `fadetime`.
use std::collections::HashMap;

use avian3d::prelude::{ColliderOf, RigidBody, Sensor, SpatialQuery, SpatialQueryFilter};
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use p2_assets::GameFsRes;
use p2_core::MainCamera;
use source_assets::kv::{self, Kv, KvValue};

use crate::audio::{gain, parse_soundlevel, wave_path, GameSound, SoundScripts};
use crate::io::Ents;
use crate::level::{LevelLoaded, LevelOwned};
use crate::settings::Settings;

/// Metres per Source unit.
const U: f32 = 0.0254;

pub struct SoundscapesPlugin;

impl Plugin for SoundscapesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundscapeDefs>()
            .init_resource::<SoundscapeState>()
            .add_systems(Startup, load_soundscapes)
            .add_systems(Update, (reset_on_load, run_soundscapes).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

/// Every soundscape definition by lowercased name.
#[derive(Resource, Default)]
struct SoundscapeDefs(HashMap<String, Kv>);

fn load_soundscapes(game: Res<GameFsRes>, mut defs: ResMut<SoundscapeDefs>) {
    let manifest = game.read("scripts/soundscapes_manifest.txt").map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let parsed = kv::parse(&manifest);
    let files: Vec<String> = parsed.entries.iter().flat_map(|(_, v)| match v {
        KvValue::Block(b) => b.entries.iter().filter(|(k, _)| k.eq_ignore_ascii_case("file")).filter_map(|(_, v)| match v {
            KvValue::Str(f) => Some(f.clone()),
            _ => None,
        }).collect(),
        KvValue::Str(_) => Vec::new(),
    }).collect();
    for f in &files {
        let Some(text) = game.read(f).map(|b| String::from_utf8_lossy(&b).into_owned()) else { continue };
        add_defs(&mut defs.0, &text);
    }
    info!("portal2: {} soundscapes from {} files", defs.0.len(), files.len());
}

fn add_defs(defs: &mut HashMap<String, Kv>, text: &str) {
    for (name, v) in kv::parse(text).entries {
        if let KvValue::Block(b) = v {
            defs.insert(name.to_ascii_lowercase(), b);
        }
    }
}

/// Where a looping layer plays.
#[derive(Clone, Debug, PartialEq)]
enum Place {
    /// Everywhere (no falloff).
    Ambient,
    /// A fixed Source point.
    Origin(Vec3),
    /// The speaker's `positionN` entity.
    Index(usize),
}

#[derive(Clone, Debug, PartialEq)]
enum Layer {
    Looping { wave: String, volume: f32, pitch: f32, soundlevel: f32, place: Place },
    Random { waves: Vec<String>, volume: (f32, f32), pitch: (f32, f32), time: (f32, f32), soundlevel: f32 },
}

/// `"0.3,0.45"` as a range (one number: both ends).
fn range(s: Option<&str>, default: f32) -> (f32, f32) {
    let parts: Vec<f32> = s.unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect();
    match parts.as_slice() {
        [a, b] => (*a, *b),
        [a] => (*a, *a),
        _ => (default, default),
    }
}

fn waves(b: &Kv) -> Vec<String> {
    let mut out: Vec<String> = b.get("wave").map(|w| vec![w.to_string()]).unwrap_or_default();
    if let Some(r) = b.block("rndwave") {
        out.extend(r.entries.iter().filter_map(|(k, v)| match v {
            KvValue::Str(w) if k == "wave" => Some(w.clone()),
            _ => None,
        }));
    }
    out
}

/// A soundscape's sounds, with nested `playsoundscape`s folded in at their
/// volume.
fn expand(defs: &HashMap<String, Kv>, name: &str, scale: f32, depth: u32, out: &mut Vec<Layer>) {
    let Some(def) = defs.get(&name.to_ascii_lowercase()) else { return };
    if depth > 8 {
        return;
    }
    for (key, v) in &def.entries {
        let KvValue::Block(b) = v else { continue };
        match key.to_ascii_lowercase().as_str() {
            "playlooping" => {
                let Some(wave) = waves(b).into_iter().next() else { continue };
                let place = if let Some(o) = b.get("origin") {
                    let p: Vec<f32> = o.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if p.len() == 3 { Place::Origin(Vec3::new(p[0], p[1], p[2])) } else { Place::Ambient }
                } else if let Some(i) = b.get("position").and_then(|p| p.trim().parse().ok()) {
                    Place::Index(i)
                } else {
                    Place::Ambient
                };
                let soundlevel = if b.get("attenuation").is_some_and(|a| a.trim().parse::<f32>().unwrap_or(1.0) == 0.0) {
                    0.0
                } else {
                    b.get("soundlevel").map(parse_soundlevel).unwrap_or(75.0)
                };
                let volume = range(b.get("volume"), 1.0);
                let pitch = range(b.get("pitch"), 100.0);
                out.push(Layer::Looping { wave, volume: (volume.0 + volume.1) * 0.5 * scale, pitch: (pitch.0 + pitch.1) * 0.5, soundlevel, place });
            }
            "playrandom" => {
                let w = waves(b);
                if w.is_empty() {
                    continue;
                }
                let v = range(b.get("volume"), 1.0);
                out.push(Layer::Random {
                    waves: w,
                    volume: (v.0 * scale, v.1 * scale),
                    pitch: range(b.get("pitch"), 100.0),
                    time: range(b.get("time"), 5.0),
                    soundlevel: b.get("soundlevel").map(parse_soundlevel).unwrap_or(75.0),
                });
            }
            "playsoundscape" => {
                if let Some(n) = b.get("name") {
                    let v = b.get("volume").and_then(|v| v.trim().parse().ok()).unwrap_or(1.0);
                    expand(defs, n, scale * v, depth + 1, out);
                }
            }
            _ => {}
        }
    }
}

#[derive(Resource, Default)]
struct SoundscapeState {
    /// The active speaker (an `Ents` index).
    active: Option<usize>,
    /// Bumped on every switch: older sounds fade out.
    generation: u32,
    fade: f32,
    layers: Vec<Layer>,
    /// Seconds until each random layer plays next.
    timers: Vec<f32>,
    rng: u32,
    /// The menu's background map: no soundscapes (the menus stay as they are).
    menu: bool,
}

impl SoundscapeState {
    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    fn between(&mut self, (a, b): (f32, f32)) -> f32 {
        a + (b - a) * self.rand()
    }
}

/// A soundscape sound: its volume before the crossfade.
#[derive(Component)]
struct SoundscapeSound {
    generation: u32,
    volume: f32,
    fade: f32,
}

fn reset_on_load(mut loaded: MessageReader<LevelLoaded>, mut state: ResMut<SoundscapeState>) {
    if let Some(ev) = loaded.read().last() {
        let rng = state.rng;
        *state = SoundscapeState { rng: rng ^ 0x9e37_79b9, menu: ev.menu, ..default() };
    }
}

/// The soundscape a speaker plays (`env_soundscape_proxy` borrows its main
/// speaker's).
fn speaker_soundscape(ents: &Ents, i: usize) -> Option<String> {
    let e = &ents.list[i];
    match e.class.as_str() {
        "env_soundscape" => e.get("soundscape").map(str::to_string),
        "env_soundscape_proxy" => {
            let main = e.get("mainsoundscapename")?;
            let j = ents.find(main, i, None).into_iter().next()?;
            ents.list[j].get("soundscape").map(str::to_string)
        }
        _ => None,
    }
}

/// The speaker to switch to: the closest candidate, unless the active one
/// is still a candidate and at least as close.
fn pick(candidates: &[(usize, f32)], active: Option<usize>) -> Option<usize> {
    let best = candidates.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))?;
    match active.and_then(|a| candidates.iter().find(|c| c.0 == a)) {
        Some(cur) if cur.1 <= best.1 => None,
        _ if Some(best.0) == active => None,
        _ => Some(best.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_soundscapes(
    mut commands: Commands,
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    defs: Res<SoundscapeDefs>,
    mut state: ResMut<SoundscapeState>,
    mut scripts: ResMut<SoundScripts>,
    game: Res<GameFsRes>,
    mut sources: ResMut<Assets<AudioSource>>,
    settings: Res<Settings>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    spatial: SpatialQuery,
    bodies: Query<&RigidBody>,
    body_of: Query<&ColliderOf>,
    sensors: Query<(), With<Sensor>>,
    mut sounds: Query<(Entity, &mut SoundscapeSound, &mut GameSound)>,
) {
    let Some(ents) = ents else { return };
    let Some(ear) = listener.iter().next().map(|t| t.translation()) else { return };
    let dt = time.delta_secs();
    if dt <= 0.0 || state.menu {
        return;
    }

    // Choose the speaker.
    let world = |c: Entity| {
        let body = body_of.get(c).map_or(c, |b| b.body);
        matches!(bodies.get(body), Ok(RigidBody::Static)) && !sensors.contains(c)
    };
    let mut candidates = Vec::new();
    for (i, e) in ents.list.iter().enumerate() {
        if !matches!(e.class.as_str(), "env_soundscape" | "env_soundscape_proxy") || !e.enabled || e.killed {
            continue;
        }
        let at = e.transform.translation;
        let d = at.distance(ear);
        let radius = e.get_f32("radius").unwrap_or(128.0);
        if radius >= 0.0 && d > radius * U {
            continue;
        }
        let seen = match Dir3::new(at - ear) {
            Ok(dir) => spatial.cast_ray_predicate(ear, dir, d, true, &SpatialQueryFilter::default(), &world).is_none(),
            Err(_) => true,
        };
        if seen {
            candidates.push((i, d));
        }
    }
    if let Some(next) = pick(&candidates, state.active) {
        let name = speaker_soundscape(&ents, next).unwrap_or_default();
        let mut layers = Vec::new();
        expand(&defs.0, &name, 1.0, 0, &mut layers);
        let fade = defs.0.get(&name.to_ascii_lowercase()).and_then(|d| d.get("fadetime")).and_then(|f| f.trim().parse().ok()).unwrap_or(3.0f32);
        // Same sounds (another speaker with this soundscape): keep playing.
        let same = state.active.and_then(|a| speaker_soundscape(&ents, a)).is_some_and(|a| a.eq_ignore_ascii_case(&name));
        info!("portal2: soundscape {name} ({} sounds) from speaker {next}{}", layers.len(), if same { ", unchanged" } else { "" });
        state.active = Some(next);
        if !same {
            state.generation += 1;
            // The first soundscape of a map starts at once.
            state.fade = if state.generation == 1 { 0.0 } else { fade.max(0.0) };
            let mut timers = Vec::new();
            for l in &layers {
                let t = match l {
                    Layer::Random { time, .. } => state.between(*time),
                    Layer::Looping { .. } => 0.0,
                };
                timers.push(t);
            }
            state.timers = timers;
            let generation = state.generation;
            let fade_in = state.fade;
            for l in layers.clone() {
                let Layer::Looping { wave, volume, pitch, soundlevel, place } = l else { continue };
                let at = match place {
                    Place::Ambient => None,
                    Place::Origin(p) => Some(shared::source_to_world(p)),
                    Place::Index(n) => ents.list[next].get(&format!("position{n}")).and_then(|t| ents.find(t, next, None).into_iter().next()).map(|j| ents.list[j].transform.translation),
                };
                spawn(&mut commands, &mut scripts, &game, &mut sources, &settings, &wave, volume, pitch, if at.is_some() { soundlevel } else { 0.0 }, at, ear, true, generation, fade_in);
            }
            state.layers = layers;
        }
    }

    // Random one-shots.
    let generation = state.generation;
    for k in 0..state.layers.len() {
        let Layer::Random { waves, volume, pitch, time: every, soundlevel } = state.layers[k].clone() else { continue };
        state.timers[k] -= dt;
        if state.timers[k] > 0.0 {
            continue;
        }
        state.timers[k] = state.between(every).max(0.1);
        let pick = ((state.rand() * waves.len() as f32) as usize).min(waves.len() - 1);
        let vol = state.between(volume);
        let p = state.between(pitch);
        // "position random": somewhere around the listener.
        let dist = 1.5 + 3.0 * state.rand();
        let vol = vol * if soundlevel > 0.0 { gain(dist, soundlevel) } else { 1.0 };
        spawn(&mut commands, &mut scripts, &game, &mut sources, &settings, &waves[pick], vol, p, 0.0, None, ear, false, generation, 0.0);
    }

    // Crossfade.
    let fade = state.fade;
    for (e, mut s, mut g) in &mut sounds {
        let step = if fade > 0.0 { dt / fade } else { 1.0 };
        s.fade = if s.generation == generation { (s.fade + step).min(1.0) } else { s.fade - step };
        if s.fade <= 0.0 && s.generation != generation {
            commands.entity(e).despawn();
            continue;
        }
        g.base = s.volume * s.fade.max(0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn(
    commands: &mut Commands,
    scripts: &mut SoundScripts,
    game: &GameFsRes,
    sources: &mut Assets<AudioSource>,
    settings: &Settings,
    wave: &str,
    volume: f32,
    pitch: f32,
    soundlevel: f32,
    at: Option<Vec3>,
    ear: Vec3,
    looped: bool,
    generation: u32,
    fade_in: f32,
) {
    let path = wave_path(wave);
    let Some(handle) = scripts.clip(game, sources, &path) else {
        debug!("portal2: soundscape wave {path} missing");
        return;
    };
    let fade = if fade_in > 0.0 { 0.0 } else { 1.0 };
    let g = at.filter(|_| soundlevel > 0.0).map_or(1.0, |p| gain(p.distance(ear), soundlevel));
    let mix = settings.effects_volume * settings.master_volume;
    debug!("portal2: soundscape plays {path} volume {volume:.2} pitch {pitch:.0} looped {looped}");
    commands.spawn((
        Name::new(format!("soundscape {path}")),
        AudioPlayer(handle),
        PlaybackSettings {
            mode: if looped { PlaybackMode::Loop } else { PlaybackMode::Despawn },
            volume: Volume::Linear(volume * fade * mix * g),
            speed: (pitch / 100.0).clamp(0.25, 4.0),
            ..default()
        },
        GameSound { at, soundlevel, base: volume * fade, script: volume, owner: None, music: false, voice: None, ui: false },
        SoundscapeSound { generation, volume, fade },
        LevelOwned,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFS: &str = r#"
"Veg.Drips"
{
    "playrandom"
    {
        "volume" "0.3,0.45"
        "pitch" "100, 105"
        "time" "1.5, 4.5"
        "position" "random"
        "soundlevel" "SNDLVL65dB"
        "rndwave" { "wave" "a.wav" "wave" "b.wav" }
    }
}
Room.One
{
    "dsp" "1"
    "fadetime" "3.5"
    "playlooping" { "volume" "0.78" "wave" "hum_lp.wav" "pitch" "85" "attenuation" "0" }
    "playsoundscape" { "name" "Veg.Drips" "volume" "0.5" }
}
"#;

    #[test]
    fn nested_soundscapes_fold_in_at_their_volume() {
        let mut defs = HashMap::new();
        add_defs(&mut defs, DEFS);
        let mut layers = Vec::new();
        expand(&defs, "ROOM.one", 1.0, 0, &mut layers);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], Layer::Looping { wave: "hum_lp.wav".into(), volume: 0.78, pitch: 85.0, soundlevel: 0.0, place: Place::Ambient });
        let Layer::Random { waves, volume, time, soundlevel, .. } = &layers[1] else { panic!() };
        assert_eq!(waves.len(), 2);
        assert_eq!(*volume, (0.15, 0.225));
        assert_eq!(*time, (1.5, 4.5));
        assert_eq!(*soundlevel, 65.0);
    }

    #[test]
    fn closest_visible_speaker_takes_over() {
        assert_eq!(pick(&[], Some(3)), None);
        assert_eq!(pick(&[(1, 2.0), (2, 1.0)], None), Some(2));
        assert_eq!(pick(&[(1, 2.0), (2, 1.0)], Some(2)), None);
        assert_eq!(pick(&[(1, 2.0), (2, 1.0)], Some(1)), Some(2));
        // The active speaker went out of range: the one in range wins.
        assert_eq!(pick(&[(1, 2.0)], Some(5)), Some(1));
    }
}
