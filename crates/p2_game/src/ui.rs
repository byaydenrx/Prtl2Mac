//! Menus and HUD text, laid out like Portal 2's PC menus: the main menu
//! (logo at the top left, items stacked at the lower left, Play Single
//! Player and Options opening flyouts beside them) over the game's own
//! background still and menu music, the New Game chapter dialog with the
//! game's chapter pictures and titles, the pause menu, the Options dialogs
//! (rows and footer buttons; content in options.rs), quit confirmations,
//! chapter title cards and the autosave that "Continue" picks up.
//!
//! Fonts and the logo come from the player's install when it has them (see
//! [`load_menu_art`]); otherwise DejaVu Sans and a text logo.
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::{CursorGrabMode, CursorOptions, Monitor, PrimaryMonitor, PrimaryWindow};
use p2_assets::GameFsRes;
use p2_core::*;

use crate::audio::{GameSound, PlayGameSound};
use crate::bindings::{DefaultBinds, KeyAction, KeyActions};
use crate::campaign::Campaign;
use crate::level::{CurrentMap, LevelLoaded, LoadLevel};
use crate::options::{self, Ctx, Footer, OptionsPage, Row, Setting, VideoMode};
use crate::settings::{data_dir, Settings};

pub struct UiPlugin;

/// Scripted tests press a menu button by its action (`SinglePlayer`,
/// `Page(Video)`, `Toggle(VSync)`, `Adjust(Fov,1.0)`, `Done`, spaces
/// ignored), or `pause`.
#[derive(Message, Clone, Debug)]
pub struct MenuClick(pub String);

/// Show the current chapter's title card (the map's `DisplayChapterTitle()`).
#[derive(Message, Clone, Copy, Debug)]
pub struct ShowChapterTitle;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Screen::Main)
            .init_resource::<MenuState>()
            .init_resource::<MenuScroll>()
            .init_resource::<InputEaten>()
            .init_resource::<TitleCard>()
            .add_message::<ShowChapterTitle>()
            .add_message::<MenuClick>()
            .add_systems(Startup, (load_fonts, load_menu_art, open_main_menu).chain().after(crate::campaign::load_campaign))
            .add_systems(
                Update,
                (
                    (capture_key, escape_key, scripted_clicks, buttons, scroll_lists, rebuild_menu).chain(),
                    (menu_view, menu_camera, pause_world, menu_music, autosave, title_card).chain(),
                )
                    .chain()
                    .in_set(crate::level::GameSet::Logic),
            );
    }
}

/// Which menu is up. `None` while playing.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    None,
    Main,
    /// New Game: the chapter list.
    Chapters,
    Options { from_pause: bool },
    Pause,
}

/// The flyouts beside the main and pause menu items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flyout {
    SinglePlayer,
    Options,
}

/// "Are you sure?" before quitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Confirm {
    QuitGame,
    QuitToMenu,
}

/// Where the menus are within a screen.
#[derive(Resource, Default)]
struct MenuState {
    flyout: Option<Flyout>,
    confirm: Option<Confirm>,
    /// The Options page shown on [`Screen::Options`].
    page: OptionsPage,
    /// Settings as they were when each open page was entered, for Cancel.
    undo: Vec<Settings>,
    /// The Video page's unapplied resolution and display mode.
    video: Option<VideoMode>,
    /// Key-list row waiting for a key, and the row last clicked.
    capture: Option<usize>,
    selected: Option<usize>,
}

/// How far the key list is scrolled; kept apart from [`MenuState`] so
/// scrolling doesn't rebuild the menu.
#[derive(Resource, Default)]
struct MenuScroll(f32);

/// Set for the frame in which a key was taken for a binding (or Esc
/// cancelled one), so it doesn't also act as a menu key or click.
#[derive(Resource, Default)]
struct InputEaten(bool);

#[derive(Resource, Clone)]
pub struct UiFonts {
    pub bold: Handle<Font>,
    pub regular: Handle<Font>,
    /// Menu items and titles: the game's own menu font when found, else `bold`.
    pub menu: Handle<Font>,
}

/// The game's menu art.
#[derive(Resource, Default)]
struct MenuArt {
    logo: Option<Handle<Image>>,
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

/// The scrolling key list.
#[derive(Component)]
struct ScrollList;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum Action {
    SinglePlayer,
    Continue,
    /// Opens the chapter list, as in Portal 2.
    NewGame,
    Chapter(usize),
    Options,
    /// An Options page, from the Options flyout or a link row.
    Page(OptionsPage),
    Back,
    Done,
    Cancel,
    Apply,
    UseDefaults,
    ClearKey,
    Quit,
    Resume,
    Restart,
    MainMenu,
    ConfirmOk,
    ConfirmCancel,
    /// The arrows beside a value: one step back (-1) or on (+1).
    Adjust(Setting, f32),
    /// A click on the value itself: one step on, wrapping round.
    Toggle(Setting),
    /// A click on a slider's track: jump to that point.
    Slide(Setting),
    /// A key-list row: wait for a key to bind.
    Bind(usize),
}

impl From<Footer> for Action {
    fn from(f: Footer) -> Self {
        match f {
            Footer::Back => Action::Back,
            Footer::Done => Action::Done,
            Footer::Cancel => Action::Cancel,
            Footer::Apply => Action::Apply,
            Footer::UseDefaults => Action::UseDefaults,
            Footer::ClearKey => Action::ClearKey,
        }
    }
}

/// Portal 2's menu colours: off-white items that go full white over a
/// faint bar when hovered, dialogs on a dark translucent panel.
mod theme {
    use bevy::prelude::Color;

