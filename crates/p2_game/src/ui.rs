//! Menus and HUD text: the main menu over a slowly turning view of a test
//! chamber with the game's menu music, chapter select with the game's own
//! chapter pictures and titles, options, the pause menu, chapter title cards
//! and the autosave that "Continue" picks up.
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use p2_assets::GameFsRes;
use p2_core::*;

use crate::audio::{GameSound, PlayGameSound};
use crate::campaign::Campaign;
use crate::level::{CurrentMap, LevelLoaded, LoadLevel};
use crate::settings::{data_dir, frame_limit_rank, resolution_choices, step_in, Settings, SubtitleColor, ANISOTROPY, FOV_RANGE, FRAME_LIMITS, SUBTITLE_SIZES};

pub struct UiPlugin;

/// Scripted tests press a menu button by its action (`Options`,
/// `Toggle(VSync)`, `Adjust(Fov,5.0)`, spaces ignored), or `pause`.
#[derive(Message, Clone, Debug)]
pub struct MenuClick(pub String);

/// Show the current chapter's title card (the map's `DisplayChapterTitle()`).
#[derive(Message, Clone, Copy, Debug)]
pub struct ShowChapterTitle;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Screen::Main)
            .init_resource::<OptionsPage>()
            .init_resource::<TitleCard>()
            .add_message::<ShowChapterTitle>()
            .add_message::<MenuClick>()
            .add_systems(Startup, (load_fonts, open_main_menu).chain().after(crate::campaign::load_campaign))
            .add_systems(
                Update,
                (escape_key, scripted_clicks, buttons, rebuild_menu, menu_view, menu_camera, pause_world, menu_music, autosave, title_card).chain().in_set(crate::level::GameSet::Logic),
            );
    }
}

/// Which menu is up. `None` while playing.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    None,
    Main,
    Chapters,
    Options { from_pause: bool },
    Pause,
}

/// Which part of Options is up: the main list, or the Subtitles section
/// clicked into from it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OptionsPage {
    #[default]
    Main,
    Subtitles,
}

#[derive(Resource, Clone)]
pub struct UiFonts {
    pub bold: Handle<Font>,
    pub regular: Handle<Font>,
}

/// The chapter title card shown when a chapter starts.
#[derive(Resource, Default)]
struct TitleCard {
    lines: Option<(String, String, String)>,
    shown_at: f32,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct TitleCardRoot;

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Continue,
    NewGame,
    Chapters,
    Chapter(usize),
    Options,
    SubtitleOptions,
    Back,
    Quit,
    Resume,
    Restart,
    MainMenu,
    Adjust(Setting, f32),
    Toggle(Setting),
}

#[derive(Clone, Copy, Debug)]
enum Setting {
    Master,
    Music,
    Effects,
    Sensitivity,
    Fov,
    Invert,
    Smoothing,
    Fullscreen,
    Resolution,
    VSync,
    FrameLimit,
    AntiAliasing,
    Anisotropy,
    Graphics,
    Subtitles,
    SubtitleSize,
    SubtitleColor,
    SubtitleBackground,
}

/// The control at the right of an options row.
enum Ctl {
    /// `<` value `>`; the step is the change per click (its sign for list settings).
    Step(Setting, f32),
    /// The value is the button.
    Press(Action),
}

const WHITE: Color = Color::srgb(0.92, 0.94, 0.95);
const DIM: Color = Color::srgba(0.75, 0.78, 0.8, 0.85);
const HOVER: Color = Color::srgb(1.0, 1.0, 1.0);
const PANEL: Color = Color::srgba(0.02, 0.03, 0.04, 0.72);

fn load_fonts(mut commands: Commands, game: Res<GameFsRes>, mut fonts: ResMut<Assets<Font>>) {
    let load = |fonts: &mut Assets<Font>, paths: &[&str]| -> Handle<Font> {
        paths.iter().find_map(|p| game.read(p)).map(|b| fonts.add(Font::from_bytes(b))).unwrap_or_default()
    };
    let bold = load(&mut fonts, &["resource/linux_fonts/dejavusans-bold.ttf", "resource/linux_fonts/liberationsans-bold.ttf"]);
    let regular = load(&mut fonts, &["resource/linux_fonts/dejavusans.ttf", "resource/linux_fonts/liberationsans-regular.ttf"]);
    commands.insert_resource(UiFonts { bold, regular });
}

