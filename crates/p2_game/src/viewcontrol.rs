//! Cinematic cameras (`point_viewcontrol`) and the prop animations they
//! ride on. The intro's wake-up look-around is a camera parented to an
//! attachment of an invisible animated model (`ghostanim` playing
//! `sp_a1_intro`); this samples that model's animation from the install
//! and puts the player's view there until the map lets go.
use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use p2_assets::anim::Animator;
use p2_assets::GameFsRes;
use p2_core::*;
use shared::source_to_world;

use crate::io::{EntInput, Ents, PLAYER};
use crate::level::LevelLoaded;

pub struct ViewControlPlugin;

impl Plugin for ViewControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PropAnims>()
            .init_resource::<ViewControl>()
            .init_resource::<ViewProxy>()
            .add_systems(Update, (track_animations, follow_attachments, animation_done, viewcontrol_inputs, viewproxy_inputs).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            .add_systems(PostUpdate, (drive_camera, apply_viewproxy).chain().after(P2Systems::View).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Which sequence each prop is playing and since when (game seconds).
#[derive(Resource, Default)]
pub struct PropAnims {
    playing: HashMap<usize, (String, f32)>,
    /// Attachment each entity is parented to: child -> (parent ent, attachment).
    attached: HashMap<usize, (usize, String)>,
    /// Each attached child's pose relative to its attachment.
    offsets: HashMap<usize, Transform>,
    animators: HashMap<String, Option<Arc<Animator>>>,
}

impl PropAnims {
    fn animator(&mut self, game: &GameFsRes, model: &str) -> Option<Arc<Animator>> {
        let key = model.to_ascii_lowercase().replace('\\', "/");
        self.animators
            .entry(key.clone())
            .or_insert_with(|| {
                let mut a = Animator::parse(Arc::new(game.read(&key)?))?;
                if let Some(ani) = a.anim_block_file.clone() {
                    if let Some(d) = game.read(&ani) {
                        a.set_anim_blocks(Arc::new(d));
                    }
                }
                Some(Arc::new(a))
            })
            .clone()
    }

    /// World transform of `attachment` on prop `ent` now (same rotation
    /// convention as map entity transforms: Source axes seen through the
    /// world basis change).
    pub fn attachment_transform(&mut self, game: &GameFsRes, ents: &Ents, ent: usize, attachment: &str) -> Option<Transform> {
        let e = &ents.list[ent];
        let model = e.get("model")?.to_string();
        let anim = self.animator(game, &model)?;
        let (seq, cycle) = match self.playing.get(&ent) {
            Some((name, start)) => {
                let s = anim.find(name)?;
                (s, (ents.now - start).min(anim.duration(s)))
            }
            None => (e.get("defaultanim").and_then(|n| anim.find(n)).unwrap_or(0), 0.0),
        };
        let local = anim.sample(seq, cycle);
        // Bone -> model, through the parents.
        let mut to_model: Vec<bevy::math::Affine3A> = Vec::with_capacity(local.len());
        for (i, (p, q)) in local.iter().enumerate() {
            let m = bevy::math::Affine3A::from_rotation_translation(*q, *p);
            let parent = anim.skeleton.bones.get(i).map(|b| b.parent).unwrap_or(-1);
            let m = if parent >= 0 && (parent as usize) < i { to_model[parent as usize] * m } else { m };
            to_model.push(m);
        }
        let (pos, rot) = anim.attachment(attachment, &to_model)?;
        let basis = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let r = e.transform.rotation;
        let p = e.transform.translation + r * source_to_world(pos);
        // Animations the decoder can't read come out as nonsense.
        if !p.is_finite() || p.distance(e.transform.translation) > 2000.0 {
            return None;
        }
        let q = r * basis * Quat::from_mat3(&rot) * basis.inverse();
        Some(Transform::from_translation(p).with_rotation(q.normalize()))
    }

    /// Camera pose from an attachment: position, forward, up. Camera rigs
    /// (the intro's ghostanim) look down the bone's -X with +Y up.
    pub fn attachment_world(&mut self, game: &GameFsRes, ents: &Ents, ent: usize, attachment: &str) -> Option<(Vec3, Vec3, Vec3)> {
        let t = self.attachment_transform(game, ents, ent, attachment)?;
        if t.translation.distance(ents.list[ent].transform.translation) > 20.0 {
            return None;
        }
        Some((t.translation, t.rotation * Vec3::NEG_X, t.rotation * Vec3::NEG_Z))
    }
}

fn track_animations(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        anims.playing.clear();
        anims.attached.clear();
        anims.offsets.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        match msg.input.as_str() {
            "setanimation" | "setanimationnoreset" => {
                anims.playing.insert(msg.target, (msg.param.trim().to_string(), ents.now));
            }
            "setparentattachment" | "setparentattachmentmaintainoffset" => {
                let parent = ents.list[msg.target].get("parentname").and_then(|p| ents.find(p, msg.target, None).first().copied());
                if let Some(p) = parent {
                    let att = msg.param.trim().to_string();
                    // Keep the child where it is relative to the attachment
                    // (or snap onto it).
                    let offset = if msg.input == "setparentattachmentmaintainoffset" {
                        anims
                            .attachment_transform(&game, &ents, p, &att)
                            .map(|a| Transform::from_matrix(a.to_matrix().inverse() * ents.list[msg.target].transform.to_matrix()))
                            .unwrap_or_default()
                    } else {
                        Transform::IDENTITY
                    };
                    anims.attached.insert(msg.target, (p, att));
                    anims.offsets.insert(msg.target, offset);
                }
            }
            "setparent" | "clearparent" => {
                anims.attached.remove(&msg.target);
                anims.offsets.remove(&msg.target);
            }
            _ => {}
        }
    }
}

/// Props whose animation finished fire `OnAnimationDone` once.
fn animation_done(ents: Option<Res<Ents>>, game: Res<GameFsRes>, mut anims: ResMut<PropAnims>, mut fire: MessageWriter<crate::io::FireOutput>, mut done: Local<std::collections::HashSet<(usize, u32)>>) {
    let Some(ents) = ents else { return };
    let playing: Vec<(usize, String, f32)> = anims.playing.iter().map(|(k, (s, t))| (*k, s.clone(), *t)).collect();
    for (i, seq, start) in playing {
        let key = (i, start.to_bits());
        if done.contains(&key) {
            continue;
        }
        let Some(model) = ents.list[i].get("model").map(str::to_string) else { continue };
        let Some(anim) = anims.animator(&game, &model) else { continue };
        let Some(s) = anim.find(&seq) else { continue };
        if ents.now - start >= anim.duration(s) {
            done.insert(key);
            fire.write(crate::io::FireOutput { ent: i, event: "OnAnimationDone".into(), activator: None });
        }
    }
}

/// The cinematic cameras switched on, most recent last.
#[derive(Resource, Default)]
pub struct ViewControl {
    active: Vec<usize>,
    /// When each active camera lets go (hold time), game seconds.
    until: HashMap<usize, f32>,
    /// A camera is actually driving the view this frame.
    driving: bool,
}

impl ViewControl {
    pub fn is_active(&self) -> bool {
        self.driving
    }
}

#[allow(clippy::too_many_arguments)]
fn viewcontrol_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut vc: ResMut<ViewControl>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity), With<Player>>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        vc.active.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let now = ents.now;
    let expired: Vec<usize> = vc.until.iter().filter(|(_, t)| **t <= now).map(|(k, _)| *k).collect();
    for k in expired {
        vc.until.remove(&k);
        vc.active.retain(|a| *a != k);
    }
    for msg in inputs.read() {
        if ents.list[msg.target].class != "point_viewcontrol" {
            continue;
        }
        match msg.input.as_str() {
            "enable" => {
                // The cryo "blackout" eyelid camera needs the eyelid model's
                // animation and material; leave the player's own view.
                if ents.list[msg.target].name.contains("blackout") {
                    continue;
                }
                vc.active.retain(|a| *a != msg.target);
                vc.active.push(msg.target);
                // Hold time ("wait"), unless flagged to hold forever (8).
                let e = &ents.list[msg.target];
                let wait = e.get_f32("wait").unwrap_or(10.0);
                if e.spawnflags & 8 == 0 && wait > 0.0 {
                    vc.until.insert(msg.target, ents.now + wait);
                } else {
                    vc.until.remove(&msg.target);
                }
            }
            "disable" | "kill" => vc.active.retain(|a| *a != msg.target),
            "teleporttoview" | "teleportplayertoproxy" => {
                if let Some((pos, fwd, _)) = camera_pose(&mut anims, &game, &ents, msg.target) {
                    if let Ok((mut tf, mut feet, mut view, mut vel)) = players.single_mut() {
                        let at = pos - Vec3::Y * VIEW_STAND;
                        tf.translation = at;
                        *feet = InterpolatedFeet { prev: at, curr: at };
                        *view = ViewAngles { yaw: (-fwd.x).atan2(-fwd.z), pitch: fwd.y.clamp(-1.0, 1.0).asin(), roll: 0.0 };
                        vel.0 = Vec3::ZERO;
                    }
                }
            }
            _ => {}
        }
        let _ = PLAYER;
    }
}

