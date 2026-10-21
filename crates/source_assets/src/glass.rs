//! Glass and water: Portal 2's `Refract` and `Water` materials (chamber and
//! elevator windows, observation glass, flooded areas, toxic goo) drawn by
//! our own shader (`shaders/glass.wgsl`) instead of a tinted pane.
//!
//! Both read what is behind them from Bevy's screen-space transmission
//! texture: [`GlassMaterial::reads_view_transmission_texture`] puts them in
//! Bevy's transmissive pass, which copies the opaque scene (the Valve-shader
//! surfaces included: `valve_render` and `projected_render` run before it)
//! once per view, then draws them. That costs one colour copy per view and
//! one full-screen-sized texture; nothing else.
//!
//! Refract: the scene behind, offset by the normal map's xy
//! (`$normalmap`/`$bumpmap`, `$bumpframe`, `$bumptransform`) times
//! `$refractamount`, softened by `$bluramount`, tinted by `$refracttint`
//! and `$refracttinttexture`, plus an `$envmap` reflection times
//! `$envmaptint`; `$basetexture` (frames, dirt) is drawn over it by its alpha,
//! lit by the lightmap.
//!
//! Water: the same refraction, faded into `$fogcolor` from `$fogstart` to
//! `$fogend` along an assumed depth below the surface (the scene depth isn't
//! available here), mixed with a reflection (`$envmap`, else the map's sky)
//! by a Fresnel term, tinted by `$refracttint` / `$reflecttint`. Goo
//! (`nature/toxicslime*`) is water with a near, murky fog, so it comes out
//! mostly opaque green-brown, scrolling with its normal map.
//!
//! Proxies (`TextureScroll` on `$bumptransform`, `AnimatedTexture` on
//! `$bumpframe`...) run every frame through [`crate::proxy`].
//!
//! Estimates (not measured against the game): the screen-space refraction
//! scale [`REFRACT_SCALE`] and its distance falloff, and the assumed water
//! depth [`WATER_DEPTH`]. Both are constants in `glass.wgsl` too.
//!
//! Map water is measured instead: [`subdivide`] cuts the surface into a grid
//! and [`WaterFloor`] finds the depth to the geometry below each vertex,
//! carried in the vertex colour's alpha ([`FLAG_WATER_DEPTH`]). Source fogs
//! by the real depth behind the surface; sp_a1_wakeup's shallow pool
//! ($fogend 30) came out as a flat yellow sheet with the assumed 64.

use crate::kv::Kv;
use crate::proxy::{MaterialVars, ProxyContext, ProxyProgram};
use crate::vmt::MaterialInfo;
use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use std::collections::HashSet;
use std::sync::Arc;

/// Screen-space offset per unit of normal map xy and `$refractamount`, at
/// up to 128 units away (it falls off as 128 / distance beyond).
pub const REFRACT_SCALE: f32 = 0.1;
/// Water depth (Source units) assumed below a surface for its fog.
pub const WATER_DEPTH: f32 = 64.0;

