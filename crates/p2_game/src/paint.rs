//! Gels (`info_paint_sprayer`): sprayers throw blobs that fly under gravity,
//! pass through portals and paint what they hit. Blue (bounce) throws the
//! player back up on landing and off walls, orange (speed) lets them run
//! much faster with little friction, white (portal) makes any surface take
//! portals, water (erase) washes gel off.
//!
//! The maps pre-paint their surfaces by starting sprayers at spawn and
//! stopping them a moment later (sp_a3_jump_intro), so paint is simulated
//! from the start rather than read from the map.
//!
//! Paint on props, `trigger_paint_cleanser`, `prop_paint_bomb` and
//! `paint_sphere` are in `paint_props.rs`; this file keeps the surfaces.
//! A portal standing only on white gel closes when that gel is washed off
//! or painted over.
//!
//! Approximations (not measured against the game yet): splat size, the
//! bounce and speed-gel tuning below (each constant says where its number
//! comes from), blob streaks (`blob_streak_percentage`...: blobs just
//! splat), the player's own view getting splashed (not done).
use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents};
use crate::level::{LevelLoaded, LevelOwned};

pub struct PaintPlugin;

impl Plugin for PaintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paint>()
            .add_message::<PaintSurface>()
            .add_message::<crate::paint_props::PaintProp>()
            .add_systems(
                Update,
                (sprayer_inputs, spray, fly_blobs, paint_requests, sync_portal_gel, drop_unsupported_portals).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic),
            )
            .add_systems(FixedUpdate, (speed_gel.before(P2Systems::Move), bounce_gel.after(P2Systems::Move).before(P2Systems::Transit)));
    }
}

/// Source units to metres.
const U: f32 = shared::SOURCE_UNIT;
/// Radius one blob paints, u.
const SPLAT_RADIUS: f32 = 26.0;
/// Gravity on blobs, u/s^2 (`sv_gravity`).
const GRAVITY: f32 = 600.0;
/// Upward speed a blue floor gives at the least (walking or dropping a
/// little onto it), u/s. Estimate: Portal 2 bounces a player who walks onto
/// blue gel roughly a crouch-jump's worth of headroom over a standing
/// player (~170 u, = 450^2 / (2 * 600)); not measured.
pub const BOUNCE_MIN_SPEED: f32 = 450.0;
/// Speed kept off a blue floor: Portal 2 gives back the whole speed of the
/// fall, so a player bounces back to the height they fell from, over and
/// over (published behaviour, "Repulsion Gel"); 1.0 is that.
pub const BOUNCE_KEEP: f32 = 1.0;
/// Upward speed added when bouncing off a blue wall, u/s. Estimate: enough
/// to clear a little height between wall jumps, as the game does.
pub const BOUNCE_WALL_UP_SPEED: f32 = 200.0;
/// Slowest run into a blue wall that bounces, u/s (walking along one doesn't). Estimate.
pub const BOUNCE_WALL_MIN_SPEED: f32 = 50.0;
/// Running speed on orange gel, u/s. Estimate: published descriptions put
/// Propulsion Gel running at several times the 175 u/s walk; 800 u/s is
/// enough for the speed-ramp jumps in sp_a3_speed_ramp (to be checked there).
pub const SPEED_GEL_MAX: f32 = 800.0;
/// Acceleration factor on orange gel (`sv_accelerate` is 10 off it):
/// speed builds up over a few seconds instead of at once. Estimate.
pub const SPEED_GEL_ACCELERATE: f32 = 1.0;
/// Friction on orange gel (`sv_friction` is 4 off it): little, so the
/// player slides on when they let go. Estimate.
pub const SPEED_GEL_FRICTION: f32 = 0.3;
/// Most splats kept; the oldest go first.
const MAX_SPLATS: usize = 8000;
/// Most blobs in flight over all sprayers.
const MAX_BLOBS: usize = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gel {
    Bounce,
    Speed,
    Portal,
    Erase,
}

