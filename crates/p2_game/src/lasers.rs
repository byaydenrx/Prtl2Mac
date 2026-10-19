//! Thermal discouragement beams: `env_portal_laser` shoots a beam that
//! passes through linked portals, bounces off reflection cubes, powers
//! `prop_laser_catcher`s it hits and `prop_laser_relay`s it passes through
//! (`OnPowered` / `OnUnpowered`), and burns and shoves the player.
use std::collections::{HashMap, HashSet};

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::level::{GameSet, LevelLoaded, LevelOwned};
use crate::props::{Held, Prop};

pub struct LasersPlugin;

impl Plugin for LasersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LaserState>()
            .add_systems(Update, (laser_inputs, trace_lasers, crate::turrets::run_turrets, draw_beams).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

const U: f32 = shared::SOURCE_UNIT;
/// Longest a beam leg reaches.
const RANGE: f32 = 8192.0 * U;
/// Most legs (portals and cubes) one beam follows.
const MAX_LEGS: usize = 12;
/// How far off (cosine) a redirected beam may point and still be pulled
/// onto a catcher, relay or portal (about 10 degrees).
const AUTOAIM_COS: f32 = 0.985;
/// Damage per second while standing in a beam.
const BURN: f32 = 40.0;

#[derive(Resource, Default)]
pub struct LaserState {
    /// Turret sight lines to draw this frame (thin), written by `turrets.rs`.
    pub sights: Vec<(Vec3, Vec3)>,
    /// Entities a beam ends on this frame (turrets burn).
    pub hit: Vec<usize>,
    /// Emitters switched on (by ent index).
    on: HashSet<usize>,
    /// Catchers and relays powered last frame.
    powered: HashSet<usize>,
    /// Beam legs to draw: (from, to).
    legs: Vec<(Vec3, Vec3)>,
}

/// Is this cube a reflection cube (cubetype 2, or the older model)?
pub fn reflective(ents: &Ents, i: usize) -> bool {
    let e = &ents.list[i];
    e.class == "prop_weighted_cube"
        && (source_assets::map::cube_type(e.get("cubetype"), e.get("newskins"), e.get("skin")) == 2 || e.get("model").is_some_and(|m| m.to_ascii_lowercase().contains("reflection_cube")))
}

fn laser_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut state: ResMut<LaserState>, mut loaded: MessageReader<LevelLoaded>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    if loaded.read().count() > 0 {
        state.on.clear();
        state.powered.clear();
        for (i, e) in ents.list.iter().enumerate() {
            // StartState 1 = off until TurnOn.
            if e.class == "env_portal_laser" && !e.get("startstate").is_some_and(|v| v.trim() == "1") {
                state.on.insert(i);
            }
        }
    }
    for msg in inputs.read() {
        if ents.list[msg.target].class != "env_portal_laser" {
            continue;
        }
        match msg.input.as_str() {
            "turnon" => {
                state.on.insert(msg.target);
            }
            "turnoff" | "kill" => {
                state.on.remove(&msg.target);
            }
            "toggle" => {
                if !state.on.remove(&msg.target) {
                    state.on.insert(msg.target);
                }
            }
            _ => {}
        }
    }
}

/// Closest distance between segments ab and cd.
fn segment_distance(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
    let (u, v, w) = (b - a, d - c, a - c);
    let (aa, bb, cc, dd, ee) = (u.dot(u), u.dot(v), v.dot(v), u.dot(w), v.dot(w));
    let den = aa * cc - bb * bb;
    let mut s = if den > 1e-8 { ((bb * ee - cc * dd) / den).clamp(0.0, 1.0) } else { 0.0 };
    let mut t = if cc > 1e-8 { ((bb * s + ee) / cc).clamp(0.0, 1.0) } else { 0.0 };
    if aa > 1e-8 {
        s = ((bb * t - dd) / aa).clamp(0.0, 1.0);
    }
    t = if cc > 1e-8 { ((bb * s + ee) / cc).clamp(0.0, 1.0) } else { 0.0 };
    ((a + u * s) - (c + v * t)).length()
}

