//! Menus and HUD text, laid out like Portal 2's PC menus: the main menu
//! (logo at the top left, items stacked at the lower left, Play Single
//! Player and Options opening flyouts beside them) over the game's own
//! act's background movie (the still until it plays) and menu music, the New Game chapter dialog with the
//! game's chapter pictures and titles, the pause menu, the Options dialogs
//! (rows and footer buttons; content in options.rs), quit confirmations,
//! the Load Game and Save Game lists (saves themselves are `saves.rs`) and
//! chapter title cards.
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
use crate::community::CommunityMaps;
use crate::level::{CurrentMap, LevelLoaded, LoadLevel};
use crate::mods_page::ModsPage;
use crate::mounts::Mounts;
use crate::saves::{list_saves, LoadGame, Progress, SaveGame, SaveInfo, Slot};
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
            .init_resource::<ChapterTab>()
            .init_resource::<ModsPage>()
            .init_resource::<MenuMovie>()
            .add_message::<ShowChapterTitle>()
            .add_message::<MenuClick>()
            .add_systems(Startup, (load_fonts, load_menu_art, open_main_menu).chain().after(crate::campaign::load_campaign))
            .add_systems(
                Update,
                (
                    (capture_key, escape_key, scripted_clicks, buttons, scroll_lists, rebuild_menu, hover_rows, menu_movie).chain(),
                    (menu_view, menu_camera, pause_world, menu_music, title_card).chain(),
                )
                    .chain()
                    .in_set(crate::level::GameSet::Logic),
            );
    }
}

/// The background movie's image node (see [`menu_movie`]).
#[derive(Component)]
struct MenuMovieNode;

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

/// The saved-game lists over the main or pause menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavesDialog {
    Load,
    Save,
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
    /// Load Game / Save Game, and the saves it lists (newest first).
    saves: Option<SavesDialog>,
    save_list: Vec<SaveInfo>,
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

/// The New Game dialog's tab: the campaign, or the community maps (the
/// tabs only show when there are community maps), and the page of those.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ChapterTab {
    community: bool,
    page: usize,
    /// The chapter picked in the list (its picture shows beside it).
    chapter: usize,
}

/// Community maps per page of the New Game dialog.
const COMMUNITY_PER_PAGE: usize = 9;

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
    /// A row of New Game's chapter list: pick it (a second click starts it).
    SelectChapter(usize),
    Chapter(usize),
    /// Main menu items not built yet: they do nothing.
    Coop,
    CommunityChambers,
    Extras,
    ChallengeMode,
    /// Open Load Game / Save Game.
    LoadGame,
    SaveGame,
    /// A row of Load Game.
    LoadSave(usize),
    /// A row of Save Game: an existing save to overwrite, or a new one.
    SaveSlot(Option<usize>),
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
    /// New Game's tabs: the campaign (false) or the community maps (true).
    ChapterTab(bool),
    /// The previous (-1) or next (1) page of community maps.
    CommunityPage(i32),
    CommunityMap(usize),
    /// Options > Mods: a mod on/off, and undo the last change.
    ModToggle(usize),
    ModRollback,
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

/// Portal 2's menu colours. Main menu items: light grey that goes white
/// over a grey bar. Dialogs and flyouts: light grey panels (a raised title
/// tab at the top left) with near-black text; the hovered row is a dark
/// bar with white text; footer buttons are small dark buttons.
mod theme {
    use bevy::prelude::Color;

    pub const ITEM: Color = Color::srgba(0.8, 0.81, 0.82, 0.95);
    pub const ITEM_HOVER: Color = Color::WHITE;
    pub const HIGHLIGHT: Color = Color::srgba(0.55, 0.56, 0.58, 0.55);
    pub const ROW_HOVER: Color = Color::srgb(0.2, 0.2, 0.21);
    pub const SELECTED: Color = Color::srgb(0.24, 0.24, 0.25);
    pub const TITLE: Color = Color::srgb(0.05, 0.05, 0.06);
    pub const LABEL: Color = Color::srgb(0.12, 0.12, 0.13);
    pub const VALUE: Color = Color::srgb(0.12, 0.12, 0.13);
    pub const HEADING: Color = Color::srgb(0.38, 0.39, 0.41);
    pub const HINT: Color = Color::srgb(0.36, 0.37, 0.39);
    pub const PANEL: Color = Color::srgba(0.86, 0.86, 0.87, 0.97);
    pub const EDGE: Color = Color::srgba(0.0, 0.0, 0.0, 0.18);
    pub const TRACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.18);
    pub const FILL: Color = Color::srgb(0.22, 0.22, 0.23);
    pub const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
    pub const BUTTON: Color = Color::srgb(0.24, 0.24, 0.25);
    pub const BUTTON_HOVER: Color = Color::srgb(0.38, 0.38, 0.4);
    pub const BUTTON_TEXT: Color = Color::srgb(0.88, 0.88, 0.89);
    /// A slider on the hovered (dark) row.
    pub const TRACK_HOT: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
    pub const FILL_HOT: Color = Color::srgb(0.9, 0.9, 0.91);
    /// A problem line (the Mods page).
    pub const ERROR: Color = Color::srgb(0.62, 0.12, 0.1);
    /// The chosen chapter-dialog tab.
    pub const TAB_ON: Color = Color::srgb(0.12, 0.12, 0.13);
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
/// Rows and items on a light dialog panel.
const VALUE_LOOK: Look = Look { text: theme::VALUE, text_hover: Color::WHITE, bg: Color::NONE, bg_hover: theme::ROW_HOVER };
/// BACK / DONE / USE DEFAULTS.
const BUTTON_LOOK: Look = Look { text: theme::BUTTON_TEXT, text_hover: Color::WHITE, bg: theme::BUTTON, bg_hover: theme::BUTTON_HOVER };

