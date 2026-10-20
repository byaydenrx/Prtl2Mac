//! Things in the map that move or switch: elevator trains on their path
//! tracks, sliding and swinging doors, chamber doors, brushes that come and
//! go, animated props that open, and everything parented to them. The
//! player rides whatever they stand on.
use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;
use serde::{Deserialize, Serialize};
use shared::{source_to_world, SOURCE_UNIT};

use crate::io::{EntInput, Ents, FireOutput, GameEnt};
use crate::level::{CurrentMap, LevelLoaded};

pub struct MoversPlugin;

impl Plugin for MoversPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Movers>()
            .init_resource::<MoverLerp>()
            .add_systems(FixedFirst, lerp_restore)
            .add_systems(PostUpdate, lerp_draw.before(bevy::transform::TransformSystems::Propagate))
            .add_systems(Update, (setup_movers, mover_inputs).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            // On the fixed tick, after the player moves and before physics
            // syncs colliders: the player is carried by exactly the step the
            // colliders make, so the next move sees them both in step.
            .add_systems(FixedUpdate, (run_movers, follow_parents, ride).chain().after(P2Systems::Transit));
        if std::env::var_os("P2_RIDE_TRACE").is_some() {
            app.add_systems(PostUpdate, ride_trace.after(bevy::transform::TransformSystems::Propagate));
        }
    }
}

/// Things moved on the last tick: (last tick's translation, this tick's).
/// They are drawn between the two at the fixed-step overstep, like the
/// player's feet, so a ride doesn't jitter against the interpolated view;
/// the tick's own value is put back before the next tick (physics and the
/// movers only ever see tick positions).
#[derive(Resource, Default)]
pub(crate) struct MoverLerp(HashMap<Entity, (Vec3, Vec3)>);

impl MoverLerp {
    /// Moved again in the same tick (a child of a moved parent): keep the
    /// first starting point.
    fn record(&mut self, e: Entity, before: Vec3, after: Vec3) {
        self.0.entry(e).and_modify(|v| v.1 = after).or_insert((before, after));
    }
}

fn lerp_restore(mut lerp: ResMut<MoverLerp>, mut transforms: Query<&mut Transform>) {
    for (e, (_, curr)) in lerp.0.drain() {
        if let Ok(mut t) = transforms.get_mut(e) {
            t.translation = curr;
        }
    }
}

fn lerp_draw(lerp: Res<MoverLerp>, fixed: Res<Time<Fixed>>, mut transforms: Query<&mut Transform>) {
    let a = fixed.overstep_fraction();
    for (&e, &(prev, curr)) in &lerp.0 {
        if let Ok(mut t) = transforms.get_mut(e) {
            t.translation = prev.lerp(curr, a);
        }
    }
}

/// `P2_RIDE_TRACE`: per frame, the camera's height relative to each train
/// carrying the player (a steady ride keeps it constant).
fn ride_trace(
    ents: Option<Res<Ents>>,
    movers: Res<Movers>,
    fixed: Res<Time<Fixed>>,
    players: Query<&InterpolatedFeet, With<Player>>,
    globals: Query<&GlobalTransform>,
) {
    let Some(ents) = ents else { return };
    let Ok(feet) = players.single() else { return };
    let eye = feet.at(fixed.overstep_fraction());
    for (&i, off) in &movers.riders {
        let (Some(_), Some(r)) = (off, ents.list[i].entity) else { continue };
        let Ok(car) = globals.get(r) else { continue };
        info!("RIDE {} rel {:.4} eye {:.4} car {:.4} overstep {:.2}", ents.list[i].name, eye.y - car.translation().y, eye.y, car.translation().y, fixed.overstep_fraction());
    }
}

