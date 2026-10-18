//! The developer console, Source style: off until "Developer console" is
//! turned on in Options (`con_enable 1`), then the ` (or ~) key opens it over
//! the top of the screen. Up/Down walk the history, Tab completes commands,
//! map names, entity names and inputs, PageUp/PageDown scroll, Esc or `
//! closes it. The world pauses while it is open, like the original.
//!
//! Commands go through the game's own messages and resources: `map` and
//! `restart` send [`LoadLevel`], `fire` queues an input with [`Ents::send`],
//! `give`/`upgrade_portalgun` go to the map console handler
//! ([`crate::vscripts::ConsoleCommand`]), `cl_showfps`/`cl_showpos`/`fov`
//! change [`Settings`]. Cheats (`noclip`, `god`, `setpos`, `host_timescale`,
//! `fire`, `give`...) need `sv_cheats 1`, as in Portal 2. Other modules print
//! here with [`ConsolePrint`]; scripted tests run lines with [`ConsoleSubmit`].
use std::collections::VecDeque;

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use p2_core::*;

use crate::campaign::Campaign;
use crate::community::CommunityMaps;
use crate::io::Ents;
use crate::level::{CurrentMap, GameSet, LevelLoaded, LoadLevel};
use crate::settings::{Settings, FOV_RANGE};
use crate::ui::Screen;

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Console>()
            .init_resource::<Cheats>()
            .add_message::<ConsoleSubmit>()
            .add_message::<ConsolePrint>()
            .add_systems(Startup, autoexec)
            .add_systems(PreUpdate, console_keys.after(bevy::input::InputSystems))
            .add_systems(Update, (run_commands, cheats_on_load, god_mode).chain().in_set(GameSet::Logic))
            .add_systems(PostUpdate, (hold_focus, draw_console).chain())
            .add_systems(FixedUpdate, (noclip_start.before(P2Systems::Move), noclip_move.after(P2Systems::Move).before(P2Systems::Transit)));
    }
}

/// Run a console line as if typed (scripted tests: `con <line>`).
#[derive(Message, Clone, Debug)]
pub struct ConsoleSubmit(pub String);

/// A line for the console's log (from any module).
#[derive(Message, Clone, Debug)]
pub struct ConsolePrint(pub String);

/// Cheat state. `sv_cheats` gates the cheat commands; a run with it on is
/// never a speedrun personal best.
#[derive(Resource, Clone, Debug, Default)]
pub struct Cheats {
    pub sv_cheats: bool,
    pub noclip: bool,
    pub god: bool,
    /// Where the noclip move starts this tick (after last tick's portal transit).
    noclip_from: Option<(Vec3, Vec3)>,
}

// ------------------------------------------------------------------ parsing --

/// Split a console line into commands (`;` between them, outside quotes) and
/// each command into words (`"quoted words"` stay together, `//` ends the line).
pub fn split_commands(line: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                in_word = true;
            }
            '/' if !quoted && chars.peek() == Some(&'/') => break,
            ';' if !quoted => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
                if !words.is_empty() {
                    out.push(std::mem::take(&mut words));
                }
            }
            c if c.is_whitespace() && !quoted => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            c => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(word);
    }
    if !words.is_empty() {
        out.push(words);
    }
    out
}

/// What a command's first argument completes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    None,
    Map,
    Entity,
    Bool,
    Command,
    Item,
    Mod,
    Profile,
    Config,
}

pub struct CommandDef {
    pub name: &'static str,
    pub usage: &'static str,
    pub help: &'static str,
    pub cheat: bool,
    pub arg: ArgKind,
}

const fn def(name: &'static str, usage: &'static str, help: &'static str, cheat: bool, arg: ArgKind) -> CommandDef {
    CommandDef { name, usage, help, cheat, arg }
}

/// Every command, sorted by name.
pub const COMMANDS: &[CommandDef] = &[
    def("cl_showfps", "[0/1]", "Frame rate and frame times (min/avg/max).", false, ArgKind::Bool),
    def("cl_showpos", "[0/1]", "Speed, position and ground state.", false, ArgKind::Bool),
    def("clear", "", "Clear the console.", false, ArgKind::None),
    def("cmdlist", "", "List every command.", false, ArgKind::None),
    def("echo", "<text>", "Print text.", false, ArgKind::None),
    def("ent_fire", "<target> <input> [param]", "Send an input to map entities (same as fire).", true, ArgKind::Entity),
    def("ent_list", "[filter]", "List map entities: index, class, name, position.", false, ArgKind::Entity),
    def("exec", "<file>", "Run the commands in <data dir>/cfg/<file>.cfg (autoexec.cfg runs at startup).", false, ArgKind::Config),
    def("exit", "", "Quit the game.", false, ArgKind::None),
    def("find", "<text>", "Find commands by name or help text.", false, ArgKind::None),
    def("fire", "<target> <input> [param]", "Send an input to map entities, like an output would.", true, ArgKind::Entity),
    def("fov", "[degrees]", "Field of view (the Options setting, 75 to 110).", false, ArgKind::None),
    def("fov_desired", "[degrees]", "Field of view (the Options setting, 75 to 110).", false, ArgKind::None),
    def("getpos", "", "Print your position and view as setpos/setang.", false, ArgKind::None),
    def("give", "<item>", "Give an item (weapon_portalgun).", true, ArgKind::Item),
    def("god", "", "Toggle invulnerability.", true, ArgKind::None),
    def("help", "[command]", "Help for a command.", false, ArgKind::Command),
    def("host_timescale", "[scale]", "Game speed (1 is normal).", true, ArgKind::None),
    def("map", "<name>", "Start a map (campaign, community or a .bsp path).", false, ArgKind::Map),
    def("maps", "[filter]", "List the maps you can start.", false, ArgKind::None),
    def("mod_disable", "<id>", "Turn a mod off in the active mod profile (from the next start).", false, ArgKind::Mod),
    def("mod_enable", "<id>", "Turn an installed mod on in the active mod profile (from the next start).", false, ArgKind::Mod),
    def("mod_list", "", "List installed mods; [x] marks the ones the active profile enables.", false, ArgKind::None),
    def("mod_profile", "[name]", "List mod profiles, or make another one active (from the next start).", false, ArgKind::Profile),
    def("mod_rollback", "", "Undo the last change to the active mod profile.", false, ArgKind::None),
    def("mods", "", "Show the enabled mods and any problems with them.", false, ArgKind::None),
    def("noclip", "", "Toggle flying through walls.", true, ArgKind::None),
    def("practice_delete", "<name>", "Delete a practice checkpoint of this map.", false, ArgKind::None),
    def("practice_list", "", "List this map's practice checkpoints.", false, ArgKind::None),
    def("practice_load", "[name]", "Go back to a practice checkpoint (position and velocity).", false, ArgKind::None),
    def("practice_save", "[name]", "Save a practice checkpoint here (default name: the current slot).", false, ArgKind::None),
    def("quit", "", "Quit the game.", false, ArgKind::None),
    def("restart", "", "Start the current map again.", false, ArgKind::None),
    def("setang", "<pitch> <yaw> [roll]", "Set your view angles (degrees, Source convention).", true, ArgKind::None),
    def("setpos", "<x> <y> <z>", "Move your feet to a Source position (units).", true, ArgKind::None),
    def("speedrun_reset", "", "Reset the speedrun timer (the next map start begins a run).", false, ArgKind::None),
    def("speedrun_splits", "", "Show the personal best, its splits, the golds and the sum of best.", false, ArgKind::None),
    def("sv_cheats", "[0/1]", "Allow cheat commands.", false, ArgKind::Bool),
    def("upgrade_portalgun", "", "Give the dual portal gun.", true, ArgKind::None),
];

pub fn command(name: &str) -> Option<&'static CommandDef> {
    COMMANDS.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// A parsed command, ready to run.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Clear,
    ShowFps(Option<bool>),
    ShowPos(Option<bool>),
    ListCommands,
    Echo(String),
    Exec(String),
    Fire { target: String, input: String, param: String },
    EntList(Option<String>),
    Quit,
    Find(String),
    Fov(Option<f32>),
    GetPos,
    Give(String),
    God,
    Help(Option<String>),
    Timescale(Option<f32>),
    Map(String),
    Maps(Option<String>),
    Mods,
    ModList,
    ModEnable(String),
    ModDisable(String),
    ModProfile(Option<String>),
    ModRollback,
    Noclip,
    PracticeDelete(String),
    PracticeList,
    PracticeLoad(Option<String>),
    PracticeSave(Option<String>),
    Restart,
    SetAng(f32, f32, f32),
    SetPos(Vec3),
    SpeedrunReset,
    SpeedrunSplits,
    SvCheats(Option<bool>),
    UpgradePortalgun,
}

