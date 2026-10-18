//! The developer, overlay and speedrun options, kept in `settings.cfg` with
//! the rest ([`crate::settings::Settings::tools`]). Every one is off (or the
//! original behaviour) until the player turns it on in Options.
use std::path::PathBuf;

use bevy::prelude::*;

/// How the speedrun timer counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeedrunMode {
    /// One run across maps: restarts and deaths keep the clock going.
    #[default]
    FullRun,
    /// Each map is its own run: every (re)start of a map starts the clock again.
    SingleMap,
}

impl SpeedrunMode {
    pub const ALL: [SpeedrunMode; 2] = [Self::FullRun, Self::SingleMap];

    pub fn key(self) -> &'static str {
        match self {
            Self::FullRun => "full",
            Self::SingleMap => "map",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::FullRun => "FULL RUN",
            Self::SingleMap => "SINGLE MAP",
        }
    }
}

/// Keys the speedrun options can be bound to (the Options page steps through them).
pub const BIND_KEYS: [(KeyCode, &str); 18] = [
    (KeyCode::F1, "F1"),
    (KeyCode::F2, "F2"),
    (KeyCode::F3, "F3"),
    (KeyCode::F4, "F4"),
    (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"),
    (KeyCode::F7, "F7"),
    (KeyCode::F8, "F8"),
    (KeyCode::F9, "F9"),
    (KeyCode::F10, "F10"),
    (KeyCode::F11, "F11"),
    (KeyCode::F12, "F12"),
    (KeyCode::Insert, "INSERT"),
    (KeyCode::Delete, "DELETE"),
    (KeyCode::Home, "HOME"),
    (KeyCode::End, "END"),
    (KeyCode::PageUp, "PAGEUP"),
    (KeyCode::PageDown, "PAGEDOWN"),
];

pub fn key_name(k: KeyCode) -> &'static str {
    BIND_KEYS.iter().find(|(c, _)| *c == k).map_or("?", |(_, n)| n)
}

pub fn key_from_name(s: &str) -> Option<KeyCode> {
    BIND_KEYS.iter().find(|(_, n)| n.eq_ignore_ascii_case(s.trim())).map(|(c, _)| *c)
}

/// Practice checkpoint slots the keys save to and load from.
pub const CHECKPOINT_SLOTS: u8 = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolSettings {
    /// `con_enable`: the ` key opens the developer console.
    pub dev_console: bool,
    /// `cl_showfps`: frame rate and frame times in a corner.
    pub show_fps: bool,
    /// `cl_showpos`: speed, position and ground state.
    pub show_pos: bool,
    /// The speedrun timer and automatic splits.
    pub speedrun_timer: bool,
    pub speedrun_mode: SpeedrunMode,
    /// A key that starts the current map again.
    pub quick_restart: bool,
    pub quick_restart_key: KeyCode,
    /// Practice checkpoints: save and load position and velocity.
    pub practice: bool,
    pub checkpoint_save_key: KeyCode,
    pub checkpoint_load_key: KeyCode,
    /// 1..=[`CHECKPOINT_SLOTS`]; the keys use slot `"1"`, `"2"`...
    pub checkpoint_slot: u8,
    /// Another folder to find community `.bsp` maps in (besides `<data>/maps`).
    pub community_maps_dir: Option<PathBuf>,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            dev_console: false,
            show_fps: false,
            show_pos: false,
            speedrun_timer: false,
            speedrun_mode: SpeedrunMode::FullRun,
            quick_restart: false,
            quick_restart_key: KeyCode::F6,
            practice: false,
            checkpoint_save_key: KeyCode::F7,
            checkpoint_load_key: KeyCode::F8,
            checkpoint_slot: 1,
            community_maps_dir: None,
        }
    }
}

