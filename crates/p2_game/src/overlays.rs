//! Corner overlays, both off by default (Options, or `cl_showfps` /
//! `cl_showpos` in the console):
//!
//! * FPS: frames per second over the last second with the frame time's
//!   min / avg / max, to check v-sync and the frame cap.
//! * Movement stats: speed (and horizontal speed) in units per second,
//!   position and view in Source units, and whether the player is on the ground.
use std::collections::VecDeque;

use bevy::prelude::*;
use p2_core::*;

use crate::level::CurrentMap;
use crate::settings::Settings;

pub struct OverlaysPlugin;

impl Plugin for OverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStats>().add_systems(Last, record_frame).add_systems(Update, draw_overlays.in_set(crate::level::GameSet::Logic));
    }
}

/// Frame times of the last second.
#[derive(Resource, Default)]
pub struct FrameStats {
    /// (real time at the frame, its frame time), oldest first.
    frames: VecDeque<(f64, f64)>,
}

/// What the FPS line shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSummary {
    pub fps: f64,
    /// Milliseconds.
    pub min: f64,
    pub avg: f64,
    pub max: f64,
}

impl FrameStats {
    /// Window the numbers cover, seconds.
    pub const WINDOW: f64 = 1.0;

    pub fn push(&mut self, now: f64, dt: f64) {
        if dt > 0.0 {
            self.frames.push_back((now, dt));
        }
        while self.frames.front().is_some_and(|(t, _)| now - t > Self::WINDOW) {
            self.frames.pop_front();
        }
    }

    pub fn summary(&self) -> Option<FrameSummary> {
        if self.frames.is_empty() {
            return None;
        }
        let n = self.frames.len() as f64;
        let total: f64 = self.frames.iter().map(|f| f.1).sum();
        let min = self.frames.iter().map(|f| f.1).fold(f64::MAX, f64::min);
        let max = self.frames.iter().map(|f| f.1).fold(0.0, f64::max);
        Some(FrameSummary { fps: n / total, min: min * 1000.0, avg: total / n * 1000.0, max: max * 1000.0 })
    }
}

pub fn fps_text(s: &FrameSummary, map: &str) -> String {
    format!("{:.0} fps  {map}\nframe {:.2} ms  min {:.2}  max {:.2}", s.fps, s.avg, s.min, s.max)
}

/// The cl_showpos block: speed in u/s, position and angles in Source terms.
pub fn pos_text(feet: Vec3, vel: Vec3, view: &ViewAngles, on_ground: bool) -> String {
    let p = crate::console::to_source(feet);
    let a = crate::console::source_angles(view);
    let v = vel / SOURCE_UNIT;
    let horizontal = Vec2::new(v.x, v.z).length();
    format!(
        "pos:  {:.2} {:.2} {:.2}\nang:  {:.2} {:.2} {:.2}\nvel:  {:.2}  (horizontal {:.2})\nground: {}",
        p.x,
        p.y,
        p.z,
        a.x,
        a.y,
        a.z,
        v.length(),
        horizontal,
        if on_ground { "yes" } else { "no" }
    )
}

/// Every frame's real duration (frame start to frame start, so the frame
/// limiter's wait and v-sync are included).
fn record_frame(time: Res<Time<Real>>, settings: Res<Settings>, mut stats: ResMut<FrameStats>) {
    if !settings.tools.show_fps {
        if !stats.frames.is_empty() {
            stats.frames.clear();
        }
        return;
    }
    stats.push(time.elapsed_secs_f64(), time.delta_secs_f64());
}

