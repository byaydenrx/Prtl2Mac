//! The speedrun timer's logic, without the game around it: a run's clock
//! and splits, comparison with the personal best and best segments
//! ("golds"), their files, and practice checkpoints. `speedrun.rs` feeds it
//! map starts, exits and frame times.
use std::collections::BTreeMap;

use bevy::math::Vec3;

use crate::tool_settings::SpeedrunMode;

/// Written as the `version` line of every file here.
pub const FILE_VERSION: u32 = 1;

/// `1:02.35`, `12:04.00`, `1:02:03.45` (centiseconds, rounded down).
pub fn fmt_time(t: f64) -> String {
    let cs = (t.max(0.0) * 100.0).floor() as u64;
    let (h, m, s, c) = (cs / 360_000, cs / 6000 % 60, cs / 100 % 60, cs % 100);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}.{c:02}")
    } else {
        format!("{m}:{s:02}.{c:02}")
    }
}

/// `+1.23`, `-0.40`, `-1:05.20` (ahead is negative).
pub fn fmt_delta(d: f64) -> String {
    let sign = if d < 0.0 { '-' } else { '+' };
    let a = d.abs();
    if a >= 60.0 {
        format!("{sign}{}", fmt_time(a))
    } else {
        format!("{sign}{:.2}", (a * 100.0).floor() / 100.0)
    }
}

/// Map names as file names: lowercase letters, digits, `_ - .`.
pub fn file_safe(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') { c.to_ascii_lowercase() } else { '_' }).collect()
}

/// A run's splits (cumulative times), and its final time once finished.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Splits {
    pub category: String,
    /// Map, run time when it was left.
    pub maps: Vec<(String, f64)>,
    /// Chapter (1-based), run time when it ended.
    pub chapters: Vec<(usize, f64)>,
    /// Set when the run reached its end.
    pub total: Option<f64>,
}

fn kv_lines(text: &str) -> impl Iterator<Item = (&str, Vec<&str>)> {
    text.lines().filter_map(|l| {
        let mut w = l.split_whitespace();
        let k = w.next()?;
        Some((k, w.collect()))
    })
}

fn time_ok(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|t| t.is_finite() && *t >= 0.0)
}

impl Splits {
    pub fn parse(text: &str) -> Self {
        let mut s = Splits::default();
        for (k, v) in kv_lines(text) {
            match (k, v.as_slice()) {
                ("category", [c]) => s.category = c.to_string(),
                ("split", [m, t]) => {
                    if let Some(t) = time_ok(t) {
                        s.maps.push((m.to_string(), t));
                    }
                }
                ("chapter", [n, t]) => {
                    if let (Ok(n), Some(t)) = (n.parse(), time_ok(t)) {
                        s.chapters.push((n, t));
                    }
                }
                ("total", [t]) => s.total = time_ok(t),
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("version {FILE_VERSION}\ncategory {}\n", self.category);
        if let Some(t) = self.total {
            out += &format!("total {t:.4}\n");
        }
        for (m, t) in &self.maps {
            out += &format!("split {m} {t:.4}\n");
        }
        for (c, t) in &self.chapters {
            out += &format!("chapter {c} {t:.4}\n");
        }
        out
    }
}

/// Best segment times ever: per map, and per chapter (whole chapters only).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Golds {
    pub maps: BTreeMap<String, f64>,
    pub chapters: BTreeMap<usize, f64>,
}

impl Golds {
    pub fn parse(text: &str) -> Self {
        let mut g = Golds::default();
        for (k, v) in kv_lines(text) {
            match (k, v.as_slice()) {
                ("map", [m, t]) => {
                    if let Some(t) = time_ok(t) {
                        g.maps.insert(m.to_string(), t);
                    }
                }
                ("chapter", [n, t]) => {
                    if let (Ok(n), Some(t)) = (n.parse(), time_ok(t)) {
                        g.chapters.insert(n, t);
                    }
                }
                _ => {}
            }
        }
        g
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("version {FILE_VERSION}\n");
        for (m, t) in &self.maps {
            out += &format!("map {m} {t:.4}\n");
        }
        for (c, t) in &self.chapters {
            out += &format!("chapter {c} {t:.4}\n");
        }
        out
    }
}

/// Why a run can't be a personal best.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    Cheats,
    Practice,
}

