//! Animated props: every `prop_dynamic` (and test chamber door, personality
//! core) plays its model's current sequence. The model is drawn as rigid
//! per-bone parts (`source_assets::AnimatedModel` / `BonePart`); this moves
//! the parts to the sampled pose, only when the pose changes.
//!
//! Inputs: `SetAnimation`, `SetAnimationNoReset`, `SetDefaultAnimation`,
//! `SetPlaybackRate`; test chamber doors play `open`/`close` on
//! `Open`/`Close`. A non-looping sequence that ends fires `OnAnimationDone`
//! and the prop goes back to its default animation (`DefaultAnim`, or the
//! last `SetDefaultAnimation`), unless `HoldAnimation` is set: then it, like
//! a prop with no default, holds the last frame. A non-looping default
//! sequence replays (only `HoldAnimation` stops it), without firing again.
//!
//! The playback state lives in [`PropAnims::playing`], shared with the
//! cameras and attachments of `viewcontrol.rs`.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_assets::anim::Animator;
use p2_assets::GameFsRes;
use source_assets::{AnimatedModel, BonePart};

use crate::io::{EntInput, Ents, FireOutput, GameEnt};
use crate::level::LevelLoaded;
use crate::viewcontrol::PropAnims;

pub struct AnimPropsPlugin;

impl Plugin for AnimPropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Posed>().add_systems(
            Update,
            (anim_inputs, start_default_anims, advance_anims, pose_parts)
                .chain()
                .after(crate::io::run_io)
                .before(crate::viewcontrol::track_animations)
                .in_set(crate::level::GameSet::Logic),
        );
    }
}

/// One prop's sequence playback.
#[derive(Clone, Debug, PartialEq)]
pub struct Playback {
    /// The sequence, by name as the map gave it.
    pub seq: String,
    /// Seconds into the sequence (game time times the playback rate).
    pub time: f32,
    pub rate: f32,
    /// Set by an input (`SetAnimation`): fires `OnAnimationDone` when it ends.
    /// The default sequence is not.
    pub explicit: bool,
    /// A non-looping sequence reached its end and holds its last frame.
    pub finished: bool,
}

impl Playback {
    pub fn new(seq: &str, explicit: bool) -> Self {
        Playback { seq: seq.to_string(), time: 0.0, rate: 1.0, explicit, finished: false }
    }
}

/// Length and looping of a sequence.
#[derive(Clone, Copy, Debug)]
pub struct SeqTiming {
    pub duration: f32,
    pub looping: bool,
}

impl SeqTiming {
    fn of(anim: &Animator, seq: usize) -> Self {
        SeqTiming { duration: anim.duration(seq), looping: anim.looping(seq) }
    }
}

/// Advances a playback by `dt` game seconds. `timing` describes a sequence
/// by name (`None`: unknown, which just keeps time); `default` is the
/// prop's default animation, `hold` its `HoldAnimation`. Returns true when
/// `OnAnimationDone` fires.
pub fn step(pb: &mut Playback, dt: f32, timing: &dyn Fn(&str) -> Option<SeqTiming>, default: Option<&str>, hold: bool) -> bool {
    if pb.finished {
        return false;
    }
    pb.time += dt * pb.rate;
    let Some(t) = timing(&pb.seq) else { return false };
    if t.looping {
        if t.duration > 0.0 {
            pb.time = pb.time.rem_euclid(t.duration);
        }
        return false;
    }
    let ended = if pb.rate >= 0.0 { pb.time >= t.duration } else { pb.time <= 0.0 };
    if !ended {
        return false;
    }
    let fired = pb.explicit;
    pb.time = pb.time.clamp(0.0, t.duration);
    match default.filter(|d| !hold && timing(d).is_some()) {
        // Back to the default from the start (a non-looping default replays).
        Some(d) => *pb = Playback::new(d, false),
        None => pb.finished = true,
    }
    fired
}

/// The default animation a prop returns to: the last `SetDefaultAnimation`,
/// else its `DefaultAnim`, else its class's (test chamber doors).
fn default_anim(anims: &PropAnims, i: usize, e: &GameEnt) -> Option<String> {
    anims
        .defaults
        .get(&i)
        .cloned()
        .or_else(|| e.get("defaultanim").map(str::trim).filter(|s| !s.is_empty()).map(str::to_string))
        .or_else(|| source_assets::map::class_default_sequence(&e.class).map(str::to_string))
}

/// What a prop plays when nothing asked for anything: its default
/// animation, else (personality cores) its idle activity, else sequence 0.
fn initial_seq(anims: &PropAnims, i: usize, e: &GameEnt, anim: &Animator) -> Option<String> {
    if let Some(d) = default_anim(anims, i, e).filter(|d| anim.find(d).is_some()) {
        return Some(d);
    }
    let seq = (e.class == "npc_personality_core").then(|| anim.find_activity("ACT_IDLE")).flatten().unwrap_or(0);
    anim.sequences.get(seq).map(|s| s.label.clone())
}

fn hold_animation(e: &GameEnt) -> bool {
    e.get("holdanimation").is_some_and(|v| v.trim() == "1")
}

