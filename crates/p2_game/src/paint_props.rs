//! Gel on things that move, and the entities that paint or clean in bulk:
//!
//! * Props (cubes, spheres, turrets...) hit by a blob take its gel: a coat of
//!   its colour over the prop, and its power. A blue cube bounces (high
//!   restitution, and off blue floors), an orange one slides (no friction),
//!   a white one only looks white. Water washes the coat off. Map props can
//!   start painted with `paintpower` (0 bounce, 2 speed, 3 portal, 4 none).
//! * `trigger_paint_cleanser` washes the paint off props inside it (while
//!   enabled).
//! * `prop_paint_bomb` (dropped from droppers in the chapter 6-8 paint
//!   chambers) bursts on its first hard impact and paints everything within
//!   [`BOMB_RADIUS`] with its `painttype`, props included, then fires
//!   `OnExploded`. `bombtype` (dry/wet look) isn't told apart.
//! * `paint_sphere`: the `Paint` input paints every surface within its
//!   `radius` with its `painttype`.
//!
//! Approximations: the coat is a translucent box or ball over the prop, not
//! paint on its texture; bomb and sphere coverage comes from a fan of rays
//! (so splats land where rays reach, with gaps far out).
use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::io::{EntInput, Ents, FireOutput};
use crate::level::LevelOwned;
use crate::paint::{first_solid, Gel, Paint, PaintSurface};
use crate::props::{Held, Prop};

pub struct PaintPropsPlugin;

impl Plugin for PaintPropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (painted_from_map, cleansers, bombs, paint_spheres, coat_props, props_on_blue).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

const U: f32 = shared::SOURCE_UNIT;
/// Reach of a paint bomb's burst, u. Estimate from the bomb chambers:
/// one bomb covers a few metres of floor and the walls next to it.
pub const BOMB_RADIUS: f32 = 160.0;
/// Rays in a burst (bomb, paint_sphere): enough that splats (26 u across)
/// overlap out to about 100 u.
pub const BURST_RAYS: usize = 160;
/// A change of velocity this big in one frame is a hit (u/s): far more than
/// gravity gives in a frame (600 u/s^2), less than dropping onto a floor
/// from a dropper.
pub const BOMB_IMPACT: f32 = 120.0;
/// Speed a prop must be falling at to bounce off a blue floor, u/s.
pub const PROP_BOUNCE_MIN: f32 = 100.0;
/// Restitution of a blue-painted prop: it bounces back almost as high as
/// it fell (Portal 2's painted cubes keep bouncing). Estimate.
pub const PAINTED_RESTITUTION: f32 = 0.9;
/// Friction of an orange-painted prop: it slides away when pushed. Estimate.
pub const PAINTED_FRICTION: f32 = 0.02;
/// The coat sits this much (fraction) outside the prop's box.
const COAT_GROW: f32 = 0.04;
/// `paint_sphere` radius when the map doesn't give one, u.
const SPHERE_RADIUS: f32 = 128.0;

/// Paint a prop (`Gel::Erase` washes it).
#[derive(Message, Clone, Copy, Debug)]
pub struct PaintProp {
    pub entity: Entity,
    pub gel: Gel,
}

/// A prop's paint, and how it slid and bounced before it.
#[derive(Component, Clone, Debug)]
pub struct PropPaint {
    pub gel: Gel,
    friction: Friction,
    restitution: Restitution,
    coat: Entity,
}

/// What a prop's physics become with `gel` (`None`: back to its own).
pub fn painted_surface(gel: Gel) -> (Option<Friction>, Option<Restitution>) {
    match gel {
        Gel::Bounce => (None, Some(Restitution::new(PAINTED_RESTITUTION).with_combine_rule(CoefficientCombine::Max))),
        Gel::Speed => (Some(Friction::new(PAINTED_FRICTION).with_combine_rule(CoefficientCombine::Min)), None),
        Gel::Portal | Gel::Erase => (None, None),
    }
}