/// Where a view controller looks from: its parent's animated attachment, or
/// its own place, facing its target when it has one.
fn camera_pose(anims: &mut PropAnims, game: &GameFsRes, ents: &Ents, i: usize) -> Option<(Vec3, Vec3, Vec3)> {
    if let Some((parent, att)) = anims.attached.get(&i).cloned() {
        // Cameras riding animated rigs (the intro's wake-up) aren't matched
        // to the game yet; P2_CINEMATIC=1 turns them on for testing.
        if std::env::var_os("P2_CINEMATIC").is_none() {
            return None;
        }
        return anims.attachment_world(game, ents, parent, &att);
    }
    let e = &ents.list[i];
    let pos = e.transform.translation;
    let fwd = e
        .get("target")
        .and_then(|t| ents.find(t, i, None).first().copied())
        .map(|t| (ents.list[t].transform.translation - pos).normalize_or(Vec3::NEG_Z))
        .unwrap_or(e.transform.rotation * Vec3::X);
    Some((pos, fwd, Vec3::Y))
}

/// While a view controller is on, the main camera is its camera and the
/// player can't move or look.
fn drive_camera(
    mut vc: ResMut<ViewControl>,
    ents: Option<Res<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut cams: Query<&mut Transform, With<MainCamera>>,
    mut locked: ResMut<InputLocked>,
    mut was_on: Local<bool>,
) {
    // Only a camera with a pose takes the view (and the controls).
    let pose = match (vc.active.last(), ents.as_ref()) {
        (Some(&i), Some(ents)) => camera_pose(&mut anims, &game, ents, i).map(|p| (i, p)),
        _ => None,
    };
    let on = pose.is_some();
    if vc.driving != on {
        vc.driving = on;
    }
    if on != *was_on {
        *was_on = on;
        if std::env::var_os("P2_SCRIPT").is_none() {
            locked.0 = on;
        }
    }
    let (Some((i, (pos, fwd, up))), Some(ents)) = (pose, ents) else { return };
    if std::env::var_os("P2_TRACE").is_some() {
        let src = pos / shared::SOURCE_UNIT;
        info!("TRACE viewcontrol {} at source ({:.0} {:.0} {:.0}) fwd {:.2?} up {:.2?} anim {:?}", ents.list[i].name, src.x, -src.z, src.y, fwd, up, anims.attached.get(&i));
    }
    for mut tf in &mut cams {
        tf.translation = pos;
        tf.look_to(fwd, up.normalize_or(Vec3::Y));
    }
}

