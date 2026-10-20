//! Portals the map opens itself (`prop_portal`): the portal carousel, the
//! orange portal left open for the first gun. They share linkage group 0
//! with the player's gun, so a shot of the same colour replaces them.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents, FireOutput};
use crate::level::{LevelLoaded, LevelOwned};

pub struct MapPortalsPlugin;

impl Plugin for MapPortalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LinkedDoors>().add_systems(
            Update,
            (spawn_active_portals, portal_inputs, linked_door_inputs, sync_linked_doors, sync_noportal_volumes, sync_placement_helpers, log_shots, clear_portal_tunnels, portal_detectors)
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
    walls: WallBodies,
) {
    let Some(ev) = loaded.read().last().cloned() else { return };
    if ev.menu {
        return;
    }
    let Some(ents) = ents else { return };
    let Ok((player, mut gun)) = players.single_mut() else { return };
    for i in 0..ents.list.len() {
        if ents.list[i].class == "prop_portal" && ents.list[i].get("activated").is_some_and(|v| v.trim() == "1") {
            open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None, &walls);
        }
    }
}

/// Which colliders can be a portal's wall: anything but a dynamic body. A
/// cube sitting in the opening when a map portal (re)opens is not the wall:
/// listed as one, its motion would fizzle the portal and the player would
/// pass through it.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct WallBodies<'w, 's> {
    bodies: Query<'w, 's, &'static RigidBody>,
    body_of: Query<'w, 's, &'static ColliderOf>,
}

impl WallBodies<'_, '_> {
    pub(crate) fn can_be_wall(&self, c: Entity) -> bool {
        let body = self.body_of.get(c).map_or(c, |o| o.body);
        !matches!(self.bodies.get(body), Ok(RigidBody::Dynamic))
    }
}

/// How far behind an opening the world is cut away: a standing player's
/// whole hull (72 u) passes before anything behind the wall can hold it.
const TUNNEL_DEPTH: f32 = 80.0 * shared::SOURCE_UNIT;
/// The tunnel stays this far inside the opening's edges, so the floor a wall
/// portal stands on (its bottom edge) is not cut away.
const TUNNEL_INSET: f32 = 2.0 * shared::SOURCE_UNIT;

/// Source cuts the world out of a portal's hole, not only the face the portal
/// sits on: a floor made of two brush layers held the player and cubes on the
/// lower one, 6 u under a floor portal (sp_a1_intro4). Every static collider
/// in the tunnel behind a new portal's opening joins its `surfaces`, which the
/// player and props ignore while in the hole. Only static world: a mover
/// listed there would fizzle the portal when it moves.
fn clear_portal_tunnels(
    mut placed: MessageReader<PortalPlaced>,
    mut pending: Local<Vec<(Entity, u8)>>,
    mut portals: Query<(&mut Portal, &Transform)>,
    spatial: SpatialQuery,
    world: Query<(Option<&ColliderOf>, Has<Sensor>, Has<PortalHelperCollider>)>,
    bodies: Query<&RigidBody>,
) {
    pending.extend(placed.read().map(|p| (p.portal, 0)));
    let mut retry = Vec::new();
    for (e, tries) in pending.drain(..) {
        let Ok((mut portal, tf)) = portals.get_mut(e) else {
            // Spawned by a command that hasn't applied yet (or closed again).
            if tries < 3 {
                retry.push((e, tries + 1));
            }
            continue;
        };
        let frame = Portal::frame(tf);
        let half = (portal.half_size - Vec2::splat(TUNNEL_INSET)).max(Vec2::splat(TUNNEL_INSET));
        let centre = frame.to_world(Vec3::new(0.0, 0.0, -(TUNNEL_INSET + TUNNEL_DEPTH * 0.5)));
        let shape = Collider::cuboid(2.0 * half.x, 2.0 * half.y, TUNNEL_DEPTH);
        let mut added = 0;
        for c in spatial.shape_intersections(&shape, centre, frame.rotation, &SpatialQueryFilter::default()) {
            let Ok((of, sensor, helper)) = world.get(c) else { continue };
            let is_static = of.map_or(true, |o| matches!(bodies.get(o.body), Ok(RigidBody::Static) | Err(_)));
            if !sensor && !helper && is_static && !portal.surfaces.contains(&c) {
                portal.surfaces.push(c);
                added += 1;
            }
        }
        if added > 0 {
            debug!("portal2: {added} more colliders cut out of the hole behind {e:?}");
        }
    }
    *pending = retry;
}

