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

use crate::audio::PlayGameSound;
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
            .init_resource::<CarriedHover>()
            .add_message::<ShowChapterTitle>()
            .add_message::<MenuClick>()
            .add_systems(Startup, (load_fonts, load_menu_art, open_main_menu).chain().after(crate::campaign::load_campaign))
            .add_systems(
                Update,
                (
                    (capture_key, escape_key, scripted_clicks, buttons, scroll_lists, rebuild_menu, carry_hover, hover_rows, describe_rows, menu_movie).chain(),
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
pub(crate) enum Flyout {
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
pub(crate) struct MenuState {
    pub(crate) flyout: Option<Flyout>,
    confirm: Option<Confirm>,
    /// Load Game / Save Game, and the saves it lists (newest first).
    saves: Option<SavesDialog>,
    save_list: Vec<SaveInfo>,
    /// The picked row of that list (an index into `save_list`; `None` is
    /// Save Game's new-save row).
    save_pick: Option<usize>,
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
    /// Dialog titles and items: UniversLTStd-BoldCn (univercb.vfont), else `menu`.
    pub title: Handle<Font>,
}

/// The game's menu art.
#[derive(Resource, Default)]
struct MenuArt {
    logo: Option<Handle<Image>>,
    /// materials/vgui/menu_tiles: the 50-unit tiles Portal 2 builds its
    /// dialogs from, in [`TILES`] order.
    tiles: Option<Vec<Handle<Image>>>,
}

/// The dialog tile pieces (materials/vgui/menu_tiles/ui_tile_128_*).
const TILES: [&str; 9] = ["crnr_topl", "top", "crnr_topr", "left", "interior1", "right", "crnr_bottoml", "bottom", "crnr_bottomr"];

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
    /// Pick a row of Load / Save Game (a second click loads or saves it).
    PickSave(Option<usize>),
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

    // basemodui_scheme.res MainMenuButton: TextColor 120 120 120,
    // FocusColor white, CursorColor 255 255 255 15.
    pub const ITEM: Color = Color::srgb(120.0 / 255.0, 120.0 / 255.0, 120.0 / 255.0);
    pub const ITEM_HOVER: Color = Color::WHITE;
    // Source blends in gamma space (15/255 white lifts a dark 17 to ~31);
    // Bevy blends in linear light, where 15/255 gives 72. ~3/255 gives
    // vgui's result over the menu's dark backgrounds.
    pub const HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 3.0 / 255.0);
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
    /// SliderControl.BackgroundColor 150 150 150 (tiled dialogs).
    pub const TILE_TRACK: Color = Color::srgb(150.0 / 255.0, 150.0 / 255.0, 150.0 / 255.0);
    /// HybridButton.FocusColor 240 240 240.
    pub const TILE_FOCUS: Color = Color::srgb(240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0);
    /// A problem line (the Mods page).
    pub const ERROR: Color = Color::srgb(0.62, 0.12, 0.1);
    /// HybridButton.DisabledColor 140 140 140.
    pub const DISABLED: Color = Color::srgb(140.0 / 255.0, 140.0 / 255.0, 140.0 / 255.0);
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
/// DefaultButton on a tiled dialog: HybridButton TextColor black,
/// FocusColor 240, CursorColor 0 0 0 240.
const DIALOG_LOOK: Look = Look { text: Color::BLACK, text_hover: Color::srgb(240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0), bg: Color::NONE, bg_hover: Color::srgba(0.0, 0.0, 0.0, 240.0 / 255.0) };
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

/// DialogTitle: UniversLTStd-BoldCn tall 33; DialogMenuItem: tall 18
/// weight 700 (drawn here with the Bold Cn face).
const TITLE_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(33.0 / 1.265 * 100.0 / 480.0);
const DIALOG_ITEM_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(18.0 / 1.265 * 100.0 / 480.0);
const DESC_TITLE_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(24.0 / 1.265 * 100.0 / 480.0);
const DESC_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(17.0 / 1.265 * 100.0 / 480.0);
/// The ◀ ▶ on a hovered tiled-dialog row (MarlettLarge in Portal 2).
const TILE_ARROW_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(10.0 * 100.0 / 480.0);

/// Menu item text size; dialogs scale from it.
const ITEM_SIZE: f32 = 24.0;

/// The main/pause menu item font: MainMenuItem is UniversLTStd-Cn "tall"
/// 22 units. vgui's tall is the Windows cell height (OS/2 winAscent +
/// winDescent = 1.265 em for Univers 57 Cn), so the em is 22 / 1.265.
const ITEM_FONT: bevy::text::FontSize = bevy::text::FontSize::Vh(22.0 / 1.265 * 100.0 / 480.0);

/// One unit of Portal 2's menu layout (.res files are 480 units tall), as
/// a share of the screen height.
const fn units(u: f32) -> Val {
    Val::Vh(u * 100.0 / 480.0)
}

fn load_fonts(mut commands: Commands, game: Res<GameFsRes>, mut fonts: ResMut<Assets<Font>>) {
    let load = |fonts: &mut Assets<Font>, paths: &[&str]| -> Option<Handle<Font>> { paths.iter().find_map(|p| game.read(p)).map(|b| fonts.add(Font::from_bytes(b))) };
    let bold = load(&mut fonts, &["resource/linux_fonts/dejavusans-bold.ttf", "resource/linux_fonts/liberationsans-bold.ttf"]).unwrap_or_default();
    let regular = load(&mut fonts, &["resource/linux_fonts/dejavusans.ttf", "resource/linux_fonts/liberationsans-regular.ttf"]).unwrap_or_default();
    // The menu font: P2_MENU_FONT (a game path or a file), else Portal 2's
    // own UniversLTStd-Cn (basemodui_scheme.res CustomFontFiles), else the
    // best match among the install's plain font files.
    let univers = game.read("vgui/fonts/universc.vfont").and_then(|b| decode_vfont(&b));
    if univers.is_none() {
        warn!("portal2: vgui/fonts/universc.vfont missing or unreadable");
    }
    let files: Vec<String> = game.0.list("resource").into_iter().filter(|f| f.ends_with(".ttf") || f.ends_with(".otf")).collect();
    info!("portal2: font files in the game: {files:?}");
    let chosen = std::env::var("P2_MENU_FONT").ok().or_else(|| if univers.is_some() { None } else { pick_menu_font(&files).cloned() });
    let menu = chosen.as_deref().and_then(|p| game.read(p).or_else(|| std::fs::read(p).ok()).map(|b| (p, fonts.add(Font::from_bytes(b)))));
    let menu = menu.or_else(|| univers.map(|b| ("vgui/fonts/universc.vfont", fonts.add(Font::from_bytes(b)))));
    // No Univers: macOS ships DIN Alternate, a close match.
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
    let title = game.read("vgui/fonts/univercb.vfont").and_then(|b| decode_vfont(&b)).map(|b| fonts.add(Font::from_bytes(b))).unwrap_or_else(|| menu.clone());
    commands.insert_resource(UiFonts { bold, regular, menu, title });
}

/// A Valve `.vfont` as the TrueType file it wraps. The file is the font,
/// then a salt (its length in the byte before the trailer), then
/// "VFONT1". Each byte is XORed with a key that becomes the previous
/// encoded byte + 0xA7; an sfnt starts with a 0 byte, which gives the
/// first key. Worked out from the install's files (known plaintext).
pub(crate) fn decode_vfont(data: &[u8]) -> Option<Vec<u8>> {
    let n = data.len().checked_sub(6).filter(|_| data.ends_with(b"VFONT1"))?;
    let salt = *data.get(n.checked_sub(1)?)? as usize;
    let body = &data[..n.checked_sub(1 + salt)?];
    let mut key = *body.first()?;
    Some(body.iter().map(|&c| {
        let p = c ^ key;
        key = c.wrapping_add(0xA7);
        p
    }).collect())
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
    let tiles = TILES
        .iter()
        .map(|t| {
            let vtf = source_assets::vtf::VtfTexture::parse(&game.read(&format!("materials/vgui/menu_tiles/ui_tile_128_{t}.vtf"))?).ok()?;
            Some(images.add(vtf.into_image(false, true)))
        })
        .collect::<Option<Vec<_>>>();
    if tiles.is_none() {
        warn!("portal2: materials/vgui/menu_tiles missing; dialogs use plain panels");
    }
    commands.insert_resource(MenuArt { logo, tiles });
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

/// A main/pause menu item (MainMenuButton: text inset 10 units).
/// `open`: its flyout is showing, so it stays lit as in Portal 2.
fn item(fonts: &UiFonts, label: &str, action: Action, open: bool) -> impl Bundle + use<> {
    let look = if open { Look { text: ITEM_LOOK.text_hover, bg: ITEM_LOOK.bg_hover, ..ITEM_LOOK } } else { ITEM_LOOK };
    (
        Button,
        action,
        look,
        Node { padding: UiRect::axes(units(10.0), Val::Px(0.0)), height: Val::Percent(100.0), align_items: AlignItems::Center, ..default() },
        BackgroundColor(look.bg),
        children![(Text::new(label.to_string()), TextFont { font: (&fonts.menu).into(), font_size: ITEM_FONT, ..default() }, TextColor(look.text))],
    )
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
    // mainmenu.res (widescreen PC): items at x 88, 20 tall every 30 units,
    // the last (Quit) at y 378.
    p.spawn(Node {
        position_type: PositionType::Absolute,
        left: units(88.0),
        bottom: units(480.0 - 378.0 - 20.0),
        flex_direction: FlexDirection::Column,
        row_gap: units(10.0),
        ..default()
    })
    .with_children(|col| {
        for (label, action, fly) in items {
            col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, height: units(20.0), ..default() }).with_children(|row| {
                row.spawn(item(fonts, label, *action, fly.is_some() && *fly == open));
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

/// Which [`TILES`] piece sits at column `c`, row `r` of a tiled dialog:
/// a title tab `tab` tiles wide on row 0 over a body `wide` x `tall`
/// (rows 1..=tall). `None`: empty (beside the tab).
fn tile_piece(c: usize, r: usize, tab: usize, wide: usize, tall: usize) -> Option<usize> {
    let last_c = if r == 0 { tab - 1 } else { wide - 1 };
    if r == 0 && c >= tab {
        return None;
    }
    let col = if c == 0 { 0 } else if c == last_c { 2 } else { 1 };
    let row = if r == tall {
        2
    } else if r == 0 || (r == 1 && c >= tab) {
        0
    } else {
        1
    };
    // Under the tab the body continues the tab's sides; its own top edge
    // starts past the tab (the left corner there is plain top edge).
    let col = if r == 1 && c >= tab && c != last_c { 1 } else { col };
    Some(row * 3 + col)
}

/// A Portal 2 dialog (CBaseModFrame, dialogstyle 1) as singleplayer.res
/// and coopmode.res lay it out: pinned 100 units from the left and 75 from
/// the bottom (Dialog.PinFromLeft/PinFromBottom), a body `wide` x `tall`
/// tiles of 50 units under a title tab (DialogTitle, offset 10,9), built
/// from the menu_tiles art; DefaultButton rows every 25 units from 25,
/// upper case, black on the tiles, white on a black cursor bar; disabled
/// rows grey (HybridButton.DisabledColor 140). The Back button sits below.
fn tiled_dialog(p: &mut ChildSpawnerCommands, fonts: &UiFonts, art: Option<&MenuArt>, title: &str, wide: usize, tall: usize, rows: &[(&str, Option<Action>)]) {
    tile_frame(p, fonts, art, title, wide, tall, &[("Back", Action::Back)], |d| {
        for (i, (label, action)) in rows.iter().enumerate() {
            match action {
                Some(a) => {
                    d.spawn(dialog_row(fonts, label, *a, false, 25.0 * (i as f32 + 1.0)));
                }
                None => {
                    d.spawn(disabled_row(fonts, label, 25.0 * (i as f32 + 1.0)));
                }
            }
        }
    });
}

/// Options as options.res lays it out: a tiled dialog in place of the menu
/// column (body 5 x 3 tiles for Portal 2's four rows; taller when our own
/// pages follow them), one DefaultButton row per page.
fn options_dialog(p: &mut ChildSpawnerCommands, fonts: &UiFonts, art: Option<&MenuArt>, pages: &[(&str, Action)]) {
    let rows: Vec<(&str, Option<Action>)> = pages.iter().map(|(l, a)| (*l, Some(*a))).collect();
    tiled_dialog(p, fonts, art, "Options", 5, dialog_tall(rows.len()), &rows);
}

/// The pause menu as ingamemainmenu.res lays it out for single player: a
/// tiled dialog (dialogstyle 1), body 5 x 4, DefaultButton rows every 25
/// units from 25: Return to Game, Save Game, Load Last Save, Options, Exit
/// to Main Menu (Restart Level and the rest only show in challenge,
/// community and puzzle maker maps). No footer: Return to Game or Esc
/// closes it. The title tab is left blank: neither the .res nor the
/// localization files give the frame a title.
fn pause_dialog(p: &mut ChildSpawnerCommands, fonts: &UiFonts, art: Option<&MenuArt>, have_save: bool) {
    let rows = pause_rows(have_save);
    tile_frame(p, fonts, art, "", 5, 4, &[], |d| {
        for (i, (label, action)) in rows.iter().enumerate() {
            let y = 25.0 * (i as f32 + 1.0);
            match action {
                Some(a) => {
                    d.spawn(dialog_row(fonts, label, *a, false, y));
                }
                None => {
                    d.spawn(disabled_row(fonts, label, y));
                }
            }
        }
    });
}

/// The pause dialog's rows (Portal 2's English labels); Load Last Save is
/// greyed without a save.
fn pause_rows(have_save: bool) -> [(&'static str, Option<Action>); 5] {
    [
        ("Return to Game", Some(Action::Resume)),
        ("Save Game", Some(Action::SaveGame)),
        ("Load Last Save", have_save.then_some(Action::Continue)),
        ("Options", Some(Action::Options)),
        ("Exit to Main Menu", Some(Action::MainMenu)),
    ]
}

/// Body tiles (50 units) for `rows` DefaultButton rows every 25 units from
/// 25, at least options.res's 3.
fn dialog_tall(rows: usize) -> usize {
    (((rows + 1) * 25).div_ceil(50)).max(3)
}

/// How many 50-unit tiles the title tab needs: about 13 units a BoldCn 33
/// capital plus the 10-unit inset each side and a little margin (a 10-letter
/// title such as CONTROLLER measures ~141 units, so 3 tiles clip it).
fn title_tab_tiles(title: &str) -> usize {
    ((title.len() as f32 * 13.0 + 25.0) / 50.0).ceil().max(3.0) as usize
}

/// The tiles of a Portal 2 dialog ([`tiled_dialog`]): `content` is laid out
/// in units from the body's top left (under the title tab); the footer
/// buttons sit below the frame (FooterPanel.OffsetY 12).
fn tile_frame(p: &mut ChildSpawnerCommands, fonts: &UiFonts, art: Option<&MenuArt>, title: &str, wide: usize, tall: usize, footer: &[(&str, Action)], content: impl FnOnce(&mut ChildSpawnerCommands)) {
    const T: f32 = 50.0;
    let tab = title_tab_tiles(title);
    let tab = tab.min(wide);
    let top = 480.0 - 75.0 - (tall as f32 + 1.0) * T;
    p.spawn(Node {
        position_type: PositionType::Absolute,
        left: units(100.0),
        top: units(top),
        width: units(wide as f32 * T),
        height: units((tall as f32 + 1.0) * T),
        ..default()
    })
    .with_children(|d| {
        for r in 0..=tall {
            for c in 0..wide {
                let Some(piece) = tile_piece(c, r, tab, wide, tall) else { continue };
                let node = Node { position_type: PositionType::Absolute, left: units(c as f32 * T), top: units(r as f32 * T), width: units(T), height: units(T), ..default() };
                match art.and_then(|a| a.tiles.as_ref()) {
                    Some(t) => {
                        d.spawn((ImageNode::new(t[piece].clone()), node));
                    }
                    None => {
                        d.spawn((node, BackgroundColor(theme::PANEL)));
                    }
                }
            }
        }
        d.spawn((
            Node { position_type: PositionType::Absolute, left: units(10.0), top: units(9.0), ..default() },
            children![(Text::new(title.to_uppercase()), TextFont { font: (&fonts.title).into(), font_size: TITLE_FONT, ..default() }, TextColor(Color::BLACK))],
        ));
        d.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: units(T), width: units(wide as f32 * T), height: units(tall as f32 * T), ..default() })
            .with_children(content);
        // FooterPanel: OffsetY 12 below the dialog.
        d.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: units((tall as f32 + 1.0) * T + 12.0), column_gap: units(6.0), ..default() })
            .with_children(|f| {
                for (label, action) in footer {
                    f.spawn(button(&fonts.menu, *label, *action, 16.0, BUTTON_LOOK));
                }
            });
    });
}

/// A DefaultButton row of a tiled dialog, 25 units tall at `y` units below
/// the body's top: upper case, black, white on the black cursor bar
/// (`lit`: the picked row, kept on the bar).
fn dialog_row(fonts: &UiFonts, label: &str, action: Action, lit: bool, y: f32) -> impl Bundle + use<> {
    let look = if lit { Look { text: DIALOG_LOOK.text_hover, bg: DIALOG_LOOK.bg_hover, ..DIALOG_LOOK } } else { DIALOG_LOOK };
    let font = TextFont { font: (&fonts.title).into(), font_size: DIALOG_ITEM_FONT, ..default() };
    (Button, action, look, row_node(y), BackgroundColor(look.bg), children![(Text::new(label.to_uppercase()), font, TextColor(look.text))])
}

/// A greyed row (HybridButton.DisabledColor 140) of a tiled dialog.
fn disabled_row(fonts: &UiFonts, label: &str, y: f32) -> impl Bundle + use<> {
    let font = TextFont { font: (&fonts.title).into(), font_size: DIALOG_ITEM_FONT, ..default() };
    (row_node(y), children![(Text::new(label.to_uppercase()), font, TextColor(theme::DISABLED))])
}

fn row_node(y: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        width: Val::Percent(100.0),
        top: units(y - 2.5),
        height: units(25.0),
        padding: UiRect::left(units(10.0)),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A Settings row on a tiled dialog (audio.res): 25 units tall at `y`
/// units below the body's top. LeftDialogButton / DialogListButton:
/// label at TextInsetX 10, the slider or list value anchored to the right
/// edge, DialogButton font (UniversLTStd-LightCn tall 18 weight 700 on PC
/// HiDef; drawn with the Cn face), black on the tiles; the hovered row on
/// the HybridButton cursor bar (black 240) with 240 text ([`hover_rows`]).
/// SliderControl: track BackgroundColor 150 grey, fill ForegroundColor
/// black (FocusColor white), no number.
fn tile_option_row(p: &mut ChildSpawnerCommands, fonts: &UiFonts, row: &Row, y: f32) {
    let font = |size| TextFont { font: (&fonts.menu).into(), font_size: size, ..default() };
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        width: Val::Percent(100.0),
        top: units(y),
        height: units(25.0),
        padding: UiRect::horizontal(units(10.0)),
        align_items: AlignItems::Center,
        column_gap: units(2.0),
        ..default()
    };
    let label = |r: &mut ChildSpawnerCommands, s: &str| {
        r.spawn((Node { flex_grow: 1.0, ..default() }, children![(Text::new(s.to_string()), font(DIALOG_ITEM_FONT), TextColor(Color::BLACK))]));
    };
    let arrow = |r: &mut ChildSpawnerCommands, left: bool, action: Action| {
        r.spawn((
            Button,
            action,
            VALUE_LOOK,
            InRow,
            RowArrow,
            Visibility::Hidden,
            Node { padding: UiRect::horizontal(units(3.0)), align_items: AlignItems::Center, ..default() },
            BackgroundColor(Color::NONE),
            children![(Text::new(if left { "\u{25C0}" } else { "\u{25B6}" }), font(TILE_ARROW_FONT), TextColor(Color::BLACK))],
        ));
    };
    match row {
        Row::Choice { label: l, value, setting } => {
            p.spawn((hover_row(), OnTiles, RowHelp { label: l.clone(), setting: *setting })).insert(node.clone()).with_children(|r| {
                label(r, l);
                arrow(r, true, Action::Adjust(*setting, -1.0));
                r.spawn((
                    Button,
                    Action::Toggle(*setting),
                    VALUE_LOOK,
                    InRow,
                    Node { min_width: units(130.0), flex_shrink: 0.0, justify_content: JustifyContent::FlexEnd, ..default() },
                    BackgroundColor(Color::NONE),
                    children![(Text::new(value.clone()), font(DIALOG_ITEM_FONT), TextColor(Color::BLACK), TextLayout::new(Justify::Right, LineBreak::NoWrap))],
                ));
                arrow(r, false, Action::Adjust(*setting, 1.0));
            });
        }
        Row::Slider { label: l, fraction, setting, .. } => {
            p.spawn((hover_row(), OnTiles, RowHelp { label: l.clone(), setting: *setting })).insert(node.clone()).with_children(|r| {
                label(r, l);
                arrow(r, true, Action::Adjust(*setting, -1.0));
                r.spawn((
                    Button,
                    Action::Slide(*setting),
                    Look { text: Color::BLACK, text_hover: Color::BLACK, bg: Color::NONE, bg_hover: Color::NONE },
                    InRow,
                    RelativeCursorPosition::default(),
                    Node { width: units(130.0), height: units(20.0), align_items: AlignItems::Center, ..default() },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|t| {
                    t.spawn((SliderTrack, Node { width: Val::Percent(100.0), height: units(4.0), ..default() }, BackgroundColor(theme::TILE_TRACK)))
                        .with_child((SliderFill, Node { width: Val::Percent(fraction * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::BLACK)));
                });
                arrow(r, false, Action::Adjust(*setting, 1.0));
            });
        }
        Row::Link { label: l, page } => {
            let look = DIALOG_LOOK;
            p.spawn((Button, Action::Page(*page), look, node, BackgroundColor(look.bg), children![(Text::new(l.clone()), font(DIALOG_ITEM_FONT), TextColor(look.text))]));
        }
        // audio.res has no headings, notes or key rows.
        _ => {}
    }
}

/// A tiled Settings row's label and setting, for the description pane.
#[derive(Component)]
struct RowHelp {
    label: String,
    setting: Setting,
}

/// advancedvideo.res LblDescriptionTitle / LblDescription.
#[derive(Component)]
struct DescTitle;
#[derive(Component)]
struct DescText;

/// The Advanced Video description pane (advancedvideo.res, WIN32WIDE):
/// LblDescriptionTitle at 360,25, 180 x 25, VideoSettingsDescriptionTitle
/// (UniversLTStd-BoldCn tall 24); LblDescription at 360,60, 180 x 180,
/// wrapped, VideoSettingsDescription (LightCn tall 17 weight 700 on HiDef,
/// drawn with the Cn face); both black. Filled by [`describe_rows`].
/// Starts on `first` (Portal 2 focuses the first row when the dialog opens).
fn description_pane(p: &mut ChildSpawnerCommands, fonts: &UiFonts, first: Option<&Row>) {
    let (label, desc) = match first {
        Some(Row::Choice { label, setting, .. } | Row::Slider { label, setting, .. }) => (label.clone(), options::describe(*setting)),
        _ => (String::new(), ""),
    };
    let at = |y: f32, tall: f32| Node { position_type: PositionType::Absolute, left: units(360.0), top: units(y), width: units(180.0), height: units(tall), ..default() };
    p.spawn((at(25.0, 25.0), children![(DescTitle, Text::new(label), TextFont { font: (&fonts.title).into(), font_size: DESC_TITLE_FONT, ..default() }, TextColor(Color::BLACK))]));
    p.spawn((at(60.0, 180.0), children![(DescText, Text::new(desc), TextFont { font: (&fonts.menu).into(), font_size: DESC_FONT, ..default() }, TextColor(Color::BLACK))]));
}

/// Show the hovered row's label and description in the pane; the last one
/// stays when the cursor leaves (Portal 2 describes the focused row).
fn describe_rows(
    rows: Query<(&HoverRow, &RowHelp), Changed<HoverRow>>,
    mut title: Query<&mut Text, (With<DescTitle>, Without<DescText>)>,
    mut text: Query<&mut Text, (With<DescText>, Without<DescTitle>)>,
) {
    let Some((_, help)) = rows.iter().find(|(r, _)| r.hot) else { return };
    for mut t in &mut title {
        t.0 = help.label.clone();
    }
    for mut t in &mut text {
        t.0 = options::describe(help.setting).to_string();
    }
}

/// A [`HoverRow`] on a tiled dialog: black text idle, the HybridButton
/// cursor bar when hovered.
#[derive(Component)]
struct OnTiles;

/// The Portal 2 logo, top left: basemodui_scheme.res Logo.X 75, Logo.Y 35,
/// 240 x 60 units.
fn menu_logo(p: &mut ChildSpawnerCommands, fonts: &UiFonts, art: Option<&MenuArt>) {
    let logo_node = Node { position_type: PositionType::Absolute, left: units(75.0), top: units(35.0), width: units(240.0), height: units(60.0), ..default() };
    match art.and_then(|a| a.logo.clone()) {
        Some(img) => {
            p.spawn((ImageNode::new(img), logo_node));
        }
        None => {
            p.spawn((logo_node, children![text(&fonts.menu, 72.0, "PORTAL 2", theme::TITLE)]));
        }
    }
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

/// Actions whose buttons were hovered when the menu was last rebuilt.
#[derive(Resource, Default)]
struct CarriedHover(Vec<Action>);

/// Freshly rebuilt buttons for the actions in [`CarriedHover`] start lit
/// (and Hovered) instead of flashing idle for a frame before the UI focus
/// system finds the cursor over them.
fn carry_hover(
    mut carried: ResMut<CarriedHover>,
    mut q: Query<(Entity, &Action, &Look, &mut Interaction, Has<InRow>), Added<Interaction>>,
    mut bgs: Query<&mut BackgroundColor>,
    mut texts: Query<&mut TextColor>,
    children: Query<&Children>,
) {
    if carried.0.is_empty() {
        return;
    }
    for (e, action, look, mut interaction, in_row) in &mut q {
        if !carried.0.contains(action) {
            continue;
        }
        *interaction = Interaction::Hovered;
        if in_row {
            continue;
        }
        if let Ok(mut bg) = bgs.get_mut(e) {
            bg.0 = look.bg_hover;
        }
        for c in children.iter_descendants(e) {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = look.text_hover;
            }
        }
    }
    carried.0.clear();
}

/// The row under the mouse: dark bar, white text, arrows shown; the
/// others plain.
#[allow(clippy::type_complexity)]
fn hover_rows(
    mut rows: Query<(Entity, &RelativeCursorPosition, &mut HoverRow, &mut BackgroundColor, Has<OnTiles>), Without<SliderTrack>>,
    children: Query<&Children>,
    mut texts: Query<&mut TextColor>,
    mut arrows: Query<&mut Visibility, With<RowArrow>>,
    mut parts: Query<(&mut BackgroundColor, Has<SliderFill>), (Or<(With<SliderTrack>, With<SliderFill>)>, Without<HoverRow>)>,
    menu: Res<MenuState>,
) {
    for (e, cursor, mut row, mut bg, tiles) in &mut rows {
        // Under a confirmation, nothing is hovered.
        let hot = cursor.cursor_over() && menu.confirm.is_none();
        if hot == row.hot {
            continue;
        }
        row.hot = hot;
        bg.0 = match (hot, tiles) {
            (true, true) => DIALOG_LOOK.bg_hover,
            (true, false) => theme::ROW_HOVER,
            (false, _) => Color::NONE,
        };
        for c in children.iter_descendants(e) {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = match (hot, tiles) {
                    (true, true) => theme::TILE_FOCUS,
                    (true, false) => Color::WHITE,
                    (false, true) => Color::BLACK,
                    (false, false) => theme::VALUE,
                };
            }
            if let Ok(mut v) = arrows.get_mut(c) {
                *v = if hot { Visibility::Inherited } else { Visibility::Hidden };
            }
            if let Ok((mut b, fill)) = parts.get_mut(c) {
                b.0 = match (fill, hot, tiles) {
                    (true, true, true) => Color::WHITE,
                    (true, false, true) => Color::BLACK,
                    (false, _, true) => theme::TILE_TRACK,
                    (true, true, false) => theme::FILL_HOT,
                    (true, false, false) => theme::FILL,
                    (false, true, false) => theme::TRACK_HOT,
                    (false, false, false) => theme::TRACK,
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
    (old, mut carried): (Query<(&Interaction, &Action), With<Button>>, ResMut<CarriedHover>),
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
    // The rebuilt buttons only learn they are hovered a frame later; keep
    // the hovered ones lit meanwhile (see carry_hover).
    carried.0 = old.iter().filter(|(i, _)| **i != Interaction::None).map(|(_, a)| *a).collect();
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
            // saveloadgamedialog.res (both Load and Save Game): a tiled
            // dialog, body 9 x 4 tiles; SaveGameList at 0,25, 200 x 150,
            // savegameitem rows 25 units (label inset 10), scrolling; the
            // picked save's picture (SaveGameImage 212,15, 225 x 152) with
            // AutoSaveLabel over its top right (NewGameChapterName, white
            // 200) and SaveGameInfo at 212,170 under it.
            let save = menu.saves == Some(SavesDialog::Save);
            let shown = shown_saves(&menu.save_list, save);
            let pick = menu.save_pick.filter(|i| shown.contains(i)).or(if save { None } else { shown.first().copied() });
            let mut foot = Vec::new();
            if save {
                foot.push(("Save Game", Action::SaveSlot(pick)));
            } else if let Some(i) = pick {
                foot.push(("Load Game", Action::LoadSave(i)));
            }
            foot.push(("Back", Action::Back));
            commands.spawn(layer(10)).with_children(|p| {
                if *screen == Screen::Main {
                    menu_logo(p, &fonts, art.as_deref());
                }
                tile_frame(p, &fonts, art.as_deref(), if save { "Save Game" } else { "Load Game" }, 9, 4, &foot, |d| {
                    d.spawn((
                        ScrollList,
                        ScrollPosition(Vec2::new(0.0, scroll.0)),
                        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: units(25.0 - 2.5), width: units(200.0), height: units(150.0), overflow: Overflow::scroll_y(), ..default() },
                    ))
                    .with_children(|list| {
                        let mut y = 2.5;
                        if save {
                            list.spawn(dialog_row(&fonts, "New Save", Action::PickSave(None), pick.is_none(), y));
                            y += 25.0;
                        }
                        for &i in &shown {
                            list.spawn(dialog_row(&fonts, &save_row_label(&menu.save_list[i]), Action::PickSave(Some(i)), pick == Some(i), y));
                            y += 25.0;
                        }
                    });
                    if shown.is_empty() && !save {
                        // LblNoSaves (#GameUI_NoSaveGamesToDisplay).
                        d.spawn((
                            Node { position_type: PositionType::Absolute, left: units(10.0), top: units(25.0), width: units(190.0), ..default() },
                            children![(Text::new("NO SAVED GAMES TO DISPLAY."), TextFont { font: (&fonts.title).into(), font_size: DIALOG_ITEM_FONT, ..default() }, TextColor(theme::DISABLED))],
                        ));
                    }
                    let picked = pick.and_then(|i| menu.save_list.get(i));
                    let chapter = picked.and_then(|s| campaign.chapter_of(&s.header.map));
                    let pic = Node { position_type: PositionType::Absolute, left: units(212.0), top: units(15.0), width: units(225.0), height: units(152.0), ..default() };
                    match chapter.and_then(|c| chapter_pics.get(c).cloned().flatten()) {
                        Some(img) if picked.is_some() => {
                            d.spawn((ImageNode { image: img, image_mode: bevy::ui::widget::NodeImageMode::Stretch, ..default() }, pic));
                        }
                        _ => {
                            d.spawn((pic, BackgroundColor(Color::BLACK)));
                        }
                    }
                    let size = bevy::text::FontSize::Vh(18.0 / 1.265 * 100.0 / 480.0);
                    if let Some(s) = picked.filter(|s| s.header.kind != "save") {
                        d.spawn((
                            Node { position_type: PositionType::Absolute, left: units(210.0), top: units(9.0), width: units(223.0), height: units(30.0), justify_content: JustifyContent::FlexEnd, align_items: AlignItems::Center, ..default() },
                            children![(Text::new(save_kind_label(&s.header.kind, &s.name).to_uppercase()), TextFont { font: (&fonts.title).into(), font_size: size, ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 200.0 / 255.0)))],
                        ));
                    }
                    let (title, subtitle) = match (picked, chapter.and_then(|c| campaign.chapters.get(c))) {
                        (Some(_), Some(c)) => (c.title.to_uppercase(), c.subtitle.to_uppercase()),
                        (Some(s), None) => (String::new(), s.header.map.to_uppercase()),
                        (None, _) => (String::new(), String::new()),
                    };
                    d.spawn((
                        Node { position_type: PositionType::Absolute, left: units(212.0), top: units(170.0), width: units(225.0), height: units(20.0), align_items: AlignItems::Center, ..default() },
                        children![(
                            Text::new(if title.is_empty() { String::new() } else { format!("{title}  ") }),
                            TextFont { font: (&fonts.menu).into(), font_size: size, ..default() },
                            TextColor(Color::BLACK),
                            children![(TextSpan::new(subtitle), TextFont { font: (&fonts.title).into(), font_size: size, ..default() }, TextColor(Color::BLACK))],
                        )],
                    ));
                });
            });
        }
        Screen::Main => {
            let mut root = commands.spawn(layer(10));
            root.with_children(|p| {
                menu_logo(p, &fonts, art.as_deref());
                if menu.flyout == Some(Flyout::SinglePlayer) {
                    // singleplayer.res: a dialog in place of the menu column,
                    // body 5 x 4 tiles; Continue and Load are disabled
                    // without a save.
                    let save = read_save().is_some();
                    let on = |a: Action| if save { Some(a) } else { None };
                    let rows = [
                        ("Continue Game", on(Action::Continue)),
                        ("New Game", Some(Action::NewGame)),
                        ("Load Game", on(Action::LoadGame)),
                        ("Challenge Mode", Some(Action::ChallengeMode)),
                        ("Developer Commentary", None),
                    ];
                    tiled_dialog(p, &fonts, art.as_deref(), "Single Player", 5, 4, &rows);
                } else if menu.flyout == Some(Flyout::Options) {
                    options_dialog(p, &fonts, art.as_deref(), &options_flyout);
                } else {
                    item_list(p, &fonts, &MAIN_ITEMS, menu.flyout, &options_flyout);
                }
            });
        }
        Screen::Pause => {
            let mut root = commands.spawn(layer(10));
            root.with_children(|p| {
                if menu.flyout == Some(Flyout::Options) {
                    options_dialog(p, &fonts, art.as_deref(), &options_flyout);
                    return;
                }
                pause_dialog(p, &fonts, art.as_deref(), read_save().is_some());
            });
        }
        Screen::Chapters if !(tab.community && !community.maps.is_empty()) => {
            // newgamedialog.res: a tiled dialog, body 9 x 4 tiles; the
            // ChapterList (newgame_chapteritem rows, 25 units) at 0,25,
            // 200 x 150, scrolling; the picked chapter's picture
            // (ChapterImage 212,15, 225 x 152) with its ChapterLabel at
            // 212,170: the chapter in UniversLTStd-Cn, its name in BoldCn
            // (NewGameChapter / NewGameChapterName, 18 units).
            let sel = tab.chapter.min(campaign.chapters.len().saturating_sub(1));
            let mut foot = vec![("Start New Game", Action::Chapter(sel)), ("Back", Action::Back)];
            if !community.maps.is_empty() {
                foot.insert(1, ("Community Maps", Action::ChapterTab(true)));
            }
            commands.spawn(layer(10)).with_children(|p| {
                menu_logo(p, &fonts, art.as_deref());
                tile_frame(p, &fonts, art.as_deref(), "New Game", 9, 4, &foot, |d| {
                    d.spawn((
                        ScrollList,
                        ScrollPosition(Vec2::new(0.0, scroll.0)),
                        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: units(25.0 - 2.5), width: units(200.0), height: units(150.0), overflow: Overflow::scroll_y(), ..default() },
                    ))
                    .with_children(|list| {
                        for (i, c) in campaign.chapters.iter().enumerate() {
                            let y = 25.0 * i as f32 + 2.5;
                            // Chapters not reached yet: greyed, not clickable.
                            if progress.is_unlocked(i) {
                                list.spawn(dialog_row(&fonts, &c.title, Action::SelectChapter(i), i == sel, y));
                            } else {
                                list.spawn(disabled_row(&fonts, &c.title, y));
                            }
                        }
                    });
                    let pic = Node { position_type: PositionType::Absolute, left: units(212.0), top: units(15.0), width: units(225.0), height: units(152.0), ..default() };
                    match chapter_pics.get(sel) {
                        Some(Some(img)) => {
                            d.spawn((ImageNode { image: img.clone(), image_mode: bevy::ui::widget::NodeImageMode::Stretch, ..default() }, pic));
                        }
                        _ => {
                            d.spawn((pic, BackgroundColor(Color::BLACK)));
                        }
                    }
                    if let Some(c) = campaign.chapters.get(sel) {
                        let size = bevy::text::FontSize::Vh(18.0 / 1.265 * 100.0 / 480.0);
                        d.spawn((
                            Node { position_type: PositionType::Absolute, left: units(212.0), top: units(170.0), width: units(225.0), height: units(20.0), align_items: AlignItems::Center, ..default() },
                            children![(
                                Text::new(format!("{}  ", c.title.to_uppercase())),
                                TextFont { font: (&fonts.menu).into(), font_size: size, ..default() },
                                TextColor(Color::BLACK),
                                children![(TextSpan::new(c.subtitle.to_uppercase()), TextFont { font: (&fonts.title).into(), font_size: size, ..default() }, TextColor(Color::BLACK))],
                            )],
                        ));
                    }
                });
            });
        }
        Screen::Chapters => {
            let panel = dialog(&mut commands, &fonts, "NEW GAME", 980.0);
            commands.entity(panel).with_children(|p| {
                chapter_tabs(p, &fonts, true);
                let grid = Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                    column_gap: Val::Px(14.0),
                    row_gap: Val::Px(10.0),
                    width: Val::Percent(100.0),
                    ..default()
                };
                let tile = Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(6.0)), row_gap: Val::Px(2.0), overflow: Overflow::clip(), ..default() };
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
            // Tiled dialogs: (body wide, tall, first row's y in units).
            // audio.res: 7 x 5, rows every 25 units from 0 in the .res
            // order (Game Volume, Music Volume, Speaker Configuration,
            // Captioning, Language; our Subtitle style follows).
            // video.res: 7 x 4, rows every 25 from 25 (Brightness, Aspect
            // Ratio, Resolution, Display Mode, Advanced as a DefaultButton).
            // advancedvideo.res (WIN32WIDE): 11 x 5, rows 350 wide every
            // 25 from 25, a description pane at x 360. Its 8 rows plus our
            // 6 need 14: packed from 0 like Audio on a 7-tile body.
            // keyboardmouse.res: 7 x 5 (!$OSX), rows every 25 from 25
            // (Edit Keys as a DefaultButton first).
            // controlleroptions.res: 7 x 5 (!$GAMECONSOLE), rows every 25
            // from 25.
            let tiled = match page {
                OptionsPage::KeyboardMouse | OptionsPage::Controller => Some((7, 5, 25.0)),
                OptionsPage::Audio => Some((7, 5, 0.0)),
                OptionsPage::Video => Some((7, 4, 25.0)),
                OptionsPage::AdvancedVideo => Some((11, 7, 0.0)),
                _ => None,
            };
            if let Some((wide, tall, y0)) = tiled {
                let foot: Vec<(&str, Action)> = page.footer().iter().map(|&k| (k.label(), k.into())).collect();
                let described = page == OptionsPage::AdvancedVideo;
                commands.spawn(layer(10)).with_children(|p| {
                    tile_frame(p, &fonts, art.as_deref(), page.title(), wide, tall, &foot, |d| {
                        let width = if described { units(350.0) } else { Val::Percent(100.0) };
                        d.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width, height: Val::Percent(100.0), ..default() })
                            .with_children(|c| {
                                for (i, r) in rows.iter().enumerate() {
                                    tile_option_row(c, &fonts, r, y0 + 25.0 * i as f32);
                                }
                            });
                        if described {
                            description_pane(d, &fonts, rows.first());
                        }
                    });
                });
            } else {
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

/// The saves a Load (all) or Save Game (named saves only, to overwrite)
/// list shows: indices into `list`, newest first.
fn shown_saves(list: &[SaveInfo], save: bool) -> Vec<usize> {
    list.iter().enumerate().filter(|(_, s)| !save || s.header.kind == "save").map(|(i, _)| i).collect()
}

/// A savegameitem row's label: when it was saved (the chapter shows under
/// the picture).
fn save_row_label(s: &SaveInfo) -> String {
    crate::saves::time_text(s.header.saved_at)
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
            // Back to the Options dialog the page was opened from.
            menu.flyout = Some(Flyout::Options);
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
                // Load starts on the newest save, Save on the new-save row.
                menu.save_pick = if *action == Action::LoadGame { shown_saves(&menu.save_list, false).first().copied() } else { None };
                scroll.0 = 0.0;
            }
            Action::LoadSave(i) => {
                if let Some(s) = menu.save_list.get(i) {
                    load_game.write(LoadGame(Slot::Named(s.name.clone())));
                    menu.saves = None;
                    *screen = Screen::None;
                }
            }
            Action::PickSave(i) if menu.save_pick == i => {
                // A second click on the picked row: load or save it.
                if menu.saves == Some(SavesDialog::Save) {
                    let slot = match i.and_then(|i| menu.save_list.get(i)) {
                        Some(s) => Slot::Named(s.name.clone()),
                        None => Slot::New,
                    };
                    save_game.write(SaveGame(slot));
                    menu.saves = None;
                } else if let Some(s) = i.and_then(|i| menu.save_list.get(i)) {
                    load_game.write(LoadGame(Slot::Named(s.name.clone())));
                    menu.saves = None;
                    *screen = Screen::None;
                }
            }
            Action::PickSave(i) => menu.save_pick = i,
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
                scroll.0 = 0.0;
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
            Action::Back if menu.saves.is_some() => {
                menu.saves = None;
                // Back to the Single Player dialog it was opened from.
                if *screen == Screen::Main {
                    menu.flyout = Some(Flyout::SinglePlayer);
                }
            }
            Action::Back if matches!(*screen, Screen::Main | Screen::Pause) && menu.flyout.is_some() => menu.flyout = None,
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

/// One line of the chapter title card: a map's `@chapter_title_text` /
/// `@chapter_subtitle_text` game_text, whose characters are its `TextSpan`
/// children.
#[derive(Component)]
struct TitleScanLine;

/// game_text "effect 2" (scan out) as the maps' chapter title texts set it:
/// characters appear `fadein` apart, each starting in `color2` and reaching
/// `color` over `fxtime`; the whole line holds `holdtime` after the scan and
/// fades over `fadeout`. Both title lines use the same numbers.
const TITLE_FADEIN: f32 = 0.06;
const TITLE_FXTIME: f32 = 0.5;
const TITLE_HOLD: f32 = 5.0;
const TITLE_FADEOUT: f32 = 0.5;
const TITLE_COLOR2: f32 = 205.0 / 255.0;

/// Grey level and alpha of character `i` of an `n`-character line `t`
/// seconds after it starts (None: not shown yet or faded out).
fn title_scan(t: f32, i: usize, n: usize) -> Option<(f32, f32)> {
    let appear = i as f32 * TITLE_FADEIN;
    if t < appear {
        return None;
    }
    let k = ((t - appear) / TITLE_FXTIME).min(1.0);
    let grey = TITLE_COLOR2 + (1.0 - TITLE_COLOR2) * k;
    let fade_at = n as f32 * TITLE_FADEIN + TITLE_HOLD;
    let alpha = if t < fade_at { 1.0 } else { 1.0 - (t - fade_at) / TITLE_FADEOUT };
    (alpha > 0.0).then_some((grey, alpha))
}

/// "CHAPTER N / TITLE" when a chapter's first map starts: Portal 2's two
/// game_text lines (centred, y .55 and .6 of the screen), drawn by its HUD
/// message in InGameChapterTitle (UniversLTStd-Cn 19) and
/// InGameChapterSubtitle (UniversLTStd-BoldCn 34), basemodui_scheme.res.
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
    lines: Query<&Children, With<TitleScanLine>>,
    mut colors: Query<&mut TextColor>,
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
    if let Some((i, delay)) = show {
        let c = &campaign.chapters[i];
        // The game's own strings ("Chapter 1" / "The Courtesy Call"), upper
        // case as the HUD shows them.
        card.lines = Some((String::new(), c.title.to_uppercase(), c.subtitle.to_uppercase()));
        card.shown_at = time.elapsed_secs() + delay;
        for r in &roots {
            commands.entity(r).despawn();
        }
        // vgui `tall` is the Windows cell (1.265 em for Univers Cn).
        let mut line = |font: &Handle<Font>, tall: f32, y: f32, s: &str| {
            let size = bevy::text::FontSize::Vh(tall / 1.265 * 100.0 / 480.0);
            let spans: Vec<_> = s
                .chars()
                .map(|ch| (TextSpan::new(ch.to_string()), TextFont { font: font.into(), font_size: size, ..default() }, TextColor(Color::NONE)))
                .collect();
            let id = commands
                .spawn((
                    TitleScanLine,
                    Text::new(""),
                    TextFont { font: font.into(), font_size: size, ..default() },
                    bevy::text::LineHeight::RelativeToFont(1.265),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                ))
                .id();
            for span in spans {
                let child = commands.spawn(span).id();
                commands.entity(id).add_child(child);
            }
            // x -1: centred on the screen.
            let row = Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(y * 100.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            };
            commands.spawn(row).add_child(id).id()
        };
        let (_, a, b) = card.lines.clone().unwrap_or_default();
        let title = line(&fonts.menu, 19.0, 0.55, &a);
        let subtitle = line(&fonts.title, 34.0, 0.6, &b);
        commands
            .spawn((
                TitleCardRoot,
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                GlobalZIndex(5),
                bevy::picking::Pickable::IGNORE,
            ))
            .add_children(&[title, subtitle]);
    }
    if card.lines.is_none() {
        return;
    }
    let t = time.elapsed_secs() - card.shown_at;
    let mut alive = false;
    for spans in &lines {
        let n = spans.len();
        for (i, span) in spans.iter().enumerate() {
            let c = title_scan(t, i, n);
            alive |= c.is_some() || t < 0.0;
            if let Ok(mut tc) = colors.get_mut(span) {
                let (g, a) = c.unwrap_or((1.0, 0.0));
                let want = Color::srgba(g, g, g, a);
                if tc.0 != want {
                    tc.0 = want;
                }
            }
        }
    }
    if !alive && !lines.is_empty() {
        for r in &roots {
            commands.entity(r).despawn();
        }
        card.lines = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_title_scans_out_like_game_text_effect_2() {
        // The first character starts in color2 and reaches white after fxtime.
        assert_eq!(title_scan(0.0, 0, 9), Some((205.0 / 255.0, 1.0)));
        assert_eq!(title_scan(0.5, 0, 9), Some((1.0, 1.0)));
        // Later characters wait their turn, fadein apart.
        assert_eq!(title_scan(0.1, 2, 9), None);
        assert!(title_scan(0.13, 2, 9).is_some());
        // Hold 5 s after the scan, then fade out over 0.5 s.
        let fade_at = 9.0 * 0.06 + 5.0;
        assert_eq!(title_scan(fade_at - 0.01, 8, 9).unwrap().1, 1.0);
        assert!((title_scan(fade_at + 0.25, 8, 9).unwrap().1 - 0.5).abs() < 1e-4);
        assert_eq!(title_scan(fade_at + 0.5, 8, 9), None);
    }

    #[test]
    fn title_tabs_fit_their_titles() {
        assert_eq!(title_tab_tiles("CONTROLLER"), 4, "3 tiles clipped the R");
        // Unchanged from the measured screens.
        assert_eq!(title_tab_tiles("OPTIONS"), 3);
        assert_eq!(title_tab_tiles("SINGLE PLAYER"), 4);
        assert_eq!(title_tab_tiles("KEYBOARD/MOUSE"), 5);
    }

    #[test]
    fn pause_dialog_is_portal2s() {
        let labels: Vec<_> = pause_rows(true).iter().map(|r| r.0).collect();
        assert_eq!(labels, ["Return to Game", "Save Game", "Load Last Save", "Options", "Exit to Main Menu"]);
        assert!(pause_rows(false)[2].1.is_none());
    }

    #[test]
    fn options_dialog_fits_its_rows() {
        // options.res: four rows in a 3-tile body; Speedrun and Mods make 6.
        assert_eq!(dialog_tall(4), 3);
        assert_eq!(dialog_tall(5), 3);
        assert_eq!(dialog_tall(6), 4);
        // Every row (25 units from 25, 25 tall, centred) stays in the body.
        for n in 1..10 {
            assert!(25.0 * n as f32 + 12.5 <= 50.0 * dialog_tall(n) as f32, "{n}");
        }
    }

    #[test]
    fn save_game_lists_only_named_saves() {
        let save = |name: &str, kind: &str| SaveInfo {
            name: name.into(),
            header: crate::saves::SaveHeader { version: 1, map: "sp_a1_intro1".into(), saved_at: 0, kind: kind.into(), level_time: 0.0 },
        };
        let list = [save("autosave", "autosave"), save("mine", "save"), save("quick", "quick"), save("other", "save")];
        // Load Game shows every save; Save Game only those it can overwrite.
        assert_eq!(shown_saves(&list, false), vec![0, 1, 2, 3]);
        assert_eq!(shown_saves(&list, true), vec![1, 3]);
    }

    #[test]
    fn vfont_decodes_to_the_wrapped_font() {
        let font = b"\x00\x01\x00\x00\x00\x13 sfnt body \xff\x00\xa7".to_vec();
        // Encode the way the install's files are: key starts at an
        // arbitrary salt-derived byte, then previous encoded byte + 0xA7.
        let mut key = 0x18u8;
        let mut file: Vec<u8> = font.iter().map(|&p| {
            let c = p ^ key;
            key = c.wrapping_add(0xA7);
            c
        }).collect();
        file.extend_from_slice(b"salt");
        file.push(4);
        file.extend_from_slice(b"VFONT1");
        assert_eq!(decode_vfont(&file), Some(font));
        assert_eq!(decode_vfont(b"\x00\x01\x00\x00"), None);
        assert_eq!(decode_vfont(b"\x09VFONT1"), None);
    }

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

#[cfg(test)]
mod tiled_dialog_tests {
    use super::tile_piece;

    #[test]
    fn tab_over_body_pieces() {
        // A 3-tile tab over a 5 x 2 body (coopmode.res).
        let grid: Vec<Vec<Option<usize>>> = (0..=2).map(|r| (0..5).map(|c| tile_piece(c, r, 3, 5, 2)).collect()).collect();
        assert_eq!(grid[0], vec![Some(0), Some(1), Some(2), None, None]);
        assert_eq!(grid[1], vec![Some(3), Some(4), Some(4), Some(1), Some(2)]);
        assert_eq!(grid[2], vec![Some(6), Some(7), Some(7), Some(7), Some(8)]);
    }
}
