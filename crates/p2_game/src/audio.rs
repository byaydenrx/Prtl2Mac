//! Level and menu audio: every soundscript in the game's manifest, the
//! map's `ambient_generic` speakers, and music. Waves are the install's own
//! (plain PCM, or the MP3 frames Valve stores in `.wav` voice files), decoded
//! by Bevy.
use std::collections::HashMap;

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use p2_assets::GameFsRes;
use p2_core::MainCamera;
use source_assets::kv::{self, KvValue};

use crate::io::{EntInput, Ents};
use crate::level::{LevelLoaded, LevelOwned};
use crate::settings::Settings;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundScripts>()
            .add_message::<PlayGameSound>()
            .add_systems(Startup, load_soundscripts)
            .add_systems(Update, (ambient_on_load, ambient_inputs, play_sounds, attenuate).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            .add_systems(Update, pause_audio);
    }
}

#[derive(Clone, Debug)]
struct Entry {
    waves: Vec<String>,
    volume: f32,
    soundlevel: f32,
}

#[derive(Resource, Default)]
pub struct SoundScripts {
    entries: HashMap<String, Entry>,
    clips: HashMap<String, Handle<AudioSource>>,
    counter: u32,
}

/// Play a soundscript entry (`World.ElevatorArrive`) or a wave path
/// (`music/sp_a1_intro1_b1.wav`).
#[derive(Message, Clone, Debug)]
pub struct PlayGameSound {
    pub sound: String,
    pub at: Option<Vec3>,
    pub looped: bool,
    /// The `ambient_generic` it belongs to (so it can be stopped).
    pub owner: Option<usize>,
    pub music: bool,
    pub volume: f32,
    /// The actor speaking it: Source plays speech on the actor's voice
    /// channel, so a new line from the same actor stops the one before
    /// (jumping or saying "apple" while Wheatley talks no longer overlaps).
    pub voice: Option<usize>,
}

impl PlayGameSound {
    pub fn music(sound: impl Into<String>) -> Self {
        Self { sound: sound.into(), at: None, looped: false, owner: None, music: true, volume: 1.0, voice: None }
    }
}

/// A playing sound: where it is (for falloff) and what it belongs to.
#[derive(Component, Clone, Copy, Debug)]
pub struct GameSound {
    pub at: Option<Vec3>,
    pub soundlevel: f32,
    /// Soundscript and caller volume, before the player's settings.
    pub base: f32,
    /// The soundscript's own volume: an `ambient_generic` `Volume` input
    /// rescales `base` from it.
    pub script: f32,
    pub owner: Option<usize>,
    pub music: bool,
    pub voice: Option<usize>,
}

fn load_soundscripts(game: Res<GameFsRes>, mut scripts: ResMut<SoundScripts>) {
    let manifest = game.read("scripts/game_sounds_manifest.txt").map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let mut files: Vec<String> = manifest
        .lines()
        .filter(|l| l.to_ascii_lowercase().contains("precache_file") || l.to_ascii_lowercase().contains("preload_file"))
        .filter_map(|l| l.split('"').nth(3).map(str::to_string))
        .collect();
    if files.is_empty() {
        files = game.0.list("scripts/game_sounds").into_iter().filter(|f| f.ends_with(".txt")).collect();
    }
    for f in &files {
        let Some(text) = game.read(f).map(|b| String::from_utf8_lossy(&b).into_owned()) else { continue };
        let parsed = kv::parse(&text);
        for (name, v) in &parsed.entries {
            let KvValue::Block(b) = v else { continue };
            let mut waves: Vec<String> = b.get("wave").map(|w| vec![w.to_string()]).unwrap_or_default();
            if let Some(r) = b.block("rndwave") {
                for (k, v) in &r.entries {
                    if let (true, KvValue::Str(w)) = (k == "wave", v) {
                        waves.push(w.clone());
                    }
                }
            }
            if waves.is_empty() {
                continue;
            }
            let volume = b.get("volume").map(parse_volume).unwrap_or(1.0);
            let soundlevel = b.get("soundlevel").map(parse_soundlevel).unwrap_or(75.0);
            scripts.entries.insert(name.clone(), Entry { waves, volume, soundlevel });
        }
    }
    info!("portal2: {} soundscript entries from {} files", scripts.entries.len(), files.len());
}

fn parse_volume(s: &str) -> f32 {
    match s.trim().to_ascii_uppercase().as_str() {
        "VOL_NORM" => 1.0,
        v => {
            let parts: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            match parts.as_slice() {
                [a, b] => (a + b) * 0.5,
                [a] => *a,
                _ => 1.0,
            }
        }
    }
}

