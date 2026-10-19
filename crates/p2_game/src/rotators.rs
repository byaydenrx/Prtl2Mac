//! Brush entities that turn about their origin: `func_door_rotating`
//! (swing doors, gates, the gel pump levers), `func_rot_button` (breaker
//! knobs) and `func_rotating` (fans, wheels).
//!
//! Doors and buttons turn `distance` degrees at `speed` deg/s about their
//! own Z (yaw), or X with spawnflag 64 / Y with 128; spawnflag 2 turns
//! them the other way. A door fires OnOpen / OnClose when it starts and
//! OnFullyOpen / OnFullyClosed when it gets there; with a `wait` of 0 or
//! more (and no Toggle flag, 32) it swings back after that many seconds.
//! `spawnpos 1` starts it open. Use (flag 256) opens it, or closes an open
//! toggle door. A rot button pressed in (`PressIn`, or used) turns in and
//! fires OnIn on arrival, and comes back after `wait` (-1: stays in).
//!
//! `func_rotating` spins at `maxspeed` deg/s about Z (X with flag 4, Y with
//! 8, reversed with 2), starting on with flag 1, speeding up and down over
//! about a second (`fanfriction`). Children (map parenting) turn with them
//! through `movers.rs` `follow_parents`.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_core::*;
use source_assets::entities::source_angles_to_world;

use crate::io::{EntInput, Ents, FireOutput, GameEnt};
use crate::level::LevelLoaded;

pub struct RotatorsPlugin;

impl Plugin for RotatorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Rotators>()
            .add_systems(Update, rotator_inputs.after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            .add_systems(FixedUpdate, run_rotators.after(P2Systems::Transit).before(crate::movers::follow_parents));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Door,
    Button,
    Spinner,
}

#[derive(Clone, Debug)]
pub struct Rotator {
    kind: Kind,
    /// Closed (spawn) pose.
    base: Transform,
    /// Turn axis in the entity's own Source axes (pitch, yaw, roll unit).
    axis: Vec3,
    /// Current turn, degrees.
    angle: f32,
    /// Where it is heading (doors, buttons), degrees.
    target: f32,
    /// Fully open / pressed-in angle.
    open: f32,
    /// Degrees per second (doors, buttons); top speed (spinners).
    speed: f32,
    /// Spinner: current and wanted speed (signed deg/s).
    spin: f32,
    spin_want: f32,
    /// Seconds to wait before swinging back (-1: never).
    wait: f32,
    /// Time to swing back, if due.
    return_at: Option<f32>,
    /// Event fired on arrival.
    done: Option<&'static str>,
}

impl Rotator {
    fn from_ent(e: &GameEnt) -> Option<Self> {
        let flags = e.spawnflags;
        let kind = match e.class.as_str() {
            "func_door_rotating" => Kind::Door,
            "func_rot_button" => Kind::Button,
            "func_rotating" => Kind::Spinner,
            _ => return None,
        };
        // Source angles order: (pitch = about Y, yaw = about Z, roll = about X).
        let axis = if kind == Kind::Spinner {
            if flags & 4 != 0 {
                Vec3::Z
            } else if flags & 8 != 0 {
                Vec3::X
            } else {
                Vec3::Y
            }
        } else if flags & 64 != 0 {
            Vec3::Z
        } else if flags & 128 != 0 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let reverse = flags & 2 != 0;
        let sign = if reverse { -1.0 } else { 1.0 };
        let f = |k: &str, d: f32| e.get_f32(k).unwrap_or(d);
        let mut r = Rotator {
            kind,
            base: e.transform,
            axis,
            angle: 0.0,
            target: 0.0,
            open: f("distance", 90.0) * sign,
            speed: f("speed", 100.0).abs().max(1.0),
            spin: 0.0,
            spin_want: 0.0,
            wait: f("wait", if kind == Kind::Door { 4.0 } else { 1.0 }),
            return_at: None,
            done: None,
        };
        if kind == Kind::Spinner {
            r.speed = f("maxspeed", 100.0).abs();
            r.open = sign;
            if flags & 1 != 0 {
                r.spin_want = r.speed * sign;
                r.spin = r.spin_want;
            }
        }
        if kind == Kind::Door && e.get("spawnpos") == Some("1") {
            r.angle = r.open;
            r.target = r.open;
        }
        Some(r)
    }

    fn pose(&self) -> Transform {
        let turn = source_angles_to_world(self.axis * self.angle);
        Transform { rotation: (self.base.rotation * turn).normalize(), ..self.base }
    }

    fn is_open(&self) -> bool {
        self.target != 0.0
    }

    fn go(&mut self, to_open: bool) -> &'static str {
        self.target = if to_open { self.open } else { 0.0 };
        self.return_at = None;
        self.done = Some(match (self.kind, to_open) {
            (Kind::Button, true) => "onin",
            (Kind::Button, false) => "onout",
            (_, true) => "onfullyopen",
            (_, false) => "onfullyclosed",
        });
        if to_open { "onopen" } else { "onclose" }
    }
}

#[derive(Resource, Default)]
pub struct Rotators(pub HashMap<usize, Rotator>);

