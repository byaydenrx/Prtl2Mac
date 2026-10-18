//! VMT materials: patch resolution and the handful of parameters we render
//! and collide with.
//!
//! `vmt-parser` was not used: its typed serde model rejects shaders it does
//! not know and drops the `%` compile keys (`%noportal`, `%compileclip`...)
//! that portalability and tool detection depend on.

use crate::kv::{self, Kv, KvValue};

/// What a material is for, as far as rendering and collision care.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    /// A normal drawn surface.
    None,
    /// Not drawn but solid (nodraw, invisible, black-on-no-draw...).
    NoDraw,
    /// Player/NPC clip: invisible, collides with players only.
    PlayerClip,
    /// Full clip: invisible, collides with everything.
    Clip,
    /// Triggers, hints, skips, areaportals, occluders: not drawn, not solid.
    NonSolid,
    /// 2D/3D sky: not drawn by the brush renderer, the skybox is.
    Sky,
}

/// How a surface is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendKind {
    Opaque,
    /// Alpha test with a cutoff.
    Mask,
    /// `$translucent`
    Blend,
    /// `$additive`
    Additive,
}

#[derive(Clone, Debug)]
pub struct MaterialInfo {
    /// Normalised material name, no `materials/` prefix or `.vmt` (e.g. `metal/black_wall_metal_002b`).
    pub name: String,
    /// Lowercased shader (`lightmappedgeneric`, `unlitgeneric`, ...). `missing` if the VMT was not found.
    pub shader: String,
    pub base_texture: Option<String>,
    pub base_texture2: Option<String>,
    pub blend: BlendKind,
    pub alpha_cutoff: f32,
    pub alpha: f32,
    pub color: [f32; 3],
    pub no_cull: bool,
    /// Drawn without lighting (UnlitGeneric, $selfillum on world, sprites).
    pub unlit: bool,
    /// `%noportal 1`
    pub no_portal: bool,
    pub surface_prop: Option<String>,
    pub tool: ToolKind,
    pub decal: bool,
    /// Water, refraction, etc. that we draw as a tinted translucent pane.
    pub is_glassy: bool,
    /// Every resolved `$param` (keys lowercased), for the Valve-shader path.
    pub params: Kv,
    /// The `Proxies { ... }` block, in order. Empty for static materials.
    pub proxies: Vec<MaterialProxy>,
}

/// One proxy of a material's `Proxies` block, e.g.
/// `"Sine" { "sineperiod" 2 "resultVar" "$selfillumtint" }`.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialProxy {
    /// Lowercased proxy name (`animatedtexture`, `sine`, `toggletexture`...).
    pub name: String,
    /// Its keys and values, lowercased (material variable names are not
    /// case-sensitive), in file order.
    pub keys: Vec<(String, String)>,
}

impl MaterialProxy {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keys.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The material variable this proxy writes: `resultvar`, or for the
    /// texture proxies the frame / transform variable they drive.
    pub fn result_var(&self) -> Option<&str> {
        let key = match self.name.as_str() {
            "animatedtexture" => "animatedtextureframenumvar",
            "texturescroll" => "texturescrollvar",
            "toggletexture" => {
                return self.get("toggletextureframenumvar").or_else(|| self.get("toggleframe"));
            }
            _ => "resultvar",
        };
        self.get(key)
    }
}

/// The `Proxies` block of a resolved VMT (`proxies` key, any case).
pub fn parse_proxies(p: &Kv) -> Vec<MaterialProxy> {
    let Some(block) = p.block("proxies") else { return Vec::new() };
    block
        .entries
        .iter()
        .filter_map(|(name, v)| match v {
            KvValue::Block(b) => Some(MaterialProxy {
                name: name.clone(),
                keys: b
                    .entries
                    .iter()
                    .filter_map(|(k, v)| match v {
                        KvValue::Str(s) => Some((k.clone(), s.trim().to_ascii_lowercase())),
                        KvValue::Block(_) => None,
                    })
                    .collect(),
            }),
            KvValue::Str(_) => None,
        })
        .collect()
}

