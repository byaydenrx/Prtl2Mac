//! Saved games, as Portal 2 keeps them: an autosave at every level start and
//! at each `logic_autosave` the map triggers, the quick save (F6 saves, F9
//! loads, or whatever `save_quick`/`load_quick` are bound to), named saves
//! from the pause menu's Save Game and the `save`/`load` console commands.
//! Dying loads the latest save of the level (the level from its start when
//! there is none). The main menu's Continue Game loads the newest save; Load
//! Game lists them all. Which chapters New Game offers follows the furthest
//! chapter reached (`sv_unlockedchapters`).
//!
//! A save is `<data dir>/saves/<name>.sav`: a one-line JSON header (version,
//! map, time; all the Load Game list reads), then the game state as JSON.
//! Newer fields have defaults and unknown ones are ignored, so saves load
//! across updates. A level-start autosave has no state: loading it starts
//! the level as the campaign does.
//!
//! What a save holds: the player (feet, view, velocity, crouch, health, gun),
//! both portals, every map entity's I/O state (enabled, killed, value, aux,
//! spawnflags, keyvalues and outputs that changed, transform, bounds), the
//! entities made since (template copies such as dropped cubes, script scene
//! entities), the pending outputs with their times, each entity's body
//! (pose, velocities, shown/hidden, solid, removed), movers (doors, trains,
//! parenting), animated props' sequences, the held prop, which scenes are
//! playing (they start again from the top), and the scripts' plain globals.
//! Not kept: paint, a scene's position within its lines, trigger touch and
//! look timers, attachments made by `SetParentAttachment`.
//!
//! Loading starts the map, holds the entity I/O (no `logic_auto`, no
//! outputs) while the level settles for a few frames, then puts the state
//! back before the next logic frame runs; portals and the held prop follow
//! one frame later, once the restored colliders are in place. Writing
//! happens on a background thread, so saving costs the frame only the copy.
use std::io::BufRead;
use std::path::PathBuf;

use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use p2_core::*;
use serde::{Deserialize, Serialize};

use crate::anim_props::Playback;
use crate::campaign::Campaign;
use crate::console::{Console, ConsolePrint};
use crate::health::PlayerHealth;
use crate::io::{EntInput, Ents, GameEnt, IoState, Output, Pending};
use crate::level::{CurrentMap, GameSet, LevelLoaded, LevelOwned, LoadLevel};
use crate::movers::{Movers, MoversSave};
use crate::portals::MapPortal;
use crate::props::{Held, Prop};
use crate::scenes::Playing;
use crate::settings::{data_dir, may_write_data};
use crate::templates::Templates;
use crate::triggers::ScreenFade;
use crate::viewcontrol::PropAnims;
use crate::vscripts::{Scripts, ScriptsSave};

pub struct SavesPlugin;

impl Plugin for SavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Restoring>()
            .init_resource::<SaveToast>()
            .init_resource::<PendingSaveShots>()
            .insert_resource(Progress::load())
            .add_message::<SaveGame>()
            .add_message::<LoadGame>()
            .add_systems(
                Update,
                (
                    (quick_keys, start_loads, hold_level, restore_state).chain().after(crate::vscripts::start_scripts).before(crate::io::run_io),
                    (restore_late, write_saves, note_progress).chain().after(crate::io::run_io),
                    draw_toast,
                    take_save_shots,
                )
                    .in_set(GameSet::Logic),
            );
    }
}

/// Written in each save's header; older saves (lower numbers) still load.
pub const SAVE_VERSION: u32 = 1;

/// Frames a loaded level settles (colliders, props, scripts) before its
/// saved state goes back in.
const SETTLE_FRAMES: u32 = 3;

/// Which save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The level's autosave (`logic_autosave`, a level start).
    Auto,
    Quick,
    /// A save by name (`save NAME`, the Save Game list).
    Named(String),
    /// Save Game's "new saved game": the next free `saveNN`.
    New,
    /// The newest save, of this map when given (dying, Continue Game).
    Latest(Option<String>),
}

/// Save the game now (ignored while there is no level to save, or dead).
#[derive(Message, Clone, Debug)]
pub struct SaveGame(pub Slot);

/// Load a save (a level start when it has no state).
#[derive(Message, Clone, Debug)]
pub struct LoadGame(pub Slot);

// ------------------------------------------------------------ the file --

