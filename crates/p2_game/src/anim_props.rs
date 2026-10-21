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
//! Scenes (`scenes.rs`) add to that: a `sequence` event plays like
//! SetAnimation (`scenesequence`), a `gesture` event is laid over the
//! current sequence for its time (with its `event_ramp`), and a `lookat`
//! event aims the actor (Wheatley's eye) at its target through the model's
//! aim pose parameters. A new sequence crossfades from the last pose over
//! [`FADE`] seconds instead of snapping, as Source blends sequences.
//!
//! The playback state lives in [`PropAnims::playing`], shared with the
//! cameras and attachments of `viewcontrol.rs` (which also use the pose
//! drawn here, [`PropAnims::local_pose`]).
use std::collections::HashMap;

use bevy::prelude::*;
use p2_assets::anim::{Animator, SeqEvent};
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

/// Seconds a sequence change crossfades over (Source's default sequence
/// fade-in), and a scene gesture without a ramp fades in and out.
pub const FADE: f32 = 0.2;

/// One prop's sequence playback.
#[derive(Clone, Debug)]
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
    /// Unique per started playback (a restart of the same sequence gets a
    /// new one), so the pose code sees sequence changes.
    pub id: u64,
}

impl Playback {
    pub fn new(seq: &str, explicit: bool) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Playback { seq: seq.to_string(), time: 0.0, rate: 1.0, explicit, finished: false, id }
    }
}

impl Playback {
    /// `SetPlaybackRate`. A held last (or first) frame plays again when the
    /// new rate runs back into the sequence: sp_a2_laser_stairs raises its
    /// stairs by playing the held "down" sequence backwards.
    pub fn set_rate(&mut self, r: f32) {
        if self.finished && (r < 0.0 && self.time > 0.0 || r > 0.0 && self.time <= 0.0) {
            self.finished = false;
        }
        self.rate = r;
    }
}