impl MaterialInfo {
    pub fn missing(name: &str) -> Self {
        let mut m = MaterialInfo {
            name: name.to_string(),
            shader: "missing".into(),
            base_texture: None,
            base_texture2: None,
            blend: BlendKind::Opaque,
            alpha_cutoff: 0.5,
            alpha: 1.0,
            color: [1.0; 3],
            no_cull: false,
            unlit: false,
            no_portal: false,
            surface_prop: None,
            tool: ToolKind::None,
            decal: false,
            is_glassy: false,
            params: Kv::default(),
            proxies: Vec::new(),
        };
        m.tool = tool_from_name(name);
        m
    }

    /// Whether the brush renderer should emit geometry for this material.
    pub fn is_drawn(&self) -> bool {
        self.tool == ToolKind::None
    }
}

/// Classify well-known tool textures by name.
pub fn tool_from_name(name: &str) -> ToolKind {
    let n = name.trim_start_matches("materials/");
    let Some(t) = n.strip_prefix("tools/") else {
        return ToolKind::None;
    };
    // toolsblack is a real, drawn black surface in Portal 2.
    if t.starts_with("toolsblack") || t.starts_with("toolswhite") {
        return ToolKind::None;
    }
    if t.starts_with("toolsplayerclip") || t.starts_with("toolsnpcclip") {
        ToolKind::PlayerClip
    } else if t.starts_with("toolsclip") {
        ToolKind::Clip
    } else if t.starts_with("toolsskybox") || t.starts_with("toolssky") {
        ToolKind::Sky
    } else if t.starts_with("toolsnodraw")
        || t.starts_with("toolsinvisible")
        || t.starts_with("toolsblocklight")
        || t.starts_with("toolsblockbullets")
        || t.starts_with("toolsblock_los")
        || t.starts_with("toolsblockbright")
    {
        ToolKind::NoDraw
    } else {
        // trigger, hint, skip, areaportal, occluder, fog, origin, ...
        ToolKind::NonSolid
    }
}

/// Strip a texture/material path to its canonical form.
pub fn canonical_name(path: &str) -> String {
    let p = crate::fs::normalize_path(path);
    let p = p.strip_prefix("materials/").unwrap_or(&p);
    p.strip_suffix(".vmt")
        .or_else(|| p.strip_suffix(".vtf"))
        .unwrap_or(p)
        .to_string()
}

/// Load and resolve a VMT (following `patch` includes) into a flat parameter block.
/// Returns `(shader, params)`.
pub fn load_vmt(read: &dyn Fn(&str) -> Option<Vec<u8>>, name: &str) -> Option<(String, Kv)> {
    load_vmt_depth(read, &format!("materials/{}.vmt", canonical_name(name)), 0)
}

fn load_vmt_depth(
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    path: &str,
    depth: u32,
) -> Option<(String, Kv)> {
    if depth > 8 {
        return None;
    }
    let data = read(path)?;
    let text = String::from_utf8_lossy(&data);
    let kv = kv::parse(&text);
    let (shader, body) = kv.root()?;
    if shader == "patch" {
        let include = body.get("include")?;
        let (base_shader, mut params) = load_vmt_depth(read, &crate::fs::normalize_path(include), depth + 1)?;
        for section in ["insert", "replace"] {
            if let Some(b) = body.block(section) {
                for (k, v) in &b.entries {
                    params.set(k, v.clone());
                }
            }
        }
        return Some((base_shader, params));
    }
    Some((shader.to_string(), body.clone()))
}

pub fn texture_param(p: &Kv, key: &str) -> Option<String> {
    let v = p.get(key)?.trim();
    if v.is_empty() || v.starts_with('_') {
        // `_rt_*` render targets and similar engine textures
        return None;
    }
    Some(canonical_name(v))
}

