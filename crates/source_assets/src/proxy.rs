//! Material proxies at run time: the `Proxies { ... }` block of a VMT
//! ([`crate::vmt::MaterialProxy`]) compiled to a small program that rewrites
//! material variables every frame, the way Source's material system does
//! before drawing a proxied material.
//!
//! The results feed both draw paths: the Valve-shader path rebuilds the
//! surface's constant registers and picks texture frames from them
//! ([`crate::valve::SurfaceOverrides`]), the Bevy-material fallback updates
//! its tint, base texture frame and texture transform. Only surfaces whose
//! material has proxies are touched; static materials cost nothing.
//!
//! Per-entity inputs come from two resources a game sets:
//! [`TextureFrames`] (`env_texturetoggle`, read by `ToggleTexture`) and
//! [`SpeakingAmount`] (voice playback, read by `LightedMouth`). A surface
//! uses the value of its nearest ancestor (itself included) found there.
//!
//! Proxy behaviour follows the public Source SDK documentation of each
//! proxy's keys and the values observed in Portal 2's materials.

use crate::kv::{self, Kv};
use crate::material::SourceMaterial;
use crate::vmt::MaterialProxy;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------- values

/// A material variable's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Var {
    /// A scalar (`n == 1`) or a 2..4 component vector.
    Vec([f32; 4], u8),
    /// A 2D texture transform: `u' = m0 u + m1 v + m2`, `v' = m3 u + m4 v + m5`.
    Matrix([f32; 6]),
}

/// The identity texture transform.
pub const IDENTITY_TRANSFORM: [f32; 6] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];

impl Var {
    pub fn scalar(v: f32) -> Var {
        Var::Vec([v, 0.0, 0.0, 0.0], 1)
    }

    /// Component `i`; a scalar reads the same in every component (as a
    /// scalar stored into a vector variable fills it).
    pub fn comp(&self, i: usize) -> f32 {
        match *self {
            Var::Vec(v, 1) => v[0],
            Var::Vec(v, n) => {
                if i < n as usize {
                    v[i]
                } else {
                    0.0
                }
            }
            Var::Matrix(_) => 0.0,
        }
    }

    pub fn x(&self) -> f32 {
        self.comp(0)
    }

    fn width(&self) -> u8 {
        match *self {
            Var::Vec(_, n) => n,
            Var::Matrix(_) => 1,
        }
    }

    /// Parse a VMT value: a number, `[x y z]`, `{r g b}` (0..255), or a
    /// texture transform (`center .5 .5 scale 1 1 rotate 0 translate 0 0`).
    /// Texture names and other strings are `None`.
    pub fn parse(s: &str) -> Option<Var> {
        let t = s.trim();
        let first = t.split_whitespace().next()?.to_ascii_lowercase();
        if matches!(first.as_str(), "center" | "scale" | "rotate" | "translate") {
            return Some(Var::Matrix(parse_transform(t)));
        }
        let (inner, scale) = if let Some(i) = t.strip_prefix('[') {
            (i.trim_end_matches(']'), 1.0)
        } else if let Some(i) = t.strip_prefix('{') {
            (i.trim_end_matches('}'), 1.0 / 255.0)
        } else {
            // a plain number ("0.5", "1f"); texture names don't parse
            let numeric = t.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'));
            return numeric.then(|| kv::parse_f32(t)).flatten().map(Var::scalar);
        };
        let v: Vec<f32> = inner.split_whitespace().filter_map(kv::parse_f32).take(4).collect();
        if v.is_empty() {
            return None;
        }
        let mut out = [0.0; 4];
        for (o, x) in out.iter_mut().zip(&v) {
            *o = x * scale;
        }
        Some(Var::Vec(out, v.len() as u8))
    }
}

/// `center cx cy scale sx sy rotate deg translate tx ty`, built like the
/// material system does: translate(center + translate) * rotate * scale *
/// translate(-center). Missing parts keep their defaults.
pub fn parse_transform(s: &str) -> [f32; 6] {
    let toks: Vec<&str> = s.split_whitespace().collect();
    let (mut c, mut sc, mut rot, mut tr) = ([0.5f32, 0.5], [1.0f32, 1.0], 0.0f32, [0.0f32, 0.0]);
    let num = |i: usize| toks.get(i).and_then(|t| kv::parse_f32(t));
    let mut i = 0;
    while i < toks.len() {
        let pair = |i: usize, d: [f32; 2]| match (num(i + 1), num(i + 2)) {
            (Some(a), Some(b)) => ([a, b], 3),
            (Some(a), None) => ([a, a], 2),
            _ => (d, 1),
        };
        match toks[i].to_ascii_lowercase().as_str() {
            "center" => {
                let (v, n) = pair(i, c);
                c = v;
                i += n;
            }
            "scale" => {
                let (v, n) = pair(i, sc);
                sc = v;
                i += n;
            }
            "translate" => {
                let (v, n) = pair(i, tr);
                tr = v;
                i += n;
            }
            "rotate" => {
                rot = num(i + 1).unwrap_or(0.0);
                i += 2;
            }
            _ => i += 1,
        }
    }
    let (sin, cos) = rot.to_radians().sin_cos();
    let (a, b, d, e) = (cos * sc[0], -sin * sc[1], sin * sc[0], cos * sc[1]);
    [a, b, c[0] + tr[0] - (a * c[0] + b * c[1]), d, e, c[1] + tr[1] - (d * c[0] + e * c[1])]
}

