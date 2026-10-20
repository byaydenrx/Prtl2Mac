//! Exit-side copies of props in a portal opening (T-102).
//!
//! A cube half through portal `P` is drawn twice, as in Portal 2: the prop
//! itself, and a ghost of its meshes placed through `portal_pair(P, Q)`, so
//! the half that has gone in comes out of `Q`. Neither copy needs clip
//! planes: the part of each that lies behind its portal's plane is hidden by
//! that portal's surface (main view, depth-written) or by the oblique near
//! plane (portal views), the same way the player body's overlap is.
//!
//! The ghosts copy the prop's draws (Valve-shader surfaces or fallback
//! meshes) and their proxy overrides every frame, so skin changes and
//! dissolving follow the prop.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use p2_body::{ghost_transform, overlaps_opening, GhostBounds};
use p2_core::{Portal, PortalLink, PortalPair};
use source_assets::material::SourceMaterial;
use source_assets::valve::{SurfaceOverrides, ValveSurface};

use crate::props::Prop;

pub struct PropGhostsPlugin;

impl Plugin for PropGhostsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, sync_prop_ghosts.before(bevy::transform::TransformSystems::Propagate));
    }
}

/// A mesh of a [`PropGhost`].
#[derive(Component)]
struct PropGhostPart;

/// A mirror of one prop through one portal.
#[derive(Component)]
struct PropGhost {
    prop: Entity,
    portal: Entity,
    /// (source mesh, mirror mesh).
    parts: Vec<(Entity, Entity)>,
}

/// The portals (entry entity, its pair) whose opening `bounds` at `root`
/// overlaps.
pub fn ghost_portals(root: &Transform, bounds: &GhostBounds, portals: &[(Entity, p2_core::PortalFrame, Vec2, PortalPair)]) -> Vec<(Entity, PortalPair)> {
    portals
        .iter()
        .filter(|(_, frame, half, _)| overlaps_opening(frame, *half, root, bounds))
        .map(|(e, _, _, pair)| (*e, *pair))
        .collect()
}

/// The transform of `e` relative to `root` (its ancestor).
fn local_to_root(mut e: Entity, root: Entity, parents: &Query<&ChildOf>, transforms: &Query<&Transform, Without<PropGhost>>) -> Transform {
    let mut m = Transform::IDENTITY;
    while e != root {
        let Ok(t) = transforms.get(e) else { break };
        m = t.mul_transform(m);
        let Ok(p) = parents.get(e) else { break };
        e = p.parent();
    }
    m
}

/// What a prop part draws with: a Valve-shader surface (most models) or a
/// fallback `SourceMaterial` mesh.
type PartQuery<'a> = (
    Option<&'a ValveSurface>,
    Option<&'a SurfaceOverrides>,
    Option<&'a Mesh3d>,
    Option<&'a MeshMaterial3d<SourceMaterial>>,
    Option<&'a RenderLayers>,
);

