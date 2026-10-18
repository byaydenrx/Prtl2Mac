//! Excursion funnels (`prop_tractor_beam`): a tube of light from the
//! emitter, through linked portals, to the first wall. Anything inside
//! floats along it at `linearforce` units per second (negative: back toward
//! the emitter), drawn toward its middle; the player can drift out sideways.
use std::collections::HashSet;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents};
use crate::level::{GameSet, LevelLoaded, LevelOwned};
use crate::props::{Held, Prop};

pub struct TractorPlugin;

impl Plugin for TractorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Funnels>().add_systems(Update, (funnel_inputs, trace_funnels, carry_in_funnels).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

const U: f32 = shared::SOURCE_UNIT;
/// Radius of the tube.
const RADIUS: f32 = 54.0 * U;
const RANGE: f32 = 8192.0 * U;
const MAX_LEGS: usize = 8;

#[derive(Clone, Copy, Debug)]
struct Leg {
    a: Vec3,
    b: Vec3,
}

#[derive(Resource, Default)]
struct Funnels {
    /// Emitter -> its speed (u/s; negative pulls back).
    on: std::collections::HashMap<usize, f32>,
    /// Current tube of each emitter, with its speed.
    legs: Vec<(Leg, f32)>,
    /// Props being carried (to give their gravity back).
    carried: HashSet<Entity>,
}

fn funnel_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut funnels: ResMut<Funnels>, mut loaded: MessageReader<LevelLoaded>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let force = |i: usize| ents.list[i].get_f32("linearforce").unwrap_or(250.0);
    if loaded.read().count() > 0 {
        funnels.on.clear();
        funnels.carried.clear();
        for (i, e) in ents.list.iter().enumerate() {
            if e.class == "prop_tractor_beam" && !e.get("startenabled").is_some_and(|v| v.trim() == "0") {
                funnels.on.insert(i, force(i));
            }
        }
    }
    for msg in inputs.read() {
        if ents.list[msg.target].class != "prop_tractor_beam" {
            continue;
        }
        let i = msg.target;
        match msg.input.as_str() {
            "enable" => {
                let f = funnels.on.get(&i).copied().unwrap_or(force(i));
                funnels.on.insert(i, f);
            }
            "disable" | "kill" => {
                funnels.on.remove(&i);
            }
            "setlinearforce" => {
                let f = source_assets::kv::parse_f32(&msg.param).unwrap_or(force(i));
                funnels.on.insert(i, f);
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn trace_funnels(
    mut commands: Commands,
    ents: Option<Res<Ents>>,
    mut funnels: ResMut<Funnels>,
    spatial: SpatialQuery,
    globals: Query<&GlobalTransform>,
    body_of: Query<&ColliderOf>,
    kinds: Query<&RigidBody>,
    portals: Query<(&Portal, &Transform, &PortalLink)>,
    mut tubes: Query<(Entity, &mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), (With<Tube>, Without<Portal>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut look: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>, Handle<StandardMaterial>)>>,
) {
    funnels.legs.clear();
    let Some(ents) = ents else { return };
    let (mesh, blue, orange) = look
        .get_or_insert_with(|| {
            let mat = |c: Color| StandardMaterial { base_color: c, unlit: true, alpha_mode: AlphaMode::Add, fog_enabled: false, double_sided: true, cull_mode: None, ..default() };
            (
                meshes.add(Cylinder::new(1.0, 1.0)),
                materials.add(mat(Color::srgba(0.2, 0.45, 1.0, 0.12))),
                materials.add(mat(Color::srgba(1.0, 0.45, 0.1, 0.12))),
            )
        })
        .clone();
    let windows: Vec<(PortalFrame, Vec2, PortalFrame, Vec<Entity>)> = portals
        .iter()
        .filter_map(|(p, t, link)| {
            let (exit_p, exit_t, _) = portals.get(link.0).ok()?;
            Some((Portal::frame(t), p.half_size, Portal::frame(exit_t), exit_p.surfaces.clone()))
        })
        .collect();
    let blocks = |c: Entity, skip: &[Entity]| {
        if skip.contains(&c) {
            return false;
        }
        let body = body_of.get(c).map_or(c, |o| o.body);
        !skip.contains(&body) && kinds.get(body).is_ok_and(|k| !matches!(k, RigidBody::Dynamic))
    };
    let on: Vec<(usize, f32)> = funnels.on.iter().map(|(k, v)| (*k, *v)).collect();
    for (i, speed) in on {
        let e = &ents.list[i];
        if e.killed {
            continue;
        }
        let tf = e.entity.and_then(|x| globals.get(x).ok()).map(|g| g.compute_transform()).unwrap_or(e.transform);
        let mut dir = (tf.rotation * Vec3::X).normalize();
        let mut o = tf.translation + dir * (8.0 * U);
        let mut skip: Vec<Entity> = e.entity.into_iter().collect();
        for _ in 0..MAX_LEGS {
            let hit = spatial.cast_ray_predicate(o, Dir3::new(dir).unwrap_or(Dir3::X), RANGE, true, &SpatialQueryFilter::default(), &|c| blocks(c, &skip));
            let len = hit.map_or(RANGE, |h| h.distance);
            let mut through: Option<(f32, PortalPair, Vec<Entity>)> = None;
            for (frame, half, exit, walls) in &windows {
                let n = frame.normal();
                let den = dir.dot(n);
                if den >= -1e-5 {
                    continue;
                }
                let t = (frame.position - o).dot(n) / den;
                if t < 1e-3 || t > len + 0.05 {
                    continue;
                }
                let l = frame.to_local(o + dir * t);
                if l.x.abs() <= half.x && l.y.abs() <= half.y && through.as_ref().is_none_or(|b| t < b.0) {
                    through = Some((t, PortalPair::new(*frame, *exit), walls.clone()));
                }
            }
            match through {
                Some((t, pair, walls)) => {
                    let end = o + dir * t;
                    funnels.legs.push((Leg { a: o, b: end }, speed));
                    dir = pair.map_dir(dir).normalize();
                    o = pair.map_point(end) + dir * 1e-3;
                    skip = walls;
                }
                None => {
                    funnels.legs.push((Leg { a: o, b: o + dir * len }, speed));
                    break;
                }
            }
        }
    }
    // Draw: one tube per leg, reusing entities.
    let have = tubes.iter().count();
    for _ in have..funnels.legs.len() {
        commands.spawn((Tube, Mesh3d(mesh.clone()), MeshMaterial3d(blue.clone()), Transform::from_scale(Vec3::ZERO), Visibility::Hidden, LevelOwned));
    }
    for (k, (e, mut tf, mut vis, mat)) in tubes.iter_mut().enumerate() {
        let Some((leg, speed)) = funnels.legs.get(k) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let want = if *speed < 0.0 { &orange } else { &blue };
        if mat.0 != *want {
            commands.entity(e).insert(MeshMaterial3d(want.clone()));
        }
        let d = leg.b - leg.a;
        *tf = Transform::from_translation((leg.a + leg.b) * 0.5)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)))
            .with_scale(Vec3::new(RADIUS, d.length(), RADIUS));
        *vis = Visibility::Inherited;
    }
}

#[derive(Component)]
struct Tube;

/// Where `p` is along the funnel: the velocity it should have, if inside.
fn flow_at(legs: &[(Leg, f32)], p: Vec3) -> Option<Vec3> {
    for (leg, speed) in legs {
        let d = leg.b - leg.a;
        let len = d.length();
        if len < 1e-3 {
            continue;
        }
        let axis = d / len;
        let t = (p - leg.a).dot(axis);
        if t < -4.0 * U || t > len {
            continue;
        }
        let on_axis = leg.a + axis * t.clamp(0.0, len);
        let off = on_axis - p;
        if off.length() > RADIUS {
            continue;
        }
        // Along the tube, plus a pull back to its middle.
        return Some(axis * speed * U + off * 3.0);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn carry_in_funnels(
    mut commands: Commands,
    time: Res<Time>,
    mut funnels: ResMut<Funnels>,
    mut players: Query<(Entity, &Transform, &Crouch, &mut CharacterVelocity, Has<InFunnel>), With<Player>>,
    mut props: Query<(Entity, &Position, &mut LinearVelocity, Has<Held>), With<Prop>>,
) {
    let blend = 1.0 - (-6.0 * time.delta_secs()).exp();
    if let Ok((pe, tf, crouch, mut vel, inside)) = players.single_mut() {
        let centre = tf.translation + Vec3::Y * crouch.hull().y * 0.5;
        match flow_at(&funnels.legs, centre) {
            Some(want) => {
                if !inside {
                    commands.entity(pe).insert(InFunnel);
                }
                vel.0 = vel.0.lerp(want, blend);
            }
            None if inside => {
                commands.entity(pe).remove::<InFunnel>();
            }
            None => {}
        }
    }
    let mut now_carried = HashSet::new();
    for (e, pos, mut lv, held) in &mut props {
        if held {
            continue;
        }
        if let Some(want) = flow_at(&funnels.legs, pos.0) {
            now_carried.insert(e);
            if !funnels.carried.contains(&e) {
                commands.entity(e).insert(GravityScale(0.0));
            }
            lv.0 = lv.0.lerp(want, blend);
        }
    }
    for e in funnels.carried.difference(&now_carried) {
        if let Ok(mut c) = commands.get_entity(*e) {
            c.insert(GravityScale(1.0));
        }
    }
    funnels.carried = now_carried;
}