/// The main menu, as in Portal 2 (Co-op, Extras and Challenge Mode are
/// not built yet and do nothing).
const MAIN_ITEMS: [(&str, Action, Option<Flyout>); 6] = [
    ("PLAY SINGLE PLAYER", Action::SinglePlayer, Some(Flyout::SinglePlayer)),
    ("PLAY COOPERATIVE GAME", Action::Coop, None),
    ("COMMUNITY TEST CHAMBERS", Action::CommunityChambers, None),
    ("OPTIONS", Action::Options, Some(Flyout::Options)),
    ("EXTRAS", Action::Extras, None),
    ("QUIT", Action::Quit, None),
];

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
    // The install has no DIN; macOS ships DIN Alternate, the closest
    // match to Portal 2's menu face.
    let menu = menu.or_else(|| {
        let p = "/System/Library/Fonts/Supplemental/DIN Alternate Bold.ttf";
        std::fs::read(p).ok().map(|b| (p, fonts.add(Font::from_bytes(b))))
    });
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

/// The map the newest save is on (Continue Game's).
fn read_save() -> Option<String> {
    crate::saves::latest_map()
}

/// The map in `save.txt`, the autosave from before `saves/` existed.
pub(crate) fn read_legacy_save() -> Option<String> {
    std::fs::read_to_string(data_dir().join("save.txt")).ok().and_then(|s| parse_save(&s))
}

/// `save.txt` for `map` (still written, for older builds).
pub(crate) fn legacy_save_text(map: &str) -> String {
    save_text(map)
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
                            f.spawn(button(&fonts.menu, *label, *action, ITEM_SIZE * 0.8, VALUE_LOOK));
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
    dialog_on(commands, fonts, title, width, 10, Color::NONE, false)
}

/// [`dialog`] on layer `z` over `shade`, at the left or `centered`.
fn dialog_on(commands: &mut Commands, fonts: &UiFonts, title: &str, width: f32, z: i32, shade: Color, centered: bool) -> Entity {
    let mut root = commands.spawn((layer(z), BackgroundColor(shade)));
    root.insert(Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: if centered { AlignItems::Center } else { AlignItems::FlexStart },
        padding: if centered { UiRect::ZERO } else { UiRect::left(Val::Percent(6.0)) },
        ..default()
    });
    let mut panel = Entity::PLACEHOLDER;
    root.with_children(|r| {
        // Portal 2's dialog: the title sits on a tab rising from the panel's
        // top left, the same light grey.
        r.spawn((
            Node { align_self: AlignSelf::FlexStart, padding: UiRect::new(Val::Px(16.0), Val::Px(36.0), Val::Px(14.0), Val::Px(10.0)), ..default() },
            BackgroundColor(theme::PANEL),
            children![text(&fonts.menu, 36.0, title.to_uppercase(), theme::TITLE)],
        ));
        panel = r
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(width),
                    max_width: Val::Percent(88.0),
                    max_height: Val::Percent(86.0),
                    padding: UiRect::new(Val::Px(16.0), Val::Px(16.0), Val::Px(18.0), Val::Px(16.0)),
                    row_gap: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(theme::PANEL),
            ))
            .id();
    });
    panel
}

/// The bottom of a dialog: a hint at the left, buttons at the right.
fn footer(p: &mut ChildSpawnerCommands, fonts: &UiFonts, hint: &str, buttons: &[Footer]) {
    let buttons: Vec<(&str, Action)> = buttons.iter().map(|&k| (k.label(), k.into())).collect();
    footer_buttons(p, fonts, hint, &buttons);
}

/// [`footer`] with any buttons.
fn footer_buttons(p: &mut ChildSpawnerCommands, fonts: &UiFonts, hint: &str, buttons: &[(&str, Action)]) {
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
            for &(label, action) in buttons {
                b.spawn(button(&fonts.menu, label, action, 16.0, BUTTON_LOOK));
            }
        });
    });
}

/// An Options row. As in Portal 2, only the row under the mouse shows its
/// ◀ ▶ arrows, on a dark bar with white text ([`hover_rows`]).
#[derive(Component, Default)]
struct HoverRow {
    hot: bool,
}

/// A row's ◀ ▶ arrows, hidden until the row is hovered.
#[derive(Component)]
struct RowArrow;

/// A button inside a [`HoverRow`]: the row sets its colours.
#[derive(Component)]
struct InRow;

/// A slider's track and fill, which change colour on the dark bar.
#[derive(Component)]
struct SliderTrack;

#[derive(Component)]
struct SliderFill;

/// The row container: hovered via [`RelativeCursorPosition`] so the
/// buttons on it can still take the clicks.
fn hover_row() -> impl Bundle {
    (
        HoverRow::default(),
        RelativeCursorPosition::default(),
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(32.0), column_gap: Val::Px(6.0), padding: UiRect::horizontal(Val::Px(6.0)), ..default() },
        BackgroundColor(Color::NONE),
    )
}

