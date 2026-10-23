//! Bevy side: the plugin, spawning a converted map at any world offset, and
//! the components other crates query.

use crate::collision::{ColliderData, SideInfo};
use crate::config::GameConfig;
use crate::entities::{source_angles_to_world, source_view_to_world, SourceEntity};
use crate::fs::GameFs;
use crate::geometry::MeshData;
use crate::map::{self, LoadedMap};
use crate::material::{self, SourceMaterial, SourceMaterialParams};
use crate::models::ModelData;
use crate::surfaceprops::SurfaceProps;
use crate::vmt::{BlendKind, MaterialInfo};
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use shared::{source_dir_to_world, source_to_world, world_to_source, Portalable};
use std::collections::HashMap;
use std::sync::Arc;

/// Mounts the game and registers the Source material.
///
/// If the game cannot be mounted the plugin logs an error and
/// [`spawn_map`] returns an error; the app keeps running.
#[derive(Default)]
pub struct SourceMapPlugin {
    /// Override the install; `None` = `PORTAL2_DIR` or the default path.
    pub config: Option<GameConfig>,
}

impl Plugin for SourceMapPlugin {
    fn build(&self, app: &mut App) {
        material::register(app);
        let config = self.config.clone().unwrap_or_else(GameConfig::from_env);
        let game = match GameFs::mount(config) {
            Ok(fs) => {
                let surfprops = SurfaceProps::load(&fs);
                info!("source_assets: mounted {}", fs.config.root.display());
                Some((Arc::new(fs), Arc::new(surfprops)))
            }
            Err(e) => {
                error!("source_assets: {e}");
                None
            }
        };
        app.insert_resource(SourceGame {
            fs: game.as_ref().map(|g| g.0.clone()),
            surfprops: game.map(|g| g.1).unwrap_or_default(),
        })
        .init_resource::<SourceCache>()
        .init_resource::<SourceRenderSettings>()
        .init_resource::<crate::valve::ValveShaderLibrary>()
        .add_plugins(crate::valve_render::ValveRenderPlugin)
        .add_plugins(crate::proxy::MaterialProxyPlugin)
        .add_plugins(crate::projected::ProjectedLightPlugin)
        .add_plugins(crate::glass::GlassPlugin)
        .add_systems(PostUpdate, (relight_moved_props.after(TransformSystems::Propagate), update_stretch_surfaces));
        if std::env::var_os("P2_TRACE_BONE").is_some() {
            app.add_systems(PostUpdate, trace_bones.after(TransformSystems::Propagate));
        }
    }
}

/// Debug: P2_TRACE_BONE=name logs where matching bone parts are, every 2 s.
fn trace_bones(time: Res<Time>, mut next: Local<f32>, q: Query<(&Name, &GlobalTransform, &Transform, &InheritedVisibility), With<BonePart>>) {
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + 2.0;
    let key = std::env::var("P2_TRACE_BONE").unwrap_or_default();
    for (name, g, t, vis) in &q {
        if name.as_str().contains(&key) {
            info!("TRACE bone {name}: world {} rot {} local {} scale {} visible {}", g.translation(), g.rotation(), t.translation, t.scale, vis.get());
        }
    }
}

/// The mounted game filesystem.
#[derive(Resource, Clone)]
pub struct SourceGame {
    pub fs: Option<Arc<GameFs>>,
    pub surfprops: Arc<SurfaceProps>,
}

/// Default [`SourceRenderSettings::light_scale`]: Portal 2 maps set their
/// auto-exposure range to roughly 0.5..3 (`env_tonemap_controller`), so a
/// settled exposure of ~3 reproduces the in-game brightness.
pub const DEFAULT_LIGHT_SCALE: f32 = 3.0;

/// How baked lighting is turned into colour.
#[derive(Resource, Clone, Debug)]
pub struct SourceRenderSettings {
    /// Multiplier on lightmaps and prop vertex light.
    pub light_scale: f32,
    /// Ignore baked light (debugging textures and geometry).
    pub fullbright: bool,
    /// Draw surfaces with Portal 2's own shaders where translated
    /// (`SourceMaterial` stays the fallback).
    pub valve_shaders: bool,
}

impl Default for SourceRenderSettings {
    fn default() -> Self {
        SourceRenderSettings {
            light_scale: DEFAULT_LIGHT_SCALE,
            fullbright: false,
            valve_shaders: true,
        }
    }
}

/// Converted maps and uploaded images, shared between spawns.
#[derive(Resource, Default)]
pub struct SourceCache {
    pub maps: HashMap<String, Arc<LoadedMap>>,
    images: HashMap<String, Handle<Image>>,
    /// Linear (non-sRGB) uploads, for normal maps and masks.
    linear_images: HashMap<String, Handle<Image>>,
    fallback_images: HashMap<&'static str, Handle<Image>>,
    /// Per map: materials drawn by the fallback instead of Valve shaders, with why.
    pub fallback_materials: HashMap<String, Vec<(String, String)>>,
    prop_materials: HashMap<(String, bool), Handle<SourceMaterial>>,
    white: Option<Handle<Image>>,
    /// Every frame of multi-frame textures by (name, srgb), frame 0 first.
    frame_images: HashMap<(String, bool), Vec<Handle<Image>>>,
}

/// Everything [`spawn_map`] needs, as one system parameter.
#[derive(SystemParam)]
pub struct SourceMapAssets<'w> {
    pub game: Res<'w, SourceGame>,
    pub cache: ResMut<'w, SourceCache>,
    pub settings: Res<'w, SourceRenderSettings>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<SourceMaterial>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub compressed: Option<Res<'w, CompressedImageFormatSupport>>,
    pub shaders: ResMut<'w, Assets<bevy::shader::Shader>>,
    pub valve: Res<'w, crate::valve::ValveShaderLibrary>,
    pub projected: Res<'w, crate::projected::ProjectedLightAssets>,
    pub glass: ResMut<'w, Assets<crate::glass::GlassMaterial>>,
    pub water_reflection: Res<'w, crate::glass::WaterReflection>,
}

// ---------------------------------------------------------------- components

/// Root of a spawned map. Children: geometry, colliders, props.
#[derive(Component, Clone, Debug)]
pub struct SourceMapRoot {
    pub name: String,
}

/// On every brush collider: what the portal gun needs to judge a hit.
#[derive(Component, Clone, Debug)]
pub struct SourceCollider {
    /// Brush contents flags (`CONTENTS_*`).
    pub contents: u32,
    pub brush: u32,
    /// 0 = world, N = brush entity `*N`.
    pub model: u32,
    /// Outward faces covered by this collider, with material and surfaceprop.
    pub sides: Vec<SideInfo>,
}

impl SourceCollider {
    /// The side whose outward normal best matches a hit normal (world space).
    pub fn side_for_normal(&self, normal: Vec3) -> Option<&SideInfo> {
        self.sides
            .iter()
            .max_by(|a, b| a.normal.dot(normal).total_cmp(&b.normal.dot(normal)))
    }
}

/// On an entity prop drawn as rigid per-bone parts so it can animate
/// ([`crate::map::class_animates`], [`ModelData::is_animated`]). Its
/// [`BonePart`] children start in the default pose; a game moves them.
#[derive(Component, Clone, Debug)]
pub struct AnimatedModel {
    /// The `.mdl` path.
    pub model: String,
}

/// A child of an [`AnimatedModel`] carrying the meshes (and per-bone `.phy`
/// collision) that follow one bone. Its transform is that bone's
/// model-space transform in world axes ([`crate::models::bone_transform`]).
/// The skin a model with [`SkinVariant`] groups draws with; change it to
/// switch skins (a cube lighting up on a button).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelSkin(pub i32);

/// A model's spawn skin and the alternate skin it can switch to.
#[derive(Component, Clone, Copy, Debug)]
pub struct AltSkin {
    pub base: i32,
    pub alt: i32,
}

/// A group of a model's draws used by only this skin (spawned for the
/// model's spawn skin and its alternate skin where their materials differ);
/// shown while the model's [`ModelSkin`] is this skin.
#[derive(Component, Clone, Copy, Debug)]
pub struct SkinVariant(pub i32);

/// On a movable entity prop (a cube, a physics prop): Source lights a
/// moving model each frame from the light where it is, so its baked vertex
/// light is redone when it has moved or turned ([`relight_moved_props`]).
/// Without it a cube kept the light of where it spawned: a dropper's dark
/// tube made it near black everywhere (T-145).
#[derive(Component, Clone)]
pub struct RelightOnMove {
    map: Arc<LoadedMap>,
    lights: Arc<Vec<map::WorldLight>>,
    /// The model's illumination point (Source units, model space, scaled).
    illum: Vec3,
    /// Where the map was spawned (world).
    offset: Vec3,
    tint: Vec3,
    light_scale: f32,
    /// Pose (world) and time of the last lighting.
    last: Option<(Vec3, Quat, f32)>,
}

/// On each Valve surface of a [`RelightOnMove`] model: the Source
/// model-space normals its vertex light is computed from, one per vertex.
#[derive(Component, Clone)]
pub struct RelightNormals(Arc<Vec<Vec3>>);

