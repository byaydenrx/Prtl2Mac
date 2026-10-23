//! Floor turrets (`npc_portal_turret_floor`). They are physics props: the
//! player can carry them or knock them over (walking into one tips it). A
//! standing turret that sees the player in front of it (in range, nothing in
//! between) deploys and fires; when it loses sight it searches, then
//! retracts. Tipped over, it fires `OnTipped`, sprays bullets wherever it
//! points for a while, then shuts down. A laser burns it out. Its red sight
//! line points where it is aiming. It talks as Portal 2's do: deploy,
//! engaged, searching, "are you still there?", retiring, picked up, knocked,
//! flung, tipped, shut down, burning, burned out, fizzled.
//!
//! The model plays the sequence of each state (`deploy`, `idlealert`,
//! `fire`, `retract`, `eye_off`), and every shot draws a muzzle flash at one
//! of its four guns, a `weapon_tracers` tracer to where the bullet lands and
//! an impact effect there ([`turret_fx`]).
use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;
use p2_fx::ParticleEffect;
use source_assets::plugin::{SHOT_MASK, SIGHT_MASK};

use crate::anim_props::Playback;
use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents, FireOutput, PLAYER};
use crate::lasers::LaserState;
use crate::level::{LevelLoaded, LevelOwned};
use crate::props::{Held, Prop};
use crate::viewcontrol::PropAnims;

const U: f32 = shared::SOURCE_UNIT;
/// Height of the turret's eye above its origin.
const EYE: f32 = 38.0 * U;
/// Half-angle of what it sees, either side of its forward.
const VIEW_COS: f32 = 0.64; // ~50 degrees
/// Seconds from spotting the player to the first shot (the `deploy`
/// sequence is 0.67 s).
const DEPLOY_TIME: f32 = 0.6;
/// Seconds it keeps looking after losing the player.
const SEARCH_TIME: f32 = 3.0;
/// Damage per second while it has the player.
const DPS: f32 = 55.0;
/// Seconds a tipped turret sprays bullets before it shuts down.
const TIPPED_TIME: f32 = 2.5;
/// Seconds a laser takes to burn a turret out.
const BURN_TIME: f32 = 1.0;
/// A change of velocity this big (m/s) in one frame is a knock: "ow".
const COLLIDE_SPEED: f32 = 150.0 * U;
/// An impulse this big (N s) from a moving body in one step is a hit.
const HIT_IMPULSE: f32 = 25.0;
/// Flying faster than this (m/s), not carried, it was thrown or launched.
const FLUNG_SPEED: f32 = 350.0 * U;
/// Seconds between shots (one gun each, in turn).
const SHOT_INTERVAL: f32 = 0.1;
/// A tipped turret's shots stray this far (radians) from where it points.
const TIPPED_SPREAD: f32 = 0.12;
/// An aimed shot strays this far.
const AIM_SPREAD: f32 = 0.025;
/// The muzzle attachments, in firing order.
const GUNS: [&str; 4] = ["LFT_Gun1_muzzle", "RT_Gun1_muzzle", "LFT_Gun2_muzzle", "RT_Gun2_muzzle"];
/// Walking into a standing turret faster than this (m/s) tips it over.
const BUMP_SPEED: f32 = 90.0 * U;
/// How hard a bump turns it over (rad/s).
const BUMP_SPIN: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Idle,
    Deploying(f32),
    Firing,
    Searching(f32),
    Tipped(f32),
    /// A laser has it: it flails, then burns out at the time.
    Burning(f32),
    Dead,
}

/// What the model shows: one sequence (or a pair) per group of modes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anim {
    Closed,
    Open,
    Firing,
    Off,
}

impl Mode {
    fn anim(self) -> Anim {
        match self {
            Mode::Idle => Anim::Closed,
            Mode::Deploying(_) | Mode::Searching(_) => Anim::Open,
            Mode::Firing | Mode::Tipped(_) | Mode::Burning(_) => Anim::Firing,
            Mode::Dead => Anim::Off,
        }
    }
}