/// A ◀ or ▶ on a row.
fn arrow(fonts: &UiFonts, left: bool, action: Action) -> impl Bundle {
    (
        Button,
        action,
        VALUE_LOOK,
        InRow,
        RowArrow,
        Visibility::Hidden,
        Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)), align_items: AlignItems::Center, ..default() },
        BackgroundColor(Color::NONE),
        children![text(&fonts.bold, 13.0, if left { "\u{25C0}" } else { "\u{25B6}" }, theme::VALUE)],
    )
}

/// The value at a row's right; clicking it steps on (wrapping).
fn row_value(fonts: &UiFonts, value: &str, action: Action) -> impl Bundle {
    (
        Button,
        action,
        VALUE_LOOK,
        InRow,
        Node { width: Val::Px(250.0), justify_content: JustifyContent::FlexEnd, padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(4.0), Val::Px(4.0)), ..default() },
        BackgroundColor(Color::NONE),
        children![text(&fonts.regular, 18.0, value.to_string(), theme::VALUE)],
    )
}

fn row_label(r: &mut ChildSpawnerCommands, fonts: &UiFonts, s: &str) {
    r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.regular, 18.0, s.to_string(), theme::LABEL)]));
}

/// `label  ◀ value ▶` with the arrows and value doing `back`, `on`, `click`.
fn stepper_row(p: &mut ChildSpawnerCommands, fonts: &UiFonts, label: &str, value: &str, back: Action, on: Action, click: Action) {
    p.spawn(hover_row()).with_children(|r| {
        row_label(r, fonts, label);
        r.spawn(arrow(fonts, true, back));
        r.spawn(row_value(fonts, value, click));
        r.spawn(arrow(fonts, false, on));
    });
}

/// One Options row.
fn option_row(p: &mut ChildSpawnerCommands, fonts: &UiFonts, row: &Row) {
    match row {
        Row::Heading(h) => {
            p.spawn((Node { margin: UiRect::new(Val::Px(6.0), Val::Px(0.0), Val::Px(12.0), Val::Px(2.0)), ..default() }, children![text(&fonts.bold, 13.0, h.to_uppercase(), theme::HEADING)]));
        }
        Row::Choice { label, value, setting } => {
            stepper_row(p, fonts, label, value, Action::Adjust(*setting, -1.0), Action::Adjust(*setting, 1.0), Action::Toggle(*setting));
        }
        Row::Mod { label, value, index } => {
            stepper_row(p, fonts, label, value, Action::ModToggle(*index), Action::ModToggle(*index), Action::ModToggle(*index));
        }
        Row::Rollback { value } => {
            p.spawn(hover_row()).with_children(|r| {
                row_label(r, fonts, "Undo last change");
                r.spawn(row_value(fonts, value, Action::ModRollback));
            });
        }
        Row::Note { text: t, error } => {
            let color = if *error { theme::ERROR } else { theme::HINT };
            p.spawn((Node { margin: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(6.0), Val::Px(2.0)), ..default() }, children![text(&fonts.regular, 14.0, t.clone(), color)]));
        }
        Row::Slider { label, value, fraction, setting } => {
            p.spawn(hover_row()).with_children(|r| {
                row_label(r, fonts, label);
                r.spawn(arrow(fonts, true, Action::Adjust(*setting, -1.0)));
                r.spawn((Node { width: Val::Px(250.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() },))
                    .with_children(|v| {
                        v.spawn((
                            Button,
                            Action::Slide(*setting),
                            Look { text: theme::VALUE, text_hover: theme::ITEM_HOVER, bg: Color::NONE, bg_hover: Color::NONE },
                            InRow,
                            RelativeCursorPosition::default(),
                            Node { flex_grow: 1.0, height: Val::Px(24.0), align_items: AlignItems::Center, ..default() },
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|t| {
                            t.spawn((SliderTrack, Node { width: Val::Percent(100.0), height: Val::Px(6.0), ..default() }, BackgroundColor(theme::TRACK)))
                                .with_child((SliderFill, Node { width: Val::Percent(fraction * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(theme::FILL)));
                        });
                        v.spawn((Node { width: Val::Px(52.0), justify_content: JustifyContent::FlexEnd, ..default() }, children![text(&fonts.regular, 16.0, value.clone(), theme::VALUE)]));
                    });
                r.spawn(arrow(fonts, false, Action::Adjust(*setting, 1.0)));
            });
        }
        Row::Link { label: l, page } => {
            p.spawn((
                Button,
                Action::Page(*page),
                VALUE_LOOK,
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(32.0), padding: UiRect::horizontal(Val::Px(6.0)), ..default() },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|r| {
                r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.menu, 18.0, l.to_uppercase(), theme::VALUE)]));
            });
        }
        Row::Key { label: l, keys, index, selected } => {
            let (bg, color) = if *selected { (theme::SELECTED, Color::WHITE) } else { (Color::NONE, theme::VALUE) };
            p.spawn((
                Button,
                Action::Bind(*index),
                Look { text: color, text_hover: Color::WHITE, bg, bg_hover: theme::ROW_HOVER },
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, min_height: Val::Px(28.0), padding: UiRect::horizontal(Val::Px(8.0)), ..default() },
                BackgroundColor(bg),
            ))
            .with_children(|r| {
                r.spawn((Node { flex_grow: 1.0, ..default() }, children![text(&fonts.regular, 17.0, l.clone(), color)]));
                r.spawn((Node { width: Val::Px(220.0), ..default() }, children![text(&fonts.bold, 17.0, keys.clone(), color)]));
            });
        }
    }
}

