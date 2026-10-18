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
        app.add_systems(Update, (spawn_active_portals, portal_inputs).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
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
    // The wall behind it: rays from just in front of the opening, at the centre and corners.
    let mut surfaces = Vec::new();
    for (x, y) in [(0.0, 0.0), (-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8)] {
        let from = frame.to_world(Vec3::new(half.x * x, half.y * y, 0.05));
        if let Some(hit) = spatial.cast_ray(from, Dir3::new(-frame.normal()).unwrap_or(Dir3::NEG_Z), 0.2, true, &SpatialQueryFilter::from_excluded_entities([player])) {
            if !surfaces.contains(&hit.entity) {
                surfaces.push(hit.entity);
            }
        }
    }
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