/// The sequence to play on entering `to` from `from` (none: keep what is
/// playing) and the idle that follows it.
fn anim_change(from: Option<Anim>, to: Anim) -> (Option<&'static str>, &'static str) {
    match to {
        Anim::Closed => (matches!(from, Some(Anim::Open | Anim::Firing)).then_some("retract"), "idle"),
        Anim::Open => (Some(if from == Some(Anim::Firing) { "idlealert" } else { "deploy" }), "idlealert"),
        Anim::Firing => (Some("fire"), "fire"),
        Anim::Off => (Some("eye_off"), "idle_eye_off"),
    }
}

#[derive(Clone, Copy, Debug)]
struct Turret {
    mode: Mode,
    next_shot: f32,
    next_talk: f32,
    held: bool,
    /// What the model was last told to show.
    anim: Option<Anim>,
    /// The gun that fires next.
    gun: usize,
    /// Random state for spread.
    seed: u32,
    /// Its velocity last frame, for knocks.
    last_vel: Vec3,
    /// No "ow" again before this.
    next_collide: f32,
    /// Said its flung line this flight.
    flung: bool,
    /// When a search asks "are you still there?" (`None`: asked).
    ask_at: Option<f32>,
    /// The mode last written by P2_TRACE.
    traced: Mode,
}

impl Default for Turret {
    fn default() -> Self {
        Turret {
            mode: Mode::Idle,
            next_shot: 0.0,
            next_talk: 0.0,
            held: false,
            anim: None,
            gun: 0,
            seed: 0x9E37_79B9,
            last_vel: Vec3::ZERO,
            next_collide: 0.0,
            flung: false,
            ask_at: None,
            traced: Mode::Idle,
        }
    }
}

impl Turret {
    /// A number in -1..1.
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// `dir` turned off course by up to `spread` radians.
    fn stray(&mut self, dir: Vec3, spread: f32) -> Vec3 {
        let side = dir.any_orthonormal_vector();
        let up = dir.cross(side);
        (dir + side * self.rand() * spread + up * self.rand() * spread).normalize_or(dir)
    }
}

#[derive(Default)]
pub struct Turrets(HashMap<usize, Turret>);

/// One bullet for [`turret_fx`] to draw.
#[derive(Clone, Debug)]
struct Shot {
    /// The turret's entity index.
    ent: usize,
    /// Which of [`GUNS`] fired.
    gun: usize,
    /// Its eye, if the muzzle can't be found.
    eye: Vec3,
    dir: Vec3,
    /// The turret's own body and colliders (bullets start inside it).
    skip: Vec<Entity>,
}

/// What [`run_turrets`] decided this frame, for [`turret_fx`] to show.
#[derive(Resource, Default)]
pub struct TurretFx {
    /// Bullets fired.
    shots: Vec<Shot>,
    /// Sequence changes: (turret, sequence to play now, idle after it).
    anims: Vec<(usize, Option<&'static str>, &'static str)>,
    /// Turrets walked into: (body, spin that tips them).
    bumps: Vec<(Entity, Vec3)>,
}

