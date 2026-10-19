//! Loading levels: the map from the install, the player at the arrival
//! point, the gun the campaign has given by then, the map's fog on every
//! camera, and the entity table the I/O system runs on.
use std::collections::HashMap;

use bevy::prelude::*;
use p2_core::*;
use shared::source_to_world;
use source_assets::{spawn_map, MapInfo, SourceMapAssets};

use crate::campaign::Campaign;
use crate::io::Ents;

pub struct LevelPlugin;

/// Level loading first, then everything that reacts to the loaded level
/// (so the entity table and map are in place when `LevelLoaded` is read).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    Load,
    Logic,
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LoadLevel>()
            .add_message::<LevelLoaded>()
            .configure_sets(Update, (GameSet::Load, GameSet::Logic).chain())
            .add_systems(Update, load_level.in_set(GameSet::Load))
            .add_systems(Update, (map_look_on_cameras, light_on_cameras, gun_visibility, trace_player).in_set(GameSet::Logic));
    }
}

/// Load a map. `menu` loads it as the main menu's background (no control).
#[derive(Message, Clone, Debug)]
pub struct LoadLevel {
    pub map: String,
    pub menu: bool,
}

/// A map finished loading (for HUD titles, music, saves).
#[derive(Message, Clone, Debug)]
pub struct LevelLoaded {
    pub map: String,
    pub menu: bool,
}

/// The level that is loaded now.
#[derive(Resource)]
pub struct CurrentMap {
    pub info: MapInfo,
    pub menu: bool,
    /// Brush model bounds (world metres) by model index, for triggers.
    pub model_bounds: HashMap<usize, (Vec3, Vec3)>,
}

/// Which shots the gun has on a map, from the install's transition list:
/// none before the first map with the gun (unless the map hands it out),
/// blue only until the upgrade, then both.
fn gun_for(campaign: &Campaign, map: &str, info: &MapInfo) -> [bool; 2] {
    let order = |m: &str| campaign.maps.iter().position(|x| x.eq_ignore_ascii_case(m));
    let here = order(map);
    let first_gun = order(&campaign_const(campaign, "FIRST_MAP_WITH_GUN").unwrap_or_else(|| "sp_a1_intro4".into()));
    let upgrade = order(&campaign_const(campaign, "FIRST_MAP_WITH_UPGRADE_GUN").unwrap_or_else(|| "sp_a2_laser_intro".into()));
    let hands_out_gun = info.entities.iter().any(|e| e.classname == "weapon_portalgun");
    match (here, first_gun, upgrade) {
        (Some(h), Some(g), _) if h < g => [false, false],
        (Some(h), _, Some(u)) if h < u && hands_out_gun => {
            // The upgrade map (sp_a2_intro) starts with blue and picks up orange.
            if h + 1 >= u || map.starts_with("sp_a2") {
                [true, false]
            } else {
                [false, false]
            }
        }
        (Some(h), _, Some(u)) if h < u => [true, false],
        // A gun lying in a later map is the one the player lost: sp_a3_01
        // starts after the fall without it (the knockout scene drops it).
        (Some(h), _, Some(u)) if h > u && hands_out_gun => [false, false],
        _ => [true, true],
    }
}

fn campaign_const(c: &Campaign, name: &str) -> Option<String> {
    c.consts.get(name).cloned()
}

