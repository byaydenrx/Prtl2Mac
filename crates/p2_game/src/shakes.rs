//! `env_shake`: `StartShake` shakes the player's view for `duration` seconds
//! (`amplitude` inches, `frequency` shakes a second), weaker the further the
//! player is inside `radius` and not at all outside it, unless spawnflag 1
//! (GlobalShake). `StopShake` ends it; `Amplitude`/`Frequency` change a
//! running one. sp_a1_intro1's first orange portal starts
//! `shake_portal_spawn_room1`. p2_view applies the shakes to the camera.
//! Not done: the physics/rope pushes (flags 8, 16) and the on-ground check.
use bevy::prelude::*;
use p2_core::{Player, ScreenShake, ScreenShakes};

use crate::io::{EntInput, Ents};
use crate::level::LevelLoaded;

pub struct ShakesPlugin;

impl Plugin for ShakesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, shake_inputs.after(crate::io::run_io).in_set(crate::level::GameSet::Logic));
    }
}

const GLOBAL_SHAKE: u32 = 1;
const NO_VIEW_SHAKE: u32 = 32;

/// The amplitude a player `dist` units from a shake feels: full at the
/// centre, fading linearly to nothing at `radius`.
fn local_amplitude(amplitude: f32, dist: f32, radius: f32, global: bool) -> f32 {
    if global {
        return amplitude;
    }
    if radius <= 0.0 || dist >= radius {
        return 0.0;
    }
    amplitude * (1.0 - dist / radius)
}

fn shake_inputs(
    mut inputs: MessageReader<EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    time: Res<Time>,
    players: Query<&Transform, With<Player>>,
    mut shakes: ResMut<ScreenShakes>,
) {
    let now = time.elapsed_secs();
    if loaded.read().count() > 0 {
        shakes.0.clear();
    }
    shakes.0.retain(|s| s.end > now);
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let e = &ents.list[msg.target];
        if e.class != "env_shake" {
            continue;
        }
        let i = msg.target;
        let param = msg.param.trim().parse::<f32>().ok();
        match msg.input.as_str() {
            "startshake" => {
                if e.spawnflags & NO_VIEW_SHAKE != 0 {
                    continue;
                }
                let Ok(player) = players.single() else { continue };
                let dist = player.translation.distance(e.transform.translation) / shared::SOURCE_UNIT;
                let radius = e.get_f32("radius").unwrap_or(0.0);
                let amplitude = local_amplitude(e.get_f32("amplitude").unwrap_or(0.0), dist, radius, e.spawnflags & GLOBAL_SHAKE != 0);
                let duration = e.get_f32("duration").unwrap_or(1.0).max(0.01);
                debug!("portal2: env_shake {:?} start, amplitude {amplitude:.2} at {dist:.0} u", e.name);
                shakes.0.retain(|s| s.source != Some(i));
                if amplitude > 0.0 {
                    shakes.0.push(ScreenShake {
                        source: Some(i),
                        start: now,
                        end: now + duration,
                        amplitude,
                        frequency: e.get_f32("frequency").unwrap_or(40.0),
                    });
                }
            }
            "stopshake" => shakes.0.retain(|s| s.source != Some(i)),
            "amplitude" => {
                for s in shakes.0.iter_mut().filter(|s| s.source == Some(i)) {
                    s.amplitude = param.unwrap_or(s.amplitude);
                }
            }
            "frequency" => {
                for s in shakes.0.iter_mut().filter(|s| s.source == Some(i)) {
                    s.frequency = param.unwrap_or(s.frequency);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_fades_with_distance() {
        assert_eq!(local_amplitude(4.0, 0.0, 500.0, false), 4.0);
        assert!((local_amplitude(4.0, 250.0, 500.0, false) - 2.0).abs() < 1e-6);
        assert_eq!(local_amplitude(4.0, 600.0, 500.0, false), 0.0);
        assert_eq!(local_amplitude(4.0, 6000.0, 500.0, true), 4.0);
    }
}
