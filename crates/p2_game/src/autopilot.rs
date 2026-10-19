//! Scripted input for testing levels without a person at the keyboard:
//! `P2_SCRIPT="1.0 setpos -704 96 0 0 90; 2 walk 1.5; 4 use; 5 fire blue"`.
//!
//! Each step is `<seconds after the level starts> <command> [args]`:
//! `setpos x y z [pitch yaw]` (Source units/degrees, feet like the game's
//! `setpos`), `look pitch yaw`, `walk secs` (forward), `back secs`,
//! `left secs`, `right secs`, `jump`, `duck secs`, `use`, `fire blue|orange`,
//! `shot path.png`, `log`, `info <name>` (an entity's state, place, axes and bounds; also `ent`), `con <console line>` (runs a developer-console
//! command, e.g. `con sv_cheats 1` then `con noclip`; `;` can't be used inside),
//! `plan x y z half path.png` (a top-down orthographic picture of the map
//! below the Source point x y z, `half` units each way from it: a floor plan
//! cut at height z, north up).
use bevy::prelude::*;
use p2_core::*;

use crate::level::LevelLoaded;

pub struct AutopilotPlugin;

impl Plugin for AutopilotPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(script) = std::env::var("P2_SCRIPT") {
            let steps = parse(&script);
            info!("autopilot: {} steps", steps.len());
            // Scripted runs never read the real keyboard and mouse.
            app.insert_resource(InputLocked(true))
                .insert_resource(Autopilot { steps, start: None, hold: Vec::new() })
                .add_systems(Update, (drive.after(P2Systems::Input).before(P2Systems::Fire), plan_shots));
        }
    }
}

#[derive(Clone, Debug)]
struct Step {
    at: f32,
    cmd: String,
    args: Vec<String>,
    done: bool,
}

#[derive(Resource)]
struct Autopilot {
    steps: Vec<Step>,
    start: Option<f32>,
    /// Held movement: (until, wish vector, duck).
    hold: Vec<(f32, Vec2, bool)>,
}