fn parse_soundlevel(s: &str) -> f32 {
    let s = s.trim().to_ascii_uppercase();
    if let Some(db) = s.strip_prefix("SNDLVL_") {
        if db == "NONE" {
            return 0.0;
        }
        if db == "NORM" {
            return 75.0;
        }
        return db.trim_end_matches("DB").parse().unwrap_or(75.0);
    }
    s.split(',').next().and_then(|x| x.trim().parse().ok()).unwrap_or(75.0)
}

/// Strip soundscript prefix characters (`)` spatial, `*` stream, `#` music...).
fn wave_path(w: &str) -> String {
    let w = w.trim_start_matches(|c: char| "*#@<>^)}$!?&~`+%".contains(c)).replace('\\', "/");
    if w.starts_with("sound/") {
        w
    } else {
        format!("sound/{w}")
    }
}

/// Source soundlevel falloff (same curve as the portal sounds use).
fn gain(distance_m: f32, soundlevel: f32) -> f32 {
    if soundlevel <= 0.0 {
        return 1.0;
    }
    p2_fx::gain(distance_m, soundlevel)
}

#[allow(clippy::too_many_arguments)]
fn play_sounds(
    mut commands: Commands,
    mut requests: MessageReader<PlayGameSound>,
    mut scripts: ResMut<SoundScripts>,
    game: Res<GameFsRes>,
    mut sources: ResMut<Assets<AudioSource>>,
    settings: Res<Settings>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut missing: Local<std::collections::HashSet<String>>,
    voices: Query<(Entity, &GameSound)>,
) {
    let ear = listener.iter().next().map(|t| t.translation());
    for req in requests.read() {
        let (wave, volume, soundlevel) = match scripts.entries.get(&req.sound.to_ascii_lowercase()).cloned() {
            Some(e) => {
                scripts.counter = scripts.counter.wrapping_add(1);
                let pick = (scripts.counter.wrapping_mul(2_654_435_761) >> 7) as usize % e.waves.len();
                (wave_path(&e.waves[pick]), e.volume, e.soundlevel)
            }
            None if req.sound.contains('.') && (req.sound.contains('/') || req.sound.ends_with(".wav") || req.sound.ends_with(".mp3")) => {
                (wave_path(&req.sound), 1.0, 75.0)
            }
            None => {
                if missing.insert(req.sound.clone()) {
                    debug!("portal2: sound {} not found", req.sound);
                }
                continue;
            }
        };
        let handle = match scripts.clips.get(&wave) {
            Some(h) => h.clone(),
            None => {
                let Some(bytes) = game.read(&wave) else {
                    if missing.insert(wave.clone()) {
                        debug!("portal2: wave {wave} missing");
                    }
                    continue;
                };
                let h = sources.add(AudioSource { bytes: bytes.into() });
                scripts.clips.insert(wave.clone(), h.clone());
                h
            }
        };
        if let Some(v) = req.voice {
            for (e, s) in &voices {
                if s.voice == Some(v) {
                    debug!("portal2: {} cuts off the actor's previous line", req.sound);
                    commands.entity(e).despawn();
                }
            }
        }
        let base = volume * req.volume;
        let mix = if req.music { settings.music_volume } else { settings.effects_volume } * settings.master_volume;
        let level = if req.music || req.at.is_none() { 0.0 } else { soundlevel };
        let g = match (req.at, ear) {
            (Some(p), Some(e)) if level > 0.0 => gain(p.distance(e), level),
            _ => 1.0,
        };
        commands.spawn((
            Name::new(format!("sound {wave}")),
            AudioPlayer(handle),
            PlaybackSettings {
                mode: if req.looped { PlaybackMode::Loop } else { PlaybackMode::Despawn },
                volume: Volume::Linear(base * mix * g),
                ..default()
            },
            GameSound { at: req.at, soundlevel: level, base, script: volume, owner: req.owner, music: req.music, voice: req.voice },
            LevelOwned,
        ));
    }
}

/// Keep positional sounds at the right loudness as the listener moves.
fn attenuate(
    listener: Query<&GlobalTransform, With<MainCamera>>,
    settings: Res<Settings>,
    mut sounds: Query<(&GameSound, &mut AudioSink)>,
) {
    let ear = listener.iter().next().map(|t| t.translation());
    for (s, mut sink) in &mut sounds {
        let g = match (s.at, ear) {
            (Some(p), Some(e)) if s.soundlevel > 0.0 => gain(p.distance(e), s.soundlevel),
            _ => 1.0,
        };
        let mix = if s.music { settings.music_volume } else { settings.effects_volume } * settings.master_volume;
        sink.set_volume(Volume::Linear(s.base * mix * g));
    }
}