impl Invalid {
    pub fn label(self) -> &'static str {
        match self {
            Invalid::Cheats => "CHEATS",
            Invalid::Practice => "PRACTICE",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunState {
    /// No run: the next map start begins one.
    #[default]
    Idle,
    Running,
    /// Between leaving a map and the next one loading: the clock stops.
    Loading,
    Finished,
}

/// How one split went.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitResult {
    /// The map, or `Chapter N`.
    pub name: String,
    pub segment: f64,
    /// Against the personal best at the same split (negative: ahead).
    pub delta: Option<f64>,
    /// A new best segment.
    pub gold: bool,
}

/// What the timer did when a map was left.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExitOutcome {
    pub splits: Vec<SplitResult>,
    /// The golds changed (save them).
    pub golds_changed: bool,
    /// The run finished as a new personal best (save `run.splits`).
    pub new_pb: bool,
    pub finished: bool,
}

/// One run of the timer.
#[derive(Clone, Debug, Default)]
pub struct Run {
    pub state: RunState,
    pub mode: SpeedrunMode,
    pub splits: Splits,
    pub time: f64,
    pub map: String,
    map_start: f64,
    /// The chapter being timed and when it started; `None` when the run
    /// began part-way through it (no chapter gold then).
    chapter: Option<(usize, f64)>,
    expect: Option<String>,
    pub invalid: Option<Invalid>,
    pub last: Vec<SplitResult>,
}

/// Where a map sits in the campaign, for chapter splits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapPlace {
    /// 1-based chapter, `None` for maps outside the campaign.
    pub chapter: Option<usize>,
    /// The chapter's first map.
    pub chapter_start: bool,
}

impl Run {
    pub fn category(mode: SpeedrunMode, start_map: &str) -> String {
        match mode {
            SpeedrunMode::FullRun => format!("run-{}", file_safe(start_map)),
            SpeedrunMode::SingleMap => format!("map-{}", file_safe(start_map)),
        }
    }

    /// The segment of the map being played.
    pub fn segment(&self) -> f64 {
        self.time - self.map_start
    }

    fn begin(&mut self, map: &str, mode: SpeedrunMode, place: MapPlace) {
        *self = Run {
            state: RunState::Running,
            mode,
            splits: Splits { category: Run::category(mode, map), ..Default::default() },
            map: map.to_string(),
            chapter: place.chapter.filter(|_| place.chapter_start || mode == SpeedrunMode::SingleMap).map(|c| (c, 0.0)),
            ..Default::default()
        };
    }