/// Apply a texture transform to a UV.
pub fn transform_uv(m: &[f32; 6], uv: [f32; 2]) -> [f32; 2] {
    [m[0] * uv[0] + m[1] * uv[1] + m[2], m[3] * uv[0] + m[4] * uv[1] + m[5]]
}

/// A material's variables as drawn: proxy results over the VMT's values.
#[derive(Clone, Copy)]
pub struct MaterialVars<'a> {
    pub base: &'a Kv,
    /// Variables the proxies wrote this frame (later entries win).
    pub set: &'a [(String, Var)],
}

impl<'a> MaterialVars<'a> {
    /// Just the VMT's values.
    pub fn of(base: &'a Kv) -> Self {
        MaterialVars { base, set: &[] }
    }

    pub fn get(&self, key: &str) -> Option<Var> {
        self.set
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .or_else(|| self.base.get(key).and_then(Var::parse))
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        self.get(key).filter(|v| !matches!(v, Var::Matrix(_))).map(|v| v.x())
    }

    pub fn vec3(&self, key: &str) -> Option<[f32; 3]> {
        self.get(key).filter(|v| !matches!(v, Var::Matrix(_))).map(|v| [v.comp(0), v.comp(1), v.comp(2)])
    }

    /// A texture transform (identity when unset).
    pub fn transform(&self, key: &str) -> [f32; 6] {
        match self.get(key) {
            Some(Var::Matrix(m)) => m,
            _ => IDENTITY_TRANSFORM,
        }
    }
}

// ---------------------------------------------------------------- program

/// A proxy input: a literal or a material variable (optionally one component, `$color[1]`).
#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Lit(f32),
    Var(String, Option<u8>),
}

/// The variable (or one component of it) a proxy writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub var: String,
    pub comp: Option<u8>,
}

fn split_var(s: &str) -> (String, Option<u8>) {
    let s = s.trim();
    match s.find('[') {
        Some(i) => {
            let comp = s[i + 1..].trim_end_matches(']').trim().parse().ok().filter(|c: &u8| *c < 4);
            (s[..i].trim().to_string(), comp)
        }
        None => (s.to_string(), None),
    }
}

fn operand(s: &str) -> Option<Operand> {
    let s = s.trim();
    if s.starts_with('$') || s.starts_with('%') {
        let (v, c) = split_var(s);
        Some(Operand::Var(v, c))
    } else {
        kv::parse_f32(s).map(Operand::Lit)
    }
}

fn target(s: Option<&str>) -> Option<Target> {
    let s = s?.trim();
    if !s.starts_with('$') {
        return None;
    }
    let (var, comp) = split_var(s);
    Some(Target { var, comp })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Equals,
    Abs,
    Frac,
    Int,
}

/// One compiled proxy.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// `(max - min) * (sin(2 pi (t - offset) / period) * 0.5 + 0.5) + min`
    Sine { period: Operand, min: Operand, max: Operand, offset: Operand, out: Target },
    /// `rate * t + initial`
    LinearRamp { rate: Operand, initial: Operand, out: Target },
    /// A new random value in `[min, max]` every frame.
    UniformNoise { min: Operand, max: Operand, out: Target },
    Math { op: MathOp, a: Operand, b: Operand, out: Target },
    Unary { op: UnaryOp, a: Operand, out: Target },
    Clamp { a: Operand, min: Operand, max: Operand, out: Target },
    /// `a <= b ? lte : gt`
    LessOrEqual { a: Operand, b: Operand, lte: Operand, gt: Operand, out: Target },
    SelectFirstIfNonZero { a: Operand, b: Operand, out: Target },
    WrapMinMax { a: Operand, min: Operand, max: Operand, out: Target },
    /// `clamp(scale * exp(a + offset), min, max)`
    Exponential { a: Operand, min: Operand, max: Operand, scale: Operand, offset: Operand, out: Target },
    CurrentTime { out: Target },
    /// A fixed random number per entity, times `scale`.
    EntityRandom { scale: Operand, out: Target },
    /// Flipbook: frame = `rate * t` over the texture's frames.
    AnimatedTexture { texture: String, frame: Target, rate: f32, wrap: bool },
    /// Scrolls a texture transform (or a 2-vector) at `rate` along `angle` degrees.
    TextureScroll { out: Target, rate: Operand, angle: Operand, scale: Operand },
    /// Frame = the entity's texture frame index ([`TextureFrames`]).
    ToggleTexture { texture: String, frame: Target, wrap: bool },
    /// How loud the entity is talking ([`SpeakingAmount`]), 1 when silent-driven.
    LightedMouth { out: Target },
}