#[derive(Clone, Debug)]
enum Motion {
    /// Straight to a point (doors, movelinear), at a speed (m/s).
    To { target: Vec3, speed: f32, done: &'static str },
    /// Along path nodes (trains): the remaining node indices.
    Path { nodes: Vec<usize>, speed: f32, dest: Option<usize> },
}

#[derive(Clone, Debug)]
struct Mover {
    /// Where its origin is now (world metres).
    pos: Vec3,
    /// Closed position (doors) / spawn position.
    home: Vec3,
    motion: Option<Motion>,
    /// Train: the path node it is at (or heading from).
    node: Option<usize>,
}

/// Run-time state of every moving or switching map entity.
#[derive(Resource, Default)]
pub struct Movers {
    movers: HashMap<usize, Mover>,
    /// Child ent -> (parent ent, offset from the parent at spawn).
    parented: Vec<(usize, usize, Transform)>,
    /// Movement of each mover this frame (for children and riders).
    moved: HashMap<usize, Vec3>,
    /// Turn of each train this frame, about its origin (riders turn too).
    turned: HashMap<usize, Quat>,
    /// Where each parent was last tick, to catch parents moved by something
    /// other than a mover (a prop's animated attachment carrying the
    /// sp_a1_wakeup socket, whose trigger is parented to it).
    parent_seen: HashMap<usize, Transform>,
    /// Timed switches: (time, ent, show).
    pending_show: Vec<(f32, usize, bool)>,
    /// Trains carrying the player: the player's offset from the train
    /// origin (`None` until checked on the next tick).
    riders: HashMap<usize, Option<Vec3>>,
}

/// [`Movers`] as plain data, for saved games.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoversSave {
    pub movers: Vec<MoverSave>,
    /// (child, parent, offset translation, offset rotation).
    pub parented: Vec<(usize, usize, Vec3, Quat)>,
    pub pending_show: Vec<(f32, usize, bool)>,
    pub riders: Vec<(usize, Option<Vec3>)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoverSave {
    pub ent: usize,
    pub pos: Vec3,
    pub home: Vec3,
    pub node: Option<usize>,
    pub motion: Option<MotionSave>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MotionSave {
    To { target: Vec3, speed: f32, done: String },
    Path { nodes: Vec<usize>, speed: f32, dest: Option<usize> },
}

impl Movers {
    /// Whether mover `i` (an ent index) is under way.
    pub fn is_moving(&self, i: usize) -> bool {
        self.movers.get(&i).is_some_and(|m| m.motion.is_some())
    }

    pub fn save(&self) -> MoversSave {
        let mut movers: Vec<MoverSave> = self
            .movers
            .iter()
            .map(|(&ent, m)| MoverSave {
                ent,
                pos: m.pos,
                home: m.home,
                node: m.node,
                motion: m.motion.as_ref().map(|mo| match mo {
                    Motion::To { target, speed, done } => MotionSave::To { target: *target, speed: *speed, done: done.to_string() },
                    Motion::Path { nodes, speed, dest } => MotionSave::Path { nodes: nodes.clone(), speed: *speed, dest: *dest },
                }),
            })
            .collect();
        movers.sort_by_key(|m| m.ent);
        let mut riders: Vec<(usize, Option<Vec3>)> = self.riders.iter().map(|(k, v)| (*k, *v)).collect();
        riders.sort_by_key(|r| r.0);
        MoversSave {
            movers,
            parented: self.parented.iter().map(|(c, p, t)| (*c, *p, t.translation, t.rotation)).collect(),
            pending_show: self.pending_show.clone(),
            riders,
        }
    }

    /// Put back a [`MoversSave`]; movers it doesn't name keep their state.
    pub fn restore(&mut self, s: &MoversSave) {
        for m in &s.movers {
            let motion = m.motion.as_ref().map(|mo| match mo {
                MotionSave::To { target, speed, done } => {
                    let done = if done == "onfullyclosed" { "onfullyclosed" } else { "onfullyopen" };
                    Motion::To { target: *target, speed: *speed, done }
                }
                MotionSave::Path { nodes, speed, dest } => Motion::Path { nodes: nodes.clone(), speed: *speed, dest: *dest },
            });
            self.movers.insert(m.ent, Mover { pos: m.pos, home: m.home, motion, node: m.node });
        }
        self.parented = s.parented.iter().map(|&(c, p, t, r)| (c, p, Transform::from_translation(t).with_rotation(r))).collect();
        self.pending_show = s.pending_show.clone();
        self.riders = s.riders.iter().copied().collect();
        self.moved.clear();
    }
}

const MOVER_CLASSES: &[&str] = &["func_tracktrain", "func_door", "func_movelinear", "func_train", "func_platrot"];

fn u(v: f32) -> f32 {
    v * SOURCE_UNIT
}

/// Source `movedir`/angles (pitch yaw roll) to a world direction.
fn angles_dir(e: &GameEnt, key: &str) -> Vec3 {
    let v: Vec<f32> = e.get(key).unwrap_or("0 0 0").split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let (p, y) = (v.first().copied().unwrap_or(0.0).to_radians(), v.get(1).copied().unwrap_or(0.0).to_radians());
    let src = Vec3::new(p.cos() * y.cos(), p.cos() * y.sin(), -p.sin());
    source_to_world(src) / SOURCE_UNIT
}

/// How far a `func_door` / `func_movelinear` moves when it opens, and which way.
fn door_travel(e: &GameEnt, map: &CurrentMap) -> (Vec3, f32) {
    let dir = angles_dir(e, "movedir");
    let travel = if e.class == "func_movelinear" {
        u(e.get_f32("movedistance").unwrap_or(0.0))
    } else {
        let (lo, hi) = e.brush_model.and_then(|b| map.model_bounds.get(&b).copied()).unwrap_or((Vec3::ZERO, Vec3::ZERO));
        let size = hi - lo;
        (size * dir.abs()).length() - u(e.get_f32("lip").unwrap_or(0.0))
    };
    (dir, travel)
}

fn setup_movers(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    mut movers: ResMut<Movers>,
    mut transforms: Query<&mut Transform>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    let (Some(mut ents), Some(map)) = (ents, map) else { return };
    *movers = Movers::default();
    // The brush of a func_areaportalwindow is only drawn where the engine
    // fades the area portal out at a distance (black, seen from far away);
    // with no fade (Portal 2's door instances) it is never seen. Drawn, it
    // was a black plane in every open doorway.
    let windows: Vec<String> = ents.list.iter().filter(|e| e.class == "func_areaportalwindow").filter_map(|e| e.get("target").map(str::to_string)).collect();
    for w in &windows {
        for &t in &ents.find(w, 0, None) {
            if let Some(r) = ents.list[t].entity {
                commands.entity(r).insert(Visibility::Hidden);
            }
        }
    }
    for (i, e) in ents.list.iter().enumerate() {
        if MOVER_CLASSES.contains(&e.class.as_str()) {
            let pos = e.transform.translation;
            let node = (e.class == "func_tracktrain" || e.class == "func_train")
                .then(|| e.get("target").and_then(|t| ents.find(t, i, None).first().copied()))
                .flatten();
            movers.movers.insert(i, Mover { pos, home: pos, motion: None, node });
        }
    }
    // Parenting: remember each child's pose relative to its parent.
    for (i, e) in ents.list.iter().enumerate() {
        let Some(parent) = e.get("parentname") else { continue };
        let parent = parent.split(',').next().unwrap_or(parent);
        if let Some(&p) = ents.find(parent, i, None).first() {
            let pt = ents.list[p].transform;
            let offset = Transform::from_translation(pt.rotation.inverse() * (e.transform.translation - pt.translation))
                .with_rotation(pt.rotation.inverse() * e.transform.rotation);
            movers.parented.push((i, p, offset));
        }
    }
    // Doors that start open (`spawnpos 1`: lifts lowered until a laser
    // powers them, catchers hidden until they rise): the map places them
    // closed; they and what rides them start moved open.
    for i in 0..ents.list.len() {
        let e = &ents.list[i];
        if e.class != "func_door" || e.get("spawnpos") != Some("1") || !movers.movers.contains_key(&i) {
            continue;
        }
        let (dir, travel) = door_travel(e, &map);
        let delta = dir * travel;
        if let Some(m) = movers.movers.get_mut(&i) {
            m.pos += delta;
        }
        let mut riders = vec![i];
        let mut k = 0;
        while k < riders.len() {
            let p = riders[k];
            riders.extend(movers.parented.iter().filter(|c| c.1 == p && !riders.contains(&c.0)).map(|c| c.0).collect::<Vec<_>>());
            k += 1;
        }
        for r in riders {
            if r != i {
                if let Some(m) = movers.movers.get_mut(&r) {
                    m.pos += delta;
                    m.home += delta;
                }
            }
            let e = &mut ents.list[r];
            e.transform.translation += delta;
            if let Some((lo, hi)) = &mut e.bounds {
                *lo += delta;
                *hi += delta;
            }
            if let Some(mut t) = e.entity.and_then(|x| transforms.get_mut(x).ok()) {
                t.translation += delta;
            }
        }
    }
}

/// Inputs for movers, doors and switchable brushes and props.
#[allow(clippy::too_many_arguments)]
fn mover_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    mut movers: ResMut<Movers>,
    mut fire: MessageWriter<FireOutput>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    scripted: Res<crate::vscripts::ScriptedEnts>,
) {
    let (Some(mut ents), Some(map)) = (ents, map) else {
        inputs.clear();
        return;
    };
    let now = ents.now;
    for msg in inputs.read() {
        let i = msg.target;
        let class = ents.list[i].class.clone();
        let input = msg.input.as_str();
        // EF_NODRAW on anything drawn (the container ride hides its spare
        // door and debris this way); collision is left alone.
        if matches!(input, "disabledraw" | "enabledraw") && i != crate::io::PLAYER {
            if let Some(r) = ents.list[i].entity {
                commands.entity(r).insert(if input == "enabledraw" { Visibility::Inherited } else { Visibility::Hidden });
            }
            continue;
        }
        // Re-parenting at run time (Wheatley onto his socket, off his rail).
        match input {
            "clearparent" => movers.parented.retain(|p| p.0 != i),
            "setparent" => {
                movers.parented.retain(|p| p.0 != i);
                if let Some(&p) = ents.find(msg.param.trim(), i, msg.activator).first() {
                    let pt = ents.list[p].transform;
                    let e = &ents.list[i];
                    let offset = Transform::from_translation(pt.rotation.inverse() * (e.transform.translation - pt.translation))
                        .with_rotation(pt.rotation.inverse() * e.transform.rotation);
                    movers.parented.push((i, p, offset));
                }
            }
            _ => {}
        }
        match class.as_str() {
            "func_tracktrain" | "func_train" => {
                let speed = u(ents.list[i].get_f32("startspeed").unwrap_or(100.0).max(1.0));
                let Some(m) = movers.movers.get_mut(&i) else { continue };
                match input {
                    "startforward" | "resume" | "toggle" | "start" => {
                        let nodes = path_from(&ents, m.node, None);
                        m.motion = Some(Motion::Path { nodes, speed, dest: None });
                        movers.riders.insert(i, None);
                    }
                    "movetopathnode" | "teleporttopathnode" => {
                        let dest = ents.find(&msg.param, i, None).first().copied();
                        if input == "teleporttopathnode" {
                            if let Some(d) = dest {
                                m.pos = ents.list[d].transform.translation;
                                m.node = Some(d);
                            }
                        } else {
                            let nodes = path_from(&ents, m.node, dest);
                            m.motion = Some(Motion::Path { nodes, speed, dest });
                            movers.riders.insert(i, None);
                        }
                    }
                    "stop" => m.motion = None,
                    // The train's top speed: later moves use it and a move
                    // under way speeds up (sp_a1_wakeup's breaker lift goes
                    // 1 -> 52 -> 155 u/s this way; at 1 u/s it barely rose).
                    "setmaxspeed" => {
                        let p = source_assets::kv::parse_f32(&msg.param).unwrap_or(0.0).abs();
                        if p > 0.0 {
                            if let Some(Motion::Path { speed: s, .. }) = &mut m.motion {
                                *s = u(p);
                            }
                            let kv = &mut ents.list[i].keyvalues;
                            kv.retain(|(k, _)| k != "startspeed");
                            kv.push(("startspeed".into(), p.to_string()));
                        }
                    }
                    "setspeed" | "setspeedreal" | "setspeeddir" => {
                        let p = source_assets::kv::parse_f32(&msg.param).unwrap_or(0.0);
                        let want = if input == "setspeed" { speed * p } else { u(p.abs()) };
                        match &mut m.motion {
                            Some(Motion::Path { speed: s, .. }) => *s = want,
                            // A stopped train given a speed sets off along its path
                            // (the elevators' StartMoving script does this).
                            None if want > 0.0 => {
                                let nodes = path_from(&ents, m.node, None);
                                m.motion = Some(Motion::Path { nodes, speed: want, dest: None });
                                movers.riders.insert(i, None);
                            }
                            _ => {}
                        }
                    }
                    "runscriptcode" if msg.param.to_ascii_lowercase().contains("startmoving") && !scripted.0.contains(&i) => {
                        let nodes = path_from(&ents, m.node, None);
                        m.motion = Some(Motion::Path { nodes, speed, dest: None });
                        movers.riders.insert(i, None);
                    }
                    _ => {}
                }
            }
            "func_door" | "func_movelinear" => {
                let e = &ents.list[i];
                let speed = u(e.get_f32("speed").unwrap_or(100.0).max(1.0));
                let (dir, travel) = door_travel(e, &map);
                let Some(m) = movers.movers.get_mut(&i) else { continue };
                let open = m.home + dir * travel;
                match input {
                    "open" => m.motion = Some(Motion::To { target: open, speed, done: "onfullyopen" }),
                    "close" => m.motion = Some(Motion::To { target: m.home, speed, done: "onfullyclosed" }),
                    "toggle" => {
                        let closer_to_home = m.pos.distance(m.home) < m.pos.distance(open);
                        m.motion = Some(if closer_to_home {
                            Motion::To { target: open, speed, done: "onfullyopen" }
                        } else {
                            Motion::To { target: m.home, speed, done: "onfullyclosed" }
                        });
                    }
                    "setposition" => {
                        let p = source_assets::kv::parse_f32(&msg.param).unwrap_or(0.0);
                        m.motion = Some(Motion::To { target: m.home.lerp(open, p), speed, done: "onfullyopen" });
                    }
                    _ => continue,
                }
                let ev = if input == "close" { "onclose" } else { "onopen" };
                fire.write(FireOutput { ent: i, event: ev.into(), activator: msg.activator });
            }
            "prop_testchamber_door" | "prop_door_rotating" | "linked_portal_door" => match input {
                "open" => {
                    fire.write(FireOutput { ent: i, event: "OnOpen".into(), activator: msg.activator });
                    movers.pending_show.push((now + 0.35, i, false));
                }
                "close" => {
                    fire.write(FireOutput { ent: i, event: "OnClose".into(), activator: msg.activator });
                    movers.pending_show.push((now, i, true));
                }
                _ => {}
            },
            "prop_dynamic" | "prop_dynamic_override" => {
                // SetAnimation & co. are played by `anim_props.rs` (doors,
                // hatches and elevator bodies open for real, their per-bone
                // collision moving with them); they used to hide the door
                // or turn its collision off here.
                match input {
                    "turnoff" | "disable" => movers.pending_show.push((now, i, false)),
                    "turnon" | "enable" => movers.pending_show.push((now, i, true)),
                    "disablecollision" => set_solid(&mut commands, &ents.list[i], &children, &colliders, false),
                    "enablecollision" => set_solid(&mut commands, &ents.list[i], &children, &colliders, true),
                    _ => {}
                }
            }
            "func_brush" | "func_wall_toggle" | "func_illusionary" | "func_clip_vphysics" => match input {
                "enable" | "turnon" => movers.pending_show.push((now, i, true)),
                "disable" | "turnoff" => movers.pending_show.push((now, i, false)),
                "toggle" => {
                    let show = !ents.list[i].enabled;
                    ents.list[i].enabled = show;
                    movers.pending_show.push((now, i, show));
                }
                _ => {}
            },
            // The fizzler field goes with the cleanser (its trigger logic
            // reads `enabled`; it has no collision to switch).
            "trigger_portal_cleanser" => {
                if let (Some(r), "enable" | "disable") = (ents.list[i].entity, input) {
                    commands.entity(r).insert(if input == "enable" { Visibility::Inherited } else { Visibility::Hidden });
                }
            }
            _ => {}
        }
        if input == "kill" {
            if let Some(e) = ents.list[i].entity {
                commands.entity(e).despawn();
            }
        }
    }
    // Timed show/hide.
    let mut due = Vec::new();
    movers.pending_show.retain(|&(t, i, show)| {
        if t <= now {
            due.push((i, show));
            false
        } else {
            true
        }
    });
    for (i, show) in due {
        let e = &ents.list[i];
        // A test chamber door plays its open/close sequence (anim_props.rs)
        // and stays drawn; only its collision goes. Hiding it showed the
        // black area-portal window behind it mid-animation.
        if let (Some(r), false) = (e.entity, e.class == "prop_testchamber_door") {
            commands.entity(r).insert(if show { Visibility::Inherited } else { Visibility::Hidden });
        }
        set_solid(&mut commands, e, &children, &colliders, show);
        if e.class == "prop_testchamber_door" {
            fire.write(FireOutput { ent: i, event: if show { "OnFullyClosed".into() } else { "OnFullyOpen".into() }, activator: None });
        }
    }
}

/// Turn every collider under an entity on or off.
fn set_solid(commands: &mut Commands, e: &GameEnt, children: &Query<&Children>, colliders: &Query<(), With<Collider>>, solid: bool) {
    let Some(root) = e.entity else { return };
    for c in children.iter_descendants(root).chain(std::iter::once(root)) {
        if colliders.contains(c) {
            if solid {
                commands.entity(c).remove::<ColliderDisabled>();
            } else {
                commands.entity(c).insert(ColliderDisabled);
            }
        }
    }
}

/// Path nodes from `from` (exclusive) following `target`, up to `dest` (inclusive).
fn path_from(ents: &Ents, from: Option<usize>, dest: Option<usize>) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cur = from;
    if dest.is_some() && dest == from {
        return out;
    }
    for _ in 0..256 {
        let Some(c) = cur else { break };
        let next = ents.list[c].get("target").and_then(|t| ents.find(t, c, None).first().copied());
        let Some(n) = next else { break };
        if out.contains(&n) {
            break;
        }
        out.push(n);
        if Some(n) == dest {
            break;
        }
        cur = Some(n);
    }
    // A destination behind the train: go straight there.
    if let Some(d) = dest {
        if !out.contains(&d) {
            out = vec![d];
        }
    }
    out
}

