//! Trigger volumes and the inputs that act on the player: touching
//! triggers, teleports, the fizzler (emancipation grid), picking up the gun,
//! hazards, screen fades and leaving the level.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::campaign::Campaign;
use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::level::{CurrentMap, LevelLoaded, LoadLevel};
use crate::props::{Held, Prop};

pub struct TriggersPlugin;

impl Plugin for TriggersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Touching>()
            .init_resource::<LookTimes>()
            .init_resource::<ScreenFade>()
            .add_message::<FizzlePortals>()
            .add_message::<LevelExit>()
            .add_systems(Update, (touch_triggers, gun_pickup, player_inputs, level_exit, fizzle_portals, fizzle_on_moving_surfaces, draw_fade).after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

/// Which triggers each toucher is in (`PLAYER` or a prop's ent index).
#[derive(Resource, Default)]
struct Touching(std::collections::HashSet<(usize, usize)>);

/// `trigger_look`: seconds the player has looked at the target, and has
/// been inside, without a break.
#[derive(Resource, Default)]
struct LookTimes(std::collections::HashMap<usize, (f32, f32)>);

/// Close the player's portals (fizzler, `trigger_portal_cleanser`, scripts).
#[derive(Message, Clone, Copy, Debug)]
pub struct FizzlePortals;

/// Screen colour fade: colour, start, length, hold, fading in or out.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct ScreenFade {
    /// Stay faded out until another fade (Source FFADE_STAYOUT).
    pub stay: bool,
    pub color: Vec4,
    pub start: f32,
    pub duration: f32,
    pub hold: f32,
    /// true: from clear to the colour (fade out); false: from the colour to clear.
    pub out: bool,
    pub active: bool,
}

const TRIGGER_CLASSES: &[&str] = &[
    "trigger_once",
    "trigger_multiple",
    "trigger_teleport",
    "trigger_portal_cleanser",
    "trigger_hurt",
    "trigger_push",
    "trigger_look",
    "trigger_weapon_strip",
    "trigger_physics_trap",
    "trigger_catapult",
    "trigger_changelevel",
    "trigger_playerteleport",
];

fn overlaps(lo: Vec3, hi: Vec3, p_lo: Vec3, p_hi: Vec3) -> bool {
    lo.x <= p_hi.x && hi.x >= p_lo.x && lo.y <= p_hi.y && hi.y >= p_lo.y && lo.z <= p_hi.z && hi.z >= p_lo.z
}