/// The map the menu shows behind it: the chapter of the save, else chapter 2's chamber.
fn menu_background(campaign: &Campaign) -> String {
    let saved = read_save();
    let chapter = saved.as_deref().and_then(|m| campaign.chapter_of(m)).unwrap_or(1);
    let pick = campaign.chapters.get(chapter.max(1)).or(campaign.chapters.first()).map(|c| c.map.clone());
    pick.unwrap_or_else(|| "sp_a2_laser_intro".into())
}

fn open_main_menu(campaign: Res<Campaign>, mut load: MessageWriter<LoadLevel>, args: Res<crate::Launch>) {
    match &args.map {
        Some(m) => load.write(LoadLevel { map: m.clone(), menu: false }),
        None => load.write(LoadLevel { map: menu_background(&campaign), menu: true }),
    };
}

/// Written as the autosave's `version` line. 1 is the old file: just the map name.
const SAVE_VERSION: u32 = 2;

fn read_save() -> Option<String> {
    std::fs::read_to_string(data_dir().join("save.txt")).ok().and_then(|s| parse_save(&s))
}

/// The map in an autosave: `key value` lines (`map` is the one we use,
/// unknown keys are skipped), or an old file holding only the map name.
fn parse_save(text: &str) -> Option<String> {
    let lines = || text.lines().map(str::trim).filter(|l| !l.is_empty());
    let map = lines().find_map(|l| l.split_once(char::is_whitespace).filter(|(k, _)| *k == "map").map(|(_, v)| v.trim().to_string()));
    let old = || lines().next().filter(|l| !l.contains(char::is_whitespace)).map(str::to_string);
    map.or_else(old).filter(|m| !m.is_empty())
}

fn save_text(map: &str) -> String {
    format!("version {SAVE_VERSION}\nmap {map}\n")
}

fn text(font: &Handle<Font>, size: f32, s: impl Into<String>, color: Color) -> impl Bundle {
    (Text::new(s), TextFont { font: font.into(), font_size: bevy::text::FontSize::Px(size), ..default() }, TextColor(color))
}

fn button(fonts: &UiFonts, label: impl Into<String>, action: Action, size: f32) -> impl Bundle {
    (
        Button,
        action,
        Node { padding: UiRect::axes(Val::Px(4.0), Val::Px(6.0)), ..default() },
        BackgroundColor(Color::NONE),
        children![text(&fonts.bold, size, label, DIM)],
    )
}

/// One options row: label, then its control.
fn option_row(p: &mut ChildSpawnerCommands, fonts: &UiFonts, label: &str, value: String, ctl: Ctl) {
    p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), ..default() })
        .with_children(|r| {
            r.spawn((Node { width: Val::Px(280.0), ..default() }, children![text(&fonts.regular, 20.0, label, DIM)]));
            match ctl {
                Ctl::Step(setting, s) => {
                    r.spawn(button(fonts, "<", Action::Adjust(setting, -s), 22.0));
                    r.spawn((Node { width: Val::Px(130.0), justify_content: JustifyContent::Center, ..default() }, children![text(&fonts.bold, 20.0, value, WHITE)]));
                    r.spawn(button(fonts, ">", Action::Adjust(setting, s), 22.0));
                }
                Ctl::Press(action) => {
                    r.spawn(button(fonts, value, action, 20.0));
                }
            }
        });
}

fn heading(p: &mut ChildSpawnerCommands, fonts: &UiFonts, label: &str) {
    p.spawn((Node { margin: UiRect::top(Val::Px(10.0)), ..default() }, children![text(&fonts.bold, 16.0, label, WHITE)]));
}