/// Equal playbacks play the same thing at the same point (ids aside).
impl PartialEq for Playback {
    fn eq(&self, o: &Self) -> bool {
        (&self.seq, self.time, self.rate, self.explicit, self.finished) == (&o.seq, o.time, o.rate, o.explicit, o.finished)
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

/// What each prop's parts were last posed with, so a still prop costs nothing.
#[derive(Default)]
struct PoseState {
    /// The playback the pose was made for ([`Playback::id`]) and its sequence.
    id: u64,
    seq: Option<usize>,
    /// (sequence, frame) of the base pose last drawn.
    key: Option<(usize, Option<f32>)>,
    /// The pose last drawn (parent-space bones), to crossfade from.
    local: Vec<(Vec3, Quat)>,
    /// Crossfading from this pose, started at this game time.
    fade: Option<(Vec<(Vec3, Quat)>, f32)>,
    /// Smoothed aim pose parameters (model units, by pose parameter).
    aim: Vec<f32>,
    /// Layers or aim were on last time (one more pose puts them away).
    layered: bool,
    /// How this model's aim parameters turn its eye (measured once).
    aim_cal: Option<Option<AimCal>>,
}

/// What each aim pose parameter does to a model's eye, measured from its
/// own aim matrix: the eye bone's forward (+X) in the root bone's frame at
/// rest, and for each aim parameter the direction it turns it and how far
/// (radians) at +1 and -1 of its range. Wheatley's `aim_yaw` turns his eye
/// right ~29 degrees and `aim_pitch` down ~26 / up ~42, in a frame where
/// his handles are up; measuring avoids assuming names, signs or units.
#[derive(Clone, Debug)]
pub struct AimCal {
    pub rest: Vec3,
    /// (pose parameter, unit direction of motion, angle at range end, angle at range start)
    pub axes: Vec<(usize, Vec3, f32, f32)>,
}

/// The bone a model looks with: its eye light/eyeball, else the root.
fn eye_bone(anim: &Animator) -> usize {
    let names = ["eyelight_aimjoint", "eyeball_eye", "eyeball", "eye"];
    names
        .iter()
        .find_map(|n| anim.skeleton.bones.iter().position(|b| b.name.eq_ignore_ascii_case(n)))
        .or_else(|| anim.skeleton.bones.iter().position(|b| b.name.to_ascii_lowercase().contains("eye")))
        .unwrap_or(0)
}

/// Eye forward in the root bone's frame for a pose.
fn eye_dir(anim: &Animator, local: &[(Vec3, Quat)], eye: usize) -> Vec3 {
    let m = anim.bone_to_model(local);
    let (Some(root), Some(e)) = (m.first(), m.get(eye)) else { return Vec3::X };
    (Mat3::from(root.matrix3).inverse() * (Mat3::from(e.matrix3) * Vec3::X)).normalize_or(Vec3::X)
}

pub fn calibrate_aim(anim: &Animator, base: usize, layer: usize) -> Option<AimCal> {
    let eye = eye_bone(anim);
    let n = anim.pose_params.len();
    let pose_with = |j: usize, v: f32| {
        let mut p = vec![0.0; n];
        if let Some(x) = p.get_mut(j) {
            *x = v;
        }
        p
    };
    let rest_pose = vec![0.0; n];
    let rest = eye_dir(anim, &anim.evaluate(base, 0.0, &rest_pose, &[(layer, 0.0, 1.0)]), eye);
    let mut axes = Vec::new();
    for (j, p) in anim.pose_params.iter().enumerate() {
        if aim_axis(&p.name).is_none() {
            continue;
        }
        let (lo, hi) = (p.start.min(p.end), p.start.max(p.end));
        let up = eye_dir(anim, &anim.evaluate(base, 0.0, &pose_with(j, hi), &[(layer, 0.0, 1.0)]), eye);
        let down = eye_dir(anim, &anim.evaluate(base, 0.0, &pose_with(j, lo), &[(layer, 0.0, 1.0)]), eye);
        let m = (up - down).reject_from(rest).normalize_or_zero();
        if m == Vec3::ZERO {
            continue;
        }
        let ang = |d: Vec3| d.dot(m).atan2(d.dot(rest));
        axes.push((j, m, ang(up), ang(down)));
    }
    (!axes.is_empty()).then_some(AimCal { rest, axes })
}

/// Aim parameter values that turn the eye toward `dir` (root bone frame),
/// at weight `w`.
pub fn calibrated_aim(params: &[p2_assets::anim::PoseParam], cal: &AimCal, dir: Option<Vec3>, w: f32) -> Vec<f32> {
    let mut out: Vec<f32> = params.iter().map(|p| 0.0f32.clamp(p.start.min(p.end), p.start.max(p.end))).collect();
    let Some(d) = dir.and_then(|d| d.try_normalize()) else { return out };
    for &(j, m, a_hi, a_lo) in &cal.axes {
        let Some(p) = params.get(j) else { continue };
        let (lo, hi) = (p.start.min(p.end), p.start.max(p.end));
        let theta = d.dot(m).atan2(d.dot(cal.rest));
        // Linear between the rest pose (0) and each end of the range.
        let v = if theta.signum() == a_hi.signum() && a_hi.abs() > 1e-4 {
            hi * (theta / a_hi)
        } else if a_lo.abs() > 1e-4 {
            lo * (theta / a_lo)
        } else {
            0.0
        };
        out[j] = v.clamp(lo, hi) * w.clamp(0.0, 1.0);
    }
    out
}

#[derive(Resource, Default)]
struct Posed(HashMap<usize, PoseState>);

/// Crossfade weight of the new pose `elapsed` seconds into a [`FADE`]:
/// Source's ease-in-out spline from 0 to 1.
pub fn fade_weight(elapsed: f32) -> f32 {
    let s = (elapsed / FADE).clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

/// Blends two poses bone by bone (`w` = 0: `from`, 1: `to`).
pub fn blend_poses(from: &[(Vec3, Quat)], to: &[(Vec3, Quat)], w: f32) -> Vec<(Vec3, Quat)> {
    to.iter()
        .enumerate()
        .map(|(i, &(p, q))| match from.get(i) {
            Some(&(fp, fq)) => (fp.lerp(p, w), fq.slerp(q, w).normalize()),
            None => (p, q),
        })
        .collect()
}

/// Which way a pose parameter turns an aim: yaw or pitch, and its sign
/// against Source angles (yaw positive to the model's left, pitch positive
/// up). `None` for parameters that don't aim.
pub fn aim_axis(name: &str) -> Option<(bool, f32)> {
    let n = name.to_ascii_lowercase();
    // (is yaw, sign). Source's pitch angles are positive down.
    if n.contains("yaw") || n.contains("rightleft") || n.contains("right_left") {
        Some((true, 1.0))
    } else if n.contains("leftright") || n.contains("left_right") {
        Some((true, -1.0))
    } else if n.contains("pitch") {
        Some((false, -1.0))
    } else if n.contains("updown") || n.contains("up_down") {
        Some((false, 1.0))
    } else {
        None
    }
}

/// Yaw and pitch (degrees, Source convention: yaw left of +X positive,
/// pitch up positive) of a direction in Source model space (+X forward).
pub fn aim_angles(dir: Vec3) -> (f32, f32) {
    let yaw = dir.y.atan2(dir.x).to_degrees();
    let pitch = dir.z.atan2(Vec2::new(dir.x, dir.y).length()).to_degrees();
    (yaw, pitch)
}

/// Aim pose parameter values for a direction (Source model space), at
/// weight `w` (0 = rest, each parameter at 0 clamped into its range).
pub fn aim_pose(params: &[p2_assets::anim::PoseParam], dir: Option<Vec3>, w: f32) -> Vec<f32> {
    let angles = dir.filter(|d| d.length_squared() > 1e-8).map(aim_angles);
    params
        .iter()
        .map(|p| {
            let (lo, hi) = (p.start.min(p.end), p.start.max(p.end));
            let rest = 0.0f32.clamp(lo, hi);
            let aimed = match (aim_axis(&p.name), angles) {
                (Some((yaw, sign)), Some((y, pi))) => (sign * if yaw { y } else { pi }).clamp(lo, hi),
                _ => rest,
            };
            rest + (aimed - rest) * w.clamp(0.0, 1.0)
        })
        .collect()
}

/// Moves `cur` toward `target` (critically damped look-around: ~95% of the
/// way in 0.375 s); snaps when within 0.01.
pub fn smooth_toward(cur: &mut Vec<f32>, target: &[f32], dt: f32) {
    if cur.len() != target.len() {
        *cur = target.to_vec();
        return;
    }
    let k = 1.0 - (-dt * 8.0).exp();
    for (c, &t) in cur.iter_mut().zip(target) {
        *c += (t - *c) * k;
        if (t - *c).abs() < 0.01 {
            *c = t;
        }
    }
}

/// The aim-matrix layer for a base sequence that doesn't auto-layer one: a
/// delta sequence named `*aimmatrix*` whose blend grid is driven by aim
/// parameters, preferring the one sharing most name words with the base
/// (`sphere_idle_neutral` -> `sphere_aimmatrix_neutral`).
pub fn aim_layer(anim: &Animator, base: usize) -> Option<usize> {
    if anim.sequences.get(base).is_some_and(|s| !s.autolayers.is_empty()) {
        return None;
    }
    let base_words: Vec<String> = anim.sequences.get(base)?.label.to_ascii_lowercase().split('_').map(str::to_string).collect();
    anim.sequences
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            s.delta
                && s.label.to_ascii_lowercase().contains("aimmatrix")
                && anim.grid_params(*i).iter().flatten().any(|&p| anim.pose_params.get(p).is_some_and(|p| aim_axis(&p.name).is_some()))
        })
        .max_by_key(|(i, s)| {
            let shared = s.label.to_ascii_lowercase().split('_').filter(|w| base_words.iter().any(|b| b == w)).count();
            (shared, usize::MAX - i)
        })
        .map(|(i, _)| i)
}

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
        anims.local_pose.clear();
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
        if !matches!(input, "setanimation" | "setanimationnoreset" | "setdefaultanimation" | "setplaybackrate" | "door" | "scenesequence" | "floorbutton") {
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
                    pb.set_rate(r);
                }
            }
            "door" if known(param) => {
                let idle = if param == "open" { "idleopen" } else { "idleclose" };
                if known(idle) {
                    anims.defaults.insert(i, idle.to_string());
                }
                anims.playing.insert(i, Playback::new(param, true));
            }
            // Sent by `props::floor_buttons` ("down" / "up"): the button's
            // own press and release, then its pressed or rest idle.
            "floorbutton" => {
                let Some((play, idle)) = floor_button_sequences(&e.class, param == "down") else { continue };
                match idle.filter(|s| known(s)) {
                    Some(idle) => anims.defaults.insert(i, idle.to_string()),
                    None => anims.defaults.remove(&i),
                };
                if known(play) {
                    anims.playing.insert(i, Playback::new(play, false));
                }
            }
            _ => {}
        }
    }
}

