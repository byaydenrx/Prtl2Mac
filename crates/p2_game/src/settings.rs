//! Player options, kept in `~/Library/Application Support/Portal2Rust/settings.cfg`
//! (or the platform's config folder) and applied to the controller, camera,
//! audio, renderer and window as they change, plus the frame limiter.
//!
//! The file is `key value` lines with a `version` line first, then the key
//! bindings as `unbindall` / `bind "key" "command"` lines like Source's
//! config.cfg (only once the player changed one). Files from older builds
//! (no `version`) read as before; lines this build doesn't know (from a newer
//! build) are kept and written back unchanged.
//!
//! With no file (first launch) everything is Portal 2's default. Options
//! the engine doesn't use yet are still kept and shown; each says what
//! should read it.
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::window::{Monitor, MonitorSelection, PresentMode, PrimaryMonitor, PrimaryWindow, VideoModeSelection, WindowMode};
use p2_core::MainCamera;
use source_assets::valve_render::ValveQuality;

use crate::bindings::{parse_bind, BindTable};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings::load()).add_systems(Update, apply_settings).add_systems(Last, limit_frame_rate);
    }
}

/// Written as the `version` line. 1 is the file from before the line existed;
/// 3 added `closecaption` (with it, `cc_subtitles` means dialogue only, as
/// in Source) and the bindings.
pub const SETTINGS_VERSION: u32 = 3;

/// `fov_desired`: the game's 75 up to 110.
pub const FOV_RANGE: (f32, f32) = (75.0, 110.0);

/// The frame limit slider (`fps_max`), 0 is unlimited.
pub const FRAME_LIMITS: [u32; 7] = [30, 60, 120, 144, 165, 240, 0];

/// Fixed sizes offered besides native: common 4:3, 16:9 and 16:10.
pub const RESOLUTIONS: [(u32, u32); 13] = [
    (1024, 768),
    (1280, 720),
    (1280, 800),
    (1280, 960),
    (1440, 900),
    (1600, 900),
    (1600, 1200),
    (1680, 1050),
    (1920, 1080),
    (1920, 1200),
    (2560, 1440),
    (2560, 1600),
    (3840, 2160),
];

/// Anisotropic filtering steps (`mat_forceaniso`); 1 is off.
pub const ANISOTROPY: [u16; 5] = [1, 2, 4, 8, 16];

/// `mat_monitorgamma`: the Brightness slider, 2.2 by default.
pub const GAMMA_RANGE: (f32, f32) = (1.6, 2.6);

/// Captioning: off, dialogue only, or dialogue and sound effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptionMode {
    #[default]
    Off,
    Subtitles,
    ClosedCaptions,
}

impl CaptionMode {
    pub const ALL: [CaptionMode; 3] = [Self::Off, Self::Subtitles, Self::ClosedCaptions];

    pub fn shown(self) -> bool {
        self != Self::Off
    }
}

/// `snd_surround_speakers`. TODO: audio.rs mixes stereo whatever this says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Speakers {
    Headphones,
    #[default]
    Stereo,
    Quad,
    Surround51,
    Surround71,
}

impl Speakers {
    pub const ALL: [Speakers; 5] = [Self::Headphones, Self::Stereo, Self::Quad, Self::Surround51, Self::Surround71];

    pub fn cvar(self) -> u32 {
        match self {
            Self::Headphones => 0,
            Self::Stereo => 2,
            Self::Quad => 4,
            Self::Surround51 => 5,
            Self::Surround71 => 7,
        }
    }

    pub fn from_cvar(v: f32) -> Self {
        Self::ALL.into_iter().find(|s| s.cvar() as f32 == v).unwrap_or_default()
    }
}

/// The caption languages Portal 2 ships (`cc_lang`); captions fall back to
/// English when the install has no file for one.
pub const LANGUAGES: [&str; 20] = [
    "english",
    "french",
    "german",
    "spanish",
    "latam",
    "italian",
    "portuguese",
    "brazilian",
    "russian",
    "polish",
    "czech",
    "hungarian",
    "dutch",
    "danish",
    "finnish",
    "norwegian",
    "swedish",
    "japanese",
    "koreana",
    "schinese",
];

/// Caption text sizes (px) and their names.
pub const SUBTITLE_SIZES: [(f32, &str); 4] = [(16.0, "SMALL"), (20.0, "MEDIUM"), (26.0, "LARGE"), (32.0, "HUGE")];

/// Caption text colour: the game's own per-speaker colours, or one colour for all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubtitleColor {
    #[default]
    Original,
    White,
    Yellow,
    Cyan,
    Green,
}