/// `0`/`1` (any number: non-zero is on), `on`/`off`, `true`/`false`.
pub fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" => Some(true),
        "off" | "false" | "no" => Some(false),
        n => n.parse::<f32>().ok().map(|f| f != 0.0),
    }
}

fn num(s: &str) -> Result<f32, String> {
    s.parse::<f32>().ok().filter(|f| f.is_finite()).ok_or_else(|| format!("{s:?} is not a number"))
}

/// Turn one command's words into a [`Cmd`]; the error is what to print.
pub fn parse(words: &[String]) -> Result<Cmd, String> {
    let Some(first) = words.first() else { return Err(String::new()) };
    let name = first.to_ascii_lowercase();
    let Some(d) = command(&name) else { return Err(format!("Unknown command \"{first}\"")) };
    let args = &words[1..];
    let usage = || format!("Usage: {} {}", d.name, d.usage);
    let opt_bool = || -> Result<Option<bool>, String> { args.first().map(|a| parse_bool(a).ok_or_else(usage)).transpose() };
    let opt_num = || -> Result<Option<f32>, String> { args.first().map(|a| num(a)).transpose() };
    let rest = || (!args.is_empty()).then(|| args.join(" "));
    Ok(match d.name {
        "clear" => Cmd::Clear,
        "cl_showfps" => Cmd::ShowFps(opt_bool()?),
        "cl_showpos" => Cmd::ShowPos(opt_bool()?),
        "cmdlist" => Cmd::ListCommands,
        "echo" => Cmd::Echo(args.join(" ")),
        "exec" => Cmd::Exec(args.first().ok_or_else(usage)?.clone()),
        "fire" | "ent_fire" => match args {
            [t, i, p @ ..] => Cmd::Fire { target: t.clone(), input: i.clone(), param: p.join(" ") },
            _ => return Err(usage()),
        },
        "ent_list" => Cmd::EntList(rest()),
        "exit" | "quit" => Cmd::Quit,
        "find" => Cmd::Find(rest().ok_or_else(usage)?),
        "fov" | "fov_desired" => Cmd::Fov(opt_num()?),
        "getpos" => Cmd::GetPos,
        "give" => Cmd::Give(args.first().ok_or_else(usage)?.to_ascii_lowercase()),
        "god" => Cmd::God,
        "help" => Cmd::Help(args.first().cloned()),
        "host_timescale" => Cmd::Timescale(opt_num()?),
        "map" => Cmd::Map(rest().ok_or_else(usage)?),
        "maps" => Cmd::Maps(args.first().map(|a| a.trim_matches('*').to_ascii_lowercase()).filter(|a| !a.is_empty())),
        "mods" => Cmd::Mods,
        "mod_list" => Cmd::ModList,
        "mod_enable" => Cmd::ModEnable(args.first().ok_or_else(usage)?.to_ascii_lowercase()),
        "mod_disable" => Cmd::ModDisable(args.first().ok_or_else(usage)?.to_ascii_lowercase()),
        "mod_profile" => Cmd::ModProfile(args.first().map(|a| a.to_ascii_lowercase())),
        "mod_rollback" => Cmd::ModRollback,
        "noclip" => Cmd::Noclip,
        "practice_delete" => Cmd::PracticeDelete(args.first().ok_or_else(usage)?.clone()),
        "practice_list" => Cmd::PracticeList,
        "practice_load" => Cmd::PracticeLoad(args.first().cloned()),
        "practice_save" => Cmd::PracticeSave(args.first().cloned()),
        "restart" => Cmd::Restart,
        "setang" => match args {
            [p, y] => Cmd::SetAng(num(p)?, num(y)?, 0.0),
            [p, y, r, ..] => Cmd::SetAng(num(p)?, num(y)?, num(r)?),
            _ => return Err(usage()),
        },
        "setpos" => match args {
            [x, y, z, ..] => Cmd::SetPos(Vec3::new(num(x)?, num(y)?, num(z)?)),
            _ => return Err(usage()),
        },
        "speedrun_reset" => Cmd::SpeedrunReset,
        "speedrun_splits" => Cmd::SpeedrunSplits,
        "sv_cheats" => Cmd::SvCheats(opt_bool()?),
        "upgrade_portalgun" => Cmd::UpgradePortalgun,
        _ => return Err(format!("Unknown command \"{first}\"")),
    })
}

// ----------------------------------------------------------- coordinates --

/// World position (metres, Y up) to Source units (Z up).
pub fn to_source(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y) / SOURCE_UNIT
}

/// Source pitch/yaw (degrees: pitch down is positive, yaw 0 along +X) to
/// the player's view angles (radians).
pub fn view_from_source(pitch: f32, yaw: f32, roll: f32) -> ViewAngles {
    ViewAngles { yaw: (yaw - 90.0).to_radians(), pitch: -pitch.to_radians(), roll: roll.to_radians() }
}

/// The view as Source pitch, yaw, roll (degrees, yaw in -180..180).
pub fn source_angles(v: &ViewAngles) -> Vec3 {
    let mut yaw = v.yaw.to_degrees() + 90.0;
    yaw = (yaw + 180.0).rem_euclid(360.0) - 180.0;
    Vec3::new(-v.pitch.to_degrees(), yaw, v.roll.to_degrees())
}

/// `setpos x y z;setang p y r`, ready to paste back.
pub fn getpos_text(feet: Vec3, view: &ViewAngles) -> String {
    let s = to_source(feet);
    let a = source_angles(view);
    format!("setpos {:.2} {:.2} {:.2};setang {:.2} {:.2} {:.2}", s.x, s.y, s.z, a.x, a.y, a.z)
}

// ------------------------------------------------------------- history --

/// Lines entered, oldest first; Up walks back from the newest.
#[derive(Clone, Debug, Default)]
pub struct History {
    lines: Vec<String>,
    at: Option<usize>,
}

impl History {
    pub const MAX: usize = 100;

    pub fn push(&mut self, line: &str) {
        self.at = None;
        if line.trim().is_empty() || self.lines.last().is_some_and(|l| l == line) {
            return;
        }
        self.lines.retain(|l| l != line);
        self.lines.push(line.to_string());
        if self.lines.len() > Self::MAX {
            self.lines.remove(0);
        }
    }

    /// The line before the one shown (Up).
    pub fn prev(&mut self) -> Option<&str> {
        let i = match self.at {
            None => self.lines.len().checked_sub(1)?,
            Some(i) => i.saturating_sub(1),
        };
        self.at = Some(i);
        self.lines.get(i).map(String::as_str)
    }

    /// The line after it (Down); `None` past the newest (an empty line).
    pub fn next(&mut self) -> Option<&str> {
        let i = self.at? + 1;
        if i >= self.lines.len() {
            self.at = None;
            return None;
        }
        self.at = Some(i);
        self.lines.get(i).map(String::as_str)
    }