/// The first line of a save: what the Load Game list shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveHeader {
    pub version: u32,
    pub map: String,
    /// Unix seconds.
    pub saved_at: u64,
    /// `autosave`, `quick` or `save` (named saves).
    pub kind: String,
    /// Game seconds into the level.
    #[serde(default)]
    pub level_time: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    /// Map entities when saved (the rest were made at run time).
    pub base_len: usize,
    pub player: PlayerSave,
    pub health: f32,
    pub portals: Vec<PortalSave>,
    /// Map entities whose I/O state differs from the map's.
    pub ents: Vec<EntSave>,
    /// Entities made since the level started, in order.
    pub extra: Vec<ExtraEnt>,
    pub io: IoSave,
    pub bodies: Vec<BodySave>,
    pub movers: MoversSave,
    pub anims: Vec<AnimSave>,
    pub anim_defaults: Vec<(usize, String)>,
    pub held: Option<HeldSave>,
    /// Scene entities that were playing.
    pub scenes: Vec<usize>,
    pub scripts: Option<ScriptsSave>,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            base_len: 0,
            player: PlayerSave::default(),
            health: 100.0,
            portals: Vec::new(),
            ents: Vec::new(),
            extra: Vec::new(),
            io: IoSave::default(),
            bodies: Vec::new(),
            movers: MoversSave::default(),
            anims: Vec::new(),
            anim_defaults: Vec::new(),
            held: None,
            scenes: Vec::new(),
            scripts: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerSave {
    pub feet: Vec3,
    /// Yaw, pitch, roll (radians).
    pub view: [f32; 3],
    pub velocity: Vec3,
    pub ducked: bool,
    pub can_fire: [bool; 2],
    /// 0 blue, 1 orange.
    pub last_placed: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalSave {
    /// 0 blue, 1 orange.
    pub kind: u8,
    pub pos: Vec3,
    pub rot: Quat,
    pub half: Vec2,
    /// The `prop_portal` it belongs to, for a map's own portals.
    pub map_ent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputSave {
    pub event: String,
    pub target: String,
    pub input: String,
    pub param: String,
    pub delay: f32,
    pub times: i32,
}

impl OutputSave {
    fn of(o: &Output) -> Self {
        OutputSave { event: o.event.clone(), target: o.target.clone(), input: o.input.clone(), param: o.param.clone(), delay: o.delay, times: o.times }
    }
    fn to_output(&self) -> Output {
        Output { event: self.event.clone(), target: self.target.clone(), input: self.input.clone(), param: self.param.clone(), delay: self.delay, times: self.times }
    }
}

/// A map entity's state: the plain fields always, keyvalues and outputs
/// only when they changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntSave {
    pub i: usize,
    pub enabled: bool,
    pub killed: bool,
    pub spawnflags: u32,
    pub value: f32,
    pub aux: f32,
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
    pub bounds: Option<(Vec3, Vec3)>,
    #[serde(default)]
    pub keyvalues: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub outputs: Option<Vec<OutputSave>>,
}

/// An entity made at run time, all of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtraEnt {
    pub class: String,
    pub name: String,
    pub keyvalues: Vec<(String, String)>,
    pub outputs: Vec<OutputSave>,
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
    pub brush_model: Option<usize>,
    pub bounds: Option<(Vec3, Vec3)>,
    pub enabled: bool,
    pub killed: bool,
    pub spawnflags: u32,
    pub value: f32,
    pub aux: f32,
    /// The template prototype it is a copy of (made again on load).
    pub proto: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IoSave {
    pub now: f32,
    pub queue: Vec<PendingSave>,
    pub script_hooks: Vec<(usize, String, String)>,
    pub script_calls: Vec<(usize, String, Option<usize>, usize)>,
    pub rng: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingSave {
    pub at: f32,
    pub target: String,
    pub input: String,
    pub param: String,
    pub caller: usize,
    pub activator: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyKind {
    Static,
    Dynamic,
    Kinematic,
}

/// The render/physics entity of a map entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodySave {
    pub ent: usize,
    /// Despawned (killed, fizzled, broken).
    pub gone: bool,
    pub pos: Vec3,
    pub rot: Quat,
    pub hidden: bool,
    /// `None`: no colliders.
    pub solid: Option<bool>,
    pub body: Option<BodyKind>,
    /// Linear and angular velocity of a moving body.
    pub velocity: Option<(Vec3, Vec3)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimSave {
    pub ent: usize,
    pub seq: String,
    pub time: f32,
    pub rate: f32,
    pub explicit: bool,
    pub finished: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldSave {
    pub ent: usize,
    pub yaw_offset: f32,
    pub distance: f32,
}

/// A whole save: header and state.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveFile {
    pub header: SaveHeader,
    pub state: Option<GameState>,
}

impl SaveFile {
    pub fn to_text(&self) -> Result<String, String> {
        let head = serde_json::to_string(&self.header).map_err(|e| e.to_string())?;
        let body = match &self.state {
            Some(s) => serde_json::to_string(s).map_err(|e| e.to_string())?,
            None => "null".into(),
        };
        Ok(format!("{head}\n{body}\n"))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let (head, body) = text.split_once('\n').unwrap_or((text, "null"));
        let header: SaveHeader = serde_json::from_str(head).map_err(|e| format!("header: {e}"))?;
        let body = body.trim();
        let state = if body.is_empty() || body == "null" { None } else { Some(serde_json::from_str(body).map_err(|e| format!("state: {e}"))?) };
        Ok(SaveFile { header, state })
    }
}

/// Where saves live.
pub fn saves_dir() -> PathBuf {
    let d = data_dir().join("saves");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn slot_path(name: &str) -> PathBuf {
    saves_dir().join(format!("{name}.sav"))
}

/// One line of the Load Game list.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveInfo {
    /// File name without `.sav`.
    pub name: String,
    pub header: SaveHeader,
}

/// Save names: one word, file friendly.
pub fn save_name(s: &str) -> Option<String> {
    crate::splits::checkpoint_name(s)
}

/// Every save, newest first (headers only).
pub fn list_saves() -> Vec<SaveInfo> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(saves_dir()) else { return out };
    for f in dir.flatten() {
        let path = f.path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
        let Ok(file) = std::fs::File::open(&path) else { continue };
        let mut line = String::new();
        if std::io::BufReader::new(file).read_line(&mut line).is_err() {
            continue;
        }
        if let Ok(header) = serde_json::from_str::<SaveHeader>(line.trim()) {
            out.push(SaveInfo { name, header });
        }
    }
    sort_newest_first(&mut out);
    out
}

fn sort_newest_first(saves: &mut [SaveInfo]) {
    saves.sort_by(|a, b| b.header.saved_at.cmp(&a.header.saved_at).then_with(|| a.name.cmp(&b.name)));
}

/// The save a slot names, from `saves` (newest first).
pub fn pick<'a>(saves: &'a [SaveInfo], slot: &Slot) -> Option<&'a SaveInfo> {
    match slot {
        Slot::Auto => saves.iter().find(|s| s.name == "autosave"),
        Slot::Quick => saves.iter().find(|s| s.name == "quick"),
        Slot::Named(n) => saves.iter().find(|s| s.name.eq_ignore_ascii_case(n)),
        Slot::New => None,
        Slot::Latest(None) => saves.first(),
        Slot::Latest(Some(m)) => saves.iter().find(|s| s.header.map.eq_ignore_ascii_case(m)),
    }
}

/// The first free `saveNN`.
pub fn new_save_name(saves: &[SaveInfo]) -> String {
    (1..).map(|n| format!("save{n:02}")).find(|n| !saves.iter().any(|s| s.name.eq_ignore_ascii_case(n))).unwrap_or_default()
}

/// The map Continue Game would load: the newest save's, else the old
/// `save.txt`'s.
pub fn latest_map() -> Option<String> {
    list_saves().first().map(|s| s.header.map.clone()).or_else(crate::ui::read_legacy_save)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Saves are written one at a time, off the main thread.
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Write `file` as `name` (the old autosave moves to `autosave01` first).
/// Saves waiting for their picture: (save name, real time to take it).
#[derive(Resource, Default)]
pub struct PendingSaveShots(Vec<(String, f32)>);

/// A save's picture for the Load/Save Game lists (Portal 2 keeps one per
/// save): `<save>.jpg` beside it.
pub fn save_picture_path(name: &str) -> PathBuf {
    saves_dir().join(format!("{name}.jpg"))
}

/// Take each waiting save's picture from the game view: once its time has
/// come and no menu is over the view (a save made from the pause menu is
/// pictured as play resumes). 400 x 225, JPEG, written off the main thread.
fn take_save_shots(mut commands: Commands, mut shots: ResMut<PendingSaveShots>, real: Res<Time<Real>>, screen: Res<crate::ui::Screen>, map: Option<Res<CurrentMap>>) {
    if shots.0.is_empty() || *screen != crate::ui::Screen::None || map.is_none_or(|m| m.menu) {
        return;
    }
    let now = real.elapsed_secs();
    let (due, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut shots.0).into_iter().partition(|(_, at)| *at <= now);
    shots.0 = wait;
    for (name, _) in due {
        let path = save_picture_path(&name);
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(move |shot: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            let (img, path) = (shot.image.clone(), path.clone());
            std::thread::spawn(move || match img.try_into_dynamic() {
                Ok(d) => {
                    let small = d.resize_exact(400, 225, image::imageops::FilterType::Triangle).to_rgb8();
                    if let Err(e) = small.save_with_format(&path, image::ImageFormat::Jpeg) {
                        warn!("portal2: can't write save picture {}: {e}", path.display());
                    }
                }
                Err(e) => warn!("portal2: save picture not converted: {e}"),
            });
        });
    }
}

