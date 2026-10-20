//! What the Options screens show and how each option steps, kept apart
//! from drawing (ui.rs) so it can be tested.
//!
//! Like Portal 2, Options is a list of categories (Keyboard/Mouse,
//! Controller, Audio, Video); each opens its own dialog, and some dialogs
//! open a further one (Customize keyboard, Advanced video, and our
//! Subtitle style page). Our launcher-type pages (Speedrun, Mods) are two
//! more categories in the same list, so the main and pause menus keep
//! Portal 2's buttons. One function per page returns its rows. To add a
//! page: a variant in [`OptionsPage`], its parent, title and footer, a
//! function returning its rows, and a link row on its parent.
use bevy::math::UVec2;

use crate::bindings::{key_label, BindTable, KeyAction};
use crate::mods_page::ModsPage;
use crate::tool_settings::{key_name, SpeedrunMode, BIND_KEYS, CHECKPOINT_SLOTS};
use crate::settings::{
    frame_limit_rank, step_in, CaptionMode, Settings, Speakers, SubtitleColor, ANISOTROPY, FOV_RANGE, FRAME_LIMITS, GAMMA_RANGE, LANGUAGES, RESOLUTIONS,
    SUBTITLE_SIZES,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OptionsPage {
    /// The category list.
    #[default]
    List,
    KeyboardMouse,
    /// Customize keyboard: the key list.
    Keys,
    Controller,
    Audio,
    /// Our extra page under Audio: caption size, colour and background.
    Subtitles,
    Video,
    AdvancedVideo,
    /// Speedrun timer, quick restart and practice checkpoints.
    Speedrun,
    /// The installed mods and the mod profile.
    Mods,
}

/// The buttons along the bottom of a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footer {
    Back,
    Done,
    Cancel,
    Apply,
    UseDefaults,
    ClearKey,
}

impl Footer {
    pub fn label(self) -> &'static str {
        match self {
            Self::Back => "BACK",
            Self::Done => "DONE",
            Self::Cancel => "CANCEL",
            Self::Apply => "APPLY",
            Self::UseDefaults => "USE DEFAULTS",
            Self::ClearKey => "CLEAR KEY",
        }
    }
}

impl OptionsPage {
    /// Where Done/Cancel/Esc go; `None` leaves Options.
    pub fn parent(self) -> Option<OptionsPage> {
        match self {
            Self::List => None,
            Self::KeyboardMouse | Self::Controller | Self::Audio | Self::Video | Self::Speedrun | Self::Mods => Some(Self::List),
            Self::Keys => Some(Self::KeyboardMouse),
            Self::Subtitles => Some(Self::Audio),
            Self::AdvancedVideo => Some(Self::Video),
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::List => "OPTIONS",
            Self::KeyboardMouse => "KEYBOARD / MOUSE",
            Self::Keys => "CUSTOMIZE KEYBOARD",
            Self::Controller => "CONTROLLER",
            Self::Audio => "AUDIO",
            Self::Subtitles => "SUBTITLE STYLE",
            Self::Video => "VIDEO",
            Self::AdvancedVideo => "ADVANCED VIDEO",
            Self::Speedrun => "SPEEDRUN",
            Self::Mods => "MODS",
        }
    }

    pub fn footer(self) -> &'static [Footer] {
        match self {
            Self::List => &[Footer::Back],
            Self::Keys => &[Footer::UseDefaults, Footer::ClearKey, Footer::Done, Footer::Cancel],
            Self::Video => &[Footer::Apply, Footer::Done, Footer::Cancel],
            // Mod changes are written as they're made (with their own undo).
            Self::Mods => &[Footer::Done],
            _ => &[Footer::Done, Footer::Cancel],
        }
    }
}

/// Every option a row can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    // Keyboard / mouse
    Sensitivity,
    Invert,
    Smoothing,
    Accel,
    RawInput,
    // Controller
    Joystick,
    JoySensitivity,
    JoyInvert,
    JoyVibration,
    // Audio
    Master,
    Music,
    Speakers,
    Captions,
    Language,
    SubtitleSize,
    SubtitleColor,
    SubtitleBackground,
    // Video
    Resolution,
    Aspect,
    DisplayMode,
    Brightness,
    // Advanced video
    ModelDetail,
    EffectDetail,
    ShaderDetail,
    PagedPool,
    Multicore,
    AntiAliasing,
    Filtering,
    VSync,
    MotionBlur,
    Fov,
    FrameLimit,
    Graphics,
    // Extras (off by default)
    DevConsole,
    ShowFps,
    ShowPos,
    // Speedrun
    SpeedrunTimer,
    SpeedrunMode,
    QuickRestart,
    QuickRestartKey,
    Practice,
    SaveCheckpointKey,
    LoadCheckpointKey,
    CheckpointSlot,
    // Mods: the profile, which ui.rs steps through the mod store.
    ModProfile,
}

