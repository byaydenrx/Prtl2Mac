//! Floor turrets (`npc_portal_turret_floor`). They are physics props: the
//! player can carry them or knock them over. A standing turret that sees the
//! player in front of it (in range, nothing in between) deploys and fires;
//! when it loses sight it searches, then retracts. Tipped over, it fires
//! `OnTipped` and shuts down. A laser burns it out. Its red sight line
//! points where it is aiming.
use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;

use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::lasers::LaserState;
use crate::level::LevelLoaded;
use crate::props::{Held, Prop};

const U: f32 = shared::SOURCE_UNIT;
/// Height of the turret's eye above its origin.
const EYE: f32 = 38.0 * U;
/// Half-angle of what it sees, either side of its forward.
const VIEW_COS: f32 = 0.64; // ~50 degrees
/// Seconds from spotting the player to the first shot.
const DEPLOY_TIME: f32 = 0.6;
/// Seconds it keeps looking after losing the player.
const SEARCH_TIME: f32 = 3.0;
/// Damage per second while it has the player.
const DPS: f32 = 55.0;
/// Seconds a tipped turret takes to shut down.
const TIPPED_TIME: f32 = 2.5;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Idle,
    Deploying(f32),
    Firing,
    Searching(f32),
    Tipped(f32),
    Dead,
}

#[derive(Clone, Copy, Debug)]
struct Turret {
    mode: Mode,
    next_shot_sound: f32,
    next_talk: f32,
    held: bool,
}

impl Default for Turret {
    fn default() -> Self {
        Turret { mode: Mode::Idle, next_shot_sound: 0.0, next_talk: 0.0, held: false }
    }
}

#[derive(Default)]
pub struct Turrets(HashMap<usize, Turret>);