    /// A map finished loading. Full runs carry on into the map they were
    /// heading for and through restarts of the same map; anything else (and
    /// every load in single-map mode) starts a new run.
    pub fn map_started(&mut self, map: &str, mode: SpeedrunMode, place: MapPlace) {
        let same = map.eq_ignore_ascii_case(&self.map);
        let carry_on = mode == SpeedrunMode::FullRun
            && self.mode == mode
            && match self.state {
                RunState::Loading => self.expect.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(map)) || same,
                RunState::Running => same,
                _ => false,
            };
        if !carry_on {
            self.begin(map, mode, place);
            return;
        }
        if self.state == RunState::Loading && !same {
            self.map = map.to_string();
            self.map_start = self.time;
        }
        self.state = RunState::Running;
        self.expect = None;
    }

    pub fn tick(&mut self, dt: f64) {
        if self.state == RunState::Running && dt > 0.0 {
            self.time += dt;
        }
    }

    /// Back to no run (main menu, `speedrun_reset`).
    pub fn reset(&mut self) {
        *self = Run::default();
    }

    /// The map was left for `next` (`None`: the end of the campaign or a
    /// map outside it). `here`/`next_place` give the chapters for chapter
    /// splits. Golds update in place when the run is valid.
    pub fn map_exited(&mut self, next: Option<&str>, here: MapPlace, next_place: MapPlace, golds: &mut Golds, pb: Option<&Splits>) -> ExitOutcome {
        let mut out = ExitOutcome::default();
        if self.state != RunState::Running {
            return out;
        }
        let valid = self.invalid.is_none();
        let i = self.splits.maps.len();
        let segment = self.segment();
        let pb_split = pb.and_then(|p| p.maps.get(i)).filter(|(m, _)| m.eq_ignore_ascii_case(&self.map)).map(|(_, t)| *t);
        let gold = valid && golds.maps.get(&self.map).is_some_and(|g| segment < *g);
        if valid && golds.maps.get(&self.map).is_none_or(|g| segment < *g) {
            golds.maps.insert(self.map.clone(), segment);
            out.golds_changed = true;
        }
        self.splits.maps.push((self.map.clone(), self.time));
        out.splits.push(SplitResult { name: self.map.clone(), segment, delta: pb_split.map(|p| self.time - p), gold });

        // The chapter ends when the next map is in another one (or there's none).
        let chapter_ends = here.chapter.is_some() && (next.is_none() || next_place.chapter != here.chapter);
        if let (true, Some(ch)) = (chapter_ends, here.chapter) {
            let whole = self.chapter.filter(|(c, _)| *c == ch);
            let seg = whole.map(|(_, t0)| self.time - t0);
            let pb_ch = pb.and_then(|p| p.chapters.iter().find(|(c, _)| *c == ch)).map(|(_, t)| *t);
            let mut gold = false;
            if let (true, Some(seg)) = (valid, seg) {
                gold = golds.chapters.get(&ch).is_some_and(|g| seg < *g);
                if golds.chapters.get(&ch).is_none_or(|g| seg < *g) {
                    golds.chapters.insert(ch, seg);
                    out.golds_changed = true;
                }
            }
            self.splits.chapters.push((ch, self.time));
            out.splits.push(SplitResult { name: format!("Chapter {ch}"), segment: seg.unwrap_or(self.time - self.map_start), delta: pb_ch.map(|p| self.time - p), gold });
            self.chapter = next_place.chapter.map(|c| (c, self.time));
        }

        if self.mode == SpeedrunMode::SingleMap || next.is_none() {
            self.state = RunState::Finished;
            self.splits.total = Some(self.time);
            out.finished = true;
            out.new_pb = valid && pb.and_then(|p| p.total).is_none_or(|t| self.time < t);
        } else {
            self.state = RunState::Loading;
            self.expect = next.map(str::to_string);
        }
        self.last = out.splits.clone();
        out
    }
}

/// The best possible time from the golds: their sum over the PB's maps
/// (`None` while a map has no gold).
pub fn sum_of_best(pb: &Splits, golds: &Golds) -> Option<f64> {
    pb.maps.iter().map(|(m, _)| golds.maps.get(m).copied()).sum()
}

/// A text table of a category's PB: each split's time, segment and gold.
pub fn report(category: &str, pb: Option<&Splits>, golds: &Golds) -> Vec<String> {
    let Some(pb) = pb else { return vec![format!("{category}: no personal best yet")] };
    let mut out = vec![format!("{category}: personal best {}", pb.total.map(fmt_time).unwrap_or_else(|| "-".into()))];
    let mut prev = 0.0;
    for (m, t) in &pb.maps {
        let seg = t - prev;
        prev = *t;
        let gold = golds.maps.get(m).map(|g| format!("gold {}", fmt_time(*g))).unwrap_or_default();
        out.push(format!("  {m:<28} {:>10}  segment {:>9}  {gold}", fmt_time(*t), fmt_time(seg)));
    }
    for (c, t) in &pb.chapters {
        let gold = golds.chapters.get(c).map(|g| format!("gold {}", fmt_time(*g))).unwrap_or_default();
        out.push(format!("  Chapter {c:<20} {:>10}  {gold}", fmt_time(*t)));
    }
    if let Some(sob) = sum_of_best(pb, golds) {
        out.push(format!("sum of best {}", fmt_time(sob)));
    }
    out
}

/// A practice checkpoint: where the player was and how they were moving.
#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    pub name: String,
    /// Feet, Source units.
    pub pos: Vec3,
    /// Source units per second.
    pub vel: Vec3,
    /// Source degrees (pitch down positive, yaw from +X).
    pub pitch: f32,
    pub yaw: f32,
    pub ducked: bool,
}

