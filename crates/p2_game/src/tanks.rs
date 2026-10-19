//! `func_tank` (and the other `func_tank*` classes): brush entities that
//! turn toward a target. Wheatley's rail in sp_a1_intro1 hangs from two:
//! `top_swivel_1` yaws (600 deg/s, any way round), `bottom_swivel_1` yaws
//! and pitches (300 deg/s) toward `camera_1`, the player's view.
//!
//! A tank turns about its own origin, relative to the pose it spawned in
//! (or the pose its parent puts it in): yaw within +-`yawrange` (360 and
//! more: all the way round), pitch within +-`pitchrange`, at `yawrate` /
//! `pitchrate` degrees per second, aiming its barrel (`barrel`,
//! `barrely`, `barrelz` offsets) at the target. Inputs: `Activate`,
//! `Deactivate`, `SetTargetEntity` / `SetTargetEntityName` (a
//! `point_viewproxy`, `point_viewcontrol` or `!player` means the player's
//! eye), `SetTargetPosition`, `SetTargetDir`, `ClearTargetEntity` /
//! `ClearTarget`. Spawnflag 1 starts it active. Its children (map
//! parenting, `movers.rs` `follow_parents`) turn with it.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_core::*;
use source_assets::entities::source_angles_to_world;
use shared::{source_dir_to_world, source_to_world, SOURCE_UNIT};

use crate::io::{EntInput, Ents, GameEnt};
use crate::level::LevelLoaded;

pub struct TanksPlugin;

impl Plugin for TanksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tanks>()
            .add_systems(Update, tank_inputs.after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            // Before parenting moves the tanks' children, so they follow the
            // aim the same tick.
            .add_systems(FixedUpdate, run_tanks.after(P2Systems::Transit).before(crate::movers::follow_parents))
            // A tank on an animated attachment is put back unturned there
            // each frame: turn it again.
            .add_systems(Update, reapply_tanks.after(crate::viewcontrol::follow_attachments).in_set(crate::level::GameSet::Logic));
    }
}

/// What a tank aims at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TankTarget {
    /// The player's eye.
    Player,
    /// An entity's origin.
    Ent(usize),
    /// A world point (metres).
    Point(Vec3),
    /// A world direction.
    Dir(Vec3),
}

#[derive(Clone, Debug)]
pub struct Tank {
    pub active: bool,
    pub target: Option<TankTarget>,
    /// Current turn from the base pose, degrees (yaw left positive, pitch
    /// up positive).
    pub yaw: f32,
    pub pitch: f32,
    pub yaw_rate: f32,
    pub yaw_range: f32,
    pub pitch_rate: f32,
    pub pitch_range: f32,
    /// Barrel offset in the tank's own Source axes (units).
    pub barrel: Vec3,
    /// The unturned world pose (spawn pose, or where its parent put it).
    base: Transform,
    /// The pose last written, to tell whether something else moved it.
    written: Option<Transform>,
}

impl Tank {
    pub fn from_ent(e: &GameEnt) -> Self {
        let f = |k: &str, d: f32| e.get_f32(k).unwrap_or(d);
        Tank {
            active: e.spawnflags & 1 != 0,
            target: None,
            yaw: 0.0,
            pitch: 0.0,
            yaw_rate: f("yawrate", 30.0),
            yaw_range: f("yawrange", 180.0),
            pitch_rate: f("pitchrate", 30.0),
            pitch_range: f("pitchrange", 60.0),
            barrel: Vec3::new(f("barrel", 0.0), f("barrely", 0.0), f("barrelz", 0.0)),
            base: e.transform,
            written: None,
        }
    }

    /// The turn as a rotation in the tank's own (world-axes) frame.
    pub fn turn(&self) -> Quat {
        aim_rotation(self.yaw, self.pitch)
    }

    /// The world pose for an unturned `base`: remembers it and returns it turned.
    pub fn place(&mut self, base: Transform) -> Transform {
        self.base = base;
        let w = Transform { rotation: (base.rotation * self.turn()).normalize(), ..base };
        self.written = Some(w);
        w
    }