#[allow(clippy::too_many_arguments)]
fn touch_triggers(
    mut commands: Commands,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    mut touching: ResMut<Touching>,
    mut looks: ResMut<LookTimes>,
    mut players: Query<(Entity, &mut Transform, &mut InterpolatedFeet, &mut CharacterVelocity, &mut ViewAngles, &Crouch, &mut PortalGun), With<Player>>,
    props: Query<(Entity, &Prop, &Position, Has<Held>, &LinearVelocity)>,
    mut fire: MessageWriter<FireOutput>,
    mut fizzle: MessageWriter<FizzlePortals>,
    mut cleansed: MessageWriter<GunCleansed>,
    mut loaded: MessageReader<LevelLoaded>,
    time: Res<Time>,
    mut damage: MessageWriter<crate::health::DamagePlayer>,
    shapes: Option<Res<crate::level::BrushShapes>>,
) {
    if loaded.read().count() > 0 {
        touching.0.clear();
        looks.0.clear();
    }
    let dt = time.delta_secs();
    let (Some(mut ents), Some(map)) = (ents, map) else { return };
    if map.menu {
        return;
    }
    let Ok((player, mut ptf, mut feet, mut vel, mut view, crouch, mut gun)) = players.single_mut() else { return };
    let hull = crouch.hull() * 0.5;
    let p_lo = ptf.translation - Vec3::new(hull.x, 0.0, hull.z);
    let p_hi = ptf.translation + Vec3::new(hull.x, hull.y * 2.0, hull.z);
    if std::env::var_os("P2_TRACE").is_some() && ents.now < 0.05 {
        info!("TRACE player box {p_lo:.2?}..{p_hi:.2?}");
    }
    let mut now_touching = std::collections::HashSet::new();
    let mut teleport_to: Option<Teleport> = None;
    let mut teleport_props: Vec<(Entity, Teleport)> = Vec::new();
    let mut dissolve: Vec<(Entity, usize)> = Vec::new();
    let mut launch_player: Option<(Vec3, f32)> = None;
    let mut push_player = Vec3::ZERO;
    for i in 0..ents.list.len() {
        let e = &ents.list[i];
        if e.killed || !e.enabled || !TRIGGER_CLASSES.contains(&e.class.as_str()) {
            continue;
        }
        let Some((lo, hi)) = e.bounds else { continue };
        let flags = e.spawnflags;
        let class = e.class.clone();
        // spawnflags: 1 clients, 8 physics objects, 64 everything (cleansers
        // catch everything). bts1's jailbreak is armed by a 64 trigger that
        // a cube in the dropper touches (T-239).
        let everything = flags & 64 != 0;
        let wants_player = flags & 1 != 0 || everything || class == "trigger_portal_cleanser" || class == "trigger_look" || flags == 0;
        let wants_props = flags & 8 != 0 || everything || class == "trigger_portal_cleanser";
        // spawnflags 2: NPCs (the personality cores, turrets).
        let wants_npcs = flags & 2 != 0 || everything;
        let filter = e.get("filtername").filter(|f| !f.is_empty()).map(str::to_string);
        // Past the box, the trigger's own brushes (Source space, relative
        // to its origin and unturned by its angles).
        let (origin, unturn) = (e.transform.translation, e.transform.rotation.inverse());
        let model = e.brush_model;
        let in_brushes = |a: Vec3, b: Vec3| -> bool {
            let (lo, hi) = crate::io::turned_box(a.min(b) - origin, a.max(b) - origin, unturn);
            let (sa, sb) = (shared::world_to_source(lo), shared::world_to_source(hi));
            match (model, shapes.as_deref()) {
                (Some(m), Some(s)) => s.box_touches(m, sa.min(sb), sa.max(sb)).unwrap_or(true),
                _ => true,
            }
        };
        let mut touchers: Vec<usize> = Vec::new();
        if wants_player && overlaps(lo, hi, p_lo, p_hi) && in_brushes(p_lo, p_hi) {
            touchers.push(PLAYER);
        }
        if wants_props || wants_npcs {
            for (pe, prop, pos, _, _) in &props {
                let Some(pent) = ents.list.get(prop.ent) else { continue };
                let is_npc = pent.class.starts_with("npc_");
                if !(if is_npc { wants_npcs || wants_props } else { wants_props }) {
                    continue;
                }
                // Personality cores plug into sockets whose triggers sit just
                // inside the wall (sp_a1_wakeup's breaker socket is ~10 units
                // behind breaker_train's face): in Portal 2 pushing the core
                // against the slot is enough, so cores reach a little further.
                let reach = if is_npc && pent.class == "npc_personality_core" { Vec3::splat(CORE_TRIGGER_REACH) } else { Vec3::ZERO };
                if overlaps(lo, hi, pos.0 - prop.half - reach, pos.0 + prop.half + reach) && in_brushes(pos.0 - prop.half - reach, pos.0 + prop.half + reach) {
                    touchers.push(prop.ent);
                    if class == "trigger_portal_cleanser" {
                        dissolve.push((pe, prop.ent));
                    }
                }
            }
        }
        // The trigger's filter (by name, class, model) decides who counts.
        if let Some(f) = &filter {
            touchers.retain(|t| ents.filter_passes(f, i, *t));
        }
        // While touching: hazards hurt, pushers push.
        for &t in &touchers {
            let e = &ents.list[i];
            match class.as_str() {
                // `damage` is per second.
                "trigger_hurt" if t == PLAYER => {
                    damage.write(crate::health::DamagePlayer(e.get_f32("damage").unwrap_or(10.0) * dt));
                }
                "trigger_push" => {
                    let a: Vec<f32> = e.get("pushdir").unwrap_or("").split_whitespace().filter_map(|v| v.parse().ok()).collect();
                    let (p, y) = (a.first().copied().unwrap_or(0.0).to_radians(), a.get(1).copied().unwrap_or(0.0).to_radians());
                    let src = Vec3::new(p.cos() * y.cos(), p.cos() * y.sin(), -p.sin());
                    let dir = e.transform.rotation * Vec3::new(src.x, src.z, -src.y);
                    let speed = e.get_f32("speed").unwrap_or(40.0) * shared::SOURCE_UNIT;
                    if t == PLAYER {
                        push_player += dir * (speed - vel.0.dot(dir)).max(0.0);
                    } else if let Some((pe, _, _, false, lv)) = props.iter().find(|p| p.1.ent == t) {
                        commands.entity(pe).insert(LinearVelocity(lv.0 + dir * speed * dt));
                    }
                }
                _ => {}
            }
        }
        for t in touchers {
            now_touching.insert((i, t));
            if touching.0.contains(&(i, t)) {
                continue;
            }
            // Start touch.
            if std::env::var_os("P2_TRACE").is_some() {
                info!("TRACE touch {} {:?} by {t}", class, ents.list[i].name);
            }
            fire.write(FireOutput { ent: i, event: "OnStartTouch".into(), activator: Some(t) });
            // trigger_look fires OnTrigger from the look check below.
            if class != "trigger_look" {
                fire.write(FireOutput { ent: i, event: "OnTrigger".into(), activator: Some(t) });
            }
            match class.as_str() {
                // Once only: stop listening (its queued outputs still fire).
                "trigger_once" => ents.list[i].enabled = false,
                "trigger_teleport" | "trigger_playerteleport" => {
                    let e = &ents.list[i];
                    let found = |key: &str| e.get(key).filter(|v| !v.is_empty()).and_then(|v| ents.find(v, i, Some(t)).first().copied());
                    if let Some(dest) = found("target") {
                        let tp = Teleport { dest, landmark: found("landmark"), landmark_angles: e.get("uselandmarkangles").is_some_and(|v| v.trim() == "1") };
                        if t == PLAYER {
                            teleport_to = Some(tp);
                        } else if class == "trigger_teleport" && !ents.list[t].killed {
                            if let Some((pe, ..)) = props.iter().find(|p| p.1.ent == t && !p.3) {
                                teleport_props.push((pe, tp));
                            }
                        }
                    }
                }
                "trigger_catapult" => {
                    let e = &ents.list[i];
                    let (from, v_in, prop) = if t == PLAYER {
                        (ptf.translation, vel.0, None)
                    } else {
                        match props.iter().find(|p| p.1.ent == t && !p.3) {
                            Some((pe, _, pos, _, lv)) => (pos.0, lv.0, Some(pe)),
                            None => continue,
                        }
                    };
                    let speed = e.get_f32(if t == PLAYER { "playerspeed" } else { "physicsspeed" }).unwrap_or(450.0) * shared::SOURCE_UNIT;
                    let target = e.get("launchtarget").filter(|n| !n.is_empty()).and_then(|n| ents.find(n, i, Some(t)).first().copied());
                    let aim = CatapultAim {
                        player: t == PLAYER,
                        exact: e.get("useexactvelocity").is_some_and(|v| v.trim() == "1"),
                        choice: e.get("exactvelocitychoicetype").and_then(|v| v.trim().parse().ok()).unwrap_or(0),
                        incoming: v_in,
                    };
                    let launch = catapult_velocity(from, target.map(|x| ents.list[x].transform.translation), e.get("launchdirection"), speed, aim);
                    // Threshold check: only correct things already flying about
                    // this fast (chained plates); others pass through.
                    let check = e.get("usethresholdcheck").is_some_and(|v| v.trim() == "1");
                    let lower = e.get_f32("lowerthreshold").unwrap_or(0.15);
                    let upper = e.get_f32("upperthreshold").unwrap_or(0.30);
                    let ratio = v_in.length() / launch.length().max(1e-3);
                    if check && !(ratio >= 1.0 - lower && ratio <= 1.0 + upper) {
                        continue;
                    }
                    if std::env::var_os("P2_TRACE").is_some() {
                        info!("TRACE catapult {:?} launches {t} at {:.0?} u/s", e.name, launch / shared::SOURCE_UNIT);
                    }
                    match prop {
                        None => {
                            // Launch by target: no air control for a moment
                            // (AirCtrlSupressionTime; below zero is the default
                            // quarter second).
                            // A directional plate only with DirectionSuppressAirControl.
                            let set = e.get_f32("airctrlsupressiontime").unwrap_or(-1.0);
                            let by_dir = e.get("directionsuppressaircontrol").is_some_and(|v| v.trim() == "1");
                            let t = if target.is_none() && !by_dir {
                                0.0
                            } else if set > 0.0 {
                                set
                            } else {
                                0.25
                            };
                            launch_player = Some((launch, t));
                        }
                        Some(pe) => {
                            commands.entity(pe).insert((LinearVelocity(launch), AngularVelocity(Vec3::ZERO)));
                        }
                    }
                    fire.write(FireOutput { ent: i, event: "OnCatapulted".into(), activator: Some(t) });
                }
                // sp_a1_wakeup's claw takes the gun before the incinerator.
                "trigger_weapon_strip" if t == PLAYER => {
                    gun.can_fire = [false, false];
                    fizzle.write(FizzlePortals);
                }
                "trigger_portal_cleanser" if t == PLAYER => {
                    fizzle.write(FizzlePortals);
                    // The gun shakes and sparks going through the grid.
                    if gun.can_fire.contains(&true) {
                        cleansed.write(GunCleansed { gun: player });
                    }
                    fire.write(FireOutput { ent: i, event: "OnFizzle".into(), activator: Some(t) });
                }
                _ => {}
            }
        }
    }
    for (i, t) in touching.0.difference(&now_touching) {
        fire.write(FireOutput { ent: *i, event: "OnEndTouch".into(), activator: Some(*t) });
    }
    // First thing in / last thing out.
    let count = |set: &std::collections::HashSet<(usize, usize)>, i: usize| set.iter().filter(|x| x.0 == i).count();
    let triggers: std::collections::HashSet<usize> = touching.0.iter().chain(now_touching.iter()).map(|x| x.0).collect();
    for i in triggers {
        let (was, is) = (count(&touching.0, i), count(&now_touching, i));
        if was == 0 && is > 0 {
            fire.write(FireOutput { ent: i, event: "OnStartTouchAll".into(), activator: None });
        } else if was > 0 && is == 0 {
            fire.write(FireOutput { ent: i, event: "OnEndTouchAll".into(), activator: None });
        }
    }
    // trigger_look: while the player is inside, looking within `fieldofview`
    // (a cosine) of the target for `looktime` seconds fires OnTrigger once;
    // looking away resets the clock. `timeout` > 0 fires OnTimeout instead
    // after that long inside without looking. Leaving resets both.
    // TODO: spawnflag 128 (line of sight) isn't checked.
    let eye = ptf.translation + Vec3::Y * crouch.eye_height;
    let fwd = view.forward();
    looks.0.retain(|i, _| now_touching.contains(&(*i, PLAYER)));
    for &(i, t) in &now_touching {
        if t != PLAYER || ents.list[i].class != "trigger_look" || !ents.list[i].enabled {
            continue;
        }
        let e = &ents.list[i];
        let target = e.get("target").and_then(|n| ents.find(n, i, Some(PLAYER)).first().copied());
        let Some(target) = target else { continue };
        let tp = match ents.list[target].bounds {
            Some((lo, hi)) if target != PLAYER => (lo + hi) * 0.5,
            _ => ents.list[target].transform.translation,
        };
        let fov = e.get_f32("fieldofview").unwrap_or(0.9);
        let need = e.get_f32("looktime").unwrap_or(0.5);
        let timeout = e.get_f32("timeout").unwrap_or(0.0);
        let looking = (tp - eye).normalize_or_zero().dot(fwd) >= fov;
        let entry = looks.0.entry(i).or_insert((0.0, 0.0));
        entry.0 = if looking { entry.0 + dt } else { 0.0 };
        entry.1 += dt;
        let event = if looking && entry.0 >= need {
            "OnTrigger"
        } else if timeout > 0.0 && entry.1 >= timeout {
            "OnTimeout"
        } else {
            continue;
        };
        if std::env::var_os("P2_TRACE").is_some() {
            info!("TRACE trigger_look {:?} {event}", e.name);
        }
        fire.write(FireOutput { ent: i, event: event.into(), activator: Some(PLAYER) });
        ents.list[i].enabled = false;
    }
    touching.0 = now_touching;
    for (d, ent) in dissolve {
        commands.entity(d).despawn();
        crate::props::dissolved(&mut ents, ent);
    }
    vel.0 += push_player;
    if let Some((v, suppress)) = launch_player {
        vel.0 = v;
        commands.entity(player).insert(p2_core::AirControlSuppressed(suppress));
    }
    for (pe, tp) in teleport_props {
        if let Ok((_, _, pos, _, lv)) = props.get(pe) {
            let (to, turn) = tp.place(&ents, pos.0);
            commands.entity(pe).try_insert((Position(to), LinearVelocity(turn * lv.0)));
        }
    }
    if let Some(tp) = teleport_to {
        let (to, turn) = tp.place(&ents, ptf.translation);
        ptf.translation = to;
        *feet = InterpolatedFeet { prev: to, curr: to };
        if tp.landmark.is_some() {
            // Same place and motion relative to the landmark: bts1's power
            // cut swaps you into an identical dark chamber unnoticed (T-239).
            view.yaw += entity_yaw(turn) - entity_yaw(Quat::IDENTITY);
            vel.0 = turn * vel.0;
        } else {
            view.yaw = entity_yaw(ents.list[tp.dest].transform.rotation);
            vel.0 = Vec3::ZERO;
        }
    }
}