/// A path node's yaw (Source degrees about up) as a world yaw in radians;
/// `None` for a node that also pitches or rolls (not blended yet).
fn node_yaw(e: &GameEnt) -> Option<f32> {
    let v: Vec<f32> = e.get("angles").unwrap_or("0 0 0").split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let at = |k: usize| v.get(k).copied().unwrap_or(0.0);
    (at(0) == 0.0 && at(2) == 0.0).then(|| at(1).to_radians())
}

/// The turn about up of a rotation, radians (positive turns left).
fn turn_yaw(q: Quat) -> f32 {
    let f = q * Vec3::NEG_Z;
    (-f.x).atan2(-f.z)
}

/// Yaw part way (`t` 0..1) from `a` to `b`, the short way round.
fn blend_yaw(a: f32, b: f32, t: f32) -> f32 {
    let d = (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + d * t.clamp(0.0, 1.0)
}

fn run_movers(
    time: Res<Time>,
    ents: Option<ResMut<Ents>>,
    mut movers: ResMut<Movers>,
    mut transforms: Query<&mut Transform>,
    mut lerp: ResMut<MoverLerp>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else { return };
    let dt = time.delta_secs();
    movers.moved.clear();
    movers.turned.clear();
    let keys: Vec<usize> = movers.movers.keys().copied().collect();
    for i in keys {
        let Some(m) = movers.movers.get_mut(&i) else { continue };
        let before = m.pos;
        let mut events: Vec<(usize, &'static str)> = Vec::new();
        match m.motion.clone() {
            None => {}
            Some(Motion::To { target, speed, done }) => {
                let to = target - m.pos;
                let step = speed * dt;
                if to.length() <= step {
                    m.pos = target;
                    m.motion = None;
                    events.push((i, done));
                } else {
                    m.pos += to.normalize() * step;
                }
            }
            Some(Motion::Path { mut nodes, speed, dest }) => {
                let mut step = speed * dt;
                while step > 0.0 {
                    let Some(&n) = nodes.first() else {
                        m.motion = None;
                        if dest.is_some() {
                            events.push((i, "onarrivedatdestinationnode"));
                        }
                        break;
                    };
                    let target = ents.list[n].transform.translation;
                    let to = target - m.pos;
                    if to.length() <= step {
                        step -= to.length();
                        m.pos = target;
                        m.node = Some(n);
                        nodes.remove(0);
                        events.push((n, "onpass"));
                        if Some(n) == dest || nodes.is_empty() {
                            m.motion = None;
                            if dest.is_some() {
                                events.push((i, "onarrivedatdestinationnode"));
                            }
                            break;
                        }
                    } else {
                        m.pos += to.normalize() * step;
                        step = 0.0;
                    }
                }
                if let Some(Motion::Path { nodes: n, .. }) = &mut m.motion {
                    *n = nodes;
                }
            }
        }
        // `orientationtype 2` (linear blend): the train's yaw goes from the
        // node it left to the angles of the node it heads for (the
        // sp_a1_wakeup breaker lift turns 90 degrees as it sets off).
        let turn = if ents.list[i].get("orientationtype") == Some("2") {
            let next = match &m.motion {
                Some(Motion::Path { nodes, .. }) => nodes.first().copied(),
                _ => None,
            };
            let yaw_of = |n: usize| node_yaw(&ents.list[n]);
            let yaw = match (m.node, next) {
                (Some(a), Some(b)) => {
                    let (pa, pb) = (ents.list[a].transform.translation, ents.list[b].transform.translation);
                    let len = pa.distance(pb);
                    let t = if len > 1e-5 { 1.0 - (m.pos.distance(pb) / len).min(1.0) } else { 1.0 };
                    yaw_of(a).zip(yaw_of(b)).map(|(ya, yb)| blend_yaw(ya, yb, t))
                }
                (Some(a), None) => yaw_of(a),
                _ => None,
            };
            let now = turn_yaw(ents.list[i].transform.rotation.normalize());
            yaw.map(|y| blend_yaw(now, y, 1.0) - now).filter(|d| d.abs() > 1e-5).map(Quat::from_rotation_y)
        } else {
            None
        };
        let delta = m.pos - before;
        if let Some(q) = turn {
            movers.turned.insert(i, q);
            let r = (q * ents.list[i].transform.rotation).normalize();
            ents.list[i].transform.rotation = r;
            if let Some(mut t) = ents.list[i].entity.and_then(|e| transforms.get_mut(e).ok()) {
                t.rotation = r;
            }
            movers.moved.entry(i).or_insert(Vec3::ZERO);
        }
        if delta != Vec3::ZERO {
            movers.moved.insert(i, delta);
            ents.list[i].transform.translation += delta;
            if let Some((lo, hi)) = &mut ents.list[i].bounds {
                *lo += delta;
                *hi += delta;
            }
            if let Some(r) = ents.list[i].entity {
                if let Ok(mut t) = transforms.get_mut(r) {
                    let before = t.translation;
                    t.translation += delta;
                    lerp.record(r, before, t.translation);
                }
            }
        }
        for (e, ev) in events {
            fire.write(FireOutput { ent: e, event: ev.into(), activator: None });
        }
    }
}

/// Parented entities keep their pose relative to a moving parent (a
/// `func_tank` child keeps its own turn on top, `tanks.rs`).
pub(crate) fn follow_parents(
    ents: Option<ResMut<Ents>>,
    mut movers: ResMut<Movers>,
    anims: Option<Res<crate::viewcontrol::PropAnims>>,
    mut tanks: Option<ResMut<crate::tanks::Tanks>>,
    mut transforms: Query<&mut Transform, Without<Player>>,
    mut lerp: ResMut<MoverLerp>,
) {
    let Some(mut ents) = ents else { return };
    // Parents that moved since last tick without being movers.
    let parents: Vec<usize> = movers.parented.iter().map(|p| p.1).collect();
    for p in parents {
        let now = ents.list[p].transform;
        match movers.parent_seen.insert(p, now) {
            Some(was) if !movers.moved.contains_key(&p) && (was.translation != now.translation || was.rotation != now.rotation) => {
                movers.moved.insert(p, now.translation - was.translation);
            }
            _ => {}
        }
    }
    if movers.moved.is_empty() {
        return;
    }
    let parented = movers.parented.clone();
    for (child, parent, offset) in parented {
        // Children on an animated attachment are placed by viewcontrol.
        if !movers.moved.contains_key(&parent) || anims.as_ref().is_some_and(|a| a.is_attached(child)) {
            continue;
        }
        let pt = ents.list[parent].transform;
        let world = Transform::from_translation(pt.translation + pt.rotation * offset.translation).with_rotation(pt.rotation * offset.rotation);
        let world = tanks.as_mut().map_or(world, |t| t.place(child, world));
        let delta = world.translation - ents.list[child].transform.translation;
        ents.list[child].transform = world;
        if let Some((lo, hi)) = &mut ents.list[child].bounds {
            *lo += delta;
            *hi += delta;
        }
        if let Some(r) = ents.list[child].entity {
            if let Ok(mut t) = transforms.get_mut(r) {
                lerp.record(r, t.translation, world.translation);
                t.translation = world.translation;
                t.rotation = world.rotation;
            }
        }
        // A moving train's children move its own children in turn.
        movers.moved.entry(child).or_insert(delta);
        movers.parent_seen.insert(child, world);
    }
}

/// Standing on something that moved, or inside a moving train (elevators):
/// move with it.
fn ride(
    ents: Option<Res<Ents>>,
    mut movers: ResMut<Movers>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &Grounded, &mut CharacterVelocity, &mut ViewAngles), With<Player>>,
    body_of: Query<&ColliderOf>,
    parents: Query<&ChildOf>,
) {
    let Some(ents) = ents else { return };
    let Ok((mut tf, mut feet, ground, mut vel, mut view)) = players.single_mut() else { return };
    // Elevator rides: locked to the car from its start to its stop.
    let riders: Vec<(usize, Option<Vec3>)> = movers.riders.iter().map(|(k, v)| (*k, *v)).collect();
    for (i, offset) in riders {
        let Some(m) = movers.movers.get(&i) else {
            movers.riders.remove(&i);
            continue;
        };
        match offset {
            None => {
                let d = tf.translation - m.pos;
                let flat = Vec2::new(d.x, d.z).length();
                if flat < 2.5 && d.y > -5.0 && d.y < 2.0 {
                    movers.riders.insert(i, Some(d));
                } else {
                    movers.riders.remove(&i);
                }
            }
            Some(d) => {
                // A turning car turns its rider about the car's origin.
                let (pos, stopped) = (m.pos, m.motion.is_none());
                let q = movers.turned.get(&i).copied();
                let d = q.map_or(d, |q| q * d);
                let at = pos + d;
                // Teleported out of the car (setpos, a trigger_teleport): let go.
                if tf.translation.distance(at) > 2.0 {
                    movers.riders.remove(&i);
                    continue;
                }
                tf.translation = at;
                *feet = InterpolatedFeet { prev: feet.curr, curr: at };
                vel.0 = Vec3::ZERO;
                if let Some(q) = q {
                    movers.riders.insert(i, Some(d));
                    view.yaw += turn_yaw(q);
                }
                if stopped {
                    movers.riders.remove(&i);
                }
                return;
            }
        }
    }
    if movers.moved.is_empty() {
        return;
    }
    // Only this tick's end moves: the view is drawn part way along the
    // carry, as the carrier is (`MoverLerp`).
    // A turn (`q`, about the carrier's origin `pivot`) turns the player
    // round the origin and their view with it.
    let apply = |d: Vec3, turn: Option<(Quat, Vec3)>, tf: &mut Transform, feet: &mut InterpolatedFeet, view: &mut ViewAngles| {
        let mut to = tf.translation + d;
        if let Some((q, pivot)) = turn {
            to = pivot + q * (to - pivot);
            view.yaw += turn_yaw(q);
        }
        feet.curr += to - tf.translation;
        tf.translation = to;
    };
    let turn_of = |i: usize| movers.turned.get(&i).map(|&q| (q, ents.list[i].transform.translation));
    // Inside a train's box (an elevator car): carried, standing or not.
    let p = tf.translation;
    for (&i, &d) in &movers.moved {
        let e = &ents.list[i];
        if !matches!(e.class.as_str(), "func_tracktrain" | "func_train") {
            continue;
        }
        let Some((lo, hi)) = e.bounds else { continue };
        let grow = Vec3::new(0.3, 0.0, 0.3);
        if p.x > lo.x - grow.x && p.x < hi.x + grow.x && p.z > lo.z - grow.z && p.z < hi.z + grow.z && p.y > lo.y - 0.5 && p.y < hi.y + 3.0 {
            apply(d, turn_of(i), &mut tf, &mut feet, &mut view);
            return;
        }
    }
    let Some(g) = ground.entity.filter(|_| ground.on_ground) else { return };
    // The collider's owning map entity: walk up from the collider.
    let mut cur = Some(g);
    for _ in 0..4 {
        let Some(c) = cur else { break };
        if let Some((i, _)) = ents.list.iter().enumerate().find(|(_, e)| e.entity == Some(c)) {
            if let Some(&d) = movers.moved.get(&i) {
                apply(d, turn_of(i), &mut tf, &mut feet, &mut view);
            }
            return;
        }
        cur = parents.get(c).ok().map(|p| p.parent()).or_else(|| body_of.get(c).ok().map(|b| b.body));
    }
}

#[cfg(test)]
mod lerp_tests {
    use super::*;

    #[test]
    fn train_yaw_blends_the_short_way_and_riders_turn_with_it() {
        // sp_a1_wakeup: path_1 (yaw 0) -> path_4 (yaw 270) is a quarter
        // turn right, not three quarters left.
        let (a, b) = (0f32.to_radians(), 270f32.to_radians());
        assert!((blend_yaw(a, b, 0.5).to_degrees() + 45.0).abs() < 1e-3);
        assert!((blend_yaw(a, b, 1.0).to_degrees() + 90.0).abs() < 1e-3);
        let q = Quat::from_rotation_y(0.3);
        assert!((turn_yaw(q) - 0.3).abs() < 1e-5);
        // Viewed yaw and body turn agree (ViewAngles: yaw about +Y).
        let v = ViewAngles { yaw: 0.3, ..default() };
        assert!((v.forward() - q * Vec3::NEG_Z).length() < 1e-5);
    }

    #[test]
    fn moved_twice_in_a_tick_keeps_the_first_start() {
        let mut l = MoverLerp::default();
        let e = Entity::from_raw_u32(7).unwrap();
        l.record(e, Vec3::ZERO, Vec3::Y);
        l.record(e, Vec3::Y, Vec3::Y * 3.0);
        assert_eq!(l.0[&e], (Vec3::ZERO, Vec3::Y * 3.0));
    }
}