/// Entity classes that move freely and get [`RelightOnMove`].
const RELIT_CLASSES: &[&str] = &["prop_weighted_cube", "prop_monster_box", "prop_physics", "prop_physics_override", "prop_physics_multiplayer", "npc_portal_turret_floor", "prop_paint_bomb"];
/// Relight after moving this far (Source units) or turning this much (radians)...
const RELIGHT_MOVE: f32 = 8.0;
const RELIGHT_TURN: f32 = 0.15;
/// ...at most this often (seconds).
const RELIGHT_EVERY: f32 = 0.1;

/// Redoes the vertex light of moved [`RelightOnMove`] models, the same way
/// the spawn lit them, at their current position and rotation.
fn relight_moved_props(
    time: Res<Time>,
    mut roots: Query<(Entity, &GlobalTransform, &mut RelightOnMove)>,
    children: Query<&Children>,
    mut surfaces: Query<(&mut crate::valve::ValveSurface, &RelightNormals)>,
) {
    let now = time.elapsed_secs();
    for (e, gt, mut r) in &mut roots {
        let (_, rot, pos) = gt.to_scale_rotation_translation();
        match r.last {
            None => {
                // The spawn just lit it here.
                r.last = Some((pos, rot, now));
                continue;
            }
            Some((p, q, t)) => {
                let moved = world_to_source(p).distance(world_to_source(pos)) > RELIGHT_MOVE || q.angle_between(rot) > RELIGHT_TURN;
                if !moved || now - t < RELIGHT_EVERY {
                    continue;
                }
            }
        }
        r.last = Some((pos, rot, now));
        let illum = world_to_source(pos - r.offset) + world_to_source(rot * source_to_world(r.illum));
        let amb = r.map.lit_ambient_at(illum, &r.lights);
        let scale = r.tint * r.light_scale;
        for c in children.iter_descendants(e) {
            let Ok((mut surface, normals)) = surfaces.get_mut(c) else { continue };
            let mut draw = (*surface.0).clone();
            let mut vertices = (*draw.vertices).clone();
            for (v, n) in vertices.iter_mut().zip(normals.0.iter()) {
                let wn = rot * source_dir_to_world(*n);
                v.color1 = crate::valve::encode_static_light(amb.eval(Vec3::new(wn.x, -wn.z, wn.y)) * scale);
            }
            draw.vertices = Arc::new(vertices);
            surface.0 = Arc::new(draw);
        }
    }
}

/// On the Valve surface of a model's [`crate::models::STRETCH`] part:
/// per vertex, the [`BonePart`] it follows and its position and normal in
/// that bone's space (Source axes). [`update_stretch_surfaces`] moves the
/// vertices whenever a bone part moves.
#[derive(Component, Clone)]
pub struct StretchSurface {
    verts: Arc<Vec<(Entity, Vec3, Vec3)>>,
}

