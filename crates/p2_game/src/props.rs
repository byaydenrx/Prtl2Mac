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
            .init_resource::<CoreState>()
            .init_resource::<HoldTargets>()
            .init_resource::<HoldObstructions>()
            .add_systems(Update, (make_props_dynamic, hook_prop_colliders, sync_prop_transforms, core_inputs, use_key, prop_inputs, floor_buttons).chain().in_set(crate::level::GameSet::Logic))
            .add_systems(Update, player_proxy_outputs.in_set(crate::level::GameSet::Logic))
            .add_systems(FixedUpdate, (hold_obstructions, carry, hold_clearance, prop_transit, wall_passthrough).chain().after(P2Systems::Transit))
            .add_systems(FixedUpdate, held_prop_blocks_player.before(P2Systems::Move));
    }
}

/// Things the player can pick up.
pub const PICKUP_CLASSES: &[&str] = &["prop_weighted_cube", "prop_monster_box", "prop_physics", "prop_physics_override", "npc_portal_turret_floor", "prop_paint_bomb"];
/// How far the use key reaches (Portal 2 `player_use_dist` ~ 80 u from the eye).
const USE_RANGE: f32 = 80.0 * shared::SOURCE_UNIT;
/// Fastest a held object moves to its target (m/s).
const HOLD_MAX_SPEED: f32 = 1000.0 * shared::SOURCE_UNIT;
/// How quickly a held object closes the gap to its hold point (s).
const HOLD_CATCH_UP: f32 = 0.08;
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
    /// How long it has been too far from its hold point (stuck on something).
    pub stuck_for: f32,
}

/// The prop's colliders the player ignores while carrying it.
#[derive(Component, Default)]
struct HeldColliders(Vec<Entity>);

/// Put prop `body` in the player's hands: the player stops colliding with
/// its colliders and it floats (saved games put a held prop back this way).
pub fn start_holding(commands: &mut Commands, body: Entity, held: Held, children: &Query<&Children>, colliders: &Query<(), With<Collider>>, ignore: &mut IgnoreColliders) {
    let mut cols: Vec<Entity> = children.iter_descendants(body).filter(|c| colliders.contains(*c)).collect();
    if colliders.contains(body) {
        cols.push(body);
    }
    for c in &cols {
        ignore.insert(*c);
    }
    commands.entity(body).insert((held, HeldColliders(cols), GravityScale(0.0)));
}