    pub const ITEM: Color = Color::srgba(0.86, 0.88, 0.9, 0.9);
    pub const ITEM_HOVER: Color = Color::WHITE;
    pub const HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.1);
    pub const SELECTED: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);
    pub const TITLE: Color = Color::srgb(0.95, 0.96, 0.97);
    pub const LABEL: Color = Color::srgba(0.76, 0.79, 0.82, 1.0);
    pub const VALUE: Color = Color::srgb(0.95, 0.96, 0.97);
    pub const HEADING: Color = Color::srgba(0.55, 0.6, 0.65, 1.0);
    pub const HINT: Color = Color::srgba(0.6, 0.64, 0.68, 1.0);
    pub const PANEL: Color = Color::srgba(0.025, 0.03, 0.035, 0.88);
    pub const EDGE: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
    pub const TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);
    pub const FILL: Color = Color::srgba(0.86, 0.88, 0.9, 0.9);
    pub const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
}

/// A button's colours, idle and hovered.
#[derive(Component, Clone, Copy)]
struct Look {
    text: Color,
    text_hover: Color,
    bg: Color,
    bg_hover: Color,
}

const ITEM_LOOK: Look = Look { text: theme::ITEM, text_hover: theme::ITEM_HOVER, bg: Color::NONE, bg_hover: theme::HIGHLIGHT };
const VALUE_LOOK: Look = Look { text: theme::VALUE, text_hover: theme::ITEM_HOVER, bg: Color::NONE, bg_hover: theme::HIGHLIGHT };

/// Menu item text size; dialogs scale from it.
const ITEM_SIZE: f32 = 24.0;

fn load_fonts(mut commands: Commands, game: Res<GameFsRes>, mut fonts: ResMut<Assets<Font>>) {
    let load = |fonts: &mut Assets<Font>, paths: &[&str]| -> Option<Handle<Font>> { paths.iter().find_map(|p| game.read(p)).map(|b| fonts.add(Font::from_bytes(b))) };
    let bold = load(&mut fonts, &["resource/linux_fonts/dejavusans-bold.ttf", "resource/linux_fonts/liberationsans-bold.ttf"]).unwrap_or_default();
    let regular = load(&mut fonts, &["resource/linux_fonts/dejavusans.ttf", "resource/linux_fonts/liberationsans-regular.ttf"]).unwrap_or_default();
    // The menu font: P2_MENU_FONT (a game path or a file), else the best
    // match among the install's own font files.
    let files: Vec<String> = game.0.list("resource").into_iter().filter(|f| f.ends_with(".ttf") || f.ends_with(".otf")).collect();
    info!("portal2: font files in the game: {files:?}");
    let chosen = std::env::var("P2_MENU_FONT").ok().or_else(|| pick_menu_font(&files).cloned());
    let menu = chosen.as_deref().and_then(|p| game.read(p).or_else(|| std::fs::read(p).ok()).map(|b| (p, fonts.add(Font::from_bytes(b)))));
    let menu = match menu {
        Some((p, h)) => {
            info!("portal2: menu font {p}");
            h
        }
        None => bold.clone(),
    };
    commands.insert_resource(UiFonts { bold, regular, menu });
}

/// The menu font among the game's font files: Portal 2's menus use a
/// clean grotesque (Univers / DIN-like), so prefer those names; never an
/// icon font. `None` keeps DejaVu Sans Bold.
fn pick_menu_font(files: &[String]) -> Option<&String> {
    let score = |f: &String| -> i32 {
        let name = f.rsplit('/').next().unwrap_or(f).to_ascii_lowercase();
        if ["halflife", "marlett", "icon", "symbol", "dings", "dingbat", "glyph", "button", "controller", "xbox", "ps3"].iter().any(|k| name.contains(k)) {
            return 0;
        }
        let family = [("univers", 10), ("din", 8), ("frutiger", 7), ("helvetica", 6), ("trade", 5)].iter().find(|(k, _)| name.contains(k)).map_or(0, |(_, s)| *s);
        if family == 0 {
            return 0;
        }
        let weight = if name.contains("bold") || name.contains("67") || name.contains("bd") { 1 } else { 0 };
        let cond = if name.contains("cond") || name.contains("cn") { 1 } else { 0 };
        family * 4 + weight * 2 + cond
    };
    files.iter().filter(|f| score(f) > 0).max_by_key(|f| (score(f), std::cmp::Reverse(f.len())))
}