impl ToolSettings {
    /// Read one `key value` line of `settings.cfg`. True when the key is one
    /// of these (a value that doesn't read leaves the default).
    pub fn parse(&mut self, k: &str, v: &str) -> bool {
        let flag = || v.trim().parse::<f32>().ok().map(|f| f != 0.0);
        match k {
            "con_enable" => self.dev_console = flag().unwrap_or(self.dev_console),
            "cl_showfps" => self.show_fps = flag().unwrap_or(self.show_fps),
            "cl_showpos" => self.show_pos = flag().unwrap_or(self.show_pos),
            "speedrun_timer" => self.speedrun_timer = flag().unwrap_or(self.speedrun_timer),
            "speedrun_mode" => self.speedrun_mode = SpeedrunMode::ALL.into_iter().find(|m| m.key() == v.trim()).unwrap_or_default(),
            "quick_restart" => self.quick_restart = flag().unwrap_or(self.quick_restart),
            "quick_restart_key" => self.quick_restart_key = key_from_name(v).unwrap_or(self.quick_restart_key),
            "practice_mode" => self.practice = flag().unwrap_or(self.practice),
            "checkpoint_save_key" => self.checkpoint_save_key = key_from_name(v).unwrap_or(self.checkpoint_save_key),
            "checkpoint_load_key" => self.checkpoint_load_key = key_from_name(v).unwrap_or(self.checkpoint_load_key),
            "checkpoint_slot" => {
                if let Ok(n) = v.trim().parse::<u8>() {
                    self.checkpoint_slot = n.clamp(1, CHECKPOINT_SLOTS);
                }
            }
            "community_maps_dir" => self.community_maps_dir = (!v.trim().is_empty()).then(|| PathBuf::from(v.trim())),
            _ => return false,
        }
        true
    }

    /// The lines for `settings.cfg`.
    pub fn to_text(&self) -> String {
        let mut s = format!(
            "con_enable {}\ncl_showfps {}\ncl_showpos {}\nspeedrun_timer {}\nspeedrun_mode {}\nquick_restart {}\nquick_restart_key {}\npractice_mode {}\ncheckpoint_save_key {}\ncheckpoint_load_key {}\ncheckpoint_slot {}\n",
            self.dev_console as u8,
            self.show_fps as u8,
            self.show_pos as u8,
            self.speedrun_timer as u8,
            self.speedrun_mode.key(),
            self.quick_restart as u8,
            key_name(self.quick_restart_key),
            self.practice as u8,
            key_name(self.checkpoint_save_key),
            key_name(self.checkpoint_load_key),
            self.checkpoint_slot,
        );
        if let Some(d) = &self.community_maps_dir {
            s += &format!("community_maps_dir {}\n", d.display());
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> ToolSettings {
        let mut t = ToolSettings::default();
        for l in text.lines() {
            let (k, v) = l.split_once(' ').unwrap_or((l, ""));
            t.parse(k, v.trim());
        }
        t
    }

    #[test]
    fn everything_starts_off() {
        let t = ToolSettings::default();
        assert!(!t.dev_console && !t.show_fps && !t.show_pos && !t.speedrun_timer && !t.quick_restart && !t.practice);
        assert_eq!((t.quick_restart_key, t.checkpoint_save_key, t.checkpoint_load_key), (KeyCode::F6, KeyCode::F7, KeyCode::F8));
    }

    #[test]
    fn round_trip() {
        let t = ToolSettings {
            dev_console: true,
            show_fps: true,
            speedrun_timer: true,
            speedrun_mode: SpeedrunMode::SingleMap,
            quick_restart: true,
            quick_restart_key: KeyCode::Home,
            practice: true,
            checkpoint_slot: 3,
            community_maps_dir: Some(PathBuf::from("/Users/me/My Maps")),
            ..Default::default()
        };
        assert_eq!(read(&t.to_text()), t);
        assert_eq!(read(&ToolSettings::default().to_text()), ToolSettings::default());
    }

    #[test]
    fn bad_values_keep_defaults() {
        let t = read("con_enable yes\nquick_restart_key Q\ncheckpoint_slot 99\nspeedrun_mode fast\n");
        let d = ToolSettings::default();
        assert_eq!((t.dev_console, t.quick_restart_key, t.speedrun_mode), (d.dev_console, d.quick_restart_key, d.speedrun_mode));
        assert_eq!(t.checkpoint_slot, CHECKPOINT_SLOTS);
        assert!(!ToolSettings::default().parse("volume", "1"));
    }

    #[test]
    fn kept_in_settings_cfg() {
        use crate::settings::Settings;
        let s = Settings { tools: ToolSettings { show_pos: true, practice: true, checkpoint_load_key: KeyCode::End, ..Default::default() }, ..Settings::default() };
        let back = Settings::parse(&s.to_text());
        assert_eq!(back, s);
        assert!(back.unknown.is_empty());
        // A settings.cfg from before these options: all off.
        assert_eq!(Settings::parse("version 2\nvolume 0.5\n").tools, ToolSettings::default());
    }

    #[test]
    fn key_names() {
        for (k, n) in BIND_KEYS {
            assert_eq!(key_from_name(n), Some(k));
            assert_eq!(key_name(k), n);
        }
        assert_eq!(key_from_name("pageup"), Some(KeyCode::PageUp));
    }
}
