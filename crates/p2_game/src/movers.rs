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
            .add_systems(Update, (setup_movers, mover_inputs).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            // On the fixed tick, after the player moves and before physics
            // syncs colliders: the player is carried by exactly the step the
            // colliders make, so the next move sees them both in step.
            .add_systems(FixedUpdate, (run_movers, follow_parents, ride).chain().after(P2Systems::Transit));
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

fn setup_movers(mut loaded: MessageReader<LevelLoaded>, ents: Option<Res<Ents>>, mut movers: ResMut<Movers>) {
    if loaded.read().count() == 0 {
        return;
    }
    let Some(ents) = ents else { return };
    *movers = Movers::default();
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
                let dir = angles_dir(e, "movedir");
                let travel = if class == "func_movelinear" {
                    u(e.get_f32("movedistance").unwrap_or(0.0))
                } else {
                    let (lo, hi) = e.brush_model.and_then(|b| map.model_bounds.get(&b).copied()).unwrap_or((Vec3::ZERO, Vec3::ZERO));
                    let size = hi - lo;
                    (size * dir.abs()).length() - u(e.get_f32("lip").unwrap_or(0.0))
                };
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
        if let Some(r) = e.entity {
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

fn run_movers(
    time: Res<Time>,
    ents: Option<ResMut<Ents>>,
    mut movers: ResMut<Movers>,
    mut transforms: Query<&mut Transform>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else { return };
    let dt = time.delta_secs();
    movers.moved.clear();
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
        let delta = m.pos - before;
        if delta != Vec3::ZERO {
            movers.moved.insert(i, delta);
            ents.list[i].transform.translation += delta;
            if let Some((lo, hi)) = &mut ents.list[i].bounds {
                *lo += delta;
                *hi += delta;
            }
            if let Some(r) = ents.list[i].entity {
                if let Ok(mut t) = transforms.get_mut(r) {
                    t.translation += delta;
                }
            }
        }
        for (e, ev) in events {
            fire.write(FireOutput { ent: e, event: ev.into(), activator: None });
        }
    }
}

/// Parented entities keep their pose relative to a moving parent.
fn follow_parents(
    ents: Option<ResMut<Ents>>,
    mut movers: ResMut<Movers>,
    anims: Option<Res<crate::viewcontrol::PropAnims>>,
    mut transforms: Query<&mut Transform, Without<Player>>,
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
        let delta = world.translation - ents.list[child].transform.translation;
        ents.list[child].transform = world;
        if let Some((lo, hi)) = &mut ents.list[child].bounds {
            *lo += delta;
            *hi += delta;
        }
        if let Some(r) = ents.list[child].entity {
            if let Ok(mut t) = transforms.get_mut(r) {
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
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &Grounded, &mut CharacterVelocity), With<Player>>,
    body_of: Query<&ColliderOf>,
    parents: Query<&ChildOf>,
) {
    let Some(ents) = ents else { return };
    let Ok((mut tf, mut feet, ground, mut vel)) = players.single_mut() else { return };
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
                let at = m.pos + d;
                // Teleported out of the car (setpos, a trigger_teleport): let go.
                if tf.translation.distance(at) > 2.0 {
                    movers.riders.remove(&i);
                    continue;
                }
                tf.translation = at;
                *feet = InterpolatedFeet { prev: feet.curr, curr: at };
                vel.0 = Vec3::ZERO;
                if m.motion.is_none() {
                    movers.riders.remove(&i);
                }
                return;
            }
        }
    }
    if movers.moved.is_empty() {
        return;
    }
    let apply = |d: Vec3, tf: &mut Transform, feet: &mut InterpolatedFeet| {
        tf.translation += d;
        feet.prev += d;
        feet.curr += d;
    };
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
            apply(d, &mut tf, &mut feet);
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
                apply(d, &mut tf, &mut feet);
            }
            return;
        }
        cur = parents.get(c).ok().map(|p| p.parent()).or_else(|| body_of.get(c).ok().map(|b| b.body));
    }
}