#[allow(clippy::too_many_arguments)]
pub fn run_turrets(
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut lasers: ResMut<LaserState>,
    mut turrets: Local<Turrets>,
    mut inputs: MessageReader<EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
    props: Query<(Entity, &Prop, &Position, &Rotation, Has<Held>)>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    spatial: SpatialQuery,
    players: Query<(Entity, &Transform, &Crouch), With<Player>>,
    mut fire: MessageWriter<FireOutput>,
    mut play: MessageWriter<PlayGameSound>,
    mut damage: MessageWriter<crate::health::DamagePlayer>,
) {
    lasers.sights.clear();
    if loaded.read().count() > 0 {
        turrets.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let now = ents.now;
    let dt = time.delta_secs();
    let say = |play: &mut MessageWriter<PlayGameSound>, sound: &str, at: Vec3| {
        play.write(PlayGameSound { sound: sound.into(), at: Some(at), looped: false, owner: None, music: false, volume: 1.0, voice: None, ui: false });
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class != "npc_portal_turret_floor" {
            continue;
        }
        let t = turrets.0.entry(msg.target).or_default();
        match msg.input.as_str() {
            "selfdestruct" | "selfdestructimmediately" | "kill" | "dissolve" | "silentdissolve" => t.mode = Mode::Dead,
            "disable" => t.mode = Mode::Dead,
            "enable" if t.mode == Mode::Dead => t.mode = Mode::Idle,
            _ => {}
        }
    }
    let Ok((player, ptf, crouch)) = players.single() else { return };
    let target = ptf.translation + Vec3::Y * crouch.hull().y * 0.6;
    let burned: Vec<usize> = lasers.hit.clone();
    for (body, prop, pos, rot, held) in &props {
        let i = prop.ent;
        let Some(e) = ents.list.get(i) else { continue };
        if e.class != "npc_portal_turret_floor" || e.killed {
            continue;
        }
        let t = turrets.0.entry(i).or_default();
        let up = rot.0 * Vec3::Y;
        let eye = pos.0 + up * EYE;
        if held && !t.held {
            say(&mut play, "NPC_FloorTurret.TalkPickup", eye);
        }
        t.held = held;
        if burned.contains(&i) && t.mode != Mode::Dead {
            t.mode = Mode::Dead;
            say(&mut play, "NPC_FloorTurret.TalkBurned", eye);
            fire.write(FireOutput { ent: i, event: "OnDeath".into(), activator: Some(PLAYER) });
            continue;
        }
        if matches!(t.mode, Mode::Dead) {
            continue;
        }
        // Knocked over.
        if up.y < 0.5 && !held && !matches!(t.mode, Mode::Tipped(_)) {
            t.mode = Mode::Tipped(now + TIPPED_TIME);
            say(&mut play, "NPC_FloorTurret.TalkTipped", eye);
            fire.write(FireOutput { ent: i, event: "OnTipped".into(), activator: Some(PLAYER) });
        }
        if let Mode::Tipped(until) = t.mode {
            if now >= until {
                t.mode = Mode::Dead;
                say(&mut play, "NPC_FloorTurret.TalkRetire", eye);
                fire.write(FireOutput { ent: i, event: "OnDeath".into(), activator: Some(PLAYER) });
            }
            continue;
        }
        // Can it see the player?
        let fwd = (rot.0 * Vec3::X).normalize();
        let to = target - eye;
        let dist = to.length();
        let range = e.get_f32("turretrange").unwrap_or(1024.0) * U;
        let mut sees = !held && dist < range && dist > 1e-3 && fwd.dot(to / dist) > VIEW_COS;
        if sees {
            let mut skip: Vec<Entity> = vec![player, body];
            skip.extend(children.iter_descendants(body).filter(|c| colliders.contains(*c)));
            let filter = SpatialQueryFilter::from_excluded_entities(skip);
            sees = spatial.cast_ray(eye, Dir3::new(to).unwrap_or(Dir3::X), dist, true, &filter).is_none();
        }
        t.mode = match (t.mode, sees) {
            (Mode::Idle, true) | (Mode::Searching(_), true) => {
                say(&mut play, "NPC_FloorTurret.Deploy", eye);
                if now >= t.next_talk {
                    say(&mut play, "NPC_FloorTurret.TalkDeploy", eye);
                    t.next_talk = now + 4.0;
                }
                Mode::Deploying(now + DEPLOY_TIME)
            }
            (Mode::Deploying(at), true) if now >= at => Mode::Firing,
            (Mode::Deploying(_), false) | (Mode::Firing, false) => {
                if now >= t.next_talk {
                    say(&mut play, "NPC_FloorTurret.TalkSearch", eye);
                    t.next_talk = now + 4.0;
                }
                Mode::Searching(now + SEARCH_TIME)
            }
            (Mode::Searching(until), false) if now >= until => {
                say(&mut play, "NPC_FloorTurret.Retract", eye);
                if now >= t.next_talk {
                    say(&mut play, "NPC_FloorTurret.TalkAutosearch", eye);
                    t.next_talk = now + 6.0;
                }
                Mode::Idle
            }
            (m, _) => m,
        };
        // Sight line: at the player while it has them, else straight ahead.
        let aim = if matches!(t.mode, Mode::Firing | Mode::Deploying(_)) { to.normalize_or(fwd) } else { fwd };
        let filter = SpatialQueryFilter::from_excluded_entities(children.iter_descendants(body).filter(|c| colliders.contains(*c)).chain([body]));
        let reach = spatial.cast_ray(eye, Dir3::new(aim).unwrap_or(Dir3::X), range, true, &filter).map_or(range, |h| h.distance);
        lasers.sights.push((eye + aim * (6.0 * U), eye + aim * reach));
        if t.mode == Mode::Firing {
            damage.write(crate::health::DamagePlayer(DPS * dt));
            if now >= t.next_shot_sound {
                say(&mut play, "NPC_FloorTurret.ShotSounds", eye);
                t.next_shot_sound = now + 0.1;
            }
        }
    }
}