/// The Portal 2 logo from the game's vgui materials (`P2_MENU_LOGO`
/// overrides the path): a `logo` texture under materials/vgui naming the
/// game, not a co-op/DLC one.
fn load_menu_art(mut commands: Commands, game: Res<GameFsRes>, mut images: ResMut<Assets<Image>>) {
    let files = game.0.list("materials/vgui");
    let logos: Vec<&String> = files.iter().filter(|f| f.ends_with(".vtf") && f.contains("logo")).collect();
    info!("portal2: logo textures in the game: {logos:?}");
    let chosen = std::env::var("P2_MENU_LOGO").ok().or_else(|| pick_logo(&files).cloned());
    let logo = chosen.and_then(|p| {
        let vtf = source_assets::vtf::VtfTexture::parse(&game.read(&p)?).ok()?;
        info!("portal2: menu logo {p}");
        Some(images.add(vtf.into_image(false, true)))
    });
    commands.insert_resource(MenuArt { logo });
}

fn pick_logo(files: &[String]) -> Option<&String> {
    files
        .iter()
        .filter(|f| {
            let n = f.rsplit('/').next().unwrap_or(f);
            f.ends_with(".vtf") && n.contains("logo") && (n.contains("portal2") || n.contains("portal_2")) && !["coop", "dlc", "community", "puzzlemaker", "steam", "valve"].iter().any(|k| n.contains(k))
        })
        .min_by_key(|f| (!f.contains("english"), f.len()))
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

/// A button showing `label`.
fn button(font: &Handle<Font>, label: impl Into<String>, action: Action, size: f32, look: Look) -> impl Bundle {
    (
        Button,
        action,
        look,
        Node { padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)), align_items: AlignItems::Center, ..default() },
        BackgroundColor(look.bg),
        children![text(font, size, label, look.text)],
    )
}

/// A main/pause menu item.
fn item(fonts: &UiFonts, label: &str, action: Action) -> impl Bundle + use<> {
    button(&fonts.menu, label.to_string(), action, ITEM_SIZE, ITEM_LOOK)
}

/// The native size: the primary monitor's, else the window's.
fn native_size(monitors: &Query<&Monitor, With<PrimaryMonitor>>, windows: &Query<&Window, With<PrimaryWindow>>) -> UVec2 {
    monitors
        .iter()
        .next()
        .map(|m| m.physical_size())
        .or_else(|| windows.single().ok().map(|w| w.physical_size()))
        .filter(|s| s.x > 0 && s.y > 0)
        .unwrap_or(UVec2::new(1920, 1080))
}

/// A full-screen layer of the menu.
fn layer(z: i32) -> impl Bundle {
    (
        MenuRoot,
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        GlobalZIndex(z),
    )
}

/// The lower-left item column with an optional flyout beside one item.
fn item_list(p: &mut ChildSpawnerCommands, fonts: &UiFonts, items: &[(&str, Action, Option<Flyout>)], open: Option<Flyout>, flyout_items: &[(&str, Action)]) {
    p.spawn(Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(6.0),
        bottom: Val::Percent(12.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|col| {
        for (label, action, fly) in items {
            col.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|row| {
                row.spawn(item(fonts, label, *action));
                if fly.is_some() && *fly == open {
                    row.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(100.0),
                            top: Val::Px(0.0),
                            margin: UiRect::left(Val::Px(16.0)),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(240.0),
                            ..default()
                        },
                        BackgroundColor(theme::PANEL),
                        BorderColor::all(theme::EDGE),
                    ))
                    .with_children(|f| {
                        for (label, action) in flyout_items {
                            f.spawn(button(&fonts.menu, *label, *action, ITEM_SIZE * 0.8, ITEM_LOOK));
                        }
                    });
                }
            });
        }
    });
}

/// A dialog panel at the left of the screen: title, rule, then the
/// caller's content. Returns the panel entity.
fn dialog(commands: &mut Commands, fonts: &UiFonts, title: &str, width: f32) -> Entity {
    let mut root = commands.spawn((
        layer(10),
        BackgroundColor(Color::NONE),
    ));
    root.insert(Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        padding: UiRect::left(Val::Percent(6.0)),
        ..default()
    });
    let mut panel = Entity::PLACEHOLDER;
    root.with_children(|r| {
        panel = r
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(width),
                    max_width: Val::Percent(88.0),
                    max_height: Val::Percent(92.0),
                    padding: UiRect::axes(Val::Px(28.0), Val::Px(20.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(theme::PANEL),
                BorderColor::all(theme::EDGE),
            ))
            .with_children(|d| {
                d.spawn(text(&fonts.menu, 30.0, title, theme::TITLE));
                d.spawn((Node { height: Val::Px(1.0), width: Val::Percent(100.0), margin: UiRect::vertical(Val::Px(10.0)), ..default() }, BackgroundColor(theme::EDGE)));
            })
            .id();
    });
    panel
}

