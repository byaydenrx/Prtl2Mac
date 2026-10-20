//! Captions: the game's own caption text (`resource/closecaption_<lang>.txt`
//! and `subtitles_<lang>.txt`, keyed by soundscript name) shown at the
//! bottom of the screen while a voice line plays, as Options > Audio >
//! Captioning says (subtitles: the dialogue files first; closed captions:
//! the closecaption files, which also describe sounds), in the language,
//! size, colour and background the player picked.
//!
//! TODO: closed captions for sound effects need audio.rs to send
//! [`ShowCaption`] for every sound it starts; only scene lines do today.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_assets::GameFsRes;

use crate::settings::{CaptionMode, Settings};
use crate::ui::UiFonts;

pub struct CaptionsPlugin;

impl Plugin for CaptionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptionLines>()
            .add_message::<ShowCaption>()
            .add_systems(Update, (load_captions, queue_captions, draw_captions).chain());
    }
}

/// A voice line started: show its caption for `seconds`.
#[derive(Message, Clone, Debug)]
pub struct ShowCaption {
    pub sound: String,
    pub seconds: f32,
}

/// One kind of caption file (subtitles or closecaption).
#[derive(Default)]
struct CaptionSet {
    /// From the text files, by lowercased token.
    by_name: HashMap<String, (String, Color)>,
    /// From the compiled `.dat` files, by CRC32 of the lowercased token.
    by_hash: HashMap<u32, (String, Color)>,
}

impl CaptionSet {
    fn get(&self, token: &str) -> Option<(String, Color)> {
        let t = token.to_ascii_lowercase();
        self.by_name.get(&t).or_else(|| self.by_hash.get(&crc32(t.as_bytes()))).cloned()
    }

    fn len(&self) -> usize {
        self.by_name.len() + self.by_hash.len()
    }
}

#[derive(Resource, Default)]
struct CaptionText {
    language: String,
    subtitles: CaptionSet,
    closecaption: CaptionSet,
}

