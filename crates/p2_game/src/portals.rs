//! Portals the map opens itself (`prop_portal`): the portal carousel, the
//! orange portal left open for the first gun. They share linkage group 0
//! with the player's gun, so a shot of the same colour replaces them.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents};
use crate::level::{LevelLoaded, LevelOwned};

pub struct MapPortalsPlugin;

impl Plugin for MapPortalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LinkedDoors>().add_systems(
            Update,
            (spawn_active_portals, portal_inputs, linked_door_inputs, sync_linked_doors, sync_noportal_volumes, log_shots)
                .chain()
                .after(crate::io::run_io)
                .in_set(crate::level::GameSet::Logic),
        );
    }
}

/// A portal opened by a `prop_portal` entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct MapPortal {
    pub ent: usize,
}

fn kind_of(ents: &Ents, i: usize) -> PortalKind {
    if ents.list[i].get("portaltwo").is_some_and(|v| v.trim() == "1") {
        PortalKind::Orange
    } else {
        PortalKind::Blue
    }
}

fn spawn_active_portals(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    spatial: SpatialQuery,
    time: Res<Time>,
    mut players: Query<(Entity, &mut PortalGun), With<Player>>,
    mut placed: MessageWriter<PortalPlaced>,
) {
    let Some(ev) = loaded.read().last().cloned() else { return };
    if ev.menu {
        return;
    }
    let Some(ents) = ents else { return };
    let Ok((player, mut gun)) = players.single_mut() else { return };
    for i in 0..ents.list.len() {
        if ents.list[i].class == "prop_portal" && ents.list[i].get("activated").is_some_and(|v| v.trim() == "1") {
            open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None);
        }
    }
}

/// The wall colliders just behind an opening at `tf` (what the player must
/// pass through while inside the hole).
fn surfaces_behind(spatial: &SpatialQuery, tf: &Transform, half: Vec2, player: Entity) -> Vec<Entity> {
    let frame = Portal::frame(tf);
    let mut surfaces = Vec::new();
    for (x, y) in [(0.0, 0.0), (-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8), (0.0, -0.9), (0.0, 0.9), (-0.9, 0.0), (0.9, 0.0)] {
        let from = frame.to_world(Vec3::new(half.x * x, half.y * y, 0.05));
        for hit in spatial.ray_hits(from, Dir3::new(-frame.normal()).unwrap_or(Dir3::NEG_Z), 0.3, 8, true, &SpatialQueryFilter::from_excluded_entities([player])) {
            if !surfaces.contains(&hit.entity) {
                surfaces.push(hit.entity);
            }
        }
    }
    surfaces
}

/// Open the `prop_portal` at `i` (at its own place, or `at`).
#[allow(clippy::too_many_arguments)]
fn open_portal(
    commands: &mut Commands,
    ents: &Ents,
    i: usize,
    spatial: &SpatialQuery,
    now: f32,
    player: Entity,
    gun: &mut PortalGun,
    placed: &mut MessageWriter<PortalPlaced>,
    at: Option<Transform>,
) {
    let kind = kind_of(ents, i);
    let e = &ents.list[i];
    let src = at.unwrap_or(e.transform);
    // The entity faces out of the wall along its forward (Source +X, world
    // `rotation * X`); a portal's normal is its +Z.
    let rotation = src.rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let tf = Transform::from_translation(src.translation).with_rotation(rotation);
    let frame = Portal::frame(&tf);
    let half = PORTAL_HALF_SIZE;
    // The wall behind it, and the frame prop the map puts around the portal.
    let surfaces = surfaces_behind(spatial, &tf, half, player);
    let wall_count = surfaces.len();
    if let Some(old) = gun.portals[kind.index()].take() {
        commands.entity(old).despawn();
    }
    let new = commands
        .spawn((
            Name::new(format!("map {kind:?} portal {}", e.name)),
            Portal { kind, half_size: half, surfaces, placed_at: now },
            tf,
            MapPortal { ent: i },
            LevelOwned,
        ))
        .id();
    gun.portals[kind.index()] = Some(new);
    if let Some(partner) = gun.portals[kind.other().index()] {
        commands.entity(new).insert(PortalLink(partner));
        commands.entity(partner).insert(PortalLink(new));
    }
    debug!("portal2: map portal {} ({kind:?}) open at {:.2?} normal {:.2?}, wall colliders {:?}", e.name, tf.translation, frame.normal(), wall_count);
    placed.write(PortalPlaced { gun: player, portal: new, kind, transform: tf });
}

