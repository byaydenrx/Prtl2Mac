//! The loading screen, as Portal 2's LoadingProgress panel
//! (resource/ui/basemodui/loadingprogress.res; behaviour from client.dll):
//! the act's art from materials/vgui/loading_screens stretched over the
//! screen (`loadingscreen_a<act>_<n>`, `_widescreen` on wide screens), the
//! spinning logo (WorkingAnim: r75, 10, 65 x 65 units, vgui/spinner, 8
//! frames) and the row of progress dots (vgui/loadbar_dots) ending at the
//! LoadingProgess anchor r11, r11. Shown for every non-menu level load:
//! `level::load_level` waits until it is drawn, and it goes once the new
//! map has rendered a frame or two.
use bevy::prelude::*;
use p2_assets::GameFsRes;

use crate::level::PendingLoad;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingScreen>().add_systems(Update, loading_screen.after(crate::level::GameSet::Load));
    }
}

/// Dots in the bar, measured from a 1920 x 1080 Portal 2 loading shot:
/// 15 dots 11 units apart, about 7 units across.
const DOTS: usize = 15;
const DOT_PITCH: f32 = 11.0;
const DOT_SIZE: f32 = 8.0;
/// Frames the screen stays after the load, so the new map is drawn under it.
const HIDE_FRAMES: u8 = 3;
/// WorkingAnim frames per second (guess: the .res and client.dll strings
/// don't give the rate).
const SPINNER_FPS: f32 = 10.0;

#[derive(Resource, Default)]
struct LoadingScreen {
    root: Option<Entity>,
    /// Frames left before the screen goes, once the load is done.
    hide_in: Option<u8>,
    spinner: Vec<Handle<Image>>,
    dots: Option<Handle<Image>>,
    /// Loaded art by material path.
    art: Vec<(String, Handle<Image>)>,
    shown_at: f32,
}

#[derive(Component)]
struct LoadingRoot;

#[derive(Component)]
struct LoadingSpinner;

#[derive(Component)]
struct LoadingDot;

/// Which act's art a map gets, as client.dll picks it: the number after
/// `sp_a`, except the two act 3 maps that still look like act 2
/// (sp_a3_00, sp_a3_01); the credits and non-campaign maps get the default art.
fn loading_act(map: &str) -> Option<u32> {
    let m = map.to_ascii_lowercase();
    if m == "sp_a5_credits" {
        return None;
    }
    if m == "sp_a3_00" || m == "sp_a3_01" {
        return Some(2);
    }
    let rest = m.strip_prefix("sp_a")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok().filter(|&a| a > 0)
}

/// The art a map's loading screen shows: of the act's `_1`..`_4` pictures
/// that exist (`_widescreen` ones on wide screens), one picked from the map
/// name. Portal 2 cycles them every 5 s with a 1 s crossfade
/// (ui_loadingscreen_autotransition_time / transition_time); our loads are
/// one blocking frame, so a single picture is all that would ever show.
/// Which one Portal 2 starts on isn't known (guess: per map).
fn loading_art(map: &str, wide: bool, exists: impl Fn(&str) -> bool) -> Option<String> {
    let base = match loading_act(map) {
        Some(a) => format!("vgui/loading_screens/loadingscreen_a{a}"),
        None => "vgui/loading_screens/loadingscreen_default_a".into(),
    };
    let suffix = if wide { "_widescreen" } else { "" };
    let found: Vec<String> = (1..=4).map(|i| format!("{base}_{i}{suffix}")).filter(|p| exists(&format!("materials/{p}.vtf"))).collect();
    if found.is_empty() {
        return if wide { loading_art(map, false, exists) } else { None };
    }
    let h = map.to_ascii_lowercase().bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    Some(found[h as usize % found.len()].clone())
}

fn vgui_image(game: &GameFsRes, images: &mut Assets<Image>, path: &str, frame: u32) -> Option<Handle<Image>> {
    let vtf = source_assets::vtf::VtfTexture::parse(&game.0.read(&format!("materials/{path}.vtf"))?).ok()?;
    let vtf = if vtf.frame_count() > 1 { vtf.frame(frame.min(vtf.frame_count() - 1)) } else { vtf };
    Some(images.add(vtf.into_image(false, true)))
}