    /// The unturned pose now: the remembered one, unless something else
    /// (parenting, an attachment) has put the tank somewhere since.
    fn base_now(&self, current: Transform) -> Transform {
        match self.written {
            Some(w) if same_pose(w, current) => self.base,
            _ => current,
        }
    }
}

fn same_pose(a: Transform, b: Transform) -> bool {
    a.translation.distance_squared(b.translation) < 1e-10 && a.rotation.dot(b.rotation).abs() > 1.0 - 1e-7
}

/// The rotation turning a tank by `yaw` (left) and `pitch` (up) degrees,
/// in world axes: Source angles (pitch down positive) through the axis change.
pub fn aim_rotation(yaw: f32, pitch: f32) -> Quat {
    source_angles_to_world(Vec3::new(-pitch, yaw, 0.0))
}

/// Yaw and pitch (degrees, left / up positive) of a world point seen from
/// `from` in the frame of `base` (its unturned pose); `None` at the origin.
pub fn angles_to(base: Quat, from: Vec3, to: Vec3) -> Option<(f32, f32)> {
    angles_along(base, to - from)
}

/// Yaw and pitch of a world direction in the frame of `base`.
pub fn angles_along(base: Quat, dir: Vec3) -> Option<(f32, f32)> {
    let d = base.inverse() * dir;
    // World axes back to Source: x forward, y left (= -z), z up (= y).
    let (x, y, z) = (d.x, -d.z, d.y);
    if x * x + y * y + z * z < 1e-12 {
        return None;
    }
    Some((y.atan2(x).to_degrees(), z.atan2((x * x + y * y).sqrt()).to_degrees()))
}

/// Wraps an angle into (-180, 180].
fn wrap(a: f32) -> f32 {
    let a = a.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// One axis of a tank: from `cur` toward `want` by at most `step` degrees,
/// kept within +-`range` (a range of 180 or more turns freely, the short way
/// round).
pub fn turn_axis(cur: f32, want: f32, step: f32, range: f32) -> f32 {
    let free = range >= 180.0;
    let want = if free { want } else { want.clamp(-range, range) };
    let delta = if free { wrap(want - cur) } else { want - cur };
    let next = cur + delta.clamp(-step.max(0.0), step.max(0.0));
    if free { wrap(next) } else { next.clamp(-range, range) }
}

/// Run-time state of every tank, by entity index.
#[derive(Resource, Default)]
pub struct Tanks(pub HashMap<usize, Tank>);

impl Tanks {
    /// Where a tank goes when its parent puts its unturned pose at `base`
    /// (`follow_parents`); any other entity stays at `base`.
    pub fn place(&mut self, ent: usize, base: Transform) -> Transform {
        match self.0.get_mut(&ent) {
            Some(t) => t.place(base),
            None => base,
        }
    }
}

pub fn is_tank(class: &str) -> bool {
    class.starts_with("func_tank")
}

/// Targets that stand for the player's view.
fn means_player(e: &GameEnt) -> bool {
    matches!(e.class.as_str(), "player" | "point_viewproxy" | "point_viewcontrol")
}

fn parse_vec(s: &str) -> Option<Vec3> {
    let v: Vec<f32> = s.split(|c: char| c.is_whitespace() || c == ',').filter_map(|x| x.parse().ok()).collect();
    (v.len() >= 3).then(|| Vec3::new(v[0], v[1], v[2]))
}

fn tank_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut tanks: ResMut<Tanks>, mut loaded: MessageReader<LevelLoaded>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    if loaded.read().count() > 0 {
        tanks.0 = ents.list.iter().enumerate().filter(|(_, e)| is_tank(&e.class)).map(|(i, e)| (i, Tank::from_ent(e))).collect();
        if !tanks.0.is_empty() {
            debug!("portal2: {} func_tanks", tanks.0.len());
        }
    }
    for msg in inputs.read() {
        let Some(t) = tanks.0.get_mut(&msg.target) else { continue };
        let param = msg.param.trim();
        match msg.input.as_str() {
            "activate" => t.active = true,
            "deactivate" => t.active = false,
            "settargetentity" | "settargetentityname" => {
                t.target = ents.find(param, msg.target, msg.activator).first().map(|&i| if i == crate::io::PLAYER || means_player(&ents.list[i]) { TankTarget::Player } else { TankTarget::Ent(i) });
            }
            "settargetposition" => t.target = parse_vec(param).map(|p| TankTarget::Point(source_to_world(p))),
            "settargetdir" => t.target = parse_vec(param).map(|d| TankTarget::Dir(source_dir_to_world(d))),
            "cleartargetentity" | "cleartarget" => t.target = None,
            _ => {}
        }
    }
}

