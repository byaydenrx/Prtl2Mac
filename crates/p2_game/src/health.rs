//! The player's health: hazards (goo, turrets, lasers, crushers) hurt,
//! health comes back after a short rest, and at zero the screen fades and
//! the level starts again from its last save (the map start; Portal 2's
//! mid-map autosaves are not kept yet). `player_loadsaved` does the same.
use bevy::prelude::*;

use crate::io::{EntInput, Ents};
use crate::level::{CurrentMap, LevelLoaded, LoadLevel};
use crate::triggers::ScreenFade;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHealth>()
            .add_message::<DamagePlayer>()
            .add_systems(Update, (take_damage, reload_inputs, draw_hurt).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

const MAX_HEALTH: f32 = 100.0;
/// Seconds without damage before health starts coming back.
const REGEN_DELAY: f32 = 3.0;
/// Health per second while regenerating.
const REGEN_RATE: f32 = 25.0;
/// Seconds from death (or a scripted reload) to the level starting again.
const RELOAD_AFTER: f32 = 2.0;

/// Damage to the player this frame (health points).
#[derive(Message, Clone, Copy, Debug)]
pub struct DamagePlayer(pub f32);

#[derive(Resource, Debug)]
pub struct PlayerHealth {
    pub hp: f32,
    /// Game time of the last hit.
    last_hit: f32,
    /// How red the screen is (0..1); fades after a hit.
    flash: f32,
    /// When to reload the level, once dead.
    reload_at: Option<f32>,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        PlayerHealth { hp: MAX_HEALTH, last_hit: f32::MIN, flash: 0.0, reload_at: None }
    }
}

impl PlayerHealth {
    pub fn dead(&self) -> bool {
        self.hp <= 0.0
    }
}

fn start_reload(health: &mut PlayerHealth, fade: &mut ScreenFade, now: f32, real: f32) {
    if health.reload_at.is_some() {
        return;
    }
    health.reload_at = Some(now + RELOAD_AFTER);
    *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: real, duration: RELOAD_AFTER * 0.75, hold: 10.0, out: true, active: true, stay: true };
}

#[allow(clippy::too_many_arguments)]
fn take_damage(
    time: Res<Time>,
    mut hits: MessageReader<DamagePlayer>,
    mut health: ResMut<PlayerHealth>,
    mut fade: ResMut<ScreenFade>,
    mut loaded: MessageReader<LevelLoaded>,
    map: Option<Res<CurrentMap>>,
    mut load: MessageWriter<LoadLevel>,
    ents: Option<Res<Ents>>,
) {
    if loaded.read().count() > 0 {
        *health = PlayerHealth::default();
    }
    let Some(now) = ents.as_ref().map(|e| e.now) else {
        hits.clear();
        return;
    };
    let dt = time.delta_secs();
    for DamagePlayer(d) in hits.read() {
        if health.dead() || *d <= 0.0 {
            continue;
        }
        health.hp -= d;
        health.last_hit = now;
        health.flash = (health.flash + d / 40.0).min(0.8);
        if health.dead() {
            info!("portal2: the player died");
            start_reload(&mut health, &mut fade, now, time.elapsed_secs());
        }
    }
    if !health.dead() && now - health.last_hit > REGEN_DELAY {
        health.hp = (health.hp + REGEN_RATE * dt).min(MAX_HEALTH);
    }
    health.flash = (health.flash - dt * 0.8).max(0.0);
    if let (Some(at), Some(map)) = (health.reload_at, map) {
        if now >= at {
            health.reload_at = None;
            load.write(LoadLevel { map: map.info.name.clone(), menu: false });
            *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: time.elapsed_secs(), duration: 1.0, hold: 0.0, out: false, active: true, stay: false };
        }
    }
}

/// `player_loadsaved`: fade out and start the level again.
fn reload_inputs(
    time: Res<Time>,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut health: ResMut<PlayerHealth>,
    mut fade: ResMut<ScreenFade>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class == "player_loadsaved" && msg.input == "reload" {
            start_reload(&mut health, &mut fade, ents.now, time.elapsed_secs());
        }
    }
}

#[derive(Component)]
struct HurtTint;

/// A red wash over the screen after a hit.
fn draw_hurt(mut commands: Commands, health: Res<PlayerHealth>, mut tint: Query<(Entity, &mut BackgroundColor), With<HurtTint>>) {
    let a = health.flash * 0.5;
    match (tint.single_mut(), a > 0.001) {
        (Ok((_, mut bg)), true) => bg.0 = Color::srgba(0.7, 0.0, 0.0, a),
        (Ok((e, _)), false) => commands.entity(e).despawn(),
        (Err(_), true) => {
            commands.spawn((
                HurtTint,
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(Color::srgba(0.7, 0.0, 0.0, a)),
                GlobalZIndex(3),
                bevy::picking::Pickable::IGNORE,
            ));
        }
        _ => {}
    }
}