impl Op {
    fn compile(p: &MaterialProxy) -> Option<Op> {
        let g = |k: &str| p.get(k);
        let lit = |k: &str, d: f32| g(k).and_then(operand).unwrap_or(Operand::Lit(d));
        let src = |k: &str| g(k).and_then(operand);
        let out = || target(g("resultvar"));
        Some(match p.name.as_str() {
            "sine" => Op::Sine {
                period: lit("sineperiod", 1.0),
                min: lit("sinemin", 0.0),
                max: lit("sinemax", 1.0),
                offset: lit("timeoffset", 0.0),
                out: out()?,
            },
            "linearramp" => Op::LinearRamp { rate: lit("rate", 1.0), initial: lit("initialvalue", 0.0), out: out()? },
            "uniformnoise" => Op::UniformNoise { min: lit("minval", 0.0), max: lit("maxval", 1.0), out: out()? },
            "add" | "subtract" | "multiply" | "divide" => Op::Math {
                op: match p.name.as_str() {
                    "add" => MathOp::Add,
                    "subtract" => MathOp::Subtract,
                    "multiply" => MathOp::Multiply,
                    _ => MathOp::Divide,
                },
                a: src("srcvar1")?,
                b: src("srcvar2")?,
                out: out()?,
            },
            "equals" | "abs" | "frac" | "int" => Op::Unary {
                op: match p.name.as_str() {
                    "equals" => UnaryOp::Equals,
                    "abs" => UnaryOp::Abs,
                    "frac" => UnaryOp::Frac,
                    _ => UnaryOp::Int,
                },
                a: src("srcvar1")?,
                out: out()?,
            },
            "clamp" => Op::Clamp { a: src("srcvar1")?, min: lit("min", 0.0), max: lit("max", 1.0), out: out()? },
            "lessorequal" => Op::LessOrEqual {
                a: src("srcvar1")?,
                b: src("srcvar2")?,
                lte: src("lessequalvar")?,
                gt: src("greatervar")?,
                out: out()?,
            },
            "selectfirstifnonzero" => Op::SelectFirstIfNonZero { a: src("srcvar1")?, b: src("srcvar2")?, out: out()? },
            "wrapminmax" => Op::WrapMinMax { a: src("srcvar1")?, min: lit("minval", 0.0), max: lit("maxval", 1.0), out: out()? },
            "exponential" => Op::Exponential {
                a: src("srcvar1")?,
                min: lit("minval", f32::MIN),
                max: lit("maxval", f32::MAX),
                scale: lit("scale", 1.0),
                offset: lit("offset", 0.0),
                out: out()?,
            },
            "currenttime" => Op::CurrentTime { out: out()? },
            "entityrandom" => Op::EntityRandom { scale: lit("scale", 1.0), out: out()? },
            "animatedtexture" => Op::AnimatedTexture {
                texture: g("animatedtexturevar").unwrap_or("$basetexture").to_string(),
                frame: target(g("animatedtextureframenumvar")).unwrap_or(Target { var: "$frame".into(), comp: None }),
                rate: g("animatedtextureframerate").and_then(kv::parse_f32).unwrap_or(15.0),
                wrap: !g("animationnowrap").and_then(kv::parse_f32).is_some_and(|v| v != 0.0),
            },
            "texturescroll" => Op::TextureScroll {
                out: target(g("texturescrollvar")).unwrap_or(Target { var: "$basetexturetransform".into(), comp: None }),
                rate: lit("texturescrollrate", 1.0),
                angle: lit("texturescrollangle", 0.0),
                scale: lit("texturescale", 1.0),
            },
            "toggletexture" => Op::ToggleTexture {
                texture: g("toggletexturevar").unwrap_or("$basetexture").to_string(),
                frame: target(g("toggletextureframenumvar").or(g("toggleframe"))).unwrap_or(Target { var: "$frame".into(), comp: None }),
                wrap: g("toggleshouldwrap").and_then(kv::parse_f32).is_none_or(|v| v != 0.0),
            },
            "lightedmouth" => Op::LightedMouth { out: out()? },
            _ => return None,
        })
    }
}

/// A material's proxies, compiled once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProxyProgram {
    pub ops: Vec<Op>,
    /// Proxies we don't evaluate (their variables keep the VMT's values).
    pub unsupported: Vec<String>,
}

/// Per-entity and per-frame inputs to [`ProxyProgram::eval`].
pub struct ProxyContext<'a> {
    /// Seconds (game time).
    pub time: f32,
    /// The entity's texture frame index (`env_texturetoggle`).
    pub texture_frame: u32,
    /// The entity's talking amount, 0..1.
    pub speaking: f32,
    /// A fixed 0..1 random number for the entity.
    pub entity_random: f32,
    /// Random state for `UniformNoise`.
    pub noise: u32,
    /// Number of animation frames of the texture in a texture variable (`$basetexture`).
    pub frame_count: &'a dyn Fn(&str) -> u32,
}

impl ProxyContext<'_> {
    fn random(&mut self) -> f32 {
        // xorshift32
        let mut x = self.noise.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Variable values while a program runs.
struct Store<'a> {
    base: &'a Kv,
    set: Vec<(String, Var)>,
}

