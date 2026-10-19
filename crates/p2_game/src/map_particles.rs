//! The map's particle entities:
//!
//! * `info_particle_system`: the game's own `.pcf` effect named by
//!   `effect_name` (every file in `particles/particles_manifest.txt` and the
//!   map's own `maps/<map>_particles.txt` is loaded), run by `p2_fx`. CP0 is
//!   the entity (it follows its `parentname` parent), `cpoint1`..`cpoint63`
//!   name the entities other control points sit on, and `cpointN_parent`
//!   keeps control point N where it started relative to control point M.
//!   `start_active`, and the inputs `Start`, `Stop` (no new particles, the
//!   rest finish), `StopPlayEndCap`, `DestroyImmediately`, `Kill`.
//! * `env_spark`: bursts of sparks every 0.1 s + up to `MaxDelay` s while on
//!   (spawnflag 64 starts it on), `Magnitude` and `TrailLength` size them,
//!   spawnflag 128 adds a glow, 256 makes it silent (else `DoSpark`), 512
//!   sends them along the entity's forward instead of every way. Inputs
//!   `StartSpark`, `StopSpark`, `ToggleSpark`, `SparkOnce`.
//! * `env_steam`: a jet of puffs (`Speed`, `SpreadSpeed`, `StartSize`,
//!   `EndSize`, `Rate`, `JetLength`, `rendercolor`, `renderamt`,
//!   `rollspeed`, `InitialState`; spawnflag 1 emissive). `type` 1 (heat
//!   wave) is a refraction effect and isn't drawn. Inputs `TurnOn`, `TurnOff`,
//!   `Toggle`, and `JetLength`/`Rate`/`Speed`/`SpreadSpeed` with a value.
//!   Source's steam makes no sound of its own.
//!
//! Sparks and steam are Source's hard-coded effects (not `.pcf`), so they are
//! drawn here: one camera-facing mesh per emitter, rebuilt while it has
//! particles.
//!
//! Cost: emitters farther than [`CULL_DISTANCE`] or well behind the camera
//! are paused and hidden, at most [`MAX_VISIBLE_EFFECTS`] `.pcf` effects
//! run at once (nearest first), and each spark/steam emitter keeps at most
//! [`MAX_SPARKS`] / [`MAX_PUFFS`] particles.
use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::Mat3;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use p2_core::*;
use p2_fx::{ParticleEffect, Particles};

use crate::audio::PlayGameSound;
use crate::io::{EntInput, Ents};
use crate::level::{CurrentMap, LevelLoaded, LevelOwned};

pub struct MapParticlesPlugin;

impl Plugin for MapParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BaseLibrary>().add_systems(
            Update,
            (setup_on_load, particle_inputs, run_systems, sparks, steam).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic),
        );
    }
}

/// Emitters farther than this from the camera pause and hide, metres
/// (~2400 u: beyond it the game's own effects are a few pixels).
pub const CULL_DISTANCE: f32 = 60.0;
/// Emitters behind the camera hide once farther than this, metres (closer
/// ones may still reach into view).
pub const BEHIND_GRACE: f32 = 10.0;
/// Most `.pcf` map effects simulated at once; the nearest win.
pub const MAX_VISIBLE_EFFECTS: usize = 64;
pub const MAX_SPARKS: usize = 96;
pub const MAX_PUFFS: usize = 120;
const U: f32 = SOURCE_UNIT;
/// `sv_gravity`, u/s^2: sparks fall like everything else.
const GRAVITY: f32 = 600.0;

/// The particle library without any map's own files, to go back to on load.
#[derive(Resource, Default)]
struct BaseLibrary(Option<Arc<p2_fx::ParticleLibrary>>);

/// An entity colour keyvalue, `"R G B"` 0..255, as 0..1 (missing parts are 255).
pub fn entity_color(v: &str) -> [f32; 3] {
    let n: Vec<f32> = v.split_whitespace().filter_map(|x| x.parse::<f32>().ok()).collect();
    [0, 1, 2].map(|i| (n.get(i).copied().unwrap_or(255.0) / 255.0).clamp(0.0, 1.0))
}

/// The world basis of an entity's Source axes (forward X, left Y, up Z).
pub fn source_basis(rotation: Quat) -> Mat3 {
    Mat3::from_cols(rotation * Vec3::X, rotation * Vec3::NEG_Z, rotation * Vec3::Y)
}

/// `cpoint1`..`cpoint63`: (control point, target name).
pub fn control_point_targets(get: &dyn Fn(&str) -> Option<String>) -> Vec<(usize, String)> {
    (1..64).filter_map(|i| get(&format!("cpoint{i}")).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).map(|v| (i, v))).collect()
}

/// `cpointN_parent`: (control point, parent control point), non-zero only.
pub fn control_point_parents(get: &dyn Fn(&str) -> Option<String>) -> Vec<(usize, usize)> {
    (1..64)
        .filter_map(|i| {
            let p = get(&format!("cpoint{i}_parent"))?.trim().parse::<f32>().ok()? as i64;
            (p > 0 && p < 64 && p as usize != i).then_some((i, p as usize))
        })
        .collect()
}