/// Map props become dynamic bodies once the level is up.
pub fn make_props_dynamic(
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
        // Personality cores ride rails and sockets (kinematic) until the map
        // lets them fall or the player takes them.
        let core = e.class == "npc_personality_core";
        if !PICKUP_CLASSES.contains(&e.class.as_str()) && !core {
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
        if core && lo.x > hi.x {
            // No collision model: a ball about Wheatley's size.
            commands.spawn((Name::new("core collider"), Collider::sphere(14.0 * shared::SOURCE_UNIT), Transform::IDENTITY, ChildOf(entity)));
        }
        commands.entity(entity).remove::<ChildOf>().insert((
            tf,
            if core { RigidBody::Kinematic } else { RigidBody::Dynamic },
            // vphysics cube: 40 kg-ish, slides a little, rolls very little.
            Mass(40.0),
            Friction::new(0.6),
            Restitution::new(0.05),
            LinearDamping(0.05),
            AngularDamping(0.3),
            // Never asleep: world pieces they rest on move without waking
            // them (a dropper's iris opening, a func_brush switched off).
            SleepingDisabled,
            // Fast cubes (droppers, faith plates) don't tunnel through thin
            // things like light bridges.
            SweptCcd::default(),
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
    mut ents: Option<ResMut<Ents>>,
    mut fire: MessageWriter<FireOutput>,
    cores: Res<CoreState>,
) {
    let Ok((player, mut input, feet, crouch, view, mut ignore)) = players.single_mut() else { return };
    if !input.use_key {
        return;
    }
    input.use_key = false;
    if let Ok((e, hc)) = held.single() {
        // Some scenes won't let go of what you carry (Wheatley in intro7).
        if cores.drop_disabled {
            return;
        }
        for c in &hc.0 {
            ignore.remove(c);
        }
        // Let go as aimed: it keeps its drift but not the hold controller's
        // turning (a reflection cube set down stays pointed where it was).
        commands.entity(e).remove::<(Held, HeldColliders)>().insert((GravityScale(1.0), AngularVelocity::ZERO));
        return;
    }
    let eye = feet.curr + Vec3::Y * crouch.eye_height;
    let dir = Dir3::new(view.forward()).unwrap_or(Dir3::NEG_Z);
    let filter = SpatialQueryFilter::from_excluded_entities([player]);
    let hit = spatial.cast_ray(eye, dir, USE_RANGE, true, &filter);
    // Like Source's FindUseEntity: a usable button in front of the player
    // wins over whatever surface the ray happens to hit first (the bed's
    // mattress covers the bed's button).
    if let Some(ents) = ents.as_mut() {
        let hit_dist = hit.map_or(f32::MAX, |h| h.distance);
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in ents.list.iter().enumerate() {
            let usable_door = matches!(e.class.as_str(), "func_door" | "func_door_rotating" | "prop_door_rotating") && e.spawnflags & 256 != 0;
            if e.killed || !(usable_door || matches!(e.class.as_str(), "func_button" | "func_rot_button" | "prop_button" | "prop_under_button")) {
                continue;
            }
            // Brush entities have world bounds; model props get a box around
            // them (a pedestal button is about 48 units tall).
            let (lo, hi) = match (e.brush_model, e.bounds) {
                (Some(_), Some(b)) => b,
                _ => {
                    let p = e.transform.translation;
                    (p - Vec3::new(0.4, 0.05, 0.4), p + Vec3::new(0.4, 1.4, 0.4))
                }
            };
            let closest = eye.clamp(lo, hi);
            let d = closest.distance(eye);
            let toward = ((lo + hi) * 0.5 - eye).normalize_or_zero();
            if d <= USE_RANGE + 0.3 && toward.dot(*dir) > 0.8 && d <= hit_dist + 0.6 && best.is_none_or(|b| d < b.1) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            let class = ents.list[i].class.clone();
            let locked = ents.list[i].value == 0.0;
            if class.contains("door") {
                // "Use opens" doors: a locked one says so, else it opens.
                if locked {
                    fire.write(FireOutput { ent: i, event: "OnLockedUse".into(), activator: Some(PLAYER) });
                } else {
                    ents.input_to(i, "open");
                }
            } else if locked {
                fire.write(FireOutput { ent: i, event: "OnUseLocked".into(), activator: Some(PLAYER) });
            } else {
                fire.write(FireOutput { ent: i, event: "OnPressed".into(), activator: Some(PLAYER) });
                if class == "func_button" {
                    fire.write(FireOutput { ent: i, event: "OnIn".into(), activator: Some(PLAYER) });
                }
                if class == "func_rot_button" {
                    // It turns in and fires OnIn on arrival (rotators.rs).
                    ents.input_to(i, "use");
                }
            }
            return;
        }
    }
    if std::env::var_os("P2_TRACE").is_some() {
        info!("TRACE use: eye {:.2?} dir {:.2?} hit {:?}", eye, dir, hit.map(|h| (h.entity, h.distance, body_of.get(h.entity).ok().map(|c| c.body))));
    }
    let Some(hit) = hit else { return };
    let body = body_of.get(hit.entity).map_or(hit.entity, |c| c.body);
    if let Ok((prop, tf)) = props.get(body) {
        let is_core = ents.as_deref().is_some_and(|e| e.list[prop.ent].class == "npc_personality_core");
        if is_core && !cores.can_pick_up(prop.ent) {
            return;
        }
        if is_core {
            commands.entity(body).insert(RigidBody::Dynamic);
        }
        let (prop_yaw, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
        let radius = prop.half.length();
        // A held reflection cube turns its lens the way the player looks
        // (its +X out along the view), so its beam goes where you aim.
        let reflective = ents.as_deref().is_some_and(|e| crate::lasers::reflective(e, prop.ent));
        let yaw_offset = if reflective {
            std::f32::consts::FRAC_PI_2
        } else if is_core {
            CORE_CARRY_YAW
        } else {
            prop_yaw - view.yaw
        };
        let held = Held { stuck_for: 0.0, yaw_offset, distance: hold_distance(radius) };
        start_holding(&mut commands, body, held, &children, &colliders, &mut ignore);
        if let Some(mut ents) = ents {
            ents.fire(prop.ent, "onplayerpickup", Some(PLAYER));
            ents.fire(prop.ent, "onphysgunpickup", Some(PLAYER));
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
                    if ents.list[i].value == 0.0 {
                        fire.write(FireOutput { ent: i, event: "OnUseLocked".into(), activator: Some(PLAYER) });
                    } else {
                        fire.write(FireOutput { ent: i, event: "OnPressed".into(), activator: Some(PLAYER) });
                        if class == "func_button" {
                            fire.write(FireOutput { ent: i, event: "OnIn".into(), activator: Some(PLAYER) });
                        }
                    }
                }
                break;
            }
            e = body_of.get(cur).ok().map(|c| c.body);
        }
    }
}

/// How far in front of the eye a prop of this size is held.
fn hold_distance(radius: f32) -> f32 {
    radius + 24.0 * shared::SOURCE_UNIT + 16.0 * shared::SOURCE_UNIT
}

/// Where a held prop floats: Portal 2 keeps it level, a little below the
/// view line.
fn hold_point(eye: Vec3, view: &ViewAngles, distance: f32) -> Vec3 {
    let pitch = view.pitch.clamp(-1.2, 1.2);
    let fwd = Quat::from_euler(EulerRot::YXZ, view.yaw, pitch, 0.0) * Vec3::NEG_Z;
    eye + fwd * distance - Vec3::Y * (8.0 * shared::SOURCE_UNIT)
}

/// Carry: float the held prop in front of the eye. Looking through a
/// portal, the hold point is carried through it (Portal 2 lets you hold a
/// cube on the far side of a portal you're looking into).
/// Carried personality cores face the player, as in Portal 2: their eye
/// looks along the body's local -Z (the rig's root bone turns the model's
/// +X front to +Y), so turn them half way round from the view.
const CORE_CARRY_YAW: f32 = std::f32::consts::PI;

fn carry(
    mut commands: Commands,
    time: Res<Time>,
    mut players: Query<(Entity, &InterpolatedFeet, &Crouch, &ViewAngles, &mut IgnoreColliders, &CharacterVelocity), With<Player>>,
    mut held: Query<(Entity, &mut Held, &HeldColliders, &mut Transform, &mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity)>,
    portals: Query<(&Portal, &Transform, &PortalLink), Without<Held>>,
    all_portals: Query<&Transform, (With<Portal>, Without<Held>)>,
    mut teleported: MessageReader<PortalTeleported>,
    mut targets: ResMut<HoldTargets>,
    obstructions: Res<HoldObstructions>,
) {
    targets.0.clear();
    let Ok((player, feet, crouch, view, mut ignore, player_vel)) = players.single_mut() else { return };
    let player_vel = player_vel.0;
    // The player went through a portal: what they hold comes with them.
    for t in teleported.read() {
        if t.entity != player {
            continue;
        }
        for (_, _, _, mut tf, mut pos, mut rot, mut lv, _) in &mut held {
            // Only what is still on the player's side: a prop that went
            // through ahead of the player (pulled into the portal) is already
            // at the exit and would be mapped a second time.
            if !still_at_entry(&t.pair, tf.translation) {
                continue;
            }
            let p = t.pair.map_point(tf.translation);
            let r = t.pair.map_rotation(tf.rotation);
            tf.translation = p;
            tf.rotation = r;
            pos.0 = p;
            rot.0 = r;
            lv.0 = t.pair.map_dir(lv.0);
        }
    }
    let dt = time.delta_secs().max(1e-4);
    for (e, mut h, hc, tf, _, _, mut lv, mut av) in &mut held {
        let eye = feet.curr + Vec3::Y * crouch.eye_height;
        let mut target = hold_point(eye, view, h.distance);
        let mut want = Quat::from_rotation_y(view.yaw + h.yaw_offset);
        // The hold line passes through a portal: the hold point has a copy on
        // the far side. Like Source's grab controller, pull the prop toward
        // the copy on its own side: still on the player's side it heads into
        // the opening (and goes through), already through it follows the
        // far copy.
        let mut through_portal = false;
        for (p, ptf, link) in &portals {
            let Ok(exit) = all_portals.get(link.0) else { continue };
            let frame = Portal::frame(ptf);
            if segment_crosses_opening(&frame, p.half_size, eye, target).is_some() {
                through_portal = true;
                let pair = PortalPair::new(frame, Portal::frame(exit));
                let far = pair.map_point(target);
                if far.distance_squared(tf.translation) < target.distance_squared(tf.translation) {
                    target = far;
                    want = pair.map_rotation(want);
                }
                break;
            }
        }
        // A wall (or anything static) between the eye and the hold point:
        // like Source's grab controller, the hold point comes in front of
        // it, so walking into a wall brings the prop closer instead of
        // pinning it there until it drops (T-105).
        if let (false, Some(&(hit, support))) = (through_portal, obstructions.0.get(&e)) {
            target = in_front_of_obstruction(eye, target, Some(hit), support);
        }
        let reach = target - tf.translation;
        targets.0.insert(e, target);
        let to = target - tf.translation;
        // Dropped only when it stays too far (stuck behind something), not
        // when the player whips around.
        h.stuck_for = if reach.length() > HOLD_BREAK_DISTANCE { h.stuck_for + dt } else { 0.0 };
        if h.stuck_for > 0.5 {
            for c in &hc.0 {
                ignore.remove(c);
            }
            commands.entity(e).remove::<(Held, HeldColliders)>().insert(GravityScale(1.0));
            continue;
        }
        // Close the gap in about a tenth of a second, as Source's held-object
        // controller does, instead of snapping every tick (jitter).
        // On top of the player's own motion, so it keeps up in flight
        // (faith plates, falls).
        let v = to / dt.max(HOLD_CATCH_UP);
        lv.0 = player_vel + v.clamp_length_max(HOLD_MAX_SPEED);
        let delta = want * tf.rotation.inverse();
        let (axis, mut angle) = delta.to_axis_angle();
        if angle > std::f32::consts::PI {
            angle -= std::f32::consts::TAU;
        }
        av.0 = if axis.is_finite() { axis * angle / dt.max(HOLD_CATCH_UP) } else { Vec3::ZERO };
    }
}

/// How far a box of half extents `half` turned by `rot` reaches along `dir`.
fn box_support(half: Vec3, rot: Quat, dir: Vec3) -> f32 {
    half.x * (rot * Vec3::X).dot(dir).abs() + half.y * (rot * Vec3::Y).dot(dir).abs() + half.z * (rot * Vec3::Z).dot(dir).abs()
}

/// The hold point `target`, pulled back toward `eye` when something solid
/// is `hit` metres along the eye-to-target line, so a prop reaching
/// `support` along it stays in front of the surface (never behind the eye).
fn in_front_of_obstruction(eye: Vec3, target: Vec3, hit: Option<f32>, support: f32) -> Vec3 {
    let Some(d) = hit else { return target };
    let to = target - eye;
    let len = to.length();
    let keep = (d - support - 0.5 * shared::SOURCE_UNIT).clamp(0.0, len);
    if len <= 1e-6 { target } else { eye + to * (keep / len) }
}

/// Room the player keeps from a wall ahead while holding a prop: their own
/// radius plus the prop's depth along the view, so the prop fits in front
/// of the eye (T-167).
fn hold_wall_clearance(support: f32) -> f32 {
    (16.0 + 1.0) * shared::SOURCE_UNIT + 2.0 * support
}

/// Player velocity `v` with its part toward a wall of normal `n` (facing
/// the player) limited so the player stops `gap` metres further on, or
/// backs off when `gap` is negative (at most 60 u/s).
fn blocked_by_held(v: Vec3, n: Vec3, gap: f32, dt: f32) -> Vec3 {
    let toward = -v.dot(n);
    let max = (gap / dt.max(1e-4)).max(-60.0 * shared::SOURCE_UNIT);
    if toward > max { v + n * (toward - max) } else { v }
}

/// Portal 2's held prop is solid to the player: walking into a wall with a
/// cube stops the player where the cube meets the wall, instead of the
/// prop coming back over the camera. Here the player ignores the prop's
/// colliders, so the stop comes from a horizontal ray along the view
/// against the static world (not when it goes into a portal).
fn held_prop_blocks_player(
    time: Res<Time>,
    mut players: Query<(Entity, &InterpolatedFeet, &Crouch, &ViewAngles, &IgnoreColliders, &mut CharacterVelocity), With<Player>>,
    held: Query<(Entity, &HeldColliders, &Prop, &Transform, Option<&PassingThrough>), With<Held>>,
    portals: Query<(&Portal, &Transform), With<PortalLink>>,
    spatial: SpatialQuery,
    bodies: Query<&RigidBody>,
    body_of: Query<&ColliderOf>,
) {
    let Ok((player, feet, crouch, view, ignore, mut vel)) = players.single_mut() else { return };
    let eye = feet.curr + Vec3::Y * crouch.eye_height;
    let fwd = Vec3::new(-view.yaw.sin(), 0.0, -view.yaw.cos());
    let Ok(dir) = Dir3::new(fwd) else { return };
    for (e, hc, prop, tf, passing) in &held {
        let need = hold_wall_clearance(box_support(prop.half, tf.rotation, fwd));
        if portals.iter().any(|(p, ptf)| segment_crosses_opening(&Portal::frame(ptf), p.half_size, eye, eye + fwd * need).is_some()) {
            continue;
        }
        let mut skip: Vec<Entity> = hc.0.clone();
        skip.extend([e, player]);
        skip.extend(ignore.0.iter().copied());
        let filter = SpatialQueryFilter::from_excluded_entities(skip);
        let walls: &[Entity] = passing.map_or(&[], |p| &p.0);
        let solid = |c: Entity| {
            let body = body_of.get(c).map_or(c, |b| b.body);
            !matches!(bodies.get(body), Ok(RigidBody::Dynamic)) && !walls.contains(&c) && !walls.contains(&body)
        };
        let reach = need + 64.0 * shared::SOURCE_UNIT;
        let Some(hit) = spatial.cast_ray_predicate(eye, dir, reach, true, &filter, &solid) else { continue };
        let n = Vec3::new(hit.normal.x, 0.0, hit.normal.z);
        if n.length() < 0.5 {
            continue;
        }
        vel.0 = blocked_by_held(vel.0, n.normalize(), hit.distance - need, time.delta_secs());
    }
}

/// A held prop is still on the entry side of `pair` (nearer the entry
/// portal than the exit), so it goes through with the player.
fn still_at_entry(pair: &PortalPair, p: Vec3) -> bool {
    p.distance_squared(pair.entry.position) <= p.distance_squared(pair.exit.position)
}

/// Per held prop, how far along the eye-to-hold-point line something
/// static is in the way, and how far the prop reaches along that line
/// ([`hold_obstructions`], used by [`carry`]).
#[derive(Resource, Default)]
struct HoldObstructions(std::collections::HashMap<Entity, (f32, f32)>);

/// Ray from the eye to each held prop's hold point against the static
/// world (a separate system: the spatial query can't share [`carry`]'s
/// mutable positions).
fn hold_obstructions(
    mut out: ResMut<HoldObstructions>,
    players: Query<(Entity, &InterpolatedFeet, &Crouch, &ViewAngles), With<Player>>,
    held: Query<(Entity, &Held, &HeldColliders, &Prop, &Transform, Option<&PassingThrough>)>,
    spatial: SpatialQuery,
    bodies: Query<&RigidBody>,
    body_of: Query<&ColliderOf>,
) {
    out.0.clear();
    let Ok((player, feet, crouch, view)) = players.single() else { return };
    let eye = feet.curr + Vec3::Y * crouch.eye_height;
    for (e, h, hc, prop, tf, passing) in &held {
        let to = hold_point(eye, view, h.distance) - eye;
        let Ok(dir) = Dir3::new(to) else { continue };
        let mut skip: Vec<Entity> = hc.0.clone();
        skip.extend([e, player]);
        let filter = SpatialQueryFilter::from_excluded_entities(skip);
        let walls: &[Entity] = passing.map_or(&[], |p| &p.0);
        let solid = |c: Entity| {
            let body = body_of.get(c).map_or(c, |b| b.body);
            !matches!(bodies.get(body), Ok(RigidBody::Dynamic)) && !walls.contains(&c) && !walls.contains(&body)
        };
        if let Some(hit) = spatial.cast_ray_predicate(eye, dir, to.length(), true, &filter, &solid) {
            out.0.insert(e, (hit.distance, box_support(prop.half, tf.rotation, *dir)));
        }
    }
}

/// Where each held prop is being pulled this tick ([`carry`]).
#[derive(Resource, Default)]
struct HoldTargets(std::collections::HashMap<Entity, Vec3>);

/// A held prop stops at the world rather than being driven into it:
/// pressed onto a floor button it would sink in and pop out when let go.
/// Other physics props are still pushed.
fn hold_clearance(
    targets: Res<HoldTargets>,
    time: Res<Time>,
    players: Query<Entity, With<Player>>,
    mut held: Query<(Entity, &Prop, &HeldColliders, &Transform, &mut LinearVelocity, Option<&PassingThrough>), With<Held>>,
    spatial: SpatialQuery,
    bodies: Query<&RigidBody>,
    body_of: Query<&ColliderOf>,
) {
    let dt = time.delta_secs().max(1e-4);
    let player = players.single().ok();
    for (e, prop, hc, tf, mut lv, passing) in &mut held {
        let Some(&target) = targets.0.get(&e) else { continue };
        let reach = target - tf.translation;
        let Ok(dir) = Dir3::new(reach) else { continue };
        let shape = Collider::cuboid(prop.half.x * 2.0, prop.half.y * 2.0, prop.half.z * 2.0);
        let mut skip: Vec<Entity> = hc.0.clone();
        skip.push(e);
        skip.extend(player);
        let filter = SpatialQueryFilter::from_excluded_entities(skip);
        let config = ShapeCastConfig { max_distance: reach.length(), ignore_origin_penetration: true, ..default() };
        // The wall around a portal it is entering doesn't stop it.
        let walls: &[Entity] = passing.map_or(&[], |p| &p.0);
        let solid = |c: Entity| {
            let body = body_of.get(c).map_or(c, |b| b.body);
            !matches!(bodies.get(body), Ok(RigidBody::Dynamic)) && !walls.contains(&c) && !walls.contains(&body)
        };
        if let Some(hit) = spatial.cast_shape_predicate(&shape, tf.translation, tf.rotation, dir, &config, &filter, &solid) {
            // Up to the surface, then the rest slides along it.
            let free = (hit.distance - 0.25 * shared::SOURCE_UNIT).max(0.0);
            let n = if hit.normal2.dot(*dir) > 0.0 { -hit.normal2 } else { hit.normal2 };
            let rest = reach - *dir * free;
            let to = *dir * free + rest - n * rest.dot(n).min(0.0);
            lv.0 = (to / dt.max(HOLD_CATCH_UP)).clamp_length_max(HOLD_MAX_SPEED);
        }
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
        // Moved by a teleport (carried through with the player), not by
        // flying through an opening.
        if before.distance(now) > 2.0 {
            continue;
        }
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

impl PortalHooks<'_, '_> {
    /// Is one of the pair a prop passing through a portal in the other?
    fn passing(&self, a: Entity, b: Entity) -> bool {
        let body = |c: Entity| self.body_of.get(c).map_or(c, |o| o.body);
        [(a, b), (b, a)].into_iter().any(|(x, y)| self.passing.get(body(x)).is_ok_and(|p| p.0.contains(&y) || p.0.contains(&body(y))))
    }
}

impl CollisionHooks for PortalHooks<'_, '_> {
    fn filter_pairs(&self, a: Entity, b: Entity, _: &mut Commands) -> bool {
        !self.passing(a, b)
    }

    /// Pairs are only filtered when they start: a cube already resting on
    /// the floor where a portal opens under it keeps its contact (it would
    /// sit on the portal). Dropping the contact every step lets it fall in.
    fn modify_contacts(&self, contacts: &mut ContactPair, _: &mut Commands) -> bool {
        !self.passing(contacts.collider1, contacts.collider2)
    }
}

/// Every collider of a prop asks the filter.
pub fn hook_prop_colliders(mut commands: Commands, props: Query<Entity, Added<Prop>>, children: Query<&Children>, colliders: Query<(), With<Collider>>) {
    for p in &props {
        for c in children.iter_descendants(p).chain(std::iter::once(p)) {
            if colliders.contains(c) {
                commands.entity(c).insert(ActiveCollisionHooks::FILTER_PAIRS | ActiveCollisionHooks::MODIFY_CONTACTS);
            }
        }
    }
}

/// The entity table follows the physics: where a prop is now (scripts'
/// GetOrigin, parenting a carried core onto a socket, lasers).
fn sync_prop_transforms(ents: Option<ResMut<Ents>>, props: Query<(&Prop, &Transform), Changed<Transform>>) {
    let Some(mut ents) = ents else { return };
    for (p, tf) in &props {
        if let Some(e) = ents.list.get_mut(p.ent) {
            e.transform.translation = tf.translation;
            e.transform.rotation = tf.rotation;
        }
    }
}

/// Personality cores the map has made unpickable, and whether the player
/// may drop what they hold (`logic_playerproxy` SetDropEnabled).
#[derive(Resource, Default)]
pub struct CoreState {
    /// Cores are pickable until DisablePickup (sp_a1_intro7 disables Wheatley
    /// at map spawn; sp_a1_wakeup never sends EnablePickup, so dropped there
    /// he must stay pickable: T-167).
    pub pickup_disabled: std::collections::HashSet<usize>,
    pub drop_disabled: bool,
    /// Cores whose motion the map turned off (DisableMotion): taken off
    /// their parent they stay put instead of falling.
    pub motion_disabled: std::collections::HashSet<usize>,
}

impl CoreState {
    pub fn can_pick_up(&self, core: usize) -> bool {
        !self.pickup_disabled.contains(&core)
    }
}

/// A core locked into a socket or onto a rail. Taken from the player's
/// hands it must get its gravity back (carrying zeroes it), or once the
/// socket ejects it and EnableMotion makes it dynamic it hangs in the air
/// (sp_a1_intro7: Wheatley after the secret panel, T-133).
fn locked_core() -> (RigidBody, LinearVelocity, AngularVelocity, GravityScale) {
    (RigidBody::Kinematic, LinearVelocity::ZERO, AngularVelocity::ZERO, GravityScale(1.0))
}

fn core_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut cores: ResMut<CoreState>,
    props: Query<(Entity, &Prop)>,
    mut loaded: MessageReader<LevelLoaded>,
    mut players: Query<(&mut IgnoreColliders, &InterpolatedFeet, &Crouch, &ViewAngles), With<Player>>,
    held: Query<&HeldColliders>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    mut fire: MessageWriter<FireOutput>,
) {
    if loaded.read().count() > 0 {
        *cores = CoreState::default();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let i = msg.target;
        match (ents.list[i].class.as_str(), msg.input.as_str()) {
            ("npc_personality_core", "enablepickup") => {
                cores.pickup_disabled.remove(&i);
            }
            ("npc_personality_core", "disablepickup") => {
                cores.pickup_disabled.insert(i);
            }
            ("npc_personality_core", "enablemotion" | "wake") => {
                cores.motion_disabled.remove(&i);
                if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i) {
                    commands.entity(e).insert(RigidBody::Dynamic);
                }
            }
            // Locked into a socket or onto a rail: out of the player's hands.
            ("npc_personality_core", "disablemotion") => {
                cores.motion_disabled.insert(i);
                if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i) {
                    if let (Ok(hc), Ok((mut ignore, ..))) = (held.get(e), players.single_mut()) {
                        for c in &hc.0 {
                            ignore.remove(c);
                        }
                    }
                    commands.entity(e).remove::<(Held, HeldColliders)>().insert(locked_core());
                }
            }
            // Off its rail or arm: a core is a physics object that only
            // followed its parent, so it falls (sp_a1_intro7: Wheatley drops
            // from the management rail on "three"), unless its motion is off.
            ("npc_personality_core", "clearparent") => {
                let frozen = cores.motion_disabled.contains(&i) || ents.list[i].spawnflags & 8 != 0;
                if !frozen {
                    if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i) {
                        commands.entity(e).insert(RigidBody::Dynamic);
                    }
                }
            }
            // Put straight into the player's hands (sp_a1_wakeup starts
            // with Wheatley carried).
            (_, "forcepickup") => {
                let Some((e, prop)) = props.iter().find(|(_, p)| p.ent == i) else { continue };
                let Ok((mut ignore, feet, crouch, view)) = players.single_mut() else { continue };
                if held.contains(e) {
                    continue;
                }
                let mut cols: Vec<Entity> = children.iter_descendants(e).filter(|c| colliders.contains(*c)).collect();
                if colliders.contains(e) {
                    cols.push(e);
                }
                for c in &cols {
                    ignore.insert(*c);
                }
                let distance = hold_distance(prop.half.length());
                let at = hold_point(feet.curr + Vec3::Y * crouch.eye_height, view, distance);
                // Facing the player, as Wheatley is carried.
                let yaw_offset = CORE_CARRY_YAW;
                let rot = Quat::from_rotation_y(view.yaw + yaw_offset);
                commands.entity(e).insert((
                    RigidBody::Dynamic,
                    Transform::from_translation(at).with_rotation(rot),
                    Position(at),
                    Rotation(rot),
                    LinearVelocity::ZERO,
                    AngularVelocity::ZERO,
                    Held { stuck_for: 0.0, yaw_offset, distance },
                    HeldColliders(cols),
                    GravityScale(0.0),
                ));
                fire.write(FireOutput { ent: i, event: "OnPlayerPickup".into(), activator: Some(PLAYER) });
                fire.write(FireOutput { ent: i, event: "OnPhysGunPickup".into(), activator: Some(PLAYER) });
            }
            ("logic_playerproxy", "setdropenabled") => cores.drop_disabled = msg.param.trim() == "0",
            _ => {}
        }
    }
}