pub const FLAG_WATER: u32 = 1;
pub const FLAG_NORMAL: u32 = 2;
pub const FLAG_BASE: u32 = 4;
pub const FLAG_TINT_TEXTURE: u32 = 8;
pub const FLAG_ENVMAP: u32 = 16;
pub const FLAG_LIGHTMAP: u32 = 32;
pub const FLAG_VERTEX_LIGHT: u32 = 64;
/// `$lightmapwaterfog`: the water's fog colour is lit by its lightmap.
pub const FLAG_LIGHTMAP_FOG: u32 = 128;
/// Water whose vertex colour alpha is its depth (units) to the floor below.
pub const FLAG_WATER_DEPTH: u32 = 256;
/// Water reflecting the world (`$reflecttexture _rt_WaterReflection` without
/// `$reflectskyboxonly`): samples [`WaterReflection`], a mirrored view of the
/// scene above the water plane, x-flipped (see `p2_game::water_reflection`).
pub const FLAG_WORLD_REFLECT: u32 = 512;
/// Grid step (Source units) [`subdivide`] cuts map water into.
pub const WATER_GRID: f32 = 32.0;
/// Depth (units) given to water with nothing found below it.
pub const WATER_BOTTOMLESS: f32 = 4096.0;
/// Ground up to this far (units) above a water vertex buries it (depth 0).
pub const WATER_BANK: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct GlassParams {
    /// rgb `$refracttint`, w `$refractamount`.
    pub refract_tint: Vec4,
    /// rgb `$reflecttint` (water), w unused.
    pub reflect_tint: Vec4,
    /// rgb `$envmaptint` (refract), w `$bluramount`.
    pub envmap_tint: Vec4,
    /// rgb linear `$fogcolor`, w `$fogstart` (units).
    pub fog_color: Vec4,
    /// x `$fogend` (units), y exposure, z metres per unit, w unused.
    pub fog: Vec4,
    /// `$bumptransform` rows: `uv' = (dot(u.xyz, (u, v, 1)), dot(v.xyz, ...))`.
    pub normal_u: Vec4,
    pub normal_v: Vec4,
    /// `$basetexturetransform` rows (base and tint texture).
    pub base_u: Vec4,
    pub base_v: Vec4,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlassKey {
    pub double_sided: bool,
}

impl From<&GlassMaterial> for GlassKey {
    fn from(m: &GlassMaterial) -> Self {
        GlassKey { double_sided: m.double_sided }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GlassKey)]
pub struct GlassMaterial {
    #[uniform(0)]
    pub params: GlassParams,
    #[texture(1)]
    #[sampler(2)]
    pub normal_map: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub base_texture: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub tint_texture: Option<Handle<Image>>,
    #[texture(7, dimension = "cube")]
    #[sampler(8)]
    pub envmap: Option<Handle<Image>>,
    #[texture(9)]
    #[sampler(10)]
    pub lightmap: Option<Handle<Image>>,
    #[texture(11)]
    #[sampler(12)]
    pub reflection: Option<Handle<Image>>,
    pub double_sided: bool,
}

/// Render target of the water reflection camera (Source's
/// `_rt_WaterReflection`), shared by every world-reflecting water material.
#[derive(Resource, Clone)]
pub struct WaterReflection(pub Handle<Image>);

impl FromWorld for WaterReflection {
    fn from_world(world: &mut World) -> Self {
        let mut image = Image::new_target_texture(16, 16, bevy::render::render_resource::TextureFormat::Rgba16Float, None);
        image.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
        Self(world.resource_mut::<Assets<Image>>().add(image))
    }
}

/// A world-reflecting water surface: its plane height (Bevy y, metres).
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldReflectWater {
    pub height: f32,
}

/// Does this water material reflect the world, not just the sky?
pub fn reflects_world(p: &Kv) -> bool {
    p.get("$reflecttexture").is_some_and(|t| t.eq_ignore_ascii_case("_rt_WaterReflection"))
        && p.get_f32("$reflectskyboxonly").unwrap_or(0.0) == 0.0
}

impl Material for GlassMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://source_assets/shaders/glass.wgsl".into()
    }

    // Opaque: the shader composes the final colour itself (scene behind,
    // tint, reflection), so no blending; Bevy draws it in the transmissive
    // pass because it reads the transmission texture.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn reads_view_transmission_texture(&self) -> bool {
        true
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// Is this material drawn as glass/water by [`GlassMaterial`]?
pub fn is_glass(m: &MaterialInfo) -> bool {
    matches!(m.shader.as_str(), "refract" | "water")
}

/// The normal map variable (`$normalmap`, or `$bumpmap` in some refract materials).
pub fn normal_var(p: &Kv) -> Option<&'static str> {
    ["$normalmap", "$bumpmap"].into_iter().find(|k| crate::vmt::texture_param(p, k).is_some())
}