#[allow(clippy::too_many_arguments)]
pub fn run_turrets(
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut lasers: ResMut<LaserState>,
    mut turrets: Local<Turrets>,
    mut inputs: MessageReader<EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
    props: Query<(Entity, &Prop, &Position, &Rotation, &LinearVelocity, Has<Held>)>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    spatial: SpatialQuery,
    players: Query<(Entity, &Transform, &Crouch, &CharacterVelocity), With<Player>>,
    (mut fire, mut play, mut damage): (MessageWriter<FireOutput>, MessageWriter<PlayGameSound>, MessageWriter<crate::health::DamagePlayer>),
    mut fx: ResMut<TurretFx>,
    collisions: Collisions,
    dynamic: Query<&RigidBody>,
) {
    lasers.sights.clear();
    if loaded.read().count() > 0 {
        turrets.0.clear();
        *fx = TurretFx::default();
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
        let e = &ents.list[msg.target];
        if e.class != "npc_portal_turret_floor" {
            continue;
        }
        let at = e.transform.translation + Vec3::Y * EYE;
        let t = turrets.0.entry(msg.target).or_default();
        match msg.input.as_str() {
            "dissolve" if t.mode != Mode::Dead => {
                say(&mut play, "NPC_FloorTurret.TalkDissolved", at);
                t.mode = Mode::Dead;
            }
            "selfdestruct" | "selfdestructimmediately" | "kill" | "silentdissolve" | "dissolve" => t.mode = Mode::Dead,
            "disable" => t.mode = Mode::Dead,
            "enable" if t.mode == Mode::Dead => t.mode = Mode::Idle,
            _ => {}
        }
    }
    let Ok((player, ptf, crouch, pvel)) = players.single() else { return };
    let target = ptf.translation + Vec3::Y * crouch.hull().y * 0.6;
    let burned: Vec<usize> = lasers.hit.clone();
    for (body, prop, pos, rot, vel, held) in &props {
        let i = prop.ent;
        let Some(e) = ents.list.get(i) else { continue };
        if e.class != "npc_portal_turret_floor" || e.killed {
            continue;
        }
        let t = turrets.0.entry(i).or_default();
        if t.seed == Turret::default().seed {
            t.seed ^= (i as u32).wrapping_mul(2_654_435_761) | 1;
        }
        let up = rot.0 * Vec3::Y;
        let eye = pos.0 + up * EYE;
        let skip = || -> Vec<Entity> { children.iter_descendants(body).filter(|c| colliders.contains(*c)).chain([body]).collect() };
        if held && !t.held {
            say(&mut play, "NPC_FloorTurret.TalkPickup", eye);
        }
        t.held = held;
        // Knocked by something, or landing hard: "ow" (also when it is hit
        // hard without moving, e.g. a cube dropped on its head). Thrown or
        // launched: its flung line, once per flight.
        if t.mode != Mode::Dead && !held {
            let struck = skip().into_iter().any(|c| {
                collisions.collisions_with(c).any(|pair| {
                    let other = if pair.collider1 == c { pair.body2 } else { pair.body1 };
                    let moving = other.and_then(|b| dynamic.get(b).ok()).is_some_and(|b| b.is_dynamic());
                    moving && pair.manifolds.iter().flat_map(|m| m.points.iter()).map(|p| p.normal_impulse).sum::<f32>() > HIT_IMPULSE
                })
            });
            if ((vel.0 - t.last_vel).length() > COLLIDE_SPEED || struck) && now >= t.next_collide {
                say(&mut play, "NPC_FloorTurret.TalkCollide", eye);
                t.next_collide = now + 1.5;
            }
            if vel.0.length() > FLUNG_SPEED && !t.flung {
                say(&mut play, "NPC_FloorTurret.TalkFlung", eye);
                t.flung = true;
            }
        }
        if vel.0.length() < 100.0 * U {
            t.flung = false;
        }
        t.last_vel = vel.0;
        // Walked into while standing: over it goes, away from the player.
        let flat = Vec3::new(pos.0.x - ptf.translation.x, 0.0, pos.0.z - ptf.translation.z);
        let push = Vec3::new(pvel.0.x, 0.0, pvel.0.z);
        let level = ptf.translation.y < eye.y && ptf.translation.y + crouch.hull().y > pos.0.y;
        if !held && up.y > 0.9 && level && flat.length() < 34.0 * U && push.dot(flat.normalize_or_zero()) > BUMP_SPEED && !matches!(t.mode, Mode::Tipped(_) | Mode::Dead) {
            fx.bumps.push((body, up.cross(flat.normalize_or_zero()) * BUMP_SPIN));
        }
        if burned.contains(&i) && !matches!(t.mode, Mode::Burning(_) | Mode::Dead) {
            t.mode = Mode::Burning(now + BURN_TIME);
            say(&mut play, "NPC_FloorTurret.TalkStartBurning", eye);
        }
        if let Mode::Burning(until) = t.mode {
            if now >= until {
                t.mode = Mode::Dead;
                say(&mut play, "NPC_FloorTurret.TalkBurned", eye);
                say(&mut play, "NPC_FloorTurret.Die", eye);
                fire.write(FireOutput { ent: i, event: "OnDeath".into(), activator: Some(PLAYER) });
            }
        }
        if !matches!(t.mode, Mode::Dead | Mode::Burning(_)) {
            // Knocked over.
            if up.y < 0.5 && !held && !matches!(t.mode, Mode::Tipped(_)) {
                t.mode = Mode::Tipped(now + TIPPED_TIME);
                say(&mut play, "NPC_FloorTurret.TalkTipped", eye);
                fire.write(FireOutput { ent: i, event: "OnTipped".into(), activator: Some(PLAYER) });
            }
            if let Mode::Tipped(until) = t.mode {
                if now >= until {
                    t.mode = Mode::Dead;
                    say(&mut play, "NPC_FloorTurret.TalkDisabled", eye);
                    say(&mut play, "NPC_FloorTurret.Die", eye);
                    fire.write(FireOutput { ent: i, event: "OnDeath".into(), activator: Some(PLAYER) });
                }
            }
        }
        let fwd = (rot.0 * Vec3::X).normalize();
        let to = target - eye;
        let dist = to.length();
        let range = e.get_f32("turretrange").unwrap_or(1024.0) * U;
        // Nothing of `mask` between the turret and the player: it sees
        // through glass and grates (SIGHT_MASK); its bullets pass grates but
        // stop at glass (SHOT_MASK).
        let clear_to_player = |sp: &SpatialQuery, mask: u32| {
            let mut s = skip();
            s.push(player);
            let filter = SpatialQueryFilter::from_excluded_entities(s).with_mask(LayerMask(mask));
            sp.cast_ray(eye, Dir3::new(to).unwrap_or(Dir3::X), dist, true, &filter).is_none()
        };
        if !matches!(t.mode, Mode::Tipped(_) | Mode::Burning(_) | Mode::Dead) {
            // Can it see the player?
            let sees = !held && dist < range && dist > 1e-3 && fwd.dot(to / dist) > VIEW_COS && clear_to_player(&spatial, SIGHT_MASK);
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
                // Engaged: now and then "gotcha" / "I see you".
                (Mode::Firing, true) => {
                    if now >= t.next_talk {
                        say(&mut play, "NPC_FloorTurret.TalkActive", eye);
                        t.next_talk = now + 5.0;
                    }
                    Mode::Firing
                }
                (Mode::Deploying(_), false) | (Mode::Firing, false) => {
                    if now >= t.next_talk {
                        say(&mut play, "NPC_FloorTurret.TalkSearch", eye);
                        t.next_talk = now + 1.5;
                    }
                    t.ask_at = Some(now + SEARCH_TIME * 0.5);
                    Mode::Searching(now + SEARCH_TIME)
                }
                (Mode::Searching(until), false) if now >= until => {
                    say(&mut play, "NPC_FloorTurret.Retract", eye);
                    if now >= t.next_talk {
                        say(&mut play, "NPC_FloorTurret.TalkRetire", eye);
                        t.next_talk = now + 4.0;
                    }
                    Mode::Idle
                }
                (Mode::Searching(until), false) => {
                    if t.ask_at.is_some_and(|a| now >= a) && now >= t.next_talk {
                        say(&mut play, "NPC_FloorTurret.TalkAutosearch", eye);
                        t.next_talk = now + 4.0;
                        t.ask_at = None;
                    }
                    Mode::Searching(until)
                }
                (m, _) => m,
            };
        }
        if std::env::var_os("P2_TRACE").is_some() && std::mem::discriminant(&t.mode) != std::mem::discriminant(&t.traced) {
            info!("TRACE turret #{i} {:?} -> {:?} at {:.2}", t.traced, t.mode, now);
            t.traced = t.mode;
        }
        // The model shows the state.
        let anim = t.mode.anim();
        if t.anim != Some(anim) {
            let (seq, idle) = anim_change(t.anim, anim);
            fx.anims.push((i, seq, idle));
            t.anim = Some(anim);
        }
        if t.mode == Mode::Dead {
            continue;
        }
        // Sight line: at the player while it has them, else straight ahead.
        let aim = if matches!(t.mode, Mode::Firing | Mode::Deploying(_)) { to.normalize_or(fwd) } else { fwd };
        if !matches!(t.mode, Mode::Tipped(_) | Mode::Burning(_)) {
            let filter = SpatialQueryFilter::from_excluded_entities(skip()).with_mask(LayerMask(SHOT_MASK));
            let reach = spatial.cast_ray(eye, Dir3::new(aim).unwrap_or(Dir3::X), range, true, &filter).map_or(range, |h| h.distance);
            lasers.sights.push((eye + aim * (6.0 * U), eye + aim * reach));
        }
        let firing = t.mode == Mode::Firing;
        let tipped = matches!(t.mode, Mode::Tipped(_));
        // Behind glass it fires, but the glass stops the bullets.
        if firing && clear_to_player(&spatial, SHOT_MASK) {
            damage.write(crate::health::DamagePlayer(DPS * dt));
        }
        // Tipped, it hits the player only if they're where it points.
        if tipped && dist < range && fwd.dot(to / dist.max(1e-3)) > 0.97 && clear_to_player(&spatial, SHOT_MASK) {
            damage.write(crate::health::DamagePlayer(DPS * dt));
        }
        if (firing || tipped) && now >= t.next_shot {
            say(&mut play, "NPC_FloorTurret.ShotSounds", eye);
            t.next_shot = now + SHOT_INTERVAL;
            let dir = if firing { t.stray(aim, AIM_SPREAD) } else { t.stray(fwd, TIPPED_SPREAD) };
            fx.shots.push(Shot { ent: i, gun: t.gun, eye, dir, skip: skip() });
            t.gun = (t.gun + 1) % GUNS.len();
        }
    }
}