/// A map's checkpoints, by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Checkpoints {
    pub list: Vec<Checkpoint>,
}

/// Checkpoint names: one word, file friendly.
pub fn checkpoint_name(s: &str) -> Option<String> {
    let n: String = s.trim().chars().filter(|c| !c.is_whitespace()).collect();
    (!n.is_empty() && n.len() <= 32 && n.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))).then_some(n)
}

impl Checkpoints {
    pub fn parse(text: &str) -> Self {
        let mut c = Checkpoints::default();
        for (k, v) in kv_lines(text) {
            if k != "checkpoint" || v.len() < 9 {
                continue;
            }
            let f: Vec<f32> = v[1..9].iter().filter_map(|x| x.parse::<f32>().ok().filter(|f| f.is_finite())).collect();
            if f.len() != 8 {
                continue;
            }
            let Some(name) = checkpoint_name(v[0]) else { continue };
            c.set(Checkpoint { name, pos: Vec3::new(f[0], f[1], f[2]), vel: Vec3::new(f[3], f[4], f[5]), pitch: f[6], yaw: f[7], ducked: v.get(9) == Some(&"ducked") });
        }
        c
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("version {FILE_VERSION}\n# checkpoint <name> <x y z> <vx vy vz> <pitch yaw> [ducked] (Source units, degrees)\n");
        for c in &self.list {
            out += &format!(
                "checkpoint {} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3}{}\n",
                c.name,
                c.pos.x,
                c.pos.y,
                c.pos.z,
                c.vel.x,
                c.vel.y,
                c.vel.z,
                c.pitch,
                c.yaw,
                if c.ducked { " ducked" } else { "" }
            );
        }
        out
    }