/// Where a trigger_teleport sends what touches it: its destination, or,
/// with a landmark, the same offset from the destination as it had from
/// the landmark (turned too with `uselandmarkangles`).
#[derive(Clone, Copy)]
struct Teleport {
    dest: usize,
    landmark: Option<usize>,
    landmark_angles: bool,
}

impl Teleport {
    /// The new position and the turn applied to velocity and view.
    fn place(&self, ents: &Ents, from: Vec3) -> (Vec3, Quat) {
        let dest = ents.list[self.dest].transform;
        let Some(l) = self.landmark else { return (dest.translation, Quat::IDENTITY) };
        let lt = ents.list[l].transform;
        let turn = if self.landmark_angles { dest.rotation * lt.rotation.inverse() } else { Quat::IDENTITY };
        (dest.translation + turn * (from - lt.translation), turn)
    }
}

/// How far past its box a personality core counts as touching an NPC
/// trigger. sp_a1_wakeup's breaker socket trigger sits behind the breaker
/// train's face and the room's player clip keeps the player ~100 units
/// away, so a held core stops ~38 units short of it; in Portal 2 holding him
/// up to the slot plugs him in.
const CORE_TRIGGER_REACH: f32 = 40.0 * shared::SOURCE_UNIT;

/// How a faith plate aims at its launch target (trigger_catapult keys).
#[derive(Clone, Copy, Debug, Default)]
pub struct CatapultAim {
    /// The launchee is the player: Source aims its feet 32 units under the
    /// target, and a straight-up launch goes at 1.5x the speed.
    pub player: bool,
    /// useExactVelocity: launch at exactly `speed`, solving for the angle.
    pub exact: bool,
    /// exactVelocityChoiceType: 0 the arc nearer the incoming velocity,
    /// 1 the high arc, 2 the low arc.
    pub choice: i32,
    /// The launchee's velocity on entry (picks the arc for choice 0).
    pub incoming: Vec3,
}

