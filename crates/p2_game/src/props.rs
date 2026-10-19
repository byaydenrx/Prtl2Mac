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
            .add_systems(Update, (make_props_dynamic, hook_prop_colliders, sync_prop_transforms, core_inputs, use_key, prop_inputs, floor_buttons).chain().in_set(crate::level::GameSet::Logic))
            .add_systems(Update, player_proxy_outputs.in_set(crate::level::GameSet::Logic))
            .add_systems(FixedUpdate, (carry, prop_transit, wall_passthrough).chain().after(P2Systems::Transit));
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
        commands.entity(e).remove::<(Held, HeldColliders)>().insert(GravityScale(1.0));
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
        if is_core && !cores.pickup.contains(&prop.ent) {
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
    mut players: Query<(Entity, &InterpolatedFeet, &Crouch, &ViewAngles, &mut IgnoreColliders), With<Player>>,
    mut held: Query<(Entity, &mut Held, &HeldColliders, &mut Transform, &mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity)>,
    portals: Query<(&Portal, &Transform, &PortalLink), Without<Held>>,
    all_portals: Query<&Transform, (With<Portal>, Without<Held>)>,
    mut teleported: MessageReader<PortalTeleported>,
) {
    let Ok((player, feet, crouch, view, mut ignore)) = players.single_mut() else { return };
    // The player went through a portal: what they hold comes with them.
    for t in teleported.read() {
        if t.entity != player {
            continue;
        }
        for (_, _, _, mut tf, mut pos, mut rot, mut lv, _) in &mut held {
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
        // The hold line passes through a portal: carry the point through.
        for (p, ptf, link) in &portals {
            let Ok(exit) = all_portals.get(link.0) else { continue };
            let frame = Portal::frame(ptf);
            if segment_crosses_opening(&frame, p.half_size, eye, target).is_some() {
                let pair = PortalPair::new(frame, Portal::frame(exit));
                target = pair.map_point(target);
                want = pair.map_rotation(want);
                break;
            }
        }
        let to = target - tf.translation;
        // Dropped only when it stays too far (stuck behind something), not
        // when the player whips around.
        h.stuck_for = if to.length() > HOLD_BREAK_DISTANCE { h.stuck_for + dt } else { 0.0 };
        if h.stuck_for > 0.5 {
            for c in &hc.0 {
                ignore.remove(c);
            }
            commands.entity(e).remove::<(Held, HeldColliders)>().insert(GravityScale(1.0));
            continue;
        }
        // Close the gap in about a tenth of a second, as Source's held-object
        // controller does, instead of snapping every tick (jitter).
        let v = to / dt.max(HOLD_CATCH_UP);
        lv.0 = v.clamp_length_max(HOLD_MAX_SPEED);
        let delta = want * tf.rotation.inverse();
        let (axis, mut angle) = delta.to_axis_angle();
        if angle > std::f32::consts::PI {
            angle -= std::f32::consts::TAU;
        }
        av.0 = if axis.is_finite() { axis * angle / dt.max(HOLD_CATCH_UP) } else { Vec3::ZERO };
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

/// Personality cores the map has made pickable, and whether the player may
/// drop what they hold (`logic_playerproxy` SetDropEnabled).
#[derive(Resource, Default)]
pub struct CoreState {
    pub pickup: std::collections::HashSet<usize>,
    pub drop_disabled: bool,
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
                cores.pickup.insert(i);
            }
            ("npc_personality_core", "disablepickup") => {
                cores.pickup.remove(&i);
            }
            ("npc_personality_core", "enablemotion" | "wake") => {
                if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i) {
                    commands.entity(e).insert(RigidBody::Dynamic);
                }
            }
            // Locked into a socket or onto a rail: out of the player's hands.
            ("npc_personality_core", "disablemotion") => {
                if let Some((e, _)) = props.iter().find(|(_, p)| p.ent == i) {
                    if let (Ok(hc), Ok((mut ignore, ..))) = (held.get(e), players.single_mut()) {
                        for c in &hc.0 {
                            ignore.remove(c);
                        }
                    }
                    commands.entity(e).remove::<(Held, HeldColliders)>().insert((RigidBody::Kinematic, LinearVelocity::ZERO, AngularVelocity::ZERO));
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
            let Some(pe) = ents.list.get(pr.ent) else { return false };
            let class = pe.class.as_str();
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
