//! Subtitles: the game's own caption text (`resource/closecaption_english.txt`
//! and `subtitles_english.txt`, keyed by soundscript name) shown at the
//! bottom of the screen while a voice line plays.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_assets::GameFsRes;

use crate::settings::Settings;
use crate::ui::UiFonts;

pub struct CaptionsPlugin;

impl Plugin for CaptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptionLines>()
            .add_message::<ShowCaption>()
            .add_systems(Startup, load_captions)
            .add_systems(Update, (queue_captions, draw_captions).chain());
    }
}

/// A voice line started: show its caption for `seconds`.
#[derive(Message, Clone, Debug)]
pub struct ShowCaption {
    pub sound: String,
    pub seconds: f32,
}

#[derive(Resource, Default)]
struct CaptionText {
    /// From the text files, by lowercased token.
    by_name: HashMap<String, (String, Color)>,
    /// From the compiled `.dat` files, by CRC32 of the lowercased token.
    by_hash: HashMap<u32, (String, Color)>,
}

impl CaptionText {
    fn get(&self, token: &str) -> Option<(String, Color)> {
        let t = token.to_ascii_lowercase();
        self.by_name.get(&t).or_else(|| self.by_hash.get(&crc32(t.as_bytes()))).cloned()
    }
}

/// CRC-32 (IEEE), as Valve hashes caption tokens.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A compiled caption file (VCCD): a directory of (hash, block, offset,
/// length) and blocks of UTF-16 text.
fn read_vccd(b: &[u8], out: &mut HashMap<u32, (String, Color)>) {
    let i32_at = |o: usize| b.get(o..o + 4).map(|x| i32::from_le_bytes([x[0], x[1], x[2], x[3]]));
    if b.get(0..4) != Some(b"VCCD") {
        return;
    }
    let (Some(block_size), Some(entries), Some(data)) = (i32_at(12), i32_at(16), i32_at(20)) else { return };
    for k in 0..entries.max(0) as usize {
        let e = 24 + k * 12;
        let (Some(hash), Some(block)) = (b.get(e..e + 4), i32_at(e + 4)) else { break };
        let hash = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        let (Some(off), Some(len)) = (b.get(e + 8..e + 10), b.get(e + 10..e + 12)) else { break };
        let off = u16::from_le_bytes([off[0], off[1]]) as usize;
        let len = u16::from_le_bytes([len[0], len[1]]) as usize;
        let start = data as usize + block as usize * block_size as usize + off;
        let Some(text) = b.get(start..start + len) else { continue };
        let units: Vec<u16> = text.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|u| *u != 0).collect();
        out.insert(hash, clean(&String::from_utf16_lossy(&units)));
    }
}

/// Captions on screen: (text, colour, until).
#[derive(Resource, Default)]
struct CaptionLines(Vec<(String, Color, f32)>);

/// Valve caption markup to plain text and a colour: `<clr:r,g,b>` sets the
/// colour; `<sfx>`, `<norepeat:n>`, `<len:n>`, `<I>`, `<B>`... are dropped.
fn clean(raw: &str) -> (String, Color) {
    let mut out = String::new();
    let mut color = Color::srgb(0.95, 0.95, 0.95);
    let mut rest = raw;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('>') else {
            rest = "";
            break;
        };
        let tag = &rest[i + 1..i + j];
        if let Some(c) = tag.strip_prefix("clr:") {
            let v: Vec<f32> = c.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if v.len() >= 3 {
                color = Color::srgb(v[0] / 255.0, v[1] / 255.0, v[2] / 255.0);
            }
        }
        if tag == "cr" {
            out.push('\n');
        }
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    (out.trim().to_string(), color)
}

fn decode(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xFE {
        let units: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(b).into_owned()
    }
}

fn load_captions(mut commands: Commands, game: Res<GameFsRes>) {
    let mut map = HashMap::new();
    for f in ["resource/closecaption_english.txt", "resource/subtitles_english.txt"] {
        let Some(b) = game.read(f) else { continue };
        let kv = source_assets::kv::parse(&decode(&b));
        let Some((_, root)) = kv.root() else { continue };
        if let Some(tokens) = root.block("tokens") {
            for (k, v) in &tokens.entries {
                if let source_assets::kv::KvValue::Str(s) = v {
                    map.insert(k.to_ascii_lowercase(), clean(s));
                }
            }
        }
    }
    // Each game folder ships its own caption files with the same names (the
    // main campaign's, DLC1's, DLC2's); the search path would only show the
    // first, so read every folder's copy, base game first.
    let mut by_hash = HashMap::new();
    let root = game.0.config.root.clone();
    for dir in ["portal2", "portal2_dlc1", "portal2_dlc2", "update"] {
        for f in ["closecaption_english.dat", "subtitles_english.dat"] {
            if let Ok(b) = std::fs::read(root.join(dir).join("resource").join(f)) {
                read_vccd(&b, &mut by_hash);
            }
        }
    }
    if by_hash.is_empty() {
        for f in ["resource/closecaption_english.dat", "resource/subtitles_english.dat"] {
            if let Some(b) = game.read(f) {
                read_vccd(&b, &mut by_hash);
            }
        }
    }
    info!("portal2: {} caption lines ({} compiled)", map.len() + by_hash.len(), by_hash.len());
    commands.insert_resource(CaptionText { by_name: map, by_hash });
}

fn queue_captions(
    mut shown: MessageReader<ShowCaption>,
    text: Option<Res<CaptionText>>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut lines: ResMut<CaptionLines>,
) {
    let now = time.elapsed_secs();
    let before = lines.0.len();
    lines.0.retain(|l| l.2 > now);
    let mut changed = lines.0.len() != before;
    for c in shown.read() {
        if !settings.subtitles {
            continue;
        }
        let Some((t, color)) = text.as_ref().and_then(|t| t.get(&c.sound)) else { continue };
        if t.is_empty() {
            continue;
        }
        lines.0.retain(|l| l.0 != t);
        lines.0.push((t, color, now + c.seconds.max(2.0) + 0.5));
        while lines.0.len() > 3 {
            lines.0.remove(0);
        }
        changed = true;
    }
    if changed {
        lines.set_changed();
    }
}

#[derive(Component)]
struct CaptionRoot;

fn draw_captions(mut commands: Commands, lines: Res<CaptionLines>, fonts: Option<Res<UiFonts>>, roots: Query<Entity, With<CaptionRoot>>) {
    if !lines.is_changed() {
        return;
    }
    let Some(fonts) = fonts else { return };
    for r in &roots {
        commands.entity(r).despawn();
    }
    if lines.0.is_empty() {
        return;
    }
    commands
        .spawn((
            CaptionRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Percent(8.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(6),
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    max_width: Val::Percent(60.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ))
            .with_children(|b| {
                for (t, c, _) in &lines.0 {
                    b.spawn((
                        Text::new(t.clone()),
                        TextFont { font: fonts.regular.clone().into(), font_size: bevy::text::FontSize::Px(20.0), ..default() },
                        TextColor(*c),
                        TextLayout::justify(Justify::Center),
                    ));
                }
            });
        });
}