/// Faith plate launch velocity (m/s), as Portal 2's CTriggerCatapult works
/// it out (server.dll). With a target and no exact velocity the flight time
/// is the straight-line distance over `speed`, and the launch covers the
/// offset in that time plus the upward speed gravity takes back
/// (d / t + up * g t / 2). With exact velocity the speed is kept and the
/// angle solved; with no solution it falls back to the first. Without a
/// target, straight along `launchdirection` (Source pitch/yaw) at `speed`.
pub fn catapult_velocity(from: Vec3, target: Option<Vec3>, direction: Option<&str>, speed: f32, aim: CatapultAim) -> Vec3 {
    let g = 600.0 * shared::SOURCE_UNIT;
    let speed = speed.max(1e-3);
    if let Some(mut to) = target {
        if aim.player {
            to.y -= 32.0 * shared::SOURCE_UNIT;
        }
        let d = to - from;
        if aim.exact {
            if let Some(v) = exact_launch(d, speed, g, aim) {
                return v;
            }
        }
        let t = d.length() / speed;
        if t > 1e-6 {
            return d / t + Vec3::Y * (0.5 * g * t);
        }
        return Vec3::Y * speed;
    }
    let a: Vec<f32> = direction.unwrap_or("").split_whitespace().filter_map(|v| v.parse().ok()).collect();
    let (pitch, yaw) = (a.first().copied().unwrap_or(0.0).to_radians(), a.get(1).copied().unwrap_or(0.0).to_radians());
    // Source forward: (cos p cos y, cos p sin y, -sin p) -> world (x, z_up, -y).
    let src = Vec3::new(pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), -pitch.sin());
    let mut v = Vec3::new(src.x, src.z, -src.y) * speed;
    if aim.player && v.x.abs() <= 0.001 && v.z.abs() <= 0.001 {
        v.y = speed * 1.5;
    }
    v
}

