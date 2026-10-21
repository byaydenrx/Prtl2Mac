//! CPU simulation of Source particle systems (`.pcf` definitions).
//!
//! Everything here runs in **Source space** (inches, Z up), the space the
//! `.pcf` numbers are authored in. Control points are converted by the caller
//! ([`crate::render::ParticleEffect`]); the renderer converts positions back.
//!
//! Operator order and attribute defaults follow `research/specs/particles-runtime.md`
//! and `particles-params.md`. Velocities are Verlet-style: `prev` is the
//! position one step earlier, so `pos - prev` is the last displacement.
//! Operators the portal effects do not use are logged as unsupported (see
//! [`SystemInstance::unsupported`]) rather than silently guessed.
use std::collections::BTreeSet;
use std::f32::consts::PI;
use std::sync::Arc;

use bevy::math::{EulerRot, Mat3, Quat, Vec3};

use crate::pcf::{OpDef, ParticleLibrary, SystemDef};

pub const MAX_CONTROL_POINTS: usize = 64;

/// Drag is quoted per 1/30 s step (runtime spec §5, Movement Basic).
const DRAG_REF_HZ: f32 = 30.0;

/// A control point: position and orientation (columns = local X, Y, Z axes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlPoint {
    pub pos: Vec3,
    pub basis: Mat3,
    /// Set by the game / parent, rather than defaulted to CP0.
    pub set: bool,
}

impl Default for ControlPoint {
    fn default() -> Self {
        Self { pos: Vec3::ZERO, basis: Mat3::IDENTITY, set: false }
    }
}

impl ControlPoint {
    pub fn new(pos: Vec3, basis: Mat3) -> Self {
        Self { pos, basis, set: true }
    }
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.pos + self.basis * local
    }
    pub fn to_local(&self, world: Vec3) -> Vec3 {
        self.basis.transpose() * (world - self.pos)
    }
}

/// Particle field ids (PARTICLE_ATTRIBUTE_* in the public particles.h).
pub mod field {
    pub const XYZ: i32 = 0;
    pub const LIFE_DURATION: i32 = 1;
    pub const PREV_XYZ: i32 = 2;
    pub const RADIUS: i32 = 3;
    pub const ROTATION: i32 = 4;
    pub const ROTATION_SPEED: i32 = 5;
    pub const TINT_RGB: i32 = 6;
    pub const ALPHA: i32 = 7;
    pub const CREATION_TIME: i32 = 8;
    pub const SEQUENCE_NUMBER: i32 = 9;
    pub const TRAIL_LENGTH: i32 = 10;
    pub const YAW: i32 = 12;
    pub const SEQUENCE_NUMBER1: i32 = 13;
    pub const ALPHA2: i32 = 16;
    pub const NORMAL: i32 = 21;
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec3,
    pub prev: Vec3,
    pub lifetime: f32,
    pub radius: f32,
    /// Radians.
    pub rotation: f32,
    /// Radians per second.
    pub rotation_speed: f32,
    /// 0-1 per channel, sRGB-encoded as authored.
    pub tint: Vec3,
    /// 0-1; the rendered alpha is `alpha * alpha2`.
    pub alpha: f32,
    pub alpha2: f32,
    pub created: f32,
    pub sequence: f32,
    pub sequence1: f32,
    pub trail_length: f32,
    pub yaw: f32,
    pub normal: Vec3,
    pub id: u32,
    /// Values when the particle was born, after initializers (for scale and fade operators).
    pub init_radius: f32,
    pub init_alpha: f32,
    pub init_tint: Vec3,
    pub dead: bool,
}

impl Particle {
    pub fn scalar(&self, f: i32) -> f32 {
        match f {
            field::LIFE_DURATION => self.lifetime,
            field::RADIUS => self.radius,
            field::ROTATION => self.rotation,
            field::ROTATION_SPEED => self.rotation_speed,
            field::ALPHA => self.alpha,
            field::CREATION_TIME => self.created,
            field::SEQUENCE_NUMBER => self.sequence,
            field::TRAIL_LENGTH => self.trail_length,
            field::YAW => self.yaw,
            field::SEQUENCE_NUMBER1 => self.sequence1,
            field::ALPHA2 => self.alpha2,
            _ => 0.0,
        }
    }
    pub fn set_scalar(&mut self, f: i32, v: f32) {
        match f {
            field::LIFE_DURATION => self.lifetime = v,
            field::RADIUS => self.radius = v,
            field::ROTATION => self.rotation = v,
            field::ROTATION_SPEED => self.rotation_speed = v,
            field::ALPHA => self.alpha = v,
            field::CREATION_TIME => self.created = v,
            field::SEQUENCE_NUMBER => self.sequence = v,
            field::TRAIL_LENGTH => self.trail_length = v,
            field::YAW => self.yaw = v,
            field::SEQUENCE_NUMBER1 => self.sequence1 = v,
            field::ALPHA2 => self.alpha2 = v,
            _ => {}
        }
    }
    pub fn vector(&self, f: i32) -> Vec3 {
        match f {
            field::XYZ => self.pos,
            field::PREV_XYZ => self.prev,
            field::TINT_RGB => self.tint,
            field::NORMAL => self.normal,
            _ => Vec3::splat(self.scalar(f)),
        }
    }
    pub fn set_vector(&mut self, f: i32, v: Vec3) {
        match f {
            field::XYZ => self.pos = v,
            field::PREV_XYZ => self.prev = v,
            field::TINT_RGB => self.tint = v,
            field::NORMAL => self.normal = v,
            _ => self.set_scalar(f, v.x),
        }
    }
    fn is_vector_field(f: i32) -> bool {
        matches!(f, field::XYZ | field::PREV_XYZ | field::TINT_RGB | field::NORMAL)
    }
    fn age(&self, now: f32) -> f32 {
        now - self.created
    }
    /// Life fraction, 0 at birth and 1 at death.
    fn life_fraction(&self, now: f32) -> f32 {
        if self.lifetime > 0.0 {
            self.age(now) / self.lifetime
        } else {
            0.0
        }
    }
}

/// Deterministic per-particle random numbers in [0, 1).
fn hash(a: u32, b: u32, c: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ c.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

/// Random draw `k` for particle `id`, salted per operator.
fn rnd(seed: u32, id: u32, salt: u32, k: u32) -> f32 {
    hash(id ^ seed, salt, k)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn remap_clamped(v: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    if (b - a).abs() < 1e-12 {
        return if v >= b { d } else { c };
    }
    c + (d - c) * ((v - a) / (b - a)).clamp(0.0, 1.0)
}
/// Source's SmoothStep: `t^2 (3 - 2t)`.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
/// Source's Bias(): `t^(log(b)/log(0.5))`, so `b = 0.5` is linear.
fn bias(t: f32, b: f32) -> f32 {
    let b = b.clamp(1e-4, 1.0 - 1e-4);
    t.max(0.0).powf(b.ln() / 0.5f32.ln())
}
fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::from_array(a)
}
/// Uniform point in the unit sphere from three randoms.
fn in_unit_sphere(r1: f32, r2: f32, r3: f32) -> Vec3 {
    let z = r1 * 2.0 - 1.0;
    let t = r2 * 2.0 * PI;
    let s = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(s * t.cos(), s * t.sin(), z) * r3.cbrt()
}

/// 3-D gradient noise in [-1, 1].
fn noise3(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let grad = |c: Vec3| -> f32 {
        let h = hash(c.x as i32 as u32, c.y as i32 as u32, c.z as i32 as u32 ^ 0x5bd1);
        let theta = h * 2.0 * PI;
        let z = hash(c.x as i32 as u32 ^ 0x1234, c.y as i32 as u32, c.z as i32 as u32) * 2.0 - 1.0;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let g = Vec3::new(r * theta.cos(), r * theta.sin(), z);
        g.dot(p - c)
    };
    let c000 = grad(i);
    let c100 = grad(i + Vec3::X);
    let c010 = grad(i + Vec3::Y);
    let c110 = grad(i + Vec3::new(1.0, 1.0, 0.0));
    let c001 = grad(i + Vec3::Z);
    let c101 = grad(i + Vec3::new(1.0, 0.0, 1.0));
    let c011 = grad(i + Vec3::new(0.0, 1.0, 1.0));
    let c111 = grad(i + Vec3::ONE);
    let x00 = lerp(c000, c100, u.x);
    let x10 = lerp(c010, c110, u.x);
    let x01 = lerp(c001, c101, u.x);
    let x11 = lerp(c011, c111, u.x);
    let y0 = lerp(x00, x10, u.y);
    let y1 = lerp(x01, x11, u.y);
    (lerp(y0, y1, u.z) * 1.6).clamp(-1.0, 1.0)
}

/// Emitter state. `remaining` is the instantaneous emitter's unspawned count.
#[derive(Clone, Debug, Default)]
struct EmitterState {
    accumulated: f32,
    remaining: f32,
    started: bool,
    done: bool,
}

/// A child system: started after `delay` seconds, shares the parent's control points.
#[derive(Clone, Debug)]
pub struct ChildInstance {
    pub delay: f32,
    pub end_cap_only: bool,
    pub system: Option<SystemInstance>,
    name: String,
}

/// One running particle system (and its children).
#[derive(Clone, Debug)]
pub struct SystemInstance {
    pub def: Arc<SystemDef>,
    /// System time, seconds since the system started.
    pub time: f32,
    pub cps: [ControlPoint; MAX_CONTROL_POINTS],
    prev_cps: [ControlPoint; MAX_CONTROL_POINTS],
    pub particles: Vec<Particle>,
    next_id: u32,
    emitters: Vec<EmitterState>,
    /// Emission stopped; end-cap operators now run.
    pub end_cap: bool,
    pub children: Vec<ChildInstance>,
    seed: u32,
    first_update: bool,
    /// dt of the last step (Verlet time correction) and of the current step.
    pub last_dt: f32,
    cur_dt: f32,
    /// Particles emitted this step by all emitters (ring even distribution).
    step_spawned: u32,
    /// Persistent counters of the ring initializers, by initializer index.
    ring_counters: Vec<u32>,
    /// Particles emitted during the last [`Self::update`] (for stats).
    pub emitted_last: u32,
    /// Operators in the file that this simulator does not implement.
    pub unsupported: BTreeSet<String>,
    /// Emission stopped without the end cap (a map's `Stop`): the particles
    /// alive finish their lives, end-cap operators and children don't run.
    pub emission_stopped: bool,
    /// System time the emitters count from (moved by `Restart Effect after Duration`).
    emit_epoch: f32,
    /// When `Restart Effect after Duration` next restarts the emitters.
    restart_at: Option<f32>,
    /// System time the end cap started (for `Lifespan Endcap Timed Decay`).
    end_cap_at: Option<f32>,
    /// The player's eye (Source space), for `Set Control Point To Player`.
    pub player: Option<ControlPoint>,
    /// Height (Source z) of the ground or water below control point 0, for
    /// `Position Modify Place On Ground` (traced by the caller each frame).
    pub ground: Option<f32>,
    /// How much of the renderer's visibility proxy the camera sees (0..1,
    /// traced by the caller each frame; 1 when untraced).
    pub visibility: f32,
    /// Particles `Cull Random` has already spared (decided once each).
    spared: BTreeSet<u32>,
}

const KNOWN_EMITTERS: &[&str] = &["emit_instantaneously", "emit_continuously", "emit noise"];
const KNOWN_INITIALIZERS: &[&str] = &[
    "Position Within Sphere Random",
    "Position Along Ring",
    "Position Modify Warp Random",
    "Position Modify Offset Random",
    "Position Modify Place On Ground",
    "Position Along Path Sequential",
    "Lifetime Random",
    "Alpha Random",
    "Color Random",
    "Radius Random",
    "Rotation Random",
    "Rotation Speed Random",
    "Sequence Random",
    "Trail Length Random",
    "Velocity Random",
    "Normal Modify Offset Random",
    "Velocity Repulse from World",
    "Remap Noise to Scalar",
    "Remap Particle Count to Scalar",
    "Remap Control Point to Vector",
    "Remap Initial Distance to Control Point to Scalar",
    "remap initial scalar",
    "Position Within Box Random",
    "Position Along Path Random",
    "Velocity Noise",
    "Initial Velocity Noise",
    "Velocity Inherit from Control Point",
    "Rotation Yaw Flip Random",
    "Rotation Yaw Random",
    "Sequence Two Random",
    "Remap Control Point to Scalar",
];
const KNOWN_OPERATORS: &[&str] = &[
    "Remap CP Speed to CP",
    "Set Control Point Rotation",
    "Set Control Point Positions",
    "Movement Basic",
    "Movement Lock to Control Point",
    "Movement Dampen Relative to Control Point",
    "Movement Max Velocity",
    "Lifespan Decay",
    "Lifespan Minimum Radius Decay",
    "Lifespan Minimum Alpha Decay",
    "Alpha Fade In Simple",
    "Alpha Fade Out Simple",
    "Alpha Fade In Random",
    "Alpha Fade Out Random",
    "Alpha Fade and Decay",
    "Color Fade",
    "Radius Scale",
    "Clamp Scalar",
    "Ramp Scalar Linear Simple",
    "Oscillate Scalar",
    "Oscillate Vector",
    "Remap Direction to CP to Vector",
    "Remap Control Point to Vector",
    "Rotation Basic",
    "Cull when crossing sphere",
    "Pull towards control point",
    "random force",
    "twist around axis",
    "Constrain distance to control point",
    "Set Control Point To Player",
    "Restart Effect after Duration",
    "Rotation Spin Roll",
    "Rotation Spin Yaw",
    "Noise Scalar",
    "Noise Vector",
    "Remap Distance to Control Point to Scalar",
    "Remap Speed to Scalar",
    "Remap Scalar",
    "Lifespan Endcap Timed Decay",
    "Ramp Scalar Linear Random",
    "Ramp Scalar Spline Random",
    "Oscillate Scalar Simple",
    "Movement Rotate Particle Around Axis",
    "Cull Random",
    "Rotation Orient to 2D Direction",
    "Set child control points from particle positions",
    "Prevent passing through a plane",
];

/// Names of operators in `def` that this simulator does not implement.
fn unsupported_ops(def: &SystemDef) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for e in &def.emitters {
        if !KNOWN_EMITTERS.contains(&e.function.as_str()) {
            out.insert(format!("emitter {}", e.function));
        }
    }
    for i in &def.initializers {
        if !KNOWN_INITIALIZERS.contains(&i.function.as_str()) {
            out.insert(format!("initializer {}", i.function));
        }
    }
    for o in &def.operators {
        if !KNOWN_OPERATORS.contains(&o.function.as_str()) {
            out.insert(format!("operator {}", o.function));
        }
    }
    for f in &def.forces {
        if !KNOWN_OPERATORS.contains(&f.function.as_str()) {
            out.insert(format!("force {}", f.function));
        }
    }
    for c in &def.constraints {
        if !KNOWN_OPERATORS.contains(&c.function.as_str()) {
            out.insert(format!("constraint {}", c.function));
        }
    }
    out
}