    /// Add or replace by name.
    pub fn set(&mut self, c: Checkpoint) {
        match self.list.iter_mut().find(|x| x.name == c.name) {
            Some(x) => *x = c,
            None => self.list.push(c),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Checkpoint> {
        self.list.iter().find(|c| c.name == name)
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let n = self.list.len();
        self.list.retain(|c| c.name != name);
        self.list.len() != n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CH1: MapPlace = MapPlace { chapter: Some(1), chapter_start: false };
    const CH1_START: MapPlace = MapPlace { chapter: Some(1), chapter_start: true };
    const CH2_START: MapPlace = MapPlace { chapter: Some(2), chapter_start: true };
    const NONE: MapPlace = MapPlace { chapter: None, chapter_start: false };

    #[test]
    fn formatting() {
        assert_eq!(fmt_time(0.0), "0:00.00");
        assert_eq!(fmt_time(62.359), "1:02.35");
        assert_eq!(fmt_time(3723.45), "1:02:03.45");
        assert_eq!(fmt_time(-3.0), "0:00.00");
        assert_eq!(fmt_delta(1.234), "+1.23");
        assert_eq!(fmt_delta(-0.4), "-0.40");
        assert_eq!(fmt_delta(-65.2), "-1:05.20");
        assert_eq!(file_safe("SP_A1 intro/1"), "sp_a1_intro_1");
    }

    /// A run over a two-map chapter 1 and the first map of chapter 2.
    fn play(run: &mut Run, golds: &mut Golds, pb: Option<&Splits>, times: [f64; 3]) -> Vec<ExitOutcome> {
        let mut out = Vec::new();
        run.map_started("a1", SpeedrunMode::FullRun, CH1_START);
        run.tick(times[0]);
        out.push(run.map_exited(Some("a2"), CH1_START, CH1, golds, pb));
        // Loading time doesn't count.
        run.tick(100.0);
        run.map_started("a2", SpeedrunMode::FullRun, CH1);
        run.tick(times[1]);
        out.push(run.map_exited(Some("b1"), CH1, CH2_START, golds, pb));
        run.map_started("b1", SpeedrunMode::FullRun, CH2_START);
        run.tick(times[2]);
        out.push(run.map_exited(None, CH2_START, NONE, golds, pb));
        out
    }

    #[test]
    fn a_full_run_splits_per_map_and_chapter() {
        let mut run = Run::default();
        let mut golds = Golds::default();
        let out = play(&mut run, &mut golds, None, [10.0, 20.0, 5.0]);
        assert_eq!(run.splits.category, "run-a1");
        assert_eq!(run.splits.maps, [("a1".to_string(), 10.0), ("a2".into(), 30.0), ("b1".into(), 35.0)]);
        assert_eq!(run.splits.chapters, [(1, 30.0), (2, 35.0)]);
        assert_eq!(run.splits.total, Some(35.0));
        assert_eq!(run.state, RunState::Finished);
        // The first run: every segment becomes the gold, none is "gold" yet.
        assert!(out.iter().flat_map(|o| &o.splits).all(|s| !s.gold && s.delta.is_none()));
        assert!(out[2].finished && out[2].new_pb);
        assert_eq!(golds.maps["a2"], 20.0);
        assert_eq!(golds.chapters[&1], 30.0);
        // Chapter 1's split is listed after a2's.
        assert_eq!(out[1].splits.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["a2", "Chapter 1"]);
    }

    #[test]
    fn comparing_with_the_personal_best() {
        let mut golds = Golds::default();
        let mut first = Run::default();
        play(&mut first, &mut golds, None, [10.0, 20.0, 5.0]);
        let pb = first.splits.clone();
        let mut run = Run::default();
        let out = play(&mut run, &mut golds, Some(&pb), [12.0, 15.0, 6.0]);
        let a1 = &out[0].splits[0];
        assert_eq!((a1.delta, a1.gold), (Some(2.0), false));
        let a2 = &out[1].splits[0];
        // 27 against 30, and a 15 s segment beats the 20 s gold.
        assert_eq!((a2.delta, a2.gold, a2.segment), (Some(-3.0), true, 15.0));
        let ch1 = &out[1].splits[1];
        assert_eq!((ch1.delta, ch1.gold, ch1.segment), (Some(-3.0), true, 27.0));
        assert_eq!(golds.maps["a2"], 15.0);
        assert_eq!(golds.maps["a1"], 10.0);
        assert!(out[2].new_pb);
        // A slower finish is not a PB.
        let mut slow = Run::default();
        let out = play(&mut slow, &mut golds, Some(&run.splits), [30.0, 30.0, 30.0]);
        assert!(out[2].finished && !out[2].new_pb);
    }

    #[test]
    fn restarts_carry_on_and_other_maps_start_over() {
        let mut run = Run::default();
        run.map_started("a1", SpeedrunMode::FullRun, CH1_START);
        run.tick(5.0);
        // A death or restart reloads the same map: the clock keeps going.
        run.map_started("a1", SpeedrunMode::FullRun, CH1_START);
        run.tick(5.0);
        assert_eq!((run.time, run.segment()), (10.0, 10.0));
        // Starting another map (chapter select) is a new run.
        run.map_started("b1", SpeedrunMode::FullRun, CH2_START);
        assert_eq!((run.time, run.splits.category.as_str()), (0.0, "run-b1"));
        // Not ticking while idle or finished.
        run.reset();
        run.tick(3.0);
        assert_eq!(run.time, 0.0);
    }

    #[test]
    fn single_map_runs() {
        let mut golds = Golds::default();
        let mut run = Run::default();
        run.map_started("a2", SpeedrunMode::SingleMap, CH1);
        run.tick(8.0);
        // A restart starts the clock again.
        run.map_started("a2", SpeedrunMode::SingleMap, CH1);
        run.tick(7.0);
        let out = run.map_exited(Some("b1"), CH1, CH2_START, &mut golds, None);
        assert_eq!(run.splits.category, "map-a2");
        assert!(out.finished && out.new_pb);
        assert_eq!(run.splits.total, Some(7.0));
        assert_eq!(golds.maps["a2"], 7.0);
        // Exiting again does nothing.
        assert_eq!(run.map_exited(Some("b1"), CH1, CH2_START, &mut golds, None), ExitOutcome::default());
    }

    #[test]
    fn a_run_started_mid_chapter_has_no_chapter_gold() {
        let mut golds = Golds::default();
        let mut run = Run::default();
        run.map_started("a2", SpeedrunMode::FullRun, CH1);
        run.tick(4.0);
        let out = run.map_exited(Some("b1"), CH1, CH2_START, &mut golds, None);
        assert!(golds.chapters.is_empty());
        assert_eq!(out.splits[1].name, "Chapter 1");
        // Chapter 2 is timed whole from here.
        run.map_started("b1", SpeedrunMode::FullRun, CH2_START);
        run.tick(6.0);
        run.map_exited(None, CH2_START, NONE, &mut golds, None);
        assert_eq!(golds.chapters[&2], 6.0);
    }

    #[test]
    fn invalid_runs_keep_golds_and_pbs() {
        let mut golds = Golds::default();
        let mut run = Run::default();
        run.map_started("a2", SpeedrunMode::SingleMap, CH1);
        run.invalid = Some(Invalid::Practice);
        run.tick(1.0);
        let out = run.map_exited(None, CH1, NONE, &mut golds, None);
        assert!(out.finished && !out.new_pb && !out.golds_changed);
        assert!(golds.maps.is_empty());
        // A new run is valid again.
        run.map_started("a2", SpeedrunMode::SingleMap, CH1);
        assert_eq!(run.invalid, None);
    }

    #[test]
    fn files_round_trip() {
        let s = Splits { category: "run-sp_a1_intro1".into(), maps: vec![("sp_a1_intro1".into(), 61.25), ("sp_a1_intro2".into(), 99.5)], chapters: vec![(1, 99.5)], total: Some(99.5) };
        assert_eq!(Splits::parse(&s.to_text()), s);
        let g = Golds { maps: [("a".to_string(), 1.5)].into(), chapters: [(3, 120.0)].into() };
        assert_eq!(Golds::parse(&g.to_text()), g);
        // Junk lines and bad numbers are skipped.
        let s = Splits::parse("version 9\nsplit a nan\nsplit b -1\nsplit c 2\nfuture stuff\ntotal x\n");
        assert_eq!(s.maps, [("c".to_string(), 2.0)]);
        assert_eq!(s.total, None);
    }

    #[test]
    fn sum_of_best_and_report() {
        let pb = Splits { category: "run-a1".into(), maps: vec![("a1".into(), 10.0), ("a2".into(), 30.0)], chapters: vec![(1, 30.0)], total: Some(30.0) };
        let mut golds = Golds { maps: [("a1".to_string(), 9.0)].into(), chapters: [(1, 28.0)].into() };
        assert_eq!(sum_of_best(&pb, &golds), None);
        golds.maps.insert("a2".into(), 18.5);
        assert_eq!(sum_of_best(&pb, &golds), Some(27.5));
        let r = report("run-a1", Some(&pb), &golds);
        assert_eq!(r[0], "run-a1: personal best 0:30.00");
        assert!(r[2].contains("a2") && r[2].contains("segment   0:20.00") && r[2].contains("gold 0:18.50"), "{:?}", r[2]);
        assert!(r[3].contains("Chapter 1") && r[3].contains("gold 0:28.00"));
        assert_eq!(r.last().unwrap(), "sum of best 0:27.50");
        assert_eq!(report("map-x", None, &golds), ["map-x: no personal best yet"]);
    }

    #[test]
    fn checkpoints() {
        let mut c = Checkpoints::default();
        let a = Checkpoint { name: "1".into(), pos: Vec3::new(-704.0, 96.5, 12.0), vel: Vec3::new(0.0, 175.0, -20.0), pitch: 10.0, yaw: 90.0, ducked: false };
        let b = Checkpoint { name: "before_jump".into(), ducked: true, ..a.clone() };
        c.set(a.clone());
        c.set(b.clone());
        c.set(Checkpoint { yaw: 45.0, ..a.clone() });
        assert_eq!(c.list.len(), 2);
        assert_eq!(c.get("1").unwrap().yaw, 45.0);
        let back = Checkpoints::parse(&c.to_text());
        assert_eq!(back, c);
        assert!(back.get("before_jump").unwrap().ducked);
        assert!(c.remove("1") && !c.remove("1"));
        assert_eq!(Checkpoints::parse("checkpoint x 1 2 3\ncheckpoint bad/name 1 2 3 4 5 6 7 8\n").list.len(), 0);
        assert_eq!(checkpoint_name(" a b "), Some("ab".into()));
        assert_eq!(checkpoint_name(""), None);
        assert_eq!(checkpoint_name("../x"), None);
    }
}
