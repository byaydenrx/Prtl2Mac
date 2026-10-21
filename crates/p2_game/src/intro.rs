//! The startup sequence of a normal launch (no map argument): the intro
//! movies listed in `media/startupvids.txt` (Portal 2's is `media/valve.bik`,
//! the Valve logo with its own audio), a fade, the startup loading screen
//! while the menu's background map loads, then the main menu fading in from
//! the loading screen (see `loading.rs`). Any key, mouse click or Esc skips
//! the movie being played. If a movie can't be decoded the whole sequence is
//! dropped and the menu opens at once. `P2_NO_INTRO=1` (or a map argument,
//! or a scripted `--shot` run) turns it off.
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use p2_assets::GameFsRes;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Mutex;

use crate::level::{GameSet, LevelLoaded, LoadLevel};
use crate::ui::Screen;

pub struct IntroPlugin {
    pub enabled: bool,
}

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        let enabled = self.enabled && !std::env::var("P2_NO_INTRO").is_ok_and(|v| !v.is_empty() && v != "0");
        app.insert_resource(Intro { phase: if enabled { Phase::Movie } else { Phase::Done }, ..default() })
            .init_resource::<StartupLoad>()
            .add_systems(Startup, start_intro.after(crate::campaign::load_campaign))
            .add_systems(Update, run_intro.in_set(GameSet::Logic));
    }
}

/// True from the moment the startup loading screen is wanted until it has
/// faded out; `level.rs` then holds the (menu) load behind the loading
/// screen and `loading.rs` shows it and fades it out.
#[derive(Resource, Default)]
pub struct StartupLoad(pub bool);

/// Movie frame size (the intro movies are 1280x720).
const MOVIE_SIZE: UVec2 = UVec2::new(1280, 720);
/// Frame rate the movies are played at (the Bink files are 23.98).
const MOVIE_FPS: u32 = 24;
/// Fade to black after the last movie (seconds), and after a skip.
const FADE_END: f32 = 0.4;
const FADE_SKIP: f32 = 0.15;
/// A movie that shows no frame in this long is given up on.
const FIRST_FRAME_TIMEOUT: f32 = 6.0;
/// The movie list when `startupvids.txt` is missing.
const DEFAULT_PLAYLIST: &str = "media/valve.bik";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Phase {
    Movie,
    Fade,
    Loading,
    #[default]
    Done,
}

#[derive(Resource, Default)]
pub struct Intro {
    phase: Phase,
    playlist: Vec<String>,
    next: usize,
    movie: Option<Playing>,
    audio: Option<Entity>,
    image: Handle<Image>,
    root: Option<Entity>,
    picture: Option<Entity>,
    /// UI needs a camera and no level (so no player camera) is loaded yet.
    camera: Option<Entity>,
    fade_start: f32,
    fade_len: f32,
    load_frames: u32,
}

impl Intro {
    /// The sequence has not finished: the main menu is not opened yet.
    pub fn active(&self) -> bool {
        self.phase != Phase::Done
    }
}

/// One ffmpeg decoding a movie in real time; only the newest frame is kept.
struct Playing {
    path: String,
    child: Child,
    frames: Mutex<Receiver<Vec<u8>>>,
    /// Index of the next frame to take from `frames`.
    index: u32,
    /// When frame 0 arrived (the clock the frames are paced by).
    t0: Option<f32>,
    ended: bool,
    started: f32,
    audio: Option<Handle<bevy::audio::AudioSource>>,
    got_frame: bool,
}

impl Drop for Playing {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The movies in a `startupvids.txt`: one path per line, blank lines and
/// `//` comments ignored, quotes allowed.
pub fn parse_playlist(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split("//").next().unwrap_or("").trim().trim_matches('"').trim().replace('\\', "/"))
        .filter(|l| !l.is_empty())
        .collect()
}

fn playlist(game: &GameFsRes) -> Vec<String> {
    let text = crate::movies::loose_file(game, "media/startupvids.txt")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .or_else(|| game.read("media/startupvids.txt").map(|b| String::from_utf8_lossy(&b).into_owned()));
    let list = text.map(|t| parse_playlist(&t)).unwrap_or_default();
    if list.is_empty() {
        vec![DEFAULT_PLAYLIST.to_string()]
    } else {
        list
    }
}

