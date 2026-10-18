//! Speedrun and challenge tools, all off until turned on in Options >
//! Speedrun:
//!
//! * The timer: in-game time (stops in the pause menu, the console and
//!   loads), split automatically when a map is left and when a chapter ends,
//!   compared with the personal best and the best segments ("golds"), kept
//!   in `<data dir>/splits/` (macOS: `~/Library/Application Support/Portal2Rust/splits/`).
//!   A run with `sv_cheats 1` or a practice checkpoint loaded is never saved
//!   as a PB. Full-run mode carries one run across maps; single-map mode
//!   times each map on its own.
//! * Quick restart (default F6): start the current map again.
//! * Practice checkpoints (default F7 saves, F8 loads; the slot is chosen in
//!   Options): position, velocity, view and crouch, several named ones per map
//!   (`practice_save name` in the console), in `<data dir>/practice/<map>.txt`.
//!
//! The logic lives in [`crate::splits`].
use std::path::PathBuf;

use bevy::prelude::*;
use p2_core::*;

use crate::campaign::Campaign;
use crate::console::{Cheats, Console, ConsolePrint};
use crate::level::{CurrentMap, GameSet, LevelLoaded, LoadLevel};
use crate::settings::{data_dir, may_write_data, Settings};
use crate::splits::{self, fmt_delta, fmt_time, Checkpoint, Checkpoints, Golds, Invalid, MapPlace, Run, RunState, Splits};
use crate::ui::Screen;

pub struct SpeedrunPlugin;

impl Plugin for SpeedrunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Speedrun>()
            .add_message::<PracticeRequest>()
            .add_systems(Update, (keys, timer, practice, draw_hud).chain().in_set(GameSet::Logic));
    }
}

/// Practice and timer requests (keys and console commands).
#[derive(Message, Clone, Debug, PartialEq)]
pub enum PracticeRequest {
    /// Save here under a name (`None`: the slot chosen in Options).
    Save(Option<String>),
    Load(Option<String>),
    Delete(String),
    List,
    ResetTimer,
    /// Print the PB, its splits, the golds and the sum of best.
    ShowSplits,
}

#[derive(Resource, Default)]
pub struct Speedrun {
    pub run: Run,
    golds: Option<Golds>,
    /// The personal best of the run's category (loaded when the run starts).
    pb: Option<Splits>,
    /// The frame after a load: its time is the load's, not the game's.
    skip_frame: bool,
    /// A short message on the HUD (checkpoint saved...), until this real time.
    toast: Option<(String, f32)>,
}

fn splits_dir() -> PathBuf {
    data_dir().join("splits")
}

fn practice_file(map: &str) -> PathBuf {
    data_dir().join("practice").join(format!("{}.txt", splits::file_safe(map)))
}

fn write(path: &std::path::Path, text: &str) {
    if !may_write_data() {
        return;
    }
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if let Err(e) = std::fs::write(path, text) {
        warn!("portal2: can't write {}: {e}", path.display());
    }
}

fn place(campaign: &Campaign, map: &str) -> MapPlace {
    MapPlace { chapter: campaign.chapter_of(map).map(|c| c + 1), chapter_start: campaign.chapters.iter().any(|c| c.map.eq_ignore_ascii_case(map)) }
}

/// Quick restart and the checkpoint keys.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    console: Res<Console>,
    screen: Res<Screen>,
    map: Option<Res<CurrentMap>>,
    mut load: MessageWriter<LoadLevel>,
    mut practice: MessageWriter<PracticeRequest>,
) {
    let t = &settings.tools;
    let playing = !console.open && *screen == Screen::None && map.as_ref().is_some_and(|m| !m.menu);
    if !playing {
        return;
    }
    if t.quick_restart && keys.just_pressed(t.quick_restart_key) {
        if let Some(m) = &map {
            info!("portal2: quick restart of {}", m.info.name);
            load.write(LoadLevel { map: m.info.name.clone(), menu: false });
        }
    }
    if t.practice && keys.just_pressed(t.checkpoint_save_key) {
        practice.write(PracticeRequest::Save(None));
    }
    if t.practice && keys.just_pressed(t.checkpoint_load_key) {
        practice.write(PracticeRequest::Load(None));
    }
}