/// The two launch angles that reach offset `d` at exactly `speed`; None when
/// the target is out of reach (Source: "Catapult can't hit target!").
fn exact_launch(d: Vec3, speed: f32, g: f32, aim: CatapultAim) -> Option<Vec3> {
    let flat = Vec2::new(d.x, d.z);
    let x = flat.length();
    if x <= 0.0 {
        return None;
    }
    let v2 = speed * speed;
    let disc = v2 * v2 - g * (g * x * x + 2.0 * d.y * v2);
    if disc <= 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let dir = flat / (x + 1e-7);
    let arc = |tan: f32| {
        let th = tan.atan();
        let h = dir * th.cos() * speed;
        Vec3::new(h.x, th.sin() * speed, h.y)
    };
    let (high, low) = (arc((v2 + root) / (g * x)), arc((v2 - root) / (g * x)));
    let inc = aim.incoming.normalize_or_zero();
    let use_low = aim.choice != 1 && (aim.choice == 2 || inc.dot(high) <= inc.dot(low));
    Some(if use_low { low } else { high })
}

/// Console commands maps and scripts run; several can be joined with ';'.
fn console_command(line: &str, gun: &mut PortalGun, exit: &mut MessageWriter<LevelExit>) {
    for c in line.split(';').map(|c| c.trim().to_ascii_lowercase()) {
        if c.starts_with("changelevel") || c.starts_with("map ") {
            exit.write(LevelExit);
        } else if c == "give weapon_portalgun" || c == "give_portalgun" {
            gun.can_fire[0] = true;
        } else if (c.starts_with("upgrade_portalgun") || c.starts_with("upgrade_potatogun")) && gun.can_fire.iter().any(|f| *f) {
            // Upgrades only the gun the player holds: sp_a3_01's transition
            // script upgrades before the player has found the gun again.
            gun.can_fire = [true, true];
        }
    }
}