fn write_in_background(name: String, file: SaveFile) {
    std::thread::spawn(move || {
        let _one = WRITING.lock();
        let text = match file.to_text() {
            Ok(t) => t,
            Err(e) => {
                warn!("portal2: can't write save {name}: {e}");
                return;
            }
        };
        let path = slot_path(&name);
        if name == "autosave" && path.exists() {
            let _ = std::fs::rename(&path, slot_path("autosave01"));
        }
        let tmp = path.with_extension("sav.tmp");
        match std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &path)) {
            Ok(()) => debug!("portal2: saved {}", path.display()),
            Err(e) => warn!("portal2: can't write {}: {e}", path.display()),
        }
    });
}

/// "Saturday, Jun 27 7:32 PM" in local time, as Portal 2's save lists
/// write a save's time.
pub fn time_text(unix: u64) -> String {
    let (y, mo, d, h, mi) = local_time(unix as i64);
    format_time(weekday(y, mo, d), mo, d, h, mi)
}

/// 0 Sunday .. 6 Saturday.
pub fn weekday(year: i32, month: u32, day: u32) -> u32 {
    // Days since 1970-01-01 (a Thursday), by Howard Hinnant's days_from_civil.
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days + 4).rem_euclid(7) as u32
}

pub fn format_time(weekday: u32, month: u32, day: u32, hour: u32, minute: u32) -> String {
    const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m = MONTHS.get(month.saturating_sub(1) as usize).copied().unwrap_or("?");
    let w = DAYS.get(weekday as usize).copied().unwrap_or("?");
    let (h12, ampm) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };
    format!("{w}, {m} {day} {h12}:{minute:02} {ampm}")
}

#[cfg(unix)]
fn local_time(t: i64) -> (i32, u32, u32, u32, u32) {
    let tt = t as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are to live locals; localtime_r only writes `tm`.
    if unsafe { libc::localtime_r(&tt, &mut tm) }.is_null() {
        return utc_time(t);
    }
    (tm.tm_year + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32, tm.tm_hour as u32, tm.tm_min as u32)
}

#[cfg(not(unix))]
fn local_time(t: i64) -> (i32, u32, u32, u32, u32) {
    utc_time(t)
}

/// Calendar date and time of Unix seconds `t`, UTC.
pub fn utc_time(t: i64) -> (i32, u32, u32, u32, u32) {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d, (secs / 3600) as u32, (secs % 3600 / 60) as u32)
}

// ---------------------------------------------------------- chapters --

/// How many chapters New Game offers (Portal 2's `sv_unlockedchapters`):
/// the furthest one reached, kept in `<data dir>/progress.cfg`.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Progress {
    pub unlocked: usize,
}

impl Progress {
    pub fn parse(text: &str) -> Option<usize> {
        text.lines().find_map(|l| {
            let (k, v) = l.trim().split_once(char::is_whitespace)?;
            (k == "sv_unlockedchapters").then(|| v.trim().parse::<usize>().ok()).flatten()
        })
    }

    pub fn to_text(&self) -> String {
        format!("version 1\nsv_unlockedchapters {}\n", self.unlocked)
    }

    fn load() -> Self {
        // Scripted tests may pick any chapter.
        if !may_write_data() {
            return Progress { unlocked: usize::MAX };
        }
        let unlocked = std::fs::read_to_string(data_dir().join("progress.cfg")).ok().and_then(|t| Self::parse(&t)).unwrap_or(0);
        Progress { unlocked: unlocked.max(1) }
    }

    /// Is chapter `i` (0-based) open in New Game?
    pub fn is_unlocked(&self, i: usize) -> bool {
        i < self.unlocked
    }
}

/// A campaign map was reached: its chapter (and all before) unlock. The
/// first time, the old `save.txt` and saves count as reached too.
fn note_progress(mut loaded: MessageReader<LevelLoaded>, campaign: Res<Campaign>, mut progress: ResMut<Progress>, mut first: Local<bool>) {
    let mut reached: Vec<String> = loaded.read().filter(|e| !e.menu).map(|e| e.map.clone()).collect();
    if !*first {
        *first = true;
        reached.extend(latest_map());
    }
    let mut best = progress.unlocked;
    for m in &reached {
        if let Some(c) = campaign.chapter_of(m) {
            best = best.max(c + 1);
        }
    }
    if best != progress.unlocked {
        progress.unlocked = best;
        if may_write_data() {
            let _ = std::fs::write(data_dir().join("progress.cfg"), progress.to_text());
        }
    }
}

// ------------------------------------------------------------- saving --