impl SystemInstance {
    /// Moves the whole running system (particles, their previous positions,
    /// normals, and the control points) by a rigid transform given in this
    /// simulator's space: `pt` maps points, `dir` maps directions. Used to
    /// carry effects that follow the player through a portal, so nothing is
    /// left behind on the far side for a frame. Children are carried too.
    pub fn carry(&mut self, pt: &dyn Fn(Vec3) -> Vec3, dir: &dyn Fn(Vec3) -> Vec3) {
        for p in &mut self.particles {
            p.pos = pt(p.pos);
            p.prev = pt(p.prev);
            p.normal = dir(p.normal);
        }
        let map_cp = |c: &mut ControlPoint| {
            c.pos = pt(c.pos);
            c.basis = Mat3::from_cols(dir(c.basis.x_axis), dir(c.basis.y_axis), dir(c.basis.z_axis));
        };
        self.cps.iter_mut().for_each(map_cp);
        self.prev_cps.iter_mut().for_each(map_cp);
        for child in &mut self.children {
            if let Some(sys) = child.system.as_mut() {
                sys.carry(pt, dir);
            }
        }
    }

    pub fn new(def: Arc<SystemDef>, seed: u32) -> Self {
        let children = def
            .children
            .iter()
            .map(|c| ChildInstance { delay: c.delay, end_cap_only: c.end_cap, system: None, name: c.name.clone() })
            .collect();
        let emitters = vec![EmitterState::default(); def.emitters.len()];
        let ring_counters = vec![0; def.initializers.len()];
        let unsupported = unsupported_ops(&def);
        Self {
            def,
            time: 0.0,
            cps: [ControlPoint::default(); MAX_CONTROL_POINTS],
            prev_cps: [ControlPoint::default(); MAX_CONTROL_POINTS],
            particles: Vec::new(),
            next_id: 0,
            emitters,
            end_cap: false,
            children,
            seed,
            first_update: true,
            last_dt: 1.0 / 60.0,
            cur_dt: 0.0,
            step_spawned: 0,
            ring_counters,
            emitted_last: 0,
            unsupported,
            emission_stopped: false,
            emit_epoch: 0.0,
            restart_at: None,
            end_cap_at: None,
            player: None,
            ground: None,
            visibility: 1.0,
            spared: BTreeSet::new(),
        }
    }