/// Pose last put on each prop's parts: (sequence index, frame), so a still
/// prop costs nothing.
#[derive(Resource, Default)]
struct Posed(HashMap<usize, (usize, Option<f32>)>);

#[allow(clippy::too_many_arguments)]
fn anim_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut posed: ResMut<Posed>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        anims.playing.clear();
        anims.defaults.clear();
        anims.models.clear();
        posed.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let i = msg.target;
        let e = &ents.list[i];
        let param = msg.param.trim();
        // Door entities run their own open/close sequences.
        let (input, param) = match (e.class.as_str(), msg.input.as_str()) {
            ("prop_testchamber_door", "open") => ("door", "open"),
            ("prop_testchamber_door", "close") => ("door", "close"),
            (_, input) => (input, param),
        };
        if !matches!(input, "setanimation" | "setanimationnoreset" | "setdefaultanimation" | "setplaybackrate" | "door" | "scenesequence") {
            continue;
        }
        let anim = anims.model_of(i, e).and_then(|m| anims.animator(&game, &m));
        // Unknown sequences are ignored, as in the engine (but keep the name
        // when the model can't be read, for the attachment code).
        let known = |name: &str| anim.as_ref().is_none_or(|a| a.find(name).is_some());
        match input {
            "setanimation" | "setanimationnoreset" if known(param) => {
                let same = anims.playing.get(&i).is_some_and(|p| p.seq.eq_ignore_ascii_case(param) && !p.finished);
                if input == "setanimation" || !same {
                    anims.playing.insert(i, Playback::new(param, true));
                }
            }
            // A scene's sequence / gesture event: plays, then back to the
            // default, without the map's OnAnimationDone.
            "scenesequence" if known(param) => {
                anims.playing.insert(i, Playback::new(param, false));
            }
            "setdefaultanimation" if known(param) => {
                anims.defaults.insert(i, param.to_string());
            }
            "setplaybackrate" => {
                if let (Some(pb), Some(r)) = (anims.playing.get_mut(&i), source_assets::kv::parse_f32(param)) {
                    pb.rate = r;
                }
            }
            "door" if known(param) => {
                let idle = if param == "open" { "idleopen" } else { "idleclose" };
                if known(idle) {
                    anims.defaults.insert(i, idle.to_string());
                }
                anims.playing.insert(i, Playback::new(param, true));
            }
            _ => {}
        }
    }
}

/// Props drawn animated start their default sequence.
fn start_default_anims(ents: Option<Res<Ents>>, game: Res<GameFsRes>, mut anims: ResMut<PropAnims>, models: Query<&AnimatedModel>) {
    let Some(ents) = ents else { return };
    for (i, e) in ents.list.iter().enumerate() {
        if e.killed || anims.playing.contains_key(&i) && anims.models.contains_key(&i) {
            continue;
        }
        let Some(m) = e.entity.and_then(|r| models.get(r).ok()) else { continue };
        anims.models.insert(i, m.model.clone());
        if anims.playing.contains_key(&i) {
            continue;
        }
        let Some(anim) = anims.animator(&game, &m.model) else { continue };
        if let Some(seq) = initial_seq(&anims, i, e, &anim) {
            anims.playing.insert(i, Playback::new(&seq, false));
        }
    }
}

/// Plays every prop's sequence on by the game time since last frame;
/// fires `OnAnimationDone`.
fn advance_anims(ents: Option<Res<Ents>>, game: Res<GameFsRes>, mut anims: ResMut<PropAnims>, mut fire: MessageWriter<FireOutput>, mut last: Local<f32>) {
    let Some(ents) = ents else { return };
    // Game time stops in menus and restarts at 0 with each level.
    let dt = if ents.now >= *last { ents.now - *last } else { 0.0 };
    *last = ents.now;
    if dt <= 0.0 {
        return;
    }
    let keys: Vec<usize> = anims.playing.keys().copied().collect();
    for i in keys {
        let e = &ents.list[i];
        let anim = anims.model_of(i, e).and_then(|m| anims.animator(&game, &m));
        let default = default_anim(&anims, i, e);
        let timing = |name: &str| match &anim {
            Some(a) => a.find(name).map(|s| SeqTiming::of(a, s)),
            None => None,
        };
        let Some(pb) = anims.playing.get_mut(&i) else { continue };
        if step(pb, dt, &timing, default.as_deref(), hold_animation(e)) {
            fire.write(FireOutput { ent: i, event: "OnAnimationDone".into(), activator: None });
        }
    }
}