/// Resolution and display mode as picked on the Video page; they take
/// effect on Apply or Done, as in Portal 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoMode {
    pub resolution: Option<UVec2>,
    pub fullscreen: bool,
}

impl VideoMode {
    pub fn of(s: &Settings) -> Self {
        Self { resolution: s.resolution, fullscreen: s.fullscreen }
    }

    pub fn apply(self, s: &mut Settings) {
        s.resolution = self.resolution;
        s.fullscreen = self.fullscreen;
    }
}

/// The aspect ratios Portal 2's Video page offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    Normal,
    Wide16x9,
    Wide16x10,
}

impl Aspect {
    pub const ALL: [Aspect; 3] = [Self::Normal, Self::Wide16x9, Self::Wide16x10];

    pub fn ratio(self) -> f32 {
        match self {
            Self::Normal => 4.0 / 3.0,
            Self::Wide16x9 => 16.0 / 9.0,
            Self::Wide16x10 => 16.0 / 10.0,
        }
    }

    /// The nearest of the three.
    pub fn of(size: UVec2) -> Self {
        let r = size.x as f32 / size.y.max(1) as f32;
        Self::ALL.into_iter().min_by(|a, b| (a.ratio() - r).abs().total_cmp(&(b.ratio() - r).abs())).unwrap_or(Self::Wide16x9)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal (4:3)",
            Self::Wide16x9 => "Widescreen (16:9)",
            Self::Wide16x10 => "Widescreen (16:10)",
        }
    }
}

/// The resolutions listed for `aspect`: native first when it has that
/// aspect, then the common sizes and the display's own modes of that
/// aspect, smallest first.
pub fn resolutions_for(aspect: Aspect, native: UVec2) -> Vec<Option<UVec2>> {
    let mut sizes: Vec<UVec2> = RESOLUTIONS.iter().map(|&(w, h)| UVec2::new(w, h)).chain(crate::settings::display_modes()).filter(|r| Aspect::of(*r) == aspect && *r != native).collect();
    sizes.sort_by_key(|s| (s.x * s.y, s.x));
    sizes.dedup();
    (Aspect::of(native) == aspect).then_some(None).into_iter().chain(sizes.into_iter().map(Some)).collect()
}

/// Read-only things rows need besides the settings.
pub struct Ctx<'a> {
    /// The monitor's size (the window's when unknown).
    pub native: UVec2,
    /// The Video page's unapplied resolution and display mode.
    pub video: VideoMode,
    pub keys: &'a [KeyAction],
    /// The bindings in effect (the player's, else the game's defaults).
    pub binds: &'a BindTable,
    /// The key-list row waiting for a key.
    pub capture: Option<usize>,
    pub selected: Option<usize>,
    /// The Mods page's view of the mod store.
    pub mods: Option<&'a ModsPage>,
}

/// One line of a page.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A section title.
    Heading(String),
    /// `label   < value >`; clicking the value steps forward, wrapping.
    Choice { label: String, value: String, setting: Setting },
    /// A slider at `fraction` 0..1 with its value written next to it.
    Slider { label: String, value: String, fraction: f32, setting: Setting },
    /// Opens another page.
    Link { label: String, page: OptionsPage },
    /// A key-list row: the command's keys; `index` into the key list.
    Key { label: String, keys: String, index: usize, selected: bool },
    /// An installed mod, on or off; `index` into the Mods page's list.
    Mod { label: String, value: String, index: usize },
    /// Undo the last mod change (`value` says what it was).
    Rollback { value: String },
    /// A line of text; `error` shows it as a problem.
    Note { text: String, error: bool },
}

fn choice(label: &str, value: impl Into<String>, setting: Setting) -> Row {
    Row::Choice { label: label.into(), value: value.into(), setting }
}

fn on_off(on: bool) -> &'static str {
    if on {
        "Enabled"
    } else {
        "Disabled"
    }
}

fn level(v: u8) -> &'static str {
    match v {
        0 => "Low",
        1 => "Medium",
        2 => "High",
        _ => "Very High",
    }
}

/// The number a slider shows and moves: (min, max, step).
pub fn slider_range(setting: Setting) -> Option<(f32, f32, f32)> {
    Some(match setting {
        Setting::Master | Setting::Music | Setting::SubtitleBackground => (0.0, 1.0, 0.1),
        Setting::Sensitivity => (0.5, 20.0, 0.5),
        Setting::JoySensitivity => (0.2, 3.0, 0.2),
        Setting::Brightness => GAMMA_RANGE_BRIGHTNESS,
        Setting::Fov => (FOV_RANGE.0, FOV_RANGE.1, 5.0),
        _ => return None,
    })
}