/// A floor button model's (press or release sequence, idle afterwards);
/// no idle means the model's sequence 0 (its up pose).
pub fn floor_button_sequences(class: &str, down: bool) -> Option<(&'static str, Option<&'static str>)> {
    Some(match (class, down) {
        ("prop_floor_button", true) => ("down", Some("idledown")),
        ("prop_floor_button", false) => ("up", None),
        ("prop_floor_cube_button", true) => ("down", Some("idledown")),
        ("prop_floor_cube_button", false) => ("up", Some("idle")),
        ("prop_floor_ball_button", true) => ("down", Some("downIdle")),
        ("prop_floor_ball_button", false) => ("idle", None),
        ("prop_under_floor_button", true) => ("press", Some("press_idle")),
        ("prop_under_floor_button", false) => ("release", Some("release_idle")),
        _ => return None,
    })
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

/// The events a playback step passed over: those with a cycle in
/// `(from, to]` (`[0, to]` from the very start), wrapping once for a loop
/// that went past its end. Cycles are fractions of the sequence, 0..1.
pub fn crossed_events(events: &[SeqEvent], from: f32, to: f32, wrapped: bool) -> Vec<&SeqEvent> {
    let hit = |e: &&SeqEvent, a: f32, b: f32| (e.cycle > a || (a <= 0.0 && e.cycle >= 0.0)) && e.cycle <= b;
    let span = |a: f32, b: f32| {
        let mut v: Vec<&SeqEvent> = events.iter().filter(|e| hit(e, a, b)).collect();
        v.sort_by(|x, y| x.cycle.total_cmp(&y.cycle));
        v
    };
    if wrapped {
        [span(from, 1.0), span(-1.0, to)].concat()
    } else {
        span(from, to)
    }
}

/// Source's script event that sends its options (`FireUser3`, `Skin 1`) to
/// the animating entity as an input.
const EVENT_FIRE_INPUT: i32 = 1100;

/// Event 1100: the options are an input (and parameter) the prop sends
/// itself. Source fires it with the prop as caller and activator, so
/// `!activator` outputs (the vac-tube turrets' kill relay) reach the prop.
fn fire_input_event(ents: &mut Ents, i: usize, options: &str) {
    let (input, param) = options.trim().split_once(char::is_whitespace).unwrap_or((options.trim(), ""));
    ents.send_from("!self", input, param.trim(), 0.0, i, Some(i));
}
/// Old numeric sound events (`SCRIPT_EVENT_SOUND`, `CL_EVENT_SOUND`).
const EVENT_SOUNDS: [i32; 3] = [1004, 1005, 5004];

/// Plays every prop's sequence on by the game time since last frame;
/// fires `OnAnimationDone` and the sequence's animation events (inputs on
/// the prop itself, sounds) as playback passes them.
fn advance_anims(
    ents: Option<ResMut<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut fire: MessageWriter<FireOutput>,
    mut sounds: MessageWriter<crate::audio::PlayGameSound>,
    mut last: Local<f32>,
) {
    let Some(mut ents) = ents else { return };
    // Game time stops in menus and restarts at 0 with each level; a loaded
    // save sets it in one jump (its sequences come back with their times).
    let dt = if ents.now >= *last && ents.now - *last < 1.0 { ents.now - *last } else { 0.0 };
    *last = ents.now;
    if dt <= 0.0 {
        return;
    }
    let keys: Vec<usize> = anims.playing.keys().copied().collect();
    for i in keys {
        let e = &ents.list[i];
        let anim = anims.model_of(i, e).and_then(|m| anims.animator(&game, &m));
        let default = default_anim(&anims, i, e);
        let hold = hold_animation(e);
        let timing = |name: &str| match &anim {
            Some(a) => a.find(name).map(|s| SeqTiming::of(a, s)),
            None => None,
        };
        let Some(pb) = anims.playing.get_mut(&i) else { continue };
        let before = pb.clone();
        let done = step(pb, dt, &timing, default.as_deref(), hold);
        let after = pb.clone();
        if done {
            fire.write(FireOutput { ent: i, event: "OnAnimationDone".into(), activator: None });
        }
        // Animation events of the sequence that was playing.
        let Some(a) = &anim else { continue };
        let Some(seq) = a.find(&before.seq) else { continue };
        let events = &a.sequences[seq].events;
        if events.is_empty() || before.finished || before.rate <= 0.0 {
            continue;
        }
        let dur = a.duration(seq).max(1e-6);
        let from = before.time / dur;
        let (to, wrapped) = if after.id == before.id && !after.finished {
            (after.time / dur, after.time < before.time)
        } else {
            (1.0, false)
        };
        let at = ents.list[i].transform.translation;
        for ev in crossed_events(events, from, to, wrapped) {
            let name = ev.name.to_ascii_uppercase();
            if ev.id == EVENT_FIRE_INPUT {
                fire_input_event(&mut ents, i, &ev.options);
            } else if EVENT_SOUNDS.contains(&ev.id) || name.ends_with("_PLAYSOUND") || name == "AE_CL_PLAYSOUND" {
                let sound = ev.options.trim();
                if !sound.is_empty() {
                    sounds.write(crate::audio::PlayGameSound { sound: sound.into(), at: Some(at), looped: false, owner: None, music: false, volume: 1.0, voice: None, ui: false });
                }
            }
        }
    }
}

/// Scene layers and aim on each actor now: gestures (sequence, cycle,
/// weight) and a lookat (target position in the world, weight).
#[derive(Default)]
struct SceneAnim {
    gestures: Vec<(String, f32, f32)>,
    look: Option<(Vec3, f32)>,
}

fn scene_anims(ents: &Ents, scenes: &crate::scenes::Playing, player_eye: Option<Vec3>) -> HashMap<usize, SceneAnim> {
    use crate::scenes::SceneEventKind as K;
    let mut out: HashMap<usize, SceneAnim> = HashMap::new();
    for (scene, ev, t) in scenes.active(ents.now) {
        if !matches!(ev.kind, K::Gesture | K::LookAt) {
            continue;
        }
        let Some(actor) = &ev.actor else { continue };
        let w = ev.weight(t, FADE);
        let len = (ev.end - ev.start).max(1e-3);
        let target = (ev.kind == K::LookAt)
            .then(|| match ev.param.trim().to_ascii_lowercase().as_str() {
                "!player" => player_eye,
                p => crate::scenes::actor_ents(ents, scene, p).first().map(|&i| ents.list[i].transform.translation),
            })
            .flatten();
        for i in crate::scenes::actor_ents(ents, scene, actor) {
            let a = out.entry(i).or_default();
            match ev.kind {
                // Gestures are fitted to their event's length.
                K::Gesture => a.gestures.push((ev.param.clone(), (t / len).clamp(0.0, 1.0), w)),
                _ => {
                    if let Some(p) = target {
                        a.look = Some((p, w));
                    }
                }
            }
        }
    }
    out
}

/// Moves each animated prop's bone parts to its current pose, when it
/// changed: its sequence (crossfaded from the last one), auto-layers, the
/// aim matrix driven by a scene's lookat, and scene gestures on top.
#[allow(clippy::too_many_arguments)]
fn pose_parts(
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    scenes: Res<crate::scenes::Playing>,
    mut anims: ResMut<PropAnims>,
    mut posed: ResMut<Posed>,
    models: Query<(&Children, &GlobalTransform), With<AnimatedModel>>,
    mut parts: Query<(&BonePart, &mut Transform)>,
    players: Query<(&GlobalTransform, Option<&shared::EyeOffset>), With<shared::Player>>,
    mut last: Local<f32>,
) {
    let Some(ents) = ents else { return };
    let now = ents.now;
    let dt = if now >= *last { now - *last } else { 0.0 };
    *last = now;
    let eye = players.single().ok().map(|(t, o)| t.translation() + o.map_or(Vec3::ZERO, |o| o.0));
    let mut scene = scene_anims(&ents, &scenes, eye);
    let animated: Vec<usize> = anims.models.keys().copied().collect();
    for i in animated {
        let e = &ents.list[i];
        let Some((children, model_tf)) = e.entity.and_then(|r| models.get(r).ok()) else { continue };
        let Some(model) = anims.models.get(&i).cloned() else { continue };
        let Some(anim) = anims.animator(&game, &model) else { continue };
        let Some(pb) = anims.playing.get(&i).cloned() else { continue };
        let Some(seq) = anim.find(&pb.seq) else { continue };
        let st = posed.0.entry(i).or_default();
        // A new sequence (or the map restarting one) fades in from the last pose.
        if pb.id != st.id {
            let changed = st.seq != Some(seq) || pb.explicit;
            if changed && !st.local.is_empty() {
                st.fade = Some((st.local.clone(), now));
            }
            st.id = pb.id;
            st.seq = Some(seq);
        }
        let fade = st.fade.as_ref().map(|(_, t0)| fade_weight(now - t0));
        if fade.is_some_and(|w| w >= 1.0) {
            st.fade = None;
        }
        let mut sa = scene.remove(&i).unwrap_or_default();
        // Personality cores keep their eye on the player unless a scene
        // tells them to look somewhere else.
        if sa.look.is_none() && e.class == "npc_personality_core" {
            sa.look = eye.map(|p| (p, 1.0));
        }
        // Aim: the lookat target in Source model space, through the pose
        // parameters (smoothed, so the eye turns rather than jumps).
        let dir = sa.look.map(|(p, w)| {
            let local = model_tf.affine().inverse().transform_point3(p);
            (Vec3::new(local.x, -local.z, local.y), w)
        });
        if st.aim_cal.is_none() {
            st.aim_cal = Some(aim_layer(&anim, seq).and_then(|l| calibrate_aim(&anim, seq, l)));
        }
        let target = match st.aim_cal.clone().flatten() {
            Some(cal) => {
                // Into the root bone's frame of the pose last drawn.
                let root = if st.local.is_empty() { None } else { anim.bone_to_model(&st.local).first().map(|m| Mat3::from(m.matrix3)) };
                let d = dir.map(|(d, _)| root.map_or(d, |r| r.inverse() * d));
                calibrated_aim(&anim.pose_params, &cal, d, dir.map_or(0.0, |d| d.1))
            }
            None => aim_pose(&anim.pose_params, dir.map(|d| d.0), dir.map_or(0.0, |d| d.1)),
        };
        let before = st.aim.clone();
        smooth_toward(&mut st.aim, &target, dt);
        let aiming = st.aim != aim_pose(&anim.pose_params, None, 0.0) || before != st.aim;
        let layers: Vec<(usize, f32, f32)> = sa.gestures.iter().filter_map(|(name, c, w)| anim.find(name).map(|g| (g, *c, *w))).collect();
        let key = (seq, anim.frame_at(seq, pb.time));
        let layered = aiming || !layers.is_empty();
        if st.key == Some(key) && st.fade.is_none() && !layered && !st.layered && before == st.aim {
            continue;
        }
        st.key = Some(key);
        st.layered = layered;
        let mut extra = layers;
        if let Some(al) = aim_layer(&anim, seq).filter(|_| aiming) {
            extra.insert(0, (al, 0.0, 1.0));
        }
        let mut local = anim.evaluate(seq, anim.cycle_at(seq, pb.time), &st.aim, &extra);
        if let (Some((from, _)), Some(w)) = (&st.fade, fade) {
            local = blend_poses(from, &local, w);
        }
        st.local = local.clone();
        let bones = anim.bone_to_model(&local);
        anims.local_pose.insert(i, local);
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

    #[test]
    fn fire_input_event_makes_the_prop_its_activator() {
        let mut ents = Ents::default();
        ents.list.push(GameEnt::logic("prop_dynamic", "vacturret_01", Vec::new()));
        fire_input_event(&mut ents, 0, "fireuser1");
        let p = ents.io_state().queue.pop().expect("queued");
        assert_eq!((p.target.as_str(), p.input.as_str(), p.caller, p.activator), ("!self", "fireuser1", 0, Some(0)));
    }

    fn ev(cycle: f32, opt: &str) -> SeqEvent {
        SeqEvent { cycle, id: EVENT_FIRE_INPUT, name: String::new(), options: opt.into() }
    }

    #[test]
    fn negative_rate_replays_a_held_sequence_backwards() {
        let t = |_: &str| Some(SeqTiming { duration: 1.0, looping: false });
        let mut pb = Playback::new("down", true);
        assert!(step(&mut pb, 2.0, &t, None, true));
        assert!(pb.finished && pb.time == 1.0);
        pb.set_rate(-1.0);
        step(&mut pb, 0.5, &t, None, true);
        assert!(!pb.finished && (pb.time - 0.5).abs() < 1e-6);
        step(&mut pb, 1.0, &t, None, true);
        assert!(pb.finished && pb.time == 0.0);
        pb.set_rate(1.0);
        step(&mut pb, 0.25, &t, None, true);
        assert!(!pb.finished && (pb.time - 0.25).abs() < 1e-6);
    }

    #[test]
    fn floor_buttons_press_and_release() {
        assert_eq!(super::floor_button_sequences("prop_floor_button", true), Some(("down", Some("idledown"))));
        assert_eq!(super::floor_button_sequences("prop_floor_button", false), Some(("up", None)));
        assert_eq!(super::floor_button_sequences("prop_under_floor_button", false), Some(("release", Some("release_idle"))));
        assert_eq!(super::floor_button_sequences("prop_dynamic", true), None);
    }

    #[test]
    fn crossed_events_by_cycle() {
        let evs = vec![ev(0.0, "a"), ev(0.5, "b"), ev(0.7, "c"), ev(0.9, "d")];
        let names = |v: Vec<&SeqEvent>| v.iter().map(|e| e.options.clone()).collect::<Vec<_>>();
        assert_eq!(names(crossed_events(&evs, 0.0, 0.1, false)), ["a"]);
        assert_eq!(names(crossed_events(&evs, 0.1, 0.5, false)), ["b"]);
        assert_eq!(names(crossed_events(&evs, 0.5, 0.69, false)), Vec::<String>::new());
        assert_eq!(names(crossed_events(&evs, 0.6, 1.0, false)), ["c", "d"]);
        // A loop wrapping past its end: the tail, then the head again.
        assert_eq!(names(crossed_events(&evs, 0.8, 0.1, true)), ["d", "a"]);
    }

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

    #[test]
    fn restarts_get_new_ids_but_compare_equal() {
        let (a, b) = (Playback::new("open", true), Playback::new("open", true));
        assert_ne!(a.id, b.id);
        assert_eq!(a, b);
    }

    #[test]
    fn crossfade_eases_from_old_pose_to_new() {
        assert_eq!(fade_weight(0.0), 0.0);
        assert!((fade_weight(FADE * 0.5) - 0.5).abs() < 1e-6);
        assert!(fade_weight(FADE * 0.25) < 0.25, "eases in");
        assert_eq!(fade_weight(FADE), 1.0);
        assert_eq!(fade_weight(10.0), 1.0);
        let from = vec![(Vec3::ZERO, Quat::IDENTITY), (Vec3::X, Quat::IDENTITY)];
        let to = vec![(Vec3::new(2.0, 0.0, 0.0), Quat::from_rotation_z(1.0)), (Vec3::X, Quat::IDENTITY), (Vec3::Y, Quat::IDENTITY)];
        let mid = blend_poses(&from, &to, 0.5);
        assert!((mid[0].0 - Vec3::X).length() < 1e-6);
        assert!(mid[0].1.angle_between(Quat::from_rotation_z(0.5)) < 1e-4);
        // A bone the old pose lacks takes the new pose.
        assert_eq!(mid[2], to[2]);
        assert_eq!(blend_poses(&from, &to, 0.0)[0], from[0]);
        assert_eq!(blend_poses(&from, &to, 1.0)[0].0, to[0].0);
    }

    #[test]
    fn aim_angles_in_source_model_space() {
        let close = |(a, b): (f32, f32), (c, d): (f32, f32)| (a - c).abs() < 1e-3 && (b - d).abs() < 1e-3;
        assert!(close(aim_angles(Vec3::X), (0.0, 0.0)));
        assert!(close(aim_angles(Vec3::Y), (90.0, 0.0)), "left is +yaw");
        assert!(close(aim_angles(Vec3::new(1.0, -1.0, 0.0)), (-45.0, 0.0)));
        assert!(close(aim_angles(Vec3::new(1.0, 0.0, 1.0)), (0.0, 45.0)), "up is +pitch");
    }

    #[test]
    fn aim_pose_drives_named_parameters() {
        let pp = |name: &str, start: f32, end: f32| p2_assets::anim::PoseParam { name: name.into(), start, end };
        let params = vec![pp("eyes_updown", -45.0, 45.0), pp("eyes_rightleft", -60.0, 60.0), pp("head_pitch", -30.0, 30.0), pp("mouth", 0.0, 1.0), pp("body_yaw", 10.0, 90.0)];
        assert_eq!(aim_axis("eyes_leftright"), Some((true, -1.0)));
        assert_eq!(aim_axis("mouth"), None);
        // 30 degrees left and 20 up.
        let dir = Vec3::new(30f32.to_radians().cos(), 30f32.to_radians().sin(), 0.0).normalize();
        let dir = Vec3::new(dir.x, dir.y, (dir.x * dir.x + dir.y * dir.y).sqrt() * 20f32.to_radians().tan());
        let v = aim_pose(&params, Some(dir), 1.0);
        assert!((v[0] - 20.0).abs() < 1e-3, "{v:?}");
        assert!((v[1] - 30.0).abs() < 1e-3, "{v:?}");
        assert!((v[2] + 20.0).abs() < 1e-3, "Source pitch is positive down: {v:?}");
        assert_eq!(v[3], 0.0, "not an aim parameter");
        assert!((v[4] - 30.0).abs() < 1e-3);
        // Half weight is halfway from rest; rest clamps 0 into the range.
        let h = aim_pose(&params, Some(dir), 0.5);
        assert!((h[0] - 10.0).abs() < 1e-3 && (h[4] - 20.0).abs() < 1e-3, "{h:?}");
        assert_eq!(aim_pose(&params, None, 1.0), vec![0.0, 0.0, 0.0, 0.0, 10.0]);
        // Out of range targets clamp.
        let behind = aim_pose(&params, Some(Vec3::new(-1.0, 0.1, 0.0)), 1.0);
        assert_eq!(behind[1], 60.0);
    }

    #[test]
    fn aim_smoothing_turns_then_settles() {
        let mut cur = vec![0.0, 0.0];
        smooth_toward(&mut cur, &[30.0, -10.0], 1.0 / 60.0);
        assert!(cur[0] > 0.0 && cur[0] < 10.0, "{cur:?}");
        for _ in 0..120 {
            smooth_toward(&mut cur, &[30.0, -10.0], 1.0 / 60.0);
        }
        assert_eq!(cur, vec![30.0, -10.0]);
        // A new parameter list is taken as is.
        smooth_toward(&mut cur, &[1.0], 0.1);
        assert_eq!(cur, vec![1.0]);
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
