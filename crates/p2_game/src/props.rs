//! Physics props: cubes and `prop_physics` fall and tumble under Portal 2's
//! gravity, the player picks them up with E and drops them again, they go
//! through linked portals, and they weigh down floor buttons.
//!
//! Holding follows Portal 2's feel: the object floats in front of the eye,
//! upright and turned with the player, chasing its target with a capped
//! speed, and is dropped when it is pulled too far from it.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::math::{clamp_exit_velocity, exit_min_speed, segment_crosses_opening};
use p2_core::*;

use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::level::{CurrentMap, LevelLoaded, LevelOwned};

pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(Vec3::NEG_Y * 600.0 * shared::SOURCE_UNIT))
            .add_systems(Update, (make_props_dynamic, hook_prop_colliders, use_key, prop_inputs, floor_buttons).chain().in_set(crate::level::GameSet::Logic))
            .add_systems(FixedUpdate, (carry, prop_transit, wall_passthrough).chain().after(P2Systems::Transit));
    }
}

/// Things the player can pick up.
const PICKUP_CLASSES: &[&str] = &["prop_weighted_cube", "prop_monster_box", "prop_physics", "prop_physics_override"];
/// How far the use key reaches (Portal 2 `player_use_dist` ~ 80 u from the eye).
const USE_RANGE: f32 = 80.0 * shared::SOURCE_UNIT;
/// Fastest a held object moves to its target (m/s).
const HOLD_MAX_SPEED: f32 = 1000.0 * shared::SOURCE_UNIT;
/// A held object pulled this far from where it should be is dropped.
const HOLD_BREAK_DISTANCE: f32 = 64.0 * shared::SOURCE_UNIT;

/// A dynamic map prop.
#[derive(Component, Clone, Copy, Debug)]
pub struct Prop {
    /// Index in [`Ents::list`].
    pub ent: usize,
    /// Half extents of its box (m).
    pub half: Vec3,
}

/// The prop the player is carrying.
#[derive(Component, Clone, Copy, Debug)]
pub struct Held {
    /// Yaw of the prop relative to the player's yaw when picked up.
    pub yaw_offset: f32,
    /// Distance in front of the eye.
    pub distance: f32,
}

/// The prop's colliders the player ignores while carrying it.
#[derive(Component, Default)]
struct HeldColliders(Vec<Entity>);

/// Map props become dynamic bodies once the level is up.
fn make_props_dynamic(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    transforms: Query<&Transform>,
    children: Query<&Children>,
    aabbs: Query<&ColliderAabb>,
    colliders: Query<&Collider>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    let Some(ents) = ents else { return };
    for (i, e) in ents.list.iter().enumerate() {
        if !PICKUP_CLASSES.contains(&e.class.as_str()) {
            continue;
        }
        let Some(entity) = e.entity else { continue };
        // Its spawn pose under the map root (which sits at the origin), so
        // local is world. (Global transforms aren't propagated yet here.)
        let tf = transforms.get(entity).copied().unwrap_or(e.transform);
        // Its box, from the collider children.
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for c in children.iter_descendants(entity) {
            if let Ok(a) = aabbs.get(c) {
                lo = lo.min(a.min);
                hi = hi.max(a.max);
            }
        }
        let half = if lo.x <= hi.x { (hi - lo) * 0.5 } else { Vec3::splat(16.0 * shared::SOURCE_UNIT) };
        commands.entity(entity).remove::<ChildOf>().insert((
            tf,
            RigidBody::Dynamic,
            // vphysics cube: 40 kg-ish, slides a little, rolls very little.
            Mass(40.0),
            Friction::new(0.6),
            Restitution::new(0.05),
            LinearDamping(0.05),
            AngularDamping(0.3),
            Prop { ent: i, half },
            LevelOwned,
        ));
        // Its colliders were attached to the map's static body when spawned
        // under it; re-insert them so they attach to this body instead.
        for c in children.iter_descendants(entity) {
            if let Ok(col) = colliders.get(c) {
                commands.entity(c).remove::<(Collider, ColliderOf)>().insert(col.clone());
            }
        }
    }
}