/// The bottom of a dialog: a hint at the left, buttons at the right.
fn footer(p: &mut ChildSpawnerCommands, fonts: &UiFonts, hint: &str, buttons: &[Footer]) {
    p.spawn((Node { height: Val::Px(1.0), width: Val::Percent(100.0), margin: UiRect::top(Val::Px(14.0)), ..default() }, BackgroundColor(theme::EDGE)));
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        margin: UiRect::top(Val::Px(8.0)),
        column_gap: Val::Px(16.0),
        ..default()
    })
    .with_children(|f| {
        f.spawn(text(&fonts.regular, 14.0, hint, theme::HINT));
        f.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() }).with_children(|b| {
            for &k in buttons {
                b.spawn(button(&fonts.menu, k.label(), k.into(), 18.0, ITEM_LOOK));
            }
        });
    });
}

/// One Options row.
fn option_row(p: &mut ChildSpawnerCommands, fonts: &UiFonts, row: &Row) {
    let line = Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(32.0), column_gap: Val::Px(6.0), ..default() };
    let label = |r: &mut ChildSpawnerCommands, s: &str| {
        r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.regular, 18.0, s, theme::LABEL)]));
    };
    match row {
        Row::Heading(h) => {
            p.spawn((Node { margin: UiRect::top(Val::Px(12.0)), ..default() }, children![text(&fonts.bold, 13.0, h.to_uppercase(), theme::HEADING)]));
        }
        Row::Choice { label: l, value, setting } => {
            p.spawn(line).with_children(|r| {
                label(r, l);
                r.spawn(button(&fonts.bold, "<", Action::Adjust(*setting, -1.0), 18.0, ITEM_LOOK));
                r.spawn((
                    Button,
                    Action::Toggle(*setting),
                    VALUE_LOOK,
                    Node { width: Val::Px(250.0), justify_content: JustifyContent::Center, padding: UiRect::vertical(Val::Px(4.0)), ..default() },
                    BackgroundColor(Color::NONE),
                    children![text(&fonts.regular, 18.0, value.clone(), theme::VALUE)],
                ));
                r.spawn(button(&fonts.bold, ">", Action::Adjust(*setting, 1.0), 18.0, ITEM_LOOK));
            });
        }
        Row::Slider { label: l, value, fraction, setting } => {
            p.spawn(line).with_children(|r| {
                label(r, l);
                r.spawn(button(&fonts.bold, "<", Action::Adjust(*setting, -1.0), 18.0, ITEM_LOOK));
                r.spawn((Node { width: Val::Px(250.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() },))
                    .with_children(|v| {
                        v.spawn((
                            Button,
                            Action::Slide(*setting),
                            Look { text: theme::VALUE, text_hover: theme::ITEM_HOVER, bg: Color::NONE, bg_hover: Color::NONE },
                            RelativeCursorPosition::default(),
                            Node { flex_grow: 1.0, height: Val::Px(24.0), align_items: AlignItems::Center, ..default() },
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|t| {
                            t.spawn((Node { width: Val::Percent(100.0), height: Val::Px(6.0), ..default() }, BackgroundColor(theme::TRACK)))
                                .with_child((Node { width: Val::Percent(fraction * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(theme::FILL)));
                        });
                        v.spawn((Node { width: Val::Px(52.0), justify_content: JustifyContent::FlexEnd, ..default() }, children![text(&fonts.regular, 16.0, value.clone(), theme::VALUE)]));
                    });
                r.spawn(button(&fonts.bold, ">", Action::Adjust(*setting, 1.0), 18.0, ITEM_LOOK));
            });
        }
        Row::Link { label: l, page } => {
            p.spawn((
                Button,
                Action::Page(*page),
                VALUE_LOOK,
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(32.0), padding: UiRect::right(Val::Px(8.0)), ..default() },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|r| {
                r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.regular, 18.0, l.clone(), theme::VALUE)]));
                r.spawn(text(&fonts.bold, 18.0, ">", theme::VALUE));
            });
        }
        Row::Key { label: l, keys, index, selected } => {
            let bg = if *selected { theme::SELECTED } else { Color::NONE };
            p.spawn((
                Button,
                Action::Bind(*index),
                Look { text: theme::VALUE, text_hover: theme::ITEM_HOVER, bg, bg_hover: theme::HIGHLIGHT },
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(28.0), padding: UiRect::horizontal(Val::Px(8.0)), ..default() },
                BackgroundColor(bg),
            ))
            .with_children(|r| {
                r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.regular, 17.0, l.clone(), theme::VALUE)]));
                r.spawn((Node { width: Val::Px(220.0), ..default() }, children![text(&fonts.bold, 17.0, keys.clone(), theme::VALUE)]));
            });
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn rebuild_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    menu: Res<MenuState>,
    scroll: Res<MenuScroll>,
    (fonts, art, campaign, settings, actions, defaults): (Option<Res<UiFonts>>, Option<Res<MenuArt>>, Res<Campaign>, Res<Settings>, Res<KeyActions>, Res<DefaultBinds>),
    roots: Query<Entity, With<MenuRoot>>,
    game: Res<GameFsRes>,
    mut images: ResMut<Assets<Image>>,
    (monitors, windows): (Query<&Monitor, With<PrimaryMonitor>>, Query<&Window, With<PrimaryWindow>>),
    mut chapter_pics: Local<Vec<Option<Handle<Image>>>>,
    mut backgrounds: Local<std::collections::HashMap<usize, Option<Handle<Image>>>>,
) {
    let Some(fonts) = fonts else { return };
    let options_changed = (settings.is_changed() || actions.is_changed()) && matches!(*screen, Screen::Options { .. });
    if !screen.is_changed() && !menu.is_changed() && !options_changed && !fonts.is_added() {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    // Out of the game, the menu sits on the game's own menu still for the
    // act of the saved chapter; in the game, the view shows through dimmed.
    let in_game = matches!(*screen, Screen::Pause | Screen::Options { from_pause: true });
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
        let mut bg = commands.spawn((layer(9), BackgroundColor(Color::BLACK)));
        if let Some(img) = img {
            bg.with_child((
                ImageNode { image: img, image_mode: bevy::ui::widget::NodeImageMode::Stretch, ..default() },
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            ));
        }
    } else if in_game {
        commands.spawn((layer(9), BackgroundColor(theme::SHADE)));
    }
    let options_flyout: Vec<(&str, Action)> = options::rows(OptionsPage::List, &settings, &dummy_ctx())
        .into_iter()
        .filter_map(|r| match r {
            Row::Link { page, .. } => Some((page.title(), Action::Page(page))),
            _ => None,
        })
        .collect();
    match *screen {
        Screen::None => {}
        Screen::Main => {
            let mut root = commands.spawn(layer(10));
            root.with_children(|p| {
                // The logo, top left.
                let logo_node = Node { position_type: PositionType::Absolute, left: Val::Percent(6.0), top: Val::Percent(9.0), width: Val::Percent(34.0), ..default() };
                match art.as_ref().and_then(|a| a.logo.clone()) {
                    Some(img) => {
                        p.spawn((ImageNode::new(img), logo_node));
                    }
                    None => {
                        p.spawn((logo_node, children![text(&fonts.menu, 72.0, "PORTAL 2", theme::TITLE)]));
                    }
                }
                let sp: Vec<(&str, Action)> =
                    if read_save().is_some() { vec![("CONTINUE GAME", Action::Continue), ("NEW GAME", Action::NewGame)] } else { vec![("NEW GAME", Action::NewGame)] };
                let fly = if menu.flyout == Some(Flyout::Options) { &options_flyout } else { &sp };
                item_list(
                    p,
                    &fonts,
                    &[("PLAY SINGLE PLAYER", Action::SinglePlayer, Some(Flyout::SinglePlayer)), ("OPTIONS", Action::Options, Some(Flyout::Options)), ("QUIT", Action::Quit, None)],
                    menu.flyout,
                    fly,
                );
            });
        }
        Screen::Pause => {
            let mut root = commands.spawn(layer(10));
            root.with_children(|p| {
                item_list(
                    p,
                    &fonts,
                    &[
                        ("RESUME GAME", Action::Resume, None),
                        ("RESTART LEVEL", Action::Restart, None),
                        ("OPTIONS", Action::Options, Some(Flyout::Options)),
                        ("QUIT TO MAIN MENU", Action::MainMenu, None),
                        ("QUIT GAME", Action::Quit, None),
                    ],
                    menu.flyout,
                    &options_flyout,
                );
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
            let panel = dialog(&mut commands, &fonts, "NEW GAME", 980.0);
            commands.entity(panel).with_children(|p| {
                p.spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                    column_gap: Val::Px(14.0),
                    row_gap: Val::Px(10.0),
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|g| {
                    for (i, c) in campaign.chapters.iter().enumerate() {
                        g.spawn((
                            Button,
                            Action::Chapter(i),
                            VALUE_LOOK,
                            Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(6.0)), row_gap: Val::Px(2.0), ..default() },
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|b| {
                            if let Some(Some(img)) = chapter_pics.get(i) {
                                b.spawn((ImageNode::new(img.clone()), Node { width: Val::Percent(100.0), aspect_ratio: Some(16.0 / 9.0), ..default() }));
                            }
                            b.spawn(text(&fonts.regular, 14.0, c.title.to_uppercase(), theme::LABEL));
                            b.spawn(text(&fonts.menu, 17.0, c.subtitle.to_uppercase(), theme::VALUE));
                        });
                    }
                });
                footer(p, &fonts, "", &[Footer::Back]);
            });
        }
        Screen::Options { .. } => {
            let page = menu.page;
            let binds = settings.binds.clone().unwrap_or_else(|| defaults.0.clone());
            let ctx = Ctx {
                native: native_size(&monitors, &windows),
                video: menu.video.unwrap_or_else(|| VideoMode::of(&settings)),
                keys: &actions.0,
                binds: &binds,
                capture: menu.capture,
                selected: menu.selected,
            };
            let rows = options::rows(page, &settings, &ctx);
            let hint = match (page, menu.capture) {
                (OptionsPage::Keys, Some(i)) => {
                    let name = match actions.0.get(i) {
                        Some(KeyAction::Command { label, .. }) => label.as_str(),
                        _ => "",
                    };
                    format!("Press a key for \"{name}\".  ESC cancels.")
                }
                (OptionsPage::Keys, None) => "Click an action, then press its new key.".to_string(),
                (OptionsPage::Video, _) if menu.video.is_some_and(|v| v != VideoMode::of(&settings)) => "APPLY to use the new display settings.".to_string(),
                _ => String::new(),
            };
            let panel = dialog(&mut commands, &fonts, page.title(), if page == OptionsPage::Keys { 760.0 } else { 720.0 });
            commands.entity(panel).with_children(|p| {
                if page == OptionsPage::Keys {
                    p.spawn((
                        ScrollList,
                        ScrollPosition(Vec2::new(0.0, scroll.0)),
                        Node { flex_direction: FlexDirection::Column, overflow: Overflow::scroll_y(), max_height: Val::Vh(56.0), ..default() },
                    ))
                    .with_children(|l| {
                        for r in &rows {
                            option_row(l, &fonts, r);
                        }
                    });
                } else {
                    for r in &rows {
                        option_row(p, &fonts, r);
                    }
                }
                if page == OptionsPage::Subtitles {
                    // A sample caption in the chosen style.
                    p.spawn((
                        Node { margin: UiRect::top(Val::Px(16.0)), padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)), align_self: AlignSelf::Center, ..default() },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, settings.subtitle_background)),
                        children![text(&fonts.regular, settings.subtitle_size, "Subtitles will look like this.", settings.subtitle_color.color().unwrap_or(theme::VALUE))],
                    ));
                }
                footer(p, &fonts, &hint, page.footer());
            });
        }
    }
    if let Some(c) = menu.confirm {
        let (title, message) = match c {
            Confirm::QuitGame => ("QUIT", "Are you sure you want to quit?"),
            Confirm::QuitToMenu => ("QUIT TO MAIN MENU", "Are you sure you want to quit to the main menu? Any progress since the last checkpoint will be lost."),
        };
        let mut shade = commands.spawn((layer(20), BackgroundColor(theme::SHADE)));
        shade.insert(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        });
        shade.with_children(|s| {
            s.spawn((
                Node { flex_direction: FlexDirection::Column, width: Val::Px(520.0), max_width: Val::Percent(90.0), padding: UiRect::axes(Val::Px(28.0), Val::Px(20.0)), border: UiRect::all(Val::Px(1.0)), ..default() },
                BackgroundColor(theme::PANEL),
                BorderColor::all(theme::EDGE),
            ))
            .with_children(|d| {
                d.spawn(text(&fonts.menu, 26.0, title, theme::TITLE));
                d.spawn((Node { margin: UiRect::top(Val::Px(12.0)), ..default() }, children![text(&fonts.regular, 18.0, message, theme::LABEL)]));
                d.spawn((Node { height: Val::Px(1.0), width: Val::Percent(100.0), margin: UiRect::top(Val::Px(18.0)), ..default() }, BackgroundColor(theme::EDGE)));
                d.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::FlexEnd, column_gap: Val::Px(8.0), margin: UiRect::top(Val::Px(8.0)), ..default() })
                    .with_children(|b| {
                        b.spawn(button(&fonts.menu, "OK", Action::ConfirmOk, 18.0, ITEM_LOOK));
                        b.spawn(button(&fonts.menu, "CANCEL", Action::ConfirmCancel, 18.0, ITEM_LOOK));
                    });
            });
        });
    }
}