/// The ECS side of a snapshot: player, portals, bodies.
#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
struct Snap<'w, 's> {
    players: Query<'w, 's, (&'static Transform, &'static ViewAngles, &'static CharacterVelocity, &'static Crouch, &'static PortalGun), With<Player>>,
    portals: Query<'w, 's, (&'static Portal, &'static Transform, Option<&'static MapPortal>)>,
    bodies: Query<
        'w,
        's,
        (
            &'static Transform,
            Option<&'static Visibility>,
            Option<&'static RigidBody>,
            Option<&'static LinearVelocity>,
            Option<&'static AngularVelocity>,
        ),
        Without<Player>,
    >,
    children: Query<'w, 's, &'static Children>,
    colliders: Query<'w, 's, Has<ColliderDisabled>, With<Collider>>,
    held: Query<'w, 's, (&'static Prop, &'static Held)>,
}

/// Game state other modules keep.
#[derive(SystemParam)]
struct Modules<'w> {
    movers: ResMut<'w, Movers>,
    anims: ResMut<'w, PropAnims>,
    scripts: ResMut<'w, Scripts>,
    templates: ResMut<'w, Templates>,
    health: ResMut<'w, PlayerHealth>,
    playing: Res<'w, Playing>,
}

fn kind_of_body(b: &RigidBody) -> BodyKind {
    match b {
        RigidBody::Dynamic => BodyKind::Dynamic,
        RigidBody::Kinematic => BodyKind::Kinematic,
        _ => BodyKind::Static,
    }
}

/// Every entity's state as it differs from the map's own (`base`).
fn snapshot_ents(ents: &Ents, base: &Ents, templates: &Templates) -> (Vec<EntSave>, Vec<ExtraEnt>) {
    let mut saved = Vec::new();
    for (i, e) in ents.list.iter().enumerate().take(base.list.len()) {
        let b = &base.list[i];
        let kv_changed = e.keyvalues != b.keyvalues;
        let outputs_changed = e.outputs.len() != b.outputs.len() || e.outputs.iter().zip(&b.outputs).any(|(x, y)| x.times != y.times || x.event != y.event || x.target != y.target);
        let same = e.enabled == b.enabled
            && e.killed == b.killed
            && e.spawnflags == b.spawnflags
            && e.value == b.value
            && e.aux == b.aux
            && e.transform == b.transform
            && e.bounds == b.bounds
            && !kv_changed
            && !outputs_changed;
        if same {
            continue;
        }
        saved.push(EntSave {
            i,
            enabled: e.enabled,
            killed: e.killed,
            spawnflags: e.spawnflags,
            value: e.value,
            aux: e.aux,
            pos: e.transform.translation,
            rot: e.transform.rotation,
            scale: e.transform.scale,
            bounds: e.bounds,
            keyvalues: kv_changed.then(|| e.keyvalues.clone()),
            outputs: outputs_changed.then(|| e.outputs.iter().map(OutputSave::of).collect()),
        });
    }
    let extra = ents
        .list
        .iter()
        .enumerate()
        .skip(base.list.len())
        .map(|(i, e)| ExtraEnt {
            class: e.class.clone(),
            name: e.name.clone(),
            keyvalues: e.keyvalues.clone(),
            outputs: e.outputs.iter().map(OutputSave::of).collect(),
            pos: e.transform.translation,
            rot: e.transform.rotation,
            scale: e.transform.scale,
            brush_model: e.brush_model,
            bounds: e.bounds,
            enabled: e.enabled,
            killed: e.killed,
            spawnflags: e.spawnflags,
            value: e.value,
            aux: e.aux,
            proto: templates.prototype_of(i),
        })
        .collect();
    (saved, extra)
}

fn io_save(s: &IoState) -> IoSave {
    IoSave {
        now: s.now,
        queue: s
            .queue
            .iter()
            .map(|p| PendingSave { at: p.at, target: p.target.clone(), input: p.input.clone(), param: p.param.clone(), caller: p.caller, activator: p.activator })
            .collect(),
        script_hooks: s.script_hooks.clone(),
        script_calls: s.script_calls.clone(),
        rng: s.rng,
    }
}

/// Put map entities back to the map's own state, then apply the saved
/// differences and the entities made at run time (without their bodies).
pub fn restore_ents(ents: &mut Ents, base: &Ents, state: &GameState) {
    let n = base.list.len().min(ents.list.len());
    ents.truncate(n);
    for i in 0..n {
        let (e, b) = (&mut ents.list[i], &base.list[i]);
        e.enabled = b.enabled;
        e.killed = b.killed;
        e.spawnflags = b.spawnflags;
        e.value = b.value;
        e.aux = b.aux;
        e.transform = b.transform;
        e.bounds = b.bounds;
        e.keyvalues = b.keyvalues.clone();
        e.outputs = b.outputs.clone();
    }
    for s in &state.ents {
        let Some(e) = ents.list.get_mut(s.i) else { continue };
        e.enabled = s.enabled;
        e.killed = s.killed;
        e.spawnflags = s.spawnflags;
        e.value = s.value;
        e.aux = s.aux;
        e.transform = Transform { translation: s.pos, rotation: s.rot, scale: s.scale };
        e.bounds = s.bounds;
        if let Some(kv) = &s.keyvalues {
            e.keyvalues = kv.clone();
        }
        if let Some(o) = &s.outputs {
            e.outputs = o.iter().map(OutputSave::to_output).collect();
        }
    }
    for x in &state.extra {
        ents.add(GameEnt {
            class: x.class.clone(),
            name: x.name.clone(),
            keyvalues: x.keyvalues.clone(),
            outputs: x.outputs.iter().map(OutputSave::to_output).collect(),
            entity: None,
            transform: Transform { translation: x.pos, rotation: x.rot, scale: x.scale },
            brush_model: x.brush_model,
            bounds: x.bounds,
            enabled: x.enabled,
            killed: x.killed,
            spawnflags: x.spawnflags,
            value: x.value,
            aux: x.aux,
        });
    }
    ents.set_io_state(IoState {
        now: state.io.now,
        queue: state
            .io
            .queue
            .iter()
            .map(|p| Pending { at: p.at, target: p.target.clone(), input: p.input.clone(), param: p.param.clone(), caller: p.caller, activator: p.activator })
            .collect(),
        script_hooks: state.io.script_hooks.clone(),
        script_calls: state.io.script_calls.clone(),
        started: true,
        rng: state.io.rng,
    });
}

#[allow(clippy::too_many_arguments)]
fn write_saves(
    mut requests: MessageReader<SaveGame>,
    mut loaded: MessageReader<LevelLoaded>,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    map: Option<Res<CurrentMap>>,
    mut restoring: ResMut<Restoring>,
    w: Snap,
    m: Modules,
    mut print: MessageWriter<ConsolePrint>,
    mut toast: ResMut<SaveToast>,
    real: Res<Time<Real>>,
    mut shots: ResMut<PendingSaveShots>,
) {
    let mut want: Vec<Slot> = requests.read().map(|r| r.0.clone()).collect();
    // `logic_autosave`: Save, and SaveDangerous (kept when the player is
    // not about to die; here: alive).
    if let Some(ents) = &ents {
        for i in inputs.read() {
            if ents.list.get(i.target).is_some_and(|e| e.class == "logic_autosave") && matches!(i.input.as_str(), "save" | "savedangerous") {
                want.push(Slot::Auto);
            }
        }
    } else {
        inputs.clear();
    }
    // A level start: the autosave is the level itself (no state).
    let started: Vec<String> = loaded.read().filter(|e| !e.menu).map(|e| e.map.clone()).collect();
    if !may_write_data() {
        return;
    }
    let from_save = !started.is_empty() && std::mem::take(&mut restoring.from_save);
    // (A `logic_autosave` the level fired at once is the better autosave.)
    if !from_save && !want.contains(&Slot::Auto) {
        if let Some(map) = started.last() {
            let header = SaveHeader { version: SAVE_VERSION, map: map.clone(), saved_at: now_unix(), kind: "autosave".into(), level_time: 0.0 };
            write_in_background("autosave".into(), SaveFile { header, state: None });
            // Its picture once the level is drawn.
            shots.0.push(("autosave".into(), real.elapsed_secs() + 1.5));
            // Older builds read this for Continue.
            let _ = std::fs::write(data_dir().join("save.txt"), crate::ui::legacy_save_text(map));
        }
    }
    if want.is_empty() {
        return;
    }
    let (Some(ents), Some(map)) = (ents, map) else { return };
    if map.menu || restoring.save.is_some() || m.health.dead() || ents.hold {
        print.write(ConsolePrint("Can't save now.".into()));
        return;
    }
    let state = snapshot(&ents, &map, &w, &m);
    let saves = list_saves();
    for slot in want {
        let (name, kind) = match &slot {
            Slot::Auto => ("autosave".to_string(), "autosave"),
            Slot::Quick => ("quick".to_string(), "quick"),
            Slot::Named(n) => (n.clone(), "save"),
            Slot::New => (new_save_name(&saves), "save"),
            Slot::Latest(_) => continue,
        };
        let header = SaveHeader { version: SAVE_VERSION, map: map.info.name.clone(), saved_at: now_unix(), kind: kind.into(), level_time: ents.now };
        write_in_background(name.clone(), SaveFile { header, state: Some(state.clone()) });
        shots.0.push((name.clone(), real.elapsed_secs()));
        print.write(ConsolePrint(format!("Saved {name} ({})", map.info.name)));
        if kind != "autosave" {
            toast.0 = Some(real.elapsed_secs() + 1.5);
        }
    }
}

fn snapshot(ents: &Ents, map: &CurrentMap, w: &Snap, m: &Modules) -> GameState {
    let base = Ents::from_map(&map.info, &map.model_bounds);
    let (saved, extra) = snapshot_ents(ents, &base, &m.templates);
    let mut state = GameState { base_len: base.list.len(), ents: saved, extra, io: io_save(&ents.io_state()), health: m.health.hp, ..default() };
    if let Ok((tf, view, vel, crouch, gun)) = w.players.single() {
        state.player = PlayerSave {
            feet: tf.translation,
            view: [view.yaw, view.pitch, view.roll],
            velocity: vel.0,
            ducked: crouch.ducked,
            can_fire: gun.can_fire,
            last_placed: gun.last_placed.map(|k| k.index() as u8),
        };
        for p in gun.portals.iter().flatten() {
            if let Ok((portal, ptf, mp)) = w.portals.get(*p) {
                state.portals.push(PortalSave { kind: portal.kind.index() as u8, pos: ptf.translation, rot: ptf.rotation, half: portal.half_size, map_ent: mp.map(|m| m.ent) });
            }
        }
    }
    for (i, e) in ents.list.iter().enumerate() {
        let Some(root) = e.entity else { continue };
        let Ok((tf, vis, rb, lv, av)) = w.bodies.get(root) else {
            state.bodies.push(BodySave { ent: i, gone: true, pos: Vec3::ZERO, rot: Quat::IDENTITY, hidden: true, solid: None, body: None, velocity: None });
            continue;
        };
        let mut solid = None;
        for c in w.children.iter_descendants(root).chain(std::iter::once(root)) {
            if let Ok(disabled) = w.colliders.get(c) {
                solid = Some(solid.unwrap_or(true) && !disabled);
            }
        }
        let body = rb.map(kind_of_body);
        let velocity = match (body, lv, av) {
            (Some(BodyKind::Dynamic), Some(l), Some(a)) => Some((l.0, a.0)),
            _ => None,
        };
        state.bodies.push(BodySave { ent: i, gone: false, pos: tf.translation, rot: tf.rotation, hidden: vis == Some(&Visibility::Hidden), solid, body, velocity });
    }
    state.movers = m.movers.save();
    let mut anims: Vec<AnimSave> = m
        .anims
        .playing
        .iter()
        .map(|(&ent, p)| AnimSave { ent, seq: p.seq.clone(), time: p.time, rate: p.rate, explicit: p.explicit, finished: p.finished })
        .collect();
    anims.sort_by_key(|a| a.ent);
    state.anims = anims;
    let mut defaults: Vec<(usize, String)> = m.anims.defaults.iter().map(|(k, v)| (*k, v.clone())).collect();
    defaults.sort();
    state.anim_defaults = defaults;
    state.held = w.held.iter().next().map(|(p, h)| HeldSave { ent: p.ent, yaw_offset: h.yaw_offset, distance: h.distance });
    state.scenes = (0..ents.list.len()).filter(|&i| m.playing.is_playing(i)).collect();
    state.scripts = m.scripts.save();
    state
}

// ------------------------------------------------------------ loading --

/// A save being put back after its level loads.
#[derive(Resource, Default)]
pub struct Restoring {
    pub save: Option<GameState>,
    map: String,
    stage: Stage,
    /// The level loading now came from a save: no level-start autosave.
    from_save: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// Waiting for the level.
    #[default]
    Loading,
    /// The level is up, I/O held; frames left to settle.
    Settling(u32),
    /// State is back; portals and the held prop next frame.
    Late,
}

/// F6 / F9 (or their bindings) while playing.
#[allow(clippy::too_many_arguments)]
fn quick_keys(
    bindings: Option<Res<InputBindings>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    screen: Res<crate::ui::Screen>,
    console: Res<Console>,
    map: Option<Res<CurrentMap>>,
    mut save: MessageWriter<SaveGame>,
    mut load: MessageWriter<LoadGame>,
) {
    let Some(b) = bindings else { return };
    if console.open || *screen != crate::ui::Screen::None || map.is_none_or(|m| m.menu) {
        return;
    }
    let input = InputState { keys: keys.as_deref(), mouse: mouse.as_deref(), scroll: None };
    if b.just_pressed("save_quick", &input) {
        save.write(SaveGame(Slot::Quick));
    }
    if b.just_pressed("load_quick", &input) {
        load.write(LoadGame(Slot::Quick));
    }
}

/// Read the save a load asks for and start its level.
fn start_loads(
    mut requests: MessageReader<LoadGame>,
    mut restoring: ResMut<Restoring>,
    mut load: MessageWriter<LoadLevel>,
    mut print: MessageWriter<ConsolePrint>,
) {
    let Some(req) = requests.read().last().cloned() else { return };
    let saves = list_saves();
    let Some(info) = pick(&saves, &req.0) else {
        match &req.0 {
            // Dying with no save of this level: the level from the start.
            Slot::Latest(Some(map)) => {
                load.write(LoadLevel { map: map.clone(), menu: false });
            }
            // Continue Game from before saves existed.
            Slot::Latest(None) => match crate::ui::read_legacy_save() {
                Some(m) => {
                    load.write(LoadLevel { map: m, menu: false });
                }
                None => {
                    print.write(ConsolePrint("No saved game.".into()));
                }
            },
            s => {
                print.write(ConsolePrint(format!("No saved game {s:?}.")));
            }
        }
        return;
    };
    let file = match std::fs::read_to_string(slot_path(&info.name)).map_err(|e| e.to_string()).and_then(|t| SaveFile::parse(&t)) {
        Ok(f) => f,
        Err(e) => {
            warn!("portal2: can't read save {}: {e}", info.name);
            print.write(ConsolePrint(format!("Can't load {}: {e}", info.name)));
            return;
        }
    };
    info!("portal2: loading save {} ({})", info.name, file.header.map);
    load.write(LoadLevel { map: file.header.map.clone(), menu: false });
    *restoring = Restoring { save: file.state, map: file.header.map, stage: Stage::Loading, from_save: true };
}

/// The level of a save came up: hold its I/O (no autos, no outputs) and put
/// the player where they were, behind a black screen, while it settles.
#[allow(clippy::too_many_arguments)]
fn hold_level(
    mut loaded: MessageReader<LevelLoaded>,
    mut restoring: ResMut<Restoring>,
    ents: Option<ResMut<Ents>>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity), With<Player>>,
    mut fade: ResMut<ScreenFade>,
    time: Res<Time>,
) {
    let arrived = loaded.read().last().cloned();
    if restoring.save.is_none() {
        return;
    }
    if let Some(ev) = arrived {
        if ev.menu || !ev.map.eq_ignore_ascii_case(&restoring.map) {
            // Something else loaded instead (Quit to menu): drop the save.
            *restoring = Restoring::default();
            return;
        }
        let Some(mut ents) = ents else { return };
        ents.hold = true;
        if let (Some(s), Ok((mut tf, mut feet, mut view, mut vel))) = (&restoring.save, players.single_mut()) {
            put_player(&s.player, &mut tf, &mut feet, &mut view, &mut vel);
        }
        *fade = ScreenFade { color: Vec4::new(0.0, 0.0, 0.0, 1.0), start: time.elapsed_secs() + 0.15, duration: 0.6, hold: 0.0, out: false, active: true, stay: false };
        restoring.stage = Stage::Settling(SETTLE_FRAMES);
    }
}