/// Inputs on props: dissolve (fizzle) and kill.
fn prop_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<ResMut<Ents>>,
    props: Query<(Entity, &Prop)>,
) {
    let Some(mut ents) = ents else {
        inputs.clear();
        return;
    };
    for i in inputs.read() {
        let class = ents.list[i.target].class.as_str();
        if !PICKUP_CLASSES.contains(&class) {
            continue;
        }
        if matches!(i.input.as_str(), "dissolve" | "kill" | "break" | "silentdissolve") {
            if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i.target) {
                commands.entity(e).despawn();
            }
            if i.input.contains("dissolve") {
                dissolved(&mut ents, i.target);
            }
            ents.list[i.target].killed = true;
        }
    }
}

/// A prop fizzled: it fires `OnFizzled` (cube droppers send a new cube) and is gone.
pub fn dissolved(ents: &mut Ents, ent: usize) {
    if !ents.list[ent].killed {
        ents.fire(ent, "onfizzled", None);
        ents.list[ent].killed = true;
    }
}

/// Seconds a floor button must stay clear before it lets go, so a cube
/// bouncing as it settles doesn't press and release it again.
const BUTTON_RELEASE_DELAY: f32 = 0.15;

/// Whether a pressed button lets go now: clear for long enough. `clear_since`
/// tracks when it last became clear.
fn button_released(down: bool, now: f32, clear_since: &mut Option<f32>) -> bool {
    if down {
        *clear_since = None;
        return false;
    }
    now - *clear_since.get_or_insert(now) >= BUTTON_RELEASE_DELAY
}