#[derive(Component)]
struct OverlayRoot;
#[derive(Component)]
struct FpsText;
#[derive(Component)]
struct PosText;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_overlays(
    mut commands: Commands,
    settings: Res<Settings>,
    stats: Res<FrameStats>,
    fonts: Option<Res<crate::ui::UiFonts>>,
    real: Res<Time<Real>>,
    mut last_update: Local<f64>,
    map: Option<Res<CurrentMap>>,
    roots: Query<Entity, With<OverlayRoot>>,
    mut fps: Query<(&mut Text, &mut Node), (With<FpsText>, Without<PosText>)>,
    mut pos: Query<(&mut Text, &mut Node), (With<PosText>, Without<FpsText>)>,
    players: Query<(&Transform, &CharacterVelocity, &ViewAngles, &Grounded), With<Player>>,
) {
    let t = &settings.tools;
    if !t.show_fps && !t.show_pos {
        for r in &roots {
            commands.entity(r).despawn();
        }
        return;
    }
    let Some(fonts) = fonts else { return };
    if roots.is_empty() {
        let tf = TextFont { font: (&fonts.regular).into(), font_size: bevy::text::FontSize::Px(15.0), ..default() };
        commands
            .spawn((
                OverlayRoot,
                Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(12.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
                GlobalZIndex(45),
                bevy::picking::Pickable::IGNORE,
            ))
            .with_children(|p| {
                p.spawn((FpsText, Text::new(""), tf.clone(), TextColor(Color::srgb(0.95, 0.95, 0.6))));
                p.spawn((PosText, Text::new(""), tf, TextColor(Color::srgb(0.9, 0.92, 0.95))));
            });
        return;
    }
    // Four updates a second keep the numbers readable.
    let now = real.elapsed_secs_f64();
    let refresh = now - *last_update >= 0.25 || settings.is_changed();
    if refresh {
        *last_update = now;
    }
    for (mut text, mut node) in &mut fps {
        node.display = if t.show_fps { Display::Flex } else { Display::None };
        if t.show_fps && refresh {
            let name = map.as_ref().map(|m| m.info.name.as_str()).unwrap_or("");
            text.0 = stats.summary().map(|s| fps_text(&s, name)).unwrap_or_default();
        }
    }
    for (mut text, mut node) in &mut pos {
        node.display = if t.show_pos { Display::Flex } else { Display::None };
        if t.show_pos {
            let s = players.single().map(|(tf, v, view, g)| pos_text(tf.translation, v.0, view, g.on_ground)).unwrap_or_default();
            if text.0 != s {
                text.0 = s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_window_and_summary() {
        let mut s = FrameStats::default();
        assert_eq!(s.summary(), None);
        // 0.5 s of 10 ms frames, then 0.6 s of 20 ms frames: the first ones age out.
        let mut t = 0.0;
        for _ in 0..50 {
            t += 0.01;
            s.push(t, 0.01);
        }
        for _ in 0..30 {
            t += 0.02;
            s.push(t, 0.02);
        }
        let sum = s.summary().unwrap();
        assert!((sum.max - 20.0).abs() < 1e-9 && (sum.min - 10.0).abs() < 1e-9);
        // About 1 s of frames: 30 of 20 ms and ~20 of 10 ms.
        let n = s.frames.len() as f64;
        assert!((sum.fps - n / s.frames.iter().map(|f| f.1).sum::<f64>()).abs() < 1e-9);
        assert!(s.frames.iter().all(|(ft, _)| t - ft <= FrameStats::WINDOW));
        s.push(t + 0.01, 0.0);
        assert!(s.summary().is_some());
    }

    #[test]
    fn steady_frame_rate() {
        let mut s = FrameStats::default();
        for i in 1..=240 {
            s.push(i as f64 / 120.0, 1.0 / 120.0);
        }
        let sum = s.summary().unwrap();
        assert!((sum.fps - 120.0).abs() < 1e-6);
        assert!((sum.avg - 8.3333).abs() < 1e-3);
        let text = fps_text(&sum, "sp_a1_intro3");
        assert!(text.starts_with("120 fps  sp_a1_intro3\nframe 8.33 ms"), "{text}");
    }

    #[test]
    fn movement_text() {
        let feet = source_to_world(Vec3::new(-704.0, 96.0, 64.0));
        // 300 u/s along Source +X and 200 u/s up.
        let vel = source_to_world(Vec3::new(300.0, 0.0, 200.0));
        let view = crate::console::view_from_source(0.0, 90.0, 0.0);
        let t = pos_text(feet, vel, &view, false);
        assert!(t.contains("pos:  -704.00 96.00 64.00"), "{t}");
        assert!(t.contains("ang:  0.00 90.00 0.00"), "{t}");
        assert!(t.contains("vel:  360.56  (horizontal 300.00)"), "{t}");
        assert!(t.ends_with("ground: no"));
    }
}