/// Brightness runs the other way from gamma (brighter is a lower
/// `mat_monitorgamma`), so the slider shows `GAMMA_SUM - gamma`.
const GAMMA_SUM: f32 = GAMMA_RANGE.0 + GAMMA_RANGE.1;
const GAMMA_RANGE_BRIGHTNESS: (f32, f32, f32) = (GAMMA_RANGE.0, GAMMA_RANGE.1, 0.1);

pub fn slider_value(s: &Settings, setting: Setting) -> f32 {
    match setting {
        Setting::Master => s.master_volume,
        Setting::Music => s.music_volume,
        Setting::SubtitleBackground => s.subtitle_background,
        Setting::Sensitivity => s.mouse_sensitivity,
        Setting::JoySensitivity => s.joy_sensitivity,
        Setting::Brightness => GAMMA_SUM - s.gamma,
        Setting::Fov => s.fov,
        _ => 0.0,
    }
}

/// Set a slider, snapped to its step and range.
pub fn set_slider(s: &mut Settings, setting: Setting, v: f32) {
    let Some((lo, hi, step)) = slider_range(setting) else { return };
    let v = (lo + ((v - lo) / step).round() * step).clamp(lo, hi);
    // Keep 0.1 steps reading as 0.1, not 0.10000001.
    let v = (v * 1000.0).round() / 1000.0;
    match setting {
        Setting::Master => s.master_volume = v,
        Setting::Music => s.music_volume = v,
        Setting::SubtitleBackground => s.subtitle_background = v,
        Setting::Sensitivity => s.mouse_sensitivity = v,
        Setting::JoySensitivity => s.joy_sensitivity = v,
        Setting::Brightness => s.gamma = ((GAMMA_SUM - v) * 1000.0).round() / 1000.0,
        Setting::Fov => s.fov = v,
        _ => {}
    }
}

fn slider(s: &Settings, label: &str, setting: Setting) -> Row {
    let (lo, hi, _) = slider_range(setting).unwrap_or((0.0, 1.0, 0.1));
    let v = slider_value(s, setting);
    let value = match setting {
        Setting::Master | Setting::Music | Setting::SubtitleBackground => format!("{:.0}%", v * 100.0),
        Setting::Brightness => format!("{:.0}%", (v - lo) / (hi - lo) * 100.0),
        Setting::Fov => format!("{v:.0}"),
        _ => format!("{v:.1}"),
    };
    Row::Slider { label: label.into(), value, fraction: ((v - lo) / (hi - lo)).clamp(0.0, 1.0), setting }
}

pub fn caption_label(m: CaptionMode) -> &'static str {
    match m {
        CaptionMode::Off => "Off",
        CaptionMode::Subtitles => "Subtitles (dialogue only)",
        CaptionMode::ClosedCaptions => "Closed captions",
    }
}

pub fn speakers_label(s: Speakers) -> &'static str {
    match s {
        Speakers::Headphones => "Headphones",
        Speakers::Stereo => "2 Speakers",
        Speakers::Quad => "4 Speakers",
        Speakers::Surround51 => "5.1 Surround",
        Speakers::Surround71 => "7.1 Surround",
    }
}

/// `english` -> `English`; Steam's odd names spelled out.
pub fn language_label(l: &str) -> String {
    match l {
        "latam" => "Spanish (Latin America)".into(),
        "brazilian" => "Portuguese (Brazil)".into(),
        "koreana" => "Korean".into(),
        "schinese" => "Chinese (Simplified)".into(),
        _ => {
            let mut c = l.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
        }
    }
}

pub fn filtering_label(s: &Settings) -> String {
    match s.anisotropy {
        a if a > 1 => format!("Anisotropic {a}X"),
        _ if s.trilinear => "Trilinear".into(),
        _ => "Bilinear".into(),
    }
}

pub fn resolution_label(r: Option<UVec2>, native: UVec2) -> String {
    let r = r.unwrap_or(native);
    format!("{} x {}", r.x, r.y)
}

/// The rows of `page`.
pub fn rows(page: OptionsPage, s: &Settings, ctx: &Ctx) -> Vec<Row> {
    match page {
        OptionsPage::List => list_rows(),
        OptionsPage::KeyboardMouse => keyboard_mouse_rows(s),
        OptionsPage::Keys => key_rows(ctx),
        OptionsPage::Controller => controller_rows(s),
        OptionsPage::Audio => audio_rows(s),
        OptionsPage::Subtitles => subtitle_rows(s),
        OptionsPage::Video => video_rows(s, ctx),
        OptionsPage::AdvancedVideo => advanced_video_rows(s),
        OptionsPage::Speedrun => speedrun_rows(s),
        OptionsPage::Mods => mods_rows(ctx),
    }
}