/// For rows that need no context (the category list).
fn dummy_ctx() -> Ctx<'static> {
    static BINDS: std::sync::OnceLock<crate::bindings::BindTable> = std::sync::OnceLock::new();
    Ctx { native: UVec2::new(1920, 1080), video: VideoMode { resolution: None, fullscreen: false }, keys: &[], binds: BINDS.get_or_init(Default::default), capture: None, selected: None }
}

/// Into an Options page from its parent (or the Options flyout).
fn open_page(menu: &mut MenuState, scroll: &mut MenuScroll, settings: &Settings, page: OptionsPage) {
    menu.undo.push(settings.clone());
    menu.page = page;
    menu.capture = None;
    menu.selected = None;
    if page == OptionsPage::Video {
        menu.video = None;
    }
    scroll.0 = 0.0;
}

/// Out of the current Options page to its parent, keeping its changes
/// (Done, Esc) or putting them back (Cancel). Out of a top page leaves
/// Options for the menu it came from. Settings are saved either way.
fn close_page(screen: &mut Screen, menu: &mut MenuState, scroll: &mut MenuScroll, settings: &mut Settings, keep: bool) {
    let video = menu.video.take();
    let before = menu.undo.pop();
    if keep {
        if menu.page == OptionsPage::Video {
            if let Some(v) = video {
                v.apply(settings);
            }
        }
    } else if let Some(b) = before {
        *settings = b;
    }
    settings.save();
    menu.capture = None;
    menu.selected = None;
    scroll.0 = 0.0;
    match menu.page.parent() {
        Some(p) if p != OptionsPage::List => menu.page = p,
        _ => {
            menu.page = OptionsPage::List;
            menu.undo.clear();
            *screen = match *screen {
                Screen::Options { from_pause: true } => Screen::Pause,
                _ => Screen::Main,
            };
        }
    }
}