impl Gel {
    /// `painttype`: 0 bounce, 1 (unused stick/reflection) as bounce, 2 speed, 3 portal, 4 water.
    pub fn from_type(t: i32) -> Gel {
        match t {
            2 => Gel::Speed,
            3 => Gel::Portal,
            4 => Gel::Erase,
            _ => Gel::Bounce,
        }
    }
    /// A prop's `paintpower` (same numbers; 4 is no paint).
    pub fn from_power(t: i32) -> Option<Gel> {
        (0..=3).contains(&t).then(|| Gel::from_type(t))
    }
    fn index(self) -> usize {
        self as usize
    }
    pub fn color(self) -> Color {
        match self {
            Gel::Bounce => Color::srgb(0.02, 0.22, 0.72),
            Gel::Speed => Color::srgb(0.85, 0.36, 0.02),
            Gel::Portal => Color::srgb(0.72, 0.72, 0.7),
            Gel::Erase => Color::srgba(0.55, 0.65, 0.75, 0.6),
        }
    }
}

struct Blob {
    pos: Vec3,
    vel: Vec3,
    gel: Gel,
    paints: bool,
    age: f32,
    entity: Entity,
}

struct Splat {
    pos: Vec3,
    normal: Vec3,
    gel: Gel,
    entity: Entity,
}

#[derive(Default)]
struct Sprayer {
    active: bool,
    owed: f32,
    live: usize,
    /// Set by `ChangePaintType`; else the `painttype` keyvalue.
    gel: Option<Gel>,
}

/// Paint a surface point (bombs, `paint_sphere`): handled like a blob landing there.
#[derive(Message, Clone, Copy, Debug)]
pub struct PaintSurface {
    pub pos: Vec3,
    pub normal: Vec3,
    pub gel: Gel,
}

/// The first solid thing a list of ray hits reaches: not a trigger volume,
/// nearest first.
pub fn first_solid(mut hits: Vec<RayHitData>, sensors: &Query<(), With<Sensor>>) -> Option<RayHitData> {
    hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    hits.into_iter().find(|h| !sensors.contains(h.entity))
}

/// The speed off a blue surface with normal `n`, arriving at `v_in` (any
/// units): the part along the surface stays, the part into it comes back
/// out (times [`BOUNCE_KEEP`]), and at least `min` out.
pub fn bounce_off(v_in: Vec3, n: Vec3, min: f32) -> Vec3 {
    let into = (-v_in.dot(n)).max(0.0);
    let out = (into * BOUNCE_KEEP).max(min);
    v_in + n * (into + out)
}

/// How high a bounce at `speed` u/s straight up goes, u (`sv_gravity` 600).
pub fn bounce_height(speed: f32) -> f32 {
    speed * speed / (2.0 * GRAVITY)
}

#[derive(Resource, Default)]
pub struct Paint {
    sprayers: HashMap<usize, Sprayer>,
    blobs: Vec<(usize, Blob)>,
    splats: Vec<Splat>,
    /// Splat index by grid cell (half a splat wide) for merging.
    grid: HashMap<IVec3, Vec<usize>>,
    dirty: bool,
    /// White gel went away (washed, painted over): check the portals on it.
    portal_gel_lost: bool,
    rng: u32,
    look: Option<Look>,
}

struct Look {
    disc: Handle<Mesh>,
    ball: Handle<Mesh>,
    mats: [Handle<StandardMaterial>; 4],
}

