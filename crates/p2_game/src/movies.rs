//! Movie screens (`vgui_movie_display`): the elevator panels and signs that
//! play the game's Bink movies (`media/*.bik`). A group of screens
//! (`groupname`) shows its master's movie; the elevator script
//! (`videos/video_splitter.nut`) picks the movie per map (`SetMovie`), gives
//! each panel its slice of the picture (`SetUseCustomUVs`, `SetUMin`..
//! `SetVMax`) and turns them on (`Enable`).
//!
//! A screen is a quad `width` x `height` units facing the entity's forward,
//! with its bottom-left corner (as seen from the front) at the entity's
//! origin: it spans the entity's left axis (the viewer's right) and up. Movies are decoded with ffmpeg (which reads Bink) in the
//! background, at reduced size, and cached under the app's data folder;
//! without ffmpeg the screens stay dark.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use p2_assets::GameFsRes;
use shared::source_to_world;

use crate::io::{EntInput, Ents};
use crate::level::{GameSet, LevelLoaded, LevelOwned};

pub struct MoviesPlugin;

impl Plugin for MoviesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Movies>()
            .add_systems(Update, (spawn_screens, movie_inputs, play_movies).chain().after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

/// Decoded width (height follows the movie's aspect).
const DECODE_WIDTH: u32 = 192;
/// Decoded frame rate.
const DECODE_FPS: u32 = 12;

struct Screen {
    ent: usize,
    quad: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    enabled: bool,
    custom_uv: bool,
    uv: [f32; 4],
    /// Corners in world space: bottom-left, bottom-right, top-right, top-left.
    corners: [Vec3; 4],
    shown_uv: Option<[f32; 4]>,
}

/// Per master: its movie and when it started.
#[derive(Default, Clone)]
struct MasterState {
    movie: Option<String>,
    started: f32,
    looping: bool,
}

struct Decoded {
    w: u32,
    h: u32,
    fps: f32,
    frames: Vec<u8>,
}

impl Decoded {
    fn count(&self) -> usize {
        let n = (self.w * self.h * 4) as usize;
        if n == 0 { 0 } else { self.frames.len() / n }
    }
}

struct Clip {
    image: Handle<Image>,
    decoded: Arc<Mutex<Option<Option<Decoded>>>>,
    shown: Option<usize>,
    sized: bool,
}

#[derive(Resource, Default)]
pub struct Movies {
    screens: Vec<Screen>,
    /// Screen entity -> its group's master entity.
    master_of: HashMap<usize, usize>,
    masters: HashMap<usize, MasterState>,
    clips: HashMap<String, Clip>,
}

/// Source `AngleVectors`: forward, right, up of (pitch, yaw, roll) degrees.
pub fn angle_vectors(a: Vec3) -> (Vec3, Vec3, Vec3) {
    let (sp, cp) = a.x.to_radians().sin_cos();
    let (sy, cy) = a.y.to_radians().sin_cos();
    let (sr, cr) = a.z.to_radians().sin_cos();
    let forward = Vec3::new(cp * cy, cp * sy, -sp);
    let right = Vec3::new(-sr * sp * cy + cr * sy, -sr * sp * sy - cr * cy, -sr * cp);
    let up = Vec3::new(cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp);
    (forward, right, up)
}

/// Source-space corners of a screen as seen from the front: bottom-left (the
/// origin), bottom-right, top-right, top-left. The screen faces the entity's
/// forward, so the viewer's right is the entity's left (-right). The
/// elevator rings prove it: a ring's missing panels leave the exit gap only
/// this way round (sp_a2_laser_stairs: `@arrival_sign_1` and `_24` would
/// otherwise stand across the exit and its leaving trigger, T-129).
pub fn screen_corners(origin: Vec3, angles: Vec3, width: f32, height: f32) -> [Vec3; 4] {
    let (_, right, up) = angle_vectors(angles);
    let right = -right;
    [origin, origin + right * width, origin + right * width + up * height, origin + up * height]
}

/// Normalises a movie path the way the maps and scripts write it.
fn movie_path(p: &str) -> Option<String> {
    let p = p.trim().replace('\\', "/").to_ascii_lowercase();
    p.ends_with(".bik").then_some(p)
}

fn kv_vec3(s: Option<&str>) -> Vec3 {
    let v: Vec<f32> = s.unwrap_or("").split_whitespace().filter_map(|x| x.parse().ok()).collect();
    Vec3::new(v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0), v.get(2).copied().unwrap_or(0.0))
}