/// The row under the mouse: dark bar, white text, arrows shown; the
/// others plain.
#[allow(clippy::type_complexity)]
fn hover_rows(
    mut rows: Query<(Entity, &RelativeCursorPosition, &mut HoverRow, &mut BackgroundColor), Without<SliderTrack>>,
    children: Query<&Children>,
    mut texts: Query<&mut TextColor>,
    mut arrows: Query<&mut Visibility, With<RowArrow>>,
    mut parts: Query<(&mut BackgroundColor, Has<SliderFill>), (Or<(With<SliderTrack>, With<SliderFill>)>, Without<HoverRow>)>,
    menu: Res<MenuState>,
) {
    for (e, cursor, mut row, mut bg) in &mut rows {
        // Under a confirmation, nothing is hovered.
        let hot = cursor.cursor_over() && menu.confirm.is_none();
        if hot == row.hot {
            continue;
        }
        row.hot = hot;
        bg.0 = if hot { theme::ROW_HOVER } else { Color::NONE };
        for c in children.iter_descendants(e) {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = if hot { Color::WHITE } else { theme::VALUE };
            }
            if let Ok(mut v) = arrows.get_mut(c) {
                *v = if hot { Visibility::Inherited } else { Visibility::Hidden };
            }
            if let Ok((mut b, fill)) = parts.get_mut(c) {
                b.0 = match (fill, hot) {
                    (true, true) => theme::FILL_HOT,
                    (true, false) => theme::FILL,
                    (false, true) => theme::TRACK_HOT,
                    (false, false) => theme::TRACK,
                };
            }
        }
    }
}