impl CaptionText {
    /// The line for `token` in `mode`, from the mode's own files first.
    fn get(&self, token: &str, mode: CaptionMode) -> Option<(String, Color)> {
        match mode {
            CaptionMode::Off => None,
            CaptionMode::Subtitles => self.subtitles.get(token).or_else(|| self.closecaption.get(token)),
            CaptionMode::ClosedCaptions => self.closecaption.get(token).or_else(|| self.subtitles.get(token)),
        }
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

/// One caption set (`closecaption` or `subtitles`) in `lang`: the text file,
/// then every game folder's compiled file (the main campaign's, DLC1's,
/// DLC2's share names, so the search path would only show the first).
fn load_set(game: &GameFsRes, kind: &str, lang: &str) -> CaptionSet {
    let mut set = CaptionSet::default();
    if let Some(b) = game.read(&format!("resource/{kind}_{lang}.txt")) {
        let kv = source_assets::kv::parse(&decode(&b));
        if let Some(tokens) = kv.root().and_then(|(_, root)| root.block("tokens")) {
            for (k, v) in &tokens.entries {
                if let source_assets::kv::KvValue::Str(s) = v {
                    set.by_name.insert(k.to_ascii_lowercase(), clean(s));
                }
            }
        }
    }
    let file = format!("{kind}_{lang}.dat");
    let root = game.0.config.root.clone();
    for dir in ["portal2", "portal2_dlc1", "portal2_dlc2", "update"] {
        if let Ok(b) = std::fs::read(root.join(dir).join("resource").join(&file)) {
            read_vccd(&b, &mut set.by_hash);
        }
    }
    if set.by_hash.is_empty() {
        if let Some(b) = game.read(&format!("resource/{file}")) {
            read_vccd(&b, &mut set.by_hash);
        }
    }
    set
}

/// (Re)load the caption files when the language changes; a language the
/// install has no captions for falls back to English.
fn load_captions(mut commands: Commands, game: Res<GameFsRes>, settings: Res<Settings>, text: Option<Res<CaptionText>>) {
    if text.as_ref().is_some_and(|t| t.language == settings.language) {
        return;
    }
    let mut lang = settings.language.clone();
    let mut subtitles = load_set(&game, "subtitles", &lang);
    let mut closecaption = load_set(&game, "closecaption", &lang);
    if subtitles.len() + closecaption.len() == 0 && lang != "english" {
        warn!("portal2: no {lang} captions in the game files; using English");
        lang = "english".into();
        subtitles = load_set(&game, "subtitles", &lang);
        closecaption = load_set(&game, "closecaption", &lang);
    }
    info!("portal2: {} subtitle and {} caption lines ({lang})", subtitles.len(), closecaption.len());
    commands.insert_resource(CaptionText { language: settings.language.clone(), subtitles, closecaption });
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
        let Some((t, color)) = text.as_ref().and_then(|t| t.get(&c.sound, settings.captions)) else { continue };
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

/// HudCloseCaption's panel in hudlayout.res (480-unit layout).
const CC_YPOS: f32 = 276.0;
const CC_WIDE: f32 = 500.0;
const CC_TALL: f32 = 136.0;
/// Text inset from the box edge, and the box's rounded corner.
const CC_INSET: f32 = 12.0;
const CC_CORNER: f32 = 6.0;

fn units(u: f32) -> Val {
    Val::Vh(u * 100.0 / 480.0)
}

/// vgui blends in gamma space, Bevy in linear light: a black overlay of
/// alpha `a` darkens an sRGB colour by (1 - a) in vgui, which in linear
/// light is (1 - a)^2.2, so this is the linear alpha that looks the same.
fn linear_overlay_alpha(a: f32) -> f32 {
    1.0 - (1.0 - a.clamp(0.0, 1.0)).powf(2.2)
}

fn draw_captions(
    mut commands: Commands,
    lines: Res<CaptionLines>,
    settings: Res<Settings>,
    fonts: Option<Res<UiFonts>>,
    roots: Query<Entity, With<CaptionRoot>>,
) {
    if !lines.is_changed() && !settings.is_changed() {
        return;
    }
    let Some(fonts) = fonts else { return };
    for r in &roots {
        commands.entity(r).despawn();
    }
    if lines.0.is_empty() {
        return;
    }
    // hudlayout.res HudCloseCaption: xpos c-250, ypos 276, 500 x 136 units
    // of the 480-unit layout, BgAlpha 128; lines stack upward from the
    // panel's bottom (412) and the box only covers the text, CC_INSET
    // units in from its edges. Text is CloseCaption_Normal
    // (UniversLTStd-Cn, tall 20 = our MEDIUM size), left-aligned: the
    // install's caption files never use <center>.
    commands
        .spawn((
            CaptionRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: units(480.0 - (CC_YPOS + CC_TALL)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(6),
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: units(CC_WIDE),
                    max_height: units(CC_TALL),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    padding: UiRect::axes(units(CC_INSET), units(CC_INSET / 2.0)),
                    border_radius: BorderRadius::all(units(CC_CORNER)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, linear_overlay_alpha(settings.subtitle_background))),
            ))
            .with_children(|b| {
                let size = bevy::text::FontSize::Vh(settings.subtitle_size / 1.265 * 100.0 / 480.0);
                for (t, c, _) in &lines.0 {
                    b.spawn((
                        Text::new(t.clone()),
                        TextFont { font: fonts.menu.clone().into(), font_size: size, ..default() },
                        TextColor(settings.subtitle_color.color().unwrap_or(*c)),
                        TextLayout::justify(Justify::Left),
                    ));
                }
            });
        });
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn caption_box_matches_hudlayout() {
        // HudCloseCaption: centred 500-unit panel whose bottom is 68 units
        // above the screen's bottom edge.
        assert_eq!(480.0 - (CC_YPOS + CC_TALL), 68.0);
        // BgAlpha 128 blended in gamma space looks like ~0.78 linear alpha.
        let a = linear_overlay_alpha(128.0 / 255.0);
        assert!((a - 0.78).abs() < 0.01, "{a}");
        assert_eq!(linear_overlay_alpha(0.0), 0.0);
        assert_eq!(linear_overlay_alpha(1.0), 1.0);
    }
}
