//! Player options, kept in `~/Library/Application Support/Portal2Rust/settings.cfg`
//! (or the platform's config folder) and applied to the controller, camera,
//! audio and window as they change.
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use p2_core::MainCamera;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings::load()).add_systems(Update, apply_settings);
    }
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub effects_volume: f32,
    /// Source `sensitivity` (default 3).
    pub mouse_sensitivity: f32,
    pub invert_mouse: bool,
    /// `fov_desired`, 75..90.
    pub fov: f32,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { master_volume: 0.8, music_volume: 0.6, effects_volume: 1.0, mouse_sensitivity: 3.0, invert_mouse: false, fov: 75.0, fullscreen: false }
    }
}

/// Where settings and saves live.
pub fn data_dir() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Portal2Rust")
    } else {
        home.join(".local/share/portal2rust")
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(data_dir().join("settings.cfg")) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once(' ') else { continue };
            let f = v.trim().parse::<f32>().ok();
            match (k.trim(), f) {
                ("volume", Some(f)) => s.master_volume = f.clamp(0.0, 1.0),
                ("snd_musicvolume", Some(f)) => s.music_volume = f.clamp(0.0, 1.0),
                ("snd_effectsvolume", Some(f)) => s.effects_volume = f.clamp(0.0, 1.0),
                ("sensitivity", Some(f)) => s.mouse_sensitivity = f.clamp(0.1, 20.0),
                ("m_invert", Some(f)) => s.invert_mouse = f != 0.0,
                ("fov_desired", Some(f)) => s.fov = f.clamp(75.0, 90.0),
                ("fullscreen", Some(f)) => s.fullscreen = f != 0.0,
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let text = format!(
            "volume {}\nsnd_musicvolume {}\nsnd_effectsvolume {}\nsensitivity {}\nm_invert {}\nfov_desired {}\nfullscreen {}\n",
            self.master_volume, self.music_volume, self.effects_volume, self.mouse_sensitivity, self.invert_mouse as u8, self.fov, self.fullscreen as u8
        );
        if let Err(e) = std::fs::write(data_dir().join("settings.cfg"), text) {
            warn!("portal2: can't save settings: {e}");
        }
    }
}

fn apply_settings(
    settings: Res<Settings>,
    mut applied: Local<Option<Settings>>,
    mut movement: ResMut<p2_movement::MovementConfig>,
    mut fx: ResMut<p2_fx::FxSettings>,
    mut cams: Query<&mut Projection, With<MainCamera>>,
    new_cams: Query<(), Added<MainCamera>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if applied.as_ref() == Some(&*settings) && new_cams.is_empty() {
        return;
    }
    movement.mouse_sensitivity = settings.mouse_sensitivity;
    movement.invert_pitch = settings.invert_mouse;
    fx.master_volume = settings.master_volume * settings.effects_volume;
    for mut p in &mut cams {
        if let Projection::Perspective(pp) = &mut *p {
            pp.fov = p2_view::source_fov_to_vertical(settings.fov);
        }
    }
    if let Ok(mut w) = windows.single_mut() {
        let want = if settings.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
        if w.mode != want {
            w.mode = want;
        }
    }
    *applied = Some(settings.clone());
}