/// `ambient_generic` keyvalues: `message` (sound), spawnflags 1 everywhere,
/// 16 start silent, 32 not looped.
fn ambient_request(ents: &Ents, i: usize) -> Option<PlayGameSound> {
    let e = &ents.list[i];
    let sound = e.get("message")?.to_string();
    let everywhere = e.spawnflags & 1 != 0;
    // Music speakers are flagged "play everywhere" and live under music/.
    let music = everywhere && sound.to_ascii_lowercase().contains("music");
    Some(PlayGameSound {
        sound,
        at: (!everywhere).then_some(e.transform.translation),
        looped: e.spawnflags & 32 == 0 && looks_looped(e.get("message").unwrap_or("")),
        owner: Some(i),
        music,
        volume: e.get_f32("health").map(|h| (h / 10.0).clamp(0.0, 1.0)).unwrap_or(1.0),
        voice: None,
    })
}

/// Source loops a wave when its file has a cue/loop point; without reading
/// every file up front, treat ambience loops by name.
fn looks_looped(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("loop") || n.contains("_lp") || n.contains("amb") || n.contains("hum")
}

fn ambient_on_load(mut loaded: MessageReader<LevelLoaded>, ents: Option<Res<Ents>>, mut play: MessageWriter<PlayGameSound>) {
    let Some(ev) = loaded.read().last().cloned() else { return };
    if ev.menu {
        return;
    }
    let Some(ents) = ents else { return };
    for (i, e) in ents.list.iter().enumerate() {
        if e.class == "ambient_generic" && e.spawnflags & 16 == 0 {
            if let Some(r) = ambient_request(&ents, i) {
                play.write(r);
            }
        }
    }
}

fn ambient_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut playing: Query<(Entity, &mut GameSound)>,
    mut play: MessageWriter<PlayGameSound>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "ambient_generic" {
            continue;
        }
        let stop = |commands: &mut Commands, playing: &Query<(Entity, &mut GameSound)>| {
            for (pe, s) in playing {
                if s.owner == Some(msg.target) {
                    commands.entity(pe).despawn();
                }
            }
        };
        match msg.input.as_str() {
            "playsound" => {
                stop(&mut commands, &playing);
                if let Some(r) = ambient_request(&ents, msg.target) {
                    play.write(r);
                }
            }
            "stopsound" | "fadeout" | "kill" => stop(&mut commands, &playing),
            "togglesound" => {
                if playing.iter().any(|(_, s)| s.owner == Some(msg.target)) {
                    stop(&mut commands, &playing);
                } else if let Some(r) = ambient_request(&ents, msg.target) {
                    play.write(r);
                }
            }
            // Volume 0-10 on the playing sound (intro1 turns the announcer's
            // courtesy call down to 3 as Wheatley knocks, then to 0 when
            // the door opens).
            "volume" => {
                let v = ambient_volume(&msg.param);
                debug!("portal2: ambient_generic {} volume {v}", e.name);
                for (_, mut s) in &mut playing {
                    if s.owner == Some(msg.target) {
                        s.base = s.script * v;
                    }
                }
            }
            _ => {}
        }
    }
}

/// An `ambient_generic` `Volume` parameter (0-10) as a gain.
fn ambient_volume(param: &str) -> f32 {
    (param.trim().parse::<f32>().unwrap_or(10.0) / 10.0).clamp(0.0, 1.0)
}

/// The pause menu pauses every sound (voice lines pick up where they left
/// off on resume); menu music keeps playing.
fn pause_audio(time: Res<Time<Virtual>>, sinks: Query<(Entity, &AudioSink, Option<&GameSound>)>, mut paused: Local<bool>, mut seen: Local<std::collections::HashSet<Entity>>) {
    // Paused exactly when the game clock is (menus opened from pause too).
    let want = time.is_paused();
    if want == *paused {
        // A sound that started while paused waits too.
        if want {
            for (e, sink, s) in &sinks {
                if seen.insert(e) && !s.is_some_and(|s| s.music && s.at.is_none() && s.owner.is_none()) {
                    sink.pause();
                }
            }
        }
        return;
    }
    *paused = want;
    seen.clear();
    for (e, sink, s) in &sinks {
        seen.insert(e);
        if s.is_some_and(|s| s.music && s.at.is_none() && s.owner.is_none()) {
            continue;
        }
        if want {
            sink.pause();
        } else {
            sink.play();
        }
    }
}

#[cfg(test)]
mod volume_tests {
    use super::ambient_volume;

    #[test]
    fn ambient_volume_input_scales_0_to_10() {
        assert_eq!(ambient_volume("3"), 0.3);
        assert_eq!(ambient_volume("0"), 0.0);
        assert_eq!(ambient_volume(" 10 "), 1.0);
        assert_eq!(ambient_volume("25"), 1.0);
        assert_eq!(ambient_volume(""), 1.0);
    }
}