fn on_off(on: bool) -> String {
    if on { "ON".into() } else { "OFF".into() }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    page: Res<OptionsPage>,
    fonts: Option<Res<UiFonts>>,
    campaign: Res<Campaign>,
    settings: Res<Settings>,
    roots: Query<Entity, With<MenuRoot>>,
    game: Res<GameFsRes>,
    mut images: ResMut<Assets<Image>>,
    mut chapter_pics: Local<Vec<Option<Handle<Image>>>>,
    mut backgrounds: Local<std::collections::HashMap<usize, Option<Handle<Image>>>>,
) {
    let Some(fonts) = fonts else { return };
    let options_changed = (settings.is_changed() || page.is_changed()) && matches!(*screen, Screen::Options { .. });
    if !screen.is_changed() && !options_changed && !fonts.is_added() {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    let full = Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        padding: UiRect::left(Val::Percent(8.0)),
        row_gap: Val::Px(6.0),
        ..default()
    };
    // Out of the game, the menu sits on the game's own menu still for the
    // act of the saved chapter.
    if matches!(*screen, Screen::Main | Screen::Chapters | Screen::Options { from_pause: false }) {
        let chapter = read_save().and_then(|m| campaign.chapter_of(&m)).unwrap_or(0);
        let act = [1, 2, 2, 2, 3, 4, 4, 5, 5].get(chapter).copied().unwrap_or(1);
        let img = backgrounds
            .entry(act)
            .or_insert_with(|| {
                let data = game.read(&format!("materials/vgui/backgrounds/background{act:02}_widescreen.vtf"))?;
                let vtf = source_assets::vtf::VtfTexture::parse(&data).ok()?;
                Some(images.add(vtf.into_image(false, true)))
            })
            .clone();
        let mut bg = commands.spawn((
            MenuRoot,
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(9),
        ));
        if let Some(img) = img {
            bg.with_child((
                ImageNode { image: img, image_mode: bevy::ui::widget::NodeImageMode::Stretch, ..default() },
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            ));
        }
    }
    match *screen {
        Screen::None => {}
        Screen::Main => {
            let has_save = read_save().is_some();
            let mut root = commands.spawn((MenuRoot, full, GlobalZIndex(10)));
            root.with_children(|p| {
                p.spawn(text(&fonts.bold, 72.0, "PORTAL 2", WHITE));
                p.spawn((Node { height: Val::Px(36.0), ..default() },));
                if has_save {
                    p.spawn(button(&fonts, "CONTINUE GAME", Action::Continue, 26.0));
                }
                p.spawn(button(&fonts, "NEW GAME", Action::NewGame, 26.0));
                p.spawn(button(&fonts, "CHAPTER SELECT", Action::Chapters, 26.0));
                p.spawn(button(&fonts, "OPTIONS", Action::Options, 26.0));
                p.spawn(button(&fonts, "QUIT", Action::Quit, 26.0));
            });
        }
        Screen::Chapters => {
            if chapter_pics.len() != campaign.chapters.len() {
                *chapter_pics = campaign
                    .chapters
                    .iter()
                    .map(|c| {
                        let data = game.read(&format!("materials/{}.vtf", c.picture))?;
                        let vtf = source_assets::vtf::VtfTexture::parse(&data).ok()?;
                        Some(images.add(vtf.into_image(false, true)))
                    })
                    .collect();
            }
            let mut root = commands.spawn((MenuRoot, full, BackgroundColor(PANEL), GlobalZIndex(10)));
            root.with_children(|p| {
                p.spawn(text(&fonts.bold, 40.0, "CHAPTER SELECT", WHITE));
                p.spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                    column_gap: Val::Px(18.0),
                    row_gap: Val::Px(14.0),
                    width: Val::Percent(84.0),
                    margin: UiRect::vertical(Val::Px(18.0)),
                    ..default()
                })
                .with_children(|g| {
                    for (i, c) in campaign.chapters.iter().enumerate() {
                        g.spawn((
                            Button,
                            Action::Chapter(i),
                            Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(6.0)), ..default() },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
                        ))
                        .with_children(|b| {
                            if let Some(Some(img)) = chapter_pics.get(i) {
                                b.spawn((ImageNode::new(img.clone()), Node { width: Val::Percent(100.0), aspect_ratio: Some(16.0 / 9.0), ..default() }));
                            }
                            b.spawn(text(&fonts.regular, 15.0, c.title.to_uppercase(), DIM));
                            b.spawn(text(&fonts.bold, 18.0, c.subtitle.to_uppercase(), WHITE));
                        });
                    }
                });
                p.spawn(button(&fonts, "BACK", Action::Back, 22.0));
            });
        }
        Screen::Options { .. } if *page == OptionsPage::Subtitles => {
            let size = SUBTITLE_SIZES.iter().find(|z| z.0 == settings.subtitle_size).map_or(format!("{:.0}", settings.subtitle_size), |z| z.1.to_string());
            let color = format!("{:?}", settings.subtitle_color).to_uppercase();
            let mut root = commands.spawn((MenuRoot, full, BackgroundColor(PANEL), GlobalZIndex(10)));
            root.with_children(|p| {
                p.spawn(text(&fonts.bold, 40.0, "SUBTITLES", WHITE));
                p.spawn(Node { height: Val::Px(16.0), ..default() });
                option_row(p, &fonts, "SUBTITLES", on_off(settings.subtitles), Ctl::Press(Action::Toggle(Setting::Subtitles)));
                option_row(p, &fonts, "TEXT SIZE", size, Ctl::Step(Setting::SubtitleSize, 1.0));
                option_row(p, &fonts, "TEXT COLOUR", color, Ctl::Step(Setting::SubtitleColor, 1.0));
                option_row(p, &fonts, "BACKGROUND", format!("{:.0}%", settings.subtitle_background * 100.0), Ctl::Step(Setting::SubtitleBackground, 0.1));
                // A sample caption in the chosen style.
                p.spawn((
                    Node { margin: UiRect::vertical(Val::Px(18.0)), padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)), align_self: AlignSelf::FlexStart, ..default() },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, settings.subtitle_background)),
                    children![text(&fonts.regular, settings.subtitle_size, "Subtitles will look like this.", settings.subtitle_color.color().unwrap_or(WHITE))],
                ));
                p.spawn(button(&fonts, "BACK", Action::Back, 22.0));
            });
        }
        Screen::Options { .. } => {
            let pct = |v: f32| format!("{:.0}%", v * 100.0);
            let resolution = match settings.resolution {
                Some(r) => format!("{} x {}", r.x, r.y),
                // In a window, native is the size the game opened at.
                None if settings.fullscreen => "NATIVE".into(),
                None => "DEFAULT".into(),
            };
            let frame_limit = if settings.frame_limit == 0 { "UNLIMITED".into() } else { settings.frame_limit.to_string() };
            let aniso = if settings.anisotropy <= 1 { "OFF".into() } else { format!("{}X", settings.anisotropy) };
            let column = Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() };
            let mut root = commands.spawn((MenuRoot, full, BackgroundColor(PANEL), GlobalZIndex(10)));
            root.with_children(|p| {
                p.spawn(text(&fonts.bold, 40.0, "OPTIONS", WHITE));
                p.spawn(Node { height: Val::Px(6.0), ..default() });
                p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(48.0), ..default() }).with_children(|cols| {
                    cols.spawn(column.clone()).with_children(|c| {
                        heading(c, &fonts, "AUDIO");
                        option_row(c, &fonts, "MASTER VOLUME", pct(settings.master_volume), Ctl::Step(Setting::Master, 0.1));
                        option_row(c, &fonts, "MUSIC VOLUME", pct(settings.music_volume), Ctl::Step(Setting::Music, 0.1));
                        option_row(c, &fonts, "EFFECTS VOLUME", pct(settings.effects_volume), Ctl::Step(Setting::Effects, 0.1));
                        option_row(c, &fonts, "SUBTITLES", format!("{}  >", on_off(settings.subtitles)), Ctl::Press(Action::SubtitleOptions));
                        heading(c, &fonts, "MOUSE");
                        option_row(c, &fonts, "MOUSE SENSITIVITY", format!("{:.1}", settings.mouse_sensitivity), Ctl::Step(Setting::Sensitivity, 0.5));
                        option_row(c, &fonts, "INVERT MOUSE", on_off(settings.invert_mouse), Ctl::Press(Action::Toggle(Setting::Invert)));
                        option_row(c, &fonts, "MOUSE SMOOTHING", on_off(settings.mouse_filter), Ctl::Press(Action::Toggle(Setting::Smoothing)));
                    });
                    cols.spawn(column).with_children(|c| {
                        heading(c, &fonts, "VIDEO");
                        option_row(c, &fonts, "FIELD OF VIEW", format!("{:.0}", settings.fov), Ctl::Step(Setting::Fov, 5.0));
                        option_row(c, &fonts, "FULLSCREEN", on_off(settings.fullscreen), Ctl::Press(Action::Toggle(Setting::Fullscreen)));
                        option_row(c, &fonts, "RESOLUTION", resolution, Ctl::Step(Setting::Resolution, 1.0));
                        option_row(c, &fonts, "VERTICAL SYNC", on_off(settings.vsync), Ctl::Press(Action::Toggle(Setting::VSync)));
                        option_row(c, &fonts, "MAX FRAME RATE", frame_limit, Ctl::Step(Setting::FrameLimit, 1.0));
                        option_row(c, &fonts, "ANTI-ALIASING", if settings.msaa { "MSAA 4X".into() } else { "OFF".into() }, Ctl::Press(Action::Toggle(Setting::AntiAliasing)));
                        option_row(c, &fonts, "TEXTURE FILTERING", aniso, Ctl::Step(Setting::Anisotropy, 1.0));
                        option_row(c, &fonts, "GRAPHICS", if settings.enhanced_graphics { "ENHANCED".into() } else { "ORIGINAL".into() }, Ctl::Press(Action::Toggle(Setting::Graphics)));
                    });
                });
                p.spawn(Node { height: Val::Px(16.0), ..default() });
                p.spawn(button(&fonts, "BACK", Action::Back, 22.0));
            });
        }
        Screen::Pause => {
            let mut root = commands.spawn((MenuRoot, full, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)), GlobalZIndex(10)));
            root.with_children(|p| {
                p.spawn(text(&fonts.bold, 56.0, "PAUSED", WHITE));
                p.spawn(Node { height: Val::Px(24.0), ..default() });
                p.spawn(button(&fonts, "RESUME GAME", Action::Resume, 26.0));
                p.spawn(button(&fonts, "RESTART LEVEL", Action::Restart, 26.0));
                p.spawn(button(&fonts, "OPTIONS", Action::Options, 26.0));
                p.spawn(button(&fonts, "QUIT TO MAIN MENU", Action::MainMenu, 26.0));
                p.spawn(button(&fonts, "QUIT GAME", Action::Quit, 26.0));
            });
        }
    }
}