    #[cfg(test)]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

// ---------------------------------------------------------- completion --

/// What names complete from.
#[derive(Default)]
pub struct Completer {
    pub maps: Vec<String>,
    /// Entity names, with the inputs worth offering for each.
    pub ents: Vec<(String, Vec<String>)>,
    /// Installed mod ids, mod profiles and `cfg/*.cfg` names (only filled
    /// when the line's command takes one).
    pub mods: Vec<String>,
    pub profiles: Vec<String>,
    pub configs: Vec<String>,
}

/// Inputs most entities take, offered after any entity name.
const COMMON_INPUTS: &[&str] = &[
    "Trigger", "Enable", "Disable", "Toggle", "Kill", "Use", "Open", "Close", "Lock", "Unlock", "Press", "Start", "Stop", "FireUser1", "FireUser2", "FireUser3", "FireUser4",
    "SetAnimation", "Skin", "ForceSpawn", "RunScriptCode", "CallScriptFunction", "Activate", "Deactivate", "Fizzle", "PlaySound", "StopSound", "Teleport", "Fade",
];

/// Candidates for the word being typed at the end of `line`: returns the
/// text before that word and the full candidates, sorted.
pub fn complete(line: &str, c: &Completer) -> (String, Vec<String>) {
    // Only the last command of a `;` line completes.
    let start = line.rfind(';').map_or(0, |i| i + 1);
    let cmd = &line[start..];
    let words: Vec<&str> = cmd.split_whitespace().collect();
    let typing_new = cmd.ends_with(char::is_whitespace) || words.is_empty();
    let index = if typing_new { words.len() } else { words.len() - 1 };
    let partial = if typing_new { "" } else { words[index] };
    let before = format!("{}{}", &line[..start], &cmd[..cmd.len() - partial.len()]);
    let lower = partial.to_ascii_lowercase();
    let matches = |name: &str| name.to_ascii_lowercase().starts_with(&lower);
    let mut out: Vec<String> = if index == 0 {
        COMMANDS.iter().map(|d| d.name.to_string()).filter(|n| matches(n)).collect()
    } else {
        let kind = command(words[0]).map_or(ArgKind::None, |d| d.arg);
        match (kind, index) {
            (ArgKind::Map, 1) => c.maps.iter().filter(|m| matches(m)).cloned().collect(),
            (ArgKind::Bool, 1) => ["0", "1"].iter().filter(|m| matches(m)).map(|s| s.to_string()).collect(),
            (ArgKind::Command, 1) => COMMANDS.iter().map(|d| d.name.to_string()).filter(|n| matches(n)).collect(),
            (ArgKind::Item, 1) => ["weapon_portalgun".to_string()].into_iter().filter(|n| matches(n)).collect(),
            (ArgKind::Mod, 1) => c.mods.iter().filter(|m| matches(m)).cloned().collect(),
            (ArgKind::Profile, 1) => c.profiles.iter().filter(|m| matches(m)).cloned().collect(),
            (ArgKind::Config, 1) => c.configs.iter().filter(|m| matches(m)).cloned().collect(),
            (ArgKind::Entity, 1) => {
                let mut v: Vec<String> = c.ents.iter().map(|(n, _)| n.clone()).filter(|n| matches(n)).collect();
                if matches("!player") && words[0] != "ent_list" {
                    v.push("!player".into());
                }
                v
            }
            (ArgKind::Entity, 2) if words[0] != "ent_list" => {
                let target = words[1].to_ascii_lowercase();
                let mut v: Vec<String> = c.ents.iter().filter(|(n, _)| *n == target).flat_map(|(_, i)| i.iter().cloned()).collect();
                v.extend(COMMON_INPUTS.iter().map(|s| s.to_string()));
                v.retain(|n| matches(n));
                v
            }
            _ => Vec::new(),
        }
    };
    out.sort_by_key(|s| s.to_ascii_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    (before, out)
}

/// The longest start every candidate shares (ignoring case; the first's spelling).
pub fn common_prefix(c: &[String]) -> String {
    let Some(first) = c.first() else { return String::new() };
    let mut n = first.len();
    for s in &c[1..] {
        n = n.min(first.chars().zip(s.chars()).take_while(|(a, b)| a.eq_ignore_ascii_case(b)).map(|(a, _)| a.len_utf8()).sum());
    }
    first[..n].to_string()
}

// ------------------------------------------------------------- the state --

const LOG_MAX: usize = 1000;
/// Suggestions shown under the input.
const SUGGESTIONS_SHOWN: usize = 8;

#[derive(Resource)]
pub struct Console {
    pub open: bool,
    input: String,
    /// Byte index into `input` (always on a char boundary).
    cursor: usize,
    log: VecDeque<String>,
    /// Lines scrolled up from the newest.
    scroll: usize,
    history: History,
    suggestions: Vec<String>,
    /// Tab cycling: (text before the word, candidates, index shown).
    cycling: Option<(String, Vec<String>, usize)>,
    pending: Vec<String>,
    maps: Option<Vec<String>>,
    /// We paused the world clock when opening.
    paused_time: bool,
    dirty: bool,
    greeted: bool,
}

impl Default for Console {
    fn default() -> Self {
        Console {
            open: false,
            input: String::new(),
            cursor: 0,
            log: VecDeque::new(),
            scroll: 0,
            history: History::default(),
            suggestions: Vec::new(),
            cycling: None,
            pending: Vec::new(),
            maps: None,
            paused_time: false,
            dirty: true,
            greeted: false,
        }
    }
}

impl Console {
    pub fn print(&mut self, line: impl Into<String>) {
        for l in line.into().lines() {
            self.log.push_back(l.to_string());
        }
        while self.log.len() > LOG_MAX {
            self.log.pop_front();
        }
        self.scroll = 0;
        self.dirty = true;
    }

    fn set_input(&mut self, s: &str) {
        self.input = s.to_string();
        self.cursor = self.input.len();
        self.cycling = None;
        self.dirty = true;
    }

    /// The input line edited by one key (pure; tested).
    fn edit(&mut self, key: KeyCode, text: Option<&str>) {
        let prev_boundary = |s: &str, i: usize| s[..i].char_indices().last().map_or(0, |(j, _)| j);
        let next_boundary = |s: &str, i: usize| s[i..].chars().next().map_or(i, |c| i + c.len_utf8());
        match key {
            KeyCode::Backspace if self.cursor > 0 => {
                let j = prev_boundary(&self.input, self.cursor);
                self.input.replace_range(j..self.cursor, "");
                self.cursor = j;
            }
            KeyCode::Delete if self.cursor < self.input.len() => {
                let j = next_boundary(&self.input, self.cursor);
                self.input.replace_range(self.cursor..j, "");
            }
            KeyCode::ArrowLeft => self.cursor = prev_boundary(&self.input, self.cursor),
            KeyCode::ArrowRight => self.cursor = next_boundary(&self.input, self.cursor),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.len(),
            _ => {
                let Some(t) = text else { return };
                let t: String = t.chars().filter(|c| !c.is_control() && *c != '`').collect();
                if t.is_empty() {
                    return;
                }
                self.input.insert_str(self.cursor, &t);
                self.cursor += t.len();
            }
        }
        self.cycling = None;
        self.dirty = true;
    }

    /// Tab: finish the word, or step through the candidates.
    fn tab(&mut self, c: &Completer) {
        if let Some((before, cands, i)) = &mut self.cycling {
            *i = (*i + 1) % cands.len();
            let line = format!("{before}{}", cands[*i]);
            self.input = line;
            self.cursor = self.input.len();
            self.dirty = true;
            return;
        }
        let (before, cands) = complete(&self.input, c);
        match cands.len() {
            0 => {}
            1 => {
                let line = format!("{before}{} ", cands[0]);
                self.set_input(&line);
            }
            _ => {
                let typed = self.input.len() - before.len();
                let common = common_prefix(&cands);
                if common.len() > typed {
                    let line = format!("{before}{common}");
                    self.set_input(&line);
                } else {
                    let line = format!("{before}{}", cands[0]);
                    self.set_input(&line);
                    self.cycling = Some((before, cands, 0));
                }
            }
        }
    }

    fn submit(&mut self) {
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.cycling = None;
        self.history.push(&line);
        self.print(format!("] {line}"));
        if !line.trim().is_empty() {
            self.pending.push(line);
        }
    }
}

// ------------------------------------------------------------ the keys --

fn completer(console: &mut Console, game: &p2_assets::GameFsRes, community: &CommunityMaps, campaign: &Campaign, ents: Option<&Ents>) -> Completer {
    let maps = console
        .maps
        .get_or_insert_with(|| {
            let mut m: Vec<String> = game.0.list("maps/").iter().filter_map(|p| p.strip_prefix("maps/")?.strip_suffix(".bsp")).filter(|p| !p.contains('/')).map(str::to_string).collect();
            m.extend(campaign.maps.iter().cloned());
            m.sort();
            m.dedup();
            m
        })
        .clone();
    let mut maps = maps;
    maps.extend(community.maps.iter().map(|m| m.name.clone()));
    maps.sort();
    maps.dedup();
    let mut list: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(ents) = ents {
        let mut inputs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for e in &ents.list {
            for o in &e.outputs {
                inputs.entry(o.target.to_ascii_lowercase()).or_default().push(o.input.clone());
            }
        }
        for e in ents.list.iter().filter(|e| !e.name.is_empty() && !e.killed && !e.name.starts_with('!')) {
            if !list.iter().any(|(n, _)| *n == e.name) {
                list.push((e.name.clone(), inputs.get(&e.name).cloned().unwrap_or_default()));
            }
        }
    }
    // Mods, profiles and cfg files come off the disk, so only when asked for.
    let kind = console.input.split_whitespace().next().and_then(command).map(|d| d.arg);
    let (mut mods, mut profiles, mut configs) = (Vec::new(), Vec::new(), Vec::new());
    match kind {
        Some(ArgKind::Mod | ArgKind::Profile) => {
            if let Ok(m) = p2_mods::ModManager::open(crate::mounts::mods_root()) {
                mods = m.installed().into_iter().map(|i| i.manifest.id).collect();
                mods.dedup();
                profiles = m.profiles();
            }
        }
        Some(ArgKind::Config) => {
            configs = std::fs::read_dir(cfg_dir()).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".cfg").map(str::to_string)).collect();
        }
        _ => {}
    }
    Completer { maps, ents: list, mods, profiles, configs }
}

