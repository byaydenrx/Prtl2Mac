//! Hard light bridges (`prop_wall_projector`): a flat, walkable strip of
//! light from the projector straight out until it meets a wall, through any
//! linked portal in its way. Rebuilt whenever its path changes (a portal
//! moved). Players and props don't cut it; walls and doors do.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents};
use crate::level::{GameSet, LevelLoaded, LevelOwned};

pub struct BridgesPlugin;

impl Plugin for BridgesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bridges>().add_systems(Update, (bridge_inputs, build_bridges).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

const U: f32 = shared::SOURCE_UNIT;
/// Width of the bridge.
const WIDTH: f32 = 64.0 * U;
/// Thickness of its collision.
const THICK: f32 = 2.0 * U;
const RANGE: f32 = 8192.0 * U;
const MAX_LEGS: usize = 8;

/// One straight piece: start, end, its right-hand side.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Leg {
    a: Vec3,
    b: Vec3,
    right: Vec3,
}

#[derive(Resource, Default)]
struct Bridges {
    on: std::collections::HashSet<usize>,
    /// Current legs of each projector, and the entities drawn for them.
    built: std::collections::HashMap<usize, (Vec<Leg>, Vec<Entity>)>,
}

/// The collision pieces of every bridge (they never block their own trace).
#[derive(Component)]
pub struct BridgePiece;

fn bridge_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut bridges: ResMut<Bridges>, mut loaded: MessageReader<LevelLoaded>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    if loaded.read().count() > 0 {
        bridges.on.clear();
        bridges.built.clear();
        for (i, e) in ents.list.iter().enumerate() {
            if e.class == "prop_wall_projector" && !e.get("startenabled").is_some_and(|v| v.trim() == "0") {
                bridges.on.insert(i);
            }
        }
    }
    for msg in inputs.read() {
        if ents.list[msg.target].class != "prop_wall_projector" {
            continue;
        }
        match msg.input.as_str() {
            "enable" => {
                bridges.on.insert(msg.target);
            }
            "disable" | "kill" => {
                bridges.on.remove(&msg.target);
            }
            "toggle" => {
                if !bridges.on.remove(&msg.target) {
                    bridges.on.insert(msg.target);
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_bridges(
    mut commands: Commands,
    ents: Option<Res<Ents>>,
    mut bridges: ResMut<Bridges>,
    spatial: SpatialQuery,
    globals: Query<&GlobalTransform>,
    body_of: Query<&ColliderOf>,
    kinds: Query<&RigidBody>,
    pieces: Query<(), With<BridgePiece>>,
    portals: Query<(&Portal, &Transform, &PortalLink)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut look: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let Some(ents) = ents else { return };
    let (mesh, mat) = look
        .get_or_insert_with(|| {
            (
                meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                materials.add(StandardMaterial {
                    base_color: Color::srgba(0.25, 0.55, 1.0, 0.22),
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    fog_enabled: false,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }),
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
    // Only the level's fixed and moving brushes stop a bridge.
    let blocks = |c: Entity, skip: &[Entity]| {
        if skip.contains(&c) || pieces.contains(c) {
            return false;
        }
        let body = body_of.get(c).map_or(c, |o| o.body);
        !pieces.contains(body) && kinds.get(body).is_ok_and(|k| !matches!(k, RigidBody::Dynamic))
    };
    let ids: Vec<usize> = ents.list.iter().enumerate().filter(|(_, e)| e.class == "prop_wall_projector").map(|(i, _)| i).collect();
    for p in ids {
        let mut legs: Vec<Leg> = Vec::new();
        if bridges.on.contains(&p) && !ents.list[p].killed {
            let e = &ents.list[p];
            let tf = e.entity.and_then(|x| globals.get(x).ok()).map(|g| g.compute_transform()).unwrap_or(e.transform);
            let mut dir = (tf.rotation * Vec3::X).normalize();
            let mut right = (tf.rotation * Vec3::Z).normalize();
            let mut o = tf.translation + dir * (4.0 * U);
            let mut skip: Vec<Entity> = Vec::new();
            if let Some(x) = e.entity {
                skip.push(x);
            }
            for _ in 0..MAX_LEGS {
                let filter = SpatialQueryFilter::default();
                let hit = spatial.cast_ray_predicate(o, Dir3::new(dir).unwrap_or(Dir3::X), RANGE, true, &filter, &|c| blocks(c, &skip));
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
                        legs.push(Leg { a: o, b: end, right });
                        dir = pair.map_dir(dir).normalize();
                        right = pair.map_dir(right).normalize();
                        o = pair.map_point(end) + dir * 1e-3;
                        skip = walls;
                    }
                    None => {
                        legs.push(Leg { a: o, b: o + dir * len, right });
                        break;
                    }
                }
            }
        }
        let same = bridges.built.get(&p).is_some_and(|(old, _)| {
            old.len() == legs.len() && old.iter().zip(&legs).all(|(x, y)| x.a.distance(y.a) < 0.01 && x.b.distance(y.b) < 0.01 && x.right.dot(y.right) > 0.999)
        });
        if same {
            continue;
        }
        if let Some((_, old)) = bridges.built.remove(&p) {
            for e in old {
                commands.entity(e).despawn();
            }
        }
        let mut spawned = Vec::new();
        for leg in &legs {
            let d = leg.b - leg.a;
            let len = d.length();
            if len < 1e-3 {
                continue;
            }
            let fwd = d / len;
            let up = leg.right.cross(fwd).normalize_or(Vec3::Y);
            let up = if up.dot(Vec3::Y) < 0.0 && fwd.y.abs() < 0.9 { -up } else { up };
            let right = fwd.cross(up).normalize();
            let rot = Quat::from_mat3(&Mat3::from_cols(right, up, -fwd));
            let centre = (leg.a + leg.b) * 0.5;
            spawned.push(
                commands
                    .spawn((
                        BridgePiece,
                        Transform::from_translation(centre).with_rotation(rot),
                        RigidBody::Static,
                        Collider::cuboid(WIDTH, THICK, len),
                        LevelOwned,
                        Visibility::default(),
                        children![(Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::from_scale(Vec3::new(WIDTH, THICK * 0.5, len)))],
                    ))
                    .id(),
            );
        }
        if std::env::var_os("P2_TRACE").is_some() {
            info!("TRACE bridge {:?}: {} legs, {:?}", ents.list[p].name, legs.len(), legs.iter().map(|l| (l.a / U, l.b / U)).collect::<Vec<_>>());
        }
        bridges.built.insert(p, (legs, spawned));
    }
}