impl Paint {
    fn rand(&mut self) -> f32 {
        if self.rng == 0 {
            self.rng = 0x9e37_79b9;
        }
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 1_000_000) as f32 / 1_000_000.0
    }
    fn cell(p: Vec3) -> IVec3 {
        (p / (SPLAT_RADIUS * U * 0.5)).floor().as_ivec3()
    }
    fn rebuild_grid(&mut self) {
        self.grid.clear();
        for (i, s) in self.splats.iter().enumerate() {
            self.grid.entry(Self::cell(s.pos)).or_default().push(i);
        }
    }
    /// The gel on a surface facing `normal` at `p` (within `reach` metres
    /// of a splat's edge), if any.
    pub fn gel_at(&self, p: Vec3, normal: Vec3, reach: f32, min_dot: f32) -> Option<(Gel, Vec3)> {
        let r = SPLAT_RADIUS * U + reach;
        let mut best: Option<(f32, Gel, Vec3)> = None;
        for s in &self.splats {
            if s.gel == Gel::Erase || s.normal.dot(normal) < min_dot {
                continue;
            }
            let d = p - s.pos;
            let along = d.dot(s.normal);
            if along.abs() > 6.0 * U + reach {
                continue;
            }
            let flat = (d - s.normal * along).length();
            if flat <= r && best.is_none_or(|b| flat < b.0) {
                best = Some((flat, s.gel, s.normal));
            }
        }
        best.map(|b| (b.1, b.2))
    }
}

fn reset_on_load(paint: &mut Paint) {
    paint.sprayers.clear();
    paint.blobs.clear();
    paint.splats.clear();
    paint.grid.clear();
    paint.dirty = true;
}

/// Start / Stop on sprayers (and their initial state).
fn sprayer_inputs(mut paint: ResMut<Paint>, ents: Option<Res<Ents>>, mut inputs: MessageReader<EntInput>, mut loaded: MessageReader<LevelLoaded>) {
    if loaded.read().count() > 0 {
        reset_on_load(&mut paint);
        if let Some(ents) = &ents {
            for (i, e) in ents.list.iter().enumerate() {
                if e.class == "info_paint_sprayer" {
                    let active = e.get("start_active").is_some_and(|v| v.trim() == "1");
                    paint.sprayers.insert(i, Sprayer { active, ..default() });
                }
            }
        }
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class != "info_paint_sprayer" {
            continue;
        }
        let s = paint.sprayers.entry(msg.target).or_default();
        match msg.input.as_str() {
            "start" => s.active = true,
            "stop" => s.active = false,
            "toggle" => s.active = !s.active,
            // ChangePaintType <painttype>.
            "changepainttype" => {
                if let Ok(t) = msg.param.trim().parse::<f32>() {
                    s.gel = Some(Gel::from_type(t as i32));
                }
            }
            _ => {}
        }
    }
}

fn look(paint: &mut Paint, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    if paint.look.is_some() {
        return;
    }
    let mut mat = |g: Gel| materials.add(StandardMaterial { base_color: g.color(), unlit: true, fog_enabled: false, depth_bias: 50.0, ..default() });
    let mats = [mat(Gel::Bounce), mat(Gel::Speed), mat(Gel::Portal), mat(Gel::Erase)];
    paint.look = Some(Look { disc: meshes.add(Circle::new(SPLAT_RADIUS * U)), ball: meshes.add(Sphere::new(2.5 * U)), mats });
}