impl SubtitleColor {
    pub const ALL: [SubtitleColor; 5] = [Self::Original, Self::White, Self::Yellow, Self::Cyan, Self::Green];

    /// The word in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::White => "white",
            Self::Yellow => "yellow",
            Self::Cyan => "cyan",
            Self::Green => "green",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key().eq_ignore_ascii_case(s))
    }

    /// `None` keeps the caption's own colour.
    pub fn color(self) -> Option<Color> {
        match self {
            Self::Original => None,
            Self::White => Some(Color::srgb(0.95, 0.95, 0.95)),
            Self::Yellow => Some(Color::srgb(1.0, 0.9, 0.3)),
            Self::Cyan => Some(Color::srgb(0.45, 0.9, 1.0)),
            Self::Green => Some(Color::srgb(0.5, 1.0, 0.5)),
        }
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
    /// Mouse smoothing (`m_filter`).
    pub mouse_filter: bool,
    /// `fov_desired`, see [`FOV_RANGE`].
    pub fov: f32,
    pub fullscreen: bool,
    /// Physical pixels for fullscreen, window size for windowed. `None` is
    /// native: the monitor in fullscreen, the launch size in a window.
    pub resolution: Option<UVec2>,
    /// `mat_vsync`: Fifo when on, AutoNoVsync when off.
    pub vsync: bool,
    /// `fps_max`, 0 is unlimited.
    pub frame_limit: u32,
    /// `mat_antialias`: MSAA 4x on the screen cameras when on (Bevy's default).
    pub msaa: bool,
    /// `mat_forceaniso`, one of [`ANISOTROPY`].
    pub anisotropy: u16,
    /// The Original / Enhanced graphics preset. Only stored for now; see
    /// [`ValveQuality::enhanced`].
    pub enhanced_graphics: bool,
    /// Use raw mouse motion (`m_rawinput`). On by default because the
    /// controller always reads raw motion. TODO: p2_movement::input should
    /// use OS-accelerated cursor motion when off.
    pub raw_input: bool,
    /// `m_customaccel`. TODO: p2_movement::rules::look should accelerate.
    pub mouse_accel: bool,
    /// The player's key bindings; `None` until one is changed (the game's
    /// defaults apply, see `bindings::DefaultBinds`).
    pub binds: Option<BindTable>,
    /// Controller (`joystick`, `joy_sensitivity`, `joy_inverty`,
    /// `joy_vibration`). TODO: nothing reads gamepads yet.
    pub joystick: bool,
    pub joy_sensitivity: f32,
    pub joy_invert: bool,
    pub joy_vibration: bool,
    /// Speaker configuration.
    pub speakers: Speakers,
    /// Captioning (`closecaption` + `cc_subtitles`).
    pub captions: CaptionMode,
    /// Caption language (`cc_lang`), one of [`LANGUAGES`].
    pub language: String,
    /// Brightness (`mat_monitorgamma`), see [`GAMMA_RANGE`]. TODO: the
    /// renderer should apply it (Portal 2 ramps the output gamma).
    pub gamma: f32,
    /// Filtering mode below anisotropic: trilinear (`mat_trilinear`) or
    /// bilinear. TODO: source_assets::valve_render samplers should read it.
    pub trilinear: bool,
    /// `mat_motion_blur_enabled`. TODO: no motion blur pass yet.
    pub motion_blur: bool,
    /// Multicore rendering (`mat_queue_mode`). Bevy always renders on its
    /// own thread; kept for the menu.
    pub multicore: bool,
    /// Shader detail 0..3 (`gpu_level`), effect detail 0..2 (`cpu_level`),
    /// model/texture detail 0..2 (`gpu_mem_level`), paged pool memory 0..2
    /// (`mem_level`). TODO: the renderer draws everything at the top level.
    pub shader_detail: u8,
    pub effect_detail: u8,
    pub model_detail: u8,
    pub paged_pool: u8,
    /// Caption text size in px.
    pub subtitle_size: f32,
    pub subtitle_color: SubtitleColor,
    /// Opacity of the box behind captions, 0..1.
    pub subtitle_background: f32,
    /// Lines this build doesn't know (from a newer one), written back as they were.
    pub unknown: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 1.0,
            effects_volume: 1.0,
            mouse_sensitivity: 3.0,
            invert_mouse: false,
            mouse_filter: false,
            fov: 75.0,
            fullscreen: true,
            resolution: None,
            vsync: true,
            frame_limit: 0,
            msaa: true,
            anisotropy: 16,
            enhanced_graphics: false,
            raw_input: true,
            mouse_accel: false,
            binds: None,
            joystick: false,
            joy_sensitivity: 1.0,
            joy_invert: false,
            joy_vibration: true,
            speakers: Speakers::Stereo,
            captions: CaptionMode::Off,
            language: "english".into(),
            gamma: 2.2,
            trilinear: true,
            motion_blur: true,
            multicore: true,
            shader_detail: 3,
            effect_detail: 2,
            model_detail: 2,
            paged_pool: 2,
            subtitle_size: 20.0,
            subtitle_color: SubtitleColor::Original,
            subtitle_background: 0.6,
            unknown: Vec::new(),
        }
    }
}