/// A turret's muzzle flash: a sprite facing the camera (as it was when
/// fired) that lasts a moment.
#[derive(Component)]
pub struct MuzzleFlash {
    until: f32,
}

/// Seconds a muzzle flash shows.
const FLASH_TIME: f32 = 0.05;
/// Size of a muzzle flash (m).
const FLASH_SIZE: f32 = 14.0 * U;

/// The muzzle flash quad and its two looks (`effects/combinemuzzle1` and
/// `2`, additive), made once by [`load_flash_assets`] so [`turret_fx`] needs
/// no write access to the asset stores (that held up the frame's other
/// systems: ~20 fps lost).
#[derive(Resource)]
pub struct FlashAssets {
    quad: Handle<Mesh>,
    looks: Vec<Handle<StandardMaterial>>,
}

pub fn load_flash_assets(
    mut commands: Commands,
    game: Res<p2_assets::GameFsRes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut looks = Vec::new();
    for n in ["materials/effects/combinemuzzle1.vtf", "materials/effects/combinemuzzle2.vtf"] {
        let texture = game.read(n).and_then(|d| source_assets::vtf::VtfTexture::parse(&d).ok()).map(|t| images.add(t.into_image(false, true)));
        if texture.is_none() {
            warn!("portal2: no muzzle flash texture {n}");
        }
        looks.push(materials.add(StandardMaterial {
            base_color: if texture.is_some() { Color::linear_rgb(2.0, 2.0, 2.0) } else { Color::srgba(1.0, 0.8, 0.4, 0.8) },
            base_color_texture: texture,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            fog_enabled: false,
            double_sided: true,
            cull_mode: None,
            ..default()
        }));
    }
    commands.insert_resource(FlashAssets { quad: meshes.add(Rectangle::new(FLASH_SIZE, FLASH_SIZE)), looks });
}