/// View yaw (0 looks down -Z) facing a map entity's forward (`rotation * X`).
pub fn entity_yaw(rotation: Quat) -> f32 {
    let f = rotation * Vec3::X;
    (-f.x).atan2(-f.z)
}

/// Inputs on point entities that act on the player or the screen.
#[allow(clippy::too_many_arguments)]
fn player_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    time: Res<Time>,
    mut fade: ResMut<ScreenFade>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut CharacterVelocity, &mut ViewAngles, &mut PortalGun), With<Player>>,
    mut fizzle: MessageWriter<FizzlePortals>,
    mut exit: MessageWriter<LevelExit>,
    mut title: MessageWriter<crate::ui::ShowChapterTitle>,
    scripted: Res<crate::vscripts::ScriptedEnts>,
    mut console: MessageReader<crate::vscripts::ConsoleCommand>,
) {
    // Console commands from scripts (`SendToConsole`, `GivePlayerPortalgun`).
    for c in console.read() {
        if let Ok((.., mut gun)) = players.single_mut() {
            console_command(&c.0, &mut gun, &mut exit);
        }
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        match (e.class.as_str(), msg.input.as_str()) {
            ("point_teleport", "teleport") => {
                let targets = e.get("target").map(|t| ents.find(t, msg.target, msg.activator)).unwrap_or_default();
                if targets.contains(&PLAYER) {
                    if let Ok((mut tf, mut feet, mut vel, mut view, _)) = players.single_mut() {
                        tf.translation = e.transform.translation;
                        *feet = InterpolatedFeet { prev: tf.translation, curr: tf.translation };
                        view.yaw = entity_yaw(e.transform.rotation);
                        vel.0 = Vec3::ZERO;
                    }
                }
            }
            ("env_fade", "fade" | "fadereverse") => {
                let c = source_assets::kv::parse_color(e.get("rendercolor").unwrap_or("0 0 0")).unwrap_or([0.0; 3]);
                let alpha = e.get_f32("renderamt").unwrap_or(255.0) / 255.0;
                // spawnflags 1 = fade from (in), else fade to (out).
                let fade_in = e.spawnflags & 1 != 0;
                *fade = ScreenFade {
                    // rendercolor is 0..255.
                    color: Vec4::new(c[0], c[1], c[2], 0.0).map(|v| if v > 1.0 { v / 255.0 } else { v }).with_w(alpha),
                    start: time.elapsed_secs(),
                    duration: e.get_f32("duration").unwrap_or(1.0),
                    hold: e.get_f32("holdtime").unwrap_or(0.0),
                    out: if msg.input == "fadereverse" { fade_in } else { !fade_in },
                    active: true,
                    stay: e.spawnflags & 8 != 0,
                };
            }
            ("weapon_portalgun", _) | ("player", "givegun") => {}
            ("point_changelevel" | "trigger_changelevel", "changelevel") => {
                exit.write(LevelExit);
            }
            ("point_clientcommand" | "point_servercommand" | "point_broadcastclientcommand", "command") => {
                if let Ok((.., mut gun)) = players.single_mut() {
                    console_command(&msg.param, &mut gun, &mut exit);
                }
            }
            // Entities without a loaded script: stand-ins for the common calls.
            (_, "runscriptcode") if !scripted.0.contains(&msg.target) => {
                let code = msg.param.to_ascii_lowercase();
                if code.contains("transitionfrommap") {
                    exit.write(LevelExit);
                }
                if code.contains("startmoving") && e.name.contains("departure") {
                    // The departure elevator leaves: fade and move on.
                    *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: time.elapsed_secs() + 2.0, duration: 1.5, hold: 10.0, out: true, active: true, stay: true };
                    exit.write(LevelExit);
                }
                if code.contains("fizzle") {
                    fizzle.write(FizzlePortals);
                }
                if code.contains("displaychaptertitle") {
                    title.write(crate::ui::ShowChapterTitle);
                }
            }
            ("player" | "!player", "disabledraw" | "enabledraw") => {}
            _ => {}
        }
        // Gun pickups and upgrades handed out by maps and scripts.
        if e.class == "weapon_portalgun" && (msg.input == "use" || msg.input == "trigger") {
            if let Ok((.., mut gun)) = players.single_mut() {
                gun.can_fire[0] |= e.get("canfireportal1") != Some("0");
                gun.can_fire[1] |= e.get("canfireportal2") == Some("1");
            }
        }
        if msg.input == "runscriptcode" && !scripted.0.contains(&msg.target) && msg.param.to_ascii_lowercase().contains("upgrade") {
            if let Ok((.., mut gun)) = players.single_mut() {
                gun.can_fire = [true, true];
            }
        }
    }
}

