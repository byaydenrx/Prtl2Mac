//! Scripted input for testing levels without a person at the keyboard:
//! `P2_SCRIPT="1.0 setpos -704 96 0 0 90; 2 walk 1.5; 4 use; 5 fire blue"`.
//!
//! Each step is `<seconds after the level starts> <command> [args]`:
//! `setpos x y z [pitch yaw]` (Source units/degrees, feet like the game's
//! `setpos`), `look pitch yaw`, `walk secs` (forward), `back secs`,
//! `left secs`, `right secs`, `jump`, `duck secs`, `use`, `fire blue|orange`,
//! `shot path.png`, `log`, `con <console line>` (runs a developer-console
//! command, e.g. `con sv_cheats 1` then `con noclip`; `;` can't be used inside).
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
                .add_systems(Update, drive.after(P2Systems::Input).before(P2Systems::Fire));
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
    props: Query<(&crate::props::Prop, &Transform), Without<Player>>,
    prop_ents: Query<(Entity, &crate::props::Prop)>,
    children: Query<&Children>,
    collider_of: Query<(Option<&avian3d::prelude::ColliderOf>, Has<avian3d::prelude::Collider>, Has<avian3d::prelude::ColliderDisabled>)>,
    bodies: Query<&avian3d::prelude::RigidBody>,
    mut settings: ResMut<crate::settings::Settings>,
    names: Query<(Option<&Name>, Option<&ChildOf>)>,
) {
    // The script clock is real time: menus pause the world's clock.
    if loaded.read().any(|e| !e.menu) {
        pilot.start = Some(real.elapsed_secs());
    }
    let Some(start) = pilot.start else { return };
    let t = real.elapsed_secs() - start;
    let Ok((player, mut tf, mut feet, mut view, mut input, mut vel, gun)) = players.single_mut() else { return };
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
            "goto" | "lookat" => {
                // A map entity by name or class: where it is now (props move).
                let Some(ents) = ents.as_deref() else { continue };
                let key = a.first().map(|k| k.to_ascii_lowercase()).unwrap_or_default();
                let Some(i) = ents.list.iter().position(|e| e.name == key || e.class == key) else {
                    warn!("autopilot: no entity {key}");
                    continue;
                };
                let target = props.iter().find(|(p, _)| p.ent == i).map(|(_, t)| t.translation).unwrap_or(ents.list[i].transform.translation);
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
            "ray" => {
                // ray x y z: what is straight below a Source point.
                let from = shared::source_to_world(Vec3::new(f(a, 0), f(a, 1), f(a, 2)));
                let filter = avian3d::prelude::SpatialQueryFilter::from_excluded_entities([player]);
                if let Some(h) = spatial.cast_ray(from, Dir3::NEG_Y, 100.0, true, &filter) {
                    let mut chain = Vec::new();
                    let mut e = Some(h.entity);
                    while let Some(c) = e {
                        let (n, p) = names.get(c).unwrap_or((None, None));
                        chain.push(format!("{c}:{}", n.map(|n| n.as_str()).unwrap_or("-")));
                        e = p.map(|p| p.parent());
                    }
                    info!("autopilot: ray hit at source z {:.0}: {}", (from.y - h.distance) / shared::SOURCE_UNIT, chain.join(" < "));
                } else {
                    info!("autopilot: ray hit nothing");
                }
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
                for (p, t) in &props {
                    let s = t.translation / shared::SOURCE_UNIT;
                    let name = ents.as_ref().map(|e| e.list[p.ent].name.clone()).unwrap_or_default();
                    info!("autopilot: prop {name} at source ({:.0} {:.0} {:.0}) half {:.2?}", s.x, -s.z, s.y, p.half);
                }
            }
            "set" => match (a.first().map(String::as_str), a.get(1).and_then(|v| v.parse::<f32>().ok())) {
                (Some("fov"), Some(v)) => settings.fov = v,
                (Some("sensitivity"), Some(v)) => settings.mouse_sensitivity = v,
                (Some("volume"), Some(v)) => settings.master_volume = v,
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