/// CAMPAIGN / COMMUNITY MAPS above the New Game grid: small dark buttons,
/// the chosen one darker.
fn chapter_tabs(p: &mut ChildSpawnerCommands, fonts: &UiFonts, community: bool) {
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), margin: UiRect::bottom(Val::Px(10.0)), ..default() }).with_children(|r| {
        for (label, c) in [("CAMPAIGN", false), ("COMMUNITY MAPS", true)] {
            let look = if c == community { Look { bg: theme::TAB_ON, bg_hover: theme::TAB_ON, text: Color::WHITE, text_hover: Color::WHITE } } else { BUTTON_LOOK };
            r.spawn(button(&fonts.menu, label, Action::ChapterTab(c), 16.0, look));
        }
    });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn rebuild_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    menu: Res<MenuState>,
    scroll: Res<MenuScroll>,
    (fonts, art, campaign, settings, actions, defaults): (Option<Res<UiFonts>>, Option<Res<MenuArt>>, Res<Campaign>, Res<Settings>, Res<KeyActions>, Res<DefaultBinds>),
    (tab, community, mods, progress): (Res<ChapterTab>, Res<CommunityMaps>, Res<ModsPage>, Res<Progress>),
    roots: Query<Entity, With<MenuRoot>>,
    game: Res<GameFsRes>,
    mut images: ResMut<Assets<Image>>,
    (monitors, windows): (Query<&Monitor, With<PrimaryMonitor>>, Query<&Window, With<PrimaryWindow>>),
    mut chapter_pics: Local<Vec<Option<Handle<Image>>>>,
    mut backgrounds: Local<std::collections::HashMap<usize, Option<Handle<Image>>>>,
    movie: Res<MenuMovie>,
) {
    let Some(fonts) = fonts else { return };
    let options_changed = (settings.is_changed() || actions.is_changed() || mods.is_changed()) && matches!(*screen, Screen::Options { .. });
    let chapters_changed = (tab.is_changed() || community.is_changed()) && *screen == Screen::Chapters;
    if !screen.is_changed() && !menu.is_changed() && !options_changed && !chapters_changed && !fonts.is_added() {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    // Out of the game, the menu sits on the game's own menu still for the
    // act of the saved chapter; in the game, the view shows through dimmed.
    let in_game = matches!(*screen, Screen::Pause | Screen::Options { from_pause: true });
    if screen.shows_backdrop() {
        let act = menu_act(&campaign);
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
        // The act's movie over the still, shown once its frames arrive.
        bg.with_child((
            MenuMovieNode,
            ImageNode { image: movie.image.clone(), image_mode: bevy::ui::widget::NodeImageMode::Stretch, ..default() },
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            if movie.playing() { Visibility::Inherited } else { Visibility::Hidden },
        ));
    } else if in_game {
        commands.spawn((layer(9), BackgroundColor(theme::SHADE)));
    }
    // Chapter pictures: New Game's tiles and the saved-game lists.
    if (*screen == Screen::Chapters || menu.saves.is_some()) && chapter_pics.len() != campaign.chapters.len() {
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
    let options_flyout: Vec<(&str, Action)> = options::rows(OptionsPage::List, &settings, &dummy_ctx())
        .into_iter()
        .filter_map(|r| match r {
            Row::Link { page, .. } => Some((page.title(), Action::Page(page))),
            _ => None,
        })
        .collect();
    match *screen {
        Screen::None => {}
        Screen::Main | Screen::Pause if menu.saves.is_some() => {
            let save = menu.saves == Some(SavesDialog::Save);
            let panel = dialog(&mut commands, &fonts, if save { "SAVE GAME" } else { "LOAD GAME" }, 760.0);
            commands.entity(panel).with_children(|p| {
                p.spawn((ScrollList, ScrollPosition(Vec2::ZERO), Node { flex_direction: FlexDirection::Column, overflow: Overflow::scroll_y(), max_height: Val::Vh(56.0), ..default() }))
                    .with_children(|l| {
                        if save {
                            l.spawn((Button, Action::SaveSlot(None), VALUE_LOOK, save_row_node(), BackgroundColor(Color::NONE)))
                                .with_children(|r| {
                                    r.spawn((Node { width: Val::Px(128.0), height: Val::Px(72.0), ..default() }, BackgroundColor(theme::EDGE)));
                                    r.spawn(text(&fonts.menu, 18.0, "NEW SAVED GAME", theme::VALUE));
                                });
                        }
                        let shown: Vec<(usize, &SaveInfo)> = menu.save_list.iter().enumerate().filter(|(_, s)| !save || s.header.kind == "save").collect();
                        if shown.is_empty() && !save {
                            l.spawn(text(&fonts.regular, 16.0, "No saved games.", theme::LABEL));
                        }
                        for (i, s) in shown {
                            let action = if save { Action::SaveSlot(Some(i)) } else { Action::LoadSave(i) };
                            let chapter = campaign.chapter_of(&s.header.map);
                            l.spawn((Button, action, VALUE_LOOK, save_row_node(), BackgroundColor(Color::NONE))).with_children(|r| {
                                match chapter.and_then(|c| chapter_pics.get(c).cloned().flatten()) {
                                    Some(img) => {
                                        r.spawn((ImageNode::new(img), Node { width: Val::Px(128.0), height: Val::Px(72.0), ..default() }));
                                    }
                                    None => {
                                        r.spawn((Node { width: Val::Px(128.0), height: Val::Px(72.0), ..default() }, BackgroundColor(theme::EDGE)));
                                    }
                                }
                                let (title, subtitle) = chapter.and_then(|c| campaign.chapters.get(c)).map_or((String::new(), String::new()), |c| (c.title.to_uppercase(), c.subtitle.to_uppercase()));
                                r.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
                                    c.spawn(text(&fonts.regular, 13.0, title, theme::LABEL));
                                    c.spawn(text(&fonts.menu, 18.0, if subtitle.is_empty() { s.header.map.to_uppercase() } else { subtitle }, theme::VALUE));
                                    c.spawn(text(&fonts.regular, 13.0, s.header.map.clone(), theme::HINT));
                                });
                                r.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, row_gap: Val::Px(2.0), ..default() }).with_children(|c| {
                                    c.spawn(text(&fonts.regular, 13.0, crate::saves::time_text(s.header.saved_at), theme::LABEL));
                                    c.spawn(text(&fonts.regular, 13.0, save_kind_label(&s.header.kind, &s.name), theme::HINT));
                                });
                            });
                        }
                    });
                footer(p, &fonts, "", &[Footer::Back]);
            });
        }
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
                // As in Portal 2: Continue and Load Game once there is a save.
                let sp: Vec<(&str, Action)> = if read_save().is_some() {
                    vec![("CONTINUE GAME", Action::Continue), ("NEW GAME", Action::NewGame), ("LOAD GAME", Action::LoadGame), ("CHALLENGE MODE", Action::ChallengeMode)]
                } else {
                    vec![("NEW GAME", Action::NewGame), ("CHALLENGE MODE", Action::ChallengeMode)]
                };
                let fly = if menu.flyout == Some(Flyout::Options) { &options_flyout } else { &sp };
                item_list(
                    p,
                    &fonts,
                    &MAIN_ITEMS,
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
                        ("SAVE GAME", Action::SaveGame, None),
                        ("LOAD GAME", Action::LoadGame, None),
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
            let show_community = tab.community && !community.maps.is_empty();
            let panel = dialog(&mut commands, &fonts, "NEW GAME", 980.0);
            commands.entity(panel).with_children(|p| {
                if !community.maps.is_empty() {
                    chapter_tabs(p, &fonts, show_community);
                }
                let grid = Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                    column_gap: Val::Px(14.0),
                    row_gap: Val::Px(10.0),
                    width: Val::Percent(100.0),
                    ..default()
                };
                let tile = Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(6.0)), row_gap: Val::Px(2.0), overflow: Overflow::clip(), ..default() };
                if show_community {
                    let pages = community.maps.len().div_ceil(COMMUNITY_PER_PAGE);
                    let page = tab.page.min(pages - 1);
                    p.spawn(grid).with_children(|g| {
                        for (i, m) in community.maps.iter().enumerate().skip(page * COMMUNITY_PER_PAGE).take(COMMUNITY_PER_PAGE) {
                            let folder = m.path.parent().map(|f| f.display().to_string()).unwrap_or_default();
                            g.spawn((Button, Action::CommunityMap(i), VALUE_LOOK, tile.clone(), BackgroundColor(Color::NONE))).with_children(|b| {
                                b.spawn(text(&fonts.menu, 17.0, m.name.to_uppercase(), theme::VALUE));
                                b.spawn(text(&fonts.regular, 12.0, folder, theme::LABEL));
                            });
                        }
                    });
                    if pages > 1 {
                        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: Val::Px(12.0), margin: UiRect::top(Val::Px(8.0)), ..default() })
                            .with_children(|r| {
                                r.spawn(button(&fonts.bold, "\u{25C0}", Action::CommunityPage(-1), 13.0, VALUE_LOOK));
                                r.spawn(text(&fonts.menu, 16.0, format!("PAGE {} / {pages}", page + 1), theme::VALUE));
                                r.spawn(button(&fonts.bold, "\u{25B6}", Action::CommunityPage(1), 13.0, VALUE_LOOK));
                            });
                    }
                } else {
                    // As in Portal 2's New Game: the chapter list on the left,
                    // the picked chapter's picture and title on the right.
                    let sel = tab.chapter.min(campaign.chapters.len().saturating_sub(1));
                    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(24.0), width: Val::Percent(100.0), ..default() }).with_children(|r| {
                        r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Percent(44.0), row_gap: Val::Px(1.0), ..default() }).with_children(|list| {
                            for (i, c) in campaign.chapters.iter().enumerate() {
                                let row = Node { padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)), ..default() };
                                let label = format!("{}: {}", c.title.to_uppercase(), c.subtitle.to_uppercase());
                                // Chapters not reached yet: greyed, not clickable.
                                if !progress.is_unlocked(i) {
                                    list.spawn(row).with_child(text(&fonts.menu, 17.0, label, theme::HINT));
                                } else if i == sel {
                                    let look = Look { text: Color::WHITE, text_hover: Color::WHITE, bg: theme::SELECTED, bg_hover: theme::ROW_HOVER };
                                    list.spawn((Button, Action::SelectChapter(i), look, row, BackgroundColor(theme::SELECTED))).with_child(text(&fonts.menu, 17.0, label, Color::WHITE));
                                } else {
                                    list.spawn((Button, Action::SelectChapter(i), VALUE_LOOK, row, BackgroundColor(Color::NONE))).with_child(text(&fonts.menu, 17.0, label, theme::VALUE));
                                }
                            }
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: Val::Px(6.0), ..default() }).with_children(|pic| {
                            match chapter_pics.get(sel) {
                                Some(Some(img)) => {
                                    pic.spawn((ImageNode::new(img.clone()), Node { width: Val::Percent(100.0), aspect_ratio: Some(16.0 / 9.0), ..default() }));
                                }
                                _ => {
                                    pic.spawn((Node { width: Val::Percent(100.0), aspect_ratio: Some(16.0 / 9.0), ..default() }, BackgroundColor(theme::EDGE)));
                                }
                            }
                            if let Some(c) = campaign.chapters.get(sel) {
                                pic.spawn(text(&fonts.regular, 14.0, c.title.to_uppercase(), theme::LABEL));
                                pic.spawn(text(&fonts.menu, 20.0, c.subtitle.to_uppercase(), theme::VALUE));
                            }
                        });
                    });
                    footer_buttons(p, &fonts, "", &[("START NEW GAME", Action::Chapter(sel)), (Footer::Back.label(), Action::Back)]);
                    return;
                }
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
                mods: Some(&mods),
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
                if matches!(page, OptionsPage::Keys | OptionsPage::Mods) {
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
        // Centred, so its tab and panel line up as one block.
        let panel = dialog_on(&mut commands, &fonts, title, 560.0, 20, theme::SHADE, true);
        commands.entity(panel).with_children(|d| {
            d.spawn((Node { margin: UiRect::horizontal(Val::Px(6.0)), ..default() }, children![text(&fonts.regular, 18.0, message, theme::LABEL)]));
            footer_buttons(d, &fonts, "", &[("OK", Action::ConfirmOk), ("CANCEL", Action::ConfirmCancel)]);
        });
    }
}

