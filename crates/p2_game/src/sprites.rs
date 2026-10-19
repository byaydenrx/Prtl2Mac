//! Glow sprites and light beams: `env_sprite` (and `env_glow`,
//! `env_sprite_oriented`) as camera-facing quads of their sprite material,
//! `point_spotlight` as its volumetric cone (a camera-facing beam widening
//! and fading along its length), and `env_beam` as a camera-facing ribbon
//! between its two ends.
//!
//! Sprites: `model` (`sprites/*.vmt`, its `$basetexture`), world size =
//! texture size x `scale` units, `rendercolor` x `HDRColorScale`,
//! `renderamt` alpha, `rendermode` 3 / 5 / 9 (glow / additive / world glow)
//! drawn additively, others blended. A glow is pulled `GlowProxySize` units
//! toward the camera so the fixture it sits on doesn't cut it. Start on
//! with spawnflag 1, or when unnamed. Inputs: `ShowSprite`, `HideSprite`,
//! `ToggleSprite`, `ColorRedValue` / `ColorGreenValue` / `ColorBlueValue`,
//! `Color`, `Alpha`, `SetScale`. Parented sprites follow their parent
//! (they are placed from the entity's transform every frame).
//!
//! Not done: occlusion fading (`GlowProxySize` as an occlusion query),
//! animated sprite frames, `rendermode` 3's constant screen size is only
//! approximated (see [`sprite_transform`]), beam noise.
use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use p2_assets::GameFsRes;
use p2_core::*;
use shared::SOURCE_UNIT;

use crate::io::{EntInput, Ents, GameEnt};
use crate::level::{GameSet, LevelLoaded, LevelOwned};

pub struct SpritesPlugin;

impl Plugin for SpritesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Glows>()
            .add_systems(Update, (spawn_glows, glow_inputs, place_glows).chain().after(crate::io::run_io).after(crate::viewcontrol::follow_attachments).in_set(GameSet::Logic));
    }
}

/// What a glow entity draws.
#[derive(Clone, Debug, PartialEq)]
pub enum GlowKind {
    /// A camera-facing quad: size (metres, at scale 1), constant-screen-size
    /// glow (`rendermode` 3), proxy pull toward the camera (metres).
    Sprite { size: Vec2, screen_glow: bool, proxy: f32 },
    /// A spotlight cone from the entity along its forward: length, start
    /// width (metres).
    Spotlight { length: f32, width: f32 },
    /// A beam between two named entities, width (metres).
    Beam { start: String, end: String, width: f32 },
}

/// Run-time look of one glow.
#[derive(Clone, Debug, PartialEq)]
pub struct GlowLook {
    pub on: bool,
    /// `rendercolor` 0..255 (gamma space, as the map gives it).
    pub color: Vec3,
    /// `renderamt` 0..255.
    pub alpha: f32,
    pub hdr: f32,
    pub scale: f32,
}

impl GlowLook {
    pub fn from_ent(e: &GameEnt, on_flag: u32) -> Self {
        let color = e.get("rendercolor").and_then(parse_rgb).unwrap_or(Vec3::splat(255.0));
        GlowLook {
            on: e.spawnflags & on_flag != 0 || e.name.is_empty(),
            color,
            alpha: e.get_f32("renderamt").unwrap_or(255.0),
            hdr: e.get_f32("hdrcolorscale").unwrap_or(1.0),
            scale: e.get_f32("scale").filter(|s| *s > 0.0).unwrap_or(1.0),
        }
    }

    /// Linear additive colour (with the HDR scale) and alpha, for the material.
    pub fn linear(&self) -> (Vec3, f32) {
        let c = (self.color / 255.0).clamp(Vec3::ZERO, Vec3::ONE).powf(2.2) * self.hdr.max(0.0);
        (c, (self.alpha / 255.0).clamp(0.0, 1.0))
    }

    /// Applies an input; returns whether the look changed.
    pub fn input(&mut self, input: &str, param: &str, show: &str, hide: &str, toggle: &str) -> bool {
        let before = self.clone();
        let num = || source_assets::kv::parse_f32(param);
        match input {
            i if i == show => self.on = true,
            i if i == hide => self.on = false,
            i if i == toggle => self.on = !self.on,
            "colorredvalue" => self.color.x = num().unwrap_or(self.color.x),
            "colorgreenvalue" => self.color.y = num().unwrap_or(self.color.y),
            "colorbluevalue" => self.color.z = num().unwrap_or(self.color.z),
            "color" => self.color = parse_rgb(param).unwrap_or(self.color),
            "alpha" => self.alpha = num().unwrap_or(self.alpha),
            "setscale" => self.scale = num().filter(|s| *s > 0.0).unwrap_or(self.scale),
            _ => {}
        }
        *self != before
    }
}