#[allow(clippy::too_many_arguments)]
fn timer(
    mut sr: ResMut<Speedrun>,
    settings: Res<Settings>,
    campaign: Res<Campaign>,
    cheats: Res<Cheats>,
    console: Res<Console>,
    screen: Res<Screen>,
    real: Res<Time<Real>>,
    vtime: Res<Time<Virtual>>,
    map: Option<Res<CurrentMap>>,
    mut loaded: MessageReader<LevelLoaded>,
    mut exits: MessageReader<crate::triggers::LevelExit>,
    mut print: MessageWriter<ConsolePrint>,
) {
    let t = &settings.tools;
    if !t.speedrun_timer {
        loaded.clear();
        exits.clear();
        if sr.run.state != RunState::Idle {
            sr.run.reset();
        }
        return;
    }
    if sr.golds.is_none() {
        sr.golds = Some(std::fs::read_to_string(splits_dir().join("golds.txt")).map(|s| Golds::parse(&s)).unwrap_or_default());
    }
    for ev in loaded.read() {
        sr.skip_frame = true;
        if ev.menu {
            sr.run.reset();
            continue;
        }
        let name = map.as_ref().map_or(ev.map.clone(), |m| m.info.name.clone());
        let before = sr.run.splits.category.clone();
        let p = place(&campaign, &name);
        sr.run.map_started(&name, t.speedrun_mode, p);
        if sr.run.splits.category != before || sr.run.time == 0.0 {
            let cat = sr.run.splits.category.clone();
            sr.pb = std::fs::read_to_string(splits_dir().join(format!("{cat}.txt"))).ok().map(|s| Splits::parse(&s));
        }
    }
    // Cheats or a changed game speed spoil the run.
    if sr.run.state == RunState::Running && sr.run.invalid.is_none() && (cheats.sv_cheats || vtime.relative_speed() != 1.0) {
        sr.run.invalid = Some(Invalid::Cheats);
    }
    if exits.read().count() > 0 && sr.run.state == RunState::Running {
        if let Some(m) = &map {
            let here = m.info.name.clone();
            let next = campaign.next_map(&here).map(str::to_string);
            let (hp, np) = (place(&campaign, &here), next.as_deref().map(|n| place(&campaign, n)).unwrap_or_default());
            let mut golds = sr.golds.take().unwrap_or_default();
            let pb = sr.pb.clone();
            let out = sr.run.map_exited(next.as_deref(), hp, np, &mut golds, pb.as_ref());
            if out.golds_changed {
                write(&splits_dir().join("golds.txt"), &golds.to_text());
            }
            sr.golds = Some(golds);
            for s in &out.splits {
                let delta = s.delta.map(fmt_delta).unwrap_or_default();
                print.write(ConsolePrint(format!("split {}: {} (segment {}) {delta}{}", s.name, fmt_time(sr.run.time), fmt_time(s.segment), if s.gold { " gold" } else { "" })));
            }
            if out.finished {
                print.write(ConsolePrint(format!("run finished: {}{}", fmt_time(sr.run.time), if out.new_pb { " (personal best!)" } else { "" })));
            }
            if out.new_pb {
                let cat = sr.run.splits.category.clone();
                write(&splits_dir().join(format!("{cat}.txt")), &sr.run.splits.to_text());
                sr.pb = Some(sr.run.splits.clone());
            }
        }
    }
    // The clock runs while playing: not in menus, the console, the pause
    // menu or the frame after a load.
    let playing = *screen == Screen::None && !console.open && !vtime.is_paused() && map.as_ref().is_some_and(|m| !m.menu);
    let skip = std::mem::take(&mut sr.skip_frame);
    if playing && !skip {
        sr.run.tick(real.delta_secs_f64());
    }
}