/// Active sprayers throw `blobs_per_second` blobs in a cone of
/// `blob_spread_angle` from a disc of `blob_spread_radius` at
/// `min_speed`..`max_speed` along their forward, up to `maxblobcount` in
/// flight; `drawonly` blobs don't paint. `silent` (no splat sounds; blobs
/// make none here yet), `light_position_name` and `render_mode` only change
/// how the game draws and sounds them and are not used.
#[allow(clippy::too_many_arguments)]
fn spray(
    mut commands: Commands,
    time: Res<Time>,
    mut paint: ResMut<Paint>,
    ents: Option<Res<Ents>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(ents) = ents else { return };
    let dt = time.delta_secs();
    if dt <= 0.0 || paint.sprayers.values().all(|s| !s.active) {
        return;
    }
    look(&mut paint, &mut meshes, &mut materials);
    let ids: Vec<usize> = paint.sprayers.iter().filter(|(_, s)| s.active).map(|(i, _)| *i).collect();
    for i in ids {
        let e = &ents.list[i];
        if e.killed {
            continue;
        }
        let rate = e.get_f32("blobs_per_second").unwrap_or(20.0);
        let max_live = e.get_f32("maxblobcount").unwrap_or(250.0) as usize;
        let (lo, hi) = (e.get_f32("min_speed").unwrap_or(100.0), e.get_f32("max_speed").unwrap_or(100.0));
        let radius = e.get_f32("blob_spread_radius").unwrap_or(0.0) * U;
        let angle = e.get_f32("blob_spread_angle").unwrap_or(0.0).to_radians();
        let gel = paint.sprayers.get(&i).and_then(|s| s.gel).unwrap_or_else(|| Gel::from_type(e.get("painttype").and_then(|v| v.trim().parse().ok()).unwrap_or(0)));
        let paints = e.get("drawonly").is_none_or(|v| v.trim() != "1");
        let fwd = (e.transform.rotation * Vec3::X).normalize_or(Vec3::NEG_Y);
        let side = fwd.any_orthonormal_vector();
        let up = fwd.cross(side);
        let origin = e.transform.translation;
        let s = paint.sprayers.get_mut(&i).unwrap();
        s.owed += rate * dt;
        let mut n = s.owed.floor() as usize;
        s.owed -= n as f32;
        let live = s.live;
        n = n.min(max_live.saturating_sub(live)).min(MAX_BLOBS.saturating_sub(paint.blobs.len()));
        for _ in 0..n {
            let (a, b, c, d) = (paint.rand(), paint.rand(), paint.rand(), paint.rand());
            let spin = a * std::f32::consts::TAU;
            let off = (side * spin.cos() + up * spin.sin()) * radius * b.sqrt();
            let tilt = angle * 0.5 * c;
            let turn = d * std::f32::consts::TAU;
            let dir = (fwd * tilt.cos() + (side * turn.cos() + up * turn.sin()) * tilt.sin()).normalize();
            let speed = (lo + (hi - lo) * paint.rand()) * U;
            let pos = origin + off;
            let look = paint.look.as_ref().unwrap();
            let entity = commands
                .spawn((Name::new("gel blob"), Mesh3d(look.ball.clone()), MeshMaterial3d(look.mats[gel.index()].clone()), Transform::from_translation(pos), LevelOwned))
                .id();
            paint.blobs.push((i, Blob { pos, vel: dir * speed, gel, paints, age: 0.0, entity }));
        }
        paint.sprayers.get_mut(&i).unwrap().live = live + n;
    }
}

