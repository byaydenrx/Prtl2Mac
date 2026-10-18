//! Key bindings: which keys and mouse buttons drive each game command
//! (`+forward`, `+jump`, ...). Input systems read [`InputBindings`]; with no
//! resource they use [`InputBindings::default`], the original keys.
//!
//! Buttons are named as in Source configs (`w`, `space`, `ctrl`, `mouse1`,
//! `mwheeldown`), see [`InputButton::from_name`].
use std::collections::HashMap;

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

/// A key, mouse button or wheel direction a command can be bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputButton {
    Key(KeyCode),
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
}

/// Source key names and what they are. Left-hand modifiers are the plain
/// names (`shift`, `ctrl`, `alt`) like Source; right-hand ones get an `r`.
const NAMES: &[(&str, InputButton)] = {
    use InputButton::*;
    use KeyCode as K;
    &[
        ("a", Key(K::KeyA)), ("b", Key(K::KeyB)), ("c", Key(K::KeyC)), ("d", Key(K::KeyD)), ("e", Key(K::KeyE)),
        ("f", Key(K::KeyF)), ("g", Key(K::KeyG)), ("h", Key(K::KeyH)), ("i", Key(K::KeyI)), ("j", Key(K::KeyJ)),
        ("k", Key(K::KeyK)), ("l", Key(K::KeyL)), ("m", Key(K::KeyM)), ("n", Key(K::KeyN)), ("o", Key(K::KeyO)),
        ("p", Key(K::KeyP)), ("q", Key(K::KeyQ)), ("r", Key(K::KeyR)), ("s", Key(K::KeyS)), ("t", Key(K::KeyT)),
        ("u", Key(K::KeyU)), ("v", Key(K::KeyV)), ("w", Key(K::KeyW)), ("x", Key(K::KeyX)), ("y", Key(K::KeyY)),
        ("z", Key(K::KeyZ)),
        ("0", Key(K::Digit0)), ("1", Key(K::Digit1)), ("2", Key(K::Digit2)), ("3", Key(K::Digit3)), ("4", Key(K::Digit4)),
        ("5", Key(K::Digit5)), ("6", Key(K::Digit6)), ("7", Key(K::Digit7)), ("8", Key(K::Digit8)), ("9", Key(K::Digit9)),
        ("space", Key(K::Space)), ("enter", Key(K::Enter)), ("tab", Key(K::Tab)), ("escape", Key(K::Escape)),
        ("backspace", Key(K::Backspace)), ("capslock", Key(K::CapsLock)),
        ("shift", Key(K::ShiftLeft)), ("rshift", Key(K::ShiftRight)), ("ctrl", Key(K::ControlLeft)), ("rctrl", Key(K::ControlRight)),
        ("alt", Key(K::AltLeft)), ("ralt", Key(K::AltRight)), ("lwin", Key(K::SuperLeft)), ("rwin", Key(K::SuperRight)),
        ("apps", Key(K::ContextMenu)),
        ("ins", Key(K::Insert)), ("del", Key(K::Delete)), ("home", Key(K::Home)), ("end", Key(K::End)),
        ("pgup", Key(K::PageUp)), ("pgdn", Key(K::PageDown)),
        ("uparrow", Key(K::ArrowUp)), ("downarrow", Key(K::ArrowDown)), ("leftarrow", Key(K::ArrowLeft)), ("rightarrow", Key(K::ArrowRight)),
        ("f1", Key(K::F1)), ("f2", Key(K::F2)), ("f3", Key(K::F3)), ("f4", Key(K::F4)), ("f5", Key(K::F5)), ("f6", Key(K::F6)),
        ("f7", Key(K::F7)), ("f8", Key(K::F8)), ("f9", Key(K::F9)), ("f10", Key(K::F10)), ("f11", Key(K::F11)), ("f12", Key(K::F12)),
        ("pause", Key(K::Pause)), ("scrolllock", Key(K::ScrollLock)), ("numlock", Key(K::NumLock)),
        ("semicolon", Key(K::Semicolon)), ("'", Key(K::Quote)), (",", Key(K::Comma)), (".", Key(K::Period)), ("/", Key(K::Slash)),
        ("\\", Key(K::Backslash)), ("-", Key(K::Minus)), ("=", Key(K::Equal)), ("[", Key(K::BracketLeft)), ("]", Key(K::BracketRight)),
        ("`", Key(K::Backquote)),
        ("kp_ins", Key(K::Numpad0)), ("kp_end", Key(K::Numpad1)), ("kp_downarrow", Key(K::Numpad2)), ("kp_pgdn", Key(K::Numpad3)),
        ("kp_leftarrow", Key(K::Numpad4)), ("kp_5", Key(K::Numpad5)), ("kp_rightarrow", Key(K::Numpad6)), ("kp_home", Key(K::Numpad7)),
        ("kp_uparrow", Key(K::Numpad8)), ("kp_pgup", Key(K::Numpad9)), ("kp_del", Key(K::NumpadDecimal)), ("kp_slash", Key(K::NumpadDivide)),
        ("kp_multiply", Key(K::NumpadMultiply)), ("kp_minus", Key(K::NumpadSubtract)), ("kp_plus", Key(K::NumpadAdd)), ("kp_enter", Key(K::NumpadEnter)),
        ("mouse1", Mouse(MouseButton::Left)), ("mouse2", Mouse(MouseButton::Right)), ("mouse3", Mouse(MouseButton::Middle)),
        ("mouse4", Mouse(MouseButton::Back)), ("mouse5", Mouse(MouseButton::Forward)),
        ("mwheelup", WheelUp), ("mwheeldown", WheelDown),
    ]
};