impl Store<'_> {
    fn get(&self, name: &str) -> Var {
        MaterialVars { base: self.base, set: &self.set }.get(name).unwrap_or(Var::scalar(0.0))
    }

    fn read(&self, o: &Operand) -> Var {
        match o {
            Operand::Lit(v) => Var::scalar(*v),
            Operand::Var(name, None) => self.get(name),
            Operand::Var(name, Some(c)) => Var::scalar(self.get(name).comp(*c as usize)),
        }
    }

    fn put(&mut self, t: &Target, v: Var) {
        let cur = self.get(&t.var);
        let new = match (t.comp, v) {
            (Some(c), _) => {
                let (mut vals, n) = match cur {
                    Var::Vec(vals, 1) => ([vals[0]; 4], 1),
                    Var::Vec(vals, n) => (vals, n),
                    Var::Matrix(_) => ([0.0; 4], 1),
                };
                vals[c as usize] = v.x();
                Var::Vec(vals, n.max(c + 1))
            }
            // A scalar stored into a vector variable fills it.
            (None, Var::Vec(vals, 1)) => match cur {
                Var::Vec(_, n) if n > 1 => Var::Vec([vals[0]; 4], n),
                _ => v,
            },
            (None, v) => v,
        };
        match self.set.iter_mut().find(|(k, _)| *k == t.var) {
            Some(slot) => slot.1 = new,
            None => self.set.push((t.var.clone(), new)),
        }
    }
}

fn lanes(a: Var, b: Var, f: impl Fn(f32, f32) -> f32) -> Var {
    let n = a.width().max(b.width());
    let mut out = [0.0; 4];
    for (i, o) in out.iter_mut().enumerate().take(n as usize) {
        *o = f(a.comp(i), b.comp(i));
    }
    Var::Vec(out, n)
}

fn map(a: Var, f: impl Fn(f32) -> f32) -> Var {
    lanes(a, a, |x, _| f(x))
}

/// A frame index over `count` frames: wrapped, or held on the last.
pub fn pick_frame(frame: u32, count: u32, wrap: bool) -> u32 {
    let count = count.max(1);
    if wrap {
        frame % count
    } else {
        frame.min(count - 1)
    }
}

