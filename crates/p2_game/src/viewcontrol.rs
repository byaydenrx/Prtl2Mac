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
            .add_systems(Update, (track_animations, viewcontrol_inputs).chain().after(crate::io::run_io).in_set(crate::level::GameSet::Logic))
            .add_systems(PostUpdate, drive_camera.after(P2Systems::View).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Which sequence each prop is playing and since when (game seconds).
#[derive(Resource, Default)]
pub struct PropAnims {
    playing: HashMap<usize, (String, f32)>,
    /// Attachment each entity is parented to: child -> (parent ent, attachment).
    attached: HashMap<usize, (usize, String)>,
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

    /// World pose of `attachment` on prop `ent` now: position and the
    /// attachment's forward/up directions.
    pub fn attachment_world(&mut self, game: &GameFsRes, ents: &Ents, ent: usize, attachment: &str) -> Option<(Vec3, Vec3, Vec3)> {
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
        // Model space is Source's (Z up, inches); the prop's world rotation
        // is the Source rotation seen through the same basis change.
        let r = e.transform.rotation;
        let world = |v: Vec3| r * (source_to_world(v) / shared::SOURCE_UNIT);
        let p = e.transform.translation + r * source_to_world(pos);
        // Animations the decoder can't read yet come out as nonsense:
        // only trust poses near the prop.
        if !p.is_finite() || p.distance(e.transform.translation) > 20.0 {
            return None;
        }
        Some((p, world(rot * Vec3::X), world(rot * Vec3::Z)))
    }
}

fn track_animations(mut inputs: MessageReader<EntInput>, ents: Option<Res<Ents>>, mut anims: ResMut<PropAnims>, mut loaded: MessageReader<LevelLoaded>) {
    if loaded.read().count() > 0 {
        anims.playing.clear();
        anims.attached.clear();
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
                    anims.attached.insert(msg.target, (p, msg.param.trim().to_string()));
                }
            }
            "setparent" => {
                anims.attached.remove(&msg.target);
            }
            _ => {}
        }
    }
}

/// The cinematic cameras switched on, most recent last.
#[derive(Resource, Default)]
pub struct ViewControl {
    active: Vec<usize>,
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
        // Parented to an animation that can't be played yet: no camera.
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