/// Where `exec` finds its files.
fn cfg_dir() -> std::path::PathBuf {
    crate::settings::data_dir().join("cfg")
}

/// The lines of `cfg/<name>.cfg`, or what went wrong.
pub fn read_cfg(name: &str) -> Result<Vec<String>, String> {
    let file = if name.to_ascii_lowercase().ends_with(".cfg") { name.to_string() } else { format!("{name}.cfg") };
    if file.contains("..") || file.starts_with(['/', '\\']) || file.contains(':') {
        return Err(format!("exec: {name}: stay inside the cfg folder"));
    }
    let path = cfg_dir().join(&file);
    std::fs::read_to_string(&path).map(|t| t.lines().map(str::to_string).filter(|l| !l.trim().is_empty()).collect()).map_err(|_| format!("exec: couldn't exec {file} (looked in {})", cfg_dir().display()))
}

/// Run `cfg/autoexec.cfg` once at startup when the console is enabled.
fn autoexec(settings: Res<Settings>, mut console: ResMut<Console>) {
    if settings.tools.dev_console && cfg_dir().join("autoexec.cfg").is_file() {
        console.pending.push("exec autoexec".into());
    }
}

/// The console key opens and closes it; while open every key goes to it
/// (nothing else in the game sees them).
#[allow(clippy::too_many_arguments)]
fn console_keys(
    mut console: ResMut<Console>,
    settings: Res<Settings>,
    mut events: MessageReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    game: Res<p2_assets::GameFsRes>,
    community: Res<CommunityMaps>,
    campaign: Res<Campaign>,
    ents: Option<Res<Ents>>,
) {
    let was_open = console.open;
    if console.open && !settings.tools.dev_console {
        console.open = false;
    }
    let toggle = |k: KeyCode| k == KeyCode::Backquote || (cfg!(target_os = "macos") && k == KeyCode::IntlBackslash);
    for e in events.read() {
        if e.state != ButtonState::Pressed {
            continue;
        }
        if toggle(e.key_code) && !e.repeat {
            if console.open {
                console.open = false;
            } else if settings.tools.dev_console {
                console.open = true;
                console.dirty = true;
            }
            continue;
        }
        if !console.open {
            continue;
        }
        match e.key_code {
            KeyCode::Escape => console.open = false,
            KeyCode::Enter | KeyCode::NumpadEnter => console.submit(),
            KeyCode::Tab => {
                let c = completer(&mut console, &game, &community, &campaign, ents.as_deref());
                console.tab(&c);
            }
            KeyCode::ArrowUp => {
                if let Some(l) = console.history.prev().map(str::to_string) {
                    console.set_input(&l);
                }
            }
            KeyCode::ArrowDown => {
                let l = console.history.next().unwrap_or("").to_string();
                console.set_input(&l);
            }
            KeyCode::PageUp => {
                console.scroll = (console.scroll + 10).min(console.log.len().saturating_sub(1));
                console.dirty = true;
            }
            KeyCode::PageDown => {
                console.scroll = console.scroll.saturating_sub(10);
                console.dirty = true;
            }
            k => console.edit(k, e.text.as_deref()),
        }
    }
    if console.open && console.dirty {
        let c = completer(&mut console, &game, &community, &campaign, ents.as_deref());
        let (_, cands) = if console.input.trim().is_empty() { (String::new(), Vec::new()) } else { complete(&console.input, &c) };
        // A finished word isn't worth suggesting again.
        let exact = cands.len() == 1 && console.input.trim_end().to_ascii_lowercase().ends_with(&cands[0].to_ascii_lowercase());
        console.suggestions = if exact { Vec::new() } else { cands };
    }
    if console.open && !console.greeted {
        console.greeted = true;
        console.print("Developer console. Type help or cmdlist; Tab completes, Esc closes.");
    }
    if console.open || was_open {
        // Esc that closed it, and every key while open, are the console's.
        keys.reset_all();
    }
    if console.open != was_open {
        console.dirty = true;
    }
}

/// While open: the world is paused, the mouse is free and the player can't
/// move. On close, back to how the screen wants it.
#[allow(clippy::too_many_arguments)]
fn hold_focus(
    mut console: ResMut<Console>,
    mut was_open: Local<bool>,
    screen: Res<Screen>,
    map: Option<Res<CurrentMap>>,
    mut time: ResMut<Time<Virtual>>,
    mut locked: ResMut<InputLocked>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut inputs: Query<&mut MoveInput, With<Player>>,
    args: Res<p2_room::HarnessArgs>,
) {
    let scripted = args.automated() || std::env::var_os("P2_SCRIPT").is_some();
    if console.open {
        if !time.is_paused() {
            time.pause();
            console.paused_time = true;
        }
        if !scripted {
            locked.0 = true;
            if let Ok(mut c) = cursor.single_mut() {
                if c.grab_mode != CursorGrabMode::None || !c.visible {
                    c.grab_mode = CursorGrabMode::None;
                    c.visible = true;
                }
            }
        }
        if !*was_open {
            for mut i in &mut inputs {
                *i = MoveInput::default();
            }
        }
    } else if *was_open {
        if console.paused_time && *screen != Screen::Pause {
            time.unpause();
        }
        console.paused_time = false;
        if !scripted {
            let playing = *screen == Screen::None && map.as_ref().is_some_and(|m| !m.menu);
            locked.0 = !playing;
            if let Ok(mut c) = cursor.single_mut() {
                c.grab_mode = if playing { CursorGrabMode::Locked } else { CursorGrabMode::None };
                c.visible = !playing;
            }
        }
    }
    *was_open = console.open;
}

// ------------------------------------------------------------ running --

#[derive(SystemParam)]
struct World1<'w, 's> {
    settings: ResMut<'w, Settings>,
    cheats: ResMut<'w, Cheats>,
    ents: Option<ResMut<'w, Ents>>,
    map: Option<Res<'w, CurrentMap>>,
    game: Res<'w, p2_assets::GameFsRes>,
    community: Res<'w, CommunityMaps>,
    mounts: Res<'w, crate::mounts::Mounts>,
    screen: ResMut<'w, Screen>,
    time: ResMut<'w, Time<Virtual>>,
    players: Query<'w, 's, (&'static mut Transform, &'static mut InterpolatedFeet, &'static mut ViewAngles, &'static mut CharacterVelocity), With<Player>>,
}

#[derive(SystemParam)]
struct Outbox<'w> {
    load: MessageWriter<'w, LoadLevel>,
    map_console: MessageWriter<'w, crate::vscripts::ConsoleCommand>,
    practice: MessageWriter<'w, crate::speedrun::PracticeRequest>,
    exit: MessageWriter<'w, AppExit>,
}

fn on_off(b: bool) -> &'static str {
    if b {
        "ON"
    } else {
        "OFF"
    }
}

/// Can `name` be started? (A map in the game's folders, a community map, or a .bsp file.)
fn find_map(name: &str, w: &World1) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let lower = lower.trim_end_matches(".bsp");
    if name.to_ascii_lowercase().ends_with(".bsp") && std::path::Path::new(name).is_file() {
        return Some(name.to_string());
    }
    if w.game.0.exists(&format!("maps/{lower}.bsp")) {
        return Some(lower.to_string());
    }
    w.community.maps.iter().find(|m| m.name == lower).map(|m| m.launch_name())
}