fn parse(s: &str) -> Vec<Step> {
    s.split(';')
        .filter_map(|part| {
            let mut w = part.split_whitespace();
            let at = w.next()?.parse().ok()?;
            let cmd = w.next()?.to_ascii_lowercase();
            Some(Step { at, cmd, args: w.map(str::to_string).collect(), done: false })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut pilot: ResMut<Autopilot>,
    mut loaded: MessageReader<LevelLoaded>,
    mut players: Query<(Entity, &mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut MoveInput, &mut CharacterVelocity, &PortalGun), With<Player>>,
    mut fire: MessageWriter<FirePortal>,
    (mut exit, mut click, real, mut con): (MessageWriter<AppExit>, MessageWriter<crate::ui::MenuClick>, Res<Time<Real>>, MessageWriter<crate::console::ConsoleSubmit>),
    mut ents: Option<ResMut<crate::io::Ents>>,
    spatial: avian3d::prelude::SpatialQuery,
    props: Query<(&crate::props::Prop, &Transform, Option<&avian3d::prelude::Position>, Has<crate::props::Held>), Without<Player>>,
    prop_ents: Query<(Entity, &crate::props::Prop)>,
    children: Query<&Children>,
    collider_of: Query<(Option<&avian3d::prelude::ColliderOf>, Has<avian3d::prelude::Collider>, Has<avian3d::prelude::ColliderDisabled>)>,
    bodies: Query<&avian3d::prelude::RigidBody>,
    mut settings: ResMut<crate::settings::Settings>,
    (names, globals, src_cols): (Query<(Option<&Name>, Option<&ChildOf>)>, Query<&GlobalTransform>, Query<&source_assets::SourceCollider>),
) {
    // The script clock is real time: menus pause the world's clock.
    // It starts with the first level (the menu's too, for menu tests) and
    // restarts on every playable one.
    for e in loaded.read() {
        if !e.menu || pilot.start.is_none() {
            pilot.start = Some(real.elapsed_secs());
        }
    }
    let Some(start) = pilot.start else { return };
    let t = real.elapsed_secs() - start;
    let Ok((player, mut tf, mut feet, mut view, mut input, mut vel, gun)) = players.single_mut() else {
        // No player (the main menu): only menu clicks, screenshots and quit.
        for step in pilot.steps.iter_mut().filter(|s| !s.done && s.at <= t) {
            let a = &step.args;
            match step.cmd.as_str() {
                "click" => {
                    click.write(crate::ui::MenuClick(a.join("")));
                }
                "shot" => {
                    if let Some(path) = a.first() {
                        commands
                            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                            .observe(bevy::render::view::screenshot::save_to_disk(path.clone()));
                    }
                }
                "quit" => {
                    exit.write(AppExit::Success);
                }
                _ => continue,
            }
            info!("autopilot t={t:.2}: {} {:?} (menu)", step.cmd, a);
            step.done = true;
        }
        return;
    };
    let f = |a: &[String], i: usize| a.get(i).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
    let now = time.elapsed_secs();
    let mut new_holds = Vec::new();
    for step in pilot.steps.iter_mut().filter(|s| !s.done && s.at <= t) {
        step.done = true;
        let a = &step.args;
        info!("autopilot t={t:.2}: {} {:?}", step.cmd, a);
        match step.cmd.as_str() {
            "setpos" => {
                let p = shared::source_to_world(Vec3::new(f(a, 0), f(a, 1), f(a, 2)));
                // What the player holds comes along (it would be dropped as stuck).
                let moved = p - tf.translation;
                for (pe, _) in &prop_ents {
                    if let Ok((_, ht, _, true)) = props.get(pe) {
                        let to = ht.translation + moved;
                        commands.entity(pe).insert((Transform::from_translation(to).with_rotation(ht.rotation), avian3d::prelude::Position(to)));
                    }
                }
                tf.translation = p;
                *feet = InterpolatedFeet { prev: p, curr: p };
                vel.0 = Vec3::ZERO;
                if a.len() >= 5 {
                    *view = ViewAngles { yaw: (f(a, 4) - 90.0).to_radians(), pitch: -f(a, 3).to_radians(), roll: 0.0 };
                }
            }
            "input" => {
                // input <target> <input> [param]: send an input like the map does.
                if let (Some(ents), Some(t), Some(i)) = (ents.as_mut(), a.first(), a.get(1)) {
                    ents.send(t, i, a.get(2).map(String::as_str).unwrap_or(""), 0.0);
                }
            }
            "lookat" if a.len() >= 3 && a.iter().take(3).all(|v| v.parse::<f32>().is_ok()) => {
                // lookat x y z: aim at a Source point (portal surfaces).
                let target = shared::source_to_world(Vec3::new(f(a, 0), f(a, 1), f(a, 2)));
                let d = target - (tf.translation + Vec3::Y * VIEW_STAND);
                *view = ViewAngles { yaw: (-d.x).atan2(-d.z), pitch: d.y.atan2(Vec2::new(d.x, d.z).length()), roll: 0.0 };
            }
            "goto" | "lookat" => {
                // A map entity by name or class: where it is now (props move).
                let Some(ents) = ents.as_deref() else { continue };
                let key = a.first().map(|k| k.to_ascii_lowercase()).unwrap_or_default();
                let Some(i) = ents.list.iter().position(|e| e.name == key || e.class == key) else {
                    warn!("autopilot: no entity {key}");
                    continue;
                };
                // Where it is drawn now (props move, cores ride rails).
                let target = props
                    .iter()
                    .find(|(p, _, _, _)| p.ent == i)
                    .map(|(_, t, _, _)| t.translation)
                    .or_else(|| ents.list[i].entity.and_then(|r| globals.get(r).ok()).map(|g| g.translation()))
                    .unwrap_or(ents.list[i].transform.translation);
                if step.cmd == "goto" {
                    // Stand `dist` units away on the side with the most room.
                    let dist = a.get(1).and_then(|d| d.parse::<f32>().ok()).unwrap_or(64.0) * shared::SOURCE_UNIT;
                    let filter = avian3d::prelude::SpatialQueryFilter::from_excluded_entities([player]);
                    let from = target + Vec3::Y * 0.6;
                    let mut best = (0.0f32, Vec3::X);
                    for k in 0..16 {
                        let ang = k as f32 / 16.0 * std::f32::consts::TAU;
                        let d = Vec3::new(ang.cos(), 0.0, ang.sin());
                        let free = spatial.cast_ray(from, Dir3::new(d).unwrap(), 10.0, true, &filter).map_or(10.0, |h| h.distance);
                        if free > best.0 {
                            best = (free, d);
                        }
                    }
                    let mut p = target + best.1 * dist.min(best.0 - 0.4);
                    if let Some(h) = spatial.cast_ray(p + Vec3::Y * 0.6, Dir3::NEG_Y, 4.0, true, &filter) {
                        p.y = p.y + 0.6 - h.distance;
                    }
                    tf.translation = p;
                    *feet = InterpolatedFeet { prev: p, curr: p };
                    vel.0 = Vec3::ZERO;
                }
                let eye = tf.translation + Vec3::Y * VIEW_STAND;
                let d = target - eye;
                *view = ViewAngles { yaw: (-d.x).atan2(-d.z), pitch: d.y.atan2(Vec2::new(d.x, d.z).length()), roll: 0.0 };
            }
            "look" => *view = ViewAngles { yaw: (f(a, 1) - 90.0).to_radians(), pitch: -f(a, 0).to_radians(), roll: 0.0 },
            "turn" => view.yaw += f(a, 0).to_radians(),
            "walk" => new_holds.push((now + f(a, 0), Vec2::new(0.0, 1.0), false)),
            "back" => new_holds.push((now + f(a, 0), Vec2::new(0.0, -1.0), false)),
            "left" => new_holds.push((now + f(a, 0), Vec2::new(-1.0, 0.0), false)),
            "right" => new_holds.push((now + f(a, 0), Vec2::new(1.0, 0.0), false)),
            "duck" => new_holds.push((now + f(a, 0), Vec2::ZERO, true)),
            "jump" => input.jump = true,
            "use" => input.use_key = true,
            "fire" => {
                let kind = if a.first().is_some_and(|k| k.starts_with('o')) { PortalKind::Orange } else { PortalKind::Blue };
                fire.write(FirePortal { gun: player, kind });
            }
            "propto" => {
                // propto <name> x y z: move a physics prop there (tests).
                let key = a.first().map(|k| k.to_ascii_lowercase()).unwrap_or_default();
                let p = shared::source_to_world(Vec3::new(f(a, 1), f(a, 2), f(a, 3)));
                if let Some(ents) = ents.as_deref() {
                    for (pe, prop) in &prop_ents {
                        if ents.list[prop.ent].name == key {
                            commands.entity(pe).insert((Transform::from_translation(p), avian3d::prelude::Position(p), avian3d::prelude::LinearVelocity::ZERO));
                        }
                    }
                }
            }
            "info" | "ent" => {
                // info <name>: an entity's state, bounds (Source units),
                // position and axes, and a prop's physics body (where
                // triggers see it).
                let key = a.first().map(|k| k.to_ascii_lowercase()).unwrap_or_default();
                if let Some(ents) = ents.as_deref() {
                    for (i, e) in ents.list.iter().enumerate().filter(|(_, e)| e.name == key) {
                        let b = e.bounds.map(|(lo, hi)| (shared::world_to_source(lo), shared::world_to_source(hi)));
                        let r = e.transform.rotation;
                        let axis = |v: Vec3| Vec3::new(v.x, -v.z, v.y);
                        info!(
                            "autopilot: #{i} {} {:?} enabled {} killed {} value {} at source {:.0?} fwd {:.2?} up {:.2?} bounds {:.0?}",
                            e.class,
                            e.name,
                            e.enabled,
                            e.killed,
                            e.value,
                            shared::world_to_source(e.transform.translation),
                            axis(r * Vec3::X),
                            axis(r * Vec3::Y),
                            b
                        );
                    }
                    for (p, _, pos, _) in &props {
                        if ents.list[p.ent].name == key {
                            info!("autopilot: {key} body at source {:.0?} half {:.0?}", pos.map(|v| shared::world_to_source(v.0)), p.half / shared::SOURCE_UNIT);
                        }
                    }
                }
            }
            "ray" => {
                // ray x y z [dx dy dz]: what is straight below a Source
                // point (or along a Source direction).
                let from = shared::source_to_world(Vec3::new(f(a, 0), f(a, 1), f(a, 2)));
                let dir = if a.len() >= 6 { Dir3::new(shared::source_to_world(Vec3::new(f(a, 3), f(a, 4), f(a, 5)))).unwrap_or(Dir3::NEG_Y) } else { Dir3::NEG_Y };
                let filter = avian3d::prelude::SpatialQueryFilter::from_excluded_entities([player]);
                if let Some(h) = spatial.cast_ray(from, dir, 100.0, true, &filter) {
                    let mut chain = Vec::new();
                    let mut e = Some(h.entity);
                    while let Some(c) = e {
                        let (n, p) = names.get(c).unwrap_or((None, None));
                        chain.push(format!("{c}:{}", n.map(|n| n.as_str()).unwrap_or("-")));
                        e = p.map(|p| p.parent());
                    }
                    let at = from + *dir * h.distance;
                    // The face that was hit: material and whether it takes portals.
                    if let Some(side) = src_cols.get(h.entity).ok().and_then(|c| c.side_for_normal(h.normal)) {
                        info!("autopilot: ray face {} portalable {} surfaceprop {:?}", side.material, side.portalable, side.surface_prop);
                    }
                    info!(
                        "autopilot: ray hit at source z {:.0} ({:.0} {:.0} {:.0}): {}",
                        at.y / shared::SOURCE_UNIT,
                        at.x / shared::SOURCE_UNIT,
                        -at.z / shared::SOURCE_UNIT,
                        at.y / shared::SOURCE_UNIT,
                        chain.join(" < ")
                    );
                } else {
                    info!("autopilot: ray hit nothing");
                }
            }
            "plan" => {
                let at = shared::source_to_world(Vec3::new(f(a, 0), f(a, 1), f(a, 2)));
                let half = f(a, 3).max(64.0) * shared::SOURCE_UNIT;
                commands.spawn((
                    Name::new("Autopilot plan camera"),
                    PlanCamera { frames: 0, path: a.get(4).cloned().unwrap_or_else(|| "plan.png".into()) },
                    Camera3d::default(),
                    Camera { order: 10_000, clear_color: ClearColorConfig::Custom(Color::srgb(0.5, 0.0, 0.5)), ..default() },
                    bevy::core_pipeline::tonemapping::Tonemapping::None,
                    bevy::camera::Hdr,
                    Projection::Orthographic(OrthographicProjection {
                        near: 0.0,
                        far: 200.0,
                        scaling_mode: bevy::camera::ScalingMode::FixedVertical { viewport_height: 2.0 * half },
                        ..OrthographicProjection::default_3d()
                    }),
                    bevy::camera::visibility::RenderLayers::layer(0),
                    // Looking down with Source north (+Y, our -Z) at the top.
                    Transform::from_translation(at).looking_to(Dir3::NEG_Y, Dir3::NEG_Z),
                ));
            }
            "shot" => {
                if let Some(path) = a.first() {
                    commands
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(path.clone()));
                }
            }
            "log" => {
                let s = tf.translation / shared::SOURCE_UNIT;
                info!(
                    "autopilot: feet source ({:.0} {:.0} {:.0}) yaw {:.0} portals {:?}",
                    s.x,
                    -s.z,
                    s.y,
                    view.yaw.to_degrees() + 90.0,
                    gun.portals.iter().map(Option::is_some).collect::<Vec<_>>()
                );
                for (pe, _) in &prop_ents {
                    for c in children.iter_descendants(pe) {
                        if let Ok((of, has, dis)) = collider_of.get(c) {
                            if has {
                                info!("autopilot: prop {pe} collider {c} body {:?} (self {}) body kind {:?} disabled {dis}", of.map(|o| o.body), of.is_some_and(|o| o.body == pe), of.and_then(|o| bodies.get(o.body).ok()));
                            }
                        }
                    }
                    info!("autopilot: prop {pe} kind {:?}", bodies.get(pe).ok());
                }
                for (p, t, _, _) in &props {
                    let s = t.translation / shared::SOURCE_UNIT;
                    let name = ents.as_ref().map(|e| e.list[p.ent].name.clone()).unwrap_or_default();
                    info!("autopilot: prop {name} at source ({:.0} {:.0} {:.0}) half {:.2?}", s.x, -s.z, s.y, p.half);
                }
            }
            "set" => match (a.first().map(String::as_str), a.get(1).and_then(|v| v.parse::<f32>().ok())) {
                (Some("fov"), Some(v)) => settings.fov = v,
                (Some("sensitivity"), Some(v)) => settings.mouse_sensitivity = v,
                (Some("volume"), Some(v)) => settings.master_volume = v,
                (Some("showfps"), Some(v)) => settings.tools.show_fps = v != 0.0,
                (Some("showpos"), Some(v)) => settings.tools.show_pos = v != 0.0,
                _ => warn!("autopilot: set what?"),
            },
            "click" => {
                click.write(crate::ui::MenuClick(a.join("")));
            }
            "quit" => {
                exit.write(AppExit::Success);
            }
            "con" => {
                con.write(crate::console::ConsoleSubmit(a.join(" ")));
            }
            _ => warn!("autopilot: unknown command {}", step.cmd),
        }
    }
    pilot.hold.extend(new_holds);
    pilot.hold.retain(|h| h.0 > now);
    let wish = pilot.hold.iter().fold(Vec2::ZERO, |acc, h| acc + h.1);
    input.wish = wish.clamp_length_max(1.0);
    input.duck = pilot.hold.iter().any(|h| h.2);
}

/// A temporary top-down camera for `plan`: shoots the window once it has
/// rendered a few frames, then goes away.
#[derive(Component)]
struct PlanCamera {
    frames: u32,
    path: String,
}

fn plan_shots(
    mut commands: Commands,
    mut plans: Query<(Entity, &mut PlanCamera)>,
    mut others: Query<(Entity, &mut Camera, Option<&bevy::camera::RenderTarget>), (With<Camera3d>, Without<PlanCamera>)>,
    mut paused: Local<Vec<Entity>>,
) {
    let mut busy = false;
    for (e, mut cam) in &mut plans {
        cam.frames += 1;
        busy = true;
        if cam.frames == 6 {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(cam.path.clone()));
        }
        if cam.frames == 14 {
            commands.entity(e).despawn();
            busy = false;
        }
    }
    // The other window cameras draw over it: pause them meanwhile.
    for (e, mut c, target) in &mut others {
        if busy && c.is_active && !matches!(target, Some(bevy::camera::RenderTarget::Image(_))) {
            c.is_active = false;
            paused.push(e);
        } else if !busy && paused.contains(&e) {
            c.is_active = true;
        }
    }
    if !busy {
        paused.clear();
    }
}