/// Where along `o + d t` the ray enters the box, if it does.
fn ray_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3) -> Option<f32> {
    let inv = d.recip();
    let (t1, t2) = ((lo - o) * inv, (hi - o) * inv);
    let (tmin, tmax) = (t1.min(t2).max_element(), t1.max(t2).min_element());
    (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
}

#[allow(clippy::too_many_arguments)]
fn trace_lasers(
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut state: ResMut<LaserState>,
    spatial: SpatialQuery,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    props: Query<(&Prop, &Position, &Rotation, Has<Held>)>,
    portals: Query<(&Portal, &Transform, &PortalLink)>,
    mut players: Query<(Entity, &Transform, &Crouch, &mut CharacterVelocity), With<Player>>,
    mut fire: MessageWriter<FireOutput>,
    mut damage: MessageWriter<crate::health::DamagePlayer>,
) {
    state.legs.clear();
    state.hit.clear();
    let Some(ents) = ents else { return };
    if state.on.is_empty() && state.powered.is_empty() {
        return;
    }
    // Map entity owning each spawned entity.
    let owner: HashMap<Entity, usize> = ents.list.iter().enumerate().filter_map(|(i, e)| e.entity.map(|x| (x, i))).collect();
    let ent_of = |mut e: Entity| {
        for _ in 0..4 {
            if let Some(i) = owner.get(&e) {
                return Some(*i);
            }
            e = parents.get(e).ok()?.parent();
        }
        None
    };
    let prop_at: HashMap<usize, (Vec3, Quat, bool)> = props.iter().map(|(p, pos, rot, held)| (p.ent, (pos.0, rot.0, held))).collect();
    // Relays never stop a beam; emitters don't block themselves.
    let mut excluded: Vec<Entity> = Vec::new();
    let relays: Vec<usize> = ents.list.iter().enumerate().filter(|(_, e)| e.class == "prop_laser_relay" && !e.killed).map(|(i, _)| i).collect();
    for &r in &relays {
        if let Some(e) = ents.list[r].entity {
            excluded.extend(children.iter_descendants(e).filter(|c| colliders.contains(*c)));
        }
    }
    let windows: Vec<(PortalFrame, Vec2, PortalFrame, Vec<Entity>)> = portals
        .iter()
        .filter_map(|(p, t, link)| {
            let (_, exit_t, _) = portals.get(link.0).ok()?;
            let (exit_p, _, _) = portals.get(link.0).ok()?;
            Some((Portal::frame(t), p.half_size, Portal::frame(exit_t), exit_p.surfaces.clone()))
        })
        .collect();
    // Auto-aim targets (Portal 2 bends a redirected beam onto a catcher,
    // relay or portal it nearly points at): (point, facing or zero).
    let mut aim_targets: Vec<(Vec3, Vec3)> = Vec::new();
    for e in ents.list.iter().filter(|e| !e.killed) {
        let r = e.transform.rotation;
        match e.class.as_str() {
            "prop_laser_catcher" => aim_targets.push((e.transform.translation + r * Vec3::X * (8.0 * U), r * Vec3::X)),
            "prop_laser_relay" => aim_targets.push((e.transform.translation + r * Vec3::Y * (32.0 * U), Vec3::ZERO)),
            _ => {}
        }
    }
    for (frame, _, _, _) in &windows {
        aim_targets.push((frame.position, frame.normal()));
    }
    let autoaim = |o: Vec3, dir: Vec3, skip: &[Entity]| -> Vec3 {
        let filter = SpatialQueryFilter::from_excluded_entities(skip.iter().copied()).with_mask(LayerMask(!source_assets::plugin::PLAYER_CLIP_LAYER));
        let mut best: Option<(f32, Vec3)> = None;
        for &(p, facing) in &aim_targets {
            let to = p - o;
            let dist = to.length();
            if dist < 32.0 * U {
                continue;
            }
            let d = to / dist;
            let c = d.dot(dir);
            if c < AUTOAIM_COS || facing.dot(d) > -0.3 && facing != Vec3::ZERO || best.is_some_and(|b| c <= b.0) {
                continue;
            }
            // Only onto targets in plain sight.
            let clear = spatial.cast_ray(o, Dir3::new(d).unwrap_or(Dir3::X), dist, true, &filter).is_none_or(|h| h.distance > dist - 24.0 * U);
            if clear {
                best = Some((c, d));
            }
        }
        best.map_or(dir, |b| b.1)
    };
    let player = players.single_mut().ok();
    let (player_e, player_box) = match &player {
        Some((e, tf, crouch, _)) => {
            let h = crouch.hull() * 0.5;
            (Some(*e), Some((tf.translation - Vec3::new(h.x, 0.0, h.z), tf.translation + Vec3::new(h.x, h.y * 2.0, h.z))))
        }
        None => (None, None),
    };
    if let Some(p) = player_e {
        excluded.push(p);
    }
    let mut powered: HashSet<usize> = HashSet::new();
    let mut burn: Option<Vec3> = None;
    let on: Vec<usize> = state.on.iter().copied().collect();
    for emitter in on {
        let e = &ents.list[emitter];
        if e.killed {
            continue;
        }
        let tf = e.entity.and_then(|x| globals.get(x).ok()).map(|g| g.compute_transform()).unwrap_or(e.transform);
        let mut dir = (tf.rotation * Vec3::X).normalize();
        let mut o = tf.translation + dir * (8.0 * U);
        let mut skip: Vec<Entity> = excluded.clone();
        if let Some(x) = e.entity {
            skip.extend(children.iter_descendants(x).filter(|c| colliders.contains(*c)));
        }
        for _ in 0..MAX_LEGS {
            // Beams pass player clips (they stand in front of catchers).
            let filter = SpatialQueryFilter::from_excluded_entities(skip.iter().copied()).with_mask(LayerMask(!source_assets::plugin::PLAYER_CLIP_LAYER));
            let hit = spatial.cast_ray(o, Dir3::new(dir).unwrap_or(Dir3::X), RANGE, true, &filter);
            let mut len = hit.map_or(RANGE, |h| h.distance);
            // Through the front of a linked portal?
            let mut through: Option<(f32, PortalPair, Vec<Entity>)> = None;
            for (frame, half, exit, exit_walls) in &windows {
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
                    through = Some((t, PortalPair::new(*frame, *exit), exit_walls.clone()));
                }
            }
            // The player in the way?
            let player_t = player_box.and_then(|(lo, hi)| ray_box(o, dir, lo, hi)).filter(|t| *t < through.as_ref().map_or(len, |b| b.0));
            if let Some(t) = player_t {
                len = t;
                state.legs.push((o, o + dir * len));
                burn = Some(dir);
                break;
            }
            if let Some((t, pair, walls)) = through {
                state.legs.push((o, o + dir * t));
                let p = o + dir * t;
                dir = pair.map_dir(dir).normalize();
                o = pair.map_point(p) + dir * 1e-3;
                skip = excluded.clone();
                skip.extend(walls);
                dir = autoaim(o, dir, &skip);
                continue;
            }
            let end = o + dir * len;
            state.legs.push((o, end));
            // Relays the leg passes through.
            for &r in &relays {
                let rt = &ents.list[r].transform;
                let up = rt.rotation * Vec3::Y;
                let base = rt.translation + up * (8.0 * U);
                if segment_distance(o, end, base, base + up * (48.0 * U)) < 12.0 * U {
                    powered.insert(r);
                }
            }
            let Some(h) = hit else { break };
            let Some(i) = ent_of(h.entity) else { break };
            state.hit.push(i);
            match ents.list[i].class.as_str() {
                "prop_laser_catcher" => {
                    powered.insert(i);
                }
                "prop_weighted_cube" if reflective(&ents, i) => {
                    let Some((pos, rot, _)) = prop_at.get(&i) else { break };
                    o = *pos;
                    dir = (*rot * Vec3::X).normalize();
                    // The cube's own colliders don't stop its beam.
                    skip = excluded.clone();
                    if let Some(x) = ents.list[i].entity {
                        skip.extend(children.iter_descendants(x).filter(|c| colliders.contains(*c)));
                        skip.push(x);
                    }
                    o += dir * (20.0 * U);
                    dir = autoaim(o, dir, &skip);
                    continue;
                }
                _ => {}
            }
            break;
        }
    }
    // Catchers near the end of a leg count too (their models may not be solid).
    for (i, e) in ents.list.iter().enumerate() {
        if e.class != "prop_laser_catcher" || e.killed || powered.contains(&i) {
            continue;
        }
        let lens = e.transform.translation + e.transform.rotation * Vec3::X * (8.0 * U);
        if state.legs.iter().any(|(a, b)| segment_distance(*a, *b, lens, lens) < 14.0 * U && b.distance(lens) < 40.0 * U) {
            powered.insert(i);
        }
    }
    for &i in powered.difference(&state.powered) {
        fire.write(FireOutput { ent: i, event: "OnPowered".into(), activator: Some(PLAYER) });
    }
    for &i in state.powered.difference(&powered) {
        fire.write(FireOutput { ent: i, event: "OnUnpowered".into(), activator: Some(PLAYER) });
    }
    state.powered = powered;
    if std::env::var_os("P2_TRACE").is_some() && (ents.now * 2.0).fract() < time.delta_secs() * 2.0 {
        let src = |v: Vec3| (v / U).round();
        info!("TRACE lasers: {:?} powered {:?}", state.legs.iter().map(|(a, b)| (src(Vec3::new(a.x, -a.z, a.y)), src(Vec3::new(b.x, -b.z, b.y)))).collect::<Vec<_>>(), state.powered);
    }
    if let (Some(dir), Some((_, _, _, mut vel))) = (burn, player) {
        damage.write(crate::health::DamagePlayer(BURN * time.delta_secs()));
        // Shoved sideways out of the beam.
        let side = Vec3::new(-dir.z, 0.0, dir.x).normalize_or_zero();
        let side = if side.dot(vel.0) < 0.0 { -side } else { side };
        vel.0 += side * 200.0 * U * time.delta_secs() * 10.0;
    }
}