fn rotator_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut rot: ResMut<Rotators>,
    mut loaded: MessageReader<LevelLoaded>,
    mut fire: MessageWriter<FireOutput>,
    mut transforms: Query<&mut Transform, Without<Player>>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    if loaded.read().count() > 0 {
        rot.0 = ents.list.iter().enumerate().filter_map(|(i, e)| Rotator::from_ent(e).map(|r| (i, r))).collect();
        // Doors that start open.
        for (&i, r) in &rot.0 {
            if r.angle != 0.0 {
                if let Some(mut tf) = ents.list[i].entity.and_then(|x| transforms.get_mut(x).ok()) {
                    tf.rotation = r.pose().rotation;
                }
            }
        }
    }
    for msg in inputs.read() {
        let i = msg.target;
        let Some(r) = rot.0.get_mut(&i) else { continue };
        let e = &ents.list[i];
        let locked = e.value == 0.0 && r.kind != Kind::Spinner;
        let started = match (r.kind, msg.input.as_str()) {
            (Kind::Door, "open") => Some(r.go(true)),
            (Kind::Door, "close") => Some(r.go(false)),
            (Kind::Door, "toggle") => {
                let open = !r.is_open();
                Some(r.go(open))
            }
            (Kind::Door, "use") if !locked => {
                if !r.is_open() {
                    Some(r.go(true))
                } else if e.spawnflags & 32 != 0 {
                    Some(r.go(false))
                } else {
                    None
                }
            }
            (Kind::Door, "setspeed") => {
                r.speed = msg.param.trim().parse::<f32>().unwrap_or(r.speed).abs().max(1.0);
                None
            }
            // OnPressed is fired by io.rs (inputs) or props.rs (use).
            (Kind::Button, "pressin" | "press" | "use") if !locked || msg.input != "use" => {
                r.go(true);
                None
            }
            (Kind::Button, "pressout") => {
                r.go(false);
                // io.rs already fired OnOut.
                r.done = None;
                None
            }
            (Kind::Spinner, input) => {
                let dir = if r.open < 0.0 { -1.0 } else { 1.0 };
                match input {
                    "start" | "startforward" => r.spin_want = r.speed * dir,
                    "startbackward" => r.spin_want = -r.speed * dir,
                    "stop" | "stopatstartpos" => r.spin_want = 0.0,
                    "toggle" => r.spin_want = if r.spin_want == 0.0 { r.speed * dir } else { 0.0 },
                    "reverse" => r.spin_want = -r.spin_want,
                    "setspeed" => {
                        let p = msg.param.trim().parse::<f32>().unwrap_or(1.0);
                        let s = if r.spin_want < 0.0 { -1.0 } else { dir };
                        r.spin_want = r.speed * p * s;
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        };
        if let Some(ev) = started {
            fire.write(FireOutput { ent: i, event: ev.into(), activator: msg.activator });
        }
    }
}

fn run_rotators(
    time: Res<Time>,
    ents: Option<ResMut<Ents>>,
    mut rot: ResMut<Rotators>,
    mut transforms: Query<&mut Transform, Without<Player>>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else { return };
    let dt = time.delta_secs();
    let now = ents.now;
    for (&i, r) in rot.0.iter_mut() {
        let before = r.angle;
        match r.kind {
            Kind::Spinner => {
                // fanfriction: percent of top speed lost per... about a second either way.
                let accel = r.speed.max(1.0);
                r.spin += (r.spin_want - r.spin).clamp(-accel * dt, accel * dt);
                r.angle = (r.angle + r.spin * dt).rem_euclid(360.0);
            }
            _ => {
                if r.angle != r.target {
                    let step = r.speed * dt;
                    let d = r.target - r.angle;
                    if d.abs() <= step {
                        r.angle = r.target;
                        if let Some(ev) = r.done.take() {
                            fire.write(FireOutput { ent: i, event: ev.into(), activator: None });
                        }
                        // Swing back after `wait` (doors without the toggle flag, buttons).
                        let toggles = ents.list[i].spawnflags & 32 != 0;
                        if r.target != 0.0 && r.wait >= 0.0 && !toggles {
                            r.return_at = Some(now + r.wait);
                        }
                    } else {
                        r.angle += step * d.signum();
                    }
                } else if r.return_at.is_some_and(|t| now >= t) {
                    let ev = r.go(false);
                    if r.kind == Kind::Door {
                        fire.write(FireOutput { ent: i, event: ev.into(), activator: None });
                    }
                }
            }
        }
        if r.angle == before {
            continue;
        }
        let world = r.pose();
        let e = &mut ents.list[i];
        e.transform.rotation = world.rotation;
        if let Some(mut tf) = e.entity.and_then(|x| transforms.get_mut(x).ok()) {
            tf.rotation = world.rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(class: &str, kv: &[(&str, &str)], flags: u32) -> GameEnt {
        let mut e = GameEnt::logic(class, "r", kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        e.spawnflags = flags;
        e
    }

    #[test]
    fn door_yaws_about_its_origin() {
        let e = ent("func_door_rotating", &[("distance", "90"), ("speed", "90")], 0);
        let mut r = Rotator::from_ent(&e).unwrap();
        r.angle = 90.0;
        // Source +X turned 90 degrees of yaw points along Source +Y (world -Z).
        let f = r.pose().rotation * Vec3::X;
        assert!((f - Vec3::NEG_Z).length() < 1e-5, "{f}");
    }

    #[test]
    fn reverse_and_axes() {
        let e = ent("func_door_rotating", &[("distance", "70")], 2 | 128);
        let r = Rotator::from_ent(&e).unwrap();
        assert_eq!(r.open, -70.0);
        assert_eq!(r.axis, Vec3::X);
        let s = Rotator::from_ent(&ent("func_rotating", &[("maxspeed", "135")], 1 | 4)).unwrap();
        assert_eq!((s.axis, s.spin), (Vec3::Z, 135.0));
    }
}