/// Which emitters may run: within [`CULL_DISTANCE`], not well behind the
/// camera, nearest [`MAX_VISIBLE_EFFECTS`]. `at` are emitter positions.
pub fn visible_set(cam: Vec3, forward: Vec3, at: &[Vec3], cap: usize) -> Vec<bool> {
    let mut ok: Vec<(usize, f32)> = at
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let d = *p - cam;
            let dist = d.length();
            let behind = dist > BEHIND_GRACE && d.dot(forward) < -0.2 * dist;
            (dist <= CULL_DISTANCE && !behind).then_some((i, dist))
        })
        .collect();
    ok.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut out = vec![false; at.len()];
    for (i, _) in ok.into_iter().take(cap) {
        out[i] = true;
    }
    out
}

/// An `info_particle_system` and its running effect.
#[derive(Component)]
struct MapSystem {
    ent: usize,
    effect: String,
    cps: Vec<(usize, Vec<usize>)>,
    cp_parents: Vec<(usize, usize)>,
    /// Control point offsets in their parent control point's frame (set on start).
    cp_offsets: HashMap<usize, Vec3>,
    /// The entity it follows, and where it sits in that entity's frame.
    parent: Option<(usize, Transform)>,
    running: Option<Entity>,
}

/// Small deterministic random numbers for sparks and steam.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 1_000_000) as f32 / 1_000_000.0
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
    pub fn unit(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let t = self.range(0.0, std::f32::consts::TAU);
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * t.cos(), z, r * t.sin())
    }
}

fn kv(ents: &Ents, i: usize) -> impl Fn(&str) -> Option<String> + '_ {
    move |k: &str| ents.list[i].get(k).map(str::to_string)
}

fn world_tf(ents: &Ents, i: usize, globals: &Query<&GlobalTransform>) -> Transform {
    ents.list[i].entity.and_then(|e| globals.get(e).ok()).map(|g| g.compute_transform()).unwrap_or(ents.list[i].transform)
}

/// On a new level: the map's own particle files, and an emitter per entity.
#[allow(clippy::too_many_arguments)]
fn setup_on_load(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    game: Res<p2_assets::GameFsRes>,
    particles: Option<ResMut<Particles>>,
    mut base: ResMut<BaseLibrary>,
    map: Option<Res<CurrentMap>>,
) {
    let Some(ev) = loaded.read().last().cloned() else { return };
    let (Some(ents), Some(mut particles)) = (ents, particles) else { return };
    // The map's own manifest adds (or overrides) systems for this map only.
    let base_lib = base.0.get_or_insert_with(|| particles.library.clone()).clone();
    let name = map.as_ref().map_or(ev.map.clone(), |m| m.info.name.clone());
    let manifest = format!("maps/{name}_particles.txt");
    particles.library = match game.read(&manifest) {
        Some(bytes) => {
            let mut lib = (*base_lib).clone();
            let (n, problems) = lib.load_manifest(&String::from_utf8_lossy(&bytes), &|f| game.read(f));
            info!("portal2: {manifest}: {n} particle file(s)");
            for p in problems {
                warn!("portal2: {manifest}: {p}");
            }
            Arc::new(lib)
        }
        None => base_lib,
    };
    if ev.menu {
        return;
    }
    let mut rng = Rng::new(0x51ab_c0de);
    for (i, e) in ents.list.iter().enumerate() {
        let get = kv(&ents, i);
        match e.class.as_str() {
            "info_particle_system" => {
                let effect = get("effect_name").unwrap_or_default();
                if effect.is_empty() {
                    continue;
                }
                let cps = control_point_targets(&get).into_iter().map(|(cp, name)| (cp, ents.find(&name, i, None))).collect();
                let parent = get("parentname").map(|p| p.split(',').next().unwrap_or("").to_string()).filter(|p| !p.is_empty()).and_then(|p| ents.find(&p, i, None).into_iter().next()).map(|p| {
                    let local = ents.list[p].transform.compute_affine().inverse() * e.transform.compute_affine();
                    (p, Transform::from_matrix(local.into()))
                });
                let sys = MapSystem { ent: i, effect, cps, cp_parents: control_point_parents(&get), cp_offsets: HashMap::new(), parent, running: None };
                let start = get("start_active").is_some_and(|v| v.trim() == "1");
                let id = commands.spawn((Name::new(format!("info_particle_system {}", e.name)), sys, LevelOwned)).id();
                if start {
                    commands.entity(id).insert(StartNow);
                }
            }
            "env_spark" => {
                let f = |k: &str, d: f32| get(k).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(d);
                let sp = SparkEmitter {
                    ent: i,
                    on: e.spawnflags & 64 != 0,
                    max_delay: f("maxdelay", 0.0),
                    magnitude: f("magnitude", 1.0).max(1.0),
                    trail: f("traillength", 1.0).clamp(1.0, 3.0),
                    glow: e.spawnflags & 128 != 0,
                    silent: e.spawnflags & 256 != 0,
                    directional: e.spawnflags & 512 != 0,
                    next_at: 0.0,
                    sparks: Vec::new(),
                    glow_left: 0.0,
                    rng: Rng::new(rng.next().to_bits() ^ i as u32),
                    once: false,
                };
                commands.spawn((Name::new(format!("env_spark {}", e.name)), sp, Transform::IDENTITY, Visibility::default(), LevelOwned));
            }
            "env_steam" => {
                let st = SteamJet::from_keyvalues(i, e.spawnflags, &get, Rng::new(rng.next().to_bits() ^ (i as u32) << 3));
                commands.spawn((Name::new(format!("env_steam {}", e.name)), st, Transform::IDENTITY, Visibility::default(), LevelOwned));
            }
            _ => {}
        }
    }
}