/// The player walked into a `weapon_portalgun` lying in the level: take it.
pub fn gun_pickup(
    mut commands: Commands,
    ents: Option<ResMut<Ents>>,
    mut players: Query<(&Transform, &mut PortalGun), With<Player>>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else { return };
    let Ok((tf, mut gun)) = players.single_mut() else { return };
    for i in 0..ents.list.len() {
        let e = &ents.list[i];
        if e.class != "weapon_portalgun" || e.killed {
            continue;
        }
        if e.transform.translation.distance(tf.translation + Vec3::Y * 0.6) < 48.0 * shared::SOURCE_UNIT {
            gun.can_fire[0] |= e.get("canfireportal1") != Some("0");
            gun.can_fire[1] |= e.get("canfireportal2") == Some("1");
            if let Some(r) = e.entity {
                commands.entity(r).despawn();
            }
            ents.list[i].killed = true;
            fire.write(FireOutput { ent: i, event: "OnPlayerPickup".into(), activator: Some(PLAYER) });
        }
    }
}

/// End of the level: the next map in the campaign, after the fade.
#[derive(Message, Clone, Copy, Debug)]
pub struct LevelExit;

fn level_exit(
    mut exits: MessageReader<LevelExit>,
    map: Option<Res<CurrentMap>>,
    campaign: Res<Campaign>,
    time: Res<Time>,
    mut pending: Local<Option<(f32, String)>>,
    mut load: MessageWriter<LoadLevel>,
    mut fade: ResMut<ScreenFade>,
) {
    if exits.read().count() > 0 && pending.is_none() {
        if let Some(map) = &map {
            match campaign.next_map(&map.info.name) {
                Some(next) => {
                    info!("portal2: leaving {} for {next}", map.info.name);
                    *pending = Some((exit_load_at(&fade, time.elapsed_secs()), next.to_string()));
                    if !fade.active {
                        *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: time.elapsed_secs() + 2.0, duration: 1.5, hold: 10.0, out: true, active: true, stay: true };
                    }
                }
                None => info!("portal2: {} is the last map", map.info.name),
            }
        }
    }
    if let Some((at, next)) = pending.clone() {
        if time.elapsed_secs() >= at {
            *pending = None;
            load.write(LoadLevel { map: next, menu: false });
            *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: time.elapsed_secs(), duration: 1.0, hold: 0.0, out: false, active: true, stay: false };
        }
    }
}

/// When to load the next map after a level exit at `now`. A map that has
/// already faded the screen out to opaque and holds it (sp_a1_wakeup's
/// `exit_fade`, 0.3 s) changes level as soon as that fade is black, as
/// Source does; otherwise our own 2 s wait and 1.5 s fade run first.
fn exit_load_at(fade: &ScreenFade, now: f32) -> f32 {
    if fade.active && fade.out && fade.stay && fade.color.w >= 0.99 {
        (fade.start + fade.duration).max(now) + 0.1
    } else {
        now + 3.5
    }
}

/// A portal on something that moves (a flipping panel, a door) fizzles.
fn fizzle_on_moving_surfaces(
    mut commands: Commands,
    mut players: Query<&mut PortalGun, With<Player>>,
    portals: Query<(&Portal, &Transform)>,
    moved: Query<Ref<GlobalTransform>>,
    mut closed: MessageWriter<PortalClosed>,
) {
    let Ok(mut gun) = players.single_mut() else { return };
    for slot in gun.portals.iter_mut() {
        let Some(p) = *slot else { continue };
        let Ok((portal, tf)) = portals.get(p) else { continue };
        if portal.surfaces.iter().any(|s| moved.get(*s).is_ok_and(|g| g.is_changed() && !g.is_added())) {
            closed.write(PortalClosed { kind: portal.kind, transform: *tf });
            commands.entity(p).despawn();
            *slot = None;
        }
    }
}

/// Close both of the player's portals with the fizzle effect.
fn fizzle_portals(mut commands: Commands, mut reader: MessageReader<FizzlePortals>, mut players: Query<&mut PortalGun, With<Player>>, portals: Query<(&Portal, &Transform)>, mut closed: MessageWriter<PortalClosed>) {
    if reader.read().count() == 0 {
        return;
    }
    let Ok(mut gun) = players.single_mut() else { return };
    for slot in gun.portals.iter_mut() {
        if let Some(p) = slot.take() {
            if let Ok((portal, tf)) = portals.get(p) {
                closed.write(PortalClosed { kind: portal.kind, transform: *tf });
            }
            commands.entity(p).despawn();
        }
    }
}

/// The full-screen fade.
#[derive(Component)]
struct FadeOverlay;