/// While a key-list row waits, the next key, mouse button or wheel notch
/// becomes its binding; Esc gives up.
#[allow(clippy::too_many_arguments)]
fn capture_key(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    wheel: Option<Res<AccumulatedMouseScroll>>,
    mut menu: ResMut<MenuState>,
    mut eaten: ResMut<InputEaten>,
    mut settings: ResMut<Settings>,
    defaults: Res<DefaultBinds>,
    actions: Res<KeyActions>,
) {
    eaten.0 = false;
    let Some(i) = menu.capture else { return };
    if keys.just_pressed(KeyCode::Escape) {
        menu.capture = None;
        eaten.0 = true;
        return;
    }
    let input = InputState { keys: Some(&keys), mouse: mouse.as_deref(), scroll: wheel.as_deref() };
    let Some(name) = InputButton::all().find(|b| input.just_pressed(*b)).and_then(InputButton::name) else { return };
    if let Some(KeyAction::Command { command, .. }) = actions.0.get(i) {
        let mut table = settings.binds.clone().unwrap_or_else(|| defaults.0.clone());
        table.rebind(command, name);
        info!("portal2: bound {name} to {command}");
        settings.binds = Some(table);
    }
    menu.capture = None;
    eaten.0 = true;
}

/// Esc: closes a confirmation or flyout, backs out of an Options page
/// (keeping its changes), pauses and resumes.
fn escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    eaten: Res<InputEaten>,
    mut screen: ResMut<Screen>,
    mut menu: ResMut<MenuState>,
    mut scroll: ResMut<MenuScroll>,
    map: Option<Res<CurrentMap>>,
    mut settings: ResMut<Settings>,
) {
    if eaten.0 || !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if menu.confirm.is_some() {
        menu.confirm = None;
        return;
    }
    let in_game = map.as_ref().is_some_and(|m| !m.menu);
    match *screen {
        Screen::Options { .. } => close_page(&mut screen, &mut menu, &mut scroll, &mut settings, true),
        Screen::Main | Screen::Pause if menu.flyout.is_some() => menu.flyout = None,
        Screen::Chapters => *screen = Screen::Main,
        Screen::None if in_game => {
            menu.flyout = None;
            *screen = Screen::Pause;
        }
        Screen::Pause => *screen = Screen::None,
        _ => {}
    }
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

/// Mouse wheel over the key list scrolls it.
fn scroll_lists(wheel: Option<Res<AccumulatedMouseScroll>>, menu: Res<MenuState>, mut lists: Query<&mut ScrollPosition, With<ScrollList>>, mut saved: ResMut<MenuScroll>) {
    let Some(wheel) = wheel else { return };
    if wheel.delta.y == 0.0 || menu.capture.is_some() {
        return;
    }
    let dy = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * 32.0,
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for mut pos in &mut lists {
        pos.0.y = (pos.0.y - dy).max(0.0);
        saved.0 = pos.0.y;
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn buttons(
    q: Query<(Entity, &Interaction, &Action, &Look), Changed<Interaction>>,
    (mut texts, mut bgs, children, cursor): (Query<&mut TextColor>, Query<&mut BackgroundColor>, Query<&Children>, Query<&RelativeCursorPosition>),
    (mut screen, mut menu, mut scroll, eaten): (ResMut<Screen>, ResMut<MenuState>, ResMut<MenuScroll>, Res<InputEaten>),
    mut settings: ResMut<Settings>,
    (campaign, defaults, actions): (Res<Campaign>, Res<DefaultBinds>, Res<KeyActions>),
    map: Option<Res<CurrentMap>>,
    (monitors, windows): (Query<&Monitor, With<PrimaryMonitor>>, Query<&Window, With<PrimaryWindow>>),
    mut load: MessageWriter<LoadLevel>,
    mut exit: MessageWriter<AppExit>,
) {
    for (e, interaction, action, look) in &q {
        let hot = *interaction != Interaction::None;
        if let Ok(mut bg) = bgs.get_mut(e) {
            bg.0 = if hot { look.bg_hover } else { look.bg };
        }
        for c in children.iter_descendants(e) {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = if hot { look.text_hover } else { look.text };
            }
        }
        // A press while a key is being bound is the key itself.
        if *interaction != Interaction::Pressed || eaten.0 || menu.capture.is_some() {
            continue;
        }
        info!("portal2: menu {:?}", action);
        let native = native_size(&monitors, &windows);
        match *action {
            Action::SinglePlayer => menu.flyout = if menu.flyout == Some(Flyout::SinglePlayer) { None } else { Some(Flyout::SinglePlayer) },
            Action::Options => menu.flyout = if menu.flyout == Some(Flyout::Options) { None } else { Some(Flyout::Options) },
            Action::Continue => {
                if let Some(m) = read_save() {
                    load.write(LoadLevel { map: m, menu: false });
                    menu.flyout = None;
                    *screen = Screen::None;
                }
            }
            Action::NewGame => {
                menu.flyout = None;
                *screen = Screen::Chapters;
            }
            Action::Chapter(i) => {
                if let Some(c) = campaign.chapters.get(i) {
                    load.write(LoadLevel { map: c.map.clone(), menu: false });
                    *screen = Screen::None;
                }
            }
            Action::Page(p) => {
                if !matches!(*screen, Screen::Options { .. }) {
                    *screen = Screen::Options { from_pause: *screen == Screen::Pause };
                    menu.flyout = None;
                    menu.undo.clear();
                }
                open_page(&mut menu, &mut scroll, &settings, p);
            }
            Action::Back | Action::Done if matches!(*screen, Screen::Options { .. }) => close_page(&mut screen, &mut menu, &mut scroll, &mut settings, true),
            Action::Cancel => close_page(&mut screen, &mut menu, &mut scroll, &mut settings, false),
            Action::Back | Action::Done => *screen = Screen::Main,
            Action::Apply => {
                if let Some(v) = menu.video.take() {
                    v.apply(&mut settings);
                    settings.save();
                }
            }
            Action::UseDefaults => {
                settings.binds = None;
                menu.selected = None;
            }
            Action::ClearKey => {
                if let Some(KeyAction::Command { command, .. }) = menu.selected.and_then(|i| actions.0.get(i)) {
                    let mut table = settings.binds.clone().unwrap_or_else(|| defaults.0.clone());
                    table.clear(command);
                    settings.binds = Some(table);
                }
            }
            Action::Bind(i) => {
                menu.selected = Some(i);
                menu.capture = Some(i);
            }
            Action::Quit => menu.confirm = Some(Confirm::QuitGame),
            Action::MainMenu => menu.confirm = Some(Confirm::QuitToMenu),
            Action::ConfirmCancel => menu.confirm = None,
            Action::ConfirmOk => match menu.confirm.take() {
                Some(Confirm::QuitGame) => {
                    settings.save();
                    exit.write(AppExit::Success);
                }
                Some(Confirm::QuitToMenu) => {
                    load.write(LoadLevel { map: menu_background(&campaign), menu: true });
                    menu.flyout = None;
                    *screen = Screen::Main;
                }
                None => {}
            },
            Action::Resume => *screen = Screen::None,
            Action::Restart => {
                if let Some(m) = &map {
                    load.write(LoadLevel { map: m.info.name.clone(), menu: false });
                }
                *screen = Screen::None;
            }
            Action::Adjust(s, d) => step_setting(&mut menu, &mut settings, native, s, if d < 0.0 { -1 } else { 1 }, false),
            Action::Toggle(s) => step_setting(&mut menu, &mut settings, native, s, 1, true),
            Action::Slide(s) => {
                if let (Some((lo, hi, _)), Some(n)) = (options::slider_range(s), cursor.get(e).ok().and_then(|c| c.normalized)) {
                    options::set_slider(&mut settings, s, lo + (n.x + 0.5).clamp(0.0, 1.0) * (hi - lo));
                }
            }
        }
    }
}

/// One step of an Options value. The Video page's resolution, aspect and
/// display mode collect in [`MenuState::video`] until Apply/Done.
fn step_setting(menu: &mut MenuState, settings: &mut Settings, native: UVec2, s: Setting, dir: i32, wrap: bool) {
    let mut video = menu.video.unwrap_or_else(|| VideoMode::of(settings));
    options::adjust(settings, &mut video, native, s, dir, wrap);
    if matches!(s, Setting::Resolution | Setting::Aspect | Setting::DisplayMode) {
        menu.video = Some(video);
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
    fn menu_font_and_logo_choice() {
        let fonts: Vec<String> = ["resource/halflife2.ttf", "resource/linux_fonts/dejavusans.ttf", "resource/universlt57cn.ttf", "resource/universlt67boldcn.ttf", "resource/dinpro.otf"]
            .map(String::from)
            .to_vec();
        assert_eq!(pick_menu_font(&fonts).map(String::as_str), Some("resource/universlt67boldcn.ttf"));
        assert_eq!(pick_menu_font(&fonts[..2]), None);
        let vgui: Vec<String> = ["materials/vgui/portal2logo_coop.vtf", "materials/vgui/portal2logo.vtf", "materials/vgui/portal2logo_english.vtf", "materials/vgui/steamlogo.vtf"]
            .map(String::from)
            .to_vec();
        assert_eq!(pick_logo(&vgui).map(String::as_str), Some("materials/vgui/portal2logo_english.vtf"));
        assert_eq!(pick_logo(&vgui[3..]), None);
    }

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