/// Moves each animated prop's bone parts to its current pose, when it changed.
fn pose_parts(
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut posed: ResMut<Posed>,
    models: Query<&Children, With<AnimatedModel>>,
    mut parts: Query<(&BonePart, &mut Transform)>,
) {
    let Some(ents) = ents else { return };
    let animated: Vec<usize> = anims.models.keys().copied().collect();
    for i in animated {
        let e = &ents.list[i];
        let Some(children) = e.entity.and_then(|r| models.get(r).ok()) else { continue };
        let Some(model) = anims.models.get(&i).cloned() else { continue };
        let Some(anim) = anims.animator(&game, &model) else { continue };
        let Some(pb) = anims.playing.get(&i) else { continue };
        let Some(seq) = anim.find(&pb.seq) else { continue };
        let key = (seq, anim.frame_at(seq, pb.time));
        if posed.0.get(&i) == Some(&key) {
            continue;
        }
        posed.0.insert(i, key);
        let bones = anim.bone_to_model(&anim.sample(seq, pb.time));
        for c in children.iter() {
            let Ok((part, mut tf)) = parts.get_mut(c) else { continue };
            let Some(m) = bones.get(part.bone as usize) else { continue };
            let t = source_assets::models::bone_transform(*m);
            // Decoders that misread an animation give nonsense: keep the last pose.
            if !t.translation.is_finite() || !t.rotation.is_finite() {
                continue;
            }
            tf.translation = t.translation;
            tf.rotation = t.rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(name: &str) -> Option<SeqTiming> {
        match name {
            "open" => Some(SeqTiming { duration: 2.0, looping: false }),
            "idle_open" => Some(SeqTiming { duration: 1.0, looping: true }),
            "idle_close" => Some(SeqTiming { duration: 0.5, looping: false }),
            _ => None,
        }
    }

    #[test]
    fn plays_and_ends_once() {
        let mut pb = Playback::new("open", true);
        assert!(!step(&mut pb, 1.5, &timing, None, false));
        assert_eq!(pb.time, 1.5);
        assert!(step(&mut pb, 1.0, &timing, None, false), "OnAnimationDone");
        assert!(pb.finished && pb.time == 2.0, "{pb:?}");
        // Holds the last frame and never fires again.
        assert!(!step(&mut pb, 5.0, &timing, None, false));
        assert_eq!(pb.time, 2.0);
    }

    #[test]
    fn looping_wraps_and_never_ends() {
        let mut pb = Playback::new("idle_open", true);
        for _ in 0..10 {
            assert!(!step(&mut pb, 0.35, &timing, None, false));
        }
        assert!((pb.time - 0.5).abs() < 1e-4, "{}", pb.time);
        assert!(!pb.finished);
    }

    #[test]
    fn playback_rate_scales_time() {
        let mut pb = Playback::new("open", true);
        pb.rate = 0.5;
        assert!(!step(&mut pb, 3.0, &timing, None, false));
        assert_eq!(pb.time, 1.5);
        assert!(step(&mut pb, 1.0, &timing, None, false));
        // Played backwards it ends at the start.
        let mut pb = Playback::new("open", true);
        pb.time = 1.0;
        pb.rate = -1.0;
        assert!(step(&mut pb, 1.5, &timing, None, false));
        assert_eq!(pb.time, 0.0);
    }

    #[test]
    fn returns_to_default_after_ending() {
        let mut pb = Playback::new("open", true);
        assert!(step(&mut pb, 2.5, &timing, Some("idle_open"), false));
        assert_eq!(pb, Playback::new("idle_open", false));
        // HoldAnimation keeps the last frame instead.
        let mut pb = Playback::new("open", true);
        assert!(step(&mut pb, 2.5, &timing, Some("idle_open"), true));
        assert!(pb.finished && pb.seq == "open");
        // An unknown default is no default.
        let mut pb = Playback::new("open", true);
        assert!(step(&mut pb, 2.5, &timing, Some("nope"), false));
        assert!(pb.finished && pb.seq == "open");
    }

    #[test]
    fn non_looping_default_replays_silently() {
        let mut pb = Playback::new("idle_close", false);
        assert!(!step(&mut pb, 0.6, &timing, Some("idle_close"), false));
        assert_eq!(pb, Playback::new("idle_close", false));
        // Without a default it holds.
        let mut pb = Playback::new("idle_close", false);
        assert!(!step(&mut pb, 0.6, &timing, None, false));
        assert!(pb.finished);
    }

    #[test]
    fn unknown_sequences_just_keep_time() {
        let mut pb = Playback::new("mystery", true);
        assert!(!step(&mut pb, 100.0, &timing, Some("idle_open"), false));
        assert_eq!(pb.time, 100.0);
    }

    fn ent(class: &str, kv: &[(&str, &str)]) -> GameEnt {
        GameEnt::logic(class, "x", kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    #[test]
    fn default_animation_fallbacks() {
        let mut anims = PropAnims::default();
        let e = ent("prop_dynamic", &[("defaultanim", "idle_close")]);
        assert_eq!(default_anim(&anims, 3, &e).as_deref(), Some("idle_close"));
        anims.defaults.insert(3, "idle_open".into());
        assert_eq!(default_anim(&anims, 3, &e).as_deref(), Some("idle_open"));
        assert_eq!(default_anim(&anims, 4, &ent("prop_dynamic", &[("defaultanim", " ")])), None);
        assert_eq!(default_anim(&anims, 4, &ent("prop_testchamber_door", &[])).as_deref(), Some("idleclose"));
        assert!(hold_animation(&ent("prop_dynamic", &[("holdanimation", "1")])));
        assert!(!hold_animation(&ent("prop_dynamic", &[])));
    }
}