/// Entities parented to an animated attachment keep their pose relative to
/// it every frame (the container ride's camera proxy, Wheatley's rail).
fn follow_attachments(
    mut ents: Option<ResMut<Ents>>,
    game: Res<GameFsRes>,
    mut anims: ResMut<PropAnims>,
    mut transforms: Query<&mut Transform, Without<Player>>,
) {
    let Some(ents) = ents.as_mut() else { return };
    let list: Vec<(usize, usize, String)> = anims.attached.iter().map(|(c, (p, a))| (*c, *p, a.clone())).collect();
    for (child, parent, att) in list {
        if ents.list[child].class == "point_viewcontrol" {
            continue;
        }
        let Some(offset) = anims.offsets.get(&child).copied() else { continue };
        let Some(a) = anims.attachment_transform(&game, ents, parent, &att) else { continue };
        let world = Transform::from_matrix(a.to_matrix() * offset.to_matrix());
        let delta = world.translation - ents.list[child].transform.translation;
        ents.list[child].transform = world;
        if let Some((lo, hi)) = &mut ents.list[child].bounds {
            *lo += delta;
            *hi += delta;
        }
        if let Some(r) = ents.list[child].entity {
            if let Ok(mut t) = transforms.get_mut(r) {
                t.translation = world.translation;
                t.rotation = world.rotation;
            }
        }
    }
}