/// Split each triangle into a grid of triangles no longer than `step`
/// (metres) along their longest edge, interpolating every attribute.
pub fn subdivide(d: &crate::geometry::MeshData, step: f32) -> crate::geometry::MeshData {
    let mut out = crate::geometry::MeshData::default();
    let lerp3 = |a: [f32; 3], b: [f32; 3], c: [f32; 3], w: Vec3| (Vec3::from(a) * w.x + Vec3::from(b) * w.y + Vec3::from(c) * w.z).to_array();
    let lerp2 = |a: [f32; 2], b: [f32; 2], c: [f32; 2], w: Vec3| (Vec2::from(a) * w.x + Vec2::from(b) * w.y + Vec2::from(c) * w.z).to_array();
    let lerp4 = |a: [f32; 4], b: [f32; 4], c: [f32; 4], w: Vec3| (Vec4::from(a) * w.x + Vec4::from(b) * w.y + Vec4::from(c) * w.z).to_array();
    for t in d.indices.chunks_exact(3) {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        let p = [a, b, c].map(|i| Vec3::from(d.positions[i]));
        let longest = (p[1] - p[0]).length().max((p[2] - p[1]).length()).max((p[0] - p[2]).length());
        let n = ((longest / step).ceil() as usize).clamp(1, 64);
        // vertex (i, j): i steps toward b, j toward c; rows[j] its first index
        let mut rows = Vec::with_capacity(n + 1);
        for j in 0..=n {
            rows.push(out.positions.len() as u32);
            for i in 0..=n - j {
                let w = Vec3::new((n - i - j) as f32, i as f32, j as f32) / n as f32;
                out.positions.push(lerp3(d.positions[a], d.positions[b], d.positions[c], w));
                out.normals.push(lerp3(d.normals[a], d.normals[b], d.normals[c], w));
                out.uv0.push(lerp2(d.uv0[a], d.uv0[b], d.uv0[c], w));
                out.uv1.push(lerp2(d.uv1[a], d.uv1[b], d.uv1[c], w));
                out.colors.push(lerp4(d.colors[a], d.colors[b], d.colors[c], w));
            }
        }
        let at = |i: usize, j: usize| rows[j] + i as u32;
        for j in 0..n {
            for i in 0..n - j {
                out.indices.extend([at(i, j), at(i + 1, j), at(i, j + 1)]);
                if i + 1 < n - j {
                    out.indices.extend([at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
                }
            }
        }
    }
    out
}

/// Triangles binned on a horizontal grid, for the depth below a point.
pub struct WaterFloor {
    tris: Vec<[Vec3; 3]>,
    cells: std::collections::HashMap<(i32, i32), Vec<u32>>,
    cell: f32,
}

impl WaterFloor {
    /// `cell`: bin size in metres.
    pub fn new(tris: Vec<[Vec3; 3]>, cell: f32) -> Self {
        let mut cells: std::collections::HashMap<(i32, i32), Vec<u32>> = Default::default();
        for (k, t) in tris.iter().enumerate() {
            let lo = t[0].min(t[1]).min(t[2]);
            let hi = t[0].max(t[1]).max(t[2]);
            for x in (lo.x / cell).floor() as i32..=(hi.x / cell).floor() as i32 {
                for z in (lo.z / cell).floor() as i32..=(hi.z / cell).floor() as i32 {
                    cells.entry((x, z)).or_default().push(k as u32);
                }
            }
        }
        Self { tris, cells, cell }
    }

    /// Distance (metres) straight down from `p` to the nearest triangle; 0
    /// when ground lies just above `p` (within `above` metres: water edges
    /// run under the banks).
    pub fn depth_below(&self, p: Vec3, above: f32) -> Option<f32> {
        let key = ((p.x / self.cell).floor() as i32, (p.z / self.cell).floor() as i32);
        let mut best: Option<f32> = None;
        for &k in self.cells.get(&key).into_iter().flatten() {
            let [a, b, c] = self.tris[k as usize];
            let (e1, e2, q) = (Vec2::new(b.x - a.x, b.z - a.z), Vec2::new(c.x - a.x, c.z - a.z), Vec2::new(p.x - a.x, p.z - a.z));
            let det = e1.perp_dot(e2);
            if det.abs() < 1e-8 {
                continue;
            }
            let (u, v) = (q.perp_dot(e2) / det, e1.perp_dot(q) / det);
            const EPS: f32 = 1e-4;
            if u < -EPS || v < -EPS || u + v > 1.0 + EPS {
                continue;
            }
            let y = a.y + (b.y - a.y) * u + (c.y - a.y) * v;
            let d = p.y - y;
            if d >= -above && best.is_none_or(|x| d.abs() < x.abs()) {
                best = Some(d);
            }
        }
        best.map(|d| d.max(0.0))
    }
}

fn rows(m: [f32; 6]) -> (Vec4, Vec4) {
    (Vec4::new(m[0], m[1], m[2], 0.0), Vec4::new(m[3], m[4], m[5], 0.0))
}

fn srgb_to_linear(c: [f32; 3]) -> Vec3 {
    Vec3::from(c).max(Vec3::ZERO).powf(2.2)
}

/// Shader parameters from a material's variables; `flags` are the texture
/// and lighting flags the caller knows.
pub fn glass_params(water: bool, v: &MaterialVars, flags: u32, exposure: f32) -> GlassParams {
    let refract = v.vec3("$refracttint").unwrap_or([1.0; 3]);
    let reflect = v.vec3("$reflecttint").unwrap_or([1.0; 3]);
    let envtint = v.vec3("$envmaptint").unwrap_or([1.0; 3]);
    let fog = srgb_to_linear(v.vec3("$fogcolor").unwrap_or([0.0; 3]));
    let (fog_start, fog_end) = (v.f32("$fogstart").unwrap_or(0.0), v.f32("$fogend").unwrap_or(512.0));
    let (normal_u, normal_v) = rows(v.transform("$bumptransform"));
    let flow = water && v.get("$flowmap").is_some();
    let (base_u, base_v) = rows(v.transform("$basetexturetransform"));
    GlassParams {
        refract_tint: Vec4::new(refract[0], refract[1], refract[2], v.f32("$refractamount").unwrap_or(0.5)),
        // w: normal strength (flowing water's $flow_bumpstrength).
        reflect_tint: Vec4::new(reflect[0], reflect[1], reflect[2], if flow { v.f32("$flow_bumpstrength").unwrap_or(1.0) } else { 1.0 }),
        envmap_tint: Vec4::new(envtint[0], envtint[1], envtint[2], v.f32("$bluramount").unwrap_or(0.0)),
        fog_color: fog.extend(fog_start),
        // w: flowing water samples its normal map in world space, one tile
        // per $flow_normaluvscale units (0: the mesh's texture coordinates).
        fog: Vec4::new(fog_end.max(fog_start + 1.0), exposure, shared::SOURCE_UNIT, if flow { v.f32("$flow_normaluvscale").unwrap_or(1.0).max(1.0) } else { 0.0 }),
        normal_u,
        normal_v,
        base_u,
        base_v,
        flags: flags | if water { FLAG_WATER } else { 0 } | if water && v.f32("$lightmapwaterfog").unwrap_or(0.0) != 0.0 { FLAG_LIGHTMAP_FOG } else { 0 },
    }
}

/// A glass material's proxies, run once per frame per material.
#[derive(Component, Clone)]
pub struct GlassProxies {
    pub program: Arc<ProxyProgram>,
    pub params: Kv,
    /// Frames of the normal map and the base texture (empty: one frame).
    pub normal_frames: Vec<Handle<Image>>,
    pub base_frames: Vec<Handle<Image>>,
}

pub struct GlassPlugin;

impl Plugin for GlassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/glass.wgsl");
        app.add_plugins(MaterialPlugin::<GlassMaterial>::default())
            .init_resource::<WaterReflection>()
            .add_systems(PostUpdate, (update_glass_proxies, follow_exposure).chain());
    }
}