/// Save, load, list and delete checkpoints; reset the timer.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn practice(
    mut requests: MessageReader<PracticeRequest>,
    mut sr: ResMut<Speedrun>,
    settings: Res<Settings>,
    map: Option<Res<CurrentMap>>,
    real: Res<Time<Real>>,
    mut players: Query<(Entity, &mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity, &Crouch), With<Player>>,
    mut duck: MessageWriter<ForceDuck>,
    mut print: MessageWriter<ConsolePrint>,
) {
    let now = real.elapsed_secs();
    let say = |sr: &mut Speedrun, print: &mut MessageWriter<ConsolePrint>, s: String| {
        sr.toast = Some((s.clone(), now + 2.0));
        print.write(ConsolePrint(s));
    };
    for req in requests.read() {
        if *req == PracticeRequest::ResetTimer {
            sr.run.reset();
            say(&mut sr, &mut print, "speedrun timer reset".into());
            continue;
        }
        if *req == PracticeRequest::ShowSplits {
            for l in splits_report(&sr, &settings) {
                print.write(ConsolePrint(l));
            }
            continue;
        }
        let Some(m) = map.as_ref().filter(|m| !m.menu) else {
            print.write(ConsolePrint("practice: not in a map".into()));
            continue;
        };
        let file = practice_file(&m.info.name);
        let mut cps = std::fs::read_to_string(&file).map(|s| Checkpoints::parse(&s)).unwrap_or_default();
        let slot = settings.tools.checkpoint_slot.to_string();
        let name_of = |n: &Option<String>| n.as_deref().map_or(Some(slot.clone()), splits::checkpoint_name);
        match req {
            PracticeRequest::Save(n) => {
                let Some(name) = name_of(n) else {
                    say(&mut sr, &mut print, "practice: names are one word (letters, digits, _ - .)".into());
                    continue;
                };
                let Ok((_, tf, _, view, vel, crouch)) = players.single() else { continue };
                let a = crate::console::source_angles(view);
                cps.set(Checkpoint { name: name.clone(), pos: crate::console::to_source(tf.translation), vel: crate::console::to_source(vel.0), pitch: a.x, yaw: a.y, ducked: crouch.ducked });
                write(&file, &cps.to_text());
                say(&mut sr, &mut print, format!("checkpoint {name} saved"));
            }
            PracticeRequest::Load(n) => {
                let Some(name) = name_of(n) else { continue };
                let Some(c) = cps.get(&name).cloned() else {
                    say(&mut sr, &mut print, format!("no checkpoint {name} on this map (save one first)"));
                    continue;
                };
                let Ok((e, mut tf, mut feet, mut view, mut vel, _)) = players.single_mut() else { continue };
                let p = source_to_world(c.pos);
                tf.translation = p;
                *feet = InterpolatedFeet { prev: p, curr: p };
                *view = crate::console::view_from_source(c.pitch, c.yaw, 0.0);
                vel.0 = source_to_world(c.vel);
                if c.ducked {
                    duck.write(ForceDuck { entity: e });
                }
                if matches!(sr.run.state, RunState::Running | RunState::Loading) {
                    sr.run.invalid = Some(Invalid::Practice);
                }
                say(&mut sr, &mut print, format!("checkpoint {name} loaded"));
            }
            PracticeRequest::Delete(n) => {
                if cps.remove(n) {
                    write(&file, &cps.to_text());
                    say(&mut sr, &mut print, format!("checkpoint {n} deleted"));
                } else {
                    say(&mut sr, &mut print, format!("no checkpoint {n}"));
                }
            }
            PracticeRequest::List => {
                if cps.list.is_empty() {
                    print.write(ConsolePrint(format!("no checkpoints on {}", m.info.name)));
                }
                for c in &cps.list {
                    print.write(ConsolePrint(format!("  {}  setpos {:.0} {:.0} {:.0}{}", c.name, c.pos.x, c.pos.y, c.pos.z, if c.ducked { " (ducked)" } else { "" })));
                }
            }
            PracticeRequest::ResetTimer | PracticeRequest::ShowSplits => {}
        }
    }
}

/// The `speedrun_splits` lines: the run's category, its PB split by split
/// with the golds, and the sum of best.
fn splits_report(sr: &Speedrun, settings: &Settings) -> Vec<String> {
    if !settings.tools.speedrun_timer {
        return vec!["the speedrun timer is off (Options > Speedrun)".into()];
    }
    if sr.run.state == RunState::Idle {
        return vec!["no run yet: start a map".into()];
    }
    let golds = sr.golds.clone().unwrap_or_default();
    splits::report(&sr.run.splits.category, sr.pb.as_ref(), &golds)
}

#[derive(Component)]
struct SpeedrunHud;
#[derive(Component)]
struct HudLine(usize);