/// The wall colliders just behind an opening at `tf` (what the player must
/// pass through while inside the hole).
pub(crate) fn surfaces_behind(spatial: &SpatialQuery, tf: &Transform, half: Vec2, player: Entity, walls: &WallBodies) -> Vec<Entity> {
    let frame = Portal::frame(tf);
    let mut surfaces = Vec::new();
    for (x, y) in [(0.0, 0.0), (-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8), (0.0, -0.9), (0.0, 0.9), (-0.9, 0.0), (0.9, 0.0)] {
        let from = frame.to_world(Vec3::new(half.x * x, half.y * y, 0.05));
        for hit in spatial.ray_hits(from, Dir3::new(-frame.normal()).unwrap_or(Dir3::NEG_Z), 0.3, 8, true, &SpatialQueryFilter::from_excluded_entities([player])) {
            if !surfaces.contains(&hit.entity) && walls.can_be_wall(hit.entity) {
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
    walls: &WallBodies,
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
    let surfaces = surfaces_behind(spatial, &tf, half, player, walls);
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
    walls: WallBodies,
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
                    open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None, &walls);
                } else {
                    close(&mut commands, &mut gun, &mut closed);
                }
            }
            "open" => open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, None, &walls),
            "fizzle" | "close" => close(&mut commands, &mut gun, &mut closed),
            "newlocation" => {
                let v: Vec<f32> = msg.param.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if v.len() >= 6 {
                    let at = Transform::from_translation(shared::source_to_world(Vec3::new(v[0], v[1], v[2])))
                        .with_rotation(source_assets::entities::source_angles_to_world(Vec3::new(v[3], v[4], v[5])));
                    open_portal(&mut commands, &ents, i, &spatial, time.elapsed_secs(), player, &mut gun, &mut placed, Some(at), &walls);
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

/// Keep the gun's `info_placement_helper`s where their entities are (some
/// are parented to moving panels).
fn sync_placement_helpers(ents: Option<Res<Ents>>, mut helpers: ResMut<PlacementHelpers>) {
    let now: Vec<PlacementHelper> = ents
        .iter()
        .flat_map(|e| e.list.iter())
        .filter(|e| e.class == "info_placement_helper" && !e.killed)
        .map(|e| PlacementHelper {
            position: e.transform.translation,
            radius: e.get("radius").and_then(|r| r.parse::<f32>().ok()).unwrap_or(64.0) * shared::SOURCE_UNIT,
            up: (e.get("snap_to_helper_angles") == Some("1")).then(|| e.transform.rotation * Vec3::Y),
        })
        .collect();
    if helpers.0 != now {
        helpers.0 = now;
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
    walls: WallBodies,
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
        let surfaces = surfaces_behind(&spatial, &tf, half, player, &walls);
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

/// `func_portal_detector`: fires `OnStartTouchPortal` / `OnEndTouchPortal`
/// (and the `1`/`2`, `LinkedPortal` and `BothLinkedPortals` variants) as
/// portals open in or leave its brush. sp_a1_intro7 moves on when the player
/// puts a portal on the wall behind Wheatley this way.
fn portal_detectors(
    ents: Option<Res<Ents>>,
    portals: Query<(Entity, &Portal, &Transform, Option<&PortalLink>)>,
    mut inside: Local<std::collections::HashMap<usize, Vec<Entity>>>,
    mut loaded: MessageReader<LevelLoaded>,
    mut fire: MessageWriter<FireOutput>,
) {
    if loaded.read().count() > 0 {
        inside.clear();
    }
    let Some(ents) = ents else { return };
    // Portals sit on the surface; the detector brush may stop just short of it.
    let pad = Vec3::splat(2.0 * shared::SOURCE_UNIT);
    for (i, e) in ents.list.iter().enumerate() {
        if e.class != "func_portal_detector" {
            continue;
        }
        let now: Vec<Entity> = match (e.enabled && !e.killed, e.bounds) {
            (true, Some((lo, hi))) => portals
                .iter()
                .filter(|(_, _, tf, _)| {
                    let p = tf.translation;
                    p.cmpge(lo - pad).all() && p.cmple(hi + pad).all()
                })
                .map(|(pe, ..)| pe)
                .collect(),
            _ => Vec::new(),
        };
        let before = inside.get(&i).cloned().unwrap_or_default();
        let both = |set: &[Entity]| set.iter().any(|p| portals.get(*p).is_ok_and(|(_, _, _, l)| l.is_some_and(|l| set.contains(&l.0))));
        let mut out = |event: String| fire.write(FireOutput { ent: i, event, activator: None });
        for (set, other, edge) in [(&now, &before, "Start"), (&before, &now, "End")] {
            for p in set.iter().filter(|p| !other.contains(p)) {
                out(format!("On{edge}TouchPortal"));
                if let Ok((_, portal, _, link)) = portals.get(*p) {
                    out(format!("On{edge}TouchPortal{}", portal.kind.index() + 1));
                    if link.is_some() {
                        out(format!("On{edge}TouchLinkedPortal"));
                    }
                }
            }
            if both(set) && !both(other) {
                out(format!("On{edge}TouchBothLinkedPortals"));
            }
        }
        if now.is_empty() {
            inside.remove(&i);
        } else {
            inside.insert(i, now);
        }
    }
}