/// Floor buttons go down under a cube or the player and up again when it
/// leaves, playing their model's press and release.
#[allow(clippy::too_many_arguments)]
fn floor_buttons(
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    props: Query<(&Prop, &Position)>,
    players: Query<&Transform, With<Player>>,
    time: Res<Time>,
    mut pressed: Local<std::collections::HashMap<usize, Option<f32>>>,
    mut fire: MessageWriter<FireOutput>,
    mut anim: MessageWriter<crate::io::EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        pressed.clear();
    }
    let now = time.elapsed_secs();
    let button = |ent: usize, param: &str| crate::io::EntInput { target: ent, input: "floorbutton".into(), param: param.into(), activator: None };
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
            let Some(pe) = ents.list.get(pr.ent) else { return false };
            let class = pe.class.as_str();
            (class == "prop_weighted_cube" || class == "prop_monster_box" || !is_cube_button) && on(pos.0, pr.half.y + 24.0 * u)
        });
        if !is_cube_button {
            down |= player.is_some_and(|p| on(p, 24.0 * u));
        }
        match pressed.get_mut(&i) {
            None if down => {
                pressed.insert(i, None);
                fire.write(FireOutput { ent: i, event: "OnPressed".into(), activator: Some(PLAYER) });
                anim.write(button(i, "down"));
            }
            Some(clear_since) => {
                if button_released(down, now, clear_since) {
                    pressed.remove(&i);
                    fire.write(FireOutput { ent: i, event: "OnUnPressed".into(), activator: Some(PLAYER) });
                    anim.write(button(i, "up"));
                }
            }
            None => {}
        }
    }
}

