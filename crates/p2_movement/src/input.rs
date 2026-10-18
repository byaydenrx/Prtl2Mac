//! Optional keyboard and mouse input for the controller. Host apps that drive
//! [`MoveInput`] and [`ViewAngles`] themselves don't need this plugin.
//!
//! Nothing is read while [`InputLocked`] is set, and the cursor is never grabbed then.
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use p2_core::{InputBindings, InputLocked, InputState, MoveInput, Player, P2Systems, ViewAngles};

use crate::config::MovementConfig;
use crate::rules;

/// Keyboard, mouse look and cursor grab, in `Update` / `P2Systems::Input`.
///
/// WASD writes [`MoveInput::wish`], Ctrl [`MoveInput::duck`], Space latches
/// [`MoveInput::jump`] (the fixed tick consumes it), E sets [`MoveInput::use_key`].
/// The keys come from [`InputBindings`] when the app has one (those are the defaults).
/// Mouse motion turns [`ViewAngles`] while the cursor is grabbed. Left click
/// grabs the cursor and Esc releases it. Does nothing while [`InputLocked`] is true.
pub struct InputPlugin;

impl bevy::app::Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (grab_cursor, mouse_look, read_keys).in_set(P2Systems::Input));
    }
}

/// Click to grab the cursor, Esc to release it.
fn grab_cursor(
    locked: Res<InputLocked>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if locked.0 {
        return;
    }
    let Ok(mut c) = cursor.single_mut() else { return };
    if mouse.is_some_and(|m| m.just_pressed(MouseButton::Left)) && c.grab_mode == CursorGrabMode::None {
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
    }
    if keys.is_some_and(|k| k.just_pressed(KeyCode::Escape)) {
        c.grab_mode = CursorGrabMode::None;
        c.visible = true;
    }
}

/// Mouse look into [`ViewAngles`], only while the cursor is grabbed.
fn mouse_look(
    locked: Res<InputLocked>,
    cfg: Res<MovementConfig>,
    mouse: Option<Res<AccumulatedMouseMotion>>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    mut q: Query<&mut ViewAngles, With<Player>>,
    mut last: Local<Vec2>,
) {
    if locked.0 {
        return;
    }
    let grabbed = cursor.single().map(|c| c.grab_mode != CursorGrabMode::None).unwrap_or(false);
    let Some(mouse) = mouse else { return };
    let raw = if grabbed { mouse.delta } else { Vec2::ZERO };
    // m_filter: the mean of this frame's and the last frame's movement.
    let delta = if cfg.mouse_filter { (raw + *last) * 0.5 } else { raw };
    *last = raw;
    if delta == Vec2::ZERO {
        return;
    }
    for mut view in &mut q {
        let (yaw, pitch) = rules::look(&cfg, view.yaw, view.pitch, delta);
        view.yaw = yaw;
        view.pitch = pitch;
    }
}

/// Keyboard (as bound in [`InputBindings`]) into [`MoveInput`].
#[allow(clippy::type_complexity)]
fn read_keys(
    locked: Res<InputLocked>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    (mouse, scroll, bindings): (Option<Res<ButtonInput<MouseButton>>>, Option<Res<AccumulatedMouseScroll>>, Option<Res<InputBindings>>),
    mut q: Query<&mut MoveInput, With<Player>>,
) {
    if locked.0 {
        return;
    }
    let Some(keys) = keys else { return };
    let default = InputBindings::default();
    let b = bindings.as_deref().unwrap_or(&default);
    let input = InputState { keys: Some(&keys), mouse: mouse.as_deref(), scroll: scroll.as_deref() };
    let axis = |pos: &str, neg: &str| (b.pressed(pos, &input) as i32 - b.pressed(neg, &input) as i32) as f32;
    let wish = Vec2::new(axis("+moveright", "+moveleft"), axis("+forward", "+back"));
    let duck = b.pressed("+duck", &input);
    let jump = b.just_pressed("+jump", &input);
    let use_key = b.just_pressed("+use", &input);
    for mut input in &mut q {
        input.wish = wish;
        input.duck = duck;
        input.use_key = use_key;
        if jump {
            input.jump = true;
        }
    }
}