fn run_commands(mut console: ResMut<Console>, mut submitted: MessageReader<ConsoleSubmit>, mut prints: MessageReader<ConsolePrint>, mut w: World1, mut out: Outbox) {
    for p in prints.read() {
        console.print(p.0.clone());
    }
    let mut lines: VecDeque<String> = std::mem::take(&mut console.pending).into();
    for s in submitted.read() {
        console.print(format!("] {}", s.0));
        lines.push_back(s.0.clone());
    }
    // `exec` adds lines; a file that execs itself stops here.
    let mut budget = 1000;
    while let Some(line) = lines.pop_front() {
        budget -= 1;
        if budget == 0 {
            console.print("exec: too many commands at once (does a cfg exec itself?)");
            break;
        }
        for words in split_commands(&line) {
            match parse(&words) {
                Ok(cmd) => {
                    let d = command(&words[0]).expect("parsed commands exist");
                    // Cheat commands (cheat cvars may still be read).
                    let reading = matches!(cmd, Cmd::Timescale(None));
                    if d.cheat && !w.cheats.sv_cheats && !reading {
                        console.print(format!("Can't use cheat command {} in multiplayer, unless the server has sv_cheats set to 1.", d.name));
                        continue;
                    }
                    if let Cmd::Exec(name) = &cmd {
                        match read_cfg(name) {
                            Ok(file) => {
                                // The file's lines run next, before the rest of this line.
                                for l in file.into_iter().rev() {
                                    lines.push_front(l);
                                }
                            }
                            Err(e) => console.print(e),
                        }
                        continue;
                    }
                    execute(cmd, &mut console, &mut w, &mut out);
                }
                Err(e) if e.is_empty() => {}
                Err(e) => console.print(e),
            }
        }
    }
}

fn execute(cmd: Cmd, console: &mut Console, w: &mut World1, out: &mut Outbox) {
    let in_game = w.map.as_ref().is_some_and(|m| !m.menu);
    match cmd {
        Cmd::Clear => {
            console.log.clear();
            console.dirty = true;
        }
        Cmd::ShowFps(v) | Cmd::ShowPos(v) => {
            let fps = matches!(cmd, Cmd::ShowFps(_));
            let (name, cur) = if fps { ("cl_showfps", w.settings.tools.show_fps) } else { ("cl_showpos", w.settings.tools.show_pos) };
            match v {
                None => console.print(format!("\"{name}\" = \"{}\"", cur as u8)),
                Some(on) => {
                    if fps {
                        w.settings.tools.show_fps = on;
                    } else {
                        w.settings.tools.show_pos = on;
                    }
                    w.settings.save();
                }
            }
        }
        Cmd::ListCommands => {
            for d in COMMANDS {
                console.print(format!("{:<18} {}{}", d.name, d.help, if d.cheat { " (cheat)" } else { "" }));
            }
            console.print(format!("{} commands", COMMANDS.len()));
        }
        Cmd::Echo(t) => console.print(t),
        Cmd::Exec(_) => {}
        Cmd::ModList | Cmd::ModEnable(_) | Cmd::ModDisable(_) | Cmd::ModProfile(_) | Cmd::ModRollback => mod_command(cmd, console),
        Cmd::SpeedrunSplits => {
            out.practice.write(crate::speedrun::PracticeRequest::ShowSplits);
        }
        Cmd::Fire { target, input, param } => match w.ents.as_mut() {
            Some(ents) => {
                let n = ents.find(&target, crate::io::PLAYER, Some(crate::io::PLAYER)).len();
                if n == 0 {
                    console.print(format!("No entity named {target}"));
                } else {
                    ents.send(&target, &input, &param, 0.0);
                    console.print(format!("{input} -> {target} ({n} entit{})", if n == 1 { "y" } else { "ies" }));
                }
            }
            None => console.print("No map loaded"),
        },
        Cmd::EntList(filter) => {
            let Some(ents) = w.ents.as_ref() else {
                console.print("No map loaded");
                return;
            };
            let f = filter.map(|f| f.to_ascii_lowercase());
            let mut n = 0;
            for (i, e) in ents.list.iter().enumerate() {
                if e.killed || f.as_ref().is_some_and(|f| !e.class.contains(f.as_str()) && !e.name.contains(f.as_str())) {
                    continue;
                }
                n += 1;
                if n <= 300 {
                    let p = to_source(e.transform.translation);
                    let name = if e.name.is_empty() { "-" } else { e.name.as_str() };
                    console.print(format!("#{i:<5} {:<28} {:<32} ({:.0} {:.0} {:.0}){}", e.class, name, p.x, p.y, p.z, if e.enabled { "" } else { " disabled" }));
                }
            }
            console.print(if n > 300 { format!("{n} entities (first 300 shown; add a filter)") } else { format!("{n} entities") });
        }
        Cmd::Quit => {
            w.settings.save();
            out.exit.write(AppExit::Success);
        }
        Cmd::Find(t) => {
            let t = t.to_ascii_lowercase();
            for d in COMMANDS.iter().filter(|d| d.name.contains(&t) || d.help.to_ascii_lowercase().contains(&t)) {
                console.print(format!("{:<18} {}", d.name, d.help));
            }
        }
        Cmd::Fov(v) => match v {
            None => console.print(format!("\"fov_desired\" = \"{}\" (min. {} max. {})", w.settings.fov, FOV_RANGE.0, FOV_RANGE.1)),
            Some(f) => {
                w.settings.fov = f.clamp(FOV_RANGE.0, FOV_RANGE.1);
                w.settings.save();
            }
        },
        Cmd::GetPos => match w.players.single() {
            Ok((tf, _, view, _)) => console.print(getpos_text(tf.translation, view)),
            Err(_) => console.print("No player"),
        },
        Cmd::Give(item) => {
            if item == "weapon_portalgun" {
                out.map_console.write(crate::vscripts::ConsoleCommand("give weapon_portalgun".into()));
            } else {
                console.print(format!("give: can't give {item} (only weapon_portalgun)"));
            }
        }
        Cmd::UpgradePortalgun => {
            out.map_console.write(crate::vscripts::ConsoleCommand("upgrade_portalgun".into()));
        }
        Cmd::God => {
            w.cheats.god = !w.cheats.god;
            console.print(format!("godmode {}", on_off(w.cheats.god)));
        }
        Cmd::Noclip => {
            if !in_game {
                console.print("noclip: not in a map");
                return;
            }
            w.cheats.noclip = !w.cheats.noclip;
            console.print(format!("noclip {}", on_off(w.cheats.noclip)));
        }
        Cmd::Help(None) => console.print("Type cmdlist for every command, help <command> for one, find <text> to search."),
        Cmd::Help(Some(c)) => match command(&c) {
            Some(d) => console.print(format!("{} {}\n  {}{}", d.name, d.usage, d.help, if d.cheat { " Needs sv_cheats 1." } else { "" })),
            None => console.print(format!("Unknown command \"{c}\"")),
        },
        Cmd::Timescale(None) => console.print(format!("\"host_timescale\" = \"{}\" ( def. \"1.0\" )", w.time.relative_speed())),
        Cmd::Timescale(Some(x)) => {
            let x = x.clamp(0.05, 10.0);
            w.time.set_relative_speed(x);
        }
        Cmd::Map(name) => match find_map(&name, w) {
            Some(m) => {
                out.load.write(LoadLevel { map: m, menu: false });
                *w.screen = Screen::None;
                console.open = false;
            }
            None => console.print(format!("map load failed: {name} not found or invalid")),
        },
        Cmd::Maps(filter) => {
            let game = w.game.0.list("maps/");
            let mut names: Vec<String> = game.iter().filter_map(|p| p.strip_prefix("maps/")?.strip_suffix(".bsp")).filter(|p| !p.contains('/')).map(str::to_string).collect();
            // Mounted community maps are in the game's list too: show them once, marked.
            names.retain(|n| !w.community.maps.iter().any(|m| m.name == *n));
            names.extend(w.community.maps.iter().map(|m| format!("{} (community)", m.name)));
            names.sort();
            names.dedup();
            let shown: Vec<&String> = names.iter().filter(|n| filter.as_ref().is_none_or(|f| n.contains(f.as_str()))).collect();
            for n in &shown {
                console.print(format!("  {n}"));
            }
            console.print(format!("{} maps", shown.len()));
        }
        Cmd::Mods => {
            let plan = &w.mounts.mods;
            console.print(format!("mod profile: {}", plan.profile));
            if plan.loaded.is_empty() {
                console.print("  no mods loaded (manage them with the p2mods tool)");
            }
            for (id, v) in &plan.loaded {
                console.print(format!("  {id} {v}"));
            }
            for i in &plan.issues {
                console.print(format!("  {i}"));
            }
            if let Some(c) = &w.mounts.community {
                console.print(format!("community maps mounted from {}", c.display()));
            }
        }
        Cmd::PracticeDelete(name) => {
            out.practice.write(crate::speedrun::PracticeRequest::Delete(name));
        }
        Cmd::PracticeList => {
            out.practice.write(crate::speedrun::PracticeRequest::List);
        }
        Cmd::PracticeLoad(name) => {
            out.practice.write(crate::speedrun::PracticeRequest::Load(name));
        }
        Cmd::PracticeSave(name) => {
            out.practice.write(crate::speedrun::PracticeRequest::Save(name));
        }
        Cmd::SpeedrunReset => {
            out.practice.write(crate::speedrun::PracticeRequest::ResetTimer);
        }
        Cmd::Restart => match &w.map {
            Some(m) if !m.menu => {
                out.load.write(LoadLevel { map: m.info.name.clone(), menu: false });
                *w.screen = Screen::None;
                console.open = false;
            }
            _ => console.print("restart: not in a map"),
        },
        Cmd::SetAng(p, y, r) => match w.players.single_mut() {
            Ok((_, _, mut view, _)) => *view = view_from_source(p, y, r),
            Err(_) => console.print("No player"),
        },
        Cmd::SetPos(s) => match w.players.single_mut() {
            Ok((mut tf, mut feet, _, mut vel)) => {
                let p = source_to_world(s);
                tf.translation = p;
                *feet = InterpolatedFeet { prev: p, curr: p };
                vel.0 = Vec3::ZERO;
                w.cheats.noclip_from = None;
            }
            Err(_) => console.print("No player"),
        },
        Cmd::SvCheats(None) => console.print(format!("\"sv_cheats\" = \"{}\"", w.cheats.sv_cheats as u8)),
        Cmd::SvCheats(Some(on)) => {
            w.cheats.sv_cheats = on;
            if !on {
                // Cheat settings go back to normal.
                w.cheats.noclip = false;
                w.cheats.god = false;
                w.time.set_relative_speed(1.0);
            }
        }
    }
}