#[allow(clippy::too_many_arguments)]
fn portal_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    spatial: SpatialQuery,
    time: Res<Time>,
    mut players: Query<(Entity, &mut PortalGun), With<Player>>,
    portals: Query<(Entity, &Portal, &Transform, Option<&MapPortal>)>,
    mut placed: MessageWriter<PortalPlaced>,
    mut closed: MessageWriter<PortalClosed>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let Ok((player, mut gun)) = players.single_mut() else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class != "prop_portal" {
            continue;
        }
        let i = msg.target;
        let kind = kind_of(&ents, i);
        let mine = portals.iter().find(|(_, _, _, m)| m.is_some_and(|m| m.ent == i));
        let close = |commands: &mut Commands, gun: &mut PortalGun, closed: &mut MessageWriter<PortalClosed>| {
            if let Some((pe, p, tf, _)) = mine {
                closed.write(PortalClosed { kind: p.kind, transform: *tf });
                commands.entity(pe).despawn();
                if gun.portals[kind.index()] == Some(pe) {
                    gun.portals[kind.index()] = None;
                }
                if let Some(partner) = gun.portals[kind.other().index()] {
                    commands.entity(partner).remove::<PortalLink>();
                }
            }
        };
        match msg.input.as_str() {
            "setactivatedstate" => {
                if msg.param.trim() == "1" {
                    open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None);
                } else {
                    close(&mut commands, &mut gun, &mut closed);
                }
            }
            "open" => open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None),
            "fizzle" | "close" => close(&mut commands, &mut gun, &mut closed),
            "newlocation" => {
                let v: Vec<f32> = msg.param.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if v.len() >= 6 {
                    let at = Transform::from_translation(shared::source_to_world(Vec3::new(v[0], v[1], v[2])))
                        .with_rotation(source_assets::entities::source_angles_to_world(Vec3::new(v[3], v[4], v[5])));
                    open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, Some(at));
                }
            }
            _ => {}
        }
    }
}

/// Keep the gun's `func_noportal_volume` boxes in step with the map (they
/// can be enabled and disabled).
fn sync_noportal_volumes(ents: Option<Res<Ents>>, mut volumes: ResMut<NoPortalVolumes>) {
    let boxes: Vec<(Vec3, Vec3)> = ents
        .iter()
        .flat_map(|e| e.list.iter())
        .filter(|e| e.class == "func_noportal_volume" && e.enabled && !e.killed)
        .filter_map(|e| e.bounds)
        .collect();
    if volumes.0 != boxes {
        debug!("portal2: no-portal volumes {:.2?}", boxes.iter().map(|(lo, hi)| { let s = |v: Vec3| Vec3::new(v.x, -v.z, v.y) / shared::SOURCE_UNIT; (s(*lo), s(*hi)) }).collect::<Vec<_>>());
        volumes.0 = boxes;
    }
}

/// Portal shot results in the debug log (scripted tests read them).
fn log_shots(mut failed: MessageReader<PortalShotFailed>, mut placed: MessageReader<PortalPlaced>) {
    let s = |v: Vec3| Vec3::new(v.x, -v.z, v.y) / shared::SOURCE_UNIT;
    for f in failed.read() {
        debug!("portal2: {:?} shot failed: {:?} at {:.0?}", f.kind, f.reason, f.point.map(s));
    }
    for p in placed.read() {
        debug!("portal2: {:?} portal placed at {:.0?}", p.kind, s(p.transform.translation));
    }
}

/// `linked_portal_door`s: fixed world portals the map pairs up itself
/// (sp_a1_wakeup's drop into the incinerator, the finale). They are kept out
/// of the gun's two slots so they never replace the player's portals, and
/// they aren't drawn (the renderer shows the gun's pair); walking or falling
/// through one carries the player to its partner.
#[derive(Resource, Default)]
struct LinkedDoors {
    /// Map entity -> (open, partner name).
    state: std::collections::HashMap<usize, (bool, String)>,
    /// Map entity -> its spawned portal.
    spawned: std::collections::HashMap<usize, Entity>,
    dirty: bool,
}