/// The player's eye: the main camera.
fn player_eye(cams: &Query<&GlobalTransform, With<MainCamera>>, players: &Query<&Transform, With<Player>>) -> Option<Vec3> {
    cams.iter().next().map(|c| c.translation()).or_else(|| players.iter().next().map(|p| p.translation + Vec3::Y * VIEW_STAND))
}

/// Turns active tanks toward their targets.
fn run_tanks(
    time: Res<Time>,
    ents: Option<ResMut<Ents>>,
    mut tanks: ResMut<Tanks>,
    mut transforms: Query<&mut Transform, (Without<Player>, Without<MainCamera>)>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    players: Query<&Transform, With<Player>>,
) {
    let Some(mut ents) = ents else { return };
    let dt = time.delta_secs();
    let eye = player_eye(&cams, &players);
    for (&i, t) in tanks.0.iter_mut() {
        let base = t.base_now(ents.list[i].transform);
        if t.active {
            let from = base.translation + base.rotation * (source_dir_to_world(t.barrel) * SOURCE_UNIT);
            let want = match t.target {
                Some(TankTarget::Player) => eye.and_then(|p| angles_to(base.rotation, from, p)),
                Some(TankTarget::Ent(e)) => angles_to(base.rotation, from, ents.list[e].transform.translation),
                Some(TankTarget::Point(p)) => angles_to(base.rotation, from, p),
                Some(TankTarget::Dir(d)) => angles_along(base.rotation, d),
                None => None,
            };
            if let Some((yaw, pitch)) = want {
                t.yaw = turn_axis(t.yaw, yaw, t.yaw_rate * dt, t.yaw_range);
                t.pitch = turn_axis(t.pitch, pitch, t.pitch_rate * dt, t.pitch_range);
            }
        }
        let world = t.place(base);
        write_pose(&mut ents.list[i], world, &mut transforms);
    }
}

fn write_pose(e: &mut GameEnt, world: Transform, transforms: &mut Query<&mut Transform, (Without<Player>, Without<MainCamera>)>) {
    if same_pose(e.transform, world) {
        return;
    }
    e.transform = Transform { scale: e.transform.scale, ..world };
    if let Some(r) = e.entity {
        if let Ok(mut tf) = transforms.get_mut(r) {
            tf.translation = world.translation;
            tf.rotation = world.rotation;
        }
    }
}