/// Esc pauses and resumes, and backs out of sub-menus.
fn escape_key(keys: Res<ButtonInput<KeyCode>>, mut screen: ResMut<Screen>, mut page: ResMut<OptionsPage>, map: Option<Res<CurrentMap>>, settings: Res<Settings>) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // Leaving Options any way keeps what was changed.
    if matches!(*screen, Screen::Options { .. }) {
        settings.save();
        if *page == OptionsPage::Subtitles {
            *page = OptionsPage::Main;
            return;
        }
    }
    let in_game = map.as_ref().is_some_and(|m| !m.menu);
    *screen = match *screen {
        Screen::None if in_game => Screen::Pause,
        Screen::Pause => Screen::None,
        Screen::Options { from_pause: true } => Screen::Pause,
        Screen::Options { from_pause: false } | Screen::Chapters => Screen::Main,
        s => s,
    };
}

fn scripted_clicks(mut clicks: MessageReader<MenuClick>, mut screen: ResMut<Screen>, mut q: Query<(&mut Interaction, &Action)>) {
    for c in clicks.read() {
        let want = c.0.replace(' ', "").to_ascii_lowercase();
        if want == "pause" {
            *screen = Screen::Pause;
            continue;
        }
        match q.iter_mut().find(|(_, a)| format!("{a:?}").replace(' ', "").to_ascii_lowercase() == want) {
            Some((mut i, _)) => *i = Interaction::Pressed,
            None => warn!("portal2: no menu button {:?}; have {:?}", c.0, q.iter().map(|(_, a)| format!("{a:?}")).collect::<Vec<_>>()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn buttons(
    mut q: Query<(&Interaction, &Action, &Children), Changed<Interaction>>,
    mut texts: Query<&mut TextColor>,
    mut screen: ResMut<Screen>,
    mut page: ResMut<OptionsPage>,
    mut settings: ResMut<Settings>,
    campaign: Res<Campaign>,
    map: Option<Res<CurrentMap>>,
    mut load: MessageWriter<LoadLevel>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action, children) in &mut q {
        let color = match interaction {
            Interaction::Hovered | Interaction::Pressed => HOVER,
            Interaction::None => DIM,
        };
        for c in children.iter() {
            if let Ok(mut t) = texts.get_mut(c) {
                if t.0 != WHITE || matches!(interaction, Interaction::None) {
                    t.0 = color;
                }
            }
        }
        if *interaction != Interaction::Pressed {
            continue;
        }
        info!("portal2: menu {:?}", action);
        match *action {
            Action::Continue => {
                if let Some(m) = read_save() {
                    load.write(LoadLevel { map: m, menu: false });
                    *screen = Screen::None;
                }
            }
            Action::NewGame => {
                if let Some(first) = campaign.maps.first() {
                    load.write(LoadLevel { map: first.clone(), menu: false });
                    *screen = Screen::None;
                }
            }
            Action::Chapters => *screen = Screen::Chapters,
            Action::Chapter(i) => {
                if let Some(c) = campaign.chapters.get(i) {
                    load.write(LoadLevel { map: c.map.clone(), menu: false });
                    *screen = Screen::None;
                }
            }
            Action::Options => {
                *page = OptionsPage::Main;
                *screen = Screen::Options { from_pause: *screen == Screen::Pause };
            }
            Action::SubtitleOptions => *page = OptionsPage::Subtitles,
            Action::Back if *page == OptionsPage::Subtitles && matches!(*screen, Screen::Options { .. }) => {
                *page = OptionsPage::Main;
                settings.save();
            }
            Action::Back => {
                *screen = match *screen {
                    Screen::Options { from_pause: true } => Screen::Pause,
                    _ => Screen::Main,
                };
                settings.save();
            }
            Action::Quit => {
                settings.save();
                exit.write(AppExit::Success);
            }
            Action::Resume => *screen = Screen::None,
            Action::Restart => {
                if let Some(m) = &map {
                    load.write(LoadLevel { map: m.info.name.clone(), menu: false });
                }
                *screen = Screen::None;
            }
            Action::MainMenu => {
                load.write(LoadLevel { map: menu_background(&campaign), menu: true });
                *screen = Screen::Main;
            }
            // Settings picked from a list step through it.
            Action::Adjust(Setting::Resolution, d) => {
                settings.resolution = step_in(&resolution_choices(), settings.resolution, d.signum() as i32, |r| r.map_or(f32::MAX, |r| (r.x * r.y) as f32))
            }
            Action::Adjust(Setting::FrameLimit, d) => settings.frame_limit = step_in(&FRAME_LIMITS, settings.frame_limit, d.signum() as i32, frame_limit_rank),
            Action::Adjust(Setting::Anisotropy, d) => settings.anisotropy = step_in(&ANISOTROPY, settings.anisotropy, d.signum() as i32, |a| a as f32),
            Action::Adjust(Setting::SubtitleSize, d) => {
                settings.subtitle_size = step_in(&SUBTITLE_SIZES.map(|z| z.0), settings.subtitle_size, d.signum() as i32, |z| z)
            }
            Action::Adjust(Setting::SubtitleColor, d) => {
                settings.subtitle_color = step_in(&SubtitleColor::ALL, settings.subtitle_color, d.signum() as i32, |c| c as u8 as f32)
            }
            Action::Adjust(s, d) => {
                let v = match s {
                    Setting::Master => &mut settings.master_volume,
                    Setting::Music => &mut settings.music_volume,
                    Setting::Effects => &mut settings.effects_volume,
                    Setting::Sensitivity => &mut settings.mouse_sensitivity,
                    Setting::Fov => &mut settings.fov,
                    Setting::SubtitleBackground => &mut settings.subtitle_background,
                    _ => continue,
                };
                *v += d;
                match s {
                    Setting::Sensitivity => *v = v.clamp(0.5, 20.0),
                    Setting::Fov => *v = v.clamp(FOV_RANGE.0, FOV_RANGE.1),
                    _ => *v = (v.clamp(0.0, 1.0) * 10.0).round() / 10.0,
                }
            }
            Action::Toggle(s) => match s {
                Setting::Invert => settings.invert_mouse = !settings.invert_mouse,
                Setting::Smoothing => settings.mouse_filter = !settings.mouse_filter,
                Setting::Fullscreen => settings.fullscreen = !settings.fullscreen,
                Setting::VSync => settings.vsync = !settings.vsync,
                Setting::AntiAliasing => settings.msaa = !settings.msaa,
                Setting::Graphics => settings.enhanced_graphics = !settings.enhanced_graphics,
                Setting::Subtitles => settings.subtitles = !settings.subtitles,
                _ => {}
            },
        }
    }
}

/// Paused or in a menu: stop the world, free the mouse. Playing: grab it.
fn pause_world(
    screen: Res<Screen>,
    map: Option<Res<CurrentMap>>,
    mut time: ResMut<Time<Virtual>>,
    mut locked: ResMut<InputLocked>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut inputs: Query<&mut MoveInput, With<Player>>,
    args: Res<p2_room::HarnessArgs>,
) {
    if !screen.is_changed() && !map.as_ref().is_some_and(|m| m.is_changed()) {
        return;
    }
    let scripted = args.automated() || std::env::var_os("P2_SCRIPT").is_some();
    let playing = *screen == Screen::None && map.as_ref().is_some_and(|m| !m.menu);
    let in_menu_map = map.as_ref().is_none_or(|m| m.menu);
    if *screen == Screen::Pause {
        time.pause();
    } else {
        time.unpause();
    }
    // Scripted runs (screenshots) keep their own input lock.
    if !scripted {
        locked.0 = !playing;
    }
    if !playing {
        for mut i in &mut inputs {
            *i = MoveInput::default();
        }
    }
    if let Ok(mut c) = cursor.single_mut() {
        if playing && !scripted {
            c.grab_mode = CursorGrabMode::Locked;
            c.visible = false;
        } else {
            c.grab_mode = CursorGrabMode::None;
            c.visible = true;
        }
    }
    let _ = in_menu_map;
}

/// Put the menu's view in the chamber: a few metres from its puzzle (floor
/// button, cube, laser), on the side with the most room, facing it.
fn menu_view(
    mut loaded: MessageReader<LevelLoaded>,
    mut wait: Local<Option<u32>>,
    ents: Option<Res<crate::io::Ents>>,
    map: Option<Res<CurrentMap>>,
    spatial: avian3d::prelude::SpatialQuery,
    mut players: Query<(Entity, &mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity), With<Player>>,
) {
    if loaded.read().last().is_some_and(|e| e.menu) {
        *wait = Some(3);
    }
    let Some(n) = *wait else { return };
    if n > 0 {
        *wait = Some(n - 1);
        return;
    }
    *wait = None;
    let (Some(ents), Some(map)) = (ents, map) else { return };
    if !map.menu {
        return;
    }
    let Ok((player, mut tf, mut feet, mut view, mut vel)) = players.single_mut() else { return };
    let filter = avian3d::prelude::SpatialQueryFilter::from_excluded_entities([player]);
    // A focus inside the chamber: floor just below it and a ceiling above.
    let enclosed = |p: Vec3| {
        let down = spatial.cast_ray(p + Vec3::Y * 0.5, Dir3::NEG_Y, 3.0, true, &filter);
        let up = spatial.cast_ray(p + Vec3::Y * 0.5, Dir3::Y, 25.0, true, &filter);
        down.is_some() && up.is_some()
    };
    let focus_classes = ["prop_floor_button", "prop_weighted_cube", "env_portal_laser", "prop_laser_catcher", "prop_button", "prop_portal", "prop_testchamber_door"];
    let Some((class, focus)) = focus_classes.iter().find_map(|c| {
        ents.list.iter().filter(|e| e.class == *c).map(|e| e.transform.translation).find(|p| enclosed(*p)).map(|p| (*c, p))
    }) else {
        warn!("portal2: no menu view spot in {}", map.info.name);
        return;
    };
    // Stand on the floor under it.
    let focus = spatial
        .cast_ray(focus + Vec3::Y * 0.5, Dir3::NEG_Y, 3.0, true, &filter)
        .map_or(focus, |h| focus + Vec3::Y * 0.5 - Vec3::Y * h.distance);
    info!("portal2: menu view on {class} at {focus:.2?}");
    let from = focus + Vec3::Y * 1.2;
    let mut best = (0.0f32, Vec3::X);
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let free = spatial.cast_ray(from, Dir3::new(d).unwrap(), 20.0, true, &filter).map_or(20.0, |h| h.distance);
        if free > best.0 {
            best = (free, d);
        }
    }
    let back = (best.0 - 1.0).clamp(1.0, 7.0);
    let at = Vec3::new(focus.x, focus.y + 0.05, focus.z) + best.1 * back;
    tf.translation = at;
    *feet = InterpolatedFeet { prev: at, curr: at };
    let to = focus - at;
    *view = ViewAngles { yaw: (-to.x).atan2(-to.z), pitch: -0.12, roll: 0.0 };
    vel.0 = Vec3::ZERO;
}

/// Behind the main menu the view slowly looks around the chamber.
fn menu_camera(time: Res<Time<Real>>, map: Option<Res<CurrentMap>>, mut players: Query<&mut ViewAngles, With<Player>>) {
    let Some(map) = map else { return };
    if !map.menu {
        return;
    }
    for mut v in &mut players {
        let t = time.elapsed_secs();
        v.yaw += (t * 0.05).sin() * time.delta_secs() * 0.05;
        v.pitch = -0.12 + (t * 0.07).sin() * 0.05;
    }
}

/// The menu plays one of the game's main-menu tracks.
fn menu_music(mut loaded: MessageReader<LevelLoaded>, mut play: MessageWriter<PlayGameSound>, game: Res<GameFsRes>, campaign: Res<Campaign>) {
    let Some(ev) = loaded.read().last().cloned() else { return };
    if !ev.menu {
        return;
    }
    let tracks: Vec<String> = game.0.list("sound/music/mainmenu").into_iter().filter(|p| p.ends_with(".wav")).collect();
    if tracks.is_empty() {
        return;
    }
    // The track for the act the menu shows, as the game does.
    let act = campaign.chapter_of(&ev.map).unwrap_or(0);
    let track = &tracks[act.min(tracks.len() - 1)];
    play.write(PlayGameSound { looped: true, ..PlayGameSound::music(track.trim_start_matches("sound/")) });
}

/// Every level start is saved for "Continue" (see `may_write_data`).
fn autosave(mut loaded: MessageReader<LevelLoaded>) {
    for ev in loaded.read() {
        if !ev.menu && crate::settings::may_write_data() {
            let _ = std::fs::write(data_dir().join("save.txt"), save_text(&ev.map));
        }
    }
}

/// "Chapter N / Title" when a chapter's first map starts.
fn title_card(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    mut requested: MessageReader<ShowChapterTitle>,
    map: Option<Res<CurrentMap>>,
    campaign: Res<Campaign>,
    fonts: Option<Res<UiFonts>>,
    time: Res<Time<Real>>,
    mut card: ResMut<TitleCard>,
    roots: Query<Entity, With<TitleCardRoot>>,
    mut colors: Query<&mut TextColor>,
    children: Query<&Children>,
    music: Query<(), With<GameSound>>,
) {
    let Some(fonts) = fonts else { return };
    // A chapter's first map shows its card on spawn, or when the map asks.
    let mut show: Option<(usize, f32)> = None;
    for ev in loaded.read() {
        if ev.menu {
            continue;
        }
        if let Some(i) = campaign.chapters.iter().position(|c| c.map.eq_ignore_ascii_case(&ev.map)) {
            if campaign.chapters[i].display_on_spawn {
                show = Some((i, campaign.chapters[i].display_delay));
            }
        }
    }
    if requested.read().count() > 0 {
        if let Some(i) = map.as_ref().and_then(|m| campaign.chapter_of(&m.info.name)) {
            show = Some((i, 0.0));
        }
    }
    {
        if let Some((i, delay)) = show {
            let c = &campaign.chapters[i];
            // The game's own strings: "Chapter 1" / "The Courtesy Call".
            card.lines = Some((String::new(), c.title.clone(), c.subtitle.to_uppercase()));
            card.shown_at = time.elapsed_secs() + delay;
            for r in &roots {
                commands.entity(r).despawn();
            }
            let (a, b, s) = card.lines.clone().unwrap_or_default();
            commands.spawn((
                TitleCardRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                GlobalZIndex(5),
                bevy::picking::Pickable::IGNORE,
                children![
                    text(&fonts.regular, 26.0, a, Color::NONE),
                    text(&fonts.bold, 54.0, b, Color::NONE),
                    text(&fonts.regular, 20.0, s, Color::NONE),
                ],
            ));
        }
    }
    let _ = music;
    let Some(_) = &card.lines else { return };
    let t = time.elapsed_secs() - card.shown_at;
    let alpha = if t < 0.0 { 0.0 } else if t < 1.0 { t } else if t < 5.0 { 1.0 } else { (6.0 - t).max(0.0) };
    for r in &roots {
        for c in children.iter_descendants(r) {
            if let Ok(mut tc) = colors.get_mut(c) {
                tc.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
            }
        }
        if t > 6.5 {
            commands.entity(r).despawn();
            card.lines = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autosave_round_trip() {
        assert_eq!(parse_save(&save_text("sp_a2_laser_intro")).as_deref(), Some("sp_a2_laser_intro"));
    }

    #[test]
    fn autosave_reads_old_and_newer_files() {
        // Before `version`: just the map name.
        assert_eq!(parse_save("sp_a1_intro3\n").as_deref(), Some("sp_a1_intro3"));
        // A newer file with keys this build doesn't know.
        assert_eq!(parse_save("version 7\nchecksum abc\nmap sp_a4_finale1\nslot 2\n").as_deref(), Some("sp_a4_finale1"));
        assert_eq!(parse_save(""), None);
        assert_eq!(parse_save("version 2\n"), None);
    }
}