/// Interpret a resolved VMT.
pub fn material_info(name: &str, shader: &str, p: &Kv) -> MaterialInfo {
    let mut m = MaterialInfo::missing(name);
    m.shader = shader.to_string();
    m.params = p.clone();
    m.proxies = parse_proxies(p);
    m.base_texture = texture_param(p, "$basetexture");
    if m.base_texture.is_none() && matches!(shader, "eyerefract" | "eyes" | "eyeball") {
        // Eyes have no base texture: the iris is projected onto them (the
        // mesh's UVs are made by `ModelData::iris_uvs`).
        m.base_texture = texture_param(p, "$iris");
    }
    m.base_texture2 = texture_param(p, "$basetexture2");
    m.alpha = p.get_f32("$alpha").unwrap_or(1.0);
    m.color = p
        .get("$color")
        .or_else(|| p.get("$color2"))
        .and_then(kv::parse_color)
        .unwrap_or([1.0; 3]);
    m.no_cull = p.get_bool("$nocull");
    m.no_portal = p.get_bool("%noportal");
    m.surface_prop = p.get("$surfaceprop").map(|s| s.to_ascii_lowercase());
    m.decal = p.get_bool("$decal");

    m.blend = if p.get_bool("$additive") {
        BlendKind::Additive
    } else if p.get_bool("$translucent") || m.alpha < 1.0 {
        BlendKind::Blend
    } else if p.get_bool("$alphatest") {
        BlendKind::Mask
    } else {
        BlendKind::Opaque
    };
    m.alpha_cutoff = p.get_f32("$alphatestreference").filter(|v| *v > 0.0).unwrap_or(0.5);

    m.unlit = matches!(
        shader,
        "unlitgeneric" | "unlittwotexture" | "sprite" | "spritecard" | "splinerope" | "cable" | "monitorscreen"
    );

    // Compile keys decide what vbsp did with the brush side.
    m.tool = if p.get_bool("%compilesky") || p.get_bool("%compile2dsky") {
        ToolKind::Sky
    } else if p.get_bool("%compileplayerclip") || p.get_bool("%compilenpcclip") {
        ToolKind::PlayerClip
    } else if p.get_bool("%compileclip") {
        ToolKind::Clip
    } else if p.get_bool("%compilenodraw") || p.get_bool("%compileinvisible") {
        ToolKind::NoDraw
    } else if p.get_bool("%compiletrigger")
        || p.get_bool("%compilehint")
        || p.get_bool("%compileskip")
        || p.get_bool("%compileorigin")
        || p.get_bool("%compileareaportal")
        || p.get_bool("%compileoccluder")
        || p.get_bool("%compilefog")
    {
        ToolKind::NonSolid
    } else {
        tool_from_name(name)
    };

    m.is_glassy = matches!(shader, "water" | "refract" | "lightmappedreflective" | "portalrefract");
    if m.is_glassy && m.blend == BlendKind::Opaque {
        m.blend = BlendKind::Blend;
    }
    m
}