/// The movie's audio as a wav (cached next to the decoded movies).
fn extract_audio(ff: &str, src: &std::path::Path, stem: &str) -> Option<Vec<u8>> {
    let dir = crate::settings::data_dir().join("movies");
    std::fs::create_dir_all(&dir).ok()?;
    let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let wav = dir.join(format!("{stem}_{size}_audio.wav"));
    if let Ok(d) = std::fs::read(&wav) {
        if d.len() > 44 {
            return Some(d);
        }
    }
    let ok = Command::new(ff)
        .args(["-v", "error", "-y", "-i"])
        .arg(src)
        .args(["-vn", "-ac", "2", "-ar", "44100", "-f", "wav"])
        .arg(&wav)
        .stdin(Stdio::null())
        .status()
        .ok()?
        .success();
    let d = std::fs::read(&wav).ok()?;
    (ok && d.len() > 44).then_some(d)
}

/// ffmpeg decoding the movie as fast as the (small) frame queue lets it; the
/// frames are paced against the clock in `run_intro`, not by ffmpeg's `-re`
/// (which duplicated frames and ran long when the game was busy).
fn spawn_ffmpeg(ff: &str, src: &std::path::Path) -> Option<(Child, Receiver<Vec<u8>>)> {
    let mut child = Command::new(ff)
        .args(["-v", "error", "-i"])
        .arg(src)
        .args(["-an", "-vf", &format!("scale={}:{},fps={MOVIE_FPS}", MOVIE_SIZE.x, MOVIE_SIZE.y), "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(6);
    std::thread::spawn(move || loop {
        let mut frame = vec![0u8; (MOVIE_SIZE.x * MOVIE_SIZE.y * 4) as usize];
        if out.read_exact(&mut frame).is_err() || tx.send(frame).is_err() {
            break;
        }
    });
    Some((child, rx))
}

/// Starts movie `intro.next` (the next one that decodes); None when the
/// list is used up or the first one fails.
fn start_movie(intro: &mut Intro, game: &GameFsRes, audio: &mut Assets<bevy::audio::AudioSource>, now: f32) -> bool {
    while intro.next < intro.playlist.len() {
        let path = intro.playlist[intro.next].clone();
        intro.next += 1;
        let Some(ff) = crate::movies::ffmpeg() else {
            warn!("portal2: intro skipped (needs ffmpeg)");
            return false;
        };
        let Some(src) = crate::movies::loose_file(game, &path) else {
            warn!("portal2: intro movie {path} not found");
            continue;
        };
        let stem = path.rsplit('/').next().unwrap_or("intro").trim_end_matches(".bik").to_string();
        let Some((child, frames)) = spawn_ffmpeg(&ff, &src) else {
            warn!("portal2: intro movie {path} could not be decoded");
            continue;
        };
        // No audio track is not fatal: the picture plays silent.
        let handle = extract_audio(&ff, &src, &stem).map(|bytes| audio.add(bevy::audio::AudioSource { bytes: bytes.into() }));
        info!("portal2: intro movie {path}");
        intro.movie = Some(Playing { path, child, frames: Mutex::new(frames), index: 0, t0: None, ended: false, started: now, audio: handle, got_frame: false });
        return true;
    }
    false
}

fn start_intro(mut commands: Commands, mut intro: ResMut<Intro>, game: Res<GameFsRes>, mut screen: ResMut<Screen>, mut images: ResMut<Assets<Image>>, mut audio: ResMut<Assets<bevy::audio::AudioSource>>, time: Res<Time<Real>>, campaign: Res<crate::campaign::Campaign>, mut load: MessageWriter<LoadLevel>) {
    if intro.phase == Phase::Done {
        return;
    }
    intro.playlist = playlist(&game);
    if !start_movie(&mut intro, &game, &mut audio, time.elapsed_secs()) {
        // The menu's own startup was held back for us (it may even have run first).
        info!("portal2: no intro movie; opening the menu");
        intro.phase = Phase::Done;
        load.write(LoadLevel { map: crate::ui::menu_background(&campaign), menu: true });
        return;
    }
    // The menu stays closed (and its movie and music off) until the intro is done.
    *screen = Screen::None;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let img = Image::new_fill(
        Extent3d { width: MOVIE_SIZE.x, height: MOVIE_SIZE.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    intro.image = images.add(img);
    let root = commands
        .spawn((
            Name::new("intro"),
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(40),
            bevy::picking::Pickable::IGNORE,
        ))
        .id();
    let picture = commands.spawn((ChildOf(root), Node { width: Val::Px(MOVIE_SIZE.x as f32), height: Val::Px(MOVIE_SIZE.y as f32), ..default() }, ImageNode::new(intro.image.clone()).with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)))).id();
    intro.root = Some(root);
    intro.picture = Some(picture);
    intro.camera = Some(commands.spawn((Name::new("intro camera"), Camera2d)).id());
}

/// The largest 16:9 picture that fits the window.
fn fit(window: &Window) -> Vec2 {
    let (w, h) = (window.width().max(1.0), window.height().max(1.0));
    let aspect = MOVIE_SIZE.x as f32 / MOVIE_SIZE.y as f32;
    if w / h > aspect { Vec2::new(h * aspect, h) } else { Vec2::new(w, w / aspect) }
}

#[allow(clippy::too_many_arguments)]
fn run_intro(
    mut commands: Commands,
    mut intro: ResMut<Intro>,
    mut startup: ResMut<StartupLoad>,
    mut screen: ResMut<Screen>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    time: Res<Time<Real>>,
    game: Res<GameFsRes>,
    campaign: Res<crate::campaign::Campaign>,
    settings: Res<crate::settings::Settings>,
    mut images: ResMut<Assets<Image>>,
    mut audio: ResMut<Assets<bevy::audio::AudioSource>>,
    mut load: MessageWriter<LoadLevel>,
    mut loaded: MessageReader<LevelLoaded>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut Node, &mut ImageNode)>,
) {
    let now = time.elapsed_secs();
    match intro.phase {
        Phase::Done => {}
        Phase::Movie | Phase::Fade => {
            if let (Some(p), Ok(w)) = (intro.picture, windows.single()) {
                if let Ok((mut node, _)) = nodes.get_mut(p) {
                    let size = fit(w);
                    node.width = Val::Px(size.x);
                    node.height = Val::Px(size.y);
                }
            }
            if intro.phase == Phase::Fade {
                let k = 1.0 - ((now - intro.fade_start) / intro.fade_len).clamp(0.0, 1.0);
                if let Some((_, mut img)) = intro.picture.and_then(|p| nodes.get_mut(p).ok()) {
                    img.color = Color::srgba(1.0, 1.0, 1.0, k);
                }
                if k <= 0.0 {
                    // Cut to the loading screen: the menu's map loads behind it.
                    if let Some(r) = intro.root.take() {
                        commands.entity(r).despawn();
                    }
                    intro.picture = None;
                    intro.phase = Phase::Loading;
                    intro.load_frames = 0;
                    startup.0 = true;
                    load.write(LoadLevel { map: crate::ui::menu_background(&campaign), menu: true });
                    info!("portal2: intro done; startup loading screen");
                }
                return;
            }
            let skip = keys.get_just_pressed().next().is_some() || mouse.as_ref().is_some_and(|m| m.get_just_pressed().next().is_some());
            let mut finished = skip;
            let image = intro.image.clone();
            let picture = intro.picture;
            let mut new_audio = None;
            if let Some(m) = intro.movie.as_mut() {
                // Frame i is due i/fps after frame 0 arrived; catch up by dropping
                // frames if the game ran late, never run ahead.
                let t0 = m.t0.unwrap_or(now);
                let due = ((now - t0).max(0.0) * MOVIE_FPS as f32) as u32;
                let mut frame = None;
                {
                    let rx = m.frames.lock().unwrap();
                    while m.index <= due && !m.ended {
                        match rx.try_recv() {
                            Ok(f) => {
                                frame = Some(f);
                                m.index += 1;
                                if m.t0.is_none() {
                                    m.t0 = Some(now);
                                    break;
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => m.ended = true,
                        }
                    }
                }
                if let Some(frame) = frame {
                    if let Some(mut img) = images.get_mut(&image) {
                        img.data = Some(frame);
                    }
                    if !m.got_frame {
                        m.got_frame = true;
                        let vol = settings.effects_volume * settings.master_volume;
                        if let Some(handle) = m.audio.clone() {
                            let e = commands
                                .spawn((
                                    Name::new(format!("intro audio {}", m.path)),
                                    AudioPlayer(handle),
                                    PlaybackSettings { mode: bevy::audio::PlaybackMode::Despawn, volume: bevy::audio::Volume::Linear(vol), ..default() },
                                ))
                                .id();
                            new_audio = Some(e);
                            info!("portal2: intro audio started ({}, volume {vol:.2})", m.path);
                        }
                        if let Some((_, mut img)) = picture.and_then(|p| nodes.get_mut(p).ok()) {
                            img.color = Color::WHITE;
                        }
                    }
                }
                let stalled = !m.got_frame && now - m.started > FIRST_FRAME_TIMEOUT;
                if stalled {
                    warn!("portal2: intro movie {} gave no frames; skipping", m.path);
                }
                finished |= m.ended || stalled;
            } else {
                finished = true;
            }
            if new_audio.is_some() {
                intro.audio = new_audio;
            }
            if finished {
                if skip {
                    info!("portal2: intro skipped");
                }
                if let Some(m) = intro.movie.as_ref() {
                    info!("portal2: intro movie {} over after {:.1}s, {} frames", m.path, now - m.t0.unwrap_or(now), m.index);
                }
                // The next movie, or the fade out. A movie that never showed a frame
                // means ffmpeg cannot decode it: skip the sequence, straight to the menu.
                let broken = intro.movie.as_ref().is_some_and(|m| !m.got_frame) && !skip;
                intro.movie = None;
                if let Some(a) = intro.audio.take() {
                    if let Ok(mut e) = commands.get_entity(a) {
                        e.despawn();
                    }
                }
                if broken {
                    if let Some(r) = intro.root.take() {
                        commands.entity(r).despawn();
                    }
                    intro.picture = None;
                    if let Some(c) = intro.camera.take() {
                        commands.entity(c).despawn();
                    }
                    intro.phase = Phase::Done;
                    *screen = Screen::Main;
                    load.write(LoadLevel { map: crate::ui::menu_background(&campaign), menu: true });
                } else if start_movie(&mut intro, &game, &mut audio, now) {
                    // On to the next movie.
                } else {
                    intro.phase = Phase::Fade;
                    intro.fade_start = now;
                    intro.fade_len = if skip { FADE_SKIP } else { FADE_END };
                }
            }
        }
        Phase::Loading => {
            intro.load_frames += 1;
            let ready = loaded.read().any(|e| e.menu);
            if ready {
                // The menu builds under the loading screen, which fades out; the
                // level's own camera takes over from the intro's.
                *screen = Screen::Main;
                if let Some(c) = intro.camera.take() {
                    commands.entity(c).despawn();
                }
            }
            if !startup.0 && intro.load_frames > 3 {
                if !ready && *screen != Screen::Main {
                    *screen = Screen::Main;
                }
                if let Some(c) = intro.camera.take() {
                    commands.entity(c).despawn();
                }
                intro.phase = Phase::Done;
                info!("portal2: startup sequence finished");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_lines() {
        assert_eq!(parse_playlist("media/valve.bik"), vec!["media/valve.bik"]);
        assert_eq!(parse_playlist("// intro\r\n\r\n\"media\\a.bik\"\r\n  media/b.bik // second\n"), vec!["media/a.bik", "media/b.bik"]);
        assert!(parse_playlist("\n  \n// nothing\n").is_empty());
    }

    #[test]
    fn picture_fits_the_window() {
        let mut w = Window::default();
        w.resolution.set(1920.0, 1080.0);
        assert_eq!(fit(&w), Vec2::new(1920.0, 1080.0));
        w.resolution.set(2560.0, 1080.0);
        assert_eq!(fit(&w), Vec2::new(1920.0, 1080.0));
        w.resolution.set(800.0, 800.0);
        assert_eq!(fit(&w), Vec2::new(800.0, 450.0));
    }
}