/// Blobs fall, go through linked portals, and paint the first solid
/// surface they hit; a prop they hit takes the gel (the player doesn't).
#[allow(clippy::too_many_arguments)]
fn fly_blobs(
    mut commands: Commands,
    time: Res<Time>,
    mut paint: ResMut<Paint>,
    spatial: SpatialQuery,
    players: Query<Entity, With<Player>>,
    sensors: Query<(), With<Sensor>>,
    bodies: Query<&RigidBody>,
    body_of: Query<&ColliderOf>,
    portals: Query<(&Portal, &Transform, Option<&PortalLink>)>,
    mut transforms: Query<&mut Transform, Without<Portal>>,
    props: Query<(), With<crate::props::Prop>>,
    mut paint_props: MessageWriter<crate::paint_props::PaintProp>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 || paint.blobs.is_empty() {
        return;
    }
    let filter = SpatialQueryFilter::from_excluded_entities(players.iter());
    // Open, linked portals: (entry frame, half size, exit frame).
    let links: Vec<(PortalFrame, Vec2, PortalFrame)> = portals
        .iter()
        .filter_map(|(p, tf, link)| {
            let exit = portals.get(link?.0).ok()?;
            Some((Portal::frame(tf), p.half_size, Portal::frame(exit.1)))
        })
        .collect();
    let mut blobs = std::mem::take(&mut paint.blobs);
    let mut landed: Vec<(Vec3, Vec3, Gel)> = Vec::new();
    blobs.retain_mut(|(sprayer, b)| {
        b.age += dt;
        b.vel.y -= GRAVITY * U * dt;
        let step = b.vel * dt;
        let next = b.pos + step;
        // Through a portal: carried to the other side.
        for (entry, half, exit) in &links {
            let (d0, d1) = (entry.plane_distance(b.pos), entry.plane_distance(next));
            if d0 >= 0.0 && d1 < 0.0 {
                let x = b.pos + step * (d0 / (d0 - d1));
                let l = entry.to_local(x);
                if l.x.abs() <= half.x && l.y.abs() <= half.y {
                    let pair = PortalPair::new(*entry, *exit);
                    b.pos = pair.map_point(next);
                    b.vel = pair.map_dir(b.vel);
                    if let Ok(mut t) = transforms.get_mut(b.entity) {
                        t.translation = b.pos;
                    }
                    return true;
                }
            }
        }
        let len = step.length();
        let hit = Dir3::new(step).ok().and_then(|dir| first_solid(spatial.ray_hits(b.pos, dir, len, 4, true, &filter), &sensors).map(|h| (h, dir)));
        if let Some((h, dir)) = hit {
            let body = body_of.get(h.entity).map_or(h.entity, |c| c.body);
            if b.paints {
                if props.contains(body) {
                    paint_props.write(crate::paint_props::PaintProp { entity: body, gel: b.gel });
                } else if !matches!(bodies.get(body), Ok(RigidBody::Dynamic)) {
                    landed.push((b.pos + dir * h.distance, h.normal, b.gel));
                }
            }
            commands.entity(b.entity).despawn();
            *sprayer = usize::MAX;
            return false;
        }
        if b.age > 8.0 {
            commands.entity(b.entity).despawn();
            *sprayer = usize::MAX;
            return false;
        }
        b.pos = next;
        if let Ok(mut t) = transforms.get_mut(b.entity) {
            t.translation = b.pos;
        }
        true
    });
    // Count each sprayer's blobs still in flight.
    for s in paint.sprayers.values_mut() {
        s.live = 0;
    }
    for (i, _) in &blobs {
        if let Some(s) = paint.sprayers.get_mut(i) {
            s.live += 1;
        }
    }
    paint.blobs = blobs;
    for (p, n, gel) in landed {
        add_splat(&mut commands, &mut paint, p, n, gel);
    }
}

fn add_splat(commands: &mut Commands, paint: &mut Paint, p: Vec3, n: Vec3, gel: Gel) {
    let r = SPLAT_RADIUS * U;
    if gel == Gel::Erase {
        let before = paint.splats.len();
        let mut lost = false;
        paint.splats.retain(|s| {
            let gone = s.pos.distance(p) < r * 1.5;
            if gone {
                lost |= s.gel == Gel::Portal;
                commands.entity(s.entity).despawn();
            }
            !gone
        });
        paint.portal_gel_lost |= lost;
        if paint.splats.len() != before {
            paint.rebuild_grid();
            paint.dirty = true;
        }
        return;
    }
    // Close to a splat on the same surface: recolour it instead.
    let cell = Paint::cell(p);
    let mut near = None;
    'search: for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                for &k in paint.grid.get(&(cell + IVec3::new(dx, dy, dz))).map(Vec::as_slice).unwrap_or(&[]) {
                    let s = &paint.splats[k];
                    if s.pos.distance(p) < r * 0.5 && s.normal.dot(n) > 0.9 {
                        near = Some(k);
                        break 'search;
                    }
                }
            }
        }
    }
    let look = paint.look.as_ref().unwrap();
    if let Some(k) = near {
        if paint.splats[k].gel != gel {
            paint.portal_gel_lost |= paint.splats[k].gel == Gel::Portal;
            paint.splats[k].gel = gel;
            commands.entity(paint.splats[k].entity).insert(MeshMaterial3d(look.mats[gel.index()].clone()));
            paint.dirty = true;
        }
        return;
    }
    let pos = p + n * 0.3 * U;
    let entity = commands
        .spawn((
            Name::new("gel"),
            Mesh3d(look.disc.clone()),
            MeshMaterial3d(look.mats[gel.index()].clone()),
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_arc(Vec3::Z, n)),
            LevelOwned,
        ))
        .id();
    paint.splats.push(Splat { pos: p, normal: n, gel, entity });
    paint.grid.entry(cell).or_default().push(paint.splats.len() - 1);
    if paint.splats.len() > MAX_SPLATS {
        let old = paint.splats.remove(0);
        paint.portal_gel_lost |= old.gel == Gel::Portal;
        commands.entity(old.entity).despawn();
        paint.rebuild_grid();
    }
    paint.dirty = true;
}