fn update_glass_proxies(time: Res<Time>, mut materials: ResMut<Assets<GlassMaterial>>, q: Query<(&GlassProxies, &MeshMaterial3d<GlassMaterial>)>) {
    let mut done = HashSet::new();
    let t = time.elapsed_secs();
    for (p, handle) in &q {
        if !done.insert(handle.0.id()) {
            continue;
        }
        let Some(cur) = materials.get(&handle.0) else { continue };
        let (nn, nb) = (p.normal_frames.len().max(1) as u32, p.base_frames.len().max(1) as u32);
        let count = |var: &str| match var {
            "$normalmap" | "$bumpmap" => nn,
            "$basetexture" => nb,
            _ => 1,
        };
        let mut ctx = ProxyContext { time: t, texture_frame: 0, speaking: 1.0, entity_random: 0.0, noise: (t * 1000.0) as u32 | 1, frame_count: &count };
        let set = p.program.eval(&p.params, &mut ctx);
        let vars = MaterialVars { base: &p.params, set: &set };
        let water = cur.params.flags & FLAG_WATER != 0;
        let params = glass_params(water, &vars, cur.params.flags & !(FLAG_WATER | FLAG_LIGHTMAP_FOG), cur.params.fog.y);
        let pick = |frames: &[Handle<Image>], var: &str| {
            (!frames.is_empty()).then(|| frames[vars.f32(var).unwrap_or(0.0).max(0.0) as usize % frames.len()].clone())
        };
        let normal = pick(&p.normal_frames, "$bumpframe");
        let base = pick(&p.base_frames, "$frame");
        let same = params == cur.params
            && normal.as_ref().is_none_or(|n| cur.normal_map.as_ref() == Some(n))
            && base.as_ref().is_none_or(|b| cur.base_texture.as_ref() == Some(b));
        if same {
            continue;
        }
        if let Some(mut m) = materials.get_mut(&handle.0) {
            m.params = params;
            if normal.is_some() {
                m.normal_map = normal;
            }
            if base.is_some() {
                m.base_texture = base;
            }
        }
    }
}

