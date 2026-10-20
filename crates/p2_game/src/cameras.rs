//! Security cameras (`npc_security_camera`). Portal 2's camera has a
//! `Ragdoll` input that tears it off its wall mount; a portal shot that hits
//! it, or a portal opened over the wall it hangs on, does the same. Torn off,
//! it is an ordinary physics prop (it falls, and the player can carry it).
use std::collections::HashSet;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents};
use crate::level::LevelLoaded;
use crate::props::insert_prop_body;

const U: f32 = shared::SOURCE_UNIT;
/// The camera body: centre of security_camera.mdl's mesh bounds (model
/// space, Source axes) and a radius around it that covers the arm and lens.
const BODY_CENTRE: Vec3 = Vec3::new(-5.0, -17.0, -8.0);
const BODY_RADIUS: f32 = 20.0 * U;
/// How far in front of its wall a camera hangs.
const SHOT_REACH: f32 = 32.0 * U;
/// How far around a new portal's opening a wall-mounted camera is knocked off.
const PORTAL_MARGIN: f32 = 16.0 * U;

pub struct CamerasPlugin;

impl Plugin for CamerasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Detached>().add_systems(
            Update,
            detach_cameras.after(crate::io::run_io).in_set(crate::level::GameSet::Logic),
        );
    }
}

/// Cameras already torn off this level.
#[derive(Resource, Default)]
struct Detached(HashSet<usize>);

/// Whether a shot from `from` to `to` passes through a camera body centred
/// at `body`. The gun's trace can slip past its thin arm and run on to
/// the wall behind it.
pub fn shot_hits_camera(from: Vec3, to: Vec3, body: Vec3) -> bool {
    let d = to - from;
    let t = if d.length_squared() > 1e-9 { ((body - from).dot(d) / d.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
    (from + d * t).distance(body) <= BODY_RADIUS
}

/// Whether a portal opened at `frame` covers a camera mounted at `camera`
/// (on the same wall, within the opening plus a margin).
pub fn portal_covers_camera(frame: &PortalFrame, camera: Vec3) -> bool {
    let l = frame.to_local(camera);
    l.x.abs() <= PORTAL_HALF_SIZE.x + PORTAL_MARGIN && l.y.abs() <= PORTAL_HALF_SIZE.y + PORTAL_MARGIN && (-8.0 * U..=SHOT_REACH).contains(&l.z)
}

#[allow(clippy::too_many_arguments)]
fn detach_cameras(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    mut inputs: MessageReader<EntInput>,
    mut fired: MessageReader<GunFired>,
    mut placed: MessageReader<PortalPlaced>,
    ents: Option<Res<Ents>>,
    mut detached: ResMut<Detached>,
    transforms: Query<&Transform>,
    children: Query<&Children>,
    aabbs: Query<&ColliderAabb>,
    colliders: Query<&Collider>,
) {
    if loaded.read().count() > 0 {
        detached.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        fired.clear();
        placed.clear();
        return;
    };
    let mut knock: Vec<usize> = inputs
        .read()
        .filter(|m| ents.list[m.target].class == "npc_security_camera" && m.input == "ragdoll")
        .map(|m| m.target)
        .collect();
    let cameras: Vec<usize> = (0..ents.list.len())
        .filter(|&i| ents.list[i].class == "npc_security_camera" && !ents.list[i].killed && !detached.0.contains(&i))
        .collect();
    let pose = |i: usize| ents.list[i].entity.and_then(|e| transforms.get(e).ok().copied()).unwrap_or(ents.list[i].transform);
    let at = |i: usize| pose(i).translation;
    let body = |i: usize| pose(i).transform_point(shared::source_to_world(BODY_CENTRE));
    for f in fired.read() {
        knock.extend(cameras.iter().copied().filter(|&i| shot_hits_camera(f.from, f.to, body(i))));
    }
    for p in placed.read() {
        let frame = Portal::frame(&p.transform);
        knock.extend(cameras.iter().copied().filter(|&i| portal_covers_camera(&frame, at(i))));
    }
    for i in knock {
        let Some(entity) = ents.list[i].entity else { continue };
        if !detached.0.insert(i) {
            continue;
        }
        let tf = transforms.get(entity).copied().unwrap_or(ents.list[i].transform);
        debug!("portal2: security camera {i} detached at {:.0?}", tf.translation / U);
        insert_prop_body(&mut commands, i, entity, tf, false, &children, &aabbs, &colliders);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shot_near_camera_hits_it() {
        let cam = Vec3::new(1.0, 2.0, 3.0);
        // Straight through it, on to the wall behind.
        assert!(shot_hits_camera(cam - Vec3::X * 300.0 * U, cam + Vec3::X * 70.0 * U, cam));
        // Passing well beside it.
        assert!(!shot_hits_camera(cam - Vec3::X * 300.0 * U + Vec3::Y * 40.0 * U, cam + Vec3::X * 70.0 * U + Vec3::Y * 40.0 * U, cam));
        // Stopped by a wall before reaching it.
        assert!(!shot_hits_camera(cam - Vec3::X * 300.0 * U, cam - Vec3::X * 100.0 * U, cam));
    }

    #[test]
    fn portal_over_camera_wall_knocks_it_off() {
        // A wall facing +X, portal centred at the origin.
        let frame = PortalFrame::from_normal_up(Vec3::ZERO, Vec3::X, Vec3::Y).unwrap();
        // Camera on the wall, beside the opening, sticking out 8 u.
        assert!(portal_covers_camera(&frame, Vec3::new(8.0 * U, 40.0 * U, 30.0 * U)));
        // Far along the wall: untouched.
        assert!(!portal_covers_camera(&frame, Vec3::new(8.0 * U, 0.0, 120.0 * U)));
        // Across the room in front of the portal: untouched.
        assert!(!portal_covers_camera(&frame, Vec3::new(200.0 * U, 0.0, 0.0)));
        // Behind the wall: untouched.
        assert!(!portal_covers_camera(&frame, Vec3::new(-40.0 * U, 0.0, 0.0)));
    }
}