/// The mod manager from the console. Changes apply the next time the game
/// starts (mods are mounted at startup).
fn mod_command(cmd: Cmd, console: &mut Console) {
    if !matches!(cmd, Cmd::ModList | Cmd::ModProfile(None)) && !crate::settings::may_write_data() {
        console.print("mods: scripted test runs don't change the mod profile (set P2_DATA_DIR)");
        return;
    }
    let m = match p2_mods::ModManager::open(crate::mounts::mods_root()) {
        Ok(m) => m,
        Err(e) => {
            console.print(format!("mods: {e}"));
            return;
        }
    };
    let profile = m.active_profile();
    let next_start = "(takes effect the next time the game starts)";
    let changed = match cmd {
        Cmd::ModList => {
            let p = m.load_profile(&profile).unwrap_or_default();
            let installed = m.installed();
            if installed.is_empty() {
                console.print(format!("no mods installed: p2mods install <folder or .zip>, store in {}", m.root().display()));
            }
            for i in &installed {
                let on = p.get(&i.manifest.id).is_some_and(|e| e.enabled && e.version == i.manifest.version);
                console.print(format!("[{}] {} {}  {}", if on { "x" } else { " " }, i.manifest.id, i.manifest.version, i.manifest.name));
            }
            console.print(format!("profile {profile}"));
            false
        }
        Cmd::ModEnable(id) => match m.enable(&profile, &id, None) {
            Ok(v) => {
                console.print(format!("enabled {id} {v} in {profile} {next_start}"));
                true
            }
            Err(e) => {
                console.print(format!("mod_enable: {e}"));
                false
            }
        },
        Cmd::ModDisable(id) => match m.disable(&profile, &id) {
            Ok(()) => {
                console.print(format!("disabled {id} in {profile} {next_start}"));
                true
            }
            Err(e) => {
                console.print(format!("mod_disable: {e}"));
                false
            }
        },
        Cmd::ModProfile(None) => {
            for p in m.profiles() {
                console.print(format!("{} {p}", if p == profile { "*" } else { " " }));
            }
            false
        }
        Cmd::ModProfile(Some(name)) => match m.set_active_profile(&name) {
            Ok(()) => {
                console.print(format!("mod profile {name} {next_start}"));
                true
            }
            Err(e) => {
                console.print(format!("mod_profile: {e} (make one with: p2mods profile create {name})"));
                false
            }
        },
        Cmd::ModRollback => match m.rollback(&profile, None) {
            Ok(Some(label)) => {
                console.print(format!("undid \"{label}\" in {profile} {next_start}"));
                true
            }
            Ok(None) => {
                console.print(format!("nothing to roll back in {profile}"));
                false
            }
            Err(e) => {
                console.print(format!("mod_rollback: {e}"));
                false
            }
        },
        _ => false,
    };
    if changed {
        let engine = p2_mods::semver::Version::parse(env!("CARGO_PKG_VERSION")).ok();
        if let Ok((r, _)) = m.check(&m.active_profile(), engine.as_ref()) {
            for i in &r.issues {
                console.print(format!("  {i}"));
            }
        }
    }
}

// -------------------------------------------------------------- cheats --

/// A new map starts without noclip or god mode (sv_cheats stays).
fn cheats_on_load(mut loaded: MessageReader<LevelLoaded>, mut cheats: ResMut<Cheats>) {
    if loaded.read().count() > 0 {
        cheats.noclip = false;
        cheats.god = false;
        cheats.noclip_from = None;
    }
}

/// God mode: health far above anything a hazard does in one hit, so the
/// health module never sees the player die.
// TODO(local, health.rs): a `PlayerHealth::invulnerable` flag that skips
// damage would be cleaner than topping health up; this stays outside the file.
fn god_mode(cheats: Res<Cheats>, health: Option<ResMut<crate::health::PlayerHealth>>) {
    let Some(mut health) = health else { return };
    if cheats.god {
        health.hp = 1.0e9;
    } else if cheats.is_changed() && health.hp > 100.0 {
        health.hp = 100.0;
    }
}

/// Noclip speed: Source's `sv_noclipspeed` 5 times the walk speed.
const NOCLIP_SPEED: f32 = 5.0 * 175.0 * SOURCE_UNIT;

/// Before the movement tick: where the noclip move starts from (after any
/// portal transit last tick).
fn noclip_start(mut cheats: ResMut<Cheats>, players: Query<(&Transform, &CharacterVelocity), With<Player>>) {
    if !cheats.noclip {
        if cheats.noclip_from.is_some() {
            cheats.noclip_from = None;
        }
        return;
    }
    if let Ok((tf, v)) = players.single() {
        cheats.noclip_from = Some((tf.translation, v.0));
    }
}

/// After the movement tick: replace its result with a free flight along the
/// view (WASD, Space up, Ctrl down); walls don't stop it. Portals still work.
#[allow(clippy::type_complexity)]
fn noclip_move(
    cheats: Res<Cheats>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    locked: Res<InputLocked>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut CharacterVelocity, &ViewAngles, &MoveInput, &mut Grounded), With<Player>>,
) {
    let (true, Some((from, v0))) = (cheats.noclip, cheats.noclip_from) else { return };
    let Ok((mut tf, mut feet, mut vel, view, input, mut ground)) = players.single_mut() else { return };
    let dt = time.delta_secs();
    let rot = view.rotation();
    let mut wish = rot * Vec3::NEG_Z * input.wish.y + rot * Vec3::X * input.wish.x;
    if !locked.0 {
        if keys.pressed(KeyCode::Space) {
            wish.y += 1.0;
        }
        if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
            wish.y -= 1.0;
        }
    }
    let target = wish.clamp_length_max(1.0) * NOCLIP_SPEED;
    let v = v0.lerp(target, 1.0 - (-10.0 * dt).exp());
    let p = from + v * dt;
    tf.translation = p;
    feet.curr = p;
    vel.0 = v;
    ground.on_ground = false;
    ground.entity = None;
}