/// Start this system on the next update (`start_active`).
#[derive(Component)]
struct StartNow;

fn start_effect(commands: &mut Commands, sys: &mut MapSystem) {
    if let Some(e) = sys.running {
        commands.entity(e).despawn();
    }
    let mut fx = ParticleEffect::new(sys.effect.clone());
    fx.despawn_when_done = true;
    sys.cp_offsets.clear();
    sys.running = Some(commands.spawn((Name::new(format!("particles {}", sys.effect)), fx, LevelOwned)).id());
}

/// Map inputs to particle systems, sparks and steam.
#[allow(clippy::too_many_arguments)]
fn particle_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut systems: Query<(Entity, &mut MapSystem, Has<StartNow>)>,
    mut effects: Query<&mut ParticleEffect>,
    mut sparks: Query<&mut SparkEmitter>,
    mut jets: Query<&mut SteamJet>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for (e, mut sys, start) in &mut systems {
        if start {
            commands.entity(e).remove::<StartNow>();
            start_effect(&mut commands, &mut sys);
        }
        // Killed (by `Kill` or any other way): its particles go with it.
        if ents.list[sys.ent].killed {
            if let Some(r) = sys.running.take() {
                commands.entity(r).despawn();
            }
            commands.entity(e).despawn();
        }
    }
    for msg in inputs.read() {
        let class = ents.list[msg.target].class.as_str();
        match class {
            "info_particle_system" => {
                let Some((_, mut sys, _)) = systems.iter_mut().find(|(_, s, _)| s.ent == msg.target) else { continue };
                let running = sys.running.and_then(|r| effects.get_mut(r).ok());
                match msg.input.as_str() {
                    "start" => {
                        // Already emitting: nothing to do. Stopped (dying out): a new one.
                        if !running.is_some_and(|fx| fx.is_emitting()) {
                            start_effect(&mut commands, &mut sys);
                        }
                    }
                    "stop" => {
                        if let Some(mut fx) = running {
                            fx.stop_emission();
                        }
                    }
                    "stopplayendcap" => {
                        if let Some(mut fx) = running {
                            fx.stop();
                        }
                    }
                    "destroyimmediately" => {
                        if let Some(r) = sys.running.take() {
                            commands.entity(r).despawn();
                        }
                    }
                    _ => {}
                }
            }
            "env_spark" => {
                let Some(mut sp) = sparks.iter_mut().find(|s| s.ent == msg.target) else { continue };
                match msg.input.as_str() {
                    "startspark" => sp.on = true,
                    "stopspark" => sp.on = false,
                    "togglespark" => sp.on = !sp.on,
                    "sparkonce" => sp.once = true,
                    _ => {}
                }
            }
            "env_steam" => {
                let Some(mut st) = jets.iter_mut().find(|s| s.ent == msg.target) else { continue };
                let v = msg.param.trim().parse::<f32>().ok();
                match (msg.input.as_str(), v) {
                    ("turnon", _) => st.on = true,
                    ("turnoff", _) => st.on = false,
                    ("toggle", _) => st.on = !st.on,
                    ("jetlength", Some(v)) => st.jet_length = v.max(1.0),
                    ("rate", Some(v)) => st.rate = v.max(0.0),
                    ("speed", Some(v)) => st.speed = v.max(1.0),
                    ("spreadspeed", Some(v)) => st.spread = v.max(0.0),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn camera(cams: &Query<&GlobalTransform, With<MainCamera>>) -> Option<(Vec3, Vec3)> {
    cams.iter().next().map(|g| (g.translation(), g.forward().as_vec3()))
}

/// Control points every frame, and which effects run (culling).
#[allow(clippy::too_many_arguments)]
fn run_systems(
    ents: Option<Res<Ents>>,
    mut systems: Query<&mut MapSystem>,
    mut effects: Query<&mut ParticleEffect>,
    globals: Query<&GlobalTransform>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
) {
    let Some(ents) = ents else { return };
    let mut placed: Vec<(Entity, Vec3)> = Vec::new();
    for mut sys in &mut systems {
        let Some(r) = sys.running else { continue };
        let Ok(mut fx) = effects.get_mut(r) else {
            // It finished (or its system isn't in the game's files).
            sys.running = None;
            continue;
        };
        // CP0: the entity, carried by its parent.
        let own = match sys.parent {
            Some((p, local)) => world_tf(&ents, p, &globals).mul_transform(local),
            None => world_tf(&ents, sys.ent, &globals),
        };
        fx.set_cp(0, own.translation, source_basis(own.rotation));
        let mut world: HashMap<usize, Transform> = HashMap::from([(0, own)]);
        for (cp, targets) in &sys.cps {
            if let Some(&t) = targets.first() {
                let tf = world_tf(&ents, t, &globals);
                fx.set_cp(*cp, tf.translation, source_basis(tf.rotation));
                world.insert(*cp, tf);
            }
        }
        // Parented control points keep where they started relative to the parent CP.
        let parents = sys.cp_parents.clone();
        for (cp, parent) in parents {
            let (Some(p), Some(c)) = (world.get(&parent).copied(), world.get(&cp).copied()) else { continue };
            let off = *sys.cp_offsets.entry(cp).or_insert_with(|| p.rotation.inverse() * (c.translation - p.translation));
            fx.set_cp(cp, p.translation + p.rotation * off, source_basis(c.rotation));
        }
        placed.push((r, own.translation));
    }
    let Some((cam, fwd)) = camera(&cams) else { return };
    let at: Vec<Vec3> = placed.iter().map(|p| p.1).collect();
    for ((r, _), on) in placed.iter().zip(visible_set(cam, fwd, &at, MAX_VISIBLE_EFFECTS)) {
        if let Ok(mut fx) = effects.get_mut(*r) {
            if fx.visible != on {
                fx.visible = on;
            }
        }
    }
}

// ----------------------------------------------------------------- sparks --

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spark {
    /// Source units, world axes.
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    pub life: f32,
}

#[derive(Component)]
struct SparkEmitter {
    ent: usize,
    on: bool,
    max_delay: f32,
    magnitude: f32,
    trail: f32,
    glow: bool,
    silent: bool,
    directional: bool,
    next_at: f32,
    sparks: Vec<Spark>,
    glow_left: f32,
    rng: Rng,
    once: bool,
}

/// Seconds until the next burst while on: Source's 0.1 s plus a random part of `MaxDelay`.
pub fn spark_interval(max_delay: f32, r: f32) -> f32 {
    0.1 + max_delay.max(0.0) * r
}

/// One burst: more and faster sparks with `magnitude` (1, 2, 5, 8 in Hammer),
/// thrown along `dir` (directional) or every way, in Source units.
pub fn spark_burst(origin: Vec3, dir: Option<Vec3>, magnitude: f32, rng: &mut Rng) -> Vec<Spark> {
    let n = (6.0 + magnitude * 3.0).min(40.0) as usize;
    let speed = 120.0 * magnitude.sqrt();
    (0..n)
        .map(|_| {
            let d = match dir {
                Some(f) => (f + rng.unit() * 0.6).normalize_or(f),
                None => rng.unit(),
            };
            Spark { pos: origin, vel: d * speed * rng.range(0.5, 1.5), age: 0.0, life: rng.range(0.25, 0.7) }
        })
        .collect()
}

/// Sparks fall and die.
pub fn step_sparks(sparks: &mut Vec<Spark>, dt: f32) {
    for s in sparks.iter_mut() {
        s.vel.z -= GRAVITY * dt;
        s.pos += s.vel * dt;
        s.age += dt;
    }
    sparks.retain(|s| s.age < s.life);
}

#[derive(Component)]
struct SparkMesh;

#[derive(Resource)]
struct FxLook {
    additive: Handle<StandardMaterial>,
    puff: Handle<StandardMaterial>,
    puff_emissive: Handle<StandardMaterial>,
}

fn look(materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> FxLook {
    // A soft round puff, made here (no texture from the game needed).
    let size = 32u32;
    let mut px = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let d = Vec2::new(x as f32 + 0.5 - size as f32 / 2.0, y as f32 + 0.5 - size as f32 / 2.0).length() / (size as f32 / 2.0);
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.5);
            px.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let img = images.add(Image::new(
        bevy::render::render_resource::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        px,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let puff = |emissive: bool| StandardMaterial { base_color_texture: Some(img.clone()), alpha_mode: AlphaMode::Blend, unlit: emissive, perceptual_roughness: 1.0, cull_mode: None, ..default() };
    FxLook {
        additive: materials.add(StandardMaterial { base_color_texture: Some(img.clone()), alpha_mode: AlphaMode::Add, unlit: true, cull_mode: None, ..default() }),
        puff: materials.add(puff(false)),
        puff_emissive: materials.add(puff(true)),
    }
}

/// Quads for the mesh: (centre, half width along `side`, half height along `up`, colour).
fn quads_mesh(quads: &[(Vec3, Vec3, Vec3, [f32; 4])]) -> Mesh {
    let mut pos = Vec::with_capacity(quads.len() * 4);
    let mut uv = Vec::with_capacity(quads.len() * 4);
    let mut col = Vec::with_capacity(quads.len() * 4);
    let mut nrm = Vec::with_capacity(quads.len() * 4);
    let mut idx = Vec::with_capacity(quads.len() * 6);
    for (c, side, up, color) in quads {
        let b = pos.len() as u32;
        for (sx, sy, u, v) in [(-1.0, -1.0, 0.0, 1.0), (1.0, -1.0, 1.0, 1.0), (1.0, 1.0, 1.0, 0.0), (-1.0, 1.0, 0.0, 0.0)] {
            pos.push((*c + *side * sx + *up * sy).to_array());
            uv.push([u, v]);
            col.push(*color);
            nrm.push(side.cross(*up).normalize_or(Vec3::Y).to_array());
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx))
}

/// Set (or clear) an emitter's mesh child.
fn put_mesh(commands: &mut Commands, owner: Entity, kids: Option<&Children>, mesh_q: &Query<&Mesh3d, With<SparkMesh>>, meshes: &mut Assets<Mesh>, mesh: Option<Mesh>, material: &Handle<StandardMaterial>) {
    let existing = kids.and_then(|k| k.iter().find(|c| mesh_q.contains(*c)));
    match (mesh, existing) {
        (Some(m), Some(child)) => {
            if let Ok(h) = mesh_q.get(child) {
                let _ = meshes.insert(&h.0, m);
            }
            commands.entity(child).insert((Visibility::Inherited, MeshMaterial3d(material.clone())));
        }
        (Some(m), None) => {
            commands.spawn((SparkMesh, Mesh3d(meshes.add(m)), MeshMaterial3d(material.clone()), Transform::IDENTITY, NoFrustumCulling, bevy::light::NotShadowCaster, ChildOf(owner)));
        }
        (None, Some(child)) => {
            commands.entity(child).insert(Visibility::Hidden);
        }
        (None, None) => {}
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sparks(
    mut commands: Commands,
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut emitters: Query<(Entity, &mut SparkEmitter, Option<&Children>)>,
    mesh_q: Query<&Mesh3d, With<SparkMesh>>,
    globals: Query<&GlobalTransform>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    fx_look: Option<Res<FxLook>>,
    mut play: MessageWriter<PlayGameSound>,
) {
    let Some(ents) = ents else { return };
    if emitters.is_empty() {
        return;
    }
    let Some(fx_look) = fx_look else {
        commands.insert_resource(look(&mut materials, &mut images));
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let cam = camera(&cams);
    for (e, mut sp, kids) in &mut emitters {
        let sp = &mut *sp;
        if ents.list[sp.ent].killed {
            commands.entity(e).despawn();
            continue;
        }
        let tf = world_tf(&ents, sp.ent, &globals);
        let near = cam.is_none_or(|(c, f)| visible_set(c, f, &[tf.translation], 1)[0]);
        let fire = sp.once || (sp.on && now >= sp.next_at);
        if fire {
            sp.once = false;
            let r = sp.rng.next();
            sp.next_at = now + spark_interval(sp.max_delay, r);
            if near {
                let origin = crate::console::to_source(tf.translation);
                let dir = sp.directional.then(|| crate::console::to_source(tf.translation + tf.rotation * Vec3::X) - origin).map(|d| d.normalize_or(Vec3::Z));
                let room = MAX_SPARKS.saturating_sub(sp.sparks.len());
                let burst = spark_burst(origin, dir, sp.magnitude, &mut sp.rng);
                sp.sparks.extend(burst.into_iter().take(room));
                if sp.glow {
                    sp.glow_left = 0.12;
                }
            }
            if !sp.silent {
                play.write(PlayGameSound { sound: "DoSpark".into(), at: Some(tf.translation), looped: false, owner: None, music: false, volume: 1.0 });
            }
        }
        if !near {
            sp.sparks.clear();
        }
        step_sparks(&mut sp.sparks, dt);
        sp.glow_left = (sp.glow_left - dt).max(0.0);
        let Some((cam_pos, _)) = cam else { continue };
        // Streaks: tail to head, square to the view.
        let mut quads = Vec::new();
        for s in &sp.sparks {
            let head = source_to_world(s.pos);
            let len = 0.02 * sp.trail;
            let tail = head - source_dir_to_world(s.vel) * U * len;
            let mid = (head + tail) * 0.5;
            let along = (head - tail) * 0.5;
            let side = along.cross(cam_pos - mid).normalize_or(Vec3::X) * 0.6 * U * sp.magnitude.sqrt().min(2.0);
            let k = 1.0 - s.age / s.life;
            quads.push((mid, side, along, [1.0, 0.75 + 0.25 * k, 0.35 * k, k]));
        }
        if sp.glow_left > 0.0 {
            let to_cam = (cam_pos - tf.translation).normalize_or(Vec3::Z);
            let side = to_cam.any_orthonormal_vector();
            let up = to_cam.cross(side);
            let r = 12.0 * U * sp.magnitude.sqrt();
            quads.push((tf.translation, side * r, up * r, [1.0, 0.7, 0.3, sp.glow_left / 0.12]));
        }
        let mesh = (!quads.is_empty()).then(|| quads_mesh(&quads));
        put_mesh(&mut commands, e, kids, &mesh_q, &mut meshes, mesh, &fx_look.additive);
    }
}

// ------------------------------------------------------------------ steam --

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Source units along the jet's own axes (x forward).
    pub local: Vec3,
    pub vel: Vec3,
    pub age: f32,
    pub roll: f32,
}

#[derive(Component)]
pub struct SteamJet {
    ent: usize,
    pub on: bool,
    pub speed: f32,
    pub spread: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub rate: f32,
    pub jet_length: f32,
    pub color: [f32; 4],
    pub roll_speed: f32,
    pub emissive: bool,
    pub heat_wave: bool,
    owed: f32,
    pub puffs: Vec<Puff>,
    rng: Rng,
}

impl SteamJet {
    /// From the entity's keyvalues (Source's defaults where it has them).
    pub fn from_keyvalues(ent: usize, spawnflags: u32, get: &dyn Fn(&str) -> Option<String>, rng: Rng) -> Self {
        let f = |k: &str, d: f32| get(k).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(d);
        let c = get("rendercolor").map_or([1.0; 3], |v| entity_color(&v));
        SteamJet {
            ent,
            on: f("initialstate", 0.0) != 0.0,
            speed: f("speed", 120.0).max(1.0),
            spread: f("spreadspeed", 15.0).max(0.0),
            start_size: f("startsize", 10.0),
            end_size: f("endsize", 25.0),
            rate: f("rate", 26.0).max(0.0),
            jet_length: f("jetlength", 80.0).max(1.0),
            color: [c[0], c[1], c[2], (f("renderamt", 255.0) / 255.0).clamp(0.0, 1.0)],
            roll_speed: f("rollspeed", 8.0),
            emissive: spawnflags & 1 != 0,
            heat_wave: f("type", 0.0) == 1.0,
            owed: 0.0,
            puffs: Vec::new(),
            rng,
        }
    }

    /// Seconds a puff lives: the time to travel the jet's length.
    pub fn life(&self) -> f32 {
        self.jet_length / self.speed
    }

    /// Size (diameter, u) and opacity (0..1) of a puff `age` seconds old.
    pub fn look_at(&self, age: f32) -> (f32, f32) {
        let t = (age / self.life()).clamp(0.0, 1.0);
        let size = self.start_size + (self.end_size - self.start_size) * t;
        // Quick fade in, long fade out.
        let a = if t < 0.1 { t / 0.1 } else { 1.0 - (t - 0.1) / 0.9 };
        (size, self.color[3] * a.clamp(0.0, 1.0))
    }

    /// Advance: new puffs at `rate` while on, old ones move and expire.
    pub fn step(&mut self, dt: f32) {
        if self.on {
            self.owed += self.rate * dt;
        }
        let life = self.life();
        while self.owed >= 1.0 {
            self.owed -= 1.0;
            if self.puffs.len() >= MAX_PUFFS {
                continue;
            }
            let side = Vec3::new(0.0, self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * self.spread;
            let v = Vec3::new(self.speed * self.rng.range(0.9, 1.1), 0.0, 0.0) + side;
            let roll = self.rng.range(0.0, std::f32::consts::TAU);
            self.puffs.push(Puff { local: Vec3::ZERO, vel: v, age: 0.0, roll });
        }
        for p in &mut self.puffs {
            p.local += p.vel * dt;
            p.age += dt;
            p.roll += self.roll_speed.to_radians() * dt;
        }
        self.puffs.retain(|p| p.age < life);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn steam(
    mut commands: Commands,
    time: Res<Time>,
    ents: Option<Res<Ents>>,
    mut jets: Query<(Entity, &mut SteamJet, Option<&Children>)>,
    mesh_q: Query<&Mesh3d, With<SparkMesh>>,
    globals: Query<&GlobalTransform>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    fx_look: Option<Res<FxLook>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(ents) = ents else { return };
    if jets.is_empty() {
        return;
    }
    let Some(fx_look) = fx_look else {
        commands.insert_resource(look(&mut materials, &mut images));
        return;
    };
    let dt = time.delta_secs();
    let Some((cam_pos, cam_fwd)) = camera(&cams) else { return };
    for (e, mut jet, kids) in &mut jets {
        if ents.list[jet.ent].killed {
            commands.entity(e).despawn();
            continue;
        }
        let tf = world_tf(&ents, jet.ent, &globals);
        let near = visible_set(cam_pos, cam_fwd, &[tf.translation], 1)[0];
        if !near || jet.heat_wave {
            jet.puffs.clear();
            put_mesh(&mut commands, e, kids, &mesh_q, &mut meshes, None, &fx_look.puff);
            continue;
        }
        jet.step(dt);
        let basis = source_basis(tf.rotation);
        let mut quads = Vec::with_capacity(jet.puffs.len());
        // Back to front, so the blend stacks right.
        let mut order: Vec<usize> = (0..jet.puffs.len()).collect();
        let world_of = |p: &Puff| tf.translation + basis * (p.local * U);
        order.sort_by(|&a, &b| world_of(&jet.puffs[b]).distance_squared(cam_pos).total_cmp(&world_of(&jet.puffs[a]).distance_squared(cam_pos)));
        for i in order {
            let p = jet.puffs[i];
            let at = world_of(&p);
            let (size, alpha) = jet.look_at(p.age);
            let to_cam = (cam_pos - at).normalize_or(Vec3::Z);
            let s0 = to_cam.any_orthonormal_vector();
            let u0 = to_cam.cross(s0);
            let (sn, cs) = p.roll.sin_cos();
            let r = size * 0.5 * U;
            let side = (s0 * cs + u0 * sn) * r;
            let up = (u0 * cs - s0 * sn) * r;
            quads.push((at, side, up, [jet.color[0], jet.color[1], jet.color[2], alpha]));
        }
        let mesh = (!quads.is_empty()).then(|| quads_mesh(&quads));
        let mat = if jet.emissive { &fx_look.puff_emissive } else { &fx_look.puff };
        put_mesh(&mut commands, e, kids, &mesh_q, &mut meshes, mesh, mat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kvs<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| pairs.iter().find(|(a, _)| *a == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn colours() {
        assert_eq!(entity_color("255 128 0"), [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(entity_color(""), [1.0; 3]);
        assert_eq!(entity_color("0 0"), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn control_point_keys() {
        let get = kvs(&[("cpoint1", "target_a"), ("cpoint2", " "), ("cpoint63", "far"), ("cpoint3_parent", "1"), ("cpoint4_parent", "0"), ("cpoint5_parent", "5"), ("cpoint6_parent", "x")]);
        assert_eq!(control_point_targets(&get), [(1, "target_a".to_string()), (63, "far".to_string())]);
        assert_eq!(control_point_parents(&get), [(3, 1)]);
    }

    #[test]
    fn source_axes_of_an_entity() {
        // Angles 0 90 0 (yaw 90): forward is Source +Y = world -Z, up is world +Y.
        let q = source_assets::entities::source_angles_to_world(Vec3::new(0.0, 90.0, 0.0));
        let b = source_basis(q);
        assert!((b.x_axis - Vec3::NEG_Z).length() < 1e-5, "{:?}", b.x_axis);
        assert!((b.z_axis - Vec3::Y).length() < 1e-5);
        // Source left (+Y local) at yaw 90 is Source -X = world -X.
        assert!((b.y_axis - Vec3::NEG_X).length() < 1e-5, "{:?}", b.y_axis);
    }

    #[test]
    fn culling_by_distance_direction_and_count() {
        let cam = Vec3::ZERO;
        let fwd = Vec3::NEG_Z;
        let at = [Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, -200.0), Vec3::new(0.0, 0.0, 30.0), Vec3::new(0.0, 0.0, 3.0), Vec3::new(20.0, 0.0, -1.0)];
        // Near and ahead; too far; well behind; behind but close; to the side.
        assert_eq!(visible_set(cam, fwd, &at, 10), [true, false, false, true, true]);
        // A cap keeps the nearest.
        assert_eq!(visible_set(cam, fwd, &at, 2), [true, false, false, true, false]);
    }

    #[test]
    fn spark_timing_and_bursts() {
        assert_eq!(spark_interval(0.0, 0.7), 0.1);
        assert!((spark_interval(2.0, 0.5) - 1.1).abs() < 1e-6);
        let mut rng = Rng::new(9);
        let small = spark_burst(Vec3::ZERO, None, 1.0, &mut rng);
        let big = spark_burst(Vec3::ZERO, None, 8.0, &mut rng);
        assert!(big.len() > small.len());
        let speed = |v: &[Spark]| v.iter().map(|s| s.vel.length()).sum::<f32>() / v.len() as f32;
        assert!(speed(&big) > speed(&small));
        // Directional: mostly along the forward.
        let dir = spark_burst(Vec3::ZERO, Some(Vec3::X), 2.0, &mut rng);
        assert!(dir.iter().all(|s| s.vel.normalize().dot(Vec3::X) > 0.0));
        // They fall and die within their life.
        let mut s = small.clone();
        step_sparks(&mut s, 0.1);
        assert!(s.iter().all(|x| x.vel.z < small[0].vel.z.max(1e6)));
        for _ in 0..10 {
            step_sparks(&mut s, 0.1);
        }
        assert!(s.is_empty());
    }

    #[test]
    fn steam_from_keyvalues() {
        let get = kvs(&[("initialstate", "1"), ("speed", "200"), ("jetlength", "100"), ("rate", "10"), ("startsize", "8"), ("endsize", "40"), ("rendercolor", "128 128 255"), ("renderamt", "128")]);
        let mut jet = SteamJet::from_keyvalues(3, 1, &get, Rng::new(1));
        assert!(jet.on && jet.emissive && !jet.heat_wave);
        assert!((jet.life() - 0.5).abs() < 1e-6);
        assert!((jet.color[2] - 1.0).abs() < 1e-6 && (jet.color[3] - 128.0 / 255.0).abs() < 1e-6);
        // Size grows from start to end; fades in then out.
        assert_eq!(jet.look_at(0.0).0, 8.0);
        assert!((jet.look_at(0.5).0 - 40.0).abs() < 1e-4);
        assert_eq!(jet.look_at(0.0).1, 0.0);
        assert!(jet.look_at(0.05).1 > 0.4);
        assert!(jet.look_at(0.49).1 < 0.05);
        // 10 puffs a second for a second: about 5 alive (they live 0.5 s).
        for _ in 0..60 {
            jet.step(1.0 / 60.0);
        }
        assert!((4..=6).contains(&jet.puffs.len()), "{}", jet.puffs.len());
        assert!(jet.puffs.iter().all(|p| p.local.x > 0.0 && p.local.x <= 110.0 * 1.1));
        jet.on = false;
        for _ in 0..60 {
            jet.step(1.0 / 60.0);
        }
        assert!(jet.puffs.is_empty());
        // Defaults (Source's): off, 120 u/s, 80 u long.
        let d = SteamJet::from_keyvalues(0, 0, &kvs(&[]), Rng::new(1));
        assert!(!d.on && d.speed == 120.0 && d.jet_length == 80.0 && d.rate == 26.0);
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;
    use crate::io::GameEnt;

    fn ent(class: &str, name: &str, flags: u32, kvs: &[(&str, &str)]) -> GameEnt {
        let mut e = GameEnt::logic(class, name, kvs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        e.spawnflags = flags;
        e
    }

    #[test]
    fn map_entities_run() {
        let root = std::env::temp_dir().join(format!("p2-mapfx-{}", std::process::id()));
        std::fs::create_dir_all(root.join("platform")).unwrap();
        let fs = source_assets::GameFs::mount(source_assets::config::GameConfig::from_root(&root)).unwrap();
        let mut app = App::new();
        app.set_error_handler(bevy::ecs::error::panic);
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<Image>()
            .configure_sets(Update, (crate::level::GameSet::Load, crate::level::GameSet::Logic).chain())
            .add_message::<LevelLoaded>()
            .add_message::<EntInput>()
            .add_message::<crate::io::FireOutput>()
            .add_message::<PlayGameSound>()
            .insert_resource(p2_assets::GameFsRes(Arc::new(fs)))
            .init_resource::<Particles>()
            .add_systems(Update, crate::io::run_io.in_set(crate::level::GameSet::Logic))
            .add_plugins(MapParticlesPlugin);
        let mut ents = Ents::default();
        let fx = ents.add(ent("info_particle_system", "fx", 0, &[("effect_name", "steam_jet"), ("start_active", "1"), ("cpoint1", "spark")]));
        let spark = ents.add(ent("env_spark", "spark", 64 | 128, &[("magnitude", "2"), ("maxdelay", "0")]));
        let steam = ents.add(ent("env_steam", "steam", 1, &[("initialstate", "1"), ("rate", "60")]));
        app.insert_resource(ents);
        app.world_mut().spawn((MainCamera, Transform::from_xyz(0.0, 0.0, 3.0), GlobalTransform::from_xyz(0.0, 0.0, 3.0)));
        app.world_mut().write_message(LevelLoaded { map: "test".into(), menu: false });
        for _ in 0..10 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // The particle system started (start_active) and has its control points.
        let w = app.world_mut();
        let mut q = w.query::<&ParticleEffect>();
        let effects: Vec<String> = q.iter(w).map(|f| f.name.clone()).collect();
        assert_eq!(effects, ["steam_jet"]);
        let mut sq = w.query::<&SparkEmitter>();
        let sp = sq.single(w).unwrap();
        assert!(sp.on && !sp.sparks.is_empty());
        let mut jq = w.query::<&SteamJet>();
        assert!(!jq.single(w).unwrap().puffs.is_empty());
        // Stop: no new particles; the effect stays until its particles are gone.
        app.world_mut().write_message(EntInput { target: fx, input: "stop".into(), param: String::new(), activator: None });
        app.world_mut().write_message(EntInput { target: spark, input: "stopspark".into(), param: String::new(), activator: None });
        app.world_mut().write_message(EntInput { target: steam, input: "turnoff".into(), param: String::new(), activator: None });
        app.update();
        let w = app.world_mut();
        let mut q = w.query::<&ParticleEffect>();
        assert!(q.iter(w).all(|f| !f.is_emitting()));
        let mut sq = w.query::<&SparkEmitter>();
        assert!(!sq.single(w).unwrap().on);
        let mut jq = w.query::<&SteamJet>();
        assert!(!jq.single(w).unwrap().on);
        // Kill removes the system and its effect.
        app.world_mut().resource_mut::<Ents>().list[fx].killed = true;
        app.update();
        app.update();
        let w = app.world_mut();
        let mut q = w.query::<&ParticleEffect>();
        assert_eq!(q.iter(w).count(), 0);
        let mut mq = w.query::<&MapSystem>();
        assert_eq!(mq.iter(w).count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