/// Scripted test runs leave the player's settings and save alone, unless
/// `P2_DATA_DIR` points them at a test folder.
pub fn may_write_data() -> bool {
    std::env::var_os("P2_SCRIPT").is_none() || std::env::var_os("P2_DATA_DIR").is_some()
}

/// Where settings and saves live (`P2_DATA_DIR` overrides it).
pub fn data_dir() -> std::path::PathBuf {
    if let Some(d) = std::env::var_os("P2_DATA_DIR") {
        let d = std::path::PathBuf::from(d);
        let _ = std::fs::create_dir_all(&d);
        return d;
    }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Portal2Rust")
    } else {
        home.join(".local/share/portal2rust")
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// `1920x1080`, or `native`/anything else for `None`.
fn parse_resolution(v: &str) -> Option<UVec2> {
    let (w, h) = v.split_once(['x', 'X'])?;
    let (w, h) = (w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?);
    (w >= 320 && h >= 200 && w <= 16384 && h <= 16384).then_some(UVec2::new(w, h))
}

/// The step after (`dir` > 0) or before `current` in `list`, stopping at the
/// ends. A value that isn't in the list (hand-edited file) snaps to the
/// nearest entry by `rank` first.
pub fn step_in<T: Copy + PartialEq>(list: &[T], current: T, dir: i32, rank: impl Fn(T) -> f32) -> T {
    match list.iter().position(|x| *x == current) {
        Some(i) => list[(i as i32 + dir).clamp(0, list.len() as i32 - 1) as usize],
        None => *list.iter().min_by(|a, b| (rank(**a) - rank(current)).abs().total_cmp(&(rank(**b) - rank(current)).abs())).unwrap_or(&current),
    }
}

/// For [`step_in`]: unlimited counts as the fastest.
pub fn frame_limit_rank(fps: u32) -> f32 {
    if fps == 0 {
        1.0e6
    } else {
        fps as f32
    }
}

/// Every key this version reads and writes.
const KEYS: [&str; 38] = [
    "volume",
    "snd_musicvolume",
    "snd_effectsvolume",
    "sensitivity",
    "m_invert",
    "m_filter",
    "fov_desired",
    "fullscreen",
    "resolution",
    "mat_vsync",
    "fps_max",
    "mat_antialias",
    "mat_forceaniso",
    "enhanced_graphics",
    "cc_subtitles",
    "cc_size",
    "cc_color",
    "cc_bg_opacity",
    "version",
    "m_rawinput",
    "m_customaccel",
    "joystick",
    "joy_sensitivity",
    "joy_inverty",
    "joy_vibration",
    "snd_surround_speakers",
    "closecaption",
    "cc_lang",
    "mat_monitorgamma",
    "mat_trilinear",
    "mat_motion_blur_enabled",
    "mat_queue_mode",
    "gpu_level",
    "cpu_level",
    "gpu_mem_level",
    "mem_level",
    "unbindall",
    "bind",
];

impl Settings {
    pub fn load() -> Self {
        match std::fs::read_to_string(data_dir().join("settings.cfg")) {
            Ok(text) => Self::parse(&text),
            Err(_) => Settings::default(),
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut s = Settings::default();
        let mut version = 1;
        let (mut closecaption, mut cc_subtitles) = (None, None);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once(char::is_whitespace).map_or((line, ""), |(k, v)| (k, v.trim()));
            // Source configs quote values (`sensitivity "3"`).
            let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
            let f = v.parse::<f32>().ok().filter(|f| f.is_finite());
            let lvl = |max: u8| f.map(|f| f.clamp(0.0, max as f32) as u8);
            match (k, f) {
                ("version", _) => version = f.map_or(1, |f| f as u32),
                ("volume", Some(f)) => s.master_volume = f.clamp(0.0, 1.0),
                ("snd_musicvolume", Some(f)) => s.music_volume = f.clamp(0.0, 1.0),
                ("snd_effectsvolume", Some(f)) => s.effects_volume = f.clamp(0.0, 1.0),
                ("sensitivity", Some(f)) => s.mouse_sensitivity = f.clamp(0.1, 20.0),
                ("m_invert", Some(f)) => s.invert_mouse = f != 0.0,
                ("m_filter", Some(f)) => s.mouse_filter = f != 0.0,
                ("fov_desired", Some(f)) => s.fov = f.clamp(FOV_RANGE.0, FOV_RANGE.1),
                ("fullscreen", Some(f)) => s.fullscreen = f != 0.0,
                ("resolution", _) => s.resolution = parse_resolution(v),
                ("mat_vsync", Some(f)) => s.vsync = f != 0.0,
                ("fps_max", Some(f)) => s.frame_limit = if f < 1.0 { 0 } else { f.clamp(10.0, 1000.0) as u32 },
                ("mat_antialias", Some(f)) => s.msaa = f >= 2.0,
                ("mat_forceaniso", Some(f)) => s.anisotropy = ANISOTROPY.into_iter().rfind(|a| *a as f32 <= f).unwrap_or(1),
                ("enhanced_graphics", Some(f)) => s.enhanced_graphics = f != 0.0,
                ("cc_subtitles", Some(f)) => cc_subtitles = Some(f != 0.0),
                ("closecaption", Some(f)) => closecaption = Some(f != 0.0),
                ("cc_lang", _) if LANGUAGES.contains(&v.to_ascii_lowercase().as_str()) => s.language = v.to_ascii_lowercase(),
                ("m_rawinput", Some(f)) => s.raw_input = f != 0.0,
                ("m_customaccel", Some(f)) => s.mouse_accel = f != 0.0,
                ("joystick", Some(f)) => s.joystick = f != 0.0,
                ("joy_sensitivity", Some(f)) => s.joy_sensitivity = f.clamp(0.1, 5.0),
                ("joy_inverty", Some(f)) => s.joy_invert = f != 0.0,
                ("joy_vibration", Some(f)) => s.joy_vibration = f != 0.0,
                ("snd_surround_speakers", Some(f)) => s.speakers = Speakers::from_cvar(f),
                ("mat_monitorgamma", Some(f)) => s.gamma = f.clamp(GAMMA_RANGE.0, GAMMA_RANGE.1),
                ("mat_trilinear", Some(f)) => s.trilinear = f != 0.0,
                ("mat_motion_blur_enabled", Some(f)) => s.motion_blur = f != 0.0,
                ("mat_queue_mode", Some(f)) => s.multicore = f != 0.0,
                ("gpu_level", _) if lvl(3).is_some() => s.shader_detail = lvl(3).unwrap_or(3),
                ("cpu_level", _) if lvl(2).is_some() => s.effect_detail = lvl(2).unwrap_or(2),
                ("gpu_mem_level", _) if lvl(2).is_some() => s.model_detail = lvl(2).unwrap_or(2),
                ("mem_level", _) if lvl(2).is_some() => s.paged_pool = lvl(2).unwrap_or(2),
                ("unbindall", _) => s.binds = Some(BindTable::default()),
                ("bind", _) => {
                    if let Some((key, cmd)) = parse_bind(line) {
                        s.binds.get_or_insert_with(BindTable::default).0.insert(key, cmd);
                    }
                }
                ("cc_size", Some(f)) => s.subtitle_size = f.clamp(10.0, 48.0),
                ("cc_color", _) => s.subtitle_color = SubtitleColor::from_key(v).unwrap_or_default(),
                ("cc_bg_opacity", Some(f)) => s.subtitle_background = f.clamp(0.0, 1.0),
                // Ours, with a value that doesn't read: keep the default.
                _ if KEYS.contains(&k) => {}
                _ => s.unknown.push(line.to_string()),
            }
        }
        // Captions: before version 3 `cc_subtitles` alone switched them on.
        s.captions = match (closecaption, cc_subtitles) {
            (Some(false), _) => CaptionMode::Off,
            (Some(true), Some(true)) => CaptionMode::Subtitles,
            (Some(true), _) => CaptionMode::ClosedCaptions,
            (None, Some(true)) => CaptionMode::Subtitles,
            (None, Some(false)) => CaptionMode::Off,
            (None, None) => s.captions,
        };
        // Version 1 used the same keys and narrower ranges, so it reads as is.
        // Later changes to a key's meaning convert old values here.
        if version > SETTINGS_VERSION {
            info!("portal2: settings.cfg is from a newer version ({version}); keeping what this one doesn't know");
        }
        s
    }

    pub fn to_text(&self) -> String {
        let res = self.resolution.map_or("native".to_string(), |r| format!("{}x{}", r.x, r.y));
        let mut text = format!(
            "version {}\nvolume {}\nsnd_musicvolume {}\nsnd_effectsvolume {}\nsensitivity {}\nm_invert {}\nm_filter {}\nfov_desired {}\nfullscreen {}\nresolution {}\nmat_vsync {}\nfps_max {}\nmat_antialias {}\nmat_forceaniso {}\nenhanced_graphics {}\ncc_subtitles {}\ncc_size {}\ncc_color {}\ncc_bg_opacity {}\n\
             closecaption {}\ncc_lang {}\nm_rawinput {}\nm_customaccel {}\njoystick {}\njoy_sensitivity {}\njoy_inverty {}\njoy_vibration {}\nsnd_surround_speakers {}\n\
             mat_monitorgamma {}\nmat_trilinear {}\nmat_motion_blur_enabled {}\nmat_queue_mode {}\ngpu_level {}\ncpu_level {}\ngpu_mem_level {}\nmem_level {}\n",
            SETTINGS_VERSION,
            self.master_volume,
            self.music_volume,
            self.effects_volume,
            self.mouse_sensitivity,
            self.invert_mouse as u8,
            self.mouse_filter as u8,
            self.fov,
            self.fullscreen as u8,
            res,
            self.vsync as u8,
            self.frame_limit,
            if self.msaa { 4 } else { 0 },
            self.anisotropy,
            self.enhanced_graphics as u8,
            (self.captions == CaptionMode::Subtitles) as u8,
            self.subtitle_size,
            self.subtitle_color.key(),
            self.subtitle_background,
            self.captions.shown() as u8,
            self.language,
            self.raw_input as u8,
            self.mouse_accel as u8,
            self.joystick as u8,
            self.joy_sensitivity,
            self.joy_invert as u8,
            self.joy_vibration as u8,
            self.speakers.cvar(),
            self.gamma,
            self.trilinear as u8,
            self.motion_blur as u8,
            if self.multicore { 2 } else { 0 },
            self.shader_detail,
            self.effect_detail,
            self.model_detail,
            self.paged_pool,
        );
        if let Some(b) = &self.binds {
            text.push_str(&b.to_lines());
        }
        for l in &self.unknown {
            text.push_str(l);
            text.push('\n');
        }
        text
    }

    pub fn save(&self) {
        if !may_write_data() {
            return;
        }
        if let Err(e) = std::fs::write(data_dir().join("settings.cfg"), self.to_text()) {
            warn!("portal2: can't save settings: {e}");
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_settings(
    settings: Res<Settings>,
    mut applied: Local<Option<Settings>>,
    mut launch_size: Local<Option<Vec2>>,
    mut movement: ResMut<p2_movement::MovementConfig>,
    mut fx: ResMut<p2_fx::FxSettings>,
    quality: Option<ResMut<ValveQuality>>,
    mut cams: Query<&mut Projection, With<MainCamera>>,
    mut screen_cams: Query<&mut Msaa, Or<(With<MainCamera>, With<p2_view::ViewModelCamera>)>>,
    new_cams: Query<(), Added<Camera3d>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
) {
    if applied.as_ref() == Some(&*settings) && new_cams.is_empty() {
        return;
    }
    movement.mouse_sensitivity = settings.mouse_sensitivity;
    movement.invert_pitch = settings.invert_mouse;
    movement.mouse_filter = settings.mouse_filter;
    fx.master_volume = settings.master_volume * settings.effects_volume;
    let mut n = 0;
    for mut p in &mut cams {
        if let Projection::Perspective(pp) = &mut *p {
            pp.fov = p2_view::source_fov_to_vertical(settings.fov);
            n += 1;
        }
    }
    // The main view and the viewmodel share the window, so they share MSAA.
    // (Portal views render to their own textures without it.)
    let msaa = if settings.msaa { Msaa::Sample4 } else { Msaa::Off };
    for mut m in &mut screen_cams {
        if *m != msaa {
            *m = msaa;
        }
    }
    if let Some(mut q) = quality {
        let want = ValveQuality { anisotropy: settings.anisotropy, enhanced: settings.enhanced_graphics };
        if *q != want {
            *q = want;
        }
    }
    debug!("portal2: settings applied (fov {} on {n} cameras)", settings.fov);
    // Scripted test runs always stay in a window at their own size.
    let scripted = std::env::var_os("P2_SCRIPT").is_some() || std::env::args().any(|a| a == "--screenshot" || a == "--shot" || a == "--exit-after");
    if let Ok(mut w) = windows.single_mut() {
        let present = if settings.vsync { PresentMode::Fifo } else { PresentMode::AutoNoVsync };
        if w.present_mode != present {
            w.present_mode = present;
        }
        if launch_size.is_none() && w.mode == WindowMode::Windowed {
            *launch_size = Some(w.resolution.size());
        }
        let resolution = settings.resolution.filter(|_| !scripted);
        let want = if !settings.fullscreen || scripted {
            WindowMode::Windowed
        } else {
            // A fixed size takes the monitor's video mode of that size (its
            // fastest); without one, borderless at the monitor's own size.
            let mode = resolution.and_then(|r| {
                let mut mons: Vec<_> = monitors.iter().collect();
                mons.sort_by_key(|(_, primary)| !primary);
                mons.first().and_then(|(m, _)| m.video_modes.iter().filter(|v| v.physical_size == r).max_by_key(|v| (v.refresh_rate_millihertz, v.bit_depth)).copied())
            });
            if resolution.is_some() && mode.is_none() {
                warn!("portal2: no {:?} fullscreen mode on this monitor; using its native size", resolution);
            }
            match mode {
                Some(m) => WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Specific(m)),
                None => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            }
        };
        if w.mode != want {
            w.mode = want;
        }
        // Size a window only when its size or mode setting changes, so the
        // player's own resizing stays; native goes back to the launch size.
        let res_changed = applied.as_ref().is_some_and(|a| a.resolution != settings.resolution);
        let mode_changed = applied.as_ref().is_none_or(|a| a.fullscreen != settings.fullscreen);
        if want == WindowMode::Windowed && !scripted {
            match (resolution, *launch_size) {
                (Some(r), _) if res_changed || mode_changed => w.resolution.set(r.x as f32, r.y as f32),
                (None, Some(s)) if res_changed => w.resolution.set(s.x, s.y),
                _ => {}
            }
        }
    }
    *applied = Some(settings.clone());
}

/// Below this much time left, spin instead of sleeping. Grows with how late
/// the OS wakes us up.
const SPIN_MIN: Duration = Duration::from_micros(250);
const SPIN_MAX: Duration = Duration::from_millis(4);

/// When the next frame may start: one period after the last deadline, or
/// now when the frame ran late (no catching up with a burst of frames).
pub fn next_deadline(last: Option<Instant>, now: Instant, period: Duration) -> Instant {
    match last {
        Some(d) if d + period > now => d + period,
        _ => now,
    }
}

/// How long to sleep with `left` until the deadline, keeping `spin` back to
/// spin through; `None` once it's time to spin.
pub fn sleep_part(left: Duration, spin: Duration) -> Option<Duration> {
    (left > spin).then(|| left - spin)
}

/// The spin margin after a sleep that overslept by `over`: jumps up to cover
/// a late wake-up, eases back down slowly.
pub fn update_spin(spin: Duration, over: Duration) -> Duration {
    let eased = spin.mul_f32(0.99);
    eased.max(over + over / 2).clamp(SPIN_MIN, SPIN_MAX)
}

/// The frame limit: at the end of each frame, wait for the next frame's
/// start time with a coarse sleep and a spin over the last bit, so frames
/// come out evenly spaced.
fn limit_frame_rate(settings: Res<Settings>, mut deadline: Local<Option<Instant>>, mut spin: Local<Option<Duration>>) {
    if settings.frame_limit == 0 {
        *deadline = None;
        return;
    }
    let period = Duration::from_secs_f64(1.0 / settings.frame_limit as f64);
    let target = next_deadline(*deadline, Instant::now(), period);
    let mut margin = spin.unwrap_or(Duration::from_millis(1));
    loop {
        let now = Instant::now();
        if now >= target {
            break;
        }
        match sleep_part(target - now, margin) {
            Some(d) => {
                std::thread::sleep(d);
                let over = Instant::now().saturating_duration_since(now + d);
                margin = update_spin(margin, over);
            }
            None => std::hint::spin_loop(),
        }
    }
    *spin = Some(margin);
    *deadline = Some(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let s = Settings::default();
        assert_eq!(Settings::parse(&s.to_text()), s);
        assert!(s.to_text().starts_with(&format!("version {SETTINGS_VERSION}\n")));
    }

    #[test]
    fn changed_values_round_trip() {
        let s = Settings {
            master_volume: 0.3,
            fov: 105.0,
            fullscreen: true,
            resolution: Some(UVec2::new(2560, 1440)),
            vsync: false,
            frame_limit: 144,
            msaa: false,
            anisotropy: 4,
            enhanced_graphics: true,
            captions: CaptionMode::ClosedCaptions,
            language: "german".into(),
            joystick: true,
            joy_sensitivity: 1.5,
            speakers: Speakers::Surround51,
            gamma: 1.8,
            trilinear: false,
            motion_blur: false,
            multicore: false,
            shader_detail: 1,
            effect_detail: 0,
            model_detail: 1,
            paged_pool: 0,
            raw_input: false,
            mouse_accel: true,
            binds: Some(BindTable::builtin()),
            subtitle_size: 26.0,
            subtitle_color: SubtitleColor::Yellow,
            subtitle_background: 0.2,
            ..Settings::default()
        };
        assert_eq!(Settings::parse(&s.to_text()), s);
    }

    #[test]
    fn old_file_reads_with_new_defaults() {
        // A settings.cfg from before `version` existed.
        let old = "volume 0.5\nsnd_musicvolume 0.4\nsnd_effectsvolume 1\nsensitivity 2.5\nm_invert 1\nm_filter 0\nfov_desired 90\nfullscreen 1\ncc_subtitles 0\n";
        let s = Settings::parse(old);
        assert_eq!(s.master_volume, 0.5);
        assert_eq!(s.mouse_sensitivity, 2.5);
        assert!(s.invert_mouse && s.fullscreen);
        assert_eq!(s.captions, CaptionMode::Off);
        assert_eq!(Settings::parse("cc_subtitles 1\n").captions, CaptionMode::Subtitles);
        assert_eq!(s.fov, 90.0);
        let d = Settings::default();
        assert_eq!((s.vsync, s.frame_limit, s.msaa, s.anisotropy, s.resolution), (d.vsync, d.frame_limit, d.msaa, d.anisotropy, d.resolution));
        assert!(s.unknown.is_empty());
    }

    #[test]
    fn unknown_keys_are_kept_and_bad_values_ignored() {
        let s = Settings::parse("version 9\nfuture_thing 1 2 3\nvolume loud\nfov_desired 200\nfps_max 0\nresolution 12x3\ncc_color mauve\n");
        assert_eq!(s.unknown, vec!["future_thing 1 2 3".to_string()]);
        assert_eq!(s.master_volume, Settings::default().master_volume);
        assert_eq!(s.fov, FOV_RANGE.1);
        assert_eq!(s.frame_limit, 0);
        assert_eq!(s.resolution, None);
        assert_eq!(s.subtitle_color, SubtitleColor::Original);
        let again = Settings::parse(&s.to_text());
        assert_eq!(again, s);
    }

    #[test]
    fn first_launch_is_portal2s_defaults() {
        let s = Settings::default();
        assert_eq!((s.master_volume, s.music_volume, s.mouse_sensitivity, s.fov), (1.0, 1.0, 3.0, 75.0));
        assert!(s.fullscreen && s.resolution.is_none() && s.vsync);
        assert_eq!(s.captions, CaptionMode::Off);
        assert_eq!(s.language, "english");
        assert_eq!(s.gamma, 2.2);
        assert!(!s.invert_mouse && !s.mouse_filter && !s.mouse_accel);
        assert_eq!(s.speakers, Speakers::Stereo);
        assert!(s.motion_blur && s.multicore);
        assert_eq!((s.model_detail, s.effect_detail, s.shader_detail, s.paged_pool), (2, 2, 3, 2));
        // Extras at the original behaviour: no cap, the Original look, the game's own caption colours.
        assert_eq!((s.frame_limit, s.enhanced_graphics, s.subtitle_color), (0, false, SubtitleColor::Original));
        // Bindings: the game's defaults until the player changes one.
        assert!(s.binds.is_none());
        assert!(!s.to_text().contains("bind"));
    }

    #[test]
    fn captions_old_and_new() {
        // Version 2 and older: cc_subtitles switched captions on or off.
        assert_eq!(Settings::parse("version 2\ncc_subtitles 1\n").captions, CaptionMode::Subtitles);
        assert_eq!(Settings::parse("version 2\ncc_subtitles 0\n").captions, CaptionMode::Off);
        // Version 3: Source's pair.
        assert_eq!(Settings::parse("closecaption 1\ncc_subtitles 0\n").captions, CaptionMode::ClosedCaptions);
        assert_eq!(Settings::parse("closecaption 1\ncc_subtitles 1\n").captions, CaptionMode::Subtitles);
        assert_eq!(Settings::parse("closecaption 0\ncc_subtitles 1\n").captions, CaptionMode::Off);
        for m in CaptionMode::ALL {
            let s = Settings { captions: m, ..Settings::default() };
            assert_eq!(Settings::parse(&s.to_text()).captions, m);
        }
        assert_eq!(Settings::parse("cc_lang klingon\n").language, "english");
        assert_eq!(Settings::parse("cc_lang \"German\"\n").language, "german");
    }

    #[test]
    fn bindings_in_the_file() {
        let mut b = BindTable::builtin();
        b.rebind("+jump", "mwheeldown");
        let s = Settings { binds: Some(b.clone()), ..Settings::default() };
        let text = s.to_text();
        assert!(text.contains("unbindall\nbind"));
        let back = Settings::parse(&text);
        assert_eq!(back.binds, Some(b));
        assert!(back.unknown.is_empty());
        // Everything unbound stays unbound.
        let none = Settings { binds: Some(BindTable::default()), ..Settings::default() };
        assert_eq!(Settings::parse(&none.to_text()).binds, Some(BindTable::default()));
        // A version 2 file with no bindings keeps the defaults.
        assert!(Settings::parse("version 2\nvolume 0.5\n").binds.is_none());
    }

    #[test]
    fn quoted_source_values_read() {
        let s = Settings::parse("sensitivity \"2.5\"\nmat_monitorgamma \"2.0\"\nsnd_surround_speakers \"5\"\ngpu_level 9\nmem_level -3\nmat_queue_mode -1\n");
        assert_eq!(s.mouse_sensitivity, 2.5);
        assert_eq!(s.gamma, 2.0);
        assert_eq!(s.speakers, Speakers::Surround51);
        assert_eq!((s.shader_detail, s.paged_pool), (3, 0));
        assert!(s.multicore);
        assert!(s.unknown.is_empty());
    }

    #[test]
    fn anisotropy_snaps_down() {
        assert_eq!(Settings::parse("mat_forceaniso 0").anisotropy, 1);
        assert_eq!(Settings::parse("mat_forceaniso 6").anisotropy, 4);
        assert_eq!(Settings::parse("mat_forceaniso 16").anisotropy, 16);
        assert_eq!(Settings::parse("mat_forceaniso 64").anisotropy, 16);
    }

    #[test]
    fn stepping_lists() {
        assert_eq!(step_in(&FRAME_LIMITS, 0, 1, frame_limit_rank), 0);
        assert_eq!(step_in(&FRAME_LIMITS, 0, -1, frame_limit_rank), 240);
        assert_eq!(step_in(&FRAME_LIMITS, 30, -1, frame_limit_rank), 30);
        assert_eq!(step_in(&FRAME_LIMITS, 60, 1, frame_limit_rank), 120);
        // 75 from a hand-edited file snaps to 60 before stepping.
        assert_eq!(step_in(&FRAME_LIMITS, 75, 1, frame_limit_rank), 60);
    }

    #[test]
    fn deadlines_keep_cadence_and_resync_when_late() {
        let t0 = Instant::now();
        let p = Duration::from_micros(16_667);
        assert_eq!(next_deadline(None, t0, p), t0);
        // On time: exactly one period after the last deadline, whatever the work took.
        assert_eq!(next_deadline(Some(t0), t0 + Duration::from_millis(5), p), t0 + p);
        // Late: start now instead of catching up.
        let late = t0 + Duration::from_millis(40);
        assert_eq!(next_deadline(Some(t0), late, p), late);
    }

    #[test]
    fn sleep_then_spin() {
        let spin = Duration::from_millis(1);
        assert_eq!(sleep_part(Duration::from_millis(10), spin), Some(Duration::from_millis(9)));
        assert_eq!(sleep_part(Duration::from_micros(800), spin), None);
        // A late wake-up widens the margin at once; it shrinks back slowly and within bounds.
        let wide = update_spin(spin, Duration::from_millis(2));
        assert_eq!(wide, Duration::from_millis(3));
        let eased = update_spin(wide, Duration::ZERO);
        assert!(eased < wide && eased > Duration::from_micros(2900));
        assert_eq!(update_spin(SPIN_MIN, Duration::ZERO), SPIN_MIN);
        assert_eq!(update_spin(spin, Duration::from_millis(50)), SPIN_MAX);
    }
}