/// Shows what [`run_turrets`] decided: the turrets' sequences, the spin of
/// a turret walked into, and each bullet: a muzzle flash at the gun, a
/// tracer to where it stops and the surface's impact effect there.
#[allow(clippy::too_many_arguments)]
pub fn turret_fx(
    mut commands: Commands,
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut fx: ResMut<TurretFx>,
    mut anims: ResMut<PropAnims>,
    game: Option<Res<p2_assets::GameFsRes>>,
    spatial: SpatialQuery,
    players: Query<Entity, With<Player>>,
    surfaces: Query<&source_assets::plugin::SourceCollider>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    flashes: Query<(Entity, &MuzzleFlash)>,
    assets: Option<Res<FlashAssets>>,
) {
    let now = time.elapsed_secs();
    let cam = cams.iter().next().map(|c| c.translation());
    for (e, f) in &flashes {
        if now >= f.until {
            commands.entity(e).despawn();
        }
    }
    for (i, seq, idle) in fx.anims.drain(..) {
        anims.defaults.insert(i, idle.to_string());
        if let Some(seq) = seq {
            anims.playing.insert(i, Playback::new(seq, false));
        }
    }
    for (body, spin) in fx.bumps.drain(..) {
        commands.entity(body).insert(AngularVelocity(spin));
    }
    let shots = std::mem::take(&mut fx.shots);
    let (Some(ents), Some(game), Some(assets)) = (ents, game, assets) else { return };
    let player = players.single().ok();
    let src_basis = crate::map_particles::source_basis(Quat::IDENTITY);
    for (k, shot) in shots.into_iter().enumerate() {
        let muzzle = anims.attachment_transform(&game, &ents, shot.ent, GUNS[shot.gun]).map(|t| t.translation).unwrap_or(shot.eye);
        let filter = SpatialQueryFilter::from_excluded_entities(shot.skip.iter().copied()).with_mask(LayerMask(SHOT_MASK));
        let range = 4096.0 * U;
        let hit = spatial.cast_ray(muzzle, Dir3::new(shot.dir).unwrap_or(Dir3::X), range, true, &filter);
        let end = muzzle + shot.dir * hit.map_or(range, |h| h.distance);
        let mut tracer = ParticleEffect::new("weapon_tracers");
        tracer.despawn_when_done = true;
        tracer.set_cp(0, muzzle, src_basis);
        tracer.set_cp(1, end, src_basis);
        commands.spawn((Name::new("turret tracer"), tracer, LevelOwned));
        // Every other bullet leaves an impact (the cheap variants): a full
        // effect per bullet cost frames with several turrets firing.
        if let Some(h) = hit.filter(|h| Some(h.entity) != player && shot.gun % 2 == 0) {
            let surface = surfaces.get(h.entity).ok().and_then(|c| c.side_for_normal(h.normal)).and_then(|s| s.surface_prop.clone()).unwrap_or_default();
            let mut impact = ParticleEffect::new(impact_system(&surface));
            impact.despawn_when_done = true;
            // The effect's forward (+X) is the surface normal.
            impact.set_cp(0, end, crate::map_particles::source_basis(Quat::from_rotation_arc(Vec3::X, h.normal.normalize_or(Vec3::Y))));
            commands.spawn((Name::new("turret bullet impact"), impact, LevelOwned));
        }
        let look = assets.looks[k % assets.looks.len()].clone();
        let mut tf = Transform::from_translation(muzzle + shot.dir * (4.0 * U));
        if let Some(cam) = cam {
            tf.look_at(cam, Vec3::Y);
        }
        commands.spawn((MuzzleFlash { until: now + FLASH_TIME }, Mesh3d(assets.quad.clone()), MeshMaterial3d(look), tf, LevelOwned));
    }
}