#[allow(clippy::too_many_arguments)]
fn loading_screen(
    mut commands: Commands,
    mut screen: ResMut<LoadingScreen>,
    pending: Res<PendingLoad>,
    game: Res<GameFsRes>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut spinner: Query<&mut ImageNode, (With<LoadingSpinner>, Without<LoadingDot>)>,
    mut dots: Query<&mut ImageNode, (With<LoadingDot>, Without<LoadingSpinner>)>,
) {
    let loading = pending.0.as_ref().filter(|(req, _)| !req.menu);
    match (loading, screen.root) {
        (Some((req, _)), None) => {
            let wide = windows.single().map(|w| w.width() / w.height().max(1.0) > 1.5).unwrap_or(true);
            let art = loading_art(&req.map, wide, |p| game.0.exists(p)).and_then(|p| {
                if let Some((_, h)) = screen.art.iter().find(|(q, _)| *q == p) {
                    return Some(h.clone());
                }
                let h = vgui_image(&game, &mut images, &p, 0)?;
                screen.art.push((p, h.clone()));
                Some(h)
            });
            if screen.spinner.is_empty() {
                screen.spinner = (0..8).filter_map(|f| vgui_image(&game, &mut images, "vgui/spinner", f)).collect();
            }
            if screen.dots.is_none() {
                screen.dots = vgui_image(&game, &mut images, "vgui/loadbar_dots", 0);
            }
            let root = spawn_screen(&mut commands, art, screen.spinner.first().cloned(), screen.dots.clone());
            info!("portal2: loading screen for {}", req.map);
            screen.root = Some(root);
            screen.hide_in = None;
            screen.shown_at = time.elapsed_secs();
        }
        (None, Some(root)) => {
            // The load ran: the bar is full until the screen goes.
            // P2_LOADING_HOLD=<seconds> keeps it up (for screenshots).
            let hold = std::env::var("P2_LOADING_HOLD").ok().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let held = time.elapsed_secs() - screen.shown_at < hold;
            let n = if held { HIDE_FRAMES } else { screen.hide_in.unwrap_or(HIDE_FRAMES) };
            if n == 0 {
                commands.entity(root).despawn();
                screen.root = None;
                screen.hide_in = None;
                return;
            }
            screen.hide_in = Some(n - 1);
            for mut d in &mut dots {
                d.rect = Some(dot_cell(true));
                d.color = Color::WHITE;
            }
        }
        _ => {}
    }
    if screen.root.is_some() && !screen.spinner.is_empty() {
        let f = ((time.elapsed_secs() - screen.shown_at) * SPINNER_FPS) as usize % screen.spinner.len();
        for mut s in &mut spinner {
            s.image = screen.spinner[f].clone();
        }
    }
}

/// loadbar_dots.vtf is four 16 x 16 cells: white, blue, orange, empty.
/// Lit dots are the blue cell, unlit ones the white cell (its own alpha is
/// low; it reads grey 72 over the art in the reference shot).
fn dot_cell(lit: bool) -> Rect {
    let x = if lit { 16.0 } else { 0.0 };
    Rect::new(x, 0.0, x + 16.0, 16.0)
}

const fn units(u: f32) -> Val {
    Val::Vh(u * 100.0 / 480.0)
}

fn spawn_screen(commands: &mut Commands, art: Option<Handle<Image>>, spinner: Option<Handle<Image>>, dots: Option<Handle<Image>>) -> Entity {
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let root = commands
        .spawn((LoadingRoot, full.clone(), BackgroundColor(Color::BLACK), GlobalZIndex(30), bevy::picking::Pickable::IGNORE))
        .id();
    if let Some(art) = art {
        commands.entity(root).with_child((full, ImageNode::new(art)));
    }
    if let Some(spin) = spinner {
        commands.entity(root).with_child((
            LoadingSpinner,
            Node { position_type: PositionType::Absolute, right: units(10.0), top: units(10.0), width: units(65.0), height: units(65.0), ..default() },
            ImageNode::new(spin),
        ));
    }
    if let Some(dots) = dots {
        commands.entity(root).with_children(|p| {
            p.spawn(Node {
                position_type: PositionType::Absolute,
                right: units(11.0),
                bottom: units(11.0),
                flex_direction: FlexDirection::Row,
                column_gap: units(DOT_PITCH - DOT_SIZE),
                ..default()
            })
            .with_children(|row| {
                for _ in 0..DOTS {
                    // Bevy blends in linear light; vgui blends gamma-encoded
                    // colours, so the faint unlit dots get a lower alpha to
                    // look as dark as in the game.
                    let mut img = ImageNode::new(dots.clone()).with_color(Color::srgba(1.0, 1.0, 1.0, 0.3));
                    img.rect = Some(dot_cell(false));
                    row.spawn((LoadingDot, Node { width: units(DOT_SIZE), height: units(DOT_SIZE), ..default() }, img));
                }
            });
        });
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_from_map_name() {
        assert_eq!(loading_act("sp_a1_intro1"), Some(1));
        assert_eq!(loading_act("SP_A2_laser_intro"), Some(2));
        assert_eq!(loading_act("sp_a3_00"), Some(2));
        assert_eq!(loading_act("sp_a3_01"), Some(2));
        assert_eq!(loading_act("sp_a3_03"), Some(3));
        assert_eq!(loading_act("sp_a4_finale4"), Some(4));
        assert_eq!(loading_act("sp_a5_credits"), None);
        assert_eq!(loading_act("mp_coop_lobby_3"), None);
    }

    #[test]
    fn art_prefers_widescreen_and_existing_files() {
        let have = |p: &str| p.contains("_a5_1") || p.contains("default_a_2");
        assert_eq!(loading_art("sp_a5_finale1", true, have).as_deref(), Some("vgui/loading_screens/loadingscreen_a5_1_widescreen"));
        assert_eq!(loading_art("sp_a5_finale1", false, have).as_deref(), Some("vgui/loading_screens/loadingscreen_a5_1"));
        assert_eq!(loading_art("my_map", true, have).as_deref(), Some("vgui/loading_screens/loadingscreen_default_a_2_widescreen"));
        // No widescreen file: the 4:3 one.
        let only43 = |p: &str| p.ends_with("_a1_3.vtf");
        assert_eq!(loading_art("sp_a1_intro2", true, only43).as_deref(), Some("vgui/loading_screens/loadingscreen_a1_3"));
        assert_eq!(loading_art("sp_a1_intro2", true, |_| false), None);
        // The same map always gets the same picture.
        let all = |_: &str| true;
        assert_eq!(loading_art("sp_a2_bts1", true, all), loading_art("sp_a2_bts1", true, all));
    }
}