// ------------------------------------------------------------- drawing --

#[derive(Component)]
struct ConsoleRoot;
#[derive(Component)]
struct ConsoleLogText;
#[derive(Component)]
struct ConsoleInputText;
#[derive(Component)]
struct ConsoleSuggest;

const BG: Color = Color::srgba(0.05, 0.06, 0.07, 0.94);
const TEXT: Color = Color::srgb(0.85, 0.87, 0.88);
const LINE_PX: f32 = 18.0;

#[allow(clippy::too_many_arguments)]
fn draw_console(
    mut commands: Commands,
    mut console: ResMut<Console>,
    fonts: Option<Res<crate::ui::UiFonts>>,
    game: Res<p2_assets::GameFsRes>,
    mut font_assets: ResMut<Assets<Font>>,
    mut mono: Local<Option<Handle<Font>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut roots: Query<&mut Visibility, With<ConsoleRoot>>,
    mut texts: Query<(&mut Text, Has<ConsoleLogText>, Has<ConsoleInputText>)>,
    mut suggest: Query<(&mut Node, &Children), With<ConsoleSuggest>>,
) {
    let Some(fonts) = fonts else { return };
    if roots.is_empty() {
        if !console.open {
            return;
        }
        // A fixed-width font when the install has one.
        let font = mono
            .get_or_insert_with(|| {
                ["resource/linux_fonts/dejavusansmono.ttf", "resource/linux_fonts/liberationmono-regular.ttf", "resource/linux_fonts/couriernew.ttf"]
                    .iter()
                    .find_map(|p| game.read(p))
                    .map(|b| font_assets.add(Font::from_bytes(b)))
                    .unwrap_or_else(|| fonts.regular.clone())
            })
            .clone();
        let tf = |size: f32| TextFont { font: (&font).into(), font_size: bevy::text::FontSize::Px(size), ..default() };
        commands
            .spawn((
                ConsoleRoot,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(45.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BG),
                GlobalZIndex(50),
                bevy::picking::Pickable::IGNORE,
            ))
            .with_children(|p| {
                p.spawn((ConsoleLogText, Text::new(""), tf(14.0), TextColor(TEXT)));
                p.spawn((Node { height: Val::Px(1.0), margin: UiRect::vertical(Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.2))));
                p.spawn((ConsoleInputText, Text::new(""), tf(16.0), TextColor(Color::WHITE)));
            });
        commands
            .spawn((
                ConsoleSuggest,
                Node { position_type: PositionType::Absolute, top: Val::Percent(45.0), left: Val::Px(24.0), padding: UiRect::all(Val::Px(6.0)), display: Display::None, ..default() },
                BackgroundColor(Color::srgba(0.1, 0.11, 0.13, 0.96)),
                GlobalZIndex(51),
                bevy::picking::Pickable::IGNORE,
            ))
            .with_child((Text::new(""), tf(14.0), TextColor(Color::srgb(0.75, 0.8, 0.85))));
        console.dirty = true;
        return;
    }
    if !console.dirty {
        return;
    }
    console.dirty = false;
    let vis = if console.open { Visibility::Inherited } else { Visibility::Hidden };
    for mut v in &mut roots {
        *v = vis;
    }
    let height = windows.single().map_or(720.0, |w| w.height());
    let rows = (((height * 0.45 - 60.0) / LINE_PX) as usize).max(4);
    let end = console.log.len().saturating_sub(console.scroll);
    let start = end.saturating_sub(rows);
    let mut log: Vec<&str> = console.log.range(start..end).map(String::as_str).collect();
    if console.scroll > 0 {
        log.push("  ^ ^ ^ (PageDown for newer lines)");
    }
    let log = log.join("\n");
    let mut input = console.input.clone();
    input.insert(console.cursor, '_');
    let input = format!("] {input}");
    for (mut t, is_log, is_input) in &mut texts {
        if is_log && t.0 != log {
            t.0 = log.clone();
        } else if is_input && t.0 != input {
            t.0 = input.clone();
        }
    }
    let shown: Vec<String> = console
        .suggestions
        .iter()
        .take(SUGGESTIONS_SHOWN)
        .map(|s| match command(s) {
            Some(d) if !d.usage.is_empty() => format!("{s} {}", d.usage),
            _ => s.clone(),
        })
        .collect();
    let more = console.suggestions.len().saturating_sub(SUGGESTIONS_SHOWN);
    for (mut node, children) in &mut suggest {
        node.display = if console.open && !shown.is_empty() { Display::Flex } else { Display::None };
        let mut text = shown.join("\n");
        if more > 0 {
            text += &format!("\n... {more} more");
        }
        for c in children.iter() {
            if let Ok((mut t, _, _)) = texts.get_mut(c) {
                t.0 = text.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn splitting_lines() {
        assert_eq!(split_commands("map sp_a1_intro1"), vec![words("map sp_a1_intro1")]);
        assert_eq!(split_commands("  sv_cheats 1; noclip ;;god  "), vec![words("sv_cheats 1"), words("noclip"), words("god")]);
        assert_eq!(split_commands("echo \"a; b\" c"), vec![vec!["echo".to_string(), "a; b".into(), "c".into()]]);
        assert_eq!(split_commands("fire box \"\" // a comment; noclip"), vec![vec!["fire".to_string(), "box".into(), "".into()]]);
        assert!(split_commands("   ").is_empty());
        assert!(split_commands("// nothing").is_empty());
    }

    #[test]
    fn parsing_commands() {
        let p = |s: &str| parse(&words(s));
        assert_eq!(p("map sp_a2_laser_intro"), Ok(Cmd::Map("sp_a2_laser_intro".into())));
        assert_eq!(p("MAP Foo"), Ok(Cmd::Map("Foo".into())));
        assert_eq!(p("cl_showfps 1"), Ok(Cmd::ShowFps(Some(true))));
        assert_eq!(p("cl_showpos 0"), Ok(Cmd::ShowPos(Some(false))));
        assert_eq!(p("cl_showpos"), Ok(Cmd::ShowPos(None)));
        assert_eq!(p("sv_cheats on"), Ok(Cmd::SvCheats(Some(true))));
        assert_eq!(p("host_timescale 0.5"), Ok(Cmd::Timescale(Some(0.5))));
        assert_eq!(p("fov 100"), Ok(Cmd::Fov(Some(100.0))));
        assert_eq!(p("setpos -704 96 0"), Ok(Cmd::SetPos(Vec3::new(-704.0, 96.0, 0.0))));
        assert_eq!(p("setang 10 90"), Ok(Cmd::SetAng(10.0, 90.0, 0.0)));
        assert_eq!(p("fire @door Open"), Ok(Cmd::Fire { target: "@door".into(), input: "Open".into(), param: String::new() }));
        assert_eq!(p("ent_fire box SetAnimation open idle"), Ok(Cmd::Fire { target: "box".into(), input: "SetAnimation".into(), param: "open idle".into() }));
        assert_eq!(p("ent_list prop_"), Ok(Cmd::EntList(Some("prop_".into()))));
        assert_eq!(p("give weapon_PortalGun"), Ok(Cmd::Give("weapon_portalgun".into())));
        assert_eq!(p("maps *a1*"), Ok(Cmd::Maps(Some("a1".into()))));
        assert_eq!(p("practice_save start"), Ok(Cmd::PracticeSave(Some("start".into()))));
        assert_eq!(p("practice_load"), Ok(Cmd::PracticeLoad(None)));
    }

    #[test]
    fn parse_errors_say_what_to_type() {
        let p = |s: &str| parse(&words(s));
        assert_eq!(p("frobnicate"), Err("Unknown command \"frobnicate\"".into()));
        assert_eq!(p("map"), Err("Usage: map <name>".into()));
        assert_eq!(p("fire box"), Err("Usage: fire <target> <input> [param]".into()));
        assert_eq!(p("setpos 1 2"), Err("Usage: setpos <x> <y> <z>".into()));
        assert_eq!(p("setpos 1 2 three"), Err("\"three\" is not a number".into()));
        assert_eq!(p("cl_showfps maybe"), Err("Usage: cl_showfps [0/1]".into()));
        assert_eq!(p("host_timescale inf"), Err("\"inf\" is not a number".into()));
        assert_eq!(parse(&[]), Err(String::new()));
    }

    #[test]
    fn every_command_parses_and_is_sorted() {
        for d in COMMANDS {
            // A usage-shaped example for each.
            let example: Vec<String> = std::iter::once(d.name.to_string())
                .chain(d.usage.split_whitespace().filter(|u| u.starts_with('<')).map(|_| "1".to_string()))
                .collect();
            let example = if d.name.ends_with("setpos") { words("setpos 1 2 3") } else { example };
            assert!(parse(&example).is_ok(), "{}: {:?}", d.name, parse(&example));
        }
        let names: Vec<&str> = COMMANDS.iter().map(|d| d.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn cheats_are_marked() {
        for c in ["noclip", "god", "give", "upgrade_portalgun", "setpos", "setang", "host_timescale", "fire", "ent_fire"] {
            assert!(command(c).unwrap().cheat, "{c}");
        }
        for c in ["map", "restart", "getpos", "fov", "cl_showfps", "cl_showpos", "sv_cheats", "ent_list"] {
            assert!(!command(c).unwrap().cheat, "{c}");
        }
    }

    #[test]
    fn bools() {
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("2"), Some(true));
        assert_eq!(parse_bool("OFF"), Some(false));
        assert_eq!(parse_bool("x"), None);
    }

    #[test]
    fn getpos_round_trips_through_setpos() {
        let feet = source_to_world(Vec3::new(-704.0, 96.5, 12.0));
        let view = view_from_source(15.0, 135.0, 0.0);
        let text = getpos_text(feet, &view);
        assert_eq!(text, "setpos -704.00 96.50 12.00;setang 15.00 135.00 0.00");
        let cmds = split_commands(&text);
        assert_eq!(parse(&cmds[0]), Ok(Cmd::SetPos(Vec3::new(-704.0, 96.5, 12.0))));
        assert_eq!(parse(&cmds[1]), Ok(Cmd::SetAng(15.0, 135.0, 0.0)));
        // Source yaw 90 looks along Source +Y = world -Z; positive pitch looks down.
        let f = view_from_source(0.0, 90.0, 0.0).forward();
        assert!((f - Vec3::NEG_Z).length() < 1e-5);
        assert!(view_from_source(30.0, 0.0, 0.0).forward().y < 0.0);
        // Yaw wraps to -180..180.
        assert!((source_angles(&view_from_source(0.0, 270.0, 0.0)).y + 90.0).abs() < 1e-3);
    }

    #[test]
    fn history_walks_back_and_forth() {
        let mut h = History::default();
        assert_eq!(h.prev(), None);
        for l in ["noclip", "god", "god", "map a", "noclip"] {
            h.push(l);
        }
        // Repeats collapse; a re-used line moves to the newest.
        assert_eq!(h.lines(), ["god", "map a", "noclip"]);
        assert_eq!(h.prev(), Some("noclip"));
        assert_eq!(h.prev(), Some("map a"));
        assert_eq!(h.prev(), Some("god"));
        assert_eq!(h.prev(), Some("god"));
        assert_eq!(h.next(), Some("map a"));
        assert_eq!(h.next(), Some("noclip"));
        assert_eq!(h.next(), None);
        assert_eq!(h.prev(), Some("noclip"));
        for i in 0..150 {
            h.push(&format!("echo {i}"));
        }
        assert_eq!(h.lines().len(), History::MAX);
        assert_eq!(h.lines().last().unwrap(), "echo 149");
    }

    fn completer() -> Completer {
        Completer {
            maps: ["sp_a1_intro1", "sp_a1_intro2", "sp_a2_laser_intro", "my_community_map"].map(String::from).to_vec(),
            ents: vec![("@exit_door".into(), vec!["Open".into(), "Close".into()]), ("box_dropper".into(), vec!["FireUser1".into()])],
            mods: vec!["better-cubes".into(), "better-sounds".into()],
            profiles: vec!["default".into(), "speedrun".into()],
            configs: vec!["autoexec".into()],
        }
    }

    #[test]
    fn completing_commands_and_arguments() {
        let c = completer();
        assert_eq!(complete("noc", &c), (String::new(), vec!["noclip".to_string()]));
        assert_eq!(complete("cl_", &c).1, ["cl_showfps", "cl_showpos"]);
        assert_eq!(complete("map sp_a1", &c), ("map ".to_string(), vec!["sp_a1_intro1".to_string(), "sp_a1_intro2".into()]));
        assert_eq!(complete("map ", &c).1.len(), 4);
        assert_eq!(complete("fire @ex", &c).1, ["@exit_door"]);
        assert_eq!(complete("fire !p", &c).1, ["!player"]);
        let inputs = complete("fire @exit_door Op", &c);
        assert_eq!(inputs, ("fire @exit_door ".to_string(), vec!["Open".to_string()]));
        assert!(complete("fire @exit_door ", &c).1.contains(&"Close".to_string()));
        assert_eq!(complete("sv_cheats ", &c).1, ["0", "1"]);
        assert_eq!(complete("help no", &c).1, ["noclip"]);
        assert_eq!(complete("give w", &c).1, ["weapon_portalgun"]);
        assert_eq!(complete("mod_enable better-s", &c).1, ["better-sounds"]);
        assert_eq!(complete("mod_profile sp", &c).1, ["speedrun"]);
        assert_eq!(complete("exec auto", &c).1, ["autoexec"]);
        // Only the last command of a line.
        assert_eq!(complete("sv_cheats 1; nocl", &c), ("sv_cheats 1; ".to_string(), vec!["noclip".to_string()]));
        assert!(complete("echo hel", &c).1.is_empty());
        assert!(complete("zzz", &c).1.is_empty());
    }

    #[test]
    fn exec_stays_in_the_cfg_folder() {
        for bad in ["../settings", "/etc/passwd", "a/../../b", "C:evil"] {
            assert!(read_cfg(bad).unwrap_err().contains("inside the cfg folder"), "{bad}");
        }
    }

    #[test]
    fn common_prefixes() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(common_prefix(&v(&["sp_a1_intro1", "sp_a1_intro2"])), "sp_a1_intro");
        assert_eq!(common_prefix(&v(&["cl_showfps", "CL_SHOWPOS"])), "cl_show");
        assert_eq!(common_prefix(&v(&["abc"])), "abc");
        assert_eq!(common_prefix(&[]), "");
    }

    #[test]
    fn tab_completes_then_cycles() {
        let c = completer();
        let mut con = Console::default();
        con.set_input("nocl");
        con.tab(&c);
        assert_eq!(con.input, "noclip ");
        // Nothing more in common: step through the candidates, round and round.
        con.set_input("map sp_a");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro1");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro2");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a2_laser_intro");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro1");
        // The common start first, then the candidates.
        con.set_input("map sp_a1");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro1");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro2");
        con.tab(&c);
        assert_eq!(con.input, "map sp_a1_intro1");
        // Typing stops the cycle.
        con.edit(KeyCode::KeyX, Some("x"));
        assert!(con.cycling.is_none());
    }

    #[test]
    fn editing_the_input_line() {
        let mut con = Console::default();
        for ch in ["n", "o", "c", "l", "i", "p"] {
            con.edit(KeyCode::KeyA, Some(ch));
        }
        assert_eq!(con.input, "noclip");
        con.edit(KeyCode::Home, None);
        con.edit(KeyCode::Delete, None);
        con.edit(KeyCode::End, None);
        con.edit(KeyCode::Backspace, None);
        con.edit(KeyCode::ArrowLeft, None);
        con.edit(KeyCode::KeyA, Some("é"));
        assert_eq!(con.input, "ocléi");
        con.edit(KeyCode::ArrowRight, None);
        con.edit(KeyCode::Backspace, None);
        con.edit(KeyCode::Backspace, None);
        assert_eq!(con.input, "ocl");
        // Control characters and the console key don't type.
        con.edit(KeyCode::KeyC, Some("\u{3}"));
        con.edit(KeyCode::Backquote, Some("`"));
        assert_eq!(con.input, "ocl");
        con.submit();
        assert_eq!(con.input, "");
        assert_eq!(con.pending, ["ocl"]);
        assert_eq!(con.log.back().map(String::as_str), Some("] ocl"));
    }
}