/// A Load Game / Save Game row: picture, chapter and map, then time and kind.
fn save_row_node() -> Node {
    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), padding: UiRect::all(Val::Px(6.0)), ..default() }
}

/// "Autosave", "Quick save" or the save's name.
fn save_kind_label(kind: &str, name: &str) -> String {
    match kind {
        "autosave" => "Autosave".into(),
        "quick" => "Quick save".into(),
        _ => name.to_string(),
    }
}

/// For rows that need no context (the category list).
fn dummy_ctx() -> Ctx<'static> {
    static BINDS: std::sync::OnceLock<crate::bindings::BindTable> = std::sync::OnceLock::new();
    Ctx { native: UVec2::new(1920, 1080), video: VideoMode { resolution: None, fullscreen: false }, keys: &[], binds: BINDS.get_or_init(Default::default), capture: None, selected: None, mods: None }
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
    if menu.saves.is_some() {
        menu.saves = None;
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

/// Older click names that still work: (old, now).
const CLICK_ALIASES: [(&str, &str); 4] = [("chapters", "newgame"), ("speedrunoptions", "page(speedrun)"), ("modsoptions", "page(mods)"), ("subtitleoptions", "page(subtitles)")];

fn scripted_clicks(mut clicks: MessageReader<MenuClick>, mut screen: ResMut<Screen>, mut q: Query<(&mut Interaction, &Action)>) {
    for c in clicks.read() {
        let want = c.0.replace(' ', "").to_ascii_lowercase();
        let want = CLICK_ALIASES.iter().find(|(old, _)| *old == want).map_or(want.clone(), |(_, now)| now.to_string());
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
    q: Query<(Entity, &Interaction, &Action, &Look, Has<InRow>), Changed<Interaction>>,
    (mut texts, mut bgs, children, cursor): (Query<&mut TextColor>, Query<&mut BackgroundColor>, Query<&Children>, Query<&RelativeCursorPosition>),
    (mut screen, mut menu, mut scroll, eaten): (ResMut<Screen>, ResMut<MenuState>, ResMut<MenuScroll>, Res<InputEaten>),
    mut settings: ResMut<Settings>,
    (campaign, defaults, actions): (Res<Campaign>, Res<DefaultBinds>, Res<KeyActions>),
    map: Option<Res<CurrentMap>>,
    (monitors, windows): (Query<&Monitor, With<PrimaryMonitor>>, Query<&Window, With<PrimaryWindow>>),
    (mut tab, mut community, mut mods, mounts): (ResMut<ChapterTab>, ResMut<CommunityMaps>, ResMut<ModsPage>, Res<Mounts>),
    mut load: MessageWriter<LoadLevel>,
    mut exit: MessageWriter<AppExit>,
    (mut load_game, mut save_game): (MessageWriter<LoadGame>, MessageWriter<SaveGame>),
) {
    for (e, interaction, action, look, in_row) in &q {
        let hot = *interaction != Interaction::None;
        // A row's own buttons take its colours (see hover_rows).
        if !in_row {
            if let Ok(mut bg) = bgs.get_mut(e) {
                bg.0 = if hot { look.bg_hover } else { look.bg };
            }
            for c in children.iter_descendants(e) {
                if let Ok(mut t) = texts.get_mut(c) {
                    t.0 = if hot { look.text_hover } else { look.text };
                }
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
                if read_save().is_some() {
                    load_game.write(LoadGame(Slot::Latest(None)));
                    menu.flyout = None;
                    *screen = Screen::None;
                }
            }
            Action::LoadGame | Action::SaveGame => {
                menu.flyout = None;
                menu.save_list = list_saves();
                menu.saves = Some(if *action == Action::SaveGame { SavesDialog::Save } else { SavesDialog::Load });
            }
            Action::LoadSave(i) => {
                if let Some(s) = menu.save_list.get(i) {
                    load_game.write(LoadGame(Slot::Named(s.name.clone())));
                    menu.saves = None;
                    *screen = Screen::None;
                }
            }
            Action::SaveSlot(i) => {
                let slot = match i.and_then(|i| menu.save_list.get(i)) {
                    Some(s) => Slot::Named(s.name.clone()),
                    None => Slot::New,
                };
                save_game.write(SaveGame(slot));
                menu.saves = None;
            }
            Action::NewGame => {
                menu.flyout = None;
                *screen = Screen::Chapters;
                // Maps copied in since the game started show up too.
                community.rescan();
                *tab = ChapterTab::default();
            }
            Action::ChapterTab(c) => *tab = ChapterTab { community: c, page: 0, chapter: tab.chapter },
            Action::SelectChapter(i) if tab.chapter == i => {
                if let Some(c) = campaign.chapters.get(i) {
                    load.write(LoadLevel { map: c.map.clone(), menu: false });
                    *screen = Screen::None;
                }
            }
            Action::SelectChapter(i) => tab.chapter = i,
            Action::CommunityChambers => {
                community.rescan();
                if !community.maps.is_empty() {
                    menu.flyout = None;
                    *screen = Screen::Chapters;
                    *tab = ChapterTab { community: true, ..default() };
                }
            }
            Action::Coop | Action::Extras | Action::ChallengeMode => {}
            Action::CommunityPage(d) => {
                let pages = community.maps.len().div_ceil(COMMUNITY_PER_PAGE).max(1);
                tab.page = (tab.page as i32 + d).clamp(0, pages as i32 - 1) as usize;
            }
            Action::CommunityMap(i) => {
                if let Some(m) = community.maps.get(i) {
                    load.write(LoadLevel { map: m.launch_name(), menu: false });
                    *screen = Screen::None;
                }
            }
            Action::ModToggle(i) => mods.toggle(i, &mounts.mods),
            Action::ModRollback => mods.rollback(&mounts.mods),
            Action::Adjust(Setting::ModProfile, d) => mods.step_profile(if d < 0.0 { -1 } else { 1 }, &mounts.mods),
            Action::Toggle(Setting::ModProfile) => {
                // Clicking the name steps on, round to the first after the last.
                let last = mods.profiles.last().is_some_and(|p| *p == mods.profile);
                let d = if last { -(mods.profiles.len() as i32) } else { 1 };
                mods.step_profile(d, &mounts.mods);
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
                if p == OptionsPage::Mods {
                    mods.note.clear();
                    mods.refresh(&mounts.mods);
                }
                open_page(&mut menu, &mut scroll, &settings, p);
            }
            Action::Back if menu.saves.is_some() => menu.saves = None,
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
    // The world stays stopped in every menu opened from the pause menu.
    if matches!(*screen, Screen::Pause | Screen::Options { from_pause: true }) {
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

impl Screen {
    /// Out of the game, menus sit on the act's background movie.
    fn shows_backdrop(self) -> bool {
        matches!(self, Screen::Main | Screen::Chapters | Screen::Options { from_pause: false })
    }
}

/// The act (1..=5) of a chapter (index), for the menu's movie, still and music.
fn act_of_chapter(chapter: usize) -> usize {
    [1, 2, 2, 2, 3, 4, 4, 5, 5].get(chapter).copied().unwrap_or(1)
}

/// The act of the newest save's chapter (act 1 without a save).
fn menu_act(campaign: &Campaign) -> usize {
    act_of_chapter(read_save().and_then(|m| campaign.chapter_of(&m)).unwrap_or(0))
}

/// The menu's background movie (`media/menu_act0N.bik`), streamed by ffmpeg
/// looping in real time; only the newest frame is kept.
#[derive(Resource)]
struct MenuMovie {
    image: Handle<Image>,
    act: usize,
    child: Option<std::process::Child>,
    latest: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    size: UVec2,
    shown: bool,
}

impl Default for MenuMovie {
    fn default() -> Self {
        Self { image: Handle::default(), act: 0, child: None, latest: default(), size: UVec2::ZERO, shown: false }
    }
}

impl MenuMovie {
    fn playing(&self) -> bool {
        self.child.is_some() && self.shown
    }

    fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.act = 0;
        self.shown = false;
        *self.latest.lock().unwrap() = None;
    }
}

impl Drop for MenuMovie {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Menu movie frame size (the movies are 1280x720).
const MENU_MOVIE_SIZE: UVec2 = UVec2::new(1280, 720);

/// Starts the act's movie while a backdrop menu is up, stops it in the game,
/// and puts its newest frame on screen.
fn menu_movie(
    screen: Res<Screen>,
    campaign: Res<Campaign>,
    game: Res<GameFsRes>,
    mut movie: ResMut<MenuMovie>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(&mut Visibility, &mut ImageNode), With<MenuMovieNode>>,
    mut failed: Local<std::collections::HashSet<usize>>,
) {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use std::io::Read;
    if !screen.shows_backdrop() {
        if movie.child.is_some() {
            movie.stop();
        }
        return;
    }
    if screen.is_changed() || movie.child.is_none() {
        let act = menu_act(&campaign);
        if movie.act != act && !failed.contains(&act) {
            movie.stop();
            let path = format!("media/menu_act{act:02}.bik");
            let size = MENU_MOVIE_SIZE;
            let spawned = crate::movies::ffmpeg().zip(crate::movies::loose_file(&game, &path)).and_then(|(ff, src)| {
                std::process::Command::new(ff)
                    .args(["-v", "error", "-re", "-stream_loop", "-1", "-i"])
                    .arg(src)
                    .args(["-vf", &format!("scale={}:{}", size.x, size.y), "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .ok()
            });
            match spawned {
                Some(mut child) => {
                    let mut out = child.stdout.take().unwrap();
                    let slot = movie.latest.clone();
                    std::thread::spawn(move || {
                        let mut frame = vec![0u8; (size.x * size.y * 4) as usize];
                        while out.read_exact(&mut frame).is_ok() {
                            *slot.lock().unwrap() = Some(frame.clone());
                        }
                    });
                    info!("portal2: menu movie {path}");
                    if movie.size != size {
                        let img = Image::new_fill(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::default());
                        movie.image = images.add(img);
                        movie.size = size;
                    }
                    movie.child = Some(child);
                    movie.act = act;
                }
                None => {
                    warn!("portal2: menu movie {path} unavailable (needs ffmpeg); showing the still");
                    failed.insert(act);
                }
            }
        }
    }
    let frame = movie.latest.lock().unwrap().take();
    if let Some(frame) = frame {
        if let Some(mut img) = images.get_mut(&movie.image) {
            img.data = Some(frame);
        }
        movie.shown = true;
    }
    let want = if movie.playing() { Visibility::Inherited } else { Visibility::Hidden };
    for (mut v, mut node) in &mut nodes {
        if *v != want {
            *v = want;
        }
        if node.image != movie.image {
            node.image = movie.image.clone();
        }
    }
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
    // portal2_background0N: the track of the saved chapter's act, like the movie.
    let track = &tracks[(menu_act(&campaign) - 1).min(tracks.len() - 1)];
    play.write(PlayGameSound { looped: true, ..PlayGameSound::music(track.trim_start_matches("sound/")) });
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
    fn menu_act_by_chapter() {
        // Chapters 1..9 -> menu_act01..05 (and portal2_background01..05).
        let acts: Vec<usize> = (0..9).map(act_of_chapter).collect();
        assert_eq!(acts, [1, 2, 2, 2, 3, 4, 4, 5, 5]);
        assert_eq!(act_of_chapter(42), 1);
    }

    #[test]
    fn main_menu_items_are_portal_2s() {
        let labels: Vec<&str> = MAIN_ITEMS.iter().map(|i| i.0).collect();
        assert_eq!(labels, ["PLAY SINGLE PLAYER", "PLAY COOPERATIVE GAME", "COMMUNITY TEST CHAMBERS", "OPTIONS", "EXTRAS", "QUIT"]);
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