fn quad_mesh(corners: &[Vec3; 4], uv: [f32; 4]) -> Mesh {
    let [u0, u1, v0, v1] = uv;
    let n = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or_zero();
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners.iter().map(|c| c.to_array()).collect::<Vec<_>>());
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![n.to_array(); 4]);
    // v runs down the picture: the top edge is vmin.
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]]);
    m.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    m
}

fn spawn_screens(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    mut movies: ResMut<Movies>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    movies.screens.clear();
    movies.master_of.clear();
    movies.masters.clear();
    let Some(ents) = ents else { return };
    // Group masters: the screen in each group that isn't a forced slave.
    let mut group_master: HashMap<String, usize> = HashMap::new();
    for (i, e) in ents.list.iter().enumerate() {
        if e.class == "vgui_movie_display" && e.get("forcedslave") != Some("1") {
            if let Some(g) = e.get("groupname").filter(|g| !g.is_empty()) {
                group_master.entry(g.to_ascii_lowercase()).or_insert(i);
            }
        }
    }
    for (i, e) in ents.list.iter().enumerate() {
        if e.class != "vgui_movie_display" {
            continue;
        }
        let master = e.get("groupname").filter(|g| !g.is_empty()).and_then(|g| group_master.get(&g.to_ascii_lowercase()).copied()).unwrap_or(i);
        movies.master_of.insert(i, master);
        if master == i {
            movies.masters.insert(
                i,
                MasterState { movie: e.get("moviefilename").and_then(movie_path), started: 0.0, looping: e.get("looping").unwrap_or("1") != "0" },
            );
        }
        let w = e.get_f32("width").unwrap_or(64.0);
        let h = e.get_f32("height").unwrap_or(64.0);
        // The elevator's screen panels (elevator/elevator_screen brushes)
        // stand a few units in front of the displays' origins, along the
        // shaft's curve: bring the picture out in front of them.
        let angles = kv_vec3(e.get("angles"));
        let lift = angle_vectors(angles).0 * 8.0;
        let src = screen_corners(kv_vec3(e.get("origin")) + lift, angles, w, h);
        let corners = src.map(source_to_world);
        let uv = [0.0, 1.0, 0.0, 1.0];
        let mesh = meshes.add(quad_mesh(&corners, uv));
        let material = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: None,
            double_sided: true,
            ..default()
        });
        let quad = commands
            .spawn((Name::new(format!("movie screen {}", e.name)), Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::IDENTITY, Visibility::Hidden, LevelOwned))
            .id();
        movies.screens.push(Screen { ent: i, quad, mesh, material, enabled: false, custom_uv: false, uv, corners, shown_uv: Some(uv) });
    }
    if !movies.screens.is_empty() {
        info!("portal2: {} movie screens", movies.screens.len());
    }
}

fn movie_inputs(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut movies: ResMut<Movies>) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let now = ents.now;
    for msg in inputs.read() {
        if ents.list.get(msg.target).is_none_or(|e| e.class != "vgui_movie_display") {
            continue;
        }
        debug!("movie: {} {} {:?}", ents.list[msg.target].name, msg.input, msg.param);
        let f = || source_assets::kv::parse_f32(&msg.param);
        let master = movies.master_of.get(&msg.target).copied().unwrap_or(msg.target);
        match msg.input.as_str() {
            "setmovie" => {
                let st = movies.masters.entry(msg.target).or_default();
                st.movie = movie_path(&msg.param);
                st.started = now;
            }
            "takeoverasmaster" => {
                let group: Vec<usize> = movies.master_of.iter().filter(|(_, m)| **m == master).map(|(s, _)| *s).collect();
                for s in group {
                    movies.master_of.insert(s, msg.target);
                }
                let st = movies.masters.get(&master).cloned().unwrap_or_default();
                movies.masters.insert(msg.target, st);
            }
            input => {
                for s in movies.screens.iter_mut().filter(|s| s.ent == msg.target) {
                    match input {
                        "enable" => {
                            s.enabled = true;
                        }
                        "disable" => s.enabled = false,
                        "setusecustomuvs" => s.custom_uv = msg.param.trim() != "0",
                        "setumin" => s.uv[0] = f().unwrap_or(s.uv[0]),
                        "setumax" => s.uv[1] = f().unwrap_or(s.uv[1]),
                        "setvmin" => s.uv[2] = f().unwrap_or(s.uv[2]),
                        "setvmax" => s.uv[3] = f().unwrap_or(s.uv[3]),
                        _ => {}
                    }
                }
                // A master starts its movie when it is turned on.
                if input == "enable" && msg.target == master {
                    if let Some(st) = movies.masters.get_mut(&master) {
                        st.started = now;
                    }
                }
            }
        }
    }
}