fn put_player(p: &PlayerSave, tf: &mut Transform, feet: &mut InterpolatedFeet, view: &mut ViewAngles, vel: &mut CharacterVelocity) {
    tf.translation = p.feet;
    *feet = InterpolatedFeet { prev: p.feet, curr: p.feet };
    *view = ViewAngles { yaw: p.view[0], pitch: p.view[1], roll: p.view[2] };
    vel.0 = p.velocity;
}

/// The ECS side of a restore.
#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
struct Bodies<'w, 's> {
    players: Query<'w, 's, (Entity, &'static mut Transform, &'static mut InterpolatedFeet, &'static mut ViewAngles, &'static mut CharacterVelocity, &'static mut PortalGun), With<Player>>,
    bodies: Query<
        'w,
        's,
        (
            &'static mut Transform,
            Option<&'static RigidBody>,
            Option<&'static mut Position>,
            Option<&'static mut Rotation>,
            Option<&'static mut LinearVelocity>,
            Option<&'static mut AngularVelocity>,
        ),
        Without<Player>,
    >,
    children: Query<'w, 's, &'static Children>,
    colliders: Query<'w, 's, (), With<Collider>>,
    props: Query<'w, 's, &'static Prop>,
}

/// After settling: entities, bodies, movers, animations, scripts, health.
#[allow(clippy::too_many_arguments)]
fn restore_state(
    mut commands: Commands,
    mut restoring: ResMut<Restoring>,
    ents: Option<ResMut<Ents>>,
    map: Option<Res<CurrentMap>>,
    mut b: Bodies,
    mut m: Modules,
    mut duck: MessageWriter<ForceDuck>,
) {
    let Stage::Settling(n) = restoring.stage else { return };
    if n > 0 {
        restoring.stage = Stage::Settling(n - 1);
        return;
    }
    let (Some(mut ents), Some(map), Some(state)) = (ents, map, restoring.save.take()) else {
        *restoring = Restoring::default();
        return;
    };
    let base = Ents::from_map(&map.info, &map.model_bounds);
    if base.list.len() != state.base_len {
        warn!("portal2: the save's map has changed ({} entities, now {}); loading what still fits", state.base_len, base.list.len());
    }
    restore_ents(&mut ents, &base, &state);
    // Bodies of map entities.
    for s in &state.bodies {
        let Some(root) = ents.list.get(s.ent).and_then(|e| e.entity) else { continue };
        if s.gone {
            if let Ok(mut ec) = commands.get_entity(root) {
                ec.despawn();
            }
            continue;
        }
        let Ok((mut tf, rb, pos, rot, lv, av)) = b.bodies.get_mut(root) else { continue };
        tf.translation = s.pos;
        tf.rotation = s.rot;
        if let Some(mut p) = pos {
            p.0 = s.pos;
        }
        if let Some(mut r) = rot {
            r.0 = s.rot;
        }
        let (l, a) = s.velocity.unwrap_or((Vec3::ZERO, Vec3::ZERO));
        if let Some(mut v) = lv {
            v.0 = l;
        }
        if let Some(mut v) = av {
            v.0 = a;
        }
        let mut ec = commands.entity(root);
        ec.insert(if s.hidden { Visibility::Hidden } else { Visibility::Inherited });
        if let (Some(want), Some(have)) = (s.body, rb.map(kind_of_body)) {
            if want != have {
                ec.insert(match want {
                    BodyKind::Dynamic => RigidBody::Dynamic,
                    BodyKind::Kinematic => RigidBody::Kinematic,
                    BodyKind::Static => RigidBody::Static,
                });
            }
        }
        if let Some(solid) = s.solid {
            for c in b.children.iter_descendants(root).chain(std::iter::once(root)) {
                if b.colliders.contains(c) {
                    if solid {
                        commands.entity(c).remove::<ColliderDisabled>();
                    } else {
                        commands.entity(c).insert(ColliderDisabled);
                    }
                }
            }
        }
    }
    // Template copies (dropped cubes...) made again.
    let body_of = |i: usize| state.bodies.iter().find(|s| s.ent == i);
    for (k, x) in state.extra.iter().enumerate() {
        let idx = state.base_len + k;
        let (Some(proto), Some(body)) = (x.proto, body_of(idx)) else { continue };
        if body.gone || idx >= ents.list.len() || proto >= ents.list.len() {
            continue;
        }
        let tf = Transform { translation: body.pos, rotation: body.rot, scale: x.scale };
        let copy = crate::templates::spawn_copy(&mut commands, &ents, proto, idx, tf, &b.props);
        if let Some(c) = copy {
            if let Some((l, a)) = body.velocity {
                commands.entity(c).insert((LinearVelocity(l), AngularVelocity(a)));
            }
            if body.hidden {
                commands.entity(c).insert(Visibility::Hidden);
            }
        }
        ents.list[idx].entity = copy;
        m.templates.note_copy(idx, proto);
    }
    m.movers.restore(&state.movers);
    m.anims.playing = state
        .anims
        .iter()
        .map(|a| (a.ent, Playback { time: a.time, rate: a.rate, finished: a.finished, ..Playback::new(&a.seq, a.explicit) }))
        .collect();
    m.anims.defaults = state.anim_defaults.iter().cloned().collect();
    if let Some(s) = &state.scripts {
        m.scripts.restore(s);
    }
    m.health.hp = state.health;
    // Scenes start over from their first line.
    for &i in &state.scenes {
        if i < ents.list.len() {
            ents.input_to(i, "start");
        }
    }
    if let Ok((player, mut tf, mut feet, mut view, mut vel, mut gun)) = b.players.single_mut() {
        put_player(&state.player, &mut tf, &mut feet, &mut view, &mut vel);
        gun.can_fire = state.player.can_fire;
        gun.last_placed = state.player.last_placed.map(|k| if k == 1 { PortalKind::Orange } else { PortalKind::Blue });
        if state.player.ducked {
            duck.write(ForceDuck { entity: player });
        }
    }
    ents.hold = false;
    info!("portal2: save restored ({} entities changed, {} made, {} pending outputs)", state.ents.len(), state.extra.len(), state.io.queue.len());
    restoring.save = Some(state);
    restoring.stage = Stage::Late;
}

/// One frame after the state: portals (on the restored walls) and the held prop.
#[allow(clippy::too_many_arguments)]
fn restore_late(
    mut commands: Commands,
    mut restoring: ResMut<Restoring>,
    spatial: SpatialQuery,
    time: Res<Time>,
    mut players: Query<(Entity, &mut PortalGun, &mut IgnoreColliders), With<Player>>,
    props: Query<(Entity, &Prop)>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
    walls: crate::portals::WallBodies,
) {
    if restoring.stage != Stage::Late {
        return;
    }
    let state = restoring.save.take();
    *restoring = Restoring::default();
    let (Some(state), Ok((player, mut gun, mut ignore))) = (state, players.single_mut()) else { return };
    for p in gun.portals.iter_mut() {
        if let Some(e) = p.take() {
            commands.entity(e).despawn();
        }
    }
    for p in &state.portals {
        let kind = if p.kind == 1 { PortalKind::Orange } else { PortalKind::Blue };
        let tf = Transform::from_translation(p.pos).with_rotation(p.rot);
        let surfaces = crate::portals::surfaces_behind(&spatial, &tf, p.half, player, &walls);
        let mut ec = commands.spawn((Name::new(format!("{kind:?} portal")), Portal { kind, half_size: p.half, surfaces, placed_at: time.elapsed_secs() }, tf));
        if let Some(ent) = p.map_ent {
            ec.insert((MapPortal { ent }, LevelOwned));
        }
        gun.portals[kind.index()] = Some(ec.id());
    }
    if let [Some(a), Some(b)] = gun.portals {
        commands.entity(a).insert(PortalLink(b));
        commands.entity(b).insert(PortalLink(a));
    }
    if let Some(h) = &state.held {
        if let Some((body, _)) = props.iter().find(|(_, p)| p.ent == h.ent) {
            let held = Held { yaw_offset: h.yaw_offset, distance: h.distance, stuck_for: 0.0 };
            crate::props::start_holding(&mut commands, body, held, &children, &colliders, &mut ignore);
        }
    }
}

/// "Saving..." at the bottom right after a quick or named save.
#[derive(Resource, Default)]
pub struct SaveToast(Option<f32>);

#[derive(Component)]
struct ToastText;

fn draw_toast(mut commands: Commands, mut toast: ResMut<SaveToast>, real: Res<Time<Real>>, shown: Query<Entity, With<ToastText>>, fonts: Option<Res<crate::ui::UiFonts>>) {
    let on = toast.0.is_some_and(|t| real.elapsed_secs() < t);
    if !on {
        toast.0 = None;
        for e in &shown {
            commands.entity(e).despawn();
        }
        return;
    }
    let Some(fonts) = fonts else { return };
    if shown.is_empty() {
        commands.spawn((
            ToastText,
            Node { position_type: PositionType::Absolute, right: Val::Percent(3.0), bottom: Val::Percent(4.0), ..default() },
            GlobalZIndex(40),
            bevy::picking::Pickable::IGNORE,
            Text::new("SAVING..."),
            TextFont { font: fonts.menu.clone().into(), font_size: bevy::text::FontSize::Px(18.0), ..default() },
            TextColor(Color::srgba(0.92, 0.94, 0.95, 0.85)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(map: &str, at: u64) -> SaveHeader {
        SaveHeader { version: SAVE_VERSION, map: map.into(), saved_at: at, kind: "save".into(), level_time: 12.5 }
    }

    fn sample_state() -> GameState {
        GameState {
            base_len: 3,
            player: PlayerSave { feet: Vec3::new(1.0, 2.0, 3.0), view: [0.5, -0.1, 0.0], velocity: Vec3::X, ducked: true, can_fire: [true, false], last_placed: Some(0) },
            health: 75.0,
            portals: vec![PortalSave { kind: 1, pos: Vec3::Y, rot: Quat::from_rotation_y(1.0), half: Vec2::new(0.6, 1.0), map_ent: Some(2) }],
            ents: vec![EntSave {
                i: 1,
                enabled: false,
                killed: false,
                spawnflags: 4,
                value: 2.0,
                aux: 0.0,
                pos: Vec3::ZERO,
                rot: Quat::IDENTITY,
                scale: Vec3::ONE,
                bounds: None,
                keyvalues: Some(vec![("max".into(), "3".into())]),
                outputs: None,
            }],
            io: IoSave {
                now: 30.0,
                queue: vec![PendingSave { at: 31.5, target: "door".into(), input: "open".into(), param: String::new(), caller: 1, activator: Some(0) }],
                ..default()
            },
            bodies: vec![BodySave { ent: 2, gone: false, pos: Vec3::Z, rot: Quat::IDENTITY, hidden: true, solid: Some(false), body: Some(BodyKind::Dynamic), velocity: Some((Vec3::X, Vec3::Y)) }],
            anims: vec![AnimSave { ent: 2, seq: "open".into(), time: 0.4, rate: 1.0, explicit: true, finished: false }],
            held: Some(HeldSave { ent: 2, yaw_offset: 0.3, distance: 1.2 }),
            scenes: vec![1],
            scripts: Some(ScriptsSave {
                root: vec![("progress".into(), crate::vscripts::PlainValue::Int(3)), ("names".into(), crate::vscripts::PlainValue::Array(vec![crate::vscripts::PlainValue::Str("a".into())]))],
                ..default()
            }),
            ..default()
        }
    }

    #[test]
    fn save_file_round_trip() {
        let f = SaveFile { header: header("sp_a2_laser_intro", 1_760_000_000), state: Some(sample_state()) };
        let text = f.to_text().unwrap();
        assert_eq!(text.lines().count(), 2, "header line, state line");
        assert_eq!(SaveFile::parse(&text).unwrap(), f);
        // A level-start autosave has no state.
        let fresh = SaveFile { header: header("sp_a1_intro1", 5), state: None };
        assert_eq!(SaveFile::parse(&fresh.to_text().unwrap()).unwrap(), fresh);
    }

    #[test]
    fn older_and_newer_saves_still_load() {
        // From an older version: fields added since are missing.
        let old = "{\"version\":0,\"map\":\"sp_a1_intro3\",\"saved_at\":10,\"kind\":\"quick\"}\n{\"base_len\":5,\"health\":50.0}\n";
        let f = SaveFile::parse(old).unwrap();
        assert_eq!(f.header.level_time, 0.0);
        let s = f.state.unwrap();
        assert_eq!((s.base_len, s.health), (5, 50.0));
        assert!(s.ents.is_empty() && s.scripts.is_none());
        // From a newer one: unknown fields are skipped.
        let new = "{\"version\":9,\"map\":\"m\",\"saved_at\":1,\"kind\":\"save\",\"thumbnail\":\"x\"}\n{\"base_len\":1,\"paint\":[1,2]}\n";
        assert_eq!(SaveFile::parse(new).unwrap().state.unwrap().base_len, 1);
        assert!(SaveFile::parse("not json").is_err());
    }

    #[test]
    fn slots_pick_saves() {
        let s = |name: &str, map: &str, at: u64| SaveInfo { name: name.into(), header: header(map, at) };
        let mut saves = vec![s("autosave", "sp_a2_bridge_intro", 100), s("quick", "sp_a2_laser_intro", 300), s("save01", "sp_a2_bridge_intro", 200)];
        sort_newest_first(&mut saves);
        assert_eq!(saves[0].name, "quick");
        assert_eq!(pick(&saves, &Slot::Latest(None)).unwrap().name, "quick");
        assert_eq!(pick(&saves, &Slot::Latest(Some("sp_a2_bridge_intro".into()))).unwrap().name, "save01");
        assert_eq!(pick(&saves, &Slot::Latest(Some("sp_a1_intro1".into()))), None);
        assert_eq!(pick(&saves, &Slot::Auto).unwrap().header.map, "sp_a2_bridge_intro");
        assert_eq!(pick(&saves, &Slot::Named("SAVE01".into())).unwrap().name, "save01");
        assert_eq!(new_save_name(&saves), "save02");
        assert_eq!(new_save_name(&[]), "save01");
    }

    #[test]
    fn progress_file() {
        let p = Progress { unlocked: 4 };
        assert_eq!(Progress::parse(&p.to_text()), Some(4));
        assert_eq!(Progress::parse("version 1\n"), None);
        assert!(p.is_unlocked(3) && !p.is_unlocked(4));
    }

    #[test]
    fn calendar() {
        assert_eq!(utc_time(0), (1970, 1, 1, 0, 0));
        // 2026-10-18 14:03 UTC.
        assert_eq!(utc_time(1_792_332_180), (2026, 10, 18, 14, 3));
        assert_eq!(utc_time(951_782_400), (2000, 2, 29, 0, 0));
        // Portal 2's style (owner spec 2026-10-21).
        assert_eq!(format_time(weekday(2020, 6, 27), 6, 27, 19, 32), "Saturday, Jun 27 7:32 PM");
        assert_eq!(format_time(weekday(2026, 10, 21), 10, 21, 0, 24), "Wednesday, Oct 21 12:24 AM");
        assert_eq!(weekday(1970, 1, 1), 4);
        assert_eq!(weekday(2000, 2, 29), 2);
    }

    fn ent(class: &str, name: &str) -> GameEnt {
        GameEnt::logic(class, name, vec![("max".into(), "5".into())])
    }

    fn ents_of(list: Vec<GameEnt>) -> Ents {
        let mut e = Ents::default();
        for g in list {
            e.add(g);
        }
        e
    }

    #[test]
    fn entity_state_round_trip() {
        let base = ents_of(vec![ent("player", "!player"), ent("math_counter", "count"), ent("logic_relay", "relay")]);
        let mut live = ents_of(base.list.clone());
        live.list[1].value = 3.0;
        live.list[1].keyvalues.push(("min".into(), "1".into()));
        live.list[2].killed = true;
        live.add(ent("logic_choreographed_scene", "@scene"));
        live.send("relay", "trigger", "", 2.0);
        let templates = Templates::default();
        let (saved, extra) = snapshot_ents(&live, &base, &templates);
        assert_eq!(saved.iter().map(|s| s.i).collect::<Vec<_>>(), vec![1, 2], "only changed entities");
        assert!(saved[0].keyvalues.is_some() && saved[1].keyvalues.is_none());
        let state = GameState { base_len: 3, ents: saved, extra, io: io_save(&live.io_state()), ..default() };
        // Into a fresh level that made an entity of its own at spawn.
        let mut fresh = ents_of(base.list.clone());
        fresh.add(ent("info_target", "made_at_spawn"));
        restore_ents(&mut fresh, &base, &state);
        assert_eq!(fresh.list.len(), 4);
        assert_eq!(fresh.list[1].value, 3.0);
        assert_eq!(fresh.list[1].get("min"), Some("1"));
        assert!(fresh.list[2].killed);
        assert_eq!(fresh.list[3].name, "@scene");
        assert_eq!(fresh.find("@scene", 0, None), vec![3]);
        assert!(fresh.find("made_at_spawn", 0, None).is_empty());
        assert_eq!(io_save(&fresh.io_state()).queue, state.io.queue);
    }
}