impl ProxyProgram {
    pub fn compile(proxies: &[MaterialProxy]) -> Self {
        let mut p = ProxyProgram::default();
        for proxy in proxies {
            match Op::compile(proxy) {
                Some(op) => p.ops.push(op),
                None => p.unsupported.push(proxy.name.clone()),
            }
        }
        p
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Run the proxies in order over the material's variables; returns the
    /// variables they wrote (read them through [`MaterialVars`]).
    pub fn eval(&self, base: &Kv, ctx: &mut ProxyContext) -> Vec<(String, Var)> {
        let mut s = Store { base, set: Vec::new() };
        let t = ctx.time;
        for op in &self.ops {
            match op {
                Op::Sine { period, min, max, offset, out } => {
                    let period = s.read(period).x();
                    let (lo, hi) = (s.read(min).x(), s.read(max).x());
                    let phase = if period != 0.0 { (t - s.read(offset).x()) / period } else { 0.0 };
                    let v = (hi - lo) * ((std::f32::consts::TAU * phase).sin() * 0.5 + 0.5) + lo;
                    s.put(out, Var::scalar(v));
                }
                Op::LinearRamp { rate, initial, out } => {
                    let v = s.read(rate).x() * t + s.read(initial).x();
                    s.put(out, Var::scalar(v));
                }
                Op::UniformNoise { min, max, out } => {
                    let (lo, hi) = (s.read(min).x(), s.read(max).x());
                    let v = lo + (hi - lo) * ctx.random();
                    s.put(out, Var::scalar(v));
                }
                Op::Math { op, a, b, out } => {
                    let (a, b) = (s.read(a), s.read(b));
                    let v = match op {
                        MathOp::Add => lanes(a, b, |x, y| x + y),
                        MathOp::Subtract => lanes(a, b, |x, y| x - y),
                        MathOp::Multiply => lanes(a, b, |x, y| x * y),
                        MathOp::Divide => lanes(a, b, |x, y| if y != 0.0 { x / y } else { x }),
                    };
                    s.put(out, v);
                }
                Op::Unary { op, a, out } => {
                    let a = s.read(a);
                    let v = match op {
                        UnaryOp::Equals => a,
                        UnaryOp::Abs => map(a, f32::abs),
                        UnaryOp::Frac => map(a, |x| x - x.floor()),
                        UnaryOp::Int => map(a, f32::trunc),
                    };
                    s.put(out, v);
                }
                Op::Clamp { a, min, max, out } => {
                    let (lo, hi) = (s.read(min).x(), s.read(max).x());
                    let v = map(s.read(a), |x| x.max(lo).min(hi));
                    s.put(out, v);
                }
                Op::LessOrEqual { a, b, lte, gt, out } => {
                    let v = if s.read(a).x() <= s.read(b).x() { s.read(lte) } else { s.read(gt) };
                    s.put(out, v);
                }
                Op::SelectFirstIfNonZero { a, b, out } => {
                    let a = s.read(a);
                    let nonzero = (0..a.width() as usize).any(|i| a.comp(i) != 0.0);
                    let v = if nonzero { a } else { s.read(b) };
                    s.put(out, v);
                }
                Op::WrapMinMax { a, min, max, out } => {
                    let (lo, hi) = (s.read(min).x(), s.read(max).x());
                    let v = map(s.read(a), |x| if hi > lo { lo + (x - lo).rem_euclid(hi - lo) } else { lo });
                    s.put(out, v);
                }
                Op::Exponential { a, min, max, scale, offset, out } => {
                    let (lo, hi) = (s.read(min).x(), s.read(max).x());
                    let (k, o) = (s.read(scale).x(), s.read(offset).x());
                    let v = map(s.read(a), |x| (k * (x + o).exp()).max(lo).min(hi));
                    s.put(out, v);
                }
                Op::CurrentTime { out } => s.put(out, Var::scalar(t)),
                Op::EntityRandom { scale, out } => {
                    let v = ctx.entity_random * s.read(scale).x();
                    s.put(out, Var::scalar(v));
                }
                Op::AnimatedTexture { texture, frame, rate, wrap } => {
                    let count = (ctx.frame_count)(texture);
                    let f = (rate * t).max(0.0) as u32;
                    s.put(frame, Var::scalar(pick_frame(f, count, *wrap) as f32));
                }
                Op::TextureScroll { out, rate, angle, scale } => {
                    let (rate, angle, scale) = (s.read(rate).x(), s.read(angle).x(), s.read(scale).x());
                    let (sin, cos) = angle.to_radians().sin_cos();
                    let so = (t * cos * rate).rem_euclid(1.0);
                    let to = (t * sin * rate).rem_euclid(1.0);
                    let v = if out.var.ends_with("transform") && out.comp.is_none() {
                        Var::Matrix([scale, 0.0, so, 0.0, scale, to])
                    } else {
                        Var::Vec([so, to, 0.0, 0.0], 2)
                    };
                    s.put(out, v);
                }
                Op::ToggleTexture { texture, frame, wrap } => {
                    let count = (ctx.frame_count)(texture);
                    s.put(frame, Var::scalar(pick_frame(ctx.texture_frame, count, *wrap) as f32));
                }
                Op::LightedMouth { out } => s.put(out, Var::scalar(ctx.speaking.clamp(0.0, 1.0))),
            }
        }
        s.set
    }
}

// ---------------------------------------------------------------- Bevy side

/// Texture frame index per entity, as `env_texturetoggle` sets it
/// (`SetTextureIndex` / `IncrementTextureIndex`). Key: the entity a map
/// entity was spawned as (`MapEntity::entity`: the brush entity or the prop),
/// or an overlay's surface ([`SourceOverlay`]). Read by `ToggleTexture`.
#[derive(Resource, Default, Debug, Clone)]
pub struct TextureFrames(pub HashMap<Entity, u32>);

/// How much each entity is talking, 0..1 (its voice's loudness), keyed like
/// [`TextureFrames`]. Read by `LightedMouth`; an entity not in the map counts
/// as 1 (fully lit), so a game only needs to write while voice plays.
#[derive(Resource, Default, Debug, Clone)]
pub struct SpeakingAmount(pub HashMap<Entity, f32>);

/// On the surface entities of a named `info_overlay`: its overlay id, as the
/// map's `info_overlay_accessor` names it (`overlayid`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceOverlay {
    pub id: i32,
}

/// The value of `e`'s nearest ancestor (itself first) in `map`.
pub fn inherited<T: Copy>(e: Entity, map: &HashMap<Entity, T>, parents: &Query<&ChildOf>) -> Option<T> {
    if map.is_empty() {
        return None;
    }
    let mut cur = e;
    for _ in 0..16 {
        if let Some(v) = map.get(&cur) {
            return Some(*v);
        }
        cur = parents.get(cur).ok()?.parent();
    }
    None
}

fn entity_random(e: Entity) -> f32 {
    let mut x = (e.to_bits() as u32) ^ ((e.to_bits() >> 32) as u32).wrapping_mul(0x9e37_79b9);
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// A proxied material on the Bevy-material fallback path. The mesh owns its
/// own `SourceMaterial` (proxied materials are not shared between meshes).
#[derive(Component, Clone)]
pub struct FallbackProxies {
    pub program: Arc<ProxyProgram>,
    pub params: Kv,
    /// Every frame of the base texture (empty when it has one frame).
    pub frames: Vec<Handle<Image>>,
    /// Whether `$color` / `$alpha` drive the tint (not for the fixed glass pane).
    pub drive_color: bool,
}

impl FallbackProxies {
    /// For a material with proxies we evaluate; `None` for static ones.
    pub fn new(m: &crate::vmt::MaterialInfo, frames: Vec<Handle<Image>>) -> Option<Self> {
        let program = ProxyProgram::compile(&m.proxies);
        (!program.is_empty()).then(|| FallbackProxies {
            program: Arc::new(program),
            params: m.params.clone(),
            frames,
            drive_color: !(m.is_glassy && m.base_texture.is_none()),
        })
    }
}

pub struct MaterialProxyPlugin;

impl Plugin for MaterialProxyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureFrames>()
            .init_resource::<SpeakingAmount>()
            .add_systems(PostUpdate, (attach_valve_overrides, update_valve_proxies).chain())
            .add_systems(PostUpdate, update_fallback_proxies);
    }
}