/// A portal opened by a `linked_portal_door`.
#[derive(Component, Clone, Copy, Debug)]
pub struct LinkedPortalDoor {
    pub ent: usize,
}

fn linked_door_inputs(mut loaded: MessageReader<LevelLoaded>, mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut doors: ResMut<LinkedDoors>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    if loaded.read().last().is_some() {
        // Despawned with the level (LevelOwned).
        doors.spawned.clear();
        doors.state = (0..ents.list.len())
            .filter(|&i| ents.list[i].class == "linked_portal_door")
            .map(|i| {
                let e = &ents.list[i];
                (i, (e.get("startactive").is_none_or(|v| v.trim() != "0"), e.get("partnername").unwrap_or("").trim().to_string()))
            })
            .collect();
        doors.dirty = true;
    }
    for msg in inputs.read() {
        let Some(st) = doors.state.get_mut(&msg.target) else { continue };
        match msg.input.as_str() {
            "open" => st.0 = true,
            "close" => st.0 = false,
            "setpartner" => st.1 = msg.param.trim().to_string(),
            _ => continue,
        }
        doors.dirty = true;
    }
}

fn sync_linked_doors(
    mut commands: Commands,
    ents: Option<Res<Ents>>,
    mut doors: ResMut<LinkedDoors>,
    spatial: SpatialQuery,
    time: Res<Time>,
    players: Query<Entity, With<Player>>,
) {
    let Some(ents) = ents else { return };
    if !doors.dirty {
        return;
    }
    let Ok(player) = players.single() else { return };
    doors.dirty = false;
    // Linked pairs: both open, and either names the other.
    let by_name = |name: &str| doors.state.keys().copied().find(|&j| ents.list[j].name == name);
    let mut partner_of: std::collections::HashMap<usize, usize> = Default::default();
    for (&i, (open, partner)) in &doors.state {
        if !*open || partner.is_empty() {
            continue;
        }
        if let Some(j) = by_name(partner).filter(|j| *j != i && doors.state[j].0) {
            partner_of.insert(i, j);
            partner_of.entry(j).or_insert(i);
        }
    }
    // Close the ones no longer linked.
    let stale: Vec<usize> = doors.spawned.keys().copied().filter(|i| !partner_of.contains_key(i)).collect();
    for i in stale {
        if let Some(e) = doors.spawned.remove(&i) {
            commands.entity(e).despawn();
            debug!("portal2: linked portal door {} closed", ents.list[i].name);
        }
    }
    let mut order: Vec<usize> = partner_of.keys().copied().collect();
    order.sort();
    for &i in &order {
        if doors.spawned.contains_key(&i) {
            continue;
        }
        let e = &ents.list[i];
        let w = e.get_f32("width").unwrap_or(64.0);
        let h = e.get_f32("height").unwrap_or(64.0);
        let half = Vec2::new(w, h) * 0.5 * shared::SOURCE_UNIT;
        let tf = Transform::from_translation(e.transform.translation).with_rotation(e.transform.rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let surfaces = surfaces_behind(&spatial, &tf, half, player);
        let kind = if order.iter().position(|x| *x == i).unwrap_or(0) % 2 == 0 { PortalKind::Blue } else { PortalKind::Orange };
        let id = commands
            .spawn((Name::new(format!("linked portal door {}", e.name)), Portal { kind, half_size: half, surfaces, placed_at: time.elapsed_secs() }, tf, LinkedPortalDoor { ent: i }, LevelOwned))
            .id();
        debug!("portal2: linked portal door {} open at {:.2?}", e.name, tf.translation);
        doors.spawned.insert(i, id);
    }
    for (&i, &j) in &partner_of {
        if let (Some(&a), Some(&b)) = (doors.spawned.get(&i), doors.spawned.get(&j)) {
            commands.entity(a).insert(PortalLink(b));
        }
    }
}