/// Directions spread evenly over a sphere (a Fibonacci lattice).
pub fn burst_directions(n: usize) -> Vec<Vec3> {
    let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let a = golden * i as f32;
            Vec3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

/// Did a body moving at `before` hit something to be moving at `now` a
/// frame later? (Units per second; `threshold` in the same units.)
pub fn impacted(before: Vec3, now: Vec3, threshold: f32) -> bool {
    (now - before).length() > threshold
}

/// Map props that start painted (`paintpower`), including droppers' copies.
fn painted_from_map(ents: Option<Res<Ents>>, added: Query<(Entity, &Prop), Added<Prop>>, mut paint: MessageWriter<PaintProp>) {
    let Some(ents) = ents else { return };
    for (e, prop) in &added {
        let Some(ent) = ents.list.get(prop.ent) else { continue };
        if ent.class == "prop_paint_bomb" {
            continue;
        }
        if let Some(gel) = ent.get("paintpower").and_then(|v| v.trim().parse::<f32>().ok()).and_then(|t| Gel::from_power(t as i32)) {
            paint.write(PaintProp { entity: e, gel });
        }
    }
}

/// Props inside an enabled `trigger_paint_cleanser` lose their paint.
fn cleansers(ents: Option<Res<Ents>>, props: Query<(Entity, &Prop, &Position), With<PropPaint>>, mut paint: MessageWriter<PaintProp>) {
    let Some(ents) = ents else { return };
    if props.is_empty() {
        return;
    }
    for e in ents.list.iter().filter(|e| e.class == "trigger_paint_cleanser" && e.enabled && !e.killed) {
        let Some((lo, hi)) = e.bounds else { continue };
        for (pe, prop, pos) in &props {
            let (plo, phi) = (pos.0 - prop.half, pos.0 + prop.half);
            if plo.cmple(hi).all() && phi.cmpge(lo).all() {
                paint.write(PaintProp { entity: pe, gel: Gel::Erase });
            }
        }
    }
}

/// Splats on everything a fan of rays from `centre` reaches within
/// `radius` (metres), and paint on the props there.
#[allow(clippy::too_many_arguments)]
fn burst(
    centre: Vec3,
    radius: f32,
    gel: Gel,
    exclude: Vec<Entity>,
    spatial: &SpatialQuery,
    sensors: &Query<(), With<Sensor>>,
    body_of: &Query<&ColliderOf>,
    props: &Query<(Entity, &Prop, &Position, Option<&Held>, &LinearVelocity)>,
    surfaces: &mut MessageWriter<PaintSurface>,
    prop_paint: &mut MessageWriter<PaintProp>,
) {
    let filter = SpatialQueryFilter::from_excluded_entities(exclude.iter().copied());
    let mut painted: Vec<Entity> = Vec::new();
    for d in burst_directions(BURST_RAYS) {
        let Ok(dir) = Dir3::new(d) else { continue };
        let Some(h) = first_solid(spatial.ray_hits(centre, dir, radius, 4, true, &filter), sensors) else { continue };
        let body = body_of.get(h.entity).map_or(h.entity, |c| c.body);
        if props.contains(body) {
            if !painted.contains(&body) {
                painted.push(body);
                prop_paint.write(PaintProp { entity: body, gel });
            }
        } else {
            surfaces.write(PaintSurface { pos: centre + d * h.distance, normal: h.normal, gel });
        }
    }
}

/// Paint bombs burst on their first hard hit.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn bombs(
    mut commands: Commands,
    ents: Option<ResMut<Ents>>,
    spatial: SpatialQuery,
    sensors: Query<(), With<Sensor>>,
    body_of: Query<&ColliderOf>,
    children: Query<&Children>,
    props: Query<(Entity, &Prop, &Position, Option<&Held>, &LinearVelocity)>,
    mut last: Local<HashMap<Entity, Vec3>>,
    mut surfaces: MessageWriter<PaintSurface>,
    mut prop_paint: MessageWriter<PaintProp>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else {
        last.clear();
        return;
    };
    let mut seen = Vec::new();
    let mut burst_now: Vec<(Entity, usize, Vec3)> = Vec::new();
    for (e, prop, pos, held, vel) in &props {
        let Some(ent) = ents.list.get(prop.ent) else { continue };
        if ent.class != "prop_paint_bomb" || ent.killed {
            continue;
        }
        seen.push(e);
        let v = vel.0 / U;
        if let Some(before) = last.insert(e, v) {
            if held.is_none() && impacted(before, v, BOMB_IMPACT) {
                burst_now.push((e, prop.ent, pos.0));
            }
        }
    }
    last.retain(|e, _| seen.contains(e));
    for (e, i, centre) in burst_now {
        let gel = Gel::from_type(ents.list[i].get("painttype").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0) as i32);
        let mut exclude: Vec<Entity> = children.iter_descendants(e).collect();
        exclude.push(e);
        burst(centre, BOMB_RADIUS * U, gel, exclude, &spatial, &sensors, &body_of, &props, &mut surfaces, &mut prop_paint);
        info!("portal2: paint bomb {:?} burst ({gel:?})", ents.list[i].name);
        ents.list[i].killed = true;
        fire.write(FireOutput { ent: i, event: "OnExploded".into(), activator: None });
        commands.entity(e).despawn();
        last.remove(&e);
    }
}