/// Give new Valve surfaces with proxied materials their per-frame overrides.
fn attach_valve_overrides(mut commands: Commands, q: Query<(Entity, &crate::valve::ValveSurface), Added<crate::valve::ValveSurface>>) {
    for (e, s) in &q {
        if s.0.dynamic.is_some() {
            commands.entity(e).insert(crate::valve::SurfaceOverrides::default());
        }
    }
}

fn update_valve_proxies(
    time: Res<Time>,
    frames: Res<TextureFrames>,
    speaking: Res<SpeakingAmount>,
    parents: Query<&ChildOf>,
    mut q: Query<(Entity, &crate::valve::ValveSurface, &mut crate::valve::SurfaceOverrides)>,
) {
    let t = time.elapsed_secs();
    let mut noise = time.elapsed().as_nanos() as u32 | 1;
    for (e, s, mut o) in &mut q {
        let Some(d) = &s.0.dynamic else { continue };
        let count = |var: &str| d.texture_frames.iter().find(|(v, _)| v == var).map_or(1, |(_, n)| *n);
        let mut ctx = ProxyContext {
            time: t,
            texture_frame: inherited(e, &frames.0, &parents).unwrap_or(0),
            speaking: inherited(e, &speaking.0, &parents).unwrap_or(1.0),
            entity_random: entity_random(e),
            noise,
            frame_count: &count,
        };
        let set = d.program.eval(&d.params, &mut ctx);
        noise = ctx.noise;
        let data = crate::valve::dynamic_overrides(&s.0.textures, d, &MaterialVars { base: &d.params, set: &set });
        o.0 = Arc::new(data);
    }
}

fn update_fallback_proxies(
    time: Res<Time>,
    frames: Res<TextureFrames>,
    speaking: Res<SpeakingAmount>,
    parents: Query<&ChildOf>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    q: Query<(Entity, &FallbackProxies, &MeshMaterial3d<SourceMaterial>)>,
) {
    let t = time.elapsed_secs();
    let mut noise = time.elapsed().as_nanos() as u32 | 1;
    for (e, p, handle) in &q {
        let n = p.frames.len().max(1) as u32;
        let count = |var: &str| if var == "$basetexture" { n } else { 1 };
        let mut ctx = ProxyContext {
            time: t,
            texture_frame: inherited(e, &frames.0, &parents).unwrap_or(0),
            speaking: inherited(e, &speaking.0, &parents).unwrap_or(1.0),
            entity_random: entity_random(e),
            noise,
            frame_count: &count,
        };
        let set = p.program.eval(&p.params, &mut ctx);
        noise = ctx.noise;
        let vars = MaterialVars { base: &p.params, set: &set };
        let Some(cur) = materials.get(&handle.0) else { continue };
        let mut params = cur.params;
        if p.drive_color {
            let c = vars.vec3("$color").or_else(|| vars.vec3("$color2")).unwrap_or([1.0; 3]);
            params.color = Vec4::new(c[0], c[1], c[2], vars.f32("$alpha").unwrap_or(1.0));
        }
        let m = vars.transform("$basetexturetransform");
        params.uv_transform_u = Vec4::new(m[0], m[1], m[2], 0.0);
        params.uv_transform_v = Vec4::new(m[3], m[4], m[5], 0.0);
        let base = (!p.frames.is_empty()).then(|| {
            let f = vars.f32("$frame").unwrap_or(0.0).max(0.0) as u32 % n;
            p.frames[f as usize].clone()
        });
        let same_tex = base.as_ref().is_none_or(|b| cur.base_texture.as_ref() == Some(b));
        if same_tex && params_eq(&cur.params, &params) {
            continue;
        }
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.params = params;
            if let Some(b) = base {
                mat.base_texture = Some(b);
            }
        }
    }
}

