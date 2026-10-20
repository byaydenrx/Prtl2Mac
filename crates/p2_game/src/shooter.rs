//! `env_shooter`: `Shoot` throws `m_igibs` copies of its `shootmodel` along
//! its facing at `m_flvelocity` (plus up to `m_flvariance` of scatter); each
//! falls under gravity and is removed after `m_flgiblife` seconds. The
//! loader spawns the model once, hidden, as the prototype. sp_a1_wakeup's
//! portal gun drop is one (glados_camera.nut fires `gun_shooter Shoot`).
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::io::{EntInput, Ents};

pub struct ShooterPlugin;

impl Plugin for ShooterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (shoot, expire_gibs).after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

/// A thrown copy, removed at `until` (game seconds).
#[derive(Component)]
struct ShooterGib {
    until: f32,
}

/// Launch velocity of gib `n` (world, metres/s): `speed` along `forward`,
/// tilted by up to `variance` (0..1) in a fixed spread so runs repeat.
fn gib_velocity(forward: Vec3, speed: f32, variance: f32, n: u32) -> Vec3 {
    let f = forward.normalize_or(Vec3::X);
    if variance <= 0.0 {
        return f * speed;
    }
    let side = f.any_orthonormal_vector();
    let up = f.cross(side);
    let a = n as f32 * 2.399_963; // golden angle
    (f + (side * a.cos() + up * a.sin()) * variance.clamp(0.0, 1.0)).normalize() * speed
}

fn shoot(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    time: Res<Time>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "env_shooter" || msg.input != "shoot" {
            continue;
        }
        let Some(proto) = e.entity else { continue };
        let tf = e.transform;
        let speed = e.get_f32("m_flvelocity").unwrap_or(200.0) * shared::SOURCE_UNIT;
        let variance = e.get_f32("m_flvariance").unwrap_or(0.0);
        let life = e.get_f32("m_flgiblife").filter(|l| *l > 0.0).unwrap_or(4.0);
        let gravity = e.get_f32("gibgravityscale").unwrap_or(1.0);
        let count = e.get_f32("m_igibs").unwrap_or(1.0).max(1.0) as u32;
        // The model's box (a model entity's bounds are model-local metres).
        let (center, half) = e.bounds.map_or((Vec3::ZERO, Vec3::splat(4.0 * shared::SOURCE_UNIT)), |(lo, hi)| {
            ((lo + hi) * 0.5, ((hi - lo) * 0.5).max(Vec3::splat(0.5 * shared::SOURCE_UNIT)))
        });
        for n in 0..count {
            let gib = commands
                .entity(proto)
                .clone_and_spawn_with_opt_out(|b| {
                    b.linked_cloning(true);
                })
                .id();
            commands.entity(gib).remove::<ChildOf>().insert((
                Transform::from_translation(tf.translation).with_rotation(tf.rotation).with_scale(tf.scale),
                Visibility::Visible,
                RigidBody::Dynamic,
                LinearVelocity(gib_velocity(tf.rotation * Vec3::X, speed, variance, n)),
                GravityScale(gravity),
                Friction::new(0.6),
                Restitution::new(0.2),
                ShooterGib { until: time.elapsed_secs() + life },
                crate::level::LevelOwned,
            ));
            commands.spawn((Name::new("shooter gib collider"), Collider::cuboid(half.x * 2.0, half.y * 2.0, half.z * 2.0), Transform::from_translation(center), ChildOf(gib)));
            debug!("portal2: {:?} shoots gib {n}", e.name);
        }
    }
}

fn expire_gibs(mut commands: Commands, gibs: Query<(Entity, &ShooterGib)>, time: Res<Time>) {
    for (e, g) in &gibs {
        if time.elapsed_secs() >= g.until {
            commands.entity(e).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gib_goes_along_facing_without_variance() {
        let v = gib_velocity(Vec3::new(1.0, 0.0, 1.0), 2.0, 0.0, 3);
        assert!((v - Vec3::new(1.0, 0.0, 1.0).normalize() * 2.0).length() < 1e-5);
    }

    #[test]
    fn variance_scatters_but_keeps_speed() {
        let a = gib_velocity(Vec3::X, 3.0, 0.5, 0);
        let b = gib_velocity(Vec3::X, 3.0, 0.5, 1);
        assert!((a.length() - 3.0).abs() < 1e-4 && (b.length() - 3.0).abs() < 1e-4);
        assert!((a - b).length() > 0.1 && a.x > 0.0 && b.x > 0.0);
    }
}