impl InputButton {
    /// From a Source key name, any case (`W`, `SPACE`, `MOUSE1`).
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        // `;` is spelled out in configs, but accept it bare too.
        let name = if name == ";" { "semicolon".to_string() } else { name };
        NAMES.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
    }

    /// The Source key name, lowercase.
    pub fn name(self) -> Option<&'static str> {
        NAMES.iter().find(|(_, b)| *b == self).map(|(n, _)| *n)
    }

    /// Every key, button and wheel direction that has a name.
    pub fn all() -> impl Iterator<Item = InputButton> {
        NAMES.iter().map(|(_, b)| *b)
    }
}

/// The buttons bound to each command, by command name (`+forward`).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct InputBindings(pub HashMap<String, Vec<InputButton>>);

impl Default for InputBindings {
    /// The original keys: WASD, Space, either Ctrl, E, both mouse buttons.
    fn default() -> Self {
        use InputButton::*;
        let pairs: [(&str, &[InputButton]); 9] = [
            ("+forward", &[Key(KeyCode::KeyW)]),
            ("+back", &[Key(KeyCode::KeyS)]),
            ("+moveleft", &[Key(KeyCode::KeyA)]),
            ("+moveright", &[Key(KeyCode::KeyD)]),
            ("+jump", &[Key(KeyCode::Space)]),
            ("+duck", &[Key(KeyCode::ControlLeft), Key(KeyCode::ControlRight)]),
            ("+use", &[Key(KeyCode::KeyE)]),
            ("+attack", &[Mouse(MouseButton::Left)]),
            ("+attack2", &[Mouse(MouseButton::Right)]),
        ];
        Self(pairs.into_iter().map(|(c, b)| (c.to_string(), b.to_vec())).collect())
    }
}

/// What the input devices are doing this frame, for [`InputBindings`].
pub struct InputState<'a> {
    pub keys: Option<&'a ButtonInput<KeyCode>>,
    pub mouse: Option<&'a ButtonInput<MouseButton>>,
    pub scroll: Option<&'a AccumulatedMouseScroll>,
}

impl InputState<'_> {
    /// A wheel notch counts as a press for one frame.
    fn wheel(&self, up: bool) -> bool {
        self.scroll.is_some_and(|s| if up { s.delta.y > 0.0 } else { s.delta.y < 0.0 })
    }

    pub fn pressed(&self, b: InputButton) -> bool {
        match b {
            InputButton::Key(k) => self.keys.is_some_and(|i| i.pressed(k)),
            InputButton::Mouse(m) => self.mouse.is_some_and(|i| i.pressed(m)),
            InputButton::WheelUp => self.wheel(true),
            InputButton::WheelDown => self.wheel(false),
        }
    }

    pub fn just_pressed(&self, b: InputButton) -> bool {
        match b {
            InputButton::Key(k) => self.keys.is_some_and(|i| i.just_pressed(k)),
            InputButton::Mouse(m) => self.mouse.is_some_and(|i| i.just_pressed(m)),
            InputButton::WheelUp => self.wheel(true),
            InputButton::WheelDown => self.wheel(false),
        }
    }
}

impl InputBindings {
    pub fn buttons(&self, command: &str) -> &[InputButton] {
        self.0.get(command).map_or(&[], Vec::as_slice)
    }

    /// Is any button of `command` held?
    pub fn pressed(&self, command: &str, input: &InputState) -> bool {
        self.buttons(command).iter().any(|b| input.pressed(*b))
    }

    /// Did any button of `command` go down this frame?
    pub fn just_pressed(&self, command: &str, input: &InputState) -> bool {
        self.buttons(command).iter().any(|b| input.just_pressed(*b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for b in InputButton::all() {
            assert_eq!(InputButton::from_name(b.name().unwrap()), Some(b));
        }
        assert_eq!(InputButton::from_name("SPACE"), Some(InputButton::Key(KeyCode::Space)));
        assert_eq!(InputButton::from_name("MOUSE2"), Some(InputButton::Mouse(MouseButton::Right)));
        assert_eq!(InputButton::from_name(";"), Some(InputButton::Key(KeyCode::Semicolon)));
        assert_eq!(InputButton::from_name("joy1"), None);
    }

    #[test]
    fn defaults_are_the_original_keys() {
        let b = InputBindings::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ControlRight);
        keys.press(KeyCode::KeyW);
        let input = InputState { keys: Some(&keys), mouse: None, scroll: None };
        assert!(b.pressed("+duck", &input) && b.pressed("+forward", &input));
        assert!(b.just_pressed("+forward", &input));
        assert!(!b.pressed("+back", &input) && !b.pressed("+nothing", &input));
    }
}