fn params_eq(a: &crate::material::SourceMaterialParams, b: &crate::material::SourceMaterialParams) -> bool {
    a.color == b.color && a.uv_transform_u == b.uv_transform_u && a.uv_transform_v == b.uv_transform_v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmt;

    fn material(text: &str) -> crate::vmt::MaterialInfo {
        let kv = kv::parse(text);
        let (shader, body) = kv.root().unwrap();
        vmt::material_info("test/material", shader, body)
    }

    fn run(m: &crate::vmt::MaterialInfo, time: f32, frame: u32, speaking: f32, frames: u32) -> Vec<(String, Var)> {
        let count = move |_: &str| frames;
        let mut ctx = ProxyContext { time, texture_frame: frame, speaking, entity_random: 0.25, noise: 7, frame_count: &count };
        ProxyProgram::compile(&m.proxies).eval(&m.params, &mut ctx)
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn parses_values() {
        assert_eq!(Var::parse("0.5"), Some(Var::scalar(0.5)));
        assert_eq!(Var::parse("[1 2 3]"), Some(Var::Vec([1.0, 2.0, 3.0, 0.0], 3)));
        let c = Var::parse("{255 0 51}").unwrap();
        assert!(approx(c.comp(0), 1.0) && c.comp(1) == 0.0 && approx(c.comp(2), 0.2), "{c:?}");
        assert_eq!(Var::parse("metal/black_wall_metal_002b"), None);
        assert_eq!(Var::parse("center .5 .5 scale 1 1 rotate 0 translate 0 0"), Some(Var::Matrix(IDENTITY_TRANSFORM)));
    }

    #[test]
    fn texture_transform_matches_material_system() {
        // scale 2 about the centre keeps the centre fixed
        let m = parse_transform("center .5 .5 scale 2 2 rotate 0 translate 0 0");
        assert_eq!(transform_uv(&m, [0.5, 0.5]), [0.5, 0.5]);
        assert_eq!(transform_uv(&m, [1.0, 0.5]), [1.5, 0.5]);
        // rotate 90 about the origin, then translate
        let m = parse_transform("center 0 0 scale 1 1 rotate 90 translate .25 0");
        let r = transform_uv(&m, [1.0, 0.0]);
        assert!(approx(r[0], 0.25) && approx(r[1], 1.0), "{r:?}");
    }

    #[test]
    fn sine_oscillates_between_min_and_max() {
        let m = material(
            r#""UnlitGeneric" { "$basetexture" "x" "Proxies" { "Sine" { "sinemin" ".2" "sinemax" "1" "sineperiod" "4" "resultVar" "$alpha" } } }"#,
        );
        let at = |t| MaterialVars { base: &m.params, set: &run(&m, t, 0, 1.0, 1) }.f32("$alpha").unwrap();
        assert!(approx(at(0.0), 0.6));
        assert!(approx(at(1.0), 1.0));
        assert!(approx(at(2.0), 0.6));
        assert!(approx(at(3.0), 0.2));
    }

    #[test]
    fn linear_ramp_noise_and_math() {
        let m = material(
            r#""LightmappedGeneric"
            {
                "$basetexture" "x"
                "$selfillumtint" "[1 0.5 0.25]"
                "$scale" "2"
                "$ramp" "0"
                Proxies
                {
                    LinearRamp { rate 3 initialValue 1 resultVar $ramp }
                    Multiply { srcVar1 $selfillumtint srcVar2 $scale resultVar $tint2 }
                    Add { srcVar1 $ramp srcVar2 $scale resultVar $sum }
                    Subtract { srcVar1 $ramp srcVar2 $scale resultVar $diff }
                    Equals { srcVar1 $sum resultVar "$color[1]" }
                    Clamp { srcVar1 $ramp min 0 max 5 resultVar $clamped }
                    UniformNoise { minVal 2 maxVal 3 resultVar $noise }
                }
            }"#,
        );
        assert_eq!(m.proxies.len(), 7);
        let set = run(&m, 2.0, 0, 1.0, 1);
        let v = MaterialVars { base: &m.params, set: &set };
        assert!(approx(v.f32("$ramp").unwrap(), 7.0));
        assert_eq!(v.vec3("$tint2"), Some([2.0, 1.0, 0.5]));
        assert!(approx(v.f32("$sum").unwrap(), 9.0));
        assert!(approx(v.f32("$diff").unwrap(), 5.0));
        // one component of an unset vector: the others stay 0
        assert_eq!(v.get("$color"), Some(Var::Vec([0.0, 9.0, 0.0, 0.0], 2)));
        assert!(approx(v.f32("$clamped").unwrap(), 5.0));
        let n = v.f32("$noise").unwrap();
        assert!((2.0..=3.0).contains(&n), "{n}");
    }

    #[test]
    fn scalar_result_fills_a_vector_variable() {
        let m = material(
            r#""VertexLitGeneric" { "$selfillumtint" "[0 0 0]" "Proxies" { "LightedMouth" { "resultVar" "$selfillumtint" } } }"#,
        );
        let set = run(&m, 0.0, 0, 0.4, 1);
        assert_eq!(MaterialVars { base: &m.params, set: &set }.vec3("$selfillumtint"), Some([0.4; 3]));
        // not talking-driven: fully lit
        let set = run(&m, 0.0, 0, 1.0, 1);
        assert_eq!(MaterialVars { base: &m.params, set: &set }.vec3("$selfillumtint"), Some([1.0; 3]));
    }

    #[test]
    fn animated_texture_flips_frames() {
        let m = material(
            r#""UnlitGeneric" { "$basetexture" "screens/anim" "$frame" "0"
                "Proxies" { "AnimatedTexture" { "animatedtexturevar" "$basetexture" "animatedtextureframenumvar" "$frame" "animatedtextureframerate" 10 } } }"#,
        );
        let frame = |t, frames| MaterialVars { base: &m.params, set: &run(&m, t, 0, 1.0, frames) }.f32("$frame").unwrap();
        assert_eq!(frame(0.0, 4), 0.0);
        assert_eq!(frame(0.25, 4), 2.0);
        assert_eq!(frame(0.45, 4), 0.0); // wraps after 4
        assert_eq!(frame(0.45, 1), 0.0); // single-frame texture
        let held = material(
            r#""UnlitGeneric" { "Proxies" { "AnimatedTexture" { "animatedtexturevar" "$basetexture" "animatedtextureframenumvar" "$frame" "animatedtextureframerate" 10 "animationnowrap" 1 } } }"#,
        );
        assert_eq!(MaterialVars { base: &held.params, set: &run(&held, 5.0, 0, 1.0, 4) }.f32("$frame"), Some(3.0));
    }

    #[test]
    fn texture_scroll_builds_a_transform() {
        let m = material(
            r#""LightmappedGeneric" { "$basetexture" "x"
                "Proxies" { "TextureScroll" { "texturescrollvar" "$baseTextureTransform" "texturescrollrate" .25 "texturescrollangle" 90 } } }"#,
        );
        let set = run(&m, 3.0, 0, 1.0, 1);
        let t = MaterialVars { base: &m.params, set: &set }.transform("$basetexturetransform");
        // 0.75 along +v, nothing along u
        assert!(approx(t[2], 0.0) || approx(t[2], 1.0), "{t:?}");
        assert!(approx(t[5], 0.75), "{t:?}");
        assert_eq!((t[0], t[4]), (1.0, 1.0));
        // negative directions wrap into 0..1
        let back = material(
            r#""LightmappedGeneric" { "Proxies" { "TextureScroll" { "texturescrollvar" "$basetexturetransform" "texturescrollrate" .25 "texturescrollangle" 180 } } }"#,
        );
        let set = run(&back, 1.0, 0, 1.0, 1);
        let t = MaterialVars { base: &back.params, set: &set }.transform("$basetexturetransform");
        assert!(approx(t[2], 0.75), "{t:?}");
    }

    #[test]
    fn toggle_texture_follows_the_entity_frame() {
        // Portal 2's indicator lights (antlines / checkmarks).
        let m = material(
            r#""LightmappedGeneric"
            {
                "$basetexture" "signage/indicator_lights/indicator_lights_floor"
                "$decal" 1
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
        );
        assert_eq!(m.proxies[0].result_var(), Some("$frame"));
        let frame = |f| MaterialVars { base: &m.params, set: &run(&m, 0.0, f, 1.0, 2) }.f32("$frame").unwrap();
        assert_eq!(frame(0), 0.0);
        assert_eq!(frame(1), 1.0);
        assert_eq!(frame(5), 1.0); // no wrap: held on the last frame
        let wrapping = material(r#""UnlitGeneric" { "Proxies" { "ToggleTexture" { "toggletexturevar" "$basetexture" "toggleframe" "$frame" } } }"#);
        assert_eq!(MaterialVars { base: &wrapping.params, set: &run(&wrapping, 0.0, 3, 1.0, 2) }.f32("$frame"), Some(1.0));
    }

    #[test]
    fn unknown_proxies_are_skipped() {
        let m = material(
            r#""LightmappedGeneric" { "$alpha" ".5" "Proxies" { "PortalOpenAmount" { "resultVar" "$alpha" } "Equals" { "srcVar1" "$alpha" "resultVar" "$copy" } } }"#,
        );
        let p = ProxyProgram::compile(&m.proxies);
        assert_eq!(p.unsupported, vec!["portalopenamount".to_string()]);
        let set = run(&m, 0.0, 0, 1.0, 1);
        assert_eq!(MaterialVars { base: &m.params, set: &set }.f32("$copy"), Some(0.5));
    }

    #[test]
    fn other_math_proxies() {
        let m = material(
            r#""UnlitGeneric" { "$a" "-2.75" "$b" "4" "$z" "0"
              "Proxies" {
                "Abs" { "srcVar1" "$a" "resultVar" "$abs" }
                "Frac" { "srcVar1" "$a" "resultVar" "$frac" }
                "Int" { "srcVar1" "$a" "resultVar" "$int" }
                "Divide" { "srcVar1" "$b" "srcVar2" "$a" "resultVar" "$div" }
                "LessOrEqual" { "srcVar1" "$a" "srcVar2" "$b" "lessEqualVar" "$b" "greaterVar" "$a" "resultVar" "$le" }
                "SelectFirstIfNonZero" { "srcVar1" "$z" "srcVar2" "$b" "resultVar" "$sel" }
                "WrapMinMax" { "srcVar1" "$b" "minVal" "0" "maxVal" "3" "resultVar" "$wrap" }
                "EntityRandom" { "scale" "2" "resultVar" "$rnd" }
                "CurrentTime" { "resultVar" "$now" }
              } }"#,
        );
        let set = run(&m, 1.5, 0, 1.0, 1);
        let v = MaterialVars { base: &m.params, set: &set };
        let f = |k| v.f32(k).unwrap();
        assert!(approx(f("$abs"), 2.75));
        assert!(approx(f("$frac"), 0.25));
        assert!(approx(f("$int"), -2.0));
        assert!(approx(f("$div"), 4.0 / -2.75));
        assert!(approx(f("$le"), 4.0));
        assert!(approx(f("$sel"), 4.0));
        assert!(approx(f("$wrap"), 1.0));
        assert!(approx(f("$rnd"), 0.5));
        assert!(approx(f("$now"), 1.5));
    }
}