fn ffmpeg() -> Option<String> {
    ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"].iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string())
}

/// Decodes a movie (or reads it from the cache): RGBA frames, top row first.
fn decode(game: &GameFsRes, path: &str) -> Option<Decoded> {
    let dir = crate::settings::data_dir().join("movies");
    let stem = path.rsplit('/').next()?.trim_end_matches(".bik").to_string();
    let cache = dir.join(format!("{stem}_{DECODE_WIDTH}_{DECODE_FPS}.rgba"));
    if let Ok(d) = std::fs::read(&cache) {
        if d.len() >= 12 && &d[0..4] == b"P2MV" {
            let w = u32::from_le_bytes(d[4..8].try_into().ok()?);
            let h = u32::from_le_bytes(d[8..12].try_into().ok()?);
            return Some(Decoded { w, h, fps: DECODE_FPS as f32, frames: d[12..].to_vec() });
        }
    }
    let ff = ffmpeg()?;
    // Movies are loose files in the game folders (newest first), not in VPKs.
    let root = &game.0.config.root;
    let loose = ["update", "portal2_dlc2", "portal2_dlc1", "portal2"].iter().map(|d| root.join(d).join(path)).find(|p| p.exists());
    let (src, temp) = match loose {
        Some(p) => (p, false),
        None => {
            let bytes = game.read(path)?;
            std::fs::create_dir_all(&dir).ok()?;
            let p = dir.join(format!("{stem}.bik"));
            std::fs::write(&p, &bytes).ok()?;
            (p, true)
        }
    };
    std::fs::create_dir_all(&dir).ok()?;
    // ffmpeg prints the stream's size; the output keeps its aspect.
    let info = std::process::Command::new(&ff).args(["-hide_banner", "-i"]).arg(&src).output().ok()?;
    let text = String::from_utf8_lossy(&info.stderr);
    let (sw, sh) = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|t| t.split_once('x'))
        .filter_map(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
        .find(|(a, b)| *a > 16 && *b > 16)?;
    let w = DECODE_WIDTH;
    let h = ((w as f32 * sh as f32 / sw as f32 / 2.0).round() as u32 * 2).max(2);
    let mut child = std::process::Command::new(&ff)
        .args(["-v", "error", "-i"])
        .arg(&src)
        .args(["-vf", &format!("scale={w}:{h},fps={DECODE_FPS}"), "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut frames = Vec::new();
    child.stdout.take()?.read_to_end(&mut frames).ok()?;
    let _ = child.wait();
    if temp {
        let _ = std::fs::remove_file(&src);
    }
    if frames.len() < (w * h * 4) as usize {
        return None;
    }
    if let Ok(mut f) = std::fs::File::create(&cache) {
        let _ = f.write_all(b"P2MV");
        let _ = f.write_all(&w.to_le_bytes());
        let _ = f.write_all(&h.to_le_bytes());
        let _ = f.write_all(&frames);
    }
    Some(Decoded { w, h, fps: DECODE_FPS as f32, frames })
}

/// Which frame shows `t` seconds into a movie of `n` frames.
pub fn frame_at(t: f32, fps: f32, n: usize, looping: bool) -> usize {
    if n == 0 {
        return 0;
    }
    let f = (t.max(0.0) * fps) as usize;
    if looping { f % n } else { f.min(n - 1) }
}

#[allow(clippy::too_many_arguments)]
fn play_movies(
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut movies: ResMut<Movies>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(ents) = ents else { return };
    let now = ents.now;
    let movies = &mut *movies;
    // Start decoding every movie a master has.
    for st in movies.masters.values() {
        let Some(path) = &st.movie else { continue };
        if movies.clips.contains_key(path) {
            continue;
        }
        let image = images.add(Image::new_fill(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
        let decoded: Arc<Mutex<Option<Option<Decoded>>>> = Arc::new(Mutex::new(None));
        let (slot, game2, path2) = (decoded.clone(), game.clone(), path.clone());
        std::thread::spawn(move || {
            let d = decode(&game2, &path2);
            if d.is_none() {
                warn!("portal2: movie {path2} could not be decoded (needs ffmpeg)");
            }
            *slot.lock().unwrap() = Some(d);
        });
        movies.clips.insert(path.clone(), Clip { image, decoded, shown: None, sized: false });
    }
    // Advance each movie to its frame.
    let mut ready: HashMap<String, bool> = HashMap::new();
    for (path, clip) in movies.clips.iter_mut() {
        let guard = clip.decoded.lock().unwrap();
        let Some(Some(d)) = guard.as_ref() else { continue };
        ready.insert(path.clone(), true);
        let Some(st) = movies.masters.values().find(|s| s.movie.as_deref() == Some(path)) else { continue };
        let k = frame_at(now - st.started, d.fps, d.count(), st.looping);
        if clip.shown == Some(k) && clip.sized {
            continue;
        }
        let n = (d.w * d.h * 4) as usize;
        let Some(bytes) = d.frames.get(k * n..(k + 1) * n) else { continue };
        if !clip.sized {
            let img = Image::new(Extent3d { width: d.w, height: d.h, depth_or_array_layers: 1 }, TextureDimension::D2, bytes.to_vec(), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
            let _ = images.insert(&clip.image, img);
            clip.sized = true;
        } else if let Some(mut img) = images.get_mut(&clip.image) {
            if let Some(data) = img.data.as_mut() {
                data.copy_from_slice(bytes);
            }
        }
        clip.shown = Some(k);
    }
    // Show each screen with its master's movie, while the master is on.
    let enabled: std::collections::HashSet<usize> = movies.screens.iter().filter(|s| s.enabled).map(|s| s.ent).collect();
    for s in movies.screens.iter_mut() {
        let master = movies.master_of.get(&s.ent).copied().unwrap_or(s.ent);
        let mst = movies.masters.get(&master);
        let killed = ents.list[s.ent].killed;
        let master_on = enabled.contains(&master);
        let movie = mst.and_then(|m| m.movie.clone()).filter(|p| ready.contains_key(p));
        let show = !killed && s.enabled && master_on && movie.is_some();
        if let Ok(mut v) = vis.get_mut(s.quad) {
            let want = if show { Visibility::Inherited } else { Visibility::Hidden };
            if *v != want {
                *v = want;
            }
        }
        if !show {
            continue;
        }
        let uv = if s.custom_uv { s.uv } else { [0.0, 1.0, 0.0, 1.0] };
        if s.shown_uv != Some(uv) {
            if let Some(mut m) = meshes.get_mut(&s.mesh) {
                *m = quad_mesh(&s.corners, uv);
            }
            s.shown_uv = Some(uv);
        }
        if let (Some(path), Some(mut mat)) = (movie, materials.get_mut(&s.material)) {
            let img = movies.clips.get(&path).map(|c| c.image.clone());
            if mat.base_color_texture != img {
                mat.base_color_texture = img;
                mat.base_color = Color::WHITE;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_spans_right_and_up_from_its_origin() {
        // Yaw 0 faces +X: a viewer looking along -X has +Y on their right.
        let c = screen_corners(Vec3::ZERO, Vec3::ZERO, 64.0, 128.0);
        assert!((c[1] - Vec3::new(0.0, 64.0, 0.0)).length() < 1e-3);
        assert!((c[3] - Vec3::new(0.0, 0.0, 128.0)).length() < 1e-3);
        // Yaw 90 faces +Y: across -X.
        let c = screen_corners(Vec3::ZERO, Vec3::new(0.0, 90.0, 0.0), 10.0, 10.0);
        assert!((c[1] - Vec3::new(-10.0, 0.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn laser_stairs_ring_leaves_the_exit_clear() {
        // sp_a2_laser_stairs: @arrival_sign_1 spans to @arrival_sign_2's
        // origin, clear of the exit along y = -352 (T-129).
        let c = screen_corners(Vec3::new(-1344.74, -383.901, -96.0), Vec3::new(0.0, 165.0, 0.0), 64.0, 128.0);
        assert!((c[1] - Vec3::new(-1360.85, -444.001, -96.0)).length() < 2.5);
        assert!(c[0].y < -352.0 && c[1].y < -352.0);
    }

    #[test]
    fn frames_loop_or_hold() {
        assert_eq!(frame_at(0.0, 12.0, 10, true), 0);
        assert_eq!(frame_at(1.0, 12.0, 10, true), 2);
        assert_eq!(frame_at(5.0, 12.0, 10, false), 9);
    }

    #[test]
    fn movie_paths_normalise() {
        assert_eq!(movie_path("media\\Exercises_Horiz.bik").as_deref(), Some("media/exercises_horiz.bik"));
        assert_eq!(movie_path("media/"), None);
    }
}