/// Brush model boxes in world space (Source mins/maxs, axes swapped).
fn model_bounds(game: &source_assets::GameFs, map: &str) -> HashMap<usize, (Vec3, Vec3)> {
    let mut out = HashMap::new();
    let Some(data) = game.read(&format!("maps/{map}.bsp")) else { return out };
    let Ok(bsp) = vbsp::Bsp::read(&data) else { return out };
    for (i, m) in bsp.models.iter().enumerate() {
        let a = source_to_world(Vec3::new(m.mins.x, m.mins.y, m.mins.z));
        let b = source_to_world(Vec3::new(m.maxs.x, m.maxs.y, m.maxs.z));
        out.insert(i, (a.min(b), a.max(b)));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn load_level(
    mut commands: Commands,
    mut requests: MessageReader<LoadLevel>,
    mut assets: SourceMapAssets,
    current: Option<Res<CurrentMap>>,
    campaign: Res<Campaign>,
    game: Res<p2_assets::GameFsRes>,
    mut players: Query<(Entity, &mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity, &mut PortalGun), With<Player>>,
    portals: Query<Entity, With<Portal>>,
    level_owned: Query<Entity, With<LevelOwned>>,
    mut loaded: MessageWriter<LevelLoaded>,
) {
    let Some(req) = requests.read().last().cloned() else { return };
    // Out with the old level: the map, its portals and everything spawned for it.
    if let Some(cur) = &current {
        commands.entity(cur.info.root).despawn();
    }
    for e in portals.iter().chain(level_owned.iter()) {
        commands.entity(e).despawn();
    }
    let info = match spawn_map(&mut commands, &mut assets, &req.map, Vec3::ZERO) {
        Ok(i) => i,
        Err(e) => {
            error!("portal2: can't load {}: {e}", req.map);
            return;
        }
    };
    info!(
        "portal2: {} loaded in {:.2}s, {} entities, arrival {:.2?}",
        info.name,
        info.convert_seconds,
        info.entities.len(),
        info.arrival.translation
    );
    let (yaw, _, _) = info.arrival.rotation.to_euler(EulerRot::YXZ);
    let feet = info.arrival.translation;
    let can_fire = if req.menu { [false, false] } else { gun_for(&campaign, &req.map, &info) };
    match players.single_mut() {
        Ok((_, mut tf, mut ifeet, mut view, mut vel, mut gun)) => {
            tf.translation = feet;
            *ifeet = InterpolatedFeet { prev: feet, curr: feet };
            *view = ViewAngles { yaw, pitch: 0.0, roll: 0.0 };
            vel.0 = Vec3::ZERO;
            *gun = PortalGun { can_fire, ..default() };
        }
        Err(_) => {
            let p = spawn_player(&mut commands, feet, yaw);
            commands.entity(p).insert(PortalGun { can_fire, ..default() });
        }
    }
    commands.insert_resource(ClearColor(info.fog.as_ref().map(|f| f.color).unwrap_or(Color::BLACK)));
    let bounds = model_bounds(&game.0, &req.map);
    commands.insert_resource(Ents::from_map(&info, &bounds));
    commands.insert_resource(CurrentMap { info, menu: req.menu, model_bounds: bounds });
    loaded.write(LevelLoaded { map: req.map.clone(), menu: req.menu });
}

/// Despawned with the level (props made dynamic, map portals, sounds...).
#[derive(Component, Default, Clone)]
pub struct LevelOwned;

/// Every camera (main view, viewmodel, portal views) gets the current map's
/// fog, or its skybox when it has no fog.
fn map_look_on_cameras(
    mut commands: Commands,
    map: Option<Res<CurrentMap>>,
    all: Query<Entity, With<Camera3d>>,
    new: Query<Entity, Added<Camera3d>>,
) {
    let Some(map) = map else { return };
    let cams: Vec<Entity> = if map.is_changed() { all.iter().collect() } else { new.iter().collect() };
    for e in cams {
        let mut ec = commands.entity(e);
        ec.remove::<(DistanceFog, bevy::core_pipeline::Skybox)>();
        let fog = map.info.fog.as_ref().filter(|_| std::env::var_os("P2_NOFOG").is_none());
        match (fog, &map.info.skybox) {
            (Some(fog), _) => {
                ec.insert(fog.distance_fog());
            }
            (None, Some(sky)) => {
                ec.insert(bevy::core_pipeline::Skybox { image: Some(sky.clone()), brightness: 1000.0, ..default() });
            }
            _ => {}
        }
    }
}

/// The world is lit by its own lightmaps; Bevy's lights only reach the
/// viewmodel and Chell, so give them a soft room light.
fn light_on_cameras(mut commands: Commands, cams: Query<Entity, (Added<Camera3d>, Without<AmbientLight>)>) {
    for e in &cams {
        commands.entity(e).insert(AmbientLight { color: Color::srgb(1.0, 0.98, 0.94), brightness: 400.0, affects_lightmapped_meshes: false });
    }
}

/// No gun, no viewmodel, gun light or crosshair. (The viewmodel camera
/// stays on: the UI is drawn through it.)
fn gun_visibility(
    guns: Query<&PortalGun, With<Player>>,
    menu: Option<Res<CurrentMap>>,
    vm: Query<&Children, With<p2_view::ViewModelCamera>>,
    mut vis: Query<&mut Visibility, Without<p2_view::crosshair::CrosshairRoot>>,
    mut crosshair: Query<&mut Visibility, With<p2_view::crosshair::CrosshairRoot>>,
    mut fx: ResMut<p2_fx::FxSettings>,
    cinematic: Res<crate::viewcontrol::ViewControl>,
    screen: Res<crate::ui::Screen>,
) {
    let has_gun = guns.single().is_ok_and(|g| g.can_fire.iter().any(|c| *c));
    let in_menu = menu.is_some_and(|m| m.menu);
    let show = has_gun && !in_menu;
    let want = if show { Visibility::Inherited } else { Visibility::Hidden };
    // The dot crosshair shows from the start (the brackets only with a gun),
    // except in menus and cutscene cameras.
    let dots = !in_menu && !cinematic.is_active() && *screen == crate::ui::Screen::None;
    let want_dots = if dots { Visibility::Inherited } else { Visibility::Hidden };
    for children in &vm {
        for c in children.iter() {
            if let Ok(mut v) = vis.get_mut(c) {
                if *v != want {
                    *v = want;
                }
            }
        }
    }
    for mut v in &mut crosshair {
        if *v != want_dots {
            *v = want_dots;
        }
    }
    if fx.top_light != show {
        fx.top_light = show;
        fx.muzzle_flash = show;
    }
}

/// `P2_TRACE=1`: log the player's feet twice a second.
fn trace_player(
    time: Res<Time<Real>>,
    mut frame_times: Local<Vec<f32>>,
    cams: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    players: Query<(&Transform, &CharacterVelocity, &Grounded, &ViewAngles), With<Player>>,
    names: Query<(Option<&Name>, Option<&ChildOf>)>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    if std::env::var_os("P2_TRACE").is_none() {
        return;
    }
    frame_times.push(time.delta_secs());
    if frame_times.len() >= 120 {
        let mut v = std::mem::take(&mut *frame_times);
        v.sort_by(|a, b| a.total_cmp(b));
        let avg = v.iter().sum::<f32>() / v.len() as f32;
        info!("TRACE fps {:.0} (frame avg {:.1} ms, worst {:.1} ms)", 1.0 / avg, avg * 1000.0, v[v.len() - 1] * 1000.0);
    }
    if *frame % 30 != 0 {
        return;
    }
    if let Ok((tf, v, g, va)) = players.single() {
        info!("TRACE view {:?}", va);
        let mut chain = Vec::new();
        let mut cur = g.entity;
        while let Some(c) = cur {
            let (n, p) = names.get(c).unwrap_or((None, None));
            chain.push(n.map(|n| n.as_str().to_string()).unwrap_or_else(|| format!("{c}")));
            cur = p.map(|p| p.parent());
            if chain.len() > 4 {
                break;
            }
        }
        info!("TRACE f{} feet {:.2?} vel {:.2?} ground {} on {:?}", *frame, tf.translation, v.0, g.on_ground, chain);
        for (c, gt) in &cams {
            info!("TRACE camera active {} at {:.2?} rot {:.3?}", c.is_active, gt.translation(), gt.rotation());
        }
    }
}