/// Paint asked for by bombs and `paint_sphere`.
fn paint_requests(
    mut commands: Commands,
    mut paint: ResMut<Paint>,
    mut requests: MessageReader<PaintSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if requests.is_empty() {
        return;
    }
    look(&mut paint, &mut meshes, &mut materials);
    for r in requests.read() {
        add_splat(&mut commands, &mut paint, r.pos, r.normal, r.gel);
    }
}

/// A portal that stood only on white gel closes when the gel under it goes
/// (washed off or painted over), as in Portal 2. Portals on walls that take
/// portals anyway stay.
#[allow(clippy::too_many_arguments)]
fn drop_unsupported_portals(
    mut commands: Commands,
    mut paint: ResMut<Paint>,
    gel: Res<PortalGel>,
    spatial: SpatialQuery,
    portalable: Query<(), With<Portalable>>,
    body_of: Query<&ColliderOf>,
    sensors: Query<(), With<Sensor>>,
    portals: Query<(&Portal, &Transform)>,
    mut guns: Query<&mut PortalGun, With<Player>>,
    mut closed: MessageWriter<PortalClosed>,
) {
    if !std::mem::take(&mut paint.portal_gel_lost) {
        return;
    }
    let Ok(mut gun) = guns.single_mut() else { return };
    for slot in gun.portals.iter_mut() {
        let Some(p) = *slot else { continue };
        let Ok((portal, tf)) = portals.get(p) else { continue };
        let centre = tf.translation;
        let n = tf.rotation * Vec3::Z;
        if gel.covers(centre) {
            continue;
        }
        // Does the surface it sits on take portals by itself? (The colliders
        // the gun recorded, else whatever is right behind it.)
        let takes = |e: Entity| portalable.contains(e) || body_of.get(e).is_ok_and(|c| portalable.contains(c.body));
        let native = if portal.surfaces.is_empty() {
            let Ok(back) = Dir3::new(-n) else { continue };
            first_solid(spatial.ray_hits(centre + n * 2.0 * U, back, 8.0 * U, 4, true, &SpatialQueryFilter::default()), &sensors).is_some_and(|h| takes(h.entity))
        } else {
            portal.surfaces.iter().any(|&e| takes(e))
        };
        if !native {
            info!("portal2: the white gel under the {:?} portal is gone; it closes", portal.kind);
            closed.write(PortalClosed { kind: portal.kind, transform: *tf });
            commands.entity(p).despawn();
            *slot = None;
        }
    }
}

/// White gel for the portal gun.
fn sync_portal_gel(mut paint: ResMut<Paint>, mut gel: ResMut<PortalGel>) {
    if !paint.dirty {
        return;
    }
    paint.dirty = false;
    gel.0 = paint.splats.iter().filter(|s| s.gel == Gel::Portal).map(|s| (s.pos, s.normal, SPLAT_RADIUS * U)).collect();
}