/// Tanks riding animated attachments: `follow_attachments` put them back
/// at their unturned pose this frame; turn them again.
fn reapply_tanks(ents: Option<ResMut<Ents>>, mut tanks: ResMut<Tanks>, mut transforms: Query<&mut Transform, (Without<Player>, Without<MainCamera>)>) {
    let Some(mut ents) = ents else { return };
    for (&i, t) in tanks.0.iter_mut() {
        let current = ents.list[i].transform;
        if t.written.is_some_and(|w| same_pose(w, current)) {
            continue;
        }
        let world = t.place(current);
        write_pose(&mut ents.list[i], world, &mut transforms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn angles_in_the_tanks_frame() {
        let id = Quat::IDENTITY;
        // Source +X ahead, +Y (world -Z) left, +Z (world +Y) up.
        assert_eq!(angles_along(id, Vec3::X).map(|(y, p)| (close(y, 0.0), close(p, 0.0))), Some((true, true)));
        let (y, p) = angles_along(id, Vec3::NEG_Z).unwrap();
        assert!(close(y, 90.0) && close(p, 0.0), "{y} {p}");
        let (y, p) = angles_to(id, Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)).unwrap();
        assert!(close(y, 0.0) && close(p, 45.0), "{y} {p}");
        // A tank spawned facing Source +Y (yaw 90) sees Source +Y dead ahead.
        let base = source_angles_to_world(Vec3::new(0.0, 90.0, 0.0));
        let (y, p) = angles_along(base, source_dir_to_world(Vec3::Y)).unwrap();
        assert!(close(y, 0.0) && close(p, 0.0), "{y} {p}");
        assert_eq!(angles_to(id, Vec3::ONE, Vec3::ONE), None);
    }

    #[test]
    fn aim_rotation_points_the_barrel() {
        for (yaw, pitch) in [(0.0, 0.0), (90.0, 0.0), (-30.0, 20.0), (170.0, -45.0)] {
            let fwd = aim_rotation(yaw, pitch) * Vec3::X;
            let (y, p) = angles_along(Quat::IDENTITY, fwd).unwrap();
            assert!(close(y, yaw) && close(p, pitch), "{yaw} {pitch}: {y} {p}");
        }
    }

    #[test]
    fn turns_at_its_rate_within_its_range() {
        // 30 deg/s for 0.5 s toward 90: 15.
        assert!(close(turn_axis(0.0, 90.0, 15.0, 60.0), 15.0));
        // Stops at the range limit.
        assert!(close(turn_axis(55.0, 90.0, 15.0, 60.0), 60.0));
        assert!(close(turn_axis(0.0, -170.0, 500.0, 60.0), -60.0));
        // Arrives without overshooting.
        assert!(close(turn_axis(80.0, 90.0, 15.0, 60.0 + 100.0), 90.0));
        // A full-range tank goes the short way round, across 180.
        assert!(close(turn_axis(170.0, -170.0, 15.0, 360.0), -175.0));
        // Rate 0 never turns.
        assert!(close(turn_axis(10.0, 50.0, 0.0, 90.0), 10.0));
    }

    #[test]
    fn placing_turns_the_base_and_spots_outside_moves() {
        let e = GameEnt { spawnflags: 32769, ..GameEnt::logic("func_tank", "bottom_swivel_1", vec![("yawrate".into(), "300".into()), ("pitchrate".into(), "300".into()), ("yawrange".into(), "360".into())]) };
        let mut t = Tank::from_ent(&e);
        assert!(t.active);
        assert_eq!((t.yaw_rate, t.pitch_rate, t.yaw_range), (300.0, 300.0, 360.0));
        t.yaw = 90.0;
        let base = Transform::from_xyz(1.0, 2.0, 3.0);
        let w = t.place(base);
        assert_eq!(w.translation, base.translation);
        assert!((w.rotation * Vec3::X - Vec3::NEG_Z).length() < 1e-5, "yawed left");
        // Unmoved since: the base is remembered.
        assert_eq!(t.base_now(w).translation, base.translation);
        assert!(same_pose(t.base_now(w), base));
        // Moved by a parent: that is the new base.
        let moved = Transform::from_xyz(5.0, 0.0, 0.0);
        assert!(same_pose(t.base_now(moved), moved));
        // A child offset turns with the tank (Wheatley under the swivel).
        let child_offset = Vec3::new(0.0, -1.0, 0.5);
        let child = w.translation + w.rotation * child_offset;
        assert!((child - (base.translation + Vec3::new(0.5, -1.0, 0.0))).length() < 1e-5, "{child}");
    }

    #[test]
    fn tanks_place_only_tanks() {
        let mut tanks = Tanks::default();
        let mut t = Tank::from_ent(&GameEnt::logic("func_tank", "t", vec![]));
        t.pitch = 30.0;
        tanks.0.insert(4, t);
        let base = Transform::from_xyz(0.0, 1.0, 0.0);
        assert!(same_pose(tanks.place(3, base), base));
        let w = tanks.place(4, base);
        let (_, p) = angles_along(Quat::IDENTITY, w.rotation * Vec3::X).unwrap();
        assert!(close(p, 30.0));
        assert!(is_tank("func_tank") && is_tank("func_tankairboatgun") && !is_tank("func_door"));
        assert_eq!(parse_vec("1 2.5 -3"), Some(Vec3::new(1.0, 2.5, -3.0)));
    }
}