/// `paint_sphere`'s `Paint` input.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn paint_spheres(
    ents: Option<Res<Ents>>,
    mut inputs: MessageReader<EntInput>,
    spatial: SpatialQuery,
    sensors: Query<(), With<Sensor>>,
    body_of: Query<&ColliderOf>,
    props: Query<(Entity, &Prop, &Position, Option<&Held>, &LinearVelocity)>,
    mut surfaces: MessageWriter<PaintSurface>,
    mut prop_paint: MessageWriter<PaintProp>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "paint_sphere" || msg.input != "paint" {
            continue;
        }
        let gel = Gel::from_type(e.get_f32("painttype").unwrap_or(0.0) as i32);
        let radius = e.get_f32("radius").unwrap_or(SPHERE_RADIUS);
        burst(e.transform.translation, radius * U, gel, Vec::new(), &spatial, &sensors, &body_of, &props, &mut surfaces, &mut prop_paint);
    }
}

/// Put a coat on (or wash it off) the props that were painted.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn coat_props(
    mut commands: Commands,
    mut requests: MessageReader<PaintProp>,
    ents: Option<Res<Ents>>,
    props: Query<(&Prop, &Position, &Rotation, Option<&Friction>, Option<&Restitution>, Option<&PropPaint>)>,
    children: Query<&Children>,
    aabbs: Query<&ColliderAabb>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mats: Local<HashMap<Gel, Handle<StandardMaterial>>>,
) {
    // The last request for each prop this frame wins.
    let mut want: Vec<(Entity, Gel)> = Vec::new();
    for r in requests.read() {
        want.retain(|(e, _)| *e != r.entity);
        want.push((r.entity, r.gel));
    }
    for (e, gel) in want {
        let Ok((prop, pos, rot, friction, restitution, painted)) = props.get(e) else { continue };
        if painted.is_some_and(|p| p.gel == gel) || (painted.is_none() && gel == Gel::Erase) {
            continue;
        }
        let Ok(mut ec) = commands.get_entity(e) else { continue };
        // Back to its own surface first.
        if let Some(p) = painted {
            commands.entity(p.coat).despawn();
            commands.entity(e).insert((p.friction, p.restitution)).remove::<PropPaint>();
            ec = commands.entity(e);
        }
        if gel == Gel::Erase {
            continue;
        }
        let base_friction = painted.map_or(friction.copied().unwrap_or_default(), |p| p.friction);
        let base_restitution = painted.map_or(restitution.copied().unwrap_or_default(), |p| p.restitution);
        let (f, r) = painted_surface(gel);
        if let Some(f) = f {
            ec.insert(f);
        }
        if let Some(r) = r {
            ec.insert(r);
        }
        // The coat: a ball for spheres (`cubetype` 3), else a box over the colliders.
        let sphere = ents.as_ref().and_then(|x| x.list.get(prop.ent)).is_some_and(|x| x.get("cubetype").is_some_and(|t| t.trim() == "3") || x.class.contains("sphere"));
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for c in children.iter_descendants(e) {
            if let Ok(a) = aabbs.get(c) {
                lo = lo.min(a.min);
                hi = hi.max(a.max);
            }
        }
        let centre = if lo.x <= hi.x { rot.0.inverse() * ((lo + hi) * 0.5 - pos.0) } else { Vec3::ZERO };
        let half = prop.half * (1.0 + COAT_GROW);
        let mesh = if sphere { meshes.add(Sphere::new(half.max_element())) } else { meshes.add(Cuboid::new(half.x * 2.0, half.y * 2.0, half.z * 2.0)) };
        let mat = mats
            .entry(gel)
            .or_insert_with(|| {
                let c = gel.color().to_srgba();
                materials.add(StandardMaterial { base_color: Color::srgba(c.red, c.green, c.blue, 0.6), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.3, ..default() })
            })
            .clone();
        let coat = commands.spawn((Name::new("gel coat"), Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_translation(centre), ChildOf(e), LevelOwned)).id();
        commands.entity(e).insert(PropPaint { gel, friction: base_friction, restitution: base_restitution, coat });
    }
}