/// Resolve a material name to its info through a file reader, falling back
/// to a `missing` material when the VMT cannot be found.
pub fn resolve(read: &dyn Fn(&str) -> Option<Vec<u8>>, name: &str) -> MaterialInfo {
    let canon = canonical_name(name);
    match load_vmt(read, &canon) {
        Some((shader, params)) => material_info(&canon, &shader, &params),
        None => MaterialInfo::missing(&canon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn reader(files: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<Vec<u8>> {
        move |p: &str| files.get(p).map(|s| s.as_bytes().to_vec())
    }

    #[test]
    fn resolves_patch_materials() {
        let files = HashMap::from([
            (
                "materials/maps/test/metal/floor_1_2_3.vmt",
                r#""patch" { "include" "materials/metal/floor.vmt" "insert" { "$envmap" "maps/test/c1" } "replace" { "$alphatest" 1 } }"#,
            ),
            (
                "materials/metal/floor.vmt",
                r#""LightmappedGeneric" { "$basetexture" "Metal\Floor" "%noportal" "1" "$surfaceprop" "Metal" }"#,
            ),
        ]);
        let r = reader(files);
        let m = resolve(&r, "maps/test/metal/floor_1_2_3");
        assert_eq!(m.shader, "lightmappedgeneric");
        assert_eq!(m.base_texture.as_deref(), Some("metal/floor"));
        assert!(m.no_portal);
        assert_eq!(m.blend, BlendKind::Mask);
        assert_eq!(m.surface_prop.as_deref(), Some("metal"));
    }

    fn info(text: &str) -> MaterialInfo {
        let kv = kv::parse(text);
        let (shader, body) = kv.root().unwrap();
        material_info("test", shader, body)
    }

    #[test]
    fn parses_proxies() {
        // a personality core's eye light
        let m = info(
            r#""VertexLitGeneric"
            {
                "$basetexture" "models/npcs/personality_sphere/personality_sphere_lights"
                "$selfillum" 1
                "$selfillumtint" "[1 1 1]"
                "Proxies"
                {
                    "LightedMouth"
                    {
                        "resultVar" "$selfIllumTint"
                    }
                }
            }"#,
        );
        assert_eq!(m.proxies.len(), 1);
        assert_eq!(m.proxies[0].name, "lightedmouth");
        assert_eq!(m.proxies[0].result_var(), Some("$selfillumtint"));

        // water: an animated normal map scrolling, then a Sine on the tint
        let m = info(
            r#""Water"
            {
                "$normalmap" "liquids/water_river_normal_sharp"
                "$bumpframe" "0"
                "%compilewater" 1
                "Proxies"
                {
                    "AnimatedTexture"
                    {
                        "animatedtexturevar" "$normalmap"
                        "animatedtextureframenumvar" "$bumpframe"
                        "animatedtextureframerate" 30.00
                    }
                    "TextureScroll"
                    {
                        "texturescrollvar" "$bumptransform"
                        "texturescrollrate" .05
                        "texturescrollangle" 45.00
                    }
                    Sine // unquoted, with a comment
                    {
                        sineperiod 8
                        sinemin .5
                        sinemax 1
                        resultVar "$fogcolor[1]"
                    }
                }
            }"#,
        );
        let names: Vec<_> = m.proxies.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["animatedtexture", "texturescroll", "sine"]);
        assert_eq!(m.proxies[0].get("animatedtextureframerate"), Some("30.00"));
        assert_eq!(m.proxies[0].result_var(), Some("$bumpframe"));
        assert_eq!(m.proxies[1].result_var(), Some("$bumptransform"));
        assert_eq!(m.proxies[2].result_var(), Some("$fogcolor[1]"));
        assert_eq!(m.proxies[2].get("sineperiod"), Some("8"));

        // a static material has none
        assert!(info(r#""LightmappedGeneric" { "$basetexture" "x" }"#).proxies.is_empty());
    }

    #[test]
    fn patch_materials_keep_inserted_proxies() {
        let files = HashMap::from([
            (
                "materials/maps/sp_a1_intro2/signage/indicator_lights/indicator_lights_floor_1_2_3.vmt",
                r#""patch" { "include" "materials/signage/indicator_lights/indicator_lights_floor.vmt" "insert" { "$envmap" "maps/x/c1" } }"#,
            ),
            (
                "materials/signage/indicator_lights/indicator_lights_floor.vmt",
                r#""LightmappedGeneric"
                {
                    "$basetexture" "signage/indicator_lights/indicator_lights_floor"
                    "$decal" "1"
                    "$translucent" "1"
                    "Proxies"
                    {
                        "ToggleTexture"
                        {
                            "toggleTextureVar" "$basetexture"
                            "toggleTextureFrameNumVar" "$frame"
                            "toggleShouldWrap" "0"
                        }
                    }
                }"#,
            ),
        ]);
        let m = resolve(&reader(files), "maps/sp_a1_intro2/signage/indicator_lights/indicator_lights_floor_1_2_3");
        assert_eq!(m.proxies.len(), 1);
        let p = &m.proxies[0];
        assert_eq!(p.name, "toggletexture");
        assert_eq!(p.get("toggletexturevar"), Some("$basetexture"));
        assert_eq!(p.get("toggleshouldwrap"), Some("0"));
        assert_eq!(p.result_var(), Some("$frame"));
    }

    #[test]
    fn classifies_tools() {
        assert_eq!(tool_from_name("tools/toolsnodraw"), ToolKind::NoDraw);
        assert_eq!(tool_from_name("tools/toolsplayerclip"), ToolKind::PlayerClip);
        assert_eq!(tool_from_name("tools/toolstrigger"), ToolKind::NonSolid);
        assert_eq!(tool_from_name("tools/toolsskybox"), ToolKind::Sky);
        assert_eq!(tool_from_name("tools/toolsblack"), ToolKind::None);
        assert_eq!(tool_from_name("metal/black_wall_metal_002b"), ToolKind::None);
    }
}