/// `"r g b"` (0..255).
fn parse_rgb(s: &str) -> Option<Vec3> {
    let v: Vec<f32> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    (v.len() >= 3).then(|| Vec3::new(v[0], v[1], v[2]))
}

/// How a `rendermode` blends: additive for the glow modes and additive
/// (3, 5, 9), alpha-blended otherwise.
pub fn render_alpha_mode(rendermode: i32) -> AlphaMode {
    match rendermode {
        3 | 5 | 9 => AlphaMode::Add,
        _ => AlphaMode::Blend,
    }
}

/// A sprite's size in metres: one texel per unit at scale 1.
pub fn sprite_size(width: u32, height: u32) -> Vec2 {
    Vec2::new(width as f32, height as f32) * SOURCE_UNIT
}

/// Distance at which a constant-screen-size glow (`rendermode` 3) is drawn
/// at its world size; nearer it shrinks, farther it grows.
const SCREEN_GLOW_REF: f32 = 256.0 * SOURCE_UNIT;

/// A sprite quad facing the camera (parallel to the view plane, Source's
/// default `vp_parallel`), pulled `proxy` toward the camera.
pub fn sprite_transform(pos: Vec3, size: Vec2, scale: f32, screen_glow: bool, proxy: f32, cam_pos: Vec3, cam_rot: Quat) -> Transform {
    let to_cam = cam_pos - pos;
    let dist = to_cam.length();
    let pull = proxy.min(dist * 0.5);
    let at = pos + to_cam.normalize_or_zero() * pull;
    let k = if screen_glow { (dist / SCREEN_GLOW_REF).max(0.05) } else { 1.0 };
    Transform::from_translation(at).with_rotation(cam_rot).with_scale((size * scale * k).extend(1.0))
}

/// A beam quad (mesh x 0..1 along the beam, y -0.5..0.5 across) from `a`
/// to `b`, `width` wide, turned about its axis to face the camera.
pub fn beam_transform(a: Vec3, b: Vec3, width: f32, cam_pos: Vec3) -> Option<Transform> {
    let axis = b - a;
    let len = axis.length();
    if len < 1e-5 {
        return None;
    }
    let x = axis / len;
    let mid = (a + b) * 0.5;
    let y = x.cross(cam_pos - mid).try_normalize().or_else(|| x.any_orthonormal_vector().try_normalize())?;
    let z = x.cross(y);
    Some(Transform::from_translation(a).with_rotation(Quat::from_mat3(&Mat3::from_cols(x, y, z))).with_scale(Vec3::new(len, width, 1.0)))
}

/// Spotlight beams widen from `width` at the lamp to this many times it at
/// their far end.
pub const SPOTLIGHT_SPREAD: f32 = 2.0;