/// Keep every glass material's exposure (`fog.y`) at the Valve shaders'
/// tonemap scale, so its fog colour, reflections and lit dirt match the
/// scene around it. Only touched when it moves by more than 1%.
fn follow_exposure(exposure: Option<Res<crate::valve_render::ValveExposure>>, mut materials: ResMut<Assets<GlassMaterial>>) {
    let Some(e) = exposure.map(|e| e.0) else { return };
    let stale: Vec<AssetId<GlassMaterial>> =
        materials.iter().filter(|(_, m)| (m.params.fog.y - e).abs() > 0.01 * e.max(0.01)).map(|(id, _)| id).collect();
    for id in stale {
        if let Some(mut m) = materials.get_mut(id) {
            m.params.fog.y = e;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdivide_keeps_area_and_splits_long_edges() {
        let mut d = crate::geometry::MeshData::default();
        for p in [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 0.0, 4.0]] {
            d.positions.push(p);
            d.normals.push([0.0, 1.0, 0.0]);
            d.uv0.push([p[0], p[2]]);
            d.uv1.push([0.0; 2]);
            d.colors.push([1.0; 4]);
        }
        d.indices = vec![0, 1, 2];
        let s = subdivide(&d, 1.0);
        assert_eq!(s.indices.len() / 3, 36); // longest edge 5.66: 6 x 6
        let area: f32 = s
            .indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from(s.positions[i as usize]));
                (b - a).cross(c - a).length() / 2.0
            })
            .sum();
        assert!((area - 8.0).abs() < 1e-4);
        for t in s.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from(s.positions[i as usize]));
            // same winding (up-facing) as the source triangle
            assert!((c - a).cross(b - a).y > 0.0);
            assert!((b - a).length() <= 1.0 && (c - b).length() <= 1.0 && (a - c).length() <= 1.0);
        }
    }

    #[test]
    fn water_floor_depth_is_the_nearest_surface_below() {
        let quad = |y: f32| [[Vec3::new(-5.0, y, -5.0), Vec3::new(5.0, y, -5.0), Vec3::new(-5.0, y, 5.0)], [Vec3::new(5.0, y, -5.0), Vec3::new(5.0, y, 5.0), Vec3::new(-5.0, y, 5.0)]];
        let mut tris = quad(-3.0).to_vec();
        tris.extend(quad(-0.5));
        tris.extend(quad(2.0)); // above: ignored
        let f = WaterFloor::new(tris, 1.0);
        assert!((f.depth_below(Vec3::new(1.2, 0.0, -2.7), 0.1).unwrap() - 0.5).abs() < 1e-5);
        assert!((f.depth_below(Vec3::new(1.2, -1.0, 0.3), 0.1).unwrap() - 2.0).abs() < 1e-5);
        assert_eq!(f.depth_below(Vec3::new(9.0, 0.0, 0.0), 0.1), None);
        // under a bank: the ground just above wins over the floor far below
        assert_eq!(f.depth_below(Vec3::new(0.0, -0.6, 0.0), 0.2), Some(0.0));
    }

    fn info(text: &str) -> MaterialInfo {
        let kv = crate::kv::parse(text);
        let (shader, body) = kv.root().unwrap();
        crate::vmt::material_info("test", shader, body)
    }

    #[test]
    fn refract_parameters() {
        let m = info(
            r#""Refract"
            {
                "$normalmap" "glass/glasswindow_refract01_normal"
                "$refractamount" ".15"
                "$refracttint" "[.8 .9 1]"
                "$bluramount" "1"
                "$envmap" "env_cubemap"
                "$envmaptint" "[.3 .3 .3]"
                "$bumptransform" "center .5 .5 scale 2 2 rotate 0 translate 0 0"
            }"#,
        );
        assert!(is_glass(&m) && m.is_glassy);
        assert_eq!(normal_var(&m.params), Some("$normalmap"));
        let p = glass_params(false, &MaterialVars::of(&m.params), FLAG_NORMAL, 3.0);
        assert_eq!(p.refract_tint, Vec4::new(0.8, 0.9, 1.0, 0.15));
        assert_eq!(p.envmap_tint, Vec4::new(0.3, 0.3, 0.3, 1.0));
        assert_eq!(p.flags, FLAG_NORMAL);
        assert_eq!(p.fog.y, 3.0);
        // scale 2 about the centre
        assert_eq!(p.normal_u, Vec4::new(2.0, 0.0, -0.5, 0.0));
        assert_eq!(p.base_u, Vec4::new(1.0, 0.0, 0.0, 0.0));
        // older refract materials name the normal map $bumpmap
        assert_eq!(normal_var(&info(r#""Refract" { "$bumpmap" "x" }"#).params), Some("$bumpmap"));
    }

    #[test]
    fn water_parameters_and_scrolling() {
        let m = info(
            r#""Water"
            {
                "$normalmap" "nature/toxicslime_normal"
                "$fogcolor" "{60 52 26}"
                "$fogstart" "0"
                "$fogend" "48"
                "$reflecttint" "[.3 .3 .25]"
                "Proxies"
                {
                    "TextureScroll" { "texturescrollvar" "$bumptransform" "texturescrollrate" .1 "texturescrollangle" 90 }
                }
            }"#,
        );
        assert!(is_glass(&m));
        let v = MaterialVars::of(&m.params);
        let p = glass_params(true, &v, FLAG_NORMAL, 1.0);
        assert_eq!(p.flags, FLAG_NORMAL | FLAG_WATER);
        assert!((p.fog_color.x - (60.0f32 / 255.0).powf(2.2)).abs() < 1e-5);
        assert_eq!((p.fog_color.w, p.fog.x), (0.0, 48.0));
        // the proxy scrolls the normal map
        let count = |_: &str| 1;
        let mut ctx = ProxyContext { time: 2.0, texture_frame: 0, speaking: 1.0, entity_random: 0.0, noise: 1, frame_count: &count };
        let set = ProxyProgram::compile(&m.proxies).eval(&m.params, &mut ctx);
        let p = glass_params(true, &MaterialVars { base: &m.params, set: &set }, FLAG_NORMAL, 1.0);
        assert!((p.normal_v.z - 0.2).abs() < 1e-4, "{:?}", p.normal_v);
        // unset fog end can't make a zero-length fog range
        let p = glass_params(true, &MaterialVars::of(&info(r#""Water" { "$fogstart" "600" }"#).params), 0, 1.0);
        assert!(p.fog.x > p.fog_color.w);
    }

    /// `glass.wgsl` with Bevy's imports replaced by stubs of the same shape
    /// and every `#ifdef` taken, parsed and validated.
    #[test]
    fn shader_validates() {
        let stub = "struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) world_position: vec4<f32>, \
            @location(1) world_normal: vec3<f32>, @location(2) uv: vec2<f32>, @location(3) uv_b: vec2<f32>, @location(4) color: vec4<f32> };\n\
            struct View { world_position: vec3<f32> };\n\
            @group(0) @binding(0) var<uniform> view: View;\n\
            @group(0) @binding(24) var view_transmission_texture: texture_2d<f32>;\n\
            @group(0) @binding(25) var view_transmission_sampler: sampler;\n";
        let src = include_str!("shaders/glass.wgsl").replace("#{MATERIAL_BIND_GROUP}", "2");
        let mut body = String::new();
        let mut skip_import = false;
        let mut in_else = false;
        for line in src.lines() {
            let t = line.trim();
            // `#import a::{` ... `}` spans lines
            if t.starts_with("#import") {
                skip_import = t.ends_with('{');
                continue;
            }
            if skip_import {
                skip_import = t != "}";
                continue;
            }
            match t {
                _ if t.starts_with("#ifdef") => continue,
                "#else" => in_else = true,
                "#endif" => in_else = false,
                _ if !in_else => {
                    body.push_str(line);
                    body.push('\n');
                }
                _ => {}
            }
        }
        let full = format!("{stub}{body}");
        let module = naga::front::wgsl::parse_str(&full).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&full)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{e:?}"));
    }

    #[test]
    fn other_glassy_shaders_stay_on_the_pane() {
        assert!(!is_glass(&info(r#""LightmappedReflective" { }"#)));
        assert!(!is_glass(&info(r#""LightmappedGeneric" { "$translucent" 1 }"#)));
    }
}