/// Orange gel underfoot: a much higher top speed, gentle acceleration and
/// little friction (set on the movement tuning for this tick).
fn speed_gel(
    paint: Res<Paint>,
    mut cfg: ResMut<p2_movement::MovementConfig>,
    players: Query<(&Transform, &Grounded), With<Player>>,
    mut base: Local<Option<(f32, f32, f32)>>,
) {
    let (max, acc, fr) = *base.get_or_insert((cfg.max_speed, cfg.accelerate, cfg.friction));
    let on = players.single().is_ok_and(|(tf, g)| g.on_ground && paint.gel_at(tf.translation, Vec3::Y, 8.0 * U, 0.7).is_some_and(|x| x.0 == Gel::Speed));
    let want = if on { (SPEED_GEL_MAX, SPEED_GEL_ACCELERATE, SPEED_GEL_FRICTION) } else { (max, acc, fr) };
    if (cfg.max_speed, cfg.accelerate, cfg.friction) != want {
        (cfg.max_speed, cfg.accelerate, cfg.friction) = want;
    }
}

/// Blue gel: landing on it throws the player back up at least as fast as
/// they came down (crouching stops it); running into a blue wall bounces
/// them off it.
fn bounce_gel(
    paint: Res<Paint>,
    mut players: Query<(&Transform, &mut CharacterVelocity, &mut Grounded, &Crouch, &MoveInput), With<Player>>,
    mut air_vel: Local<Vec3>,
) {
    if paint.splats.is_empty() {
        return;
    }
    let Ok((tf, mut vel, mut grounded, crouch, input)) = players.single_mut() else { return };
    let feet = tf.translation;
    if !grounded.on_ground {
        // The speed it lands with (the move zeroes it on landing).
        *air_vel = vel.0 / U;
    }
    if grounded.on_ground && !input.duck && !crouch.ducked {
        if let Some((Gel::Bounce, n)) = paint.gel_at(feet, Vec3::Y, 8.0 * U, 0.7) {
            // Off the floor's own slope, keeping the run along it.
            let arrive = Vec3::new(vel.0.x / U, air_vel.y.min(0.0), vel.0.z / U);
            let out = bounce_off(arrive, n, BOUNCE_MIN_SPEED);
            vel.0 = out * U;
            grounded.on_ground = false;
            *air_vel = Vec3::ZERO;
            if std::env::var_os("P2_TRACE").is_some() {
                info!("TRACE bounce gel: off at {:.0} u/s (to {:.0} u)", out.y, bounce_height(out.y));
            }
            return;
        }
    }
    // Walls: the hull's side (16 u from its centre) against a blue splat.
    let centre = feet + Vec3::Y * crouch.hull().y * 0.5;
    let flat = Vec3::new(vel.0.x, 0.0, vel.0.z);
    if flat.length() < BOUNCE_WALL_MIN_SPEED * U {
        return;
    }
    let dir = flat.normalize();
    if let Some((Gel::Bounce, n)) = paint.gel_at(centre + dir * 16.0 * U, -dir, 6.0 * U, 0.5) {
        let into = vel.0.dot(n);
        if into < 0.0 && n.y.abs() < 0.7 {
            let mut out = bounce_off(vel.0 / U, n, 0.0);
            out.y = out.y.max(0.0) + BOUNCE_WALL_UP_SPEED;
            vel.0 = out * U;
            grounded.on_ground = false;
            if std::env::var_os("P2_TRACE").is_some() {
                info!("TRACE bounce gel: off a wall");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splat(pos: Vec3, normal: Vec3, gel: Gel) -> Splat {
        Splat { pos, normal, gel, entity: Entity::PLACEHOLDER }
    }

    #[test]
    fn paint_types() {
        assert_eq!(Gel::from_type(0), Gel::Bounce);
        assert_eq!(Gel::from_type(1), Gel::Bounce);
        assert_eq!(Gel::from_type(2), Gel::Speed);
        assert_eq!(Gel::from_type(3), Gel::Portal);
        assert_eq!(Gel::from_type(4), Gel::Erase);
        assert_eq!(Gel::from_power(3), Some(Gel::Portal));
        assert_eq!(Gel::from_power(4), None);
        assert_eq!(Gel::from_power(-1), None);
    }

    #[test]
    fn bounce_gives_back_the_fall() {
        // Dropped from 300 u: lands at sqrt(2 g h) = 600 u/s, bounces back to 300 u.
        let land = (2.0 * GRAVITY * 300.0f32).sqrt();
        let out = bounce_off(Vec3::new(0.0, -land, 0.0), Vec3::Y, BOUNCE_MIN_SPEED);
        assert!((out.y - land).abs() < 1e-3);
        assert!((bounce_height(out.y) - 300.0).abs() < 0.01);
        // Walking on (no fall): the least bounce, about 170 u high.
        let walk = bounce_off(Vec3::new(175.0, 0.0, 0.0), Vec3::Y, BOUNCE_MIN_SPEED);
        assert_eq!(walk, Vec3::new(175.0, BOUNCE_MIN_SPEED, 0.0));
        assert!((bounce_height(walk.y) - 168.75).abs() < 0.01);
        // Speed along the surface is kept.
        let run = bounce_off(Vec3::new(400.0, -700.0, 0.0), Vec3::Y, BOUNCE_MIN_SPEED);
        assert_eq!(run, Vec3::new(400.0, 700.0, 0.0));
    }

    #[test]
    fn bounce_off_slopes_and_walls() {
        // A 45-degree ramp facing +X: falling straight down throws you along +X.
        let n = Vec3::new(1.0, 1.0, 0.0).normalize();
        let out = bounce_off(Vec3::new(0.0, -600.0, 0.0), n, 0.0);
        assert!((out - Vec3::new(600.0, 0.0, 0.0)).length() < 1e-2, "{out:?}");
        // A wall facing -X, run into at 300 u/s: straight back.
        let out = bounce_off(Vec3::new(300.0, 0.0, 50.0), Vec3::NEG_X, 0.0);
        assert_eq!(out, Vec3::new(-300.0, 0.0, 50.0));
        // Moving away from the surface: untouched (but at least `min` out).
        assert_eq!(bounce_off(Vec3::new(0.0, 100.0, 0.0), Vec3::Y, 0.0), Vec3::new(0.0, 100.0, 0.0));
    }

    #[test]
    fn gel_under_a_point() {
        let mut p = Paint::default();
        p.splats.push(splat(Vec3::ZERO, Vec3::Y, Gel::Bounce));
        p.splats.push(splat(Vec3::new(1.0, 0.0, 0.0), Vec3::Y, Gel::Erase));
        p.splats.push(splat(Vec3::new(0.0, 1.0, 2.0), Vec3::Z, Gel::Speed));
        // Standing on the floor splat (feet a little above it).
        assert_eq!(p.gel_at(Vec3::new(0.1, 2.0 * U, 0.0), Vec3::Y, 8.0 * U, 0.7).map(|g| g.0), Some(Gel::Bounce));
        // Out past its edge, or facing the wrong way: none. Water isn't gel.
        assert_eq!(p.gel_at(Vec3::new(2.0, 0.0, 0.0), Vec3::Y, 8.0 * U, 0.7), None);
        assert_eq!(p.gel_at(Vec3::ZERO, Vec3::X, 8.0 * U, 0.7), None);
        // The wall splat, seen from in front of it.
        assert_eq!(p.gel_at(Vec3::new(0.0, 1.0, 2.0 + 4.0 * U), Vec3::Z, 6.0 * U, 0.5).map(|g| g.0), Some(Gel::Speed));
    }
}