#[allow(clippy::too_many_arguments)]
fn sync_prop_ghosts(
    mut commands: Commands,
    props: Query<(Entity, &Prop, &Transform, Option<&Visibility>)>,
    portals: Query<(Entity, &Portal, &Transform, &PortalLink), Without<Prop>>,
    exits: Query<&Transform, With<Portal>>,
    ghosts: Query<(Entity, &PropGhost)>,
    mut ghost_tfs: Query<&mut Transform, (With<PropGhost>, Without<Prop>, Without<Portal>)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform, Without<PropGhost>>,
    sources: Query<PartQuery, Without<PropGhostPart>>,
    shown: Query<&InheritedVisibility, Without<PropGhostPart>>,
) {
    // A part is drawn while its source is (a cube's other skin is hidden).
    let part_vis = |src: Entity| if shown.get(src).is_ok_and(|v| !v.get()) { Visibility::Hidden } else { Visibility::Inherited };
    let frames: Vec<_> = portals
        .iter()
        .filter_map(|(e, p, t, link)| {
            let exit = exits.get(link.0).ok()?;
            let entry = Portal::frame(t);
            Some((e, entry, p.half_size, PortalPair::new(entry, Portal::frame(exit))))
        })
        .collect();
    let mut wanted: std::collections::HashMap<(Entity, Entity), (PortalPair, Transform, Visibility)> = Default::default();
    for (e, prop, tf, vis) in &props {
        let bounds = GhostBounds { min: -prop.half, max: prop.half };
        for (portal, pair) in ghost_portals(tf, &bounds, &frames) {
            wanted.insert((e, portal), (pair, *tf, vis.copied().unwrap_or_default()));
        }
    }
    // Update or remove the existing ghosts; parts follow skin and proxy
    // changes on the prop.
    for (g, ghost) in &ghosts {
        match wanted.remove(&(ghost.prop, ghost.portal)) {
            Some((pair, root, vis)) => {
                if let Ok(mut t) = ghost_tfs.get_mut(g) {
                    *t = ghost_transform(&pair, &root);
                }
                commands.entity(g).insert(vis);
                for &(src, dst) in &ghost.parts {
                    if let Ok((surface, overrides, _, mat, _)) = sources.get(src) {
                        let mut d = commands.entity(dst);
                        d.insert(part_vis(src));
                        if let Some(s) = surface {
                            d.insert(s.clone());
                        }
                        if let Some(o) = overrides {
                            d.insert(o.clone());
                        }
                        if let Some(m) = mat {
                            d.insert(m.clone());
                        }
                    }
                }
            }
            None => commands.entity(g).despawn(),
        }
    }
    // Spawn the new ones.
    for ((prop, portal), (pair, root, vis)) in wanted {
        let g = commands.spawn((Name::new("prop ghost"), ghost_transform(&pair, &root), vis)).id();
        let mut parts = Vec::new();
        for d in children.iter_descendants(prop) {
            let Ok((surface, overrides, mesh, mat, layers)) = sources.get(d) else { continue };
            let drawn_mesh = mesh.zip(mat);
            if surface.is_none() && drawn_mesh.is_none() {
                continue;
            }
            let mut e = commands.spawn((PropGhostPart, local_to_root(d, prop, &parents, &transforms), part_vis(d), ChildOf(g)));
            if let Some(s) = surface {
                e.insert(s.clone());
            }
            if let Some(o) = overrides {
                e.insert(o.clone());
            }
            if let Some((m, h)) = drawn_mesh {
                e.insert((m.clone(), h.clone()));
            }
            if let Some(l) = layers {
                e.insert(l.clone());
            }
            parts.push((d, e.id()));
        }
        debug!("prop ghost of {prop} through {portal}: {} parts", parts.len());
        commands.entity(g).insert(PropGhost { prop, portal, parts });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2_core::PortalFrame;

    fn frame(pos: Vec3, n: Vec3) -> PortalFrame {
        PortalFrame::new(pos, Quat::from_rotation_arc(Vec3::Z, n))
    }

    #[test]
    fn a_cube_in_the_opening_gets_a_ghost_out_of_the_exit() {
        let entry = frame(Vec3::ZERO, Vec3::Z);
        let exit = frame(Vec3::new(10.0, 0.0, 0.0), Vec3::X);
        let pair = PortalPair::new(entry, exit);
        let portals = vec![(Entity::from_raw_u32(1).unwrap(), entry, Vec2::new(0.8, 1.4), pair)];
        let bounds = GhostBounds { min: Vec3::splat(-0.23), max: Vec3::splat(0.23) };
        // Half through: a ghost, and it sits half out of the exit.
        let half_in = Transform::from_xyz(0.0, 0.0, 0.0);
        let got = ghost_portals(&half_in, &bounds, &portals);
        assert_eq!(got.len(), 1);
        let g = ghost_transform(&got[0].1, &half_in);
        assert!(exit.plane_distance(g.translation).abs() < 1e-4, "ghost centre on the exit plane: {:?}", g.translation);
        // A cube going in: its front half behind the entry is in front of the exit.
        let going = Transform::from_xyz(0.0, 0.0, 0.1);
        let g = ghost_transform(&pair, &going);
        assert!(exit.plane_distance(g.translation) < 0.0);
        // Well in front of the portal, or beside the opening: none.
        assert!(ghost_portals(&Transform::from_xyz(0.0, 0.0, 1.0), &bounds, &portals).is_empty());
        assert!(ghost_portals(&Transform::from_xyz(3.0, 0.0, 0.0), &bounds, &portals).is_empty());
    }
}