/// A beam quad mesh: x 0..1, y +-0.5 at x = 1 and +-0.5 * `start_ratio` at
/// x = 0; u along x, v across.
pub fn beam_mesh(start_ratio: f32) -> Mesh {
    let h = 0.5 * start_ratio;
    let positions = vec![[0.0, -h, 0.0], [1.0, -0.5, 0.0], [1.0, 0.5, 0.0], [0.0, h, 0.0]];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

/// The volumetric cone's texture: bright at the lamp and along the middle,
/// fading to nothing at the far end and the edges.
pub fn spotlight_texel(u: f32, v: f32) -> f32 {
    let along = (1.0 - u.clamp(0.0, 1.0)).powf(1.5);
    let across = (1.0 - (2.0 * v - 1.0).abs()).clamp(0.0, 1.0).powi(2);
    along * across
}

fn spotlight_image() -> Image {
    let (w, h) = (64u32, 32u32);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let i = (spotlight_texel((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32) * 255.0) as u8;
            data.extend_from_slice(&[i, i, i, 255]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

struct Glow {
    entity: Entity,
    kind: GlowKind,
    look: GlowLook,
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct Glows(HashMap<usize, Glow>);

/// A sprite material's base texture (sRGB) and its size in texels.
fn sprite_texture(game: &GameFsRes, images: &mut Assets<Image>, material: &str) -> Option<(Handle<Image>, u32, u32)> {
    let read = |p: &str| game.read(p);
    let (_, kv) = source_assets::vmt::load_vmt(&read, &material.replace(".spr", ""))?;
    let t = kv.get("$basetexture")?;
    let data = read(&format!("materials/{}.vtf", source_assets::vmt::canonical_name(t)))?;
    let vtf = source_assets::vtf::VtfTexture::parse(&data).ok()?;
    let (w, h) = (vtf.width, vtf.height);
    Some((images.add(vtf.into_image(false, true)), w, h))
}

fn material_for(look: &GlowLook, alpha_mode: AlphaMode, texture: Option<Handle<Image>>) -> StandardMaterial {
    let (c, a) = look.linear();
    StandardMaterial {
        base_color: Color::linear_rgba(c.x, c.y, c.z, a),
        base_color_texture: texture,
        unlit: true,
        alpha_mode,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_glows(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut glows: ResMut<Glows>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    glows.0.clear();
    let Some(ents) = ents else { return };
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let ribbon = meshes.add(beam_mesh(1.0));
    let cone = meshes.add(beam_mesh(1.0 / SPOTLIGHT_SPREAD));
    let cone_tex = images.add(spotlight_image());
    let mut textures: HashMap<String, Option<(Handle<Image>, u32, u32)>> = HashMap::new();
    for (i, e) in ents.list.iter().enumerate() {
        let class = e.class.as_str();
        let (kind, look, mesh, alpha_mode, texture) = match class {
            "env_sprite" | "env_glow" | "env_sprite_oriented" => {
                let Some(model) = e.get("model").map(|m| m.to_ascii_lowercase()) else { continue };
                let tex = textures.entry(model.clone()).or_insert_with(|| sprite_texture(&game, &mut images, &model)).clone();
                let Some((tex, w, h)) = tex else {
                    debug!("portal2: {class} {:?}: no sprite texture for {model}", e.name);
                    continue;
                };
                let mode = e.get_f32("rendermode").unwrap_or(0.0) as i32;
                let proxy = e.get_f32("glowproxysize").unwrap_or(2.0) * SOURCE_UNIT;
                let kind = GlowKind::Sprite { size: sprite_size(w, h), screen_glow: mode == 3, proxy };
                (kind, GlowLook::from_ent(e, 1), quad.clone(), render_alpha_mode(mode), Some(tex))
            }
            "point_spotlight" => {
                let length = e.get_f32("spotlightlength").unwrap_or(500.0) * SOURCE_UNIT;
                let width = e.get_f32("spotlightwidth").unwrap_or(50.0) * SOURCE_UNIT;
                (GlowKind::Spotlight { length, width }, GlowLook::from_ent(e, 1), cone.clone(), AlphaMode::Add, Some(cone_tex.clone()))
            }
            "env_beam" => {
                let (Some(start), end) = (e.get("lightningstart"), e.get("lightningend")) else { continue };
                let width = e.get_f32("boltwidth").unwrap_or(2.0) * SOURCE_UNIT;
                let tex = e.get("texture").map(|t| t.to_ascii_lowercase()).and_then(|t| textures.entry(t.clone()).or_insert_with(|| sprite_texture(&game, &mut images, &t)).clone());
                let kind = GlowKind::Beam { start: start.to_string(), end: end.unwrap_or("").to_string(), width };
                let mode = e.get_f32("rendermode").unwrap_or(5.0) as i32;
                (kind, GlowLook::from_ent(e, 1), ribbon.clone(), render_alpha_mode(mode), tex.map(|t| t.0))
            }
            _ => continue,
        };
        let material = materials.add(material_for(&look, alpha_mode, texture));
        let entity = commands
            .spawn((
                Name::new(format!("{class} {}", e.name)),
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(e.transform.translation).with_scale(Vec3::ZERO),
                if look.on { Visibility::Inherited } else { Visibility::Hidden },
                bevy::light::NotShadowCaster,
                LevelOwned,
            ))
            .id();
        glows.0.insert(i, Glow { entity, kind, look, material });
    }
    if !glows.0.is_empty() {
        debug!("portal2: {} sprites / spotlights / beams", glows.0.len());
    }
}

fn glow_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut glows: ResMut<Glows>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let Some(g) = glows.0.get_mut(&msg.target) else { continue };
        let (show, hide, toggle) = match ents.list[msg.target].class.as_str() {
            "point_spotlight" => ("lighton", "lightoff", "toggle"),
            "env_beam" => ("turnon", "turnoff", "toggle"),
            _ => ("showsprite", "hidesprite", "togglesprite"),
        };
        if !g.look.input(&msg.input, msg.param.trim(), show, hide, toggle) {
            continue;
        }
        if let Ok(mut v) = vis.get_mut(g.entity) {
            *v = if g.look.on { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Some(mut m) = materials.get_mut(&g.material) {
            let (c, a) = g.look.linear();
            m.base_color = Color::linear_rgba(c.x, c.y, c.z, a);
        }
    }
}

/// Faces every glow at the camera and puts it where its entity is now.
fn place_glows(ents: Option<Res<Ents>>, glows: Res<Glows>, cams: Query<&GlobalTransform, With<MainCamera>>, mut tfs: Query<&mut Transform>) {
    let (Some(ents), Some(cam)) = (ents, cams.iter().next()) else { return };
    let (cam_pos, cam_rot) = (cam.translation(), cam.rotation());
    for (&i, g) in &glows.0 {
        if !g.look.on {
            continue;
        }
        let e = &ents.list[i];
        let at = e.transform.translation;
        let tf = match &g.kind {
            GlowKind::Sprite { size, screen_glow, proxy } => Some(sprite_transform(at, *size, g.look.scale, *screen_glow, *proxy, cam_pos, cam_rot)),
            GlowKind::Spotlight { length, width } => {
                let fwd = e.transform.rotation * Vec3::X;
                beam_transform(at, at + fwd * *length, *width * SPOTLIGHT_SPREAD, cam_pos)
            }
            GlowKind::Beam { start, end, width } => {
                let pos = |n: &str| ents.find(n, i, None).first().map(|&k| ents.list[k].transform.translation);
                let a = pos(start).unwrap_or(at);
                let b = if end.is_empty() { Some(at) } else { pos(end) };
                b.and_then(|b| beam_transform(a, b, *width, cam_pos))
            }
        };
        if let (Some(tf), Ok(mut t)) = (tf, tfs.get_mut(g.entity)) {
            *t = tf;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(class: &str, name: &str, flags: u32, kv: &[(&str, &str)]) -> GameEnt {
        GameEnt { spawnflags: flags, ..GameEnt::logic(class, name, kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()) }
    }

    #[test]
    fn look_from_keyvalues() {
        let e = ent("env_sprite", "glow1", 1, &[("rendercolor", "255 128 0"), ("renderamt", "200"), ("hdrcolorscale", "2"), ("scale", "0.25")]);
        let l = GlowLook::from_ent(&e, 1);
        assert!(l.on);
        assert_eq!(l.color, Vec3::new(255.0, 128.0, 0.0));
        assert_eq!((l.alpha, l.hdr, l.scale), (200.0, 2.0, 0.25));
        let (c, a) = l.linear();
        assert!((c.x - 2.0).abs() < 1e-5 && c.y > 0.4 && c.y < 0.5 && c.z == 0.0, "{c}");
        assert!((a - 200.0 / 255.0).abs() < 1e-6);
        // Named without "start on": off; unnamed: always on.
        assert!(!GlowLook::from_ent(&ent("env_sprite", "glow2", 0, &[]), 1).on);
        assert!(GlowLook::from_ent(&ent("env_sprite", "", 0, &[]), 1).on);
        let d = GlowLook::from_ent(&ent("env_sprite", "", 0, &[]), 1);
        assert_eq!((d.color, d.alpha, d.hdr, d.scale), (Vec3::splat(255.0), 255.0, 1.0, 1.0));
    }

    #[test]
    fn inputs_change_the_look() {
        let mut l = GlowLook::from_ent(&ent("env_sprite", "g", 0, &[]), 1);
        let s = |l: &mut GlowLook, i: &str, p: &str| l.input(i, p, "showsprite", "hidesprite", "togglesprite");
        assert!(s(&mut l, "showsprite", "") && l.on);
        assert!(!s(&mut l, "showsprite", ""), "no change");
        assert!(s(&mut l, "togglesprite", "") && !l.on);
        assert!(s(&mut l, "colorredvalue", "10") && l.color.x == 10.0);
        assert!(s(&mut l, "color", "1 2 3") && l.color == Vec3::new(1.0, 2.0, 3.0));
        assert!(s(&mut l, "alpha", "64") && l.alpha == 64.0);
        assert!(s(&mut l, "setscale", "3") && l.scale == 3.0);
        assert!(!s(&mut l, "setscale", "-1"));
        // Spotlights use LightOn / LightOff.
        assert!(l.input("lighton", "", "lighton", "lightoff", "toggle") && l.on);
        assert!(l.input("lightoff", "", "lighton", "lightoff", "toggle") && !l.on);
    }

    #[test]
    fn render_modes() {
        assert_eq!(render_alpha_mode(9), AlphaMode::Add);
        assert_eq!(render_alpha_mode(5), AlphaMode::Add);
        assert_eq!(render_alpha_mode(3), AlphaMode::Add);
        assert_eq!(render_alpha_mode(0), AlphaMode::Blend);
        assert_eq!(render_alpha_mode(1), AlphaMode::Blend);
    }

    #[test]
    fn sprites_face_the_camera_at_their_size() {
        let size = sprite_size(128, 64);
        assert!((size - Vec2::new(128.0, 64.0) * SOURCE_UNIT).length() < 1e-6);
        let cam_rot = Quat::from_rotation_y(0.7);
        let cam = Vec3::new(0.0, 0.0, 10.0);
        let t = sprite_transform(Vec3::ZERO, size, 0.5, false, 0.1, cam, cam_rot);
        assert_eq!(t.rotation, cam_rot);
        assert!((t.scale - Vec3::new(64.0 * SOURCE_UNIT, 32.0 * SOURCE_UNIT, 1.0)).length() < 1e-6);
        assert!((t.translation - Vec3::new(0.0, 0.0, 0.1)).length() < 1e-6, "pulled toward the camera");
        // Never pulled past half way.
        let near = sprite_transform(Vec3::ZERO, size, 1.0, false, 5.0, Vec3::Z, cam_rot);
        assert!((near.translation.z - 0.5).abs() < 1e-6);
        // A constant-screen-size glow grows with distance.
        let a = sprite_transform(Vec3::ZERO, size, 1.0, true, 0.0, Vec3::Z * SCREEN_GLOW_REF, cam_rot);
        let b = sprite_transform(Vec3::ZERO, size, 1.0, true, 0.0, Vec3::Z * SCREEN_GLOW_REF * 2.0, cam_rot);
        assert!((a.scale.x - size.x).abs() < 1e-6 && (b.scale.x - 2.0 * size.x).abs() < 1e-6);
    }

    #[test]
    fn beams_run_end_to_end_facing_the_camera() {
        let (a, b) = (Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 4.0, 0.0));
        let cam = Vec3::new(1.0, 2.0, 5.0);
        let t = beam_transform(a, b, 0.5, cam).unwrap();
        // Mesh x 0..1 maps onto a..b.
        assert!((t.transform_point(Vec3::ZERO) - a).length() < 1e-5);
        assert!((t.transform_point(Vec3::X) - b).length() < 1e-5);
        // Its width runs across, its face toward the camera.
        let across = t.transform_point(Vec3::new(0.0, 0.5, 0.0)) - a;
        assert!((across.length() - 0.25).abs() < 1e-5 && across.dot(b - a).abs() < 1e-5);
        let normal = t.rotation * Vec3::Z;
        assert!(normal.cross(cam - (a + b) * 0.5).length() < 1e-4, "faces along the view line");
        assert!(beam_transform(a, a, 1.0, cam).is_none());
        // Looking straight down the beam still gives a valid quad.
        assert!(beam_transform(Vec3::ZERO, Vec3::X, 1.0, Vec3::X * 5.0).is_some());
    }

    #[test]
    fn spotlight_cone_fades_out() {
        assert!(spotlight_texel(0.0, 0.5) > 0.99);
        assert!(spotlight_texel(1.0, 0.5) < 1e-6, "far end");
        assert!(spotlight_texel(0.0, 0.0) < 1e-6 && spotlight_texel(0.0, 1.0) < 1e-6, "edges");
        assert!(spotlight_texel(0.3, 0.5) > spotlight_texel(0.6, 0.5));
        let m = beam_mesh(0.5);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!() };
        assert_eq!(p[0], [0.0, -0.25, 0.0]);
        assert_eq!(p[2], [1.0, 0.5, 0.0]);
    }
}