    /// Stop emitting but let the live particles finish, without the end
    /// cap (Source's plain `Stop` on an `info_particle_system`).
    pub fn stop_emission(&mut self) {
        self.emission_stopped = true;
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.stop_emission();
            }
        }
    }

    /// The player's eye for `Set Control Point To Player`, here and in children.
    pub fn set_player(&mut self, eye: Option<ControlPoint>) {
        self.player = eye;
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.set_player(eye);
            }
        }
    }

    /// Set a control point here and in every child.
    pub fn set_cp(&mut self, i: usize, cp: ControlPoint) {
        if i < MAX_CONTROL_POINTS {
            self.cps[i] = cp;
        }
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.set_cp(i, cp);
            }
        }
    }

    /// Stop emitting and run end-cap behaviour (the system dies out).
    pub fn stop(&mut self) {
        if !self.end_cap {
            self.end_cap_at = Some(self.time);
        }
        self.end_cap = true;
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.stop();
            }
        }
    }

    /// Nothing left to simulate or draw.
    pub fn finished(&self) -> bool {
        let stopped = self.end_cap || self.emission_stopped;
        let emitting = !stopped && self.emitters.iter().any(|e| !e.done);
        !emitting
            && self.particles.is_empty()
            && self.children.iter().all(|c| c.system.as_ref().is_none_or(SystemInstance::finished))
            && (stopped || self.children.iter().all(|c| c.system.is_some() || c.end_cap_only))
            && (stopped || !self.def.emitters.is_empty())
    }

    fn cp(&self, i: i32) -> ControlPoint {
        self.cps[i.clamp(0, 63) as usize]
    }

    /// Advance the system by `dt` seconds (clamped to the file's maximum step).
    pub fn update(&mut self, dt: f32, lib: &ParticleLibrary) {
        let def = self.def.clone();
        let dt = dt.clamp(0.0, def.maximum_time_step.max(1e-3));
        if dt <= 0.0 {
            return;
        }
        // Unset control points follow CP0 (the engine default).
        let cp0 = self.cps[0];
        for cp in self.cps.iter_mut().skip(1) {
            if !cp.set {
                *cp = ControlPoint { set: false, ..cp0 };
            }
        }
        if self.first_update {
            self.prev_cps = self.cps;
        }
        self.cur_dt = dt;
        self.time += dt;
        let now = self.time;
        self.emitted_last = 0;

        // 1. Control-point operators, in file order.
        for op in &def.operators {
            if !self.op_active(op) {
                continue;
            }
            match op.function.as_str() {
                "Set Control Point Rotation" => self.op_cp_rotation(op, dt),
                "Set Control Point Positions" => self.op_cp_positions(op),
                "Set Control Point To Player" => self.op_cp_to_player(op),
                "Restart Effect after Duration" => self.op_restart(op, now),
                "Remap CP Speed to CP" => self.op_cp_speed(op, dt),
                _ => {}
            }
        }

        // 2. Emission.
        if !self.end_cap && !self.emission_stopped {
            let mut counts = Vec::with_capacity(def.emitters.len());
            for (ei, em) in def.emitters.iter().enumerate() {
                counts.push(self.emit_count(ei, em, dt));
            }
            self.step_spawned = counts.iter().sum::<usize>() as u32;
            for n in counts {
                for _ in 0..n {
                    self.spawn_particle(now - dt * 0.5);
                }
            }
        }

        // 3. Particle operators, forces, constraints.
        self.run_operators(&def, dt, now);
        self.run_forces(&def, dt);
        self.run_constraints(&def);
        self.particles.retain(|p| !p.dead);

        // 4. Children start after their delay and see the parent's control points.
        for ci in 0..self.children.len() {
            let start = self.children[ci].delay;
            let only_end = self.children[ci].end_cap_only;
            let held = self.emission_stopped && !self.end_cap;
            if self.children[ci].system.is_none() && now >= start && (!only_end || self.end_cap) && !held {
                if let Some(child_def) = lib.get(&self.children[ci].name) {
                    let mut s = SystemInstance::new(child_def, self.seed.wrapping_add(ci as u32 * 7777 + 1));
                    for (i, cp) in self.cps.iter().enumerate() {
                        if cp.set || i == 0 {
                            s.cps[i] = *cp;
                        }
                    }
                    s.cps[0].set = true;
                    if self.end_cap && !only_end {
                        s.end_cap = true;
                    }
                    self.children[ci].system = Some(s);
                }
            }
            let cps = self.cps;
            if let Some(s) = &mut self.children[ci].system {
                for (i, cp) in cps.iter().enumerate() {
                    if cp.set {
                        s.cps[i] = *cp;
                    }
                }
                s.update(dt, lib);
            }
        }
        self.last_dt = dt;
        self.prev_cps = self.cps;
        self.first_update = false;
    }

    fn op_active(&self, op: &OpDef) -> bool {
        match op.end_cap_state() {
            0 => !self.end_cap,
            1 => self.end_cap,
            _ => true,
        }
    }

    /// `operator start/end fadein/fadeout` strength in 0..1 at system time `now`.
    fn op_strength(op: &OpDef, now: f32) -> f32 {
        let (fin_s, fin_e) = (op.f("operator start fadein", 0.0), op.f("operator end fadein", 0.0));
        let (fout_s, fout_e) = (op.f("operator start fadeout", 0.0), op.f("operator end fadeout", 0.0));
        let mut strength = 1.0;
        if fin_e > fin_s {
            strength *= remap_clamped(now, fin_s, fin_e, 0.0, 1.0);
        }
        if fout_e > fout_s {
            strength *= remap_clamped(now, fout_s, fout_e, 1.0, 0.0);
        }
        strength
    }

    fn op_cp_rotation(&mut self, op: &OpDef, dt: f32) {
        let axis = v3(op.v3("Rotation Axis", [0.0, 0.0, 1.0])).normalize_or_zero();
        let rate = op.f("Rotation Rate", 180.0).to_radians();
        let i = op.i("Control Point", 0).clamp(0, 63) as usize;
        let local = op.i("Local Space Control Point", -1);
        let axis = if local >= 0 { self.cps[local.min(63) as usize].basis * axis } else { axis };
        if axis != Vec3::ZERO && rate != 0.0 {
            let q = Quat::from_axis_angle(axis.normalize(), rate * dt);
            self.cps[i].basis = Mat3::from_quat(q) * self.cps[i].basis;
            self.cps[i].set = true;
        }
    }

    /// `Remap CP Speed to CP`: how fast a control point moves (u/s), remapped
    /// into one component of another control point (GLaDOS's drag effects
    /// only emit while she moves).
    fn op_cp_speed(&mut self, op: &OpDef, dt: f32) {
        let out = op.i("output control point", -1);
        if !(0..64).contains(&out) {
            return;
        }
        let i = op.i("input control point", 0).clamp(0, 63) as usize;
        let speed = (self.cps[i].pos - self.prev_cps[i].pos).length() / dt.max(1e-4);
        let v = remap_clamped(speed, op.f("input minimum", 0.0), op.f("input maximum", 1.0), op.f("output minimum", 0.0), op.f("output maximum", 1.0));
        let field = op.i("Output field 0-2 X/Y/Z", 0).clamp(0, 2) as usize;
        let cp = &mut self.cps[out as usize];
        cp.pos[field] = v;
        cp.set = true;
    }

    fn op_cp_positions(&mut self, op: &OpDef) {
        let from = op.i("Control Point to offset positions from", 0).clamp(0, 63) as usize;
        let world = op.b("Set positions in world space", false);
        let base = self.cps[from];
        let entries = [
            ("First Control Point Number", "First Control Point Location", 1, [128.0, 0.0, 0.0]),
            ("Second Control Point Number", "Second Control Point Location", 2, [0.0, 128.0, 0.0]),
            ("Third Control Point Number", "Third Control Point Location", 3, [-128.0, 0.0, 0.0]),
            ("Fourth Control Point Number", "Fourth Control Point Location", 4, [0.0, -128.0, 0.0]),
        ];
        for (nk, lk, nd, ld) in entries {
            let n = op.i(nk, nd).clamp(0, 63) as usize;
            let l = v3(op.v3(lk, ld));
            let pos = if world { l } else { base.to_world(l) };
            self.cps[n] = ControlPoint { pos, basis: base.basis, set: true };
        }
    }

    /// Particles this emitter creates this step (runtime spec §3).
    fn emit_count(&mut self, ei: usize, em: &OpDef, dt: f32) -> usize {
        let now = self.time - self.emit_epoch;
        // One random per emitter for "emission_start_time max" (fixed for the system's life).
        let r = rnd(self.seed, ei as u32, 0xE1, 7);
        let start_max = em.f("emission_start_time max", -1.0);
        let start = if start_max >= 0.0 {
            lerp(em.f("emission_start_time", 0.0), start_max, r)
        } else {
            em.f("emission_start_time", 0.0)
        };
        let per_step_cap = match em.i("maximum emission per frame", -1) {
            m if m > 0 => m as usize,
            _ => self.def.max_particles,
        };
        let st = &mut self.emitters[ei];
        if st.done || now < start {
            return 0;
        }
        match em.function.as_str() {
            "emit_instantaneously" => {
                if !st.started {
                    st.started = true;
                    let num = em.i("num_to_emit", 100).max(0) as f32;
                    let min = em.i("num_to_emit_minimum", -1);
                    st.remaining = if min >= 0 { lerp(min as f32, num, r).round() } else { num };
                }
                let n = st.remaining.min(per_step_cap as f32).max(0.0).floor();
                st.remaining -= n;
                if st.remaining < 1.0 {
                    st.done = true;
                }
                n as usize
            }
            "emit_continuously" => {
                let duration = em.f("emission_duration", 0.0);
                if duration > 0.0 && now > start + duration {
                    st.done = true;
                    return 0;
                }
                // `emission count scale control point`: the rate times one
                // component of that control point (only once something set it).
                let scale = match em.i("emission count scale control point", -1) {
                    i if (0..64).contains(&i) && self.cps[i as usize].set => {
                        let c = em.i("emission count scale control point field", 0).clamp(0, 2) as usize;
                        self.cps[i as usize].pos[c].max(0.0)
                    }
                    _ => 1.0,
                };
                let st = &mut self.emitters[ei];
                st.accumulated += em.f("emission_rate", 100.0) * scale * dt;
                let n = st.accumulated.floor();
                st.accumulated -= n;
                n as usize
            }
            "emit noise" => {
                // A rate between the minimum and maximum, varying with noise over time.
                let duration = em.f("emission duration", 0.0);
                if duration > 0.0 && now > start + duration {
                    st.done = true;
                    return 0;
                }
                let (lo, hi) = (em.f("emission minimum", 0.0), em.f("emission maximum", 100.0));
                let tscale = em.f("time noise coordinate scale", 0.1);
                let n = noise3(Vec3::splat(now * tscale + em.f("time coordinate offset", 0.0)) + Vec3::new(ei as f32 * 17.0, 0.0, 0.0));
                let mut t = if em.b("absolute value", false) { n.abs() } else { n * 0.5 + 0.5 };
                if em.b("invert absolute value", false) {
                    t = 1.0 - t;
                }
                st.accumulated += lerp(lo, hi, t.clamp(0.0, 1.0)) * dt;
                let k = st.accumulated.floor();
                st.accumulated -= k;
                k as usize
            }
            _ => {
                // Unsupported emitter: nothing emitted (see `unsupported`).
                st.done = true;
                0
            }
        }
    }

    fn spawn_particle(&mut self, created: f32) {
        if self.particles.len() >= self.def.max_particles {
            return;
        }
        let def = self.def.clone();
        let c = def.color;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.emitted_last += 1;
        let cp0 = self.cps[0].pos;
        let mut p = Particle {
            pos: cp0,
            prev: cp0,
            lifetime: 1.0,
            radius: def.radius,
            rotation: def.rotation.to_radians(),
            rotation_speed: def.rotation_speed.to_radians(),
            tint: Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32) / 255.0,
            alpha: c[3] as f32 / 255.0,
            alpha2: 1.0,
            created,
            sequence: def.sequence_number as f32,
            sequence1: 0.0,
            trail_length: 0.1,
            yaw: 0.0,
            normal: Vec3::Z,
            id,
            init_radius: 0.0,
            init_alpha: 0.0,
            init_tint: Vec3::ZERO,
            dead: false,
        };
        // Position creators first, then position modifiers, then the rest in
        // authored order (modifiers and remaps read the created position).
        let rank = |f: &str| match f {
            "Position Within Sphere Random" | "Position Along Ring" | "Position Along Path Sequential" | "Position Within Box Random" | "Position Along Path Random" => 0,
            "Position Modify Warp Random" | "Position Modify Offset Random" | "Velocity Repulse from World" | "Position Modify Place On Ground" => 1,
            _ => 2,
        };
        let mut order: Vec<usize> = (0..def.initializers.len()).collect();
        order.sort_by_key(|&i| rank(&def.initializers[i].function));
        for oi in order {
            self.run_initializer(oi, &def.initializers[oi], &mut p);
        }
        p.init_radius = p.radius;
        p.init_alpha = p.alpha;
        p.init_tint = p.tint;
        self.particles.push(p);
    }

    /// Writes an initializer's output. Angle fields arrive in degrees (as the file states them).
    fn set_initial(p: &mut Particle, f: i32, v: f32, scale_initial: bool) {
        let v = if f == field::ROTATION || f == field::YAW { v.to_radians() } else { v };
        if Particle::is_vector_field(f) {
            let cur = p.vector(f);
            p.set_vector(f, if scale_initial { cur * v } else { Vec3::splat(v) });
        } else {
            let cur = p.scalar(f);
            p.set_scalar(f, if scale_initial { cur * v } else { v });
        }
    }

    fn run_initializer(&mut self, oi: usize, op: &OpDef, p: &mut Particle) {
        let seed = self.seed;
        let id = p.id;
        let salt = oi as u32 * 7919 + 17;
        let r = |k: u32| rnd(seed, id, salt, k);
        let rexp = |k: u32, lo: f32, hi: f32, e: f32| lerp(lo, hi, r(k).powf(e.max(1e-3)));
        let dt = self.cur_dt;
        match op.function.as_str() {
            "Position Within Sphere Random" => {
                let cp = self.cp(op.i("control_point_number", 0));
                let dmin = op.f("distance_min", 0.0);
                let dmax = op.f("distance_max", 0.0);
                let bias_v = v3(op.v3("distance_bias", [1.0, 1.0, 1.0]));
                let abs = v3(op.v3("distance_bias_absolute_value", [0.0, 0.0, 0.0]));
                let local_bias = op.b("bias in local system", false);
                let d0 = in_unit_sphere(r(1), r(2), r(3));
                let mut d = d0 * bias_v;
                if abs.x != 0.0 {
                    d.x = d.x.abs();
                }
                if abs.y != 0.0 {
                    d.y = d.y.abs();
                }
                if abs.z != 0.0 {
                    d.z = d.z.abs();
                }
                let dir = d.normalize_or_zero();
                let dist = lerp(dmin, dmax, r(10));
                let world_dir = if local_bias { cp.basis * dir } else { dir };
                p.pos = cp.pos + world_dir * dist;
                let smin = op.f("speed_min", 0.0);
                let smax = op.f("speed_max", 0.0);
                let sexp = op.f("speed_random_exponent", 1.0);
                let speed = rexp(11, smin, smax, sexp);
                let lmin = v3(op.v3("speed_in_local_coordinate_system_min", [0.0; 3]));
                let lmax = v3(op.v3("speed_in_local_coordinate_system_max", [0.0; 3]));
                let local_v = Vec3::new(lerp(lmin.x, lmax.x, r(12)), lerp(lmin.y, lmax.y, r(13)), lerp(lmin.z, lmax.z, r(14)));
                let vel = world_dir * speed + cp.basis * local_v;
                p.prev = p.pos - vel * dt;
            }
            "Position Along Ring" => {
                let cp = self.cp(op.i("control point number", 0));
                let radius = op.f("initial radius", 0.0);
                let thick = op.f("thickness", 0.0);
                let even = op.b("even distribution", false);
                let count = op.f("even distribution count", -1.0);
                let n = if even && count > 0.0 { count } else { self.step_spawned.max(1) as f32 };
                let step = 2.0 * PI / n;
                let counter = self.ring_counters[oi];
                self.ring_counters[oi] = counter.wrapping_add(1);
                let angle = if even { counter as f32 * step } else { r(1) * 2.0 * PI };
                let ring = Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0);
                let rot = Quat::from_euler(
                    EulerRot::ZYX,
                    op.f("yaw", 0.0).to_radians(),
                    op.f("pitch", 0.0).to_radians(),
                    op.f("roll", 0.0).to_radians(),
                );
                let local = rot * ring + in_unit_sphere(r(2), r(3), r(4)) * thick;
                p.pos = cp.to_world(local);
                let speed = lerp(op.f("min initial speed", 0.0), op.f("max initial speed", 0.0), r(5));
                // Outward from the control point, with the file's XY-only option.
                let outward = (p.pos - cp.pos).normalize_or_zero();
                p.prev = p.pos - outward * speed * dt;
                if op.b("XY velocity only", true) {
                    p.prev.z = p.pos.z;
                }
            }
            "Position Modify Warp Random" => {
                let cp = self.cp(op.i("control point number", 0));
                let wmin = v3(op.v3("warp min", [1.0; 3]));
                let wmax = v3(op.v3("warp max", [1.0; 3]));
                let w = Vec3::new(lerp(wmin.x, wmax.x, r(1)), lerp(wmin.y, wmax.y, r(2)), lerp(wmin.z, wmax.z, r(3)));
                // Scale position and previous position in the CP's frame.
                p.pos = cp.to_world(cp.to_local(p.pos) * w);
                p.prev = cp.to_world(cp.to_local(p.prev) * w);
            }
            // Source traces down from each particle; one trace below control
            // point 0 per frame (`ground`) stands in for it (flat floors and
            // water, e.g. GLaDOS's head drag splashes on her chamber's pool).
            "Position Modify Place On Ground" => {
                let Some(ground) = self.ground else { return };
                let reach = op.f("max trace length", 128.0);
                let drop = p.pos.z - ground;
                if (0.0..=reach).contains(&drop) {
                    let z = ground + op.f("offset", 0.0);
                    p.prev.z += z - p.pos.z;
                    p.pos.z = z;
                }
            }
            "Position Modify Offset Random" => {
                let cp = self.cp(op.i("control_point_number", 0));
                let omin = v3(op.v3("offset min", [0.0; 3]));
                let omax = v3(op.v3("offset max", [0.0; 3]));
                let mut o = Vec3::new(lerp(omin.x, omax.x, r(1)), lerp(omin.y, omax.y, r(2)), lerp(omin.z, omax.z, r(3)));
                if op.b("offset proportional to radius 0/1", false) {
                    o *= p.radius;
                }
                if op.b("offset in local space 0/1", false) {
                    o = cp.basis * o;
                }
                p.pos += o;
                p.prev += o;
            }
            "Velocity Repulse from World" => {
                // No world traces here: these effects (portal close, bad
                // surface) are spawned on a surface with CP0's +Z as its
                // normal. A trace of `Trace Length` from a particle on the
                // surface hits it at distance 0, so in offset mode the particle
                // moves the full trace length out along the normal [I].
                if op.b("Offset instead of accelerate", false) {
                    let o = self.cp(0).basis * Vec3::new(0.0, 0.0, op.f("Trace Length", 64.0));
                    p.pos += o;
                    p.prev += o;
                }
            }
            "Position Along Path Sequential" => {
                let start = self.cp(op.i("start control point number", 0)).pos;
                let end = self.cp(op.i("end control point number", 0)).pos;
                let n = op.f("particles to map from start to end", 100.0).max(1.0);
                let loops = op.b("restart behavior (0 = bounce, 1 = loop )", true);
                let mut t = (id as f32 % n) / n;
                if !loops && (id as f32 / n) as u32 % 2 == 1 {
                    t = 1.0 - t;
                }
                let bulge = op.f("bulge", 0.0);
                let mut pos = start.lerp(end, t);
                if bulge != 0.0 {
                    // Approximation: bow the path sideways, zero at both ends.
                    let d = end - start;
                    let side = d.cross(Vec3::Z).normalize_or_zero();
                    pos += side * bulge * (t * PI).sin();
                }
                p.pos = pos;
                p.prev = pos;
            }
            "Lifetime Random" => {
                p.lifetime = rexp(1, op.f("lifetime_min", 0.0), op.f("lifetime_max", 0.0), op.f("lifetime_random_exponent", 1.0));
            }
            "Alpha Random" => {
                let a = rexp(1, op.f("alpha_min", 255.0), op.f("alpha_max", 255.0), op.f("alpha_random_exponent", 1.0));
                p.alpha = a / 255.0;
            }
            "Color Random" => {
                let c1 = op.color("color1", [255; 4]);
                let c2 = op.color("color2", [255; 4]);
                let t = r(1);
                p.tint = Vec3::new(
                    lerp(c1[0] as f32, c2[0] as f32, t),
                    lerp(c1[1] as f32, c2[1] as f32, t),
                    lerp(c1[2] as f32, c2[2] as f32, t),
                ) / 255.0;
            }
            "Radius Random" => {
                p.radius = rexp(1, op.f("radius_min", 1.0), op.f("radius_max", 1.0), op.f("radius_random_exponent", 1.0));
            }
            "Rotation Random" => {
                let init = op.f("rotation_initial", 0.0);
                let off = rexp(1, op.f("rotation_offset_min", 0.0), op.f("rotation_offset_max", 360.0), op.f("rotation_random_exponent", 1.0));
                p.rotation = (init + off).to_radians();
            }
            "Rotation Speed Random" => {
                let s = rexp(1, op.f("rotation_speed_random_min", 0.0), op.f("rotation_speed_random_max", 360.0), op.f("rotation_speed_random_exponent", 1.0));
                p.rotation_speed = (op.f("rotation_speed_constant", 0.0) + s).to_radians();
            }
            "Sequence Random" => {
                let lo = op.i("sequence_min", 0);
                let hi = op.i("sequence_max", 0).max(lo);
                p.sequence = (lo + ((hi - lo + 1) as f32 * r(1)) as i32).min(hi) as f32;
            }
            "Trail Length Random" => {
                p.trail_length = rexp(1, op.f("length_min", 0.1), op.f("length_max", 0.1), op.f("length_random_exponent", 1.0));
            }
            "Velocity Random" => {
                let cp = self.cp(op.i("control_point_number", 0));
                let smin = op.f("random_speed_min", 0.0);
                let smax = op.f("random_speed_max", 0.0);
                let lmin = v3(op.v3("speed_in_local_coordinate_system_min", [0.0; 3]));
                let lmax = v3(op.v3("speed_in_local_coordinate_system_max", [0.0; 3]));
                let local_v = Vec3::new(lerp(lmin.x, lmax.x, r(1)), lerp(lmin.y, lmax.y, r(2)), lerp(lmin.z, lmax.z, r(3)));
                let rand_v = in_unit_sphere(r(4), r(5), 1.0).normalize_or_zero() * lerp(smin, smax, r(6));
                let vel = cp.basis * local_v + rand_v;
                p.prev -= vel * dt;
            }
            "Normal Modify Offset Random" => {
                let cp = self.cp(op.i("control_point_number", 0));
                let omin = v3(op.v3("offset min", [0.0; 3]));
                let omax = v3(op.v3("offset max", [0.0; 3]));
                let mut o = Vec3::new(lerp(omin.x, omax.x, r(1)), lerp(omin.y, omax.y, r(2)), lerp(omin.z, omax.z, r(3)));
                if op.b("offset in local space 0/1", false) {
                    o = cp.basis * o;
                }
                p.normal += o;
                if op.b("normalize output 0/1", false) {
                    p.normal = p.normal.normalize_or_zero();
                }
            }
            "Remap Noise to Scalar" => {
                // Defaults from particles-params.md: time scale 0.1, spatial 0.001.
                let f = op.i("output field", field::RADIUS);
                let omin = op.f("output minimum", 0.0);
                let omax = op.f("output maximum", 1.0);
                let tscale = op.f("time noise coordinate scale", 0.1);
                let sscale = op.f("spatial noise coordinate scale", 0.001);
                let soff = v3(op.v3("spatial coordinate offset", [0.0; 3]));
                let toff = op.f("time coordinate offset", 0.0);
                let real_ms = self.time * 1000.0;
                let q = (p.pos + soff) * sscale + Vec3::splat(real_ms * tscale + p.created + toff);
                let n = noise3(q);
                let mut t = if op.b("absolute value", false) { n.abs() } else { n * 0.5 + 0.5 };
                if op.b("invert absolute value", false) {
                    t = 1.0 - t;
                }
                Self::set_initial(p, f, lerp(omin, omax, t.clamp(0.0, 1.0)), false);
            }
            "Remap Particle Count to Scalar" => {
                let f = op.i("output field", field::RADIUS);
                let imin = op.f("input minimum", 0.0);
                let imax = op.f("input maximum", 10.0);
                let omin = op.f("output minimum", 0.0);
                let omax = op.f("output maximum", 1.0);
                let only = op.b("only active within specified input range", false);
                let c = id as f32;
                if only && (c < imin || c > imax) {
                    return;
                }
                let v = remap_clamped(c, imin, imax, omin, omax);
                Self::set_initial(p, f, v, op.b("output is scalar of initial random range", false));
            }
            "Remap Control Point to Vector" => {
                let cp = self.cp(op.i("input control point number", 0));
                let f = op.i("output field", field::XYZ);
                let imin = v3(op.v3("input minimum", [0.0; 3]));
                let imax = v3(op.v3("input maximum", [0.0; 3]));
                let omin = v3(op.v3("output minimum", [0.0; 3]));
                let omax = v3(op.v3("output maximum", [0.0; 3]));
                let v = Vec3::new(
                    remap_clamped(cp.pos.x, imin.x, imax.x, omin.x, omax.x),
                    remap_clamped(cp.pos.y, imin.y, imax.y, omin.y, omax.y),
                    remap_clamped(cp.pos.z, imin.z, imax.z, omin.z, omax.z),
                );
                if Particle::is_vector_field(f) {
                    p.set_vector(f, v);
                } else {
                    p.set_scalar(f, v.x);
                }
            }
            "Remap Initial Distance to Control Point to Scalar" => {
                let cp = self.cp(op.i("control point", 0));
                let f = op.i("output field", field::RADIUS);
                let dmin = op.f("distance minimum", 0.0);
                let dmax = op.f("distance maximum", 128.0);
                let omin = op.f("output minimum", 0.0);
                let omax = op.f("output maximum", 1.0);
                let d = p.pos.distance(cp.pos);
                if op.b("only active within specified distance", false) && (d < dmin || d > dmax) {
                    return;
                }
                let v = remap_clamped(d, dmin, dmax, omin, omax);
                Self::set_initial(p, f, v, op.b("output is scalar of initial random range", false));
            }
            "remap initial scalar" => {
                // Default input is creation time (particles-params.md).
                let fin = op.i("input field", field::CREATION_TIME);
                let f = op.i("output field", field::RADIUS);
                let v = p.scalar(fin);
                let imin = op.f("input minimum", 0.0);
                let imax = op.f("input maximum", 1.0);
                if op.b("only active within specified input range", false) && (v < imin || v > imax) {
                    return;
                }
                let out = remap_clamped(v, imin, imax, op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                Self::set_initial(p, f, out, op.b("output is scalar of initial random range", false));
            }
            "Position Within Box Random" => {
                let cp = self.cp(op.i("control point number", 0));
                let (lo, hi) = (v3(op.v3("min", [0.0; 3])), v3(op.v3("max", [0.0; 3])));
                let mut o = Vec3::new(lerp(lo.x, hi.x, r(1)), lerp(lo.y, hi.y, r(2)), lerp(lo.z, hi.z, r(3)));
                if op.b("use local space", false) {
                    o = cp.basis * o;
                }
                p.pos = cp.pos + o;
                p.prev = p.pos;
            }
            "Position Along Path Random" => {
                let start = self.cp(op.i("start control point number", 0)).pos;
                let end = self.cp(op.i("end control point number", 0)).pos;
                let t = r(1);
                let mut pos = start.lerp(end, t);
                let bulge = op.f("bulge", 0.0);
                if bulge != 0.0 {
                    // Approximation: a random sideways bow, zero at both ends.
                    let side = in_unit_sphere(r(2), r(3), 1.0).normalize_or_zero();
                    pos += side * bulge * (t * PI).sin() * start.distance(end);
                }
                p.pos = pos;
                p.prev = pos;
            }
            "Velocity Noise" | "Initial Velocity Noise" => {
                let cp = self.cp(op.i("Control Point Number", 0));
                let tscale = op.f("Time Noise Coordinate Scale", 1.0);
                let sscale = op.f("Spatial Noise Coordinate Scale", 0.01);
                let toff = op.f("Time Coordinate Offset", 0.0);
                let soff = v3(op.v3("Spatial Coordinate Offset", [0.0; 3]));
                let (omin, omax) = (v3(op.v3("output minimum", [0.0; 3])), v3(op.v3("output maximum", [1.0; 3])));
                let abs = op.b("Absolute Value", false);
                let base = (p.pos + soff) * sscale + Vec3::splat(self.time * tscale + toff + id as f32 * 0.013);
                let ch = |k: f32| {
                    let n = noise3(base + Vec3::splat(k * 31.7));
                    if abs { n.abs() } else { n * 0.5 + 0.5 }
                };
                let mut vel = Vec3::new(lerp(omin.x, omax.x, ch(0.0)), lerp(omin.y, omax.y, ch(1.0)), lerp(omin.z, omax.z, ch(2.0)));
                if op.b("Apply Velocity in Local Space (0/1)", false) {
                    vel = cp.basis * vel;
                }
                p.prev -= vel * dt;
            }
            "Velocity Inherit from Control Point" => {
                let i = op.i("control point number", 0).clamp(0, 63) as usize;
                let scale = op.f("velocity scale", 1.0);
                let vel = (self.cps[i].pos - self.prev_cps[i].pos) / self.last_dt.max(1e-4);
                p.prev -= vel * scale * dt;
            }
            "Rotation Yaw Flip Random" => {
                if r(1) < op.f("Flip Percentage", 0.5) {
                    p.yaw += PI;
                }
            }
            "Rotation Yaw Random" => {
                let init = op.f("yaw_initial", 0.0);
                let off = rexp(1, op.f("yaw_offset_min", 0.0), op.f("yaw_offset_max", 360.0), op.f("yaw_random_exponent", 1.0));
                p.yaw = (init + off).to_radians();
            }
            "Sequence Two Random" => {
                let lo = op.i("sequence_min", 0);
                let hi = op.i("sequence_max", 0).max(lo);
                p.sequence1 = (lo + ((hi - lo + 1) as f32 * r(1)) as i32).min(hi) as f32;
            }
            "Remap Control Point to Scalar" => {
                let cp = self.cp(op.i("input control point number", 0));
                let comp = op.i("input field 0-2 X/Y/Z", 0).clamp(0, 2) as usize;
                let f = op.i("output field", field::RADIUS);
                let v = remap_clamped(cp.pos[comp], op.f("input minimum", 0.0), op.f("input maximum", 1.0), op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                Self::set_initial(p, f, v, op.b("output is scalar of initial random range", false));
            }
            _ => {}
        }
    }

    fn op_cp_to_player(&mut self, op: &OpDef) {
        let Some(eye) = self.player else { return };
        let i = op.i("Control Point Number", 1).clamp(0, 63) as usize;
        let off = v3(op.v3("Control Point Offset", [0.0; 3]));
        let basis = if op.b("Use Eye Orientation", false) { eye.basis } else { Mat3::IDENTITY };
        self.cps[i] = ControlPoint { pos: eye.pos + basis * off, basis, set: true };
    }

    /// `Restart Effect after Duration`: the emitters start over every few seconds.
    fn op_restart(&mut self, op: &OpDef, now: f32) {
        let (lo, hi) = (op.f("Minimum Restart Time", 0.0), op.f("Maximum Restart Time", 1.0));
        let at = *self.restart_at.get_or_insert_with(|| now + lerp(lo, hi, hash(self.seed, 0x5e57, now.to_bits())).max(0.05));
        if now < at {
            return;
        }
        if op.b("Only Restart Children", false) {
            for c in &mut self.children {
                c.system = None;
            }
        } else {
            self.emit_epoch = now;
            for e in &mut self.emitters {
                *e = EmitterState::default();
            }
        }
        self.restart_at = Some(now + lerp(lo, hi, hash(self.seed, 0x5e58, now.to_bits())).max(0.05));
    }

    fn run_operators(&mut self, def: &SystemDef, dt: f32, now: f32) {
        let cps = self.cps;
        let prev_cps = self.prev_cps;
        let seed = self.seed;
        let prev_dt = self.last_dt.max(1e-4);
        for (oi, op) in def.operators.iter().enumerate() {
            if !self.op_active(op) {
                continue;
            }
            let strength = Self::op_strength(op, now);
            if strength <= 0.0 {
                continue;
            }
            let salt = oi as u32 * 7919 + 101;
            let rand = |id: u32, k: u32| rnd(seed, id, salt, k);
            let cpn = |k: &str, d: i32| cps[op.i(k, d).clamp(0, 63) as usize];
            match op.function.as_str() {
                "Movement Basic" => {
                    let g = v3(op.v3("gravity", [0.0; 3]));
                    let drag = op.f("drag", 0.0);
                    let mut decay = if drag > 0.0 { (1.0 - drag).clamp(0.0, 1.0).powf(dt * DRAG_REF_HZ) } else { 1.0 };
                    decay *= dt / prev_dt;
                    for p in &mut self.particles {
                        let vel = p.pos - p.prev;
                        let new = p.pos + vel * decay + g * (dt * dt) * strength;
                        p.prev = p.pos;
                        p.pos = new;
                    }
                }
                "Movement Lock to Control Point" => {
                    let i = op.i("control_point_number", 0).clamp(0, 63) as usize;
                    let (a, b) = (prev_cps[i], cps[i]);
                    let lock_rot = op.b("lock rotation", false);
                    let (start_min, start_max) = (op.f("start_fadeout_min", 1.0), op.f("start_fadeout_max", 1.0));
                    let (end_min, end_max) = (op.f("end_fadeout_min", 1.0), op.f("end_fadeout_max", 1.0));
                    if a == b {
                        continue;
                    }
                    for p in &mut self.particles {
                        // The lock fades out over life between per-particle start and end times.
                        let lf = p.life_fraction(now);
                        let s = lerp(start_min, start_max, rand(p.id, 1));
                        let e = lerp(end_min, end_max, rand(p.id, 2));
                        let w = 1.0 - remap_clamped(lf, s, e, 0.0, 1.0);
                        if w <= 0.0 {
                            continue;
                        }
                        let moved = |x: Vec3| -> Vec3 {
                            if lock_rot {
                                b.to_world(a.to_local(x))
                            } else {
                                x + (b.pos - a.pos)
                            }
                        };
                        p.pos = p.pos.lerp(moved(p.pos), w);
                        p.prev = p.prev.lerp(moved(p.prev), w);
                        if lock_rot {
                            let rot_delta = b.basis * a.basis.transpose();
                            p.normal = rot_delta.mul_vec3(p.normal).normalize_or_zero();
                        }
                    }
                }
                "Movement Dampen Relative to Control Point" => {
                    let cp = cpn("control_point_number", 0);
                    let range = op.f("falloff range", 100.0).max(1e-3);
                    let scale = op.f("dampen scale", 1.0);
                    for p in &mut self.particles {
                        let d = p.pos.distance(cp.pos);
                        let k = (d / range).clamp(0.0, 1.0).powf(scale.max(1e-3));
                        let vel = (p.pos - p.prev) * lerp(1.0, k, strength);
                        p.prev = p.pos - vel;
                    }
                }
                "Movement Max Velocity" => {
                    let maxv = op.f("Maximum Velocity", 0.0) * dt;
                    for p in &mut self.particles {
                        let v = p.pos - p.prev;
                        if v.length() > maxv {
                            p.prev = p.pos - v.normalize_or_zero() * maxv;
                        }
                    }
                }
                "Lifespan Decay" => {
                    for p in &mut self.particles {
                        if p.lifetime <= 0.0 || p.age(now) >= p.lifetime {
                            p.dead = true;
                        }
                    }
                }
                "Lifespan Minimum Radius Decay" => {
                    let minr = op.f("minimum radius", 1.0);
                    for p in &mut self.particles {
                        if p.radius < minr {
                            p.dead = true;
                        }
                    }
                }
                "Lifespan Minimum Alpha Decay" => {
                    let mina = op.f("minimum alpha", 0.0);
                    for p in &mut self.particles {
                        if p.alpha * p.alpha2 <= mina {
                            p.dead = true;
                        }
                    }
                }
                "Alpha Fade In Simple" => {
                    let t = op.f("proportional fade in time", 0.25).max(1e-5);
                    for p in &mut self.particles {
                        let lf = p.life_fraction(now);
                        if lf < t {
                            p.alpha = p.init_alpha * smoothstep(lf / t);
                        }
                    }
                }
                "Alpha Fade Out Simple" => {
                    let t = op.f("proportional fade out time", 0.25).max(1e-5);
                    for p in &mut self.particles {
                        let lf = p.life_fraction(now);
                        if lf > 1.0 - t {
                            p.alpha = p.init_alpha * (1.0 - smoothstep((lf - (1.0 - t)) / t));
                        }
                    }
                }
                "Alpha Fade In Random" => {
                    let proportional = op.b("proportional 0/1", true);
                    let (tmin, tmax) = (op.f("fade in time min", 0.25), op.f("fade in time max", 0.25));
                    for p in &mut self.particles {
                        let t = lerp(tmin, tmax, rand(p.id, 1)).max(1e-5);
                        let a = if proportional { p.life_fraction(now) } else { p.age(now) };
                        if a < t {
                            p.alpha = p.init_alpha * smoothstep(a / t);
                        }
                    }
                }
                "Alpha Fade Out Random" => {
                    let proportional = op.b("proportional 0/1", true);
                    let ease = op.b("ease in and out", true);
                    let (tmin, tmax) = (op.f("fade out time min", 0.25), op.f("fade out time max", 0.25));
                    for p in &mut self.particles {
                        let t = lerp(tmin, tmax, rand(p.id, 1)).max(1e-5);
                        let (left, span) = if proportional {
                            (1.0 - p.life_fraction(now), 1.0)
                        } else {
                            (p.lifetime - p.age(now), 1.0)
                        };
                        if left < t {
                            let f = (left / t).clamp(0.0, 1.0);
                            let s = if ease { smoothstep(f) } else { f };
                            p.alpha = p.init_alpha * s;
                        }
                        let _ = span;
                    }
                }
                "Alpha Fade and Decay" => {
                    let sa = op.f("start_alpha", 1.0);
                    let ea = op.f("end_alpha", 0.0);
                    let sfi = op.f("start_fade_in_time", 0.0);
                    let efi = op.f("end_fade_in_time", 0.5);
                    let sfo = op.f("start_fade_out_time", 0.5);
                    let efo = op.f("end_fade_out_time", 1.0);
                    for p in &mut self.particles {
                        let a = p.life_fraction(now);
                        let s = if a < efi {
                            lerp(sa, 1.0, smoothstep(remap_clamped(a, sfi, efi, 0.0, 1.0)))
                        } else if a > sfo {
                            lerp(1.0, ea, smoothstep(remap_clamped(a, sfo, efo, 0.0, 1.0)))
                        } else {
                            1.0
                        };
                        p.alpha = p.init_alpha * s;
                        if a >= 1.0 {
                            p.dead = true;
                        }
                    }
                }
                "Color Fade" => {
                    let c = op.color("color_fade", [255; 4]);
                    let target = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32) / 255.0;
                    let st = op.f("fade_start_time", 0.0);
                    let et = op.f("fade_end_time", 1.0);
                    let ease = op.b("ease_in_and_out", true);
                    for p in &mut self.particles {
                        let mut t = remap_clamped(p.life_fraction(now), st, et, 0.0, 1.0);
                        if ease {
                            t = smoothstep(t);
                        }
                        p.tint = p.init_tint.lerp(target, t * strength);
                    }
                }
                "Radius Scale" => {
                    let st = op.f("start_time", 0.0);
                    let et = op.f("end_time", 1.0);
                    let rs = op.f("radius_start_scale", 1.0);
                    let re = op.f("radius_end_scale", 1.0);
                    let ease = op.b("ease_in_and_out", false);
                    let b = op.f("scale_bias", 0.5);
                    for p in &mut self.particles {
                        let t = remap_clamped(p.life_fraction(now), st, et, 0.0, 1.0);
                        let s = if ease { smoothstep(t) } else { bias(t, b) };
                        let target = p.init_radius * lerp(rs, re, s);
                        p.radius = lerp(p.radius, target, strength);
                    }
                }
                "Clamp Scalar" => {
                    let f = op.i("output field", field::RADIUS);
                    let lo = op.f("output minimum", 0.0);
                    let hi = op.f("output maximum", 1.0);
                    for p in &mut self.particles {
                        let v = p.scalar(f);
                        p.set_scalar(f, lerp(v, v.clamp(lo.min(hi), hi.max(lo)), strength));
                    }
                }
                "Ramp Scalar Linear Simple" => {
                    let f = op.i("ramp field", field::RADIUS);
                    let rate = op.f("ramp rate", 0.0);
                    let st = op.f("start time", 0.0);
                    let et = op.f("end time", 1.0);
                    for p in &mut self.particles {
                        let lf = p.life_fraction(now);
                        if lf >= st && lf < et && p.lifetime > 0.0 {
                            let v = p.scalar(f) + rate * dt * strength;
                            p.set_scalar(f, v);
                        }
                    }
                }
                "Oscillate Scalar" => {
                    let f = op.i("oscillation field", field::ALPHA);
                    let (rmin, rmax) = (op.f("oscillation rate min", 0.0), op.f("oscillation rate max", 0.0));
                    let (fmin, fmax) = (op.f("oscillation frequency min", 1.0), op.f("oscillation frequency max", 1.0));
                    let proportional = op.b("proportional 0/1", true);
                    let (stmin, stmax) = (op.f("start time min", 0.0), op.f("start time max", 0.0));
                    let (etmin, etmax) = (op.f("end time min", 1.0), op.f("end time max", 1.0));
                    let mult = op.f("oscillation multiplier", 2.0);
                    let phase = op.f("oscillation start phase", 0.5);
                    for p in &mut self.particles {
                        let t = if proportional { p.life_fraction(now) } else { p.age(now) };
                        let st = lerp(stmin, stmax, rand(p.id, 3));
                        let et = lerp(etmin, etmax, rand(p.id, 4));
                        if t < st || t > et {
                            continue;
                        }
                        let rate = lerp(rmin, rmax, rand(p.id, 1));
                        let freq = lerp(fmin, fmax, rand(p.id, 2));
                        let s = ((t * freq * mult + phase) * PI).sin();
                        let v = p.scalar(f) + rate * s * dt * strength;
                        p.set_scalar(f, v);
                    }
                }
                "Oscillate Vector" => {
                    let f = op.i("oscillation field", field::XYZ);
                    let rmin = v3(op.v3("oscillation rate min", [0.0; 3]));
                    let rmax = v3(op.v3("oscillation rate max", [0.0; 3]));
                    let fmin = v3(op.v3("oscillation frequency min", [1.0; 3]));
                    let fmax = v3(op.v3("oscillation frequency max", [1.0; 3]));
                    let proportional = op.b("proportional 0/1", true);
                    let (stmin, stmax) = (op.f("start time min", 0.0), op.f("start time max", 0.0));
                    let (etmin, etmax) = (op.f("end time min", 1.0), op.f("end time max", 1.0));
                    let mult = op.f("oscillation multiplier", 2.0);
                    let phase = op.f("oscillation start phase", 0.5);
                    for p in &mut self.particles {
                        let t = if proportional { p.life_fraction(now) } else { p.age(now) };
                        let st = lerp(stmin, stmax, rand(p.id, 7));
                        let et = lerp(etmin, etmax, rand(p.id, 8));
                        if t < st || t > et {
                            continue;
                        }
                        let rate = Vec3::new(
                            lerp(rmin.x, rmax.x, rand(p.id, 1)),
                            lerp(rmin.y, rmax.y, rand(p.id, 2)),
                            lerp(rmin.z, rmax.z, rand(p.id, 3)),
                        );
                        let freq = Vec3::new(
                            lerp(fmin.x, fmax.x, rand(p.id, 4)),
                            lerp(fmin.y, fmax.y, rand(p.id, 5)),
                            lerp(fmin.z, fmax.z, rand(p.id, 6)),
                        );
                        let s = Vec3::new(
                            ((t * freq.x * mult + phase) * PI).sin(),
                            ((t * freq.y * mult + phase) * PI).sin(),
                            ((t * freq.z * mult + phase) * PI).sin(),
                        );
                        let d = rate * s * dt * strength;
                        let v = p.vector(f) + d;
                        p.set_vector(f, v);
                        if f == field::XYZ {
                            p.prev += d;
                        }
                    }
                }
                "Remap Direction to CP to Vector" => {
                    let cp = cpn("control point", 0);
                    let f = op.i("output field", field::NORMAL);
                    let scale = op.f("scale factor", 1.0);
                    let rot = op.f("offset rotation", 0.0).to_radians();
                    let axis = v3(op.v3("offset axis", [0.0; 3])).normalize_or_zero();
                    let norm = op.b("normalize", false);
                    for p in &mut self.particles {
                        // Particle minus CP: points away from the control point.
                        let mut d = p.pos - cp.pos;
                        if axis != Vec3::ZERO && rot != 0.0 {
                            d = Quat::from_axis_angle(axis, rot) * d;
                        }
                        let out = if norm { d.normalize_or_zero() * scale } else { d * scale / dt };
                        p.set_vector(f, out);
                    }
                }
                "Remap Control Point to Vector" => {
                    let cp = cpn("input control point number", 0);
                    let f = op.i("output field", field::XYZ);
                    let imin = v3(op.v3("input minimum", [0.0; 3]));
                    let imax = v3(op.v3("input maximum", [0.0; 3]));
                    let omin = v3(op.v3("output minimum", [0.0; 3]));
                    let omax = v3(op.v3("output maximum", [0.0; 3]));
                    let v = Vec3::new(
                        remap_clamped(cp.pos.x, imin.x, imax.x, omin.x, omax.x),
                        remap_clamped(cp.pos.y, imin.y, imax.y, omin.y, omax.y),
                        remap_clamped(cp.pos.z, imin.z, imax.z, omin.z, omax.z),
                    );
                    for p in &mut self.particles {
                        p.set_vector(f, v);
                    }
                }
                "Rotation Basic" => {
                    for p in &mut self.particles {
                        p.rotation += p.rotation_speed * dt * strength;
                    }
                }
                "Cull when crossing sphere" => {
                    let cp = cpn("Control Point", 0);
                    let dist = op.f("Cull Distance", 0.0);
                    let inside = op.b("Cull inside instead of outside", false);
                    for p in &mut self.particles {
                        let d = p.pos.distance(cp.pos);
                        if (inside && d < dist) || (!inside && d > dist) {
                            p.dead = true;
                        }
                    }
                }
                "Rotation Spin Roll" | "Rotation Spin Yaw" => {
                    let rate = op.f("spin_rate_degrees", 0.0).to_radians();
                    let min = op.f("spin_rate_min", 0.0).to_radians();
                    let stop = op.f("spin_stop_time", 0.0);
                    let yaw = op.function == "Rotation Spin Yaw";
                    for p in &mut self.particles {
                        // Slows from the rate to the minimum over `spin_stop_time` seconds of age.
                        let r = if stop > 0.0 { lerp(rate, min, (p.age(now) / stop).clamp(0.0, 1.0)) } else { rate };
                        if yaw {
                            p.yaw += r * dt * strength;
                        } else {
                            p.rotation += r * dt * strength;
                        }
                    }
                }
                "Noise Scalar" => {
                    let f = op.i("output field", field::RADIUS);
                    let (lo, hi) = (op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                    let scale = op.f("noise coordinate scale", 0.1);
                    let additive = op.b("additive", false);
                    for p in &mut self.particles {
                        let n = noise3(p.pos * scale * 0.01 + Vec3::splat(now * scale + p.id as f32 * 0.37)) * 0.5 + 0.5;
                        let v = lerp(lo, hi, n);
                        let out = if additive { p.scalar(f) + v * dt } else { lerp(p.scalar(f), v, strength) };
                        p.set_scalar(f, out);
                    }
                }
                "Noise Vector" => {
                    let f = op.i("output field", field::TINT_RGB);
                    let (lo, hi) = (v3(op.v3("output minimum", [0.0; 3])), v3(op.v3("output maximum", [1.0; 3])));
                    let scale = op.f("noise coordinate scale", 0.1);
                    let additive = op.b("additive", false);
                    for p in &mut self.particles {
                        let q = p.pos * scale * 0.01 + Vec3::splat(now * scale + p.id as f32 * 0.37);
                        let n = |k: f32| noise3(q + Vec3::splat(k * 19.1)) * 0.5 + 0.5;
                        let v = Vec3::new(lerp(lo.x, hi.x, n(0.0)), lerp(lo.y, hi.y, n(1.0)), lerp(lo.z, hi.z, n(2.0)));
                        let out = if additive { p.vector(f) + v * dt } else { p.vector(f).lerp(v, strength) };
                        p.set_vector(f, out);
                        if f == field::XYZ && additive {
                            p.prev += v * dt;
                        }
                    }
                }
                "Remap Distance to Control Point to Scalar" => {
                    let cp = cpn("control point", 0);
                    let f = op.i("output field", field::RADIUS);
                    let (dmin, dmax) = (op.f("distance minimum", 0.0), op.f("distance maximum", 128.0));
                    let (omin, omax) = (op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                    let only = op.b("only active within specified distance", false);
                    for p in &mut self.particles {
                        let d = p.pos.distance(cp.pos);
                        if only && (d < dmin || d > dmax) {
                            continue;
                        }
                        let v = remap_clamped(d, dmin, dmax, omin, omax);
                        let cur = p.scalar(f);
                        p.set_scalar(f, lerp(cur, v, strength));
                    }
                }
                "Remap Speed to Scalar" => {
                    let f = op.i("output field", field::RADIUS);
                    let (imin, imax) = (op.f("Input minimum", 0.0), op.f("Input maximum", 1.0));
                    let (omin, omax) = (op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                    for p in &mut self.particles {
                        let speed = (p.pos - p.prev).length() / prev_dt;
                        let cur = p.scalar(f);
                        p.set_scalar(f, lerp(cur, remap_clamped(speed, imin, imax, omin, omax), strength));
                    }
                }
                "Remap Scalar" => {
                    let fin = op.i("Input field", field::ALPHA);
                    let f = op.i("output field", field::RADIUS);
                    let (imin, imax) = (op.f("Input minimum", 0.0), op.f("Input maximum", 1.0));
                    let (omin, omax) = (op.f("output minimum", 0.0), op.f("output maximum", 1.0));
                    for p in &mut self.particles {
                        let v = remap_clamped(p.scalar(fin), imin, imax, omin, omax);
                        let cur = p.scalar(f);
                        p.set_scalar(f, lerp(cur, v, strength));
                    }
                }
                "Lifespan Endcap Timed Decay" => {
                    let decay = op.f("decay time", 1.0);
                    if let Some(t0) = self.end_cap_at {
                        if now - t0 >= decay {
                            self.particles.iter_mut().for_each(|p| p.dead = true);
                        }
                    }
                }
                "Ramp Scalar Linear Random" | "Ramp Scalar Spline Random" => {
                    let f = op.i("ramp field", field::RADIUS);
                    let (rmin, rmax) = (op.f("ramp rate min", 0.0), op.f("ramp rate max", 0.0));
                    let (stmin, stmax) = (op.f("start time min", 0.0), op.f("start time max", 0.0));
                    let (etmin, etmax) = (op.f("end time min", 1.0), op.f("end time max", 1.0));
                    let proportional = op.b("proportional 0/1", true);
                    let spline = op.function == "Ramp Scalar Spline Random";
                    for p in &mut self.particles {
                        let t = if proportional { p.life_fraction(now) } else { p.age(now) };
                        let (st, et) = (lerp(stmin, stmax, rand(p.id, 2)), lerp(etmin, etmax, rand(p.id, 3)));
                        if t < st || t >= et {
                            continue;
                        }
                        let mut rate = lerp(rmin, rmax, rand(p.id, 1));
                        if spline {
                            // Eases in and out across the window (approximation of the spline).
                            let k = ((t - st) / (et - st).max(1e-4)).clamp(0.0, 1.0);
                            rate *= 6.0 * k * (1.0 - k);
                        }
                        let v = p.scalar(f) + rate * dt * strength;
                        p.set_scalar(f, v);
                    }
                }
                "Oscillate Scalar Simple" => {
                    let f = op.i("oscillation field", field::ALPHA);
                    let rate = op.f("Oscillation Rate", 0.0);
                    let freq = op.f("Oscillation Frequency", 1.0);
                    for p in &mut self.particles {
                        let s = (p.age(now) * freq * 2.0 * PI).sin();
                        let v = p.scalar(f) + rate * s * dt * strength;
                        p.set_scalar(f, v);
                    }
                }
                "Movement Rotate Particle Around Axis" => {
                    let cp = cpn("Control Point", 0);
                    let mut axis = v3(op.v3("Rotation Axis", [0.0, 0.0, 1.0])).normalize_or(Vec3::Z);
                    if op.b("Use Local Space", false) {
                        axis = cp.basis * axis;
                    }
                    let q = Quat::from_axis_angle(axis, op.f("Rotation Rate", 180.0).to_radians() * dt * strength);
                    for p in &mut self.particles {
                        p.pos = cp.pos + q * (p.pos - cp.pos);
                        p.prev = cp.pos + q * (p.prev - cp.pos);
                    }
                }
                "Cull Random" => {
                    let pct = op.f("Cull Percentage", 0.5);
                    let (st, et) = (op.f("Cull Start Time", 0.0), op.f("Cull End Time", 1.0));
                    let e = op.f("Cull Time Exponent", 1.0);
                    for p in &mut self.particles {
                        let lf = p.life_fraction(now);
                        if lf < st || lf > et || self.spared.contains(&p.id) {
                            continue;
                        }
                        // Each particle gets one draw, at a time in the window.
                        let when = lerp(st, et, rand(p.id, 2).powf(e.max(1e-3)));
                        if lf >= when {
                            if rand(p.id, 1) < pct {
                                p.dead = true;
                            } else {
                                self.spared.insert(p.id);
                            }
                        }
                    }
                }
                "Rotation Orient to 2D Direction" => {
                    let off = op.f("Rotation Offset", 0.0).to_radians();
                    let spin = op.f("Spin Strength", 1.0);
                    for p in &mut self.particles {
                        let v = p.pos - p.prev;
                        if v.x == 0.0 && v.y == 0.0 {
                            continue;
                        }
                        let target = v.y.atan2(v.x) + off;
                        p.rotation = lerp(p.rotation, target, (spin * strength).clamp(0.0, 1.0));
                    }
                }
                "Set child control points from particle positions" => {
                    let first_cp = op.i("First control point to set", 3).clamp(0, 63) as usize;
                    let count = op.i("# of control points to set", 1).max(0) as usize;
                    let first_p = op.i("First particle to copy", 0).max(0) as usize;
                    let mut ordered: Vec<&Particle> = self.particles.iter().collect();
                    ordered.sort_by_key(|p| p.id);
                    let pts: Vec<Vec3> = ordered.iter().skip(first_p).take(count).map(|p| p.pos).collect();
                    for c in &mut self.children {
                        if let Some(sys) = &mut c.system {
                            for (k, pos) in pts.iter().enumerate() {
                                if first_cp + k < MAX_CONTROL_POINTS {
                                    sys.cps[first_cp + k] = ControlPoint { pos: *pos, basis: Mat3::IDENTITY, set: true };
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if self.spared.len() > 4096 {
            let alive: BTreeSet<u32> = self.particles.iter().map(|p| p.id).collect();
            self.spared.retain(|id| alive.contains(id));
        }
    }

    fn run_forces(&mut self, def: &SystemDef, dt: f32) {
        let cps = self.cps;
        let seed = self.seed;
        for (fi, op) in def.forces.iter().enumerate() {
            if !self.op_active(op) {
                continue;
            }
            let strength = Self::op_strength(op, self.time);
            match op.function.as_str() {
                "Pull towards control point" => {
                    let cp = cps[op.i("control point number", 0).clamp(0, 63) as usize];
                    let amount = op.f("amount of force", 0.0);
                    let falloff = op.f("falloff power", 2.0);
                    for p in &mut self.particles {
                        let d = cp.pos - p.pos;
                        let len = d.length().max(1.0);
                        let force = d / len * amount / len.powf(falloff).max(1e-3);
                        p.prev -= force * (dt * dt) * strength;
                    }
                }
                "random force" => {
                    let lo = v3(op.v3("min force", [0.0; 3]));
                    let hi = v3(op.v3("max force", [0.0; 3]));
                    let k = (self.time * 30.0) as u32;
                    for p in &mut self.particles {
                        let f = Vec3::new(
                            lerp(lo.x, hi.x, hash(p.id ^ seed, fi as u32, k * 3)),
                            lerp(lo.y, hi.y, hash(p.id ^ seed, fi as u32, k * 3 + 1)),
                            lerp(lo.z, hi.z, hash(p.id ^ seed, fi as u32, k * 3 + 2)),
                        );
                        p.prev -= f * (dt * dt) * strength;
                    }
                }
                "twist around axis" => {
                    let amount = op.f("amount of force", 0.0);
                    let mut axis = v3(op.v3("twist axis", [0.0, 0.0, 1.0])).normalize_or(Vec3::Z);
                    let cp = cps[op.i("control point number", 0).clamp(0, 63) as usize];
                    if op.b("object local space axis 0/1", false) {
                        axis = cp.basis * axis;
                    }
                    for p in &mut self.particles {
                        let r = p.pos - cp.pos;
                        let tangent = axis.cross(r).normalize_or_zero();
                        p.prev -= tangent * amount * (dt * dt) * strength;
                    }
                }
                _ => {}
            }
        }
    }

    fn run_constraints(&mut self, def: &SystemDef) {
        let cps = self.cps;
        for op in &def.constraints {
            if op.function == "Prevent passing through a plane" {
                let cp = cps[op.i("control point number", 0).clamp(0, 63) as usize];
                let mut point = v3(op.v3("plane point", [0.0; 3]));
                let mut normal = v3(op.v3("plane normal", [0.0, 0.0, 1.0])).normalize_or(Vec3::Z);
                if !op.b("global origin", false) {
                    point = cp.to_world(point);
                }
                if !op.b("global normal", false) {
                    normal = (cp.basis * normal).normalize_or(Vec3::Z);
                }
                for p in &mut self.particles {
                    let d = (p.pos - point).dot(normal);
                    if d < 0.0 {
                        p.pos -= normal * d;
                    }
                }
                continue;
            }
            if op.function == "Constrain distance to control point" {
                let cp = cps[op.i("control point number", 0).clamp(0, 63) as usize];
                let mind = op.f("minimum distance", 0.0);
                let maxd = op.f("maximum distance", 100.0);
                let off = v3(op.v3("offset of center", [0.0; 3]));
                let center = cp.pos + off;
                for p in &mut self.particles {
                    let d = p.pos - center;
                    let len = d.length();
                    if len > maxd && len > 0.0 {
                        p.pos = center + d / len * maxd;
                    } else if len < mind && len > 0.0 {
                        p.pos = center + d / len * mind;
                    }
                }
            }
        }
    }

    /// Whether this system or a running child places particles on the ground.
    pub fn wants_ground(&self) -> bool {
        let mut any = false;
        self.for_each(&mut |s| any |= s.def.initializers.iter().any(|o| o.function == "Position Modify Place On Ground"));
        any
    }

    /// Give this system and its running children the ground height below
    /// their control point 0 (`trace` gets that point, Source space).
    pub fn set_ground(&mut self, trace: &mut dyn FnMut(Vec3) -> Option<f32>) {
        self.ground = trace(self.cps[0].pos);
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.set_ground(trace);
            }
        }
    }

    /// The control point and radius (Source units) of the first renderer
    /// with a visibility proxy (`Visibility Proxy Input Control Point Number`).
    fn visibility_proxy(&self) -> Option<(usize, f32)> {
        self.def.renderers.iter().find_map(|r| {
            let cp = r.i("Visibility Proxy Input Control Point Number", -1);
            (0..MAX_CONTROL_POINTS as i32).contains(&cp).then(|| (cp as usize, r.f("Visibility Proxy Radius", 1.0)))
        })
    }

    /// Whether this system or a running child fades by a visibility proxy.
    pub fn wants_visibility(&self) -> bool {
        let mut any = false;
        self.for_each(&mut |s| any |= s.visibility_proxy().is_some());
        any
    }

    /// Give this system and its running children how visible their proxy
    /// is (`trace` gets the proxy point, Source space, and its radius).
    /// Source measures it with an occlusion query; a line of sight stands
    /// in for it here.
    pub fn set_visibility(&mut self, trace: &mut dyn FnMut(Vec3, f32) -> f32) {
        self.visibility = self.visibility_proxy().map_or(1.0, |(cp, r)| trace(self.cps[cp].pos, r));
        for c in &mut self.children {
            if let Some(s) = &mut c.system {
                s.set_visibility(trace);
            }
        }
    }

    /// The alpha scale renderer `r` gets from the proxy's visibility:
    /// `Visibility input minimum/maximum` mapped onto `Visibility Alpha Scale
    /// minimum/maximum` (1 without a proxy).
    pub fn visibility_alpha(&self, r: &OpDef) -> f32 {
        if r.i("Visibility Proxy Input Control Point Number", -1) < 0 {
            return 1.0;
        }
        let (lo, hi) = (r.f("Visibility input minimum", 0.0), r.f("Visibility input maximum", 1.0));
        let t = if hi > lo { ((self.visibility - lo) / (hi - lo)).clamp(0.0, 1.0) } else { (self.visibility >= hi) as u8 as f32 };
        let (a0, a1) = (r.f("Visibility Alpha Scale minimum", 0.0), r.f("Visibility Alpha Scale maximum", 1.0));
        a0 + (a1 - a0) * t
    }

    /// Visit this system and all running children.
    pub fn for_each<'a>(&'a self, f: &mut dyn FnMut(&'a SystemInstance)) {
        f(self);
        for c in &self.children {
            if let Some(s) = &c.system {
                s.for_each(f);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_and_bias() {
        assert_eq!(remap_clamped(5.0, 0.0, 10.0, 0.0, 1.0), 0.5);
        assert_eq!(remap_clamped(50.0, 0.0, 10.0, 0.0, 1.0), 1.0);
        assert!((bias(0.5, 0.5) - 0.5).abs() < 1e-5);
        assert!(bias(0.5, 0.8) > 0.5);
    }

    #[test]
    fn smoothstep_endpoints() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
    }

    use crate::dmx::Attr;

    fn op(function: &str, params: &[(&str, Attr)]) -> OpDef {
        OpDef { function: function.into(), params: params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() }
    }

    fn system(emitters: Vec<OpDef>, initializers: Vec<OpDef>, operators: Vec<OpDef>, constraints: Vec<OpDef>) -> Arc<SystemDef> {
        Arc::new(SystemDef {
            name: "test".into(),
            material: String::new(),
            max_particles: 500,
            initial_particles: 0,
            radius: 5.0,
            color: [255; 4],
            rotation: 0.0,
            rotation_speed: 0.0,
            sequence_number: 0,
            control_point_to_disable_rendering_if_it_is_the_camera: -1,
            maximum_time_step: 0.1,
            maximum_sim_tick_rate: 0.0,
            minimum_sim_tick_rate: 0.0,
            view_model_effect: false,
            screen_space_effect: false,
            emitters,
            initializers,
            operators,
            renderers: Vec::new(),
            forces: Vec::new(),
            constraints,
            children: Vec::new(),
        })
    }

    fn burst(n: i32) -> OpDef {
        op("emit_instantaneously", &[("num_to_emit", Attr::Int(n))])
    }

    fn lifetime(t: f32) -> OpDef {
        op("Lifetime Random", &[("lifetime_min", Attr::Float(t)), ("lifetime_max", Attr::Float(t))])
    }

    fn run(def: Arc<SystemDef>, steps: usize) -> SystemInstance {
        let lib = ParticleLibrary::default();
        let mut s = SystemInstance::new(def, 7);
        s.set_cp(0, ControlPoint::new(Vec3::new(100.0, 0.0, 0.0), Mat3::IDENTITY));
        for _ in 0..steps {
            s.update(1.0 / 30.0, &lib);
        }
        s
    }

    #[test]
    fn box_positions_stay_in_the_box() {
        let def = system(vec![burst(200)], vec![op("Position Within Box Random", &[("min", Attr::Vec3([-10.0, -5.0, 0.0])), ("max", Attr::Vec3([10.0, 5.0, 2.0]))]), lifetime(5.0)], vec![], vec![]);
        let s = run(def, 1);
        assert_eq!(s.particles.len(), 200);
        for p in &s.particles {
            let l = p.pos - Vec3::new(100.0, 0.0, 0.0);
            assert!(l.x.abs() <= 10.0 && l.y.abs() <= 5.0 && (0.0..=2.0).contains(&l.z), "{l:?}");
        }
        assert!(s.unsupported.is_empty(), "{:?}", s.unsupported);
    }

    #[test]
    fn noise_emitter_rate_stays_in_range() {
        let def = system(vec![op("emit noise", &[("emission minimum", Attr::Float(30.0)), ("emission maximum", Attr::Float(60.0))])], vec![lifetime(100.0)], vec![], vec![]);
        let s = run(def, 60);
        // Two seconds at 30..60 per second.
        assert!((55..=125).contains(&s.particles.len()), "{}", s.particles.len());
    }

    #[test]
    fn spin_and_spin_down() {
        let def = system(
            vec![burst(1)],
            vec![lifetime(10.0)],
            vec![op("Rotation Spin Roll", &[("spin_rate_degrees", Attr::Float(90.0)), ("spin_rate_min", Attr::Float(0.0)), ("spin_stop_time", Attr::Float(1.0))])],
            vec![],
        );
        let s = run(def, 60);
        // Spins down from 90 deg/s to 0 over 1 s: about 45 degrees in all.
        let deg = s.particles[0].rotation.to_degrees();
        assert!((40.0..50.0).contains(&deg), "{deg}");
    }

    #[test]
    fn plane_constraint_keeps_particles_above() {
        let def = system(
            vec![burst(20)],
            vec![lifetime(10.0), op("Velocity Random", &[("speed_in_local_coordinate_system_min", Attr::Vec3([0.0, 0.0, -300.0])), ("speed_in_local_coordinate_system_max", Attr::Vec3([0.0, 0.0, -300.0]))])],
            vec![op("Movement Basic", &[])],
            vec![op("Prevent passing through a plane", &[("plane point", Attr::Vec3([0.0, 0.0, -10.0]))])],
        );
        let s = run(def, 30);
        assert!(s.particles.iter().all(|p| p.pos.z >= -10.0 - 1e-3), "{:?}", s.particles.iter().map(|p| p.pos.z).collect::<Vec<_>>());
    }

    #[test]
    fn cull_random_removes_about_its_share() {
        let def = system(vec![burst(400)], vec![lifetime(1.0)], vec![op("Cull Random", &[("Cull Percentage", Attr::Float(0.5)), ("Cull Start Time", Attr::Float(0.0)), ("Cull End Time", Attr::Float(0.2))])], vec![]);
        let s = run(def, 10);
        assert!((140..260).contains(&s.particles.len()), "{}", s.particles.len());
    }

    #[test]
    fn stop_emission_lets_particles_finish_without_end_cap() {
        let def = system(vec![op("emit_continuously", &[("emission_rate", Attr::Float(100.0))])], vec![lifetime(0.5)], vec![op("Lifespan Decay", &[])], vec![]);
        let lib = ParticleLibrary::default();
        let mut s = SystemInstance::new(def, 1);
        for _ in 0..10 {
            s.update(1.0 / 30.0, &lib);
        }
        assert!(!s.particles.is_empty() && !s.finished());
        s.stop_emission();
        let n = s.particles.len();
        s.update(1.0 / 30.0, &lib);
        assert!(s.particles.len() <= n, "no new particles");
        assert!(!s.end_cap);
        for _ in 0..30 {
            s.update(1.0 / 30.0, &lib);
        }
        assert!(s.particles.is_empty() && s.finished());
    }

    #[test]
    fn drag_effect_emits_only_while_its_control_point_moves() {
        // GLaDOS's drag dust: CP0 speed 1..35 u/s -> CP1.x -0.1..1, which scales the rate.
        let speed = op("Remap CP Speed to CP", &[("input minimum", Attr::Float(1.0)), ("input maximum", Attr::Float(35.0)), ("output minimum", Attr::Float(-0.1)), ("output control point", Attr::Int(1))]);
        let emit = op("emit_continuously", &[("emission_rate", Attr::Float(60.0)), ("emission count scale control point", Attr::Int(1))]);
        let def = system(vec![emit], vec![lifetime(100.0)], vec![speed], vec![]);
        let lib = ParticleLibrary::default();
        let mut s = SystemInstance::new(def, 1);
        s.cps[0].pos = Vec3::new(9000.0, 1200.0, 450.0);
        for _ in 0..60 {
            s.update(1.0 / 30.0, &lib);
        }
        assert!(s.particles.is_empty(), "still: {}", s.particles.len());
        for _ in 0..30 {
            s.cps[0].pos.z += 35.0 / 30.0;
            s.update(1.0 / 30.0, &lib);
        }
        assert!((40..=70).contains(&s.particles.len()), "moving: {}", s.particles.len());
    }

    #[test]
    fn visibility_proxy_fades_hidden_glows() {
        // electrical_arc_01_cp0_glow: a $ignorez glow, hidden by the world
        // only through its visibility proxy at control point 0.
        let proxy = op("render_animated_sprites", &[("Visibility Proxy Input Control Point Number", Attr::Int(0))]);
        let mut def = (*system(vec![burst(1)], vec![lifetime(10.0)], vec![], vec![])).clone();
        def.renderers = vec![proxy.clone()];
        let mut s = SystemInstance::new(Arc::new(def), 1);
        assert!(s.wants_visibility());
        s.cps[0].pos = Vec3::new(1.0, 2.0, 3.0);
        let mut seen = None;
        s.set_visibility(&mut |p, r| {
            seen = Some((p, r));
            0.0
        });
        assert_eq!(seen, Some((Vec3::new(1.0, 2.0, 3.0), 1.0)));
        assert_eq!(s.visibility_alpha(&proxy), 0.0);
        s.set_visibility(&mut |_, _| 1.0);
        assert_eq!(s.visibility_alpha(&proxy), 1.0);
        // Its own alpha range; renderers without a proxy are never faded.
        let ranged = op("render_animated_sprites", &[("Visibility Proxy Input Control Point Number", Attr::Int(0)), ("Visibility Alpha Scale minimum", Attr::Float(0.25))]);
        s.set_visibility(&mut |_, _| 0.0);
        assert_eq!(s.visibility_alpha(&ranged), 0.25);
        assert_eq!(s.visibility_alpha(&op("render_animated_sprites", &[])), 1.0);
        let plain = SystemInstance::new(system(vec![burst(1)], vec![], vec![], vec![]), 1);
        assert!(!plain.wants_visibility());
    }

    #[test]
    fn place_on_ground_drops_particles_within_reach() {
        // GLaDOS's head drag droplets: spawned at her head, landed on the pool.
        let ground = op("Position Modify Place On Ground", &[("max trace length", Attr::Float(512.0)), ("include water", Attr::Bool(true))]);
        let def = system(vec![burst(4)], vec![lifetime(10.0), ground], vec![], vec![]);
        let lib = ParticleLibrary::default();
        let mut s = SystemInstance::new(def.clone(), 1);
        s.cps[0].pos = Vec3::new(9000.0, 1200.0, 460.0);
        s.set_ground(&mut |_| Some(396.0));
        s.update(1.0 / 30.0, &lib);
        assert_eq!(s.particles.len(), 4);
        assert!(s.particles.iter().all(|p| (p.pos.z - 396.0).abs() < 1e-3), "{:?}", s.particles.iter().map(|p| p.pos.z).collect::<Vec<_>>());
        // Out of reach (or nothing below): left where they were made.
        let mut far = SystemInstance::new(def, 1);
        far.cps[0].pos = Vec3::new(9000.0, 1200.0, 2000.0);
        far.set_ground(&mut |_| Some(396.0));
        far.update(1.0 / 30.0, &lib);
        assert!(far.particles.iter().all(|p| (p.pos.z - 2000.0).abs() < 1e-3));
    }

    #[test]
    fn restart_after_duration_bursts_again() {
        let def = system(vec![burst(5)], vec![lifetime(100.0)], vec![op("Restart Effect after Duration", &[("Minimum Restart Time", Attr::Float(0.5)), ("Maximum Restart Time", Attr::Float(0.5))])], vec![]);
        let s = run(def, 50);
        // Bursts at 0, 0.5, 1.0, 1.5 s.
        assert!((15..=20).contains(&s.particles.len()), "{}", s.particles.len());
    }

    #[test]
    fn control_point_follows_the_player() {
        let def = system(vec![burst(1)], vec![lifetime(10.0)], vec![op("Set Control Point To Player", &[("Control Point Number", Attr::Int(2)), ("Control Point Offset", Attr::Vec3([0.0, 0.0, 10.0]))])], vec![]);
        let lib = ParticleLibrary::default();
        let mut s = SystemInstance::new(def, 1);
        s.set_player(Some(ControlPoint::new(Vec3::new(5.0, 6.0, 64.0), Mat3::IDENTITY)));
        s.update(1.0 / 30.0, &lib);
        assert_eq!(s.cps[2].pos, Vec3::new(5.0, 6.0, 74.0));
    }

    #[test]
    fn noise_is_bounded() {
        for i in 0..1000 {
            let p = Vec3::new(i as f32 * 0.37, i as f32 * -0.11, i as f32 * 0.05);
            let n = noise3(p);
            assert!((-1.0..=1.0).contains(&n));
        }
    }
}