/// Props bounce off blue floors too (a cube dropped on blue gel jumps).
fn props_on_blue(
    paint: Res<Paint>,
    mut props: Query<(Entity, &Prop, &Position, &mut LinearVelocity), Without<Held>>,
    mut last: Local<HashMap<Entity, Vec3>>,
) {
    let mut seen = Vec::new();
    for (e, prop, pos, mut vel) in &mut props {
        seen.push(e);
        let before = last.insert(e, vel.0).unwrap_or(vel.0);
        // It was falling fast and has just been stopped by the floor.
        if before.y / U > -PROP_BOUNCE_MIN || vel.0.y < before.y * 0.5 {
            continue;
        }
        let bottom = pos.0 - Vec3::Y * prop.half.y;
        if let Some((Gel::Bounce, n)) = paint.gel_at(bottom, Vec3::Y, 6.0 * U, 0.7) {
            let out = crate::paint::bounce_off(before / U, n, 0.0) * PAINTED_RESTITUTION;
            vel.0 = out * U;
            last.insert(e, vel.0);
        }
    }
    last.retain(|e, _| seen.contains(e));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bursts_cover_the_sphere_evenly() {
        let d = burst_directions(BURST_RAYS);
        assert_eq!(d.len(), BURST_RAYS);
        assert!(d.iter().all(|v| (v.length() - 1.0).abs() < 1e-4));
        // Every direction has a ray within ~16 degrees.
        for probe in [Vec3::Y, Vec3::NEG_Y, Vec3::X, Vec3::new(1.0, -1.0, 1.0).normalize()] {
            let best = d.iter().map(|v| v.dot(probe)).fold(-1.0f32, f32::max);
            assert!(best > 16f32.to_radians().cos(), "{probe:?}: {best}");
        }
        // As many rays down as up.
        let down = d.iter().filter(|v| v.y < 0.0).count();
        assert_eq!(down, BURST_RAYS / 2);
    }

    #[test]
    fn hits_versus_falling() {
        let dt = 1.0 / 60.0;
        // Falling: gravity changes velocity by 10 u/s a frame.
        assert!(!impacted(Vec3::new(0.0, -300.0, 0.0), Vec3::new(0.0, -310.0, 0.0), BOMB_IMPACT));
        assert!(600.0 * dt < BOMB_IMPACT);
        // Landing from a dropper (a 128 u drop lands at ~390 u/s).
        let land = (2.0 * 600.0 * 128.0f32).sqrt();
        assert!(impacted(Vec3::new(0.0, -land, 0.0), Vec3::ZERO, BOMB_IMPACT));
        // A glancing hit on a wall at speed.
        assert!(impacted(Vec3::new(400.0, -50.0, 0.0), Vec3::new(150.0, -60.0, 0.0), BOMB_IMPACT));
    }

    #[test]
    fn painted_physics() {
        let (f, r) = painted_surface(Gel::Bounce);
        assert!(f.is_none());
        let r = r.unwrap();
        assert_eq!(r.coefficient, PAINTED_RESTITUTION);
        assert_eq!(r.combine_rule, CoefficientCombine::Max);
        let (f, r) = painted_surface(Gel::Speed);
        assert!(r.is_none());
        assert_eq!(f.unwrap().combine_rule, CoefficientCombine::Min);
        assert_eq!(painted_surface(Gel::Portal), (None, None));
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;
    use crate::io::GameEnt;

    pub fn app_for_tests() -> App {
        let mut app = App::new();
        app.set_error_handler(bevy::ecs::error::panic);
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(p2_core::CorePlugin)
            .configure_sets(Update, (crate::level::GameSet::Load, crate::level::GameSet::Logic).chain())
            .add_message::<crate::level::LevelLoaded>()
            .add_message::<EntInput>()
            .add_message::<FireOutput>()
            .init_resource::<p2_movement::MovementConfig>()
            .init_resource::<avian3d::collider_tree::ColliderTrees>()
            .add_systems(Update, crate::io::run_io.in_set(crate::level::GameSet::Logic))
            .add_plugins((crate::paint::PaintPlugin, PaintPropsPlugin));
        app
    }

    fn spawn_prop(app: &mut App, ent: usize, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((Prop { ent, half: Vec3::splat(16.0 * U) }, Position(at), Rotation::default(), LinearVelocity::ZERO, Friction::new(0.6), Restitution::new(0.05), Transform::from_translation(at)))
            .id()
    }

    #[test]
    fn props_take_gel_and_lose_it() {
        let mut app = app_for_tests();
        let mut ents = Ents::default();
        let cube = ents.add(GameEnt::logic("prop_weighted_cube", "cube", vec![]));
        let orange = ents.add(GameEnt::logic("prop_weighted_cube", "orange", vec![("paintpower".into(), "2".into())]));
        let mut cleanser = GameEnt::logic("trigger_paint_cleanser", "wash", vec![]);
        cleanser.bounds = Some((Vec3::new(9.0, -1.0, -1.0), Vec3::new(11.0, 1.0, 1.0)));
        ents.add(cleanser);
        app.insert_resource(ents);
        let a = spawn_prop(&mut app, cube, Vec3::ZERO);
        let b = spawn_prop(&mut app, orange, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        app.update();
        // paintpower 2 from the map: orange, slippery.
        assert_eq!(app.world().get::<PropPaint>(b).map(|p| p.gel), Some(Gel::Speed));
        assert_eq!(app.world().get::<Friction>(b).unwrap().dynamic_coefficient, PAINTED_FRICTION);
        assert!(app.world().get::<PropPaint>(a).is_none());

        // A blob of blue on the cube: it bounces, and wears a coat.
        app.world_mut().write_message(PaintProp { entity: a, gel: Gel::Bounce });
        app.update();
        let p = app.world().get::<PropPaint>(a).cloned().unwrap();
        assert_eq!(p.gel, Gel::Bounce);
        assert_eq!(app.world().get::<Restitution>(a).unwrap().coefficient, PAINTED_RESTITUTION);
        assert!(app.world().get_entity(p.coat).is_ok());
        // Painted over with white: no bounce, still coated.
        app.world_mut().write_message(PaintProp { entity: a, gel: Gel::Portal });
        app.update();
        assert_eq!(app.world().get::<PropPaint>(a).map(|p| p.gel), Some(Gel::Portal));
        assert_eq!(app.world().get::<Restitution>(a).unwrap().coefficient, 0.05);
        assert!(app.world().get_entity(p.coat).is_err());
        // Water washes it: its own surface again.
        app.world_mut().write_message(PaintProp { entity: a, gel: Gel::Erase });
        app.update();
        assert!(app.world().get::<PropPaint>(a).is_none());
        assert_eq!(app.world().get::<Restitution>(a).unwrap().coefficient, 0.05);

        // Carried through the cleanser: the orange cube comes out clean.
        app.world_mut().get_mut::<Position>(b).unwrap().0 = Vec3::new(10.0, 0.0, 0.0);
        app.update();
        app.update();
        assert!(app.world().get::<PropPaint>(b).is_none());
        assert_eq!(app.world().get::<Friction>(b).unwrap().dynamic_coefficient, 0.6);
    }

    #[test]
    fn a_disabled_cleanser_leaves_paint() {
        let mut app = app_for_tests();
        let mut ents = Ents::default();
        let i = ents.add(GameEnt::logic("prop_weighted_cube", "c", vec![("paintpower".into(), "0".into())]));
        let mut cleanser = GameEnt::logic("trigger_paint_cleanser", "wash", vec![]);
        cleanser.bounds = Some((Vec3::splat(-1.0), Vec3::splat(1.0)));
        cleanser.enabled = false;
        ents.add(cleanser);
        app.insert_resource(ents);
        let a = spawn_prop(&mut app, i, Vec3::ZERO);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().get::<PropPaint>(a).map(|p| p.gel), Some(Gel::Bounce));
    }
}

#[cfg(test)]
mod portal_tests {
    use super::*;
    use p2_core::*;

    #[test]
    fn a_portal_on_washed_off_white_gel_closes() {
        let mut app = app_tests_app();
        app.insert_resource(Ents::default());
        // White gel on a wall that doesn't take portals, and on one that does.
        let plain = app.world_mut().spawn(Name::new("plain wall")).id();
        let white_wall = app.world_mut().spawn((Name::new("portal wall"), Portalable)).id();
        for x in [0.0, 10.0] {
            app.world_mut().write_message(PaintSurface { pos: Vec3::new(x, 1.0, 0.0), normal: Vec3::Z, gel: Gel::Portal });
        }
        app.update();
        assert!(app.world().resource::<PortalGel>().covers(Vec3::new(0.0, 1.0, 0.0)));
        let portal = |app: &mut App, kind: PortalKind, x: f32, wall: Entity| {
            app.world_mut().spawn((Portal { kind, half_size: PORTAL_HALF_SIZE, surfaces: vec![wall], placed_at: 0.0 }, Transform::from_xyz(x, 1.0, 0.0))).id()
        };
        let blue = portal(&mut app, PortalKind::Blue, 0.0, plain);
        let orange = portal(&mut app, PortalKind::Orange, 10.0, white_wall);
        let player = app.world_mut().spawn((Player, PortalGun { portals: [Some(blue), Some(orange)], ..PortalGun::full() })).id();
        // Water over both.
        for x in [0.0, 10.0] {
            app.world_mut().write_message(PaintSurface { pos: Vec3::new(x, 1.0, 0.0), normal: Vec3::Z, gel: Gel::Erase });
        }
        app.update();
        app.update();
        let gun = app.world().get::<PortalGun>(player).unwrap();
        // The one held up only by gel is gone; the one on a portal wall stays.
        assert_eq!(gun.portals, [None, Some(orange)]);
        assert!(app.world().get_entity(blue).is_err());
        assert!(!app.world().resource::<PortalGel>().covers(Vec3::new(0.0, 1.0, 0.0)));
    }

    fn app_tests_app() -> App {
        super::app_tests::app_for_tests()
    }
}