/// Puts each [`StretchSurface`] vertex where its bone part now holds it.
fn update_stretch_surfaces(mut surfaces: Query<(&mut crate::valve::ValveSurface, &StretchSurface)>, parts: Query<Ref<Transform>, With<BonePart>>) {
    for (mut surface, st) in &mut surfaces {
        if !st.verts.iter().any(|(p, ..)| parts.get(*p).is_ok_and(|t| t.is_changed())) {
            continue;
        }
        let mut draw = (*surface.0).clone();
        let mut vertices = (*draw.vertices).clone();
        for (v, &(part, pos, n)) in vertices.iter_mut().zip(st.verts.iter()) {
            let Ok(t) = parts.get(part) else { continue };
            let p = world_to_source(t.transform_point(source_to_world(pos)));
            let wn = t.rotation * source_dir_to_world(n);
            v.position = [p.x, p.y, p.z, 1.0];
            v.normal = [wn.x, -wn.z, wn.y, 1.0];
        }
        draw.vertices = Arc::new(vertices);
        surface.0 = Arc::new(draw);
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BonePart {
    pub bone: u16,
}

pub use shared::PortalShotPassthrough;

/// A rendered entity from the map's entity lump or static prop lump.
#[derive(Component, Clone, Debug)]
pub struct SourceEntityRef {
    pub kind: MapEntityKind,
    /// Index into [`MapInfo::entities`].
    pub index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapEntityKind {
    PlayerStart,
    StaticProp,
    WeightedCube,
    DynamicProp,
    PhysicsProp,
    Door,
    Button,
    /// Any other brush entity with a model (`func_brush`, `func_door`...).
    BrushEntity,
    LightEnvironment,
    /// Laser emitters/catchers, turrets, cameras and other `prop_*` classes.
    OtherProp,
    Other,
}

/// One entity of interest, in world space (map offset applied).
#[derive(Clone, Debug)]
pub struct MapEntity {
    pub kind: MapEntityKind,
    pub classname: String,
    pub targetname: Option<String>,
    pub transform: Transform,
    /// `.mdl` path for studio models, `*N` brush model index as `Some` in `brush_model`.
    pub model: Option<String>,
    pub brush_model: Option<usize>,
    pub skin: i32,
    /// The spawned render entity, if any.
    pub entity: Option<Entity>,
    /// Model-space bounds in world axes (metres), when a model is known.
    pub bounds: Option<(Vec3, Vec3)>,
    pub keyvalues: Vec<(String, String)>,
}

/// `light_environment` (sun) in world space.
#[derive(Clone, Debug)]
pub struct LightEnvironment {
    /// Direction light travels, world space.
    pub direction: Vec3,
    /// Linear colour and brightness from `_light` (RGB 0..1, intensity).
    pub color: Vec3,
    pub brightness: f32,
    pub ambient_color: Vec3,
    pub ambient_brightness: f32,
}

/// `env_fog_controller` settings (the master, else first enabled one), world units.
#[derive(Clone, Debug)]
pub struct MapFog {
    /// Linear colour.
    pub color: Color,
    /// Metres.
    pub start: f32,
    pub end: f32,
    pub max_density: f32,
}

impl MapFog {
    /// The same fog as a Bevy [`DistanceFog`] for a camera.
    pub fn distance_fog(&self) -> DistanceFog {
        DistanceFog {
            color: self.color.with_alpha(self.max_density),
            falloff: FogFalloff::Linear { start: self.start, end: self.end },
            ..default()
        }
    }
}

/// What [`spawn_map`] returns.
#[derive(Clone, Debug)]
pub struct MapInfo {
    pub name: String,
    pub root: Entity,
    pub offset: Vec3,
    /// `info_player_start` in world space: feet position, yaw/pitch rotation (-Z forward).
    /// In most Portal 2 maps this is a hidden room; the game immediately
    /// teleports the player into the arrival elevator (see `arrival`).
    pub player_start: Transform,
    /// Where the player actually begins: the `@arrival_teleport`
    /// point_teleport (inside the arrival elevator) when the map has one,
    /// else `player_start`.
    pub arrival: Transform,
    pub entities: Vec<MapEntity>,
    pub light_environment: Option<LightEnvironment>,
    pub fog: Option<MapFog>,
    /// Cubemap from the 2D skybox (`skyname`), for `bevy::core_pipeline::Skybox`.
    pub skybox: Option<Handle<Image>>,
    pub skyname: Option<String>,
    /// World-space bounds of the brush geometry.
    pub bounds: (Vec3, Vec3),
    /// Seconds spent converting (0 when the map came from the cache).
    pub convert_seconds: f32,
}

impl MapInfo {
    pub fn entities_of(&self, kind: MapEntityKind) -> impl Iterator<Item = &MapEntity> {
        self.entities.iter().filter(move |e| e.kind == kind)
    }
}

// ---------------------------------------------------------------- spawning

/// Convert (or reuse) a map without touching the world. Useful on a
/// background thread: `load_map` then [`spawn_loaded_map`].
pub fn load_map(game: &SourceGame, name: &str) -> anyhow::Result<LoadedMap> {
    let fs = game
        .fs
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Portal 2 is not mounted (set {})", crate::config::PORTAL2_DIR_ENV))?;
    map::load_map(fs, &game.surfprops, name)
}

/// Load `name` (e.g. `"sp_a1_intro3"`) and spawn it with its origin at `offset`.
pub fn spawn_map(
    commands: &mut Commands,
    assets: &mut SourceMapAssets,
    name: &str,
    offset: Vec3,
) -> anyhow::Result<MapInfo> {
    let key = name.to_ascii_lowercase();
    let (map, secs) = match assets.cache.maps.get(&key) {
        Some(m) => (m.clone(), 0.0),
        None => {
            let t = std::time::Instant::now();
            let m = Arc::new(load_map(&assets.game, name)?);
            let secs = t.elapsed().as_secs_f32();
            info!("source_assets: converted {name} in {secs:.2}s {:?}", m.timings);
            assets.cache.maps.insert(key, m.clone());
            (m, secs)
        }
    };
    let mut info = spawn_loaded_map(commands, assets, &map, offset);
    info.convert_seconds = secs;
    Ok(info)
}

fn white_image(assets: &mut SourceMapAssets) -> Handle<Image> {
    if let Some(h) = &assets.cache.white {
        return h.clone();
    }
    let img = Image::new(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let h = assets.images.add(img);
    assets.cache.white = Some(h.clone());
    h
}

fn bc_supported(assets: &SourceMapAssets) -> bool {
    assets
        .compressed
        .as_ref()
        .is_some_and(|c| c.0.contains(CompressedImageFormats::BC))
}

fn texture_handle(assets: &mut SourceMapAssets, map: &LoadedMap, name: &str) -> Option<Handle<Image>> {
    if let Some(h) = assets.cache.images.get(name) {
        return Some(h.clone());
    }
    let tex = map.textures.get(name)?;
    let bc = bc_supported(assets);
    let h = assets.images.add(tex.frame(0).into_image(bc, true));
    assets.cache.images.insert(name.to_string(), h.clone());
    Some(h)
}

/// Every animation frame of a multi-frame texture (frame 0 is `first`, its
/// existing upload); empty for single-frame textures.
fn texture_frames(
    images: &mut Assets<Image>,
    cache: &mut SourceCache,
    bc: bool,
    map: &LoadedMap,
    name: &str,
    srgb: bool,
    first: &Handle<Image>,
) -> Vec<Handle<Image>> {
    let Some(tex) = map.textures.get(name).filter(|t| t.frame_count() > 1) else { return Vec::new() };
    cache
        .frame_images
        .entry((name.to_string(), srgb))
        .or_insert_with(|| {
            std::iter::once(first.clone())
                .chain((1..tex.frame_count()).map(|i| images.add(tex.frame(i).into_image(bc, srgb))))
                .collect()
        })
        .clone()
}

/// Per-mesh state for a proxied material on the fallback path: the
/// material (its own copy, so proxies can change it per entity) and its
/// proxies. `None` for materials without proxies we evaluate.
fn fallback_proxies(assets: &mut SourceMapAssets, map: &LoadedMap, m: &MaterialInfo, mat: &Handle<SourceMaterial>) -> Option<(Handle<SourceMaterial>, crate::proxy::FallbackProxies)> {
    if m.proxies.is_empty() {
        return None;
    }
    let frames = match (m.base_texture.as_deref(), assets.materials.get(mat).and_then(|x| x.base_texture.clone())) {
        (Some(name), Some(first)) => {
            let bc = bc_supported(assets);
            texture_frames(&mut assets.images, &mut assets.cache, bc, map, name, true, &first)
        }
        _ => Vec::new(),
    };
    let proxies = crate::proxy::FallbackProxies::new(m, frames)?;
    let own = assets.materials.get(mat)?.clone();
    Some((assets.materials.add(own), proxies))
}

/// A glass/water material and its proxies (if any).
type GlassBuilt = (Handle<crate::glass::GlassMaterial>, Option<crate::glass::GlassProxies>);

/// A `Refract` or `Water` material as [`crate::glass::GlassMaterial`].
/// `lightmap`: brushes (lit dirt/frames by the lightmap; models by vertex
/// colour); `sky`: what water reflects without a cubemap of its own.
fn glass_material(assets: &mut SourceMapAssets, map: &LoadedMap, m: &MaterialInfo, lightmap: Option<Handle<Image>>, sky: Option<&Handle<Image>>) -> GlassBuilt {
    use crate::glass::*;
    let bc = bc_supported(assets);
    let water = m.shader == "water";
    let (images, cache) = (&mut assets.images, &mut *assets.cache);
    let mut tex = |key: &str, srgb: bool| {
        crate::vmt::texture_param(&m.params, key)
            .filter(|t| t != "env_cubemap")
            .and_then(|t| valve_texture(images, cache, bc, map, &t, srgb))
    };
    let normal = normal_var(&m.params).and_then(|k| tex(k, false));
    let base = tex("$basetexture", true);
    let tint = tex("$refracttinttexture", true);
    let envmap = tex("$envmap", true).filter(|t| t.cube).map(|t| t.image).or_else(|| sky.filter(|_| water).cloned());
    let mut flags = if lightmap.is_some() { FLAG_LIGHTMAP } else { FLAG_VERTEX_LIGHT };
    for (on, f) in [(normal.is_some(), FLAG_NORMAL), (base.is_some(), FLAG_BASE), (tint.is_some(), FLAG_TINT_TEXTURE), (envmap.is_some(), FLAG_ENVMAP)] {
        if on {
            flags |= f;
        }
    }
    if water && lightmap.is_some() {
        flags |= FLAG_WATER_DEPTH;
    }
    let reflection = (water && reflects_world(&m.params)).then(|| assets.water_reflection.0.clone());
    if reflection.is_some() {
        flags |= FLAG_WORLD_REFLECT;
    }
    let params = glass_params(water, &crate::proxy::MaterialVars::of(&m.params), flags, assets.settings.light_scale);
    let program = crate::proxy::ProxyProgram::compile(&m.proxies);
    let proxies = (!program.is_empty()).then(|| GlassProxies {
        program: Arc::new(program),
        params: m.params.clone(),
        normal_frames: normal.as_ref().map(|t| t.frames.clone()).unwrap_or_default(),
        base_frames: base.as_ref().map(|t| t.frames.clone()).unwrap_or_default(),
    });
    let mat = GlassMaterial {
        params,
        normal_map: normal.map(|t| t.image),
        base_texture: base.map(|t| t.image),
        tint_texture: tint.map(|t| t.image),
        envmap,
        lightmap,
        reflection,
        double_sided: m.no_cull || std::env::var_os("SOURCE_NOCULL").is_some(),
    };
    (assets.glass.add(mat), proxies)
}

/// Per-map inputs for model surfaces on Valve shaders.
struct ValveCtx {
    fog: [f32; 4],
    fog_color: [f32; 3],
    /// Scale from our decoded vertex light to what the engine's VS decodes.
    light_scale: f32,
    fallbacks: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
}

/// A texture for the Valve path: (handle, sampler, is cube).
fn valve_texture(
    images: &mut Assets<Image>,
    cache: &mut SourceCache,
    bc: bool,
    map: &LoadedMap,
    name: &str,
    srgb: bool,
) -> Option<crate::valve::EnvTexture> {
    // `name#alpha`: that texture's alpha as an RGB mask (see VtfTexture::alpha_as_rgb).
    if let Some(src) = name.strip_suffix(crate::valve::ALPHA_MASK_SUFFIX) {
        let tex = map.textures.get(src)?;
        let sampler = crate::valve::SamplerKey::from_vtf_flags(tex.flags);
        let image = match cache.linear_images.get(name) {
            Some(h) => h.clone(),
            None => {
                let h = images.add(tex.alpha_as_rgb()?.into_image(bc, false));
                cache.linear_images.insert(name.to_string(), h.clone());
                h
            }
        };
        return Some(crate::valve::EnvTexture { image, sampler, cube: false, frames: Vec::new() });
    }
    let tex = map.textures.get(name)?;
    let sampler = crate::valve::SamplerKey::from_vtf_flags(tex.flags);
    let cube = tex.faces == 6;
    let table = if srgb { &mut cache.images } else { &mut cache.linear_images };
    let image = match table.get(name) {
        Some(h) => h.clone(),
        None => {
            let h = images.add(tex.frame(0).into_image(bc, srgb));
            table.insert(name.to_string(), h.clone());
            h
        }
    };
    let frames = texture_frames(images, cache, bc, map, name, srgb, &image);
    Some(crate::valve::EnvTexture { image, sampler, cube, frames })
}

fn fallback_image(images: &mut Assets<Image>, cache: &mut SourceCache, name: &'static str, rgba: [u8; 4], srgb: bool) -> Handle<Image> {
    cache
        .fallback_images
        .entry(name)
        .or_insert_with(|| {
            images.add(Image::new(
                Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                TextureDimension::D2,
                rgba.to_vec(),
                if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm },
                RenderAssetUsages::RENDER_WORLD,
            ))
        })
        .clone()
}

fn material_params(m: &MaterialInfo, flags: u32, light_scale: f32) -> (SourceMaterialParams, AlphaMode) {
    let mut flags = flags;
    let mut color = Vec4::new(m.color[0], m.color[1], m.color[2], m.alpha);
    let alpha_mode = match m.blend {
        BlendKind::Opaque => AlphaMode::Opaque,
        BlendKind::Mask => {
            flags |= material::FLAG_ALPHA_TEST;
            AlphaMode::Mask(m.alpha_cutoff)
        }
        BlendKind::Blend => AlphaMode::Blend,
        BlendKind::Additive => AlphaMode::Add,
    };
    if m.unlit {
        flags |= material::FLAG_UNLIT;
    }
    if m.base_texture2.is_some() {
        flags |= material::FLAG_BLEND2;
    }
    if m.is_glassy && m.base_texture.is_none() {
        // refraction/water without a base texture: faint tinted pane
        color = Vec4::new(0.6, 0.75, 0.8, 0.15);
        flags |= material::FLAG_IGNORE_TEX_ALPHA;
    }
    if m.shader == "solidenergy" {
        let [r, g, b] = solid_energy_tint(m);
        color = Vec4::new(r, g, b, 1.0);
        flags |= material::FLAG_GREEN_MASK;
    }
    let uv = crate::proxy::MaterialVars::of(&m.params).transform("$basetexturetransform");
    (
        SourceMaterialParams {
            color,
            light_scale,
            alpha_cutoff: m.alpha_cutoff,
            flags,
            pbr_flags: pbr_flags(alpha_mode),
            uv_transform_u: Vec4::new(uv[0], uv[1], uv[2], 0.0),
            uv_transform_v: Vec4::new(uv[3], uv[4], uv[5], 0.0),
        },
        alpha_mode,
    )
}

/// SolidEnergy (fizzler, light bridge edges) keeps its ripples in the base
/// texture's green channel and colours them with the flow colours; without
/// the flow shader, tint the ripples with a quarter of the vortex colour.
fn solid_energy_tint(m: &MaterialInfo) -> [f32; 3] {
    let vortex = m.params.get("$flow_vortex_color").and_then(crate::kv::parse_color).unwrap_or([0.64, 2.058, 2.56]);
    vortex.map(|c| c * 0.25)
}

fn pbr_flags(alpha_mode: AlphaMode) -> u32 {
    use bevy::pbr::StandardMaterialFlags as F;
    let mode = match alpha_mode {
        AlphaMode::Opaque => F::ALPHA_MODE_OPAQUE,
        AlphaMode::Mask(_) => F::ALPHA_MODE_MASK,
        AlphaMode::Blend => F::ALPHA_MODE_BLEND,
        AlphaMode::Premultiplied => F::ALPHA_MODE_PREMULTIPLIED,
        AlphaMode::Add => F::ALPHA_MODE_ADD,
        AlphaMode::Multiply => F::ALPHA_MODE_MULTIPLY,
        _ => F::ALPHA_MODE_OPAQUE,
    };
    (F::FOG_ENABLED | mode).bits()
}

fn make_material(
    assets: &mut SourceMapAssets,
    map: &LoadedMap,
    m: &MaterialInfo,
    flags: u32,
    lightmap: Option<Handle<Image>>,
) -> SourceMaterial {
    let light_scale = assets.settings.light_scale;
    let mut flags = if assets.settings.fullbright { flags | material::FLAG_UNLIT } else { flags };
    if std::env::var_os("SOURCE_LIGHT_ONLY").is_some() {
        flags |= material::FLAG_LIGHT_ONLY;
    }
    let (params, alpha_mode) = material_params(m, flags, light_scale);
    let base = m
        .base_texture
        .as_deref()
        .and_then(|t| texture_handle(assets, map, t))
        .or_else(|| Some(white_image(assets)));
    let base2 = m.base_texture2.as_deref().and_then(|t| texture_handle(assets, map, t));
    SourceMaterial {
        params,
        base_texture: base,
        lightmap,
        base_texture2: base2,
        proj_lights: assets.projected.lights.clone(),
        proj_cookies: Some(assets.projected.cookies.clone()),
        proj_shadows: Some(assets.projected.shadows.clone()),
        alpha_mode,
        double_sided: m.no_cull || std::env::var_os("SOURCE_NOCULL").is_some(),
    }
}

/// The world's opaque brush faces and displacements, for the depth below
/// map water ([`crate::glass::WaterFloor`]).
fn water_floor_of(map: &LoadedMap) -> crate::glass::WaterFloor {
    let mut tris = Vec::new();
    let mut add = |d: &MeshData| {
        for t in d.indices.chunks_exact(3) {
            tris.push([t[0], t[1], t[2]].map(|i| Vec3::from(d.positions[i as usize])));
        }
    };
    for bm in map.brush_meshes.iter().filter(|bm| bm.model == 0 && !crate::glass::is_glass(&map.materials[bm.material])) {
        add(&bm.mesh);
    }
    add(&map.displacement_collision);
    crate::glass::WaterFloor::new(tris, 1.0)
}

fn mesh_from_brush(d: &MeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, d.uv0.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, d.uv1.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, d.colors.clone())
        .with_inserted_indices(Indices::U32(d.indices.clone()))
}

fn lightmap_image(map: &LoadedMap) -> Image {
    let lm = &map.lightmap;
    let data: Vec<u8> = bytemuck::cast_slice(&lm.texels).to_vec();
    let mut img = Image::new(
        Extent3d { width: lm.width, height: lm.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgb9e5Ufloat,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    img
}

/// Build a cubemap from the six sky faces (Source order rt, lf, bk, ft, up, dn).
fn skybox_image(assets: &mut SourceMapAssets, map: &LoadedMap) -> Option<Handle<Image>> {
    let faces = map.sky_faces.as_ref()?;
    let mut decoded = Vec::new();
    for f in faces {
        let t = map.textures.get(f)?;
        let mut t = t.clone();
        t.mips.truncate(1);
        t.decompress();
        decoded.push(t);
    }
    // Every face must match the first in size and format.
    let (w, h, fmt) = (decoded[0].width, decoded[0].height, decoded[0].format);
    let size = w.max(h);
    if decoded.iter().any(|t| t.format != fmt) {
        return None;
    }
    let bpp = if fmt == crate::vtf::PixelFormat::Rgba16F { 8 } else { 4 };
    let resize = |t: &crate::vtf::VtfTexture| -> Vec<u8> {
        // nearest-neighbour resize to size x size (Portal 2 skies have half-height side faces)
        let mut out = vec![0u8; (size * size) as usize * bpp];
        for y in 0..size {
            let sy = (y * t.height / size).min(t.height - 1);
            for x in 0..size {
                let sx = (x * t.width / size).min(t.width - 1);
                let s = ((sy * t.width + sx) as usize) * bpp;
                let d = ((y * size + x) as usize) * bpp;
                out[d..d + bpp].copy_from_slice(&t.mips[0][s..s + bpp]);
            }
        }
        out
    };
    let rot = |px: Vec<u8>, quarter_turns: u32| -> Vec<u8> {
        let mut cur = px;
        for _ in 0..quarter_turns % 4 {
            let mut out = vec![0u8; cur.len()];
            for y in 0..size {
                for x in 0..size {
                    // rotate 90 degrees clockwise
                    let s = ((y * size + x) as usize) * bpp;
                    let d = ((x * size + (size - 1 - y)) as usize) * bpp;
                    out[d..d + bpp].copy_from_slice(&cur[s..s + bpp]);
                }
            }
            cur = out;
        }
        cur
    };
    let [rt, lf, bk, ft, up, dn] = [0, 1, 2, 3, 4, 5].map(|i| resize(&decoded[i]));
    // Source: ft looks down +X, bk -X, lf +Y, rt -Y, up +Z, dn -Z.
    // World: +X = Source +X, +Y = Source +Z, +Z = Source -Y.
    // Cubemap layer order: +X, -X, +Y, -Y, +Z, -Z.
    let layers = [ft, bk, rot(up, 1), rot(dn, 3), rt, lf];
    let data: Vec<u8> = layers.concat();
    let format = if bpp == 8 { TextureFormat::Rgba16Float } else { TextureFormat::Rgba8UnormSrgb };
    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 6 },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    Some(assets.images.add(img))
}

fn classify(e: &SourceEntity) -> MapEntityKind {
    match e.classname.as_str() {
        "info_player_start" => MapEntityKind::PlayerStart,
        "prop_weighted_cube" | "prop_monster_box" => MapEntityKind::WeightedCube,
        "prop_dynamic" | "prop_dynamic_override" | "prop_dynamic_ornament" => MapEntityKind::DynamicProp,
        "prop_physics" | "prop_physics_override" | "prop_physics_multiplayer" => MapEntityKind::PhysicsProp,
        "prop_testchamber_door" | "func_door" | "func_door_rotating" | "prop_door_rotating" => MapEntityKind::Door,
        "prop_floor_button" | "prop_floor_cube_button" | "prop_floor_ball_button" | "prop_button"
        | "prop_under_button" | "prop_under_floor_button" | "func_button" => MapEntityKind::Button,
        "light_environment" => MapEntityKind::LightEnvironment,
        c if c.starts_with("prop_") => MapEntityKind::OtherProp,
        _ if e.brush_model().is_some() => MapEntityKind::BrushEntity,
        _ => MapEntityKind::Other,
    }
}

fn parse_light(s: Option<&str>) -> (Vec3, f32) {
    let v: Vec<f32> = s
        .unwrap_or("255 255 255 200")
        .split_whitespace()
        .filter_map(crate::kv::parse_f32)
        .collect();
    let c = if v.len() >= 3 { Vec3::new(v[0], v[1], v[2]) / 255.0 } else { Vec3::ONE };
    let b = v.get(3).copied().unwrap_or(200.0);
    // Source colours are gamma space.
    (c.powf(2.2), b)
}

/// Spawn a converted map. Returns entity/player-start info in world space.
pub fn spawn_loaded_map(
    commands: &mut Commands,
    assets: &mut SourceMapAssets,
    map: &Arc<LoadedMap>,
    offset: Vec3,
) -> MapInfo {
    let root = commands
        .spawn((
            Name::new(format!("source map {}", map.name)),
            SourceMapRoot { name: map.name.clone() },
            Transform::from_translation(offset),
            Visibility::default(),
            RigidBody::Static,
        ))
        .id();

    let lightmap = assets.images.add(lightmap_image(map));

    // Parent entity per brush model (0 = world at the root itself).
    let mut model_parent: HashMap<usize, Entity> = HashMap::new();
    model_parent.insert(0, root);
    let mut model_entities = Vec::new();
    for (&m, &ei) in &map.brush_model_entity {
        let e = &map.entities[ei];
        let child = commands
            .spawn((
                Name::new(format!("{} *{m}", e.classname)),
                Transform::from_translation(source_to_world(e.origin)).with_rotation(source_angles_to_world(e.angles)),
                // Start-disabled brushes (func_brush & co.) wait hidden for
                // an Enable input; their colliders spawn disabled below.
                if map::starts_disabled(e) { Visibility::Hidden } else { Visibility::default() },
                ChildOf(root),
            ))
            .id();
        model_parent.insert(m, child);
        model_entities.push((m, ei, child));
    }

    // Map fog for the Valve shaders (range fog in Source units, linear colour).
    let (valve_fog, valve_fog_color) = crate::entities::map_fog_controller(&map.entities)
        .map(|e| {
            let c = (e.get_vec3("fogcolor").unwrap_or(Vec3::splat(128.0)) / 255.0).powf(2.2);
            (
                crate::valve::fog_params(
                    e.get_f32("fogstart").unwrap_or(0.0),
                    e.get_f32("fogend").unwrap_or(10000.0),
                    e.get_f32("fogmaxdensity").unwrap_or(1.0).clamp(0.0, 1.0),
                ),
                [c.x, c.y, c.z],
            )
        })
        .unwrap_or(([0.0, 1.0, 0.0, 0.0], [0.0; 3]));
    let bc = bc_supported(assets);
    let use_valve = assets.settings.valve_shaders && assets.game.fs.is_some();
    let mut valve_fallbacks: Vec<(String, String)> = Vec::new();
    let mut valve_done: HashMap<usize, Result<(), String>> = HashMap::new();

    // The sky, which water reflects when it has no cubemap of its own.
    let skybox = skybox_image(assets, map);
    // ...but not on a fogged map: there the sky is never seen (the view
    // ends in fog), and the white sky of sp_a1_intro3 turned its murky
    // water white.
    let fogged = map.entities.iter().any(|e| e.classname == "env_fog_controller" && e.get("fogenable") == Some("1"));
    let water_sky = skybox.clone().filter(|_| !fogged);

    // Brush meshes: one material per texdata for this map.
    let mut mats: HashMap<usize, Handle<SourceMaterial>> = HashMap::new();
    let mut glass_mats: HashMap<usize, GlassBuilt> = HashMap::new();
    let mut bounds = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let mut water_floor: Option<crate::glass::WaterFloor> = None;
    for bm in map.brush_meshes.iter().chain(map.overlays.iter()) {
        let parent = model_parent.get(&bm.model).copied().unwrap_or(root);
        if bm.model == 0 {
            for p in &bm.mesh.positions {
                let p = Vec3::from(*p) + offset;
                bounds.0 = bounds.0.min(p);
                bounds.1 = bounds.1.max(p);
            }
        }
        let m = &map.materials[bm.material];
        if std::env::var("P2_HIDE_MAT").is_ok_and(|h| h.split(',').any(|k| !k.is_empty() && m.name.contains(k))) {
            continue;
        }
        if std::env::var_os("P2_LIST_MATS").is_some() {
            info!("TRACE brush material {} ({}) model {}", m.name, m.shader, bm.model);
        }
        if crate::glass::is_glass(m) {
            let (h, proxies) = match glass_mats.get(&bm.material) {
                Some(g) => g.clone(),
                None => {
                    let g = glass_material(assets, map, m, Some(lightmap.clone()), water_sky.as_ref());
                    glass_mats.insert(bm.material, g.clone());
                    g
                }
            };
            // water: cut into a grid carrying the depth to the floor below
            let mut reflect_plane = None;
            let mesh = if m.shader == "water" {
                let floor = water_floor.get_or_insert_with(|| water_floor_of(map));
                let mut d = crate::glass::subdivide(&bm.mesh, crate::glass::WATER_GRID * shared::SOURCE_UNIT);
                for (p, c) in d.positions.iter().zip(&mut d.colors) {
                    c[3] = floor.depth_below(Vec3::from(*p), crate::glass::WATER_BANK * shared::SOURCE_UNIT).map_or(crate::glass::WATER_BOTTOMLESS, |d| d / shared::SOURCE_UNIT);
                }
                if std::env::var_os("P2_LIST_MATS").is_some() {
                    let ds: Vec<f32> = d.colors.iter().map(|c| c[3]).collect();
                    let y = d.positions.first().map_or(0.0, |p| p[1] / shared::SOURCE_UNIT);
                    let mean = ds.iter().filter(|&&x| x < crate::glass::WATER_BOTTOMLESS).sum::<f32>() / ds.len().max(1) as f32;
                    let deep = ds.iter().filter(|&&x| x >= crate::glass::WATER_BOTTOMLESS).count();
                    info!("TRACE water {} at z {y:.0}: {} verts, mean depth {mean:.1} units, {deep} bottomless", m.name, ds.len());
                }
                if crate::glass::reflects_world(&m.params) && !d.positions.is_empty() {
                    let y = d.positions.iter().map(|p| p[1]).sum::<f32>() / d.positions.len() as f32;
                    reflect_plane = Some(crate::glass::WorldReflectWater { height: y });
                }
                mesh_from_brush(&d)
            } else {
                mesh_from_brush(&bm.mesh)
            };
            let mut ec = commands.spawn((
                Name::new(format!("glass {}", m.name)),
                Mesh3d(assets.meshes.add(mesh)),
                MeshMaterial3d(h),
                Transform::IDENTITY,
                ChildOf(parent),
            ));
            if let Some(p) = proxies {
                ec.insert(p);
            }
            if let Some(w) = reflect_plane {
                ec.insert(w);
            }
            continue;
        }
        if use_valve && matches!(m.shader.as_str(), "lightmappedgeneric" | "worldvertextransition" | "unlitgeneric") && !m.is_glassy {
            let fs = assets.game.fs.clone().unwrap();
            let white = fallback_image(&mut assets.images, &mut assets.cache, "white", [255; 4], true);
            let black = fallback_image(&mut assets.images, &mut assets.cache, "black", [0, 0, 0, 255], true);
            let grey = fallback_image(&mut assets.images, &mut assets.cache, "grey", [128, 128, 128, 255], false);
            let flat = fallback_image(&mut assets.images, &mut assets.cache, "flat_normal", [128, 128, 255, 255], false);
            let images = &mut assets.images;
            let cache = &mut *assets.cache;
            let mut texture = |name: &str, srgb: bool| valve_texture(images, cache, bc, map, name, srgb);
            let mut env = crate::valve::MapShaderEnv {
                fs: &fs,
                fog: valve_fog,
                fog_color: valve_fog_color,
                lightmap: lightmap.clone(),
                texture: &mut texture,
                white,
                black,
                grey,
                flat_normal: flat,
            };
            let built = if m.shader == "unlitgeneric" {
                crate::valve::vertex_lit_generic(
                    &assets.valve,
                    &mut assets.shaders,
                    &mut env,
                    m,
                    crate::valve::brush_vertices(&bm.mesh),
                    bm.mesh.indices.clone(),
                )
            } else {
                crate::valve::lightmapped_generic(&assets.valve, &mut assets.shaders, &mut env, m, &bm.mesh)
            };
            match built {
                Ok(draw) => {
                    let mut ec = commands.spawn((
                        Name::new(format!("valve brush {}", m.name)),
                        crate::valve::ValveSurface(Arc::new(draw)),
                        Transform::IDENTITY,
                        Visibility::default(),
                        ChildOf(parent),
                    ));
                    if let Some(id) = bm.overlay_id {
                        ec.insert(crate::proxy::SourceOverlay { id });
                    }
                    valve_done.entry(bm.material).or_insert(Ok(()));
                    continue;
                }
                Err(e) => {
                    valve_done.entry(bm.material).or_insert(Err(e));
                }
            }
        } else if use_valve {
            valve_done.entry(bm.material).or_insert(Err(format!("shader {} not translated yet", m.shader)));
        }
        let handle = match mats.get(&bm.material) {
            Some(h) => h.clone(),
            None => {
                let m = &map.materials[bm.material];
                let mat = make_material(assets, map, m, material::FLAG_LIGHTMAP, Some(lightmap.clone()));
                let h = assets.materials.add(mat);
                mats.insert(bm.material, h.clone());
                h
            }
        };
        let proxied = fallback_proxies(assets, map, &map.materials[bm.material], &handle);
        let handle = proxied.as_ref().map_or(handle, |p| p.0.clone());
        let mut ec = commands.spawn((
            Name::new(format!("brush {}", map.materials[bm.material].name)),
            Mesh3d(assets.meshes.add(mesh_from_brush(&bm.mesh))),
            MeshMaterial3d(handle),
            Transform::IDENTITY,
            ChildOf(parent),
        ));
        if let Some((_, p)) = proxied {
            ec.insert(p);
        }
        if let Some(id) = bm.overlay_id {
            ec.insert(crate::proxy::SourceOverlay { id });
        }
    }

    for (mi, r) in &valve_done {
        if let Err(e) = r {
            valve_fallbacks.push((map.materials[*mi].name.clone(), e.clone()));
        }
    }
    valve_fallbacks.sort();
    if use_valve {
        info!(
            "source_assets: {}: {} brush materials on Valve shaders, {} on the fallback: {:?}",
            map.name,
            valve_done.values().filter(|r| r.is_ok()).count(),
            valve_fallbacks.len(),
            valve_fallbacks
        );
    }
    assets.cache.fallback_materials.insert(map.name.clone(), valve_fallbacks);

    // Colliders.
    for c in &map.colliders {
        let parent = model_parent.get(&(c.model as usize)).copied().unwrap_or(root);
        let off = map
            .brush_model_entity
            .get(&(c.model as usize))
            .is_some_and(|&ei| map::brush_starts_nonsolid(&map.entities[ei]));
        if let (Some(col), true) = (spawn_collider(commands, c, parent), off) {
            commands.entity(col).insert(ColliderDisabled);
        }
    }
    // Displacement terrain: one triangle mesh, not portalable.
    let dc = &map.displacement_collision;
    if dc.indices.len() >= 3 {
        let verts: Vec<Vec3> = dc.positions.iter().map(|p| Vec3::from(*p)).collect();
        let tris: Vec<[u32; 3]> = dc.indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
        commands.spawn((Name::new("displacement collision"), Collider::trimesh(verts, tris), Transform::IDENTITY, ChildOf(root)));
    }

    // Model surfaces on Valve shaders share these per-map inputs.
    let vhv_ctx = use_valve.then(|| ValveCtx {
        fog: valve_fog,
        fog_color: valve_fog_color,
        // our .vhv decode is 2*(b/255)^2.2; the engine's VS decodes (2b/255)^2.2
        light_scale: 2f32.powf(2.2) / 2.0,
        fallbacks: Default::default(),
    });
    let amb_ctx = use_valve.then(|| ValveCtx {
        fog: valve_fog,
        fog_color: valve_fog_color,
        light_scale: 1.0,
        fallbacks: Default::default(),
    });

    // Static props.
    let mut entities_out: Vec<MapEntity> = Vec::new();
    for sp in &map.static_props {
        let Some(model) = map.models.get(&sp.model) else { continue };
        let transform = Transform::from_translation(source_to_world(sp.origin))
            .with_rotation(source_angles_to_world(sp.angles));
        let (e, _) = spawn_model(
            commands,
            assets,
            map,
            model,
            sp.skin,
            None,
            transform,
            root,
            false,
            &|src_n: Vec3, vvd: u32| match &sp.vertex_light {
                Some(vl) => vl.get(vvd as usize).copied().unwrap_or(Vec3::ZERO),
                None => {
                    // Rotate the normal into Source world space for the ambient cube.
                    let q = source_angles_to_world(sp.angles);
                    let wn = q * source_dir_to_world(src_n);
                    sp.ambient.eval(Vec3::new(wn.x, -wn.z, wn.y))
                }
            },
            Vec3::from_slice(&sp.tint[..3]),
            ModelPose::Stored,
            if sp.vertex_light.is_some() { vhv_ctx.as_ref() } else { amb_ctx.as_ref() },
        );
        if sp.solid != 0 {
            spawn_prop_collider(commands, model, e, sp.solid as i32, None, &HashMap::new());
        }
        let idx = entities_out.len();
        commands.entity(e).insert(SourceEntityRef { kind: MapEntityKind::StaticProp, index: idx });
        entities_out.push(MapEntity {
            kind: MapEntityKind::StaticProp,
            classname: "prop_static".into(),
            targetname: None,
            transform: Transform { translation: transform.translation + offset, ..transform },
            model: Some(sp.model.clone()),
            brush_model: None,
            skin: sp.skin,
            entity: Some(e),
            bounds: Some(model_bounds(model)),
            keyvalues: Vec::new(),
        });
    }

    // Entity lump.
    let mut player_start = None;
    let mut light_env = None;
    let world_lights = Arc::new(map.world_lights());
    for (ei, e) in map.entities.iter().enumerate() {
        let kind = classify(e);
        let transform = Transform::from_translation(source_to_world(e.origin) + offset)
            .with_rotation(source_angles_to_world(e.angles));
        let mut spawned = None;
        let mut bounds_e = None;
        let model_path = map::entity_model(e);
        match kind {
            MapEntityKind::PlayerStart => {
                if player_start.is_none() || e.get("spawnflags").is_some_and(|f| f == "1") {
                    player_start = Some(
                        Transform::from_translation(source_to_world(e.origin) + offset)
                            .with_rotation(source_view_to_world(e.angles)),
                    );
                }
            }
            MapEntityKind::LightEnvironment => {
                let (color, brightness) = parse_light(e.get("_light"));
                let (ambient_color, ambient_brightness) = parse_light(e.get("_ambient"));
                let mut ang = e.angles;
                if let Some(p) = e.get_f32("pitch") {
                    ang.x = p;
                }
                light_env = Some(LightEnvironment {
                    direction: source_angles_to_world(ang) * Vec3::X,
                    color,
                    brightness,
                    ambient_color,
                    ambient_brightness,
                });
            }
            _ => {}
        }
        if let Some(path) = &model_path {
            if let Some(model) = map.models.get(path) {
                let skin = if e.classname == "prop_weighted_cube" { crate::map::cube_skin(e.get("cubetype"), e.get("newskins"), e.get("skin")) } else { e.get("skin").or(e.get("modelskin")).and_then(|s| s.parse().ok()).unwrap_or(0) };
                // Cubes also get their lit skin, shown while on a button.
                let alt_skin = if e.classname == "prop_weighted_cube" { crate::map::cube_lit_skin(path, skin) } else { None };
                let scale = e.get_f32("modelscale").filter(|s| *s > 0.0).unwrap_or(1.0);
                let local = Transform::from_translation(source_to_world(e.origin))
                    .with_rotation(source_angles_to_world(e.angles))
                    .with_scale(Vec3::splat(scale));
                let rot = source_angles_to_world(e.angles);
                // Source lights models at their illumposition, rotated with the entity.
                let q_src = Quat::from_rotation_z(e.angles.y.to_radians())
                    * Quat::from_rotation_y(e.angles.x.to_radians())
                    * Quat::from_rotation_x(e.angles.z.to_radians());
                let relit = RELIT_CLASSES.contains(&e.classname.as_str());
                let amb = map.lit_ambient_at(e.origin + q_src * model.illum_position * scale, &world_lights);
                if std::env::var_os("P2_TRACE_LIGHT").is_some() {
                    let avg: Vec3 = amb.sides.iter().copied().sum::<Vec3>() / 6.0;
                    info!("TRACE light {} {:?} illum {:?} -> {:?} avg {:.3?}", e.classname, e.targetname, model.illum_position, e.origin + q_src * model.illum_position * scale, avg);
                }
                let tint = e
                    .get_vec3("rendercolor")
                    .map(|c| (c / 255.0).powf(2.2))
                    .unwrap_or(Vec3::ONE);
                let seq = e.get("defaultanim").filter(|s| !s.is_empty()).or_else(|| map::class_default_sequence(&e.classname));
                let pose = if map::class_animates(&e.classname) && model.is_animated() {
                    ModelPose::Animated(seq)
                } else {
                    ModelPose::Posed(seq)
                };
                let (ent, parts) = spawn_model(
                    commands,
                    assets,
                    map,
                    model,
                    skin,
                    alt_skin,
                    local,
                    root,
                    relit,
                    &|src_n: Vec3, _| {
                        let wn = rot * source_dir_to_world(src_n);
                        amb.eval(Vec3::new(wn.x, -wn.z, wn.y))
                    },
                    tint,
                    pose,
                    amb_ctx.as_ref(),
                );
                if let Some(alt) = alt_skin {
                    commands.entity(ent).insert((ModelSkin(skin), AltSkin { base: skin, alt }));
                }
                if let (true, Some(v)) = (relit, amb_ctx.as_ref()) {
                    commands.entity(ent).insert(RelightOnMove {
                        map: map.clone(),
                        lights: world_lights.clone(),
                        illum: model.illum_position * scale,
                        offset,
                        tint,
                        light_scale: v.light_scale,
                        last: None,
                    });
                }
                if e.get("startdisabled") == Some("1") || e.get("rendermode") == Some("10") || e.classname == "env_shooter" {
                    commands.entity(ent).insert(Visibility::Hidden);
                }
                if let Some(tn) = &e.targetname {
                    commands.entity(ent).insert(Name::new(format!("{} {tn}", e.classname)));
                }
                // Entity props collide too unless `solid` is 0 (not solid).
                let solid = if e.classname == "env_shooter" { 0 } else { e.get("solid").and_then(|v| v.trim().parse().ok()).unwrap_or(6) };
                if solid != 0 {
                    spawn_prop_collider(commands, model, ent, solid, seq, &parts);
                }
                spawned = Some(ent);
                bounds_e = Some(model_bounds(model));
            }
        }
        if spawned.is_none() {
            if let Some(m) = e.brush_model() {
                spawned = model_entities.iter().find(|x| x.0 == m).map(|x| x.2);
            }
        }
        // Logic entities (relays, triggers, counters...) are kept too: a game
        // runs the level's I/O on them.
        let idx = entities_out.len();
        if let Some(s) = spawned {
            commands.entity(s).insert(SourceEntityRef { kind, index: idx });
        }
        entities_out.push(MapEntity {
            kind,
            classname: e.classname.clone(),
            targetname: e.targetname.clone(),
            transform,
            model: model_path,
            brush_model: e.brush_model(),
            skin: if e.classname == "prop_weighted_cube" { crate::map::cube_skin(e.get("cubetype"), e.get("newskins"), e.get("skin")) } else { e.get("skin").or(e.get("modelskin")).and_then(|s| s.parse().ok()).unwrap_or(0) },
            entity: spawned,
            bounds: bounds_e,
            keyvalues: e.keyvalues.clone(),
        });
        let _ = ei;
    }

    let arrival = map
        .entities
        .iter()
        .find(|e| {
            e.classname == "point_teleport"
                && e.targetname.as_deref().is_some_and(|t| t.to_ascii_lowercase().contains("arrival_teleport"))
        })
        .map(|e| {
            Transform::from_translation(source_to_world(e.origin) + offset)
                .with_rotation(source_view_to_world(e.angles))
        });
    let fog = crate::entities::map_fog_controller(&map.entities)
        .map(|e| {
            let c = e.get_vec3("fogcolor").unwrap_or(Vec3::splat(128.0)) / 255.0;
            let c = c.powf(2.2);
            MapFog {
                color: Color::linear_rgb(c.x, c.y, c.z),
                start: e.get_f32("fogstart").unwrap_or(0.0) * shared::SOURCE_UNIT,
                end: e.get_f32("fogend").unwrap_or(10000.0) * shared::SOURCE_UNIT,
                max_density: e.get_f32("fogmaxdensity").unwrap_or(1.0).clamp(0.0, 1.0),
            }
        });
    if use_valve {
        let mut fb: std::collections::BTreeMap<String, String> = Default::default();
        for c in [&vhv_ctx, &amb_ctx].into_iter().flatten() {
            fb.extend(c.fallbacks.lock().unwrap().clone());
        }
        info!("source_assets: {}: {} model materials on the fallback: {:?}", map.name, fb.len(), fb);
        if let Some(list) = assets.cache.fallback_materials.get_mut(&map.name) {
            list.extend(fb.into_iter());
        }
    }
    if bounds.0.x > bounds.1.x {
        bounds = (offset, offset);
    }
    MapInfo {
        name: map.name.clone(),
        root,
        offset,
        player_start: player_start.unwrap_or(Transform::from_translation(offset)),
        arrival: arrival
            .or(player_start)
            .unwrap_or(Transform::from_translation(offset)),
        entities: entities_out,
        light_environment: light_env,
        fog,
        skybox,
        skyname: map.skyname.clone(),
        bounds,
        convert_seconds: 0.0,
    }
}

fn model_bounds(model: &ModelData) -> (Vec3, Vec3) {
    let a = source_to_world(model.mins);
    let b = source_to_world(model.maxs);
    (a.min(b), a.max(b))
}

/// The collision layer of player clips: queries that should see through
/// them (lasers) mask it out (`LayerMask(!PLAYER_CLIP_LAYER)`).
pub const PLAYER_CLIP_LAYER: u32 = 1 << 1;

fn spawn_collider(commands: &mut Commands, c: &ColliderData, parent: Entity) -> Option<Entity> {
    let collider = if c.pieces.len() == 1 {
        Collider::convex_hull(c.pieces[0].clone())
    } else {
        let parts: Vec<(Vec3, Quat, Collider)> = c
            .pieces
            .iter()
            .filter_map(|p| Collider::convex_hull(p.clone()).map(|col| (Vec3::ZERO, Quat::IDENTITY, col)))
            .collect();
        (!parts.is_empty()).then(|| Collider::compound(parts))
    };
    let collider = collider?;
    let mut ec = commands.spawn((
        Name::new(if c.portalable { "portalable brush" } else { "brush" }),
        collider,
        Transform::IDENTITY,
        SourceCollider {
            contents: c.contents,
            brush: c.brush,
            model: c.model,
            sides: c.sides.clone(),
        },
        ChildOf(parent),
    ));
    if c.portalable {
        ec.insert(Portalable);
    }
    if c.shot_only {
        // Nothing collides with a sensor; the gun stops at portalable ones.
        ec.insert(Sensor);
    }
    if c.shot_passthrough() {
        ec.insert(PortalShotPassthrough);
    }
    if c.player_clip_only() {
        // Player clips (toolsclip, toolsplayerclip) only stop the player,
        // whose movement is a shape cast: physics objects fall through them
        // (cube droppers have one across their chute).
        ec.insert(CollisionLayers::new(LayerMask(PLAYER_CLIP_LAYER), LayerMask::NONE));
    }
    Some(ec.id())
}


/// Prop collision: the model's `.phy` convex pieces; its bounding box only
/// for `solid 2` (SOLID_BBOX). A `solid 6` (VPhysics) model without a
/// `.phy` is not solid, as in the engine.
///
/// For a model drawn as bone parts (`bone_parts`: bone -> part entity),
/// each per-bone solid goes, unposed (it is stored in its bone's space),
/// on its bone's part, so it follows the animation.
fn spawn_prop_collider(
    commands: &mut Commands,
    model: &ModelData,
    parent: Entity,
    solid: i32,
    sequence: Option<&str>,
    bone_parts: &HashMap<usize, Entity>,
) {
    // Grate models (`$contents grate`: handrails) stop the player but not
    // portal shots, as grate brushes (T-130).
    let shot_passthrough = crate::collision::contents_shot_passthrough(model.contents);
    let compound = |pieces: Vec<&Vec<Vec3>>| -> Option<Collider> {
        let parts: Vec<(Vec3, Quat, Collider)> = pieces
            .into_iter()
            .flat_map(|p| crate::collision::contact_hulls(p))
            .filter_map(|p| Collider::convex_hull(p).map(|c| (Vec3::ZERO, Quat::IDENTITY, c)))
            .collect();
        match parts.len() {
            0 => None,
            1 => parts.into_iter().next().map(|p| p.2),
            _ => Some(Collider::compound(parts)),
        }
    };
    let collider = if !model.physics.is_empty() && !bone_parts.is_empty() {
        let mut by_bone: std::collections::BTreeMap<usize, Vec<&Vec<Vec3>>> = Default::default();
        let mut rest = Vec::new();
        for (piece, bone) in model.physics.iter().zip(&model.physics_bones) {
            match bone.filter(|b| bone_parts.contains_key(b)) {
                Some(b) => by_bone.entry(b).or_default().push(piece),
                None => rest.push(piece),
            }
        }
        for (b, pieces) in by_bone {
            if let Some(c) = compound(pieces) {
                let mut ec = commands.spawn((Name::new("prop collider"), c, Transform::IDENTITY, ChildOf(bone_parts[&b])));
                if shot_passthrough {
                    ec.insert(PortalShotPassthrough);
                }
            }
        }
        compound(rest)
    } else if !model.physics.is_empty() {
        let posed = model.posed_physics(sequence);
        compound(posed.iter().collect())
    } else if solid == 2 {
        let (lo, hi) = model_bounds(model);
        let size = hi - lo;
        (size.min_element() > 0.01).then(|| {
            Collider::compound(vec![((lo + hi) * 0.5, Quat::IDENTITY, Collider::cuboid(size.x, size.y, size.z))])
        })
    } else {
        None
    };
    if let Some(c) = collider {
        let mut ec = commands.spawn((Name::new("prop collider"), c, Transform::IDENTITY, ChildOf(parent)));
        if shot_passthrough {
            ec.insert(PortalShotPassthrough);
        }
    }
}

/// How [`spawn_model`] poses a model's vertices.
#[derive(Clone, Copy, Debug)]
enum ModelPose<'a> {
    /// As stored (static props: bones collapsed).
    Stored,
    /// Baked into frame 0 of a sequence (its name, or sequence 0).
    Posed(Option<&'a str>),
    /// Rigid per-bone parts ([`BonePart`]) under an [`AnimatedModel`],
    /// starting in frame 0 of the sequence.
    Animated(Option<&'a str>),
}

/// Spawns a model's meshes under a new entity at `transform`; returns it and,
/// for [`ModelPose::Animated`], its bone part entities by bone.
#[allow(clippy::too_many_arguments)]
fn spawn_model(
    commands: &mut Commands,
    assets: &mut SourceMapAssets,
    map: &LoadedMap,
    model: &ModelData,
    skin: i32,
    alt_skin: Option<i32>,
    transform: Transform,
    parent: Entity,
    relit: bool,
    light: &dyn Fn(Vec3, u32) -> Vec3,
    tint: Vec3,
    pose: ModelPose,
    valve: Option<&ValveCtx>,
) -> (Entity, HashMap<usize, Entity>) {
    let e = commands
        .spawn((
            Name::new(model.path.clone()),
            transform,
            Visibility::default(),
            ChildOf(parent),
        ))
        .id();
    use std::borrow::Cow;
    // Eyeball meshes get their iris projected (their stored UVs are unused).
    fn eyes<'a>(model: &ModelData, m: Cow<'a, crate::models::ModelMesh>) -> Cow<'a, crate::models::ModelMesh> {
        match model.iris_uvs(&m) {
            Some(uvs) => Cow::Owned(crate::models::ModelMesh { uvs, ..m.into_owned() }),
            None => m,
        }
    }
    let mut parts = HashMap::new();
    // (mesh, light normals if not its own source normals, parent entity,
    // per-vertex (bone part, bone-space position, normal) of a stretch part)
    type Stretch = Option<Vec<(Entity, Vec3, Vec3)>>;
    let draws: Vec<(Cow<crate::models::ModelMesh>, Option<Vec<Vec3>>, Entity, Stretch)> = match pose {
        ModelPose::Stored => model.meshes.iter().map(|m| (eyes(model, Cow::Borrowed(m)), None, e, None)).collect(),
        ModelPose::Posed(seq) => match model.posed_meshes(seq) {
            Cow::Borrowed(ms) => ms.iter().map(|m| (eyes(model, Cow::Borrowed(m)), None, e, None)).collect(),
            Cow::Owned(ms) => ms.into_iter().map(|m| (eyes(model, Cow::Owned(m)), None, e, None)).collect(),
        },
        ModelPose::Animated(seq) => {
            let sk = model.skeleton.as_ref().expect("animated models have a skeleton");
            let bone_to_model = sk.bone_to_model(seq.and_then(|n| sk.sequence_index(n)).or(Some(0)));
            let pose_to_bone: Vec<_> = sk.bones.iter().map(|b| b.pose_to_bone).collect();
            let split: Vec<_> = model
                .meshes
                .iter()
                .flat_map(|m| crate::models::split_by_bone(&eyes(model, Cow::Borrowed(m)), &pose_to_bone, &bone_to_model))
                .collect();
            let bones: std::collections::BTreeSet<usize> = split
                .iter()
                .flat_map(|p| std::iter::once(p.bone).chain(p.stretch.iter().map(|s| s.0)))
                .chain(model.physics_bones.iter().flatten().copied())
                .filter(|&b| b < sk.bones.len())
                .collect();
            for b in bones {
                let part = commands
                    .spawn((
                        Name::new(format!("bone {}", sk.bones[b].name)),
                        BonePart { bone: b as u16 },
                        crate::models::bone_transform(bone_to_model[b]),
                        Visibility::default(),
                        ChildOf(e),
                    ))
                    .id();
                parts.insert(b, part);
            }
            commands.entity(e).insert(AnimatedModel { model: model.path.clone() });
            split
                .into_iter()
                .map(|p| {
                    let parent = parts.get(&p.bone).copied().unwrap_or(e);
                    let stretch = (!p.stretch.is_empty()).then(|| p.stretch.iter().map(|&(b, pos, n)| (parts.get(&b).copied().unwrap_or(e), pos, n)).collect());
                    (Cow::Owned(p.mesh), Some(p.light_normals), parent, stretch)
                })
                .collect()
        }
    };
    // Each draw with its skin; where `alt_skin` changes a draw's material,
    // both skins' draws go in their own SkinVariant group (alt hidden).
    let mut skinned = Vec::new();
    for (i, d) in draws.iter().enumerate() {
        let mi = d.0.material_index;
        match alt_skin.filter(|&a| model.material_for(a, mi) != model.material_for(skin, mi)) {
            Some(a) => {
                skinned.push((i, skin, Some(Visibility::Inherited)));
                skinned.push((i, a, Some(Visibility::Hidden)));
            }
            None => skinned.push((i, skin, None)),
        }
    }
    for (di, skin, variant) in skinned {
        let (mesh, light_normals, e, stretch) = &draws[di];
        let (mesh, mut e) = (&**mesh, *e);
        if let Some(vis) = variant {
            e = commands.spawn((SkinVariant(skin), Transform::IDENTITY, vis, ChildOf(e))).id();
        }
        let normals = light_normals.as_deref().unwrap_or(&mesh.source_normals);
        if std::env::var("P2_TRACE_MESH").is_ok_and(|k| model.path.contains(&k)) {
            let (lo, hi) = mesh.positions.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p))));
            info!("TRACE mesh {} skin {skin} idx {} mat {:?} tris {} lo {lo} hi {hi}", model.path, mesh.material_index, model.material_for(skin, mesh.material_index), mesh.indices.len() / 3);
        }
        let Some(mat_name) = model.material_for(skin, mesh.material_index) else { continue };
        let Some(mi) = map.material_id(mat_name) else { continue };
        let minfo = &map.materials[mi];
        if !minfo.is_drawn() {
            continue;
        }
        // Debug: P2_HIDE_MAT=a,b skips model materials whose name contains one.
        if std::env::var("P2_HIDE_MAT").is_ok_and(|h| h.split(',').any(|k| !k.is_empty() && minfo.name.contains(k))) {
            continue;
        }
        if let Some(v) = valve {
            if matches!(minfo.shader.as_str(), "vertexlitgeneric" | "unlitgeneric") {
                let verts = crate::valve::model_vertices(mesh, &|i| {
                    let l = light(normals[i], mesh.vvd_index[i]) * tint * v.light_scale;
                    crate::valve::encode_static_light(l)
                });
                let fs = assets.game.fs.clone().unwrap();
                let bc = bc_supported(assets);
                let white = fallback_image(&mut assets.images, &mut assets.cache, "white", [255; 4], true);
                let black = fallback_image(&mut assets.images, &mut assets.cache, "black", [0, 0, 0, 255], true);
                let grey = fallback_image(&mut assets.images, &mut assets.cache, "grey", [128, 128, 128, 255], false);
                let flat = fallback_image(&mut assets.images, &mut assets.cache, "flat_normal", [128, 128, 255, 255], false);
                let images = &mut assets.images;
                let cache = &mut *assets.cache;
                let mut texture = |name: &str, srgb: bool| valve_texture(images, cache, bc, map, name, srgb);
                let mut env = crate::valve::MapShaderEnv {
                    fs: &fs,
                    fog: v.fog,
                    fog_color: v.fog_color,
                    lightmap: white.clone(),
                    texture: &mut texture,
                    white,
                    black,
                    grey,
                    flat_normal: flat,
                };
                match crate::valve::vertex_lit_generic(&assets.valve, &mut assets.shaders, &mut env, minfo, verts, mesh.indices.clone()) {
                    Ok(draw) => {
                        let mut ec = commands.spawn((
                            crate::valve::ValveSurface(Arc::new(draw)),
                            Transform::IDENTITY,
                            Visibility::default(),
                            ChildOf(e),
                        ));
                        if relit {
                            ec.insert(RelightNormals(Arc::new(normals.to_vec())));
                        }
                        if let Some(st) = stretch {
                            ec.insert(StretchSurface { verts: Arc::new(st.clone()) });
                        }
                        continue;
                    }
                    Err(err) => {
                        v.fallbacks.lock().unwrap().insert(minfo.name.clone(), err);
                    }
                }
            } else if !crate::glass::is_glass(minfo) {
                v.fallbacks
                    .lock()
                    .unwrap()
                    .insert(minfo.name.clone(), format!("shader {} not translated yet", minfo.shader));
            }
        }
        let key = (minfo.name.clone(), true);
        let mat = match assets.cache.prop_materials.get(&key) {
            Some(h) => h.clone(),
            None => {
                let m = make_material(assets, map, minfo, material::FLAG_VERTEX_LIGHT, None);
                let h = assets.materials.add(m);
                assets.cache.prop_materials.insert(key, h.clone());
                h
            }
        };
        let colors: Vec<[f32; 4]> = normals
            .iter()
            .zip(&mesh.vvd_index)
            .map(|(&n, &vi)| {
                let l = light(n, vi) * tint;
                [l.x, l.y, l.z, 1.0]
            })
            .collect();
        let m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh.positions.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.normals.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh.uvs.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
            .with_inserted_indices(Indices::U32(mesh.indices.clone()));
        if crate::glass::is_glass(minfo) {
            let (h, proxies) = glass_material(assets, map, minfo, None, None);
            let mut ec = commands.spawn((Mesh3d(assets.meshes.add(m)), MeshMaterial3d(h), Transform::IDENTITY, ChildOf(e)));
            if let Some(p) = proxies {
                ec.insert(p);
            }
            continue;
        }
        let proxied = fallback_proxies(assets, map, minfo, &mat);
        let mat = proxied.as_ref().map_or(mat, |p| p.0.clone());
        let mut ec = commands.spawn((
            Mesh3d(assets.meshes.add(m)),
            MeshMaterial3d(mat),
            Transform::IDENTITY,
            ChildOf(e),
        ));
        if let Some((_, p)) = proxied {
            ec.insert(p);
        }
    }
    (e, parts)
}