fn list_rows() -> Vec<Row> {
    let link = |label: &str, page| Row::Link { label: label.into(), page };
    vec![
        // options.res (PC): Audio, Video, Keyboard/Mouse, Controller; our
        // own pages follow.
        link("AUDIO", OptionsPage::Audio),
        link("VIDEO", OptionsPage::Video),
        link("KEYBOARD / MOUSE", OptionsPage::KeyboardMouse),
        link("CONTROLLER", OptionsPage::Controller),
        link("SPEEDRUN", OptionsPage::Speedrun),
        link("MODS", OptionsPage::Mods),
    ]
}

/// `FULL RUN` -> `Full run`.
fn sentence(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

fn speedrun_rows(s: &Settings) -> Vec<Row> {
    let t = &s.tools;
    vec![
        choice("Speedrun timer", on_off(t.speedrun_timer), Setting::SpeedrunTimer),
        choice("Timer mode", sentence(t.speedrun_mode.label()), Setting::SpeedrunMode),
        choice("Quick restart", on_off(t.quick_restart), Setting::QuickRestart),
        choice("Quick restart key", key_name(t.quick_restart_key), Setting::QuickRestartKey),
        choice("Practice checkpoints", on_off(t.practice), Setting::Practice),
        choice("Save checkpoint key", key_name(t.checkpoint_save_key), Setting::SaveCheckpointKey),
        choice("Load checkpoint key", key_name(t.checkpoint_load_key), Setting::LoadCheckpointKey),
        choice("Checkpoint slot", t.checkpoint_slot.to_string(), Setting::CheckpointSlot),
        Row::Note {
            text: "Splits and personal bests are saved in the splits folder next to settings.cfg. Runs with sv_cheats or a loaded checkpoint never count as a personal best.".into(),
            error: false,
        },
    ]
}

fn mods_rows(ctx: &Ctx) -> Vec<Row> {
    let default = ModsPage::default();
    let m = ctx.mods.unwrap_or(&default);
    let profile = if m.profile.is_empty() { p2_mods::DEFAULT_PROFILE.to_string() } else { m.profile.clone() };
    let mut rows = vec![choice("Profile", profile, Setting::ModProfile)];
    if m.mods.is_empty() {
        rows.push(Row::Note { text: "No mods installed. Add them with the p2mods tool: p2mods install <folder or .zip>".into(), error: false });
    }
    for (index, r) in m.mods.iter().enumerate() {
        rows.push(Row::Mod { label: format!("{} {}", r.name, r.version), value: on_off(r.enabled).into(), index });
    }
    if let Some(u) = &m.undo {
        rows.push(Row::Rollback { value: u.clone() });
    }
    for (error, text) in &m.issues {
        rows.push(Row::Note { text: text.clone(), error: *error });
    }
    let mut note = m.note.clone();
    if m.restart_needed {
        note = format!("{note}  Restart the game to load the changes.").trim().to_string();
    }
    if !note.is_empty() {
        rows.push(Row::Note { text: note, error: false });
    }
    rows
}

fn keyboard_mouse_rows(s: &Settings) -> Vec<Row> {
    vec![
        Row::Link { label: "Customize keyboard".into(), page: OptionsPage::Keys },
        slider(s, "Mouse sensitivity", Setting::Sensitivity),
        choice("Reverse mouse", on_off(s.invert_mouse), Setting::Invert),
        choice("Mouse filter", on_off(s.mouse_filter), Setting::Smoothing),
        choice("Mouse acceleration", on_off(s.mouse_accel), Setting::Accel),
        choice("Raw mouse input", on_off(s.raw_input), Setting::RawInput),
        // Portal 2 has this under Keyboard's advanced settings.
        choice("Developer console (~)", on_off(s.tools.dev_console), Setting::DevConsole),
    ]
}

fn key_rows(ctx: &Ctx) -> Vec<Row> {
    ctx.keys
        .iter()
        .enumerate()
        .map(|(index, a)| match a {
            KeyAction::Heading(h) => Row::Heading(h.clone()),
            KeyAction::Command { command, label } => {
                let keys = if ctx.capture == Some(index) {
                    "PRESS A KEY".to_string()
                } else {
                    let k = ctx.binds.keys_for(command);
                    if k.is_empty() {
                        String::new()
                    } else {
                        k.iter().take(2).map(|k| key_label(k)).collect::<Vec<_>>().join(", ")
                    }
                };
                Row::Key { label: label.clone(), keys, index, selected: ctx.selected == Some(index) }
            }
        })
        .collect()
}

fn controller_rows(s: &Settings) -> Vec<Row> {
    vec![
        choice("Game controller", on_off(s.joystick), Setting::Joystick),
        slider(s, "Look sensitivity", Setting::JoySensitivity),
        choice("Reverse look", on_off(s.joy_invert), Setting::JoyInvert),
        choice("Vibration", on_off(s.joy_vibration), Setting::JoyVibration),
    ]
}

fn audio_rows(s: &Settings) -> Vec<Row> {
    vec![
        slider(s, "Game volume", Setting::Master),
        slider(s, "Music volume", Setting::Music),
        choice("Speaker configuration", speakers_label(s.speakers), Setting::Speakers),
        choice("Captioning", caption_label(s.captions), Setting::Captions),
        choice("Language", language_label(&s.language), Setting::Language),
        Row::Link { label: "Subtitle style".into(), page: OptionsPage::Subtitles },
    ]
}

fn subtitle_rows(s: &Settings) -> Vec<Row> {
    let size = SUBTITLE_SIZES.iter().find(|z| z.0 == s.subtitle_size).map_or(format!("{:.0}", s.subtitle_size), |z| language_label(&z.1.to_lowercase()));
    let color = format!("{:?}", s.subtitle_color);
    vec![
        choice("Text size", size, Setting::SubtitleSize),
        choice("Text colour", color, Setting::SubtitleColor),
        slider(s, "Background", Setting::SubtitleBackground),
    ]
}

fn video_rows(s: &Settings, ctx: &Ctx) -> Vec<Row> {
    let v = ctx.video;
    let size = v.resolution.unwrap_or(ctx.native);
    vec![
        choice("Resolution", resolution_label(v.resolution, ctx.native), Setting::Resolution),
        choice("Aspect ratio", Aspect::of(size).label(), Setting::Aspect),
        choice("Display mode", if v.fullscreen { "Fullscreen" } else { "Windowed" }, Setting::DisplayMode),
        slider(s, "Brightness", Setting::Brightness),
        Row::Link { label: "Advanced video".into(), page: OptionsPage::AdvancedVideo },
    ]
}

fn advanced_video_rows(s: &Settings) -> Vec<Row> {
    let frame_limit = if s.frame_limit == 0 { "Unlimited".into() } else { format!("{} FPS", s.frame_limit) };
    vec![
        choice("Model / texture detail", level(s.model_detail), Setting::ModelDetail),
        choice("Effect detail", level(s.effect_detail), Setting::EffectDetail),
        choice("Shader detail", level(s.shader_detail), Setting::ShaderDetail),
        choice("Paged pool memory available", level(s.paged_pool), Setting::PagedPool),
        choice("Multicore rendering", on_off(s.multicore), Setting::Multicore),
        choice("Anti-aliasing mode", if s.msaa { "4x MSAA" } else { "None" }, Setting::AntiAliasing),
        choice("Filtering mode", filtering_label(s), Setting::Filtering),
        choice("Wait for vertical sync", on_off(s.vsync), Setting::VSync),
        choice("Motion blur", on_off(s.motion_blur), Setting::MotionBlur),
        slider(s, "Field of view", Setting::Fov),
        choice("Max frame rate", frame_limit, Setting::FrameLimit),
        choice("Graphics", if s.enhanced_graphics { "Enhanced" } else { "Original" }, Setting::Graphics),
        choice("Show frame rate", on_off(s.tools.show_fps), Setting::ShowFps),
        choice("Show movement stats", on_off(s.tools.show_pos), Setting::ShowPos),
    ]
}

/// Step through `list` from `current`: clamped at the ends, or wrapping
/// round (a click on the value itself).
fn pick<T: Copy + PartialEq>(list: &[T], current: T, dir: i32, wrap: bool) -> T {
    match list.iter().position(|x| *x == current) {
        Some(i) if wrap => list[(i as i32 + dir).rem_euclid(list.len() as i32) as usize],
        Some(i) => list[(i as i32 + dir).clamp(0, list.len() as i32 - 1) as usize],
        None => list[0],
    }
}

/// The filtering modes in menu order: (trilinear, anisotropy).
fn filter_modes() -> Vec<(bool, u16)> {
    [(false, 1), (true, 1)].into_iter().chain(ANISOTROPY.iter().filter(|a| **a > 1).map(|a| (true, *a))).collect()
}

/// Change `setting` one step in `dir` (+1/-1); sliders move by `amount`
/// steps. Resolution, aspect and display mode change `video`, which the
/// caller applies on Apply/Done.
pub fn adjust(s: &mut Settings, video: &mut VideoMode, native: UVec2, setting: Setting, dir: i32, wrap: bool) {
    if let Some((_, _, step)) = slider_range(setting) {
        let v = slider_value(s, setting) + step * dir as f32;
        set_slider(s, setting, v);
        return;
    }
    let flip = |b: &mut bool| *b = !*b;
    match setting {
        Setting::Invert => flip(&mut s.invert_mouse),
        Setting::Smoothing => flip(&mut s.mouse_filter),
        Setting::Accel => flip(&mut s.mouse_accel),
        Setting::RawInput => flip(&mut s.raw_input),
        Setting::Joystick => flip(&mut s.joystick),
        Setting::JoyInvert => flip(&mut s.joy_invert),
        Setting::JoyVibration => flip(&mut s.joy_vibration),
        Setting::Multicore => flip(&mut s.multicore),
        Setting::AntiAliasing => flip(&mut s.msaa),
        Setting::VSync => flip(&mut s.vsync),
        Setting::MotionBlur => flip(&mut s.motion_blur),
        Setting::Graphics => flip(&mut s.enhanced_graphics),
        Setting::DevConsole => flip(&mut s.tools.dev_console),
        Setting::SpeedrunTimer => flip(&mut s.tools.speedrun_timer),
        Setting::QuickRestart => flip(&mut s.tools.quick_restart),
        Setting::Practice => flip(&mut s.tools.practice),
        Setting::SpeedrunMode => s.tools.speedrun_mode = pick(&SpeedrunMode::ALL, s.tools.speedrun_mode, dir, wrap),
        Setting::QuickRestartKey | Setting::SaveCheckpointKey | Setting::LoadCheckpointKey => {
            let t = &mut s.tools;
            let key = match setting {
                Setting::QuickRestartKey => &mut t.quick_restart_key,
                Setting::SaveCheckpointKey => &mut t.checkpoint_save_key,
                _ => &mut t.checkpoint_load_key,
            };
            *key = pick(&BIND_KEYS.map(|k| k.0), *key, dir, wrap);
        }
        Setting::CheckpointSlot => s.tools.checkpoint_slot = pick(&(1..=CHECKPOINT_SLOTS).collect::<Vec<_>>(), s.tools.checkpoint_slot, dir, wrap),
        Setting::ShowFps => flip(&mut s.tools.show_fps),
        Setting::ShowPos => flip(&mut s.tools.show_pos),
        Setting::DisplayMode => flip(&mut video.fullscreen),
        Setting::Speakers => s.speakers = pick(&Speakers::ALL, s.speakers, dir, wrap),
        Setting::Captions => s.captions = pick(&CaptionMode::ALL, s.captions, dir, wrap),
        Setting::Language => {
            let i = LANGUAGES.iter().position(|l| *l == s.language).unwrap_or(0);
            s.language = LANGUAGES[pick(&(0..LANGUAGES.len()).collect::<Vec<_>>(), i, dir, wrap)].to_string();
        }
        Setting::SubtitleSize => s.subtitle_size = pick(&SUBTITLE_SIZES.map(|z| z.0), s.subtitle_size, dir, wrap),
        Setting::SubtitleColor => s.subtitle_color = pick(&SubtitleColor::ALL, s.subtitle_color, dir, wrap),
        Setting::ModelDetail => s.model_detail = pick(&[0, 1, 2], s.model_detail, dir, wrap),
        Setting::EffectDetail => s.effect_detail = pick(&[0, 1, 2], s.effect_detail, dir, wrap),
        Setting::ShaderDetail => s.shader_detail = pick(&[0, 1, 2, 3], s.shader_detail, dir, wrap),
        Setting::PagedPool => s.paged_pool = pick(&[0, 1, 2], s.paged_pool, dir, wrap),
        Setting::Filtering => {
            let modes = filter_modes();
            let now = (s.trilinear || s.anisotropy > 1, s.anisotropy);
            (s.trilinear, s.anisotropy) = pick(&modes, now, dir, wrap);
        }
        Setting::FrameLimit if wrap => s.frame_limit = pick(&FRAME_LIMITS, s.frame_limit, dir, true),
        Setting::FrameLimit => s.frame_limit = step_in(&FRAME_LIMITS, s.frame_limit, dir, frame_limit_rank),
        Setting::Resolution => {
            let list = resolutions_for(Aspect::of(video.resolution.unwrap_or(native)), native);
            video.resolution = pick(&list, video.resolution, dir, wrap);
        }
        Setting::Aspect => {
            let next = pick(&Aspect::ALL, Aspect::of(video.resolution.unwrap_or(native)), dir, wrap);
            // The largest size of that aspect that fits the screen, else the smallest.
            let list = resolutions_for(next, native);
            let fits = |r: &Option<UVec2>| r.is_none_or(|r| r.x <= native.x && r.y <= native.y);
            video.resolution = list.iter().rev().find(|r| fits(r)).or(list.first()).copied().unwrap_or(None);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE: UVec2 = UVec2::new(2560, 1600);

    fn ctx<'a>(keys: &'a [KeyAction], binds: &'a BindTable, video: VideoMode) -> Ctx<'a> {
        Ctx { native: NATIVE, video, keys, binds, capture: None, selected: None, mods: None }
    }

    #[test]
    fn every_page_has_rows_and_a_way_out() {
        let s = Settings::default();
        let keys = crate::bindings::builtin_actions();
        let binds = BindTable::builtin();
        let c = ctx(&keys, &binds, VideoMode::of(&s));
        for page in [
            OptionsPage::List,
            OptionsPage::KeyboardMouse,
            OptionsPage::Keys,
            OptionsPage::Controller,
            OptionsPage::Audio,
            OptionsPage::Subtitles,
            OptionsPage::Video,
            OptionsPage::AdvancedVideo,
            OptionsPage::Speedrun,
            OptionsPage::Mods,
        ] {
            assert!(!rows(page, &s, &c).is_empty(), "{page:?}");
            assert!(page.footer().iter().any(|f| matches!(f, Footer::Back | Footer::Done)), "{page:?}");
            // Every link leads to a page whose parent is this one.
            for r in rows(page, &s, &c) {
                if let Row::Link { page: to, .. } = r {
                    assert_eq!(to.parent(), Some(page));
                }
            }
        }
    }

    #[test]
    fn speedrun_page_steps_like_before() {
        // The values the old Speedrun page's clicks gave.
        let mut s = Settings::default();
        let mut v = VideoMode::of(&s);
        for (setting, dir) in [
            (Setting::SpeedrunTimer, 1),
            (Setting::SpeedrunMode, 1),
            (Setting::QuickRestart, 1),
            (Setting::QuickRestartKey, 1),
            (Setting::Practice, 1),
            (Setting::SaveCheckpointKey, -1),
            (Setting::LoadCheckpointKey, 1),
            (Setting::CheckpointSlot, 1),
        ] {
            adjust(&mut s, &mut v, NATIVE, setting, dir, false);
        }
        let t = &s.tools;
        assert!(t.speedrun_timer && t.quick_restart && t.practice);
        assert_eq!(t.speedrun_mode, SpeedrunMode::SingleMap);
        use bevy::input::keyboard::KeyCode;
        assert_eq!((t.quick_restart_key, t.checkpoint_save_key, t.checkpoint_load_key, t.checkpoint_slot), (KeyCode::F7, KeyCode::F6, KeyCode::F9, 2));
        let keys = crate::bindings::builtin_actions();
        let binds = BindTable::builtin();
        let c = ctx(&keys, &binds, v);
        assert!(rows(OptionsPage::Speedrun, &s, &c).contains(&choice("Timer mode", "Single map", Setting::SpeedrunMode)));
        // The mode wraps when its value is clicked.
        adjust(&mut s, &mut v, NATIVE, Setting::SpeedrunMode, 1, true);
        assert_eq!(s.tools.speedrun_mode, SpeedrunMode::FullRun);
    }

    #[test]
    fn mods_page_rows() {
        let s = Settings::default();
        let keys = crate::bindings::builtin_actions();
        let binds = BindTable::builtin();
        let mut page = ModsPage { profile: "speedrun".into(), ..Default::default() };
        page.mods.push(crate::mods_page::ModRow { id: "cubes".into(), name: "Better Cubes".into(), version: "1.0.0".into(), enabled: true });
        page.undo = Some("enable cubes 1.0.0".into());
        page.issues.push((true, "cubes: needs 2.0".into()));
        page.restart_needed = true;
        let mut c = ctx(&keys, &binds, VideoMode::of(&s));
        c.mods = Some(&page);
        let r = rows(OptionsPage::Mods, &s, &c);
        assert_eq!(r[0], choice("Profile", "speedrun", Setting::ModProfile));
        assert_eq!(r[1], Row::Mod { label: "Better Cubes 1.0.0".into(), value: "Enabled".into(), index: 0 });
        assert_eq!(r[2], Row::Rollback { value: "enable cubes 1.0.0".into() });
        assert_eq!(r[3], Row::Note { text: "cubes: needs 2.0".into(), error: true });
        assert_eq!(r[4], Row::Note { text: "Restart the game to load the changes.".into(), error: false });
        // No mod store: the default profile and how to add mods.
        c.mods = None;
        let r = rows(OptionsPage::Mods, &s, &c);
        assert_eq!(r[0], choice("Profile", p2_mods::DEFAULT_PROFILE, Setting::ModProfile));
        assert!(matches!(&r[1], Row::Note { text, .. } if text.contains("p2mods install")));
    }

    #[test]
    fn default_labels_are_portal2s() {
        let s = Settings::default();
        let keys = crate::bindings::builtin_actions();
        let binds = BindTable::builtin();
        let c = ctx(&keys, &binds, VideoMode::of(&s));
        let value = |page, label: &str| {
            rows(page, &s, &c).into_iter().find_map(|r| match r {
                Row::Choice { label: l, value, .. } | Row::Slider { label: l, value, .. } if l == label => Some(value),
                _ => None,
            })
        };
        assert_eq!(value(OptionsPage::Audio, "Captioning").as_deref(), Some("Subtitles (dialogue only)"));
        assert_eq!(value(OptionsPage::Audio, "Game volume").as_deref(), Some("100%"));
        assert_eq!(value(OptionsPage::Video, "Display mode").as_deref(), Some("Windowed"));
        assert_eq!(value(OptionsPage::Video, "Resolution").as_deref(), Some("2560 x 1600"));
        assert_eq!(value(OptionsPage::Video, "Aspect ratio").as_deref(), Some("Widescreen (16:10)"));
        assert_eq!(value(OptionsPage::Video, "Brightness").as_deref(), Some("40%"));
        assert_eq!(value(OptionsPage::AdvancedVideo, "Field of view").as_deref(), Some("75"));
        assert_eq!(value(OptionsPage::AdvancedVideo, "Max frame rate").as_deref(), Some("Unlimited"));
        assert_eq!(value(OptionsPage::KeyboardMouse, "Reverse mouse").as_deref(), Some("Disabled"));
        let jump = rows(OptionsPage::Keys, &s, &c).into_iter().find_map(|r| match r {
            Row::Key { label, keys, .. } if label == "Jump" => Some(keys),
            _ => None,
        });
        assert_eq!(jump.as_deref(), Some("SPACE"));
    }

    #[test]
    fn sliders_snap_and_clamp() {
        let mut s = Settings::default();
        let mut v = VideoMode::of(&s);
        adjust(&mut s, &mut v, NATIVE, Setting::Master, -1, false);
        assert_eq!(s.master_volume, 0.9);
        adjust(&mut s, &mut v, NATIVE, Setting::Fov, 1, false);
        assert_eq!(s.fov, 80.0);
        set_slider(&mut s, Setting::Fov, 500.0);
        assert_eq!(s.fov, FOV_RANGE.1);
        // Brighter is a lower gamma.
        adjust(&mut s, &mut v, NATIVE, Setting::Brightness, 1, false);
        assert_eq!(s.gamma, 2.1);
        set_slider(&mut s, Setting::Brightness, 0.0);
        assert_eq!(s.gamma, GAMMA_RANGE.1);
    }

    #[test]
    fn choices_clamp_or_wrap() {
        let mut s = Settings::default();
        let mut v = VideoMode::of(&s);
        adjust(&mut s, &mut v, NATIVE, Setting::Captions, -1, false);
        assert_eq!(s.captions, CaptionMode::Off);
        adjust(&mut s, &mut v, NATIVE, Setting::Captions, -1, true);
        assert_eq!(s.captions, CaptionMode::ClosedCaptions);
        adjust(&mut s, &mut v, NATIVE, Setting::ShaderDetail, 1, false);
        assert_eq!(s.shader_detail, 3);
        // Filtering: 16x is last; one down is 8x, and two below 2x is trilinear.
        adjust(&mut s, &mut v, NATIVE, Setting::Filtering, -1, false);
        assert_eq!((s.trilinear, s.anisotropy), (true, 8));
        for _ in 0..3 {
            adjust(&mut s, &mut v, NATIVE, Setting::Filtering, -1, false);
        }
        assert_eq!(filtering_label(&s), "Trilinear");
        adjust(&mut s, &mut v, NATIVE, Setting::Filtering, -1, false);
        assert_eq!(filtering_label(&s), "Bilinear");
        adjust(&mut s, &mut v, NATIVE, Setting::Language, 1, false);
        assert_eq!(s.language, "french");
    }

    #[test]
    fn video_changes_wait_for_apply() {
        let mut s = Settings::default();
        let mut v = VideoMode::of(&s);
        adjust(&mut s, &mut v, NATIVE, Setting::DisplayMode, 1, false);
        adjust(&mut s, &mut v, NATIVE, Setting::Resolution, 1, false);
        assert!(!s.fullscreen && s.resolution.is_none());
        assert_eq!(v.resolution, Some(UVec2::new(1280, 800)));
        // Aspect: 16:9 picks the largest 16:9 size that fits.
        adjust(&mut s, &mut v, NATIVE, Setting::Aspect, -1, false);
        assert_eq!(v.resolution, Some(UVec2::new(2560, 1440)));
        adjust(&mut s, &mut v, NATIVE, Setting::Aspect, -1, false);
        assert_eq!(v.resolution, Some(UVec2::new(1600, 1200)));
        v.apply(&mut s);
        assert_eq!((s.fullscreen, s.resolution), (true, Some(UVec2::new(1600, 1200))));
    }

    #[test]
    fn aspect_of_screens() {
        assert_eq!(Aspect::of(UVec2::new(1920, 1080)), Aspect::Wide16x9);
        assert_eq!(Aspect::of(UVec2::new(3024, 1964)), Aspect::Wide16x10);
        assert_eq!(Aspect::of(UVec2::new(1024, 768)), Aspect::Normal);
        let r = resolutions_for(Aspect::Wide16x10, NATIVE);
        assert_eq!(r[0], None);
        assert!(!r.contains(&Some(NATIVE)));
        assert_eq!(resolutions_for(Aspect::Normal, NATIVE)[0], Some(UVec2::new(1024, 768)));
    }
}