/// `logic_playerproxy` outputs for the player's buttons: OnJump (sp_a1_intro1's
/// "press space to talk" for Wheatley), OnDuck / OnUnDuck.
fn player_proxy_outputs(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    bindings: Option<Res<p2_core::InputBindings>>,
    ents: Option<Res<Ents>>,
    players: Query<(&MoveInput, &Crouch), With<Player>>,
    mut last: Local<(bool, bool)>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(ents) = ents else { return };
    let Ok((input, crouch)) = players.single() else { return };
    let default = p2_core::InputBindings::default();
    let b = bindings.as_deref().unwrap_or(&default);
    let pressed = keys.as_deref().is_some_and(|k| {
        let state = p2_core::InputState { keys: Some(k), mouse: mouse.as_deref(), scroll: None };
        b.just_pressed("+jump", &state)
    });
    // Also a scripted jump (autopilot sets the input directly).
    let jumped = pressed || (input.jump && !last.0);
    let ducked = crouch.wants_duck;
    let mut events: Vec<&str> = Vec::new();
    if jumped {
        events.push("OnJump");
    }
    if ducked && !last.1 {
        events.push("OnDuck");
    }
    if !ducked && last.1 {
        events.push("OnUnDuck");
    }
    *last = (input.jump, ducked);
    if events.is_empty() {
        return;
    }
    for (i, e) in ents.list.iter().enumerate() {
        if e.class == "logic_playerproxy" && !e.killed {
            for ev in &events {
                fire.write(FireOutput { ent: i, event: (*ev).into(), activator: Some(PLAYER) });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn button_waits_before_letting_go() {
        let mut clear = None;
        assert!(!super::button_released(true, 0.0, &mut clear));
        assert!(!super::button_released(false, 1.0, &mut clear), "a bounce doesn't release");
        assert!(!super::button_released(true, 1.05, &mut clear), "landing again resets the wait");
        assert!(!super::button_released(false, 1.1, &mut clear));
        assert!(super::button_released(false, 1.3, &mut clear), "clear for longer than the delay");
    }

    #[test]
    fn cores_pickable_until_disabled() {
        let mut cores = super::CoreState::default();
        assert!(cores.can_pick_up(3), "no EnablePickup needed (sp_a1_wakeup)");
        cores.pickup_disabled.insert(3);
        assert!(!cores.can_pick_up(3));
        cores.pickup_disabled.remove(&3);
        assert!(cores.can_pick_up(3));
    }

    #[test]
    fn locked_core_keeps_its_gravity_for_the_eject() {
        let (body, _, _, gravity) = super::locked_core();
        assert_eq!(body, avian3d::prelude::RigidBody::Kinematic);
        assert_eq!(gravity.0, 1.0, "a core carried into a socket must fall once ejected");
    }

    use super::*;

    #[test]
    fn held_prop_stops_the_player_at_a_wall() {
        let n = Vec3::Z; // wall facing +Z, player walking -Z into it
        let v = Vec3::new(1.0, 0.0, -3.0);
        // Far away: unchanged.
        assert_eq!(blocked_by_held(v, n, 1.0, 0.015), v);
        // 1 cm left: only 0.01 m this tick toward the wall, sideways kept.
        let b = blocked_by_held(v, n, 0.01, 0.015);
        assert!((b.z + 0.01 / 0.015).abs() < 1e-4 && b.x == 1.0, "{b}");
        // Too close: backs off, capped at 60 u/s.
        let b = blocked_by_held(v, n, -0.5, 0.015);
        assert!((b.z - 60.0 * shared::SOURCE_UNIT).abs() < 1e-4, "{b}");
        // Walking away is never slowed.
        assert_eq!(blocked_by_held(-v, n, -0.5, 0.015).z, 3.0);
        // A 50 u cube needs 17 + 50 u from the eye.
        assert!((hold_wall_clearance(25.0 * shared::SOURCE_UNIT) - 67.0 * shared::SOURCE_UNIT).abs() < 1e-5);
    }

    #[test]
    fn hold_point_comes_in_front_of_a_wall() {
        let eye = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -2.0);
        // Nothing in the way: unchanged.
        assert_eq!(in_front_of_obstruction(eye, target, None, 0.5), target);
        // A wall 1.5 m ahead, prop reaching 0.5 m: centre just under 1 m out.
        let t = in_front_of_obstruction(eye, target, Some(1.5), 0.5);
        assert!(t.z < -0.95 && t.z > -1.0, "{t}");
        // Wall nearer than the prop's reach: at the eye, never behind it.
        assert_eq!(in_front_of_obstruction(eye, target, Some(0.3), 0.5), eye);
        // A 1 m cube turned 45 degrees reaches sqrt(2)/2 along X.
        let s = box_support(Vec3::splat(0.5), Quat::from_rotation_y(std::f32::consts::FRAC_PI_4), Vec3::X);
        assert!((s - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    }

    #[test]
    fn held_prop_maps_with_the_player_only_once() {
        // Entry faces +Z at the origin, exit faces +X far away.
        let entry = PortalFrame::new(Vec3::ZERO, Quat::IDENTITY);
        let exit = PortalFrame::new(Vec3::new(50.0, 0.0, 0.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let pair = PortalPair::new(entry, exit);
        // In front of the entry, on the player's side: goes along.
        let near = Vec3::new(0.0, 0.0, 1.0);
        assert!(still_at_entry(&pair, near));
        // Already through (just out of the exit): left where it is.
        let through = pair.map_point(Vec3::new(0.0, 0.0, -0.3));
        assert!(!still_at_entry(&pair, through));
    }
}
