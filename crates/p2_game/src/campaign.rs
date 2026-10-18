//! The single-player campaign as the install describes it: the map order and
//! chapter starts from `scripts/vscripts/transitions/sp_transition_list.nut`,
//! chapter titles from `resource/portal2_english.txt`, chapter pictures from
//! `materials/vgui/chapters/`. Read at startup; nothing is hard-coded.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_assets::GameFsRes;

const TRANSITIONS: &str = "scripts/vscripts/transitions/sp_transition_list.nut";

/// One chapter: where it starts and what the menu calls it.
#[derive(Clone, Debug)]
pub struct Chapter {
    pub map: String,
    pub title: String,
    pub subtitle: String,
    /// `materials/vgui/chapters/chapterN` (without extension).
    pub picture: String,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct Campaign {
    /// Every single-player map in play order.
    pub maps: Vec<String>,
    pub chapters: Vec<Chapter>,
    /// Localised UI strings (`#token` without the `#`, lowercased).
    pub strings: HashMap<String, String>,
    /// The script's string constants (`FIRST_MAP_WITH_GUN <- "sp_a1_intro4"`).
    pub consts: HashMap<String, String>,
}

impl Campaign {
    pub fn load(fs: &source_assets::GameFs) -> Self {
        let mut c = Campaign { strings: load_strings(fs), ..default() };
        let Some(text) = fs.read(TRANSITIONS).map(|b| String::from_utf8_lossy(&b).into_owned()) else {
            warn!("portal2: {TRANSITIONS} missing; the campaign has no map order");
            return c;
        };
        c.maps = map_play_order(&text);
        for line in text.lines() {
            if let Some((k, v)) = line.split_once("<-") {
                let v = v.trim().trim_matches('"');
                if !k.trim().is_empty() && !v.is_empty() && !v.contains('[') && !v.contains('{') {
                    c.consts.insert(k.trim().to_string(), v.to_string());
                }
            }
        }
        for (i, (map, title, subtitle)) in chapter_starts(&text).into_iter().enumerate() {
            let title = c.text(&title);
            let subtitle = c.text(&subtitle);
            c.chapters.push(Chapter { map, title, subtitle, picture: format!("vgui/chapters/chapter{}", i + 1) });
        }
        info!("portal2: campaign of {} maps in {} chapters", c.maps.len(), c.chapters.len());
        c
    }

    /// A `#token` (or plain text) as the player reads it.
    pub fn text(&self, token: &str) -> String {
        let key = token.trim_start_matches('#').to_ascii_lowercase();
        self.strings.get(&key).cloned().unwrap_or_else(|| token.trim_start_matches('#').to_string())
    }

    /// The map after `map` in play order.
    pub fn next_map(&self, map: &str) -> Option<&str> {
        let i = self.maps.iter().position(|m| m.eq_ignore_ascii_case(map))?;
        self.maps.get(i + 1).map(String::as_str)
    }

    /// The chapter `map` belongs to (index).
    pub fn chapter_of(&self, map: &str) -> Option<usize> {
        let i = self.maps.iter().position(|m| m.eq_ignore_ascii_case(map))?;
        self.chapters
            .iter()
            .rposition(|c| self.maps.iter().position(|m| m.eq_ignore_ascii_case(&c.map)).is_some_and(|ci| ci <= i))
    }
}

/// `MapPlayOrder <- [ "sp_a1_intro1", ... ]`: the quoted `sp_` names, in order,
/// skipping commented-out lines.
fn map_play_order(text: &str) -> Vec<String> {
    let Some(start) = text.find("MapPlayOrder") else { return Vec::new() };
    let body = &text[start..];
    let end = body.find(']').unwrap_or(body.len());
    let mut out = Vec::new();
    for line in body[..end].lines() {
        let line = line.split("//").next().unwrap_or("");
        for part in line.split('"').skip(1).step_by(2) {
            if part.starts_with("sp_") && !out.iter().any(|m: &String| m == part) {
                out.push(part.to_string());
            }
        }
    }
    out
}

/// The chapter table: `{ map = "...", title_text = "#...", subtitle_text = "#..." ... }`.
fn chapter_starts(text: &str) -> Vec<(String, String, String)> {
    let field = |line: &str, key: &str| -> Option<String> {
        let i = line.find(key)?;
        let rest = &line[i + key.len()..];
        let a = rest.find('"')? + 1;
        let b = rest[a..].find('"')? + a;
        Some(rest[a..b].to_string())
    };
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .filter_map(|l| Some((field(l, "map =")?, field(l, "title_text =")?, field(l, "subtitle_text =")?)))
        .collect()
}

/// `resource/portal2_<lang>.txt` and `gameui_<lang>.txt` (UTF-16 KeyValues).
fn load_strings(fs: &source_assets::GameFs) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for f in ["resource/gameui_english.txt", "resource/portal2_english.txt", "resource/basemodui_english.txt"] {
        let Some(bytes) = fs.read(f) else { continue };
        let text = decode_text(&bytes);
        let kv = source_assets::kv::parse(&text);
        let Some((_, root)) = kv.root() else { continue };
        if let Some(tokens) = root.block("tokens") {
            for (k, v) in &tokens.entries {
                if let source_assets::kv::KvValue::Str(s) = v {
                    out.insert(k.clone(), s.clone());
                }
            }
        }
    }
    out
}

/// Valve's localisation files are UTF-16LE with a BOM; others are UTF-8.
fn decode_text(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xFE {
        let units: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(b).into_owned()
    }
}

pub fn load_campaign(mut commands: Commands, game: Res<GameFsRes>) {
    commands.insert_resource(Campaign::load(&game.0));
}