/// E: drop what you hold, else pick up what you look at, else press a
/// pedestal button.
#[allow(clippy::too_many_arguments)]
fn use_key(
    mut commands: Commands,
    mut players: Query<(Entity, &mut MoveInput, &InterpolatedFeet, &Crouch, &ViewAngles, &mut IgnoreColliders), With<Player>>,
    held: Query<(Entity, &HeldColliders), With<Held>>,
    props: Query<(&Prop, &Transform)>,
    body_of: Query<&ColliderOf>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    spatial: SpatialQuery,
    ents: Option<ResMut<Ents>>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Ok((player, mut input, feet, crouch, view, mut ignore)) = players.single_mut() else { return };
    if !input.use_key {
        return;
    }
    input.use_key = false;
    if let Ok((e, hc)) = held.single() {
        for c in &hc.0 {
            ignore.remove(c);
        }
        commands.entity(e).remove::<(Held, HeldColliders)>().insert(GravityScale(1.0));
        return;
    }
    let eye = feet.curr + Vec3::Y * crouch.eye_height;
    let dir = Dir3::new(view.forward()).unwrap_or(Dir3::NEG_Z);
    let filter = SpatialQueryFilter::from_excluded_entities([player]);
    let Some(hit) = spatial.cast_ray(eye, dir, USE_RANGE, true, &filter) else { return };
    let body = body_of.get(hit.entity).map_or(hit.entity, |c| c.body);
    if let Ok((prop, tf)) = props.get(body) {
        let mut cols: Vec<Entity> = children.iter_descendants(body).filter(|c| colliders.contains(*c)).collect();
        if colliders.contains(body) {
            cols.push(body);
        }
        for c in &cols {
            ignore.insert(*c);
        }
        let (prop_yaw, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
        let radius = prop.half.length();
        commands.entity(body).insert((
            Held { yaw_offset: prop_yaw - view.yaw, distance: radius + 24.0 * shared::SOURCE_UNIT + 16.0 * shared::SOURCE_UNIT },
            HeldColliders(cols),
            GravityScale(0.0),
        ));
        if let Some(mut ents) = ents {
            ents.fire(prop.ent, "onplayerpickup", Some(PLAYER));
        }
        return;
    }
    // Pedestal buttons and other usable entities: the map entity whose
    // render entity owns the hit collider.
    if let Some(ents) = ents {
        let mut e = Some(body);
        for _ in 0..4 {
            let Some(cur) = e else { break };
            if let Some(i) = ents.list.iter().position(|g| g.entity == Some(cur)) {
                let class = ents.list[i].class.as_str();
                if matches!(class, "prop_button" | "prop_under_button" | "func_button" | "func_rot_button") {
                    fire.write(FireOutput { ent: i, event: "OnPressed".into(), activator: Some(PLAYER) });
                    if class == "func_button" {
                        fire.write(FireOutput { ent: i, event: "OnIn".into(), activator: Some(PLAYER) });
                    }
                }
                break;
            }
            e = body_of.get(cur).ok().map(|c| c.body);
        }
    }
}

/// Carry: float the held prop in front of the eye.
fn carry(
    mut commands: Commands,
    time: Res<Time>,
    mut players: Query<(&InterpolatedFeet, &Crouch, &ViewAngles, &mut IgnoreColliders), With<Player>>,
    mut held: Query<(Entity, &Held, &HeldColliders, &Transform, &mut LinearVelocity, &mut AngularVelocity)>,
) {
    let Ok((feet, crouch, view, mut ignore)) = players.single_mut() else { return };
    let dt = time.delta_secs().max(1e-4);
    for (e, h, hc, tf, mut lv, mut av) in &mut held {
        let eye = feet.curr + Vec3::Y * crouch.eye_height;
        // Portal 2 keeps the held object level, a little below the view line.
        let pitch = view.pitch.clamp(-1.2, 1.2);
        let fwd = Quat::from_euler(EulerRot::YXZ, view.yaw, pitch, 0.0) * Vec3::NEG_Z;
        let target = eye + fwd * h.distance - Vec3::Y * (8.0 * shared::SOURCE_UNIT);
        let to = target - tf.translation;
        if to.length() > HOLD_BREAK_DISTANCE {
            for c in &hc.0 {
                ignore.remove(c);
            }
            commands.entity(e).remove::<(Held, HeldColliders)>().insert(GravityScale(1.0));
            continue;
        }
        let v = to / dt;
        lv.0 = v.clamp_length_max(HOLD_MAX_SPEED);
        // Upright, turned with the player.
        let want = Quat::from_rotation_y(view.yaw + h.yaw_offset);
        let delta = want * tf.rotation.inverse();
        let (axis, mut angle) = delta.to_axis_angle();
        if angle > std::f32::consts::PI {
            angle -= std::f32::consts::TAU;
        }
        av.0 = if axis.is_finite() { axis * angle / dt * 0.5 } else { Vec3::ZERO };
    }
}

/// Props whose centre crosses a linked portal's opening come out of its
/// partner, turned and moving the way the pair maps them.
fn prop_transit(
    mut props: Query<(Entity, &Prop, &mut Transform, &mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity, Has<Held>)>,
    portals: Query<(Entity, &Portal, &Transform, &PortalLink), Without<Prop>>,
    mut last: Local<std::collections::HashMap<Entity, Vec3>>,
    mut teleported: MessageWriter<PortalTeleported>,
) {
    let pairs: Vec<(Entity, Entity, PortalFrame, Vec2, PortalPair, Vec3, Vec3)> = portals
        .iter()
        .filter_map(|(e, p, t, link)| {
            let (_, _, exit_t, _) = portals.get(link.0).ok()?;
            let entry = Portal::frame(t);
            let exit = Portal::frame(exit_t);
            Some((e, link.0, entry, p.half_size, PortalPair::new(entry, exit), entry.normal(), exit.normal()))
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    for (e, _, mut tf, mut pos, mut rot, mut lv, mut av, _) in &mut props {
        seen.insert(e);
        let now = pos.0;
        let before = last.insert(e, now).unwrap_or(now);
        for (from, to, frame, half, pair, n_in, n_out) in &pairs {
            if segment_crosses_opening(frame, *half, before, now).is_none() {
                continue;
            }
            let p = pair.map_point(now);
            let r = pair.map_rotation(rot.0);
            let v = pair.map_dir(lv.0);
            let min = exit_min_speed(*n_out, *n_in, false, None);
            let v = clamp_exit_velocity(v, *n_out, min);
            pos.0 = p;
            rot.0 = r;
            tf.translation = p;
            tf.rotation = r;
            lv.0 = v;
            av.0 = pair.map_dir(av.0);
            last.insert(e, p);
            teleported.write(PortalTeleported { entity: e, from_portal: *from, to_portal: *to, pair: *pair, velocity_out: v });
            break;
        }
    }
    last.retain(|e, _| seen.contains(e));
}

/// While a prop is in front of a linked portal's opening, it does not collide
/// with the wall the portal is on, so it can slide into the hole.
#[derive(Component, Default)]
pub struct PassingThrough(pub Vec<Entity>);

fn wall_passthrough(
    mut commands: Commands,
    props: Query<(Entity, &Position, &Prop, Option<&PassingThrough>)>,
    portals: Query<(&Portal, &Transform), With<PortalLink>>,
) {
    for (e, pos, prop, passing) in &props {
        let mut walls = Vec::new();
        for (p, t) in &portals {
            let f = Portal::frame(t);
            let l = f.to_local(pos.0);
            let r = prop.half.max_element();
            if l.x.abs() < p.half_size.x && l.y.abs() < p.half_size.y && l.z > -r * 2.0 && l.z < r * 2.0 + 0.1 {
                walls.extend(p.surfaces.iter().copied());
            }
        }
        match (walls.is_empty(), passing) {
            (true, Some(_)) => {
                commands.entity(e).remove::<PassingThrough>();
            }
            (false, _) => {
                commands.entity(e).insert(PassingThrough(walls));
            }
            _ => {}
        }
    }
}

/// Collision filter: a prop passing through a portal skips that portal's wall.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PortalHooks<'w, 's> {
    passing: Query<'w, 's, &'static PassingThrough>,
    body_of: Query<'w, 's, &'static ColliderOf>,
}

impl CollisionHooks for PortalHooks<'_, '_> {
    fn filter_pairs(&self, a: Entity, b: Entity, _: &mut Commands) -> bool {
        let body = |c: Entity| self.body_of.get(c).map_or(c, |o| o.body);
        for (x, y) in [(a, b), (b, a)] {
            if let Ok(p) = self.passing.get(body(x)) {
                if p.0.contains(&y) || p.0.contains(&body(y)) {
                    return false;
                }
            }
        }
        true
    }
}

/// Every collider of a prop asks the filter.
pub fn hook_prop_colliders(mut commands: Commands, props: Query<Entity, Added<Prop>>, children: Query<&Children>, colliders: Query<(), With<Collider>>) {
    for p in &props {
        for c in children.iter_descendants(p).chain(std::iter::once(p)) {
            if colliders.contains(c) {
                commands.entity(c).insert(ActiveCollisionHooks::FILTER_PAIRS);
            }
        }
    }
}

/// Inputs on props: dissolve (fizzle) and wake.
fn prop_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    props: Query<(Entity, &Prop)>,
) {
    let Some(ents) = ents else { return };
    for i in inputs.read() {
        let class = ents.list[i.target].class.as_str();
        if !PICKUP_CLASSES.contains(&class) {
            continue;
        }
        if matches!(i.input.as_str(), "dissolve" | "kill" | "break") {
            if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i.target) {
                commands.entity(e).despawn();
            }
        }
    }
}