#[derive(Component)]
struct BeamLeg;

#[derive(Default)]
struct BeamAssets {
    mesh: Option<Handle<Mesh>>,
    core: Option<Handle<StandardMaterial>>,
    glow: Option<Handle<StandardMaterial>>,
}

fn draw_beams(
    mut commands: Commands,
    state: Res<LaserState>,
    mut legs: Query<(Entity, &mut Transform, &mut Visibility), With<BeamLeg>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<BeamAssets>,
) {
    if assets.mesh.is_none() {
        assets.mesh = Some(meshes.add(Cylinder::new(1.0, 1.0)));
        assets.core = Some(materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.85, 0.75), unlit: true, fog_enabled: false, ..default() }));
        assets.glow = Some(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.1, 0.05, 0.35),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            fog_enabled: false,
            ..default()
        }));
    }
    // Two cylinders per leg (core and glow); spawn any missing, reuse the rest.
    let legs_list = &state.legs;
    let mut legs_list: Vec<((Vec3, Vec3), f32)> = legs_list.iter().map(|l| (*l, 1.0)).collect();
    legs_list.extend(state.sights.iter().map(|l| (*l, 0.1)));
    let want = legs_list.len() * 2;
    for k in legs.iter().count()..want {
        let glow = k % 2 == 1;
        commands.spawn((
            BeamLeg,
            Mesh3d(assets.mesh.clone().unwrap()),
            MeshMaterial3d(if glow { assets.glow.clone().unwrap() } else { assets.core.clone().unwrap() }),
            Transform::from_scale(Vec3::ZERO),
            Visibility::Hidden,
            LevelOwned,
        ));
    }
    let mut i = 0;
    for (_, mut tf, mut vis) in legs.iter_mut() {
        let leg = i / 2;
        let glow = i % 2 == 1;
        i += 1;
        let Some(((a, b), thick)) = legs_list.get(leg) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let d = *b - *a;
        let r = if glow { 4.0 * U } else { 1.2 * U } * thick;
        *tf = Transform::from_translation((*a + *b) * 0.5)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)))
            .with_scale(Vec3::new(r, d.length(), r));
        *vis = Visibility::Inherited;
    }
    if std::env::var_os("P2_TRACE").is_some() && !state.legs.is_empty() {
        if let Some((_, tf, vis)) = legs.iter().next() {
            debug!("TRACE beam entities {} first {:?} {:?}", legs.iter().count(), tf.translation / U, vis);
        }
    }
}