/// An enabled `point_viewproxy`: the player's view is drawn as if their
/// place relative to this entity were relative to its `proxy` entity. The
/// container ride moves the proxy with the crane while the player stands in
/// a still copy of the vault.
#[derive(Resource, Default)]
pub struct ViewProxy {
    active: Option<usize>,
}

impl ViewProxy {
    /// The transform from the registration point to the proxy, now.
    fn mapping(&self, ents: &Ents) -> Option<Mat4> {
        let i = self.active?;
        let e = &ents.list[i];
        let proxy = e.get("proxy").and_then(|p| ents.find(p, i, None).first().copied())?;
        let from = Transform::from_translation(e.transform.translation).with_rotation(e.transform.rotation);
        let to = ents.list[proxy].transform;
        Some(to.to_matrix() * from.to_matrix().inverse())
    }
}

fn viewproxy_inputs(
    mut inputs: MessageReader<EntInput>,
    ents: Option<Res<Ents>>,
    mut proxy: ResMut<ViewProxy>,
    mut players: Query<(&mut Transform, &mut InterpolatedFeet, &mut ViewAngles, &mut CharacterVelocity), With<Player>>,
    mut loaded: MessageReader<LevelLoaded>,
) {
    if loaded.read().count() > 0 {
        proxy.active = None;
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        if ents.list[msg.target].class != "point_viewproxy" {
            continue;
        }
        match msg.input.as_str() {
            "enable" => proxy.active = Some(msg.target),
            "disable" if proxy.active == Some(msg.target) => proxy.active = None,
            "teleportplayertoproxy" => {
                let was = proxy.active;
                proxy.active = Some(msg.target);
                if let (Some(m), Ok((mut tf, mut feet, mut view, mut vel))) = (proxy.mapping(&ents), players.single_mut()) {
                    let at = m.transform_point3(tf.translation);
                    tf.translation = at;
                    *feet = InterpolatedFeet { prev: at, curr: at };
                    let fwd = m.transform_vector3(view.forward()).normalize_or(Vec3::NEG_Z);
                    view.yaw = (-fwd.x).atan2(-fwd.z);
                    vel.0 = Vec3::ZERO;
                }
                proxy.active = if was == Some(msg.target) { None } else { was };
            }
            _ => {}
        }
    }
}

/// Moves the main camera through the proxy mapping.
fn apply_viewproxy(proxy: Res<ViewProxy>, ents: Option<Res<Ents>>, mut cams: Query<&mut Transform, With<MainCamera>>) {
    let (Some(ents), true) = (ents, proxy.active.is_some()) else { return };
    let Some(m) = proxy.mapping(&ents) else { return };
    for mut tf in &mut cams {
        *tf = Transform::from_matrix(m * tf.to_matrix());
    }
}