/// Floor buttons go down under a cube or the player and up again when it leaves.
#[allow(clippy::too_many_arguments)]
fn floor_buttons(
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    props: Query<(&Prop, &Position)>,
    players: Query<&Transform, With<Player>>,
    mut pressed: Local<std::collections::HashSet<usize>>,
    mut fire: MessageWriter<FireOutput>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        pressed.clear();
    }
    let (Some(ents), Some(_)) = (ents, map) else { return };
    let u = shared::SOURCE_UNIT;
    let player = players.single().ok().map(|t| t.translation);
    for (i, e) in ents.list.iter().enumerate() {
        let is_cube_button = e.class == "prop_floor_cube_button" || e.class == "prop_floor_ball_button";
        if !(e.class == "prop_floor_button" || e.class == "prop_under_floor_button" || is_cube_button) || e.killed {
            continue;
        }
        let inv = e.transform.rotation.inverse();
        let on = |p: Vec3, lift: f32| {
            let l = inv * (p - e.transform.translation);
            Vec2::new(l.x, l.z).length() < 40.0 * u && l.y > -8.0 * u && l.y < lift
        };
        let mut down = props.iter().any(|(pr, pos)| {
            let class = ents.list[pr.ent].class.as_str();
            (class == "prop_weighted_cube" || class == "prop_monster_box" || !is_cube_button) && on(pos.0, pr.half.y + 24.0 * u)
        });
        if !is_cube_button {
            down |= player.is_some_and(|p| on(p, 24.0 * u));
        }
        let was = pressed.contains(&i);
        if down && !was {
            pressed.insert(i);
            fire.write(FireOutput { ent: i, event: "OnPressed".into(), activator: Some(PLAYER) });
        } else if !down && was {
            pressed.remove(&i);
            fire.write(FireOutput { ent: i, event: "OnUnPressed".into(), activator: Some(PLAYER) });
        }
    }
}
