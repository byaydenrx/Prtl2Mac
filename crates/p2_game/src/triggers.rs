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
            .init_resource::<ScreenFade>()
            .add_message::<FizzlePortals>()
            .add_message::<LevelExit>()
            .add_systems(Update, (touch_triggers, gun_pickup, player_inputs, level_exit, fizzle_portals, draw_fade).after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

/// Which triggers each toucher is in (`PLAYER` or a prop's ent index).
#[derive(Resource, Default)]
struct Touching(std::collections::HashSet<(usize, usize)>);

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
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut CharacterVelocity, &mut ViewAngles, &Crouch), With<Player>>,
    props: Query<(Entity, &Prop, &Position, Has<Held>)>,
    mut fire: MessageWriter<FireOutput>,
    mut fizzle: MessageWriter<FizzlePortals>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        touching.0.clear();
    }
    let (Some(mut ents), Some(map)) = (ents, map) else { return };
    if map.menu {
        return;
    }
    let Ok((mut ptf, mut feet, mut vel, mut view, crouch)) = players.single_mut() else { return };
    let hull = crouch.hull() * 0.5;
    let p_lo = ptf.translation - Vec3::new(hull.x, 0.0, hull.z);
    let p_hi = ptf.translation + Vec3::new(hull.x, hull.y * 2.0, hull.z);
    if std::env::var_os("P2_TRACE").is_some() && ents.now < 0.05 {
        info!("TRACE player box {p_lo:.2?}..{p_hi:.2?}");
    }
    let mut now_touching = std::collections::HashSet::new();
    let mut teleport_to: Option<usize> = None;
    let mut dissolve: Vec<Entity> = Vec::new();
    for i in 0..ents.list.len() {
        let e = &ents.list[i];
        if e.killed || !e.enabled || !TRIGGER_CLASSES.contains(&e.class.as_str()) {
            continue;
        }
        let Some((lo, hi)) = e.bounds else { continue };
        let flags = e.spawnflags;
        let class = e.class.clone();
        // spawnflags: 1 clients, 8 physics objects (cleansers catch everything).
        let wants_player = flags & 1 != 0 || class == "trigger_portal_cleanser" || flags == 0;
        let wants_props = flags & 8 != 0 || class == "trigger_portal_cleanser";
        let mut touchers: Vec<usize> = Vec::new();
        if wants_player && overlaps(lo, hi, p_lo, p_hi) {
            touchers.push(PLAYER);
        }
        if wants_props {
            for (pe, prop, pos, _) in &props {
                if overlaps(lo, hi, pos.0 - prop.half, pos.0 + prop.half) {
                    touchers.push(prop.ent);
                    if class == "trigger_portal_cleanser" {
                        dissolve.push(pe);
                    }
                }
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
            fire.write(FireOutput { ent: i, event: "OnTrigger".into(), activator: Some(t) });
            match class.as_str() {
                // Once only: stop listening (its queued outputs still fire).
                "trigger_once" => ents.list[i].enabled = false,
                "trigger_teleport" | "trigger_playerteleport" if t == PLAYER => {
                    teleport_to = ents.list[i].get("target").and_then(|tg| ents.find(tg, i, Some(t)).first().copied());
                }
                "trigger_portal_cleanser" if t == PLAYER => {
                    fizzle.write(FizzlePortals);
                    fire.write(FireOutput { ent: i, event: "OnFizzle".into(), activator: Some(t) });
                }
                _ => {}
            }
        }
    }
    for (i, t) in touching.0.difference(&now_touching) {
        fire.write(FireOutput { ent: *i, event: "OnEndTouch".into(), activator: Some(*t) });
    }
    touching.0 = now_touching;
    for d in dissolve {
        commands.entity(d).despawn();
    }
    if let Some(dest) = teleport_to {
        let t = ents.list[dest].transform;
        ptf.translation = t.translation;
        *feet = InterpolatedFeet { prev: t.translation, curr: t.translation };
        view.yaw = entity_yaw(t.rotation);
        vel.0 = Vec3::ZERO;
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
) {
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
                    color: Vec4::new(c[0], c[1], c[2], alpha),
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
            ("point_clientcommand" | "point_servercommand", "command") => {
                let c = msg.param.trim().to_ascii_lowercase();
                if c.starts_with("changelevel") || c.starts_with("map ") {
                    exit.write(LevelExit);
                } else if c.starts_with("fadeout") || c.starts_with("fadein") {
                    // handled by env_fade in practice
                }
            }
            (_, "runscriptcode") => {
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
        if msg.input == "runscriptcode" && msg.param.to_ascii_lowercase().contains("upgrade") {
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
                    *pending = Some((time.elapsed_secs() + 3.5, next.to_string()));
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