const GOLD: Color = Color::srgb(1.0, 0.82, 0.25);
const AHEAD: Color = Color::srgb(0.35, 0.9, 0.45);
const BEHIND: Color = Color::srgb(0.95, 0.4, 0.35);
const PLAIN: Color = Color::srgb(0.92, 0.94, 0.95);
const DIM: Color = Color::srgba(0.75, 0.78, 0.8, 0.8);

/// Top right: the run time, this map's segment, the last split against the
/// PB, and short practice messages.
#[allow(clippy::too_many_arguments)]
fn draw_hud(
    mut commands: Commands,
    sr: Res<Speedrun>,
    settings: Res<Settings>,
    fonts: Option<Res<crate::ui::UiFonts>>,
    screen: Res<Screen>,
    real: Res<Time<Real>>,
    map: Option<Res<CurrentMap>>,
    roots: Query<Entity, With<SpeedrunHud>>,
    mut lines: Query<(&HudLine, &mut Text, &mut TextColor)>,
) {
    let toast = sr.toast.as_ref().filter(|(_, until)| real.elapsed_secs() < *until).map(|(s, _)| s.clone());
    let in_game = map.as_ref().is_some_and(|m| !m.menu) && matches!(*screen, Screen::None | Screen::Pause);
    let show = in_game && (settings.tools.speedrun_timer || toast.is_some());
    if !show {
        for r in &roots {
            commands.entity(r).despawn();
        }
        return;
    }
    let Some(fonts) = fonts else { return };
    if roots.is_empty() {
        let tf = |f: &Handle<Font>, size: f32| TextFont { font: f.into(), font_size: bevy::text::FontSize::Px(size), ..default() };
        commands
            .spawn((
                SpeedrunHud,
                Node { position_type: PositionType::Absolute, top: Val::Px(12.0), right: Val::Px(16.0), flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..default() },
                GlobalZIndex(4),
                bevy::picking::Pickable::IGNORE,
            ))
            .with_children(|p| {
                p.spawn((HudLine(0), Text::new(""), tf(&fonts.bold, 30.0), TextColor(PLAIN)));
                p.spawn((HudLine(1), Text::new(""), tf(&fonts.regular, 16.0), TextColor(DIM)));
                p.spawn((HudLine(2), Text::new(""), tf(&fonts.bold, 18.0), TextColor(PLAIN)));
                p.spawn((HudLine(3), Text::new(""), tf(&fonts.regular, 16.0), TextColor(DIM)));
                p.spawn((HudLine(4), Text::new(""), tf(&fonts.regular, 16.0), TextColor(PLAIN)));
            });
        return;
    }
    let run = &sr.run;
    let timer_on = settings.tools.speedrun_timer;
    let main = if timer_on { fmt_time(run.time) } else { String::new() };
    let tag = run.invalid.map(|i| format!("  [{}]", i.label())).unwrap_or_default();
    let seg = if timer_on && run.state != RunState::Idle { format!("{}  {}{tag}", run.map, fmt_time(run.segment())) } else { String::new() };
    let (last, last_color) = match run.last.last().filter(|_| timer_on) {
        Some(s) => {
            let c = if s.gold {
                GOLD
            } else {
                match s.delta {
                    Some(d) if d < 0.0 => AHEAD,
                    Some(_) => BEHIND,
                    None => PLAIN,
                }
            };
            (format!("{}  {}", s.name, s.delta.map(fmt_delta).unwrap_or_else(|| fmt_time(s.segment))), c)
        }
        None => (String::new(), PLAIN),
    };
    let pb = match sr.pb.as_ref().and_then(|p| p.total) {
        Some(t) if timer_on => format!("PB {}", fmt_time(t)),
        _ => String::new(),
    };
    let main_color = if run.state == RunState::Finished { GOLD } else if run.invalid.is_some() { DIM } else { PLAIN };
    for (l, mut t, mut c) in &mut lines {
        let (s, col) = match l.0 {
            0 => (main.clone(), main_color),
            1 => (seg.clone(), DIM),
            2 => (last.clone(), last_color),
            3 => (pb.clone(), DIM),
            _ => (toast.clone().unwrap_or_default(), PLAIN),
        };
        if t.0 != s {
            t.0 = s;
        }
        if c.0 != col {
            c.0 = col;
        }
    }
}