fn draw_fade(mut commands: Commands, time: Res<Time>, mut fade: ResMut<ScreenFade>, mut overlay: Query<(Entity, &mut BackgroundColor), With<FadeOverlay>>) {
    let t = time.elapsed_secs();
    let alpha = if !fade.active || t < fade.start {
        if fade.active && !fade.out { fade.color.w } else { 0.0 }
    } else {
        let k = ((t - fade.start) / fade.duration.max(1e-3)).clamp(0.0, 1.0);
        if fade.out {
            if !fade.stay && t > fade.start + fade.duration + fade.hold {
                fade.active = false;
            }
            k * fade.color.w
        } else {
            if t > fade.start + fade.duration + fade.hold {
                fade.active = false;
            }
            (1.0 - k) * fade.color.w
        }
    };
    let c = Color::srgba(fade.color.x, fade.color.y, fade.color.z, alpha);
    if std::env::var_os("P2_TRACE").is_some() && (t * 2.0).fract() < time.delta_secs() * 2.0 {
        info!("TRACE fade alpha {alpha:.2} {:?}", *fade);
    }
    match overlay.single_mut() {
        Ok((_, mut bg)) => bg.0 = c,
        Err(_) => {
            commands.spawn((
                FadeOverlay,
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(c),
                GlobalZIndex(50),
                bevy::picking::Pickable::IGNORE,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_exit_after_map_fade_loads_once_black() {
        let black = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: 10.0, duration: 0.3, hold: 0.0, out: true, active: true, stay: true };
        assert!((exit_load_at(&black, 10.0) - 10.4).abs() < 1e-4);
        assert!((exit_load_at(&black, 12.0) - 12.1).abs() < 1e-4);
        // No fade of the map's own (or one that does not stay black): our 3.5 s exit.
        assert_eq!(exit_load_at(&ScreenFade::default(), 5.0), 8.5);
        assert_eq!(exit_load_at(&ScreenFade { stay: false, ..black }, 10.0), 13.5);
        assert_eq!(exit_load_at(&ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 0.5), ..black }, 10.0), 13.5);
    }

    /// Fly a launch under Portal 2 gravity; the closest the path gets to `to`.
    fn closest(from: Vec3, v: Vec3, to: Vec3) -> f32 {
        let g = 600.0 * shared::SOURCE_UNIT;
        (0..4000).map(|i| i as f32 * 0.001).map(|t| (from + v * t - Vec3::Y * 0.5 * g * t * t - to).length()).fold(f32::MAX, f32::min)
    }

    #[test]
    fn catapult_flight_time_is_straight_line_distance_over_speed() {
        let u = shared::SOURCE_UNIT;
        let (from, to) = (Vec3::ZERO, Vec3::new(0.0, 256.0, -900.0) * u);
        let prop = CatapultAim::default();
        let v = catapult_velocity(from, Some(to), None, 500.0 * u, prop);
        let t = to.length() / (500.0 * u);
        assert!((v.z - to.z / t).abs() < 1e-4);
        assert!((v.y - (to.y / t + 300.0 * u * t)).abs() < 1e-4);
        assert!(closest(from, v, to) < 1.0 * u);
        // The player's feet go 32 units under the target.
        let p = catapult_velocity(from, Some(to), None, 500.0 * u, CatapultAim { player: true, ..prop });
        assert!(closest(from, p, to - Vec3::Y * 32.0 * u) < 1.0 * u);
    }

    #[test]
    fn catapult_exact_velocity_keeps_the_speed_and_picks_the_arc() {
        let u = shared::SOURCE_UNIT;
        let (from, to) = (Vec3::ZERO, Vec3::new(800.0, 0.0, 0.0) * u);
        let aim = |choice| CatapultAim { exact: true, choice, incoming: Vec3::new(1.0, -1.0, 0.0), ..default() };
        let (high, low) = (
            catapult_velocity(from, Some(to), None, 1000.0 * u, aim(1)),
            catapult_velocity(from, Some(to), None, 1000.0 * u, aim(2)),
        );
        for v in [high, low] {
            assert!((v.length() - 1000.0 * u).abs() < 1e-3);
            assert!(closest(from, v, to) < 1.0 * u);
        }
        assert!(high.y > low.y);
        // Choice 0: the arc more along the incoming (down and forward) velocity.
        assert_eq!(catapult_velocity(from, Some(to), None, 1000.0 * u, aim(0)), low);
        // Out of reach: falls back to the non-exact launch.
        let far = Vec3::new(9000.0, 0.0, 0.0) * u;
        assert_eq!(catapult_velocity(from, Some(far), None, 1000.0 * u, aim(1)), catapult_velocity(from, Some(far), None, 1000.0 * u, CatapultAim::default()));
    }

    #[test]
    fn straight_up_catapult_throws_the_player_half_as_fast_again() {
        let up = |player| catapult_velocity(Vec3::ZERO, None, Some("-90 0 0"), 450.0, CatapultAim { player, ..default() });
        assert!((up(true).y - 675.0).abs() < 1e-3);
        assert!((up(false).y - 450.0).abs() < 1e-3);
    }
}
