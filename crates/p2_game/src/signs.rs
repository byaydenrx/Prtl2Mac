//! Chamber progress signs (`vgui_screen` with panelname `sp_progress_sign`):
//! the white board by each chamber's entrance showing the chamber number,
//! the chapter's progress and the test's hazard icons. The per-map data is
//! `scripts/vscripts/transitions/sp_lightboard_icons.txt` (level_number,
//! total_levels, and ten icons with "1" for the ones this test uses); the
//! art is `vgui/screens/vgui_coop_progress_board` (the board with its three
//! rules), `_numbers` (digit sheet, 6 x 2 cells) and `P2_lightboard_vgui`
//! (icon sheet, 5 x 5 cells). The board is composed once per map on the CPU
//! and drawn as an unlit quad; the prefab turns it on with `SetActive` when
//! the player walks in and off with `SetInactive`.
//!
//! The icon sheet's order is not the order the file lists the names in; the
//! cells below were matched to the names by their pictures.
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use p2_assets::GameFsRes;
use shared::source_to_world;

use crate::io::{EntInput, Ents};
use crate::level::{GameSet, LevelLoaded, LevelOwned};
use crate::movies::{angle_vectors, screen_corners};

pub struct SignsPlugin;

impl Plugin for SignsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Signs>().add_systems(Update, (spawn_signs, sign_inputs).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

#[derive(Resource, Default)]
struct Signs {
    /// (entity index, quad).
    quads: Vec<(usize, Entity)>,
}

/// Icon names in the sheet's cell order (row-major, 5 per row).
const ICON_CELLS: [&str; 23] = [
    "cube_drop", "cube_button", "cube_bonk", "drink_water", "goop",
    "crushers", "laser_cube", "turret", "turret_burn", "portal_fling",
    "plate_fling", "bridges", "bridge_block", "grinders", "tbeams",
    "paint_bounce", "paint_speed", "handoff", "button_stand", "laser_power",
    "portal_fling_2", "tbeam_polarity", "danger_field",
];

/// One map's sign: chamber number, chapter length and (icon, lit) in the
/// file's order.
#[derive(Debug, Clone, PartialEq)]
pub struct SignInfo {
    pub level: u32,
    pub total: u32,
    pub icons: Vec<(String, bool)>,
}

pub fn sign_info(icons_file: &str, map: &str) -> Option<SignInfo> {
    let kv = source_assets::kv::parse(icons_file);
    let (_, root) = kv.root()?;
    let m = root.block("maps")?.block(&map.to_ascii_lowercase())?;
    let num = |k: &str| m.get(k).and_then(|s| s.trim().parse().ok());
    let icons = m
        .block("icons")
        .map(|b| {
            b.entries
                .iter()
                .filter_map(|(k, v)| match v {
                    source_assets::kv::KvValue::Str(s) => Some((k.clone(), s.trim() == "1")),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    Some(SignInfo { level: num("level_number")?, total: num("total_levels").unwrap_or(19), icons })
}

/// An RGBA8 picture.
pub struct Pic {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Pic {
    fn alpha(&self, x: f32, y: f32) -> f32 {
        let xi = (x as i64).clamp(0, self.w as i64 - 1) as u32;
        let yi = (y as i64).clamp(0, self.h as i64 - 1) as u32;
        self.px[((yi * self.w + xi) * 4 + 3) as usize] as f32 / 255.0
    }
}

/// Paints `src`'s alpha (a mask) from source rect `s` into rect `d` of
/// `dst` in `color` at `opacity`.
fn stamp(dst: &mut Pic, src: &Pic, s: [f32; 4], d: [f32; 4], color: [u8; 3], opacity: f32) {
    let (x0, y0) = (d[0].max(0.0) as u32, d[1].max(0.0) as u32);
    let (x1, y1) = (((d[0] + d[2]) as u32).min(dst.w), ((d[1] + d[3]) as u32).min(dst.h));
    for y in y0..y1 {
        for x in x0..x1 {
            let u = s[0] + (x as f32 + 0.5 - d[0]) / d[2] * s[2];
            let v = s[1] + (y as f32 + 0.5 - d[1]) / d[3] * s[3];
            let a = src.alpha(u, v) * opacity;
            let i = ((y * dst.w + x) * 4) as usize;
            for c in 0..3 {
                let p = dst.px[i + c] as f32;
                dst.px[i + c] = (p + (color[c] as f32 - p) * a).round() as u8;
            }
        }
    }
}

const INK: [u8; 3] = [32, 32, 34];
/// Opacity of an icon the test doesn't use, and of the bar's unplayed part.
const DIM: f32 = 0.16;

/// Draws the sign on a copy of the board (512 x 1024): the two-digit
/// chamber number between the top rules, "NN/TT" and the progress bar in the
/// middle band, the icons in two rows of five under the third rule.
pub fn compose(board: &Pic, digits: &Pic, icons: &Pic, info: &SignInfo) -> Pic {
    let mut out = Pic { w: board.w, h: board.h, px: board.px.clone() };
    let sx = board.w as f32 / 512.0;
    let sy = board.h as f32 / 1024.0;
    let (dw, dh) = (digits.w as f32 / 6.0, digits.h as f32 / 2.0);
    let digit_src = |n: u32| [(n % 6) as f32 * dw, (n / 6) as f32 * dh, dw, dh];
    let left = 102.0;
    let width = 395.0;
    // Chamber number.
    let n = info.level.min(99);
    let (h, w) = (440.0, 440.0 * dw / dh);
    for (k, dig) in [n / 10, n % 10].into_iter().enumerate() {
        stamp(&mut out, digits, digit_src(dig), [(left + k as f32 * (w + 6.0)) * sx, 112.0 * sy, w * sx, h * sy], INK, 1.0);
    }
    // "NN/TT".
    let (h, w) = (46.0, 46.0 * dw / dh);
    let mut x = left;
    let t = info.total.min(99);
    for (k, dig) in [n / 10, n % 10, 10, t / 10, t % 10].into_iter().enumerate() {
        if dig == 10 {
            // The slash.
            for i in 0..(h as u32) {
                let px = x + 4.0 + (h - i as f32) * 0.25;
                let py = 592.0 + i as f32;
                for o in 0..3 {
                    let (ix, iy) = (((px + o as f32) * sx) as u32, (py * sy) as u32);
                    if ix < out.w && iy < out.h {
                        let j = ((iy * out.w + ix) * 4) as usize;
                        out.px[j..j + 3].copy_from_slice(&INK);
                    }
                }
            }
            x += w + 4.0;
            continue;
        }
        stamp(&mut out, digits, digit_src(dig), [x * sx, 592.0 * sy, w * sx, h * sy], INK, 1.0);
        x += w + if k == 1 || k == 4 { 4.0 } else { 2.0 };
    }
    // Progress bar: one segment per chamber, the played ones dark.
    let total = info.total.max(1);
    let gap = 3.0;
    let seg = (width - gap * (total - 1) as f32) / total as f32;
    for i in 0..total {
        let x = left + i as f32 * (seg + gap);
        let o = if i < info.level { 1.0 } else { DIM };
        for y in (658.0 * sy) as u32..(690.0 * sy) as u32 {
            for xx in (x * sx) as u32..((x + seg) * sx) as u32 {
                let j = ((y * out.w + xx.min(out.w - 1)) * 4) as usize;
                for c in 0..3 {
                    let p = out.px[j + c] as f32;
                    out.px[j + c] = (p + (INK[c] as f32 - p) * o).round() as u8;
                }
            }
        }
    }
    // Icons.
    let (cw, ch) = (icons.w as f32 / 5.0, icons.h as f32 / 5.0);
    let size = 72.0;
    let step = (width - size) / 4.0;
    for (k, (name, lit)) in info.icons.iter().take(10).enumerate() {
        let Some(cell) = ICON_CELLS.iter().position(|c| c == name) else { continue };
        let src = [(cell % 5) as f32 * cw, (cell / 5) as f32 * ch, cw, ch];
        let dst = [(left + (k % 5) as f32 * step) * sx, (736.0 + (k / 5) as f32 * (size + 10.0)) * sy, size * sx, size * sy];
        stamp(&mut out, icons, src, dst, INK, if *lit { 1.0 } else { DIM });
    }
    out
}

fn load_pic(game: &GameFsRes, path: &str) -> Option<Pic> {
    let vtf = source_assets::vtf::VtfTexture::parse(&game.read(&format!("materials/{path}.vtf"))?).ok()?;
    let px = vtf.rgba8_level0()?;
    Some(Pic { w: vtf.width, h: vtf.height, px })
}

fn spawn_signs(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    game: Option<Res<GameFsRes>>,
    mut signs: ResMut<Signs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(ev) = loaded.read().last() else { return };
    signs.quads.clear();
    let (Some(ents), Some(game)) = (ents, game) else { return };
    let panels: Vec<usize> = ents.list.iter().enumerate().filter(|(_, e)| e.class == "vgui_screen" && e.get("panelname").is_some_and(|p| p.eq_ignore_ascii_case("sp_progress_sign"))).map(|(i, _)| i).collect();
    if panels.is_empty() {
        return;
    }
    let text = game.read("scripts/vscripts/transitions/sp_lightboard_icons.txt").map(|b| String::from_utf8_lossy(&b).into_owned());
    let Some(info) = text.and_then(|t| sign_info(&t, &ev.map)) else {
        info!("portal2: no lightboard entry for {}", ev.map);
        return;
    };
    let (Some(board), Some(digits), Some(icons)) = (
        load_pic(&game, "vgui/screens/vgui_coop_progress_board"),
        load_pic(&game, "vgui/screens/vgui_coop_progress_board_numbers"),
        load_pic(&game, "vgui/screens/p2_lightboard_vgui"),
    ) else {
        return;
    };
    let pic = compose(&board, &digits, &icons, &info);
    let image = images.add(Image::new(Extent3d { width: pic.w, height: pic.h, depth_or_array_layers: 1 }, TextureDimension::D2, pic.px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
    let material = materials.add(StandardMaterial { base_color_texture: Some(image), unlit: true, cull_mode: None, ..default() });
    for i in panels {
        let e = &ents.list[i];
        let angles = crate::movies::kv_vec3(e.get("angles"));
        let lift = angle_vectors(angles).0 * 0.25;
        let corners = screen_corners(crate::movies::kv_vec3(e.get("origin")) + lift, angles, e.get_f32("width").unwrap_or(94.0), e.get_f32("height").unwrap_or(190.0)).map(source_to_world);
        let mesh = meshes.add(crate::movies::quad_mesh(&corners, [0.0, 1.0, 0.0, 1.0]));
        let vis = if e.get("spawnflags").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) & 1 != 0 { Visibility::Inherited } else { Visibility::Hidden };
        let quad = commands.spawn((Name::new(format!("progress sign {}", e.name)), Mesh3d(mesh), MeshMaterial3d(material.clone()), Transform::IDENTITY, vis, LevelOwned)).id();
        signs.quads.push((i, quad));
    }
    info!("portal2: progress sign {:02}/{} with {} icons", info.level, info.total, info.icons.iter().filter(|i| i.1).count());
}

fn sign_inputs(mut inputs: MessageReader<EntInput>, signs: Res<Signs>, mut vis: Query<&mut Visibility>) {
    for msg in inputs.read() {
        let Some(&(_, quad)) = signs.quads.iter().find(|(i, _)| *i == msg.target) else { continue };
        let Ok(mut v) = vis.get_mut(quad) else { continue };
        match msg.input.as_str() {
            "setactive" => *v = Visibility::Inherited,
            "setinactive" => *v = Visibility::Hidden,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#""sp_lightboard_icons" { "Maps" { "sp_a1_intro2" { "level_number" "1" "total_levels" "19"
        "icons" { "cube_drop" "0" "cube_button" "1" "cube_bonk" "0" } } } }"#;

    #[test]
    fn reads_a_maps_sign() {
        let s = sign_info(FILE, "SP_A1_Intro2").unwrap();
        assert_eq!((s.level, s.total), (1, 19));
        assert_eq!(s.icons, vec![("cube_drop".into(), false), ("cube_button".into(), true), ("cube_bonk".into(), false)]);
        assert!(sign_info(FILE, "sp_a1_intro3").is_none());
        assert!(s.icons.iter().all(|(n, _)| ICON_CELLS.contains(&n.as_str())));
    }

    #[test]
    fn lit_icons_print_darker_than_unused_ones() {
        let white = |w, h| Pic { w, h, px: vec![255; (w * h * 4) as usize] };
        let s = sign_info(FILE, "sp_a1_intro2").unwrap();
        let out = compose(&white(512, 1024), &white(60, 20), &white(50, 50), &s);
        let at = |x: u32, y: u32| out.px[((y * 512 + x) * 4) as usize];
        assert!(at(110, 300) < 60, "number drawn");
        assert!(at(120, 770) > 180, "first icon (unused) faint");
        let step = (395.0 - 72.0) / 4.0;
        assert!(at((102.0 + step + 20.0) as u32, 770) < 60, "second icon (used) dark");
        assert!(at(110, 670) < 60, "level 1: the first segment is played");
        assert!(at(490, 670) > 180, "the rest are not");
    }
}