/// The `impact_fx.pcf` effect for a bullet hitting a surfaceprop.
fn impact_system(surface: &str) -> &'static str {
    let s = surface.to_ascii_lowercase();
    if s.contains("metal") || s.contains("computer") {
        "impact_metal_cheap"
    } else if s.contains("glass") {
        "impact_glass_cheap"
    } else if s.contains("wood") {
        "impact_wood_cheap"
    } else if s.contains("plastic") {
        "impact_plastic_cheap"
    } else {
        "impact_concrete_cheap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turret_sequences_follow_its_state() {
        assert_eq!(anim_change(None, Anim::Closed), (None, "idle"));
        assert_eq!(anim_change(Some(Anim::Closed), Anim::Open), (Some("deploy"), "idlealert"));
        assert_eq!(anim_change(Some(Anim::Open), Anim::Firing), (Some("fire"), "fire"));
        assert_eq!(anim_change(Some(Anim::Firing), Anim::Open), (Some("idlealert"), "idlealert"));
        assert_eq!(anim_change(Some(Anim::Open), Anim::Closed), (Some("retract"), "idle"));
        assert_eq!(anim_change(Some(Anim::Firing), Anim::Off), (Some("eye_off"), "idle_eye_off"));
        assert_eq!(Mode::Tipped(1.0).anim(), Anim::Firing);
        assert_eq!(Mode::Burning(1.0).anim(), Anim::Firing);
        assert_eq!(Mode::Searching(1.0).anim(), Anim::Open);
    }

    #[test]
    fn bullet_impacts_match_the_surface() {
        assert_eq!(impact_system("metal"), "impact_metal_cheap");
        assert_eq!(impact_system("Glass"), "impact_glass_cheap");
        assert_eq!(impact_system("default"), "impact_concrete_cheap");
        assert_eq!(impact_system(""), "impact_concrete_cheap");
    }

    #[test]
    fn shots_stray_only_a_little() {
        let mut t = Turret::default();
        for _ in 0..100 {
            let d = t.stray(Vec3::X, AIM_SPREAD);
            assert!((d.length() - 1.0).abs() < 1e-4 && d.x > 0.99, "{d:?}");
        }
    }
}

