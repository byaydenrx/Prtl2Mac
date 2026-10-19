//! Print a model's sequences and sample its bones, to check animation decoding.
//!
//! cargo run -p p2_assets --example animinfo -- models/props/ghostanim.mdl [sequence] [attachment]
use std::sync::Arc;

use p2_assets::anim::Animator;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let fs = source_assets::GameFs::from_env().expect("game");
    let model = &argv[0];
    let mut a = Animator::load(&|p| fs.read(p), model).expect("parse");
    println!("{} bones, {} sequences, ani {:?}", a.skeleton.bones.len(), a.sequences.len(), a.anim_block_file);
    if let Some(f) = a.anim_block_file.clone() {
        match fs.read(&f) {
            Some(d) => {
                println!("ani {} bytes", d.len());
                a.set_anim_blocks(Arc::new(d));
            }
            None => println!("ani file missing"),
        }
    }
    for (i, b) in a.skeleton.bones.iter().enumerate() {
        println!("  bone {i} {} parent {} pos {:?} quat {:?}", b.name, b.parent, b.pos, b.quat);
    }
    for (i, s) in a.sequences.iter().enumerate().take(40) {
        println!("  seq {i} {} [{}] duration {:.2}", s.label, s.activity, a.duration(i));
    }
    for (i, p) in a.pose_params.iter().enumerate() {
        println!("  pose param {i} {} {}..{}", p.name, p.start, p.end);
    }
    // Aim matrices and anything auto-layering one (scene lookat aiming).
    for (i, s) in a.sequences.iter().enumerate() {
        let grid = a.grid_params(i);
        if s.label.to_ascii_lowercase().contains("aimmatrix") || !s.autolayers.is_empty() {
            println!("  seq {i} {} delta {} post {} grid params {grid:?} autolayers {:?}", s.label, s.delta, s.post, s.autolayers);
        }
    }
    if let (Ok(bn), Some(sq)) = (std::env::var("EVALTEST"), argv.get(1).and_then(|n| a.find(n))) {
        // One bone over a sequence: sampled (model space) and its local track.
        let b = a.skeleton.bones.iter().position(|x| x.name.eq_ignore_ascii_case(&bn)).unwrap_or(0);
        for k in 0..=4 {
            let t = a.duration(sq) * k as f32 / 4.0;
            let l = a.sample(sq, t);
            let m = a.bone_to_model(&l);
            println!("t {t:.2} model pos {:.2?} X {:.2?} Y {:.2?} | local {:.3?}", m[b].translation, m[b].matrix3 * bevy::math::Vec3A::X, m[b].matrix3 * bevy::math::Vec3A::Y, l[b]);
        }
    }
    if std::env::var_os("AIMTEST").is_some() {
        // Where the eye light (bone eyelight_aimjoint, +X) points, in the root bone's frame,
        // for aim_yaw/aim_pitch at -1/0/+1 over sphere_idle_neutral.
        let base = a.find("sphere_idle_neutral").unwrap_or(0);
        let layer = a.find("sphere_aimmatrix_neutral");
        let eye = a.skeleton.bones.iter().position(|b| b.name == "eyelight_aimjoint").unwrap_or(0);
        for pose in [[0.0f32, 0.0], [1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
            let extra: Vec<(usize, f32, f32)> = layer.into_iter().map(|l| (l, 0.0, 1.0)).collect();
            let local = a.evaluate(base, 0.0, &pose, &extra);
            let m = a.bone_to_model(&local);
            let root = m[0].matrix3;
            let f = root.inverse() * (m[eye].matrix3 * bevy::math::Vec3A::X);
            println!("aim {pose:?}: eye dir in root frame {:.2?} (yaw {:.1} pitch {:.1})", f, f.y.atan2(f.x).to_degrees(), f.z.atan2((f.x * f.x + f.y * f.y).sqrt()).to_degrees());
        }
    }
    if let Some(i) = a.find_activity("ACT_IDLE") {
        println!("ACT_IDLE = seq {i} {}", a.sequences[i].label);
    }
    if let Some(b) = std::env::var("BONE").ok().and_then(|n| a.skeleton.bones.iter().position(|b| b.name.eq_ignore_ascii_case(&n))) {
        for seq in [argv.get(1).and_then(|n| a.find(n)).unwrap_or(0)] {
            let local = a.sample(seq, 0.0);
            let m = a.bone_to_model(&local);
            let skin = m[b] * a.skeleton.bones[b].pose_to_bone;
            let refp = a.skeleton.reference_pose()[b];
            println!("disc centre (10,0,0) -> {:.2?} normal X -> {:.2?}; reference pose maps it to {:.2?}", skin.transform_point3(bevy::math::Vec3::X * 10.0), skin.transform_vector3(bevy::math::Vec3::X), refp.transform_point3(bevy::math::Vec3::X * 10.0));
            let sk = a.skeleton.bone_to_model(Some(0));
            println!("source_assets seq0: disc -> {:.2?}, root +X {:.2?}", (sk[b] * a.skeleton.bones[b].pose_to_bone).transform_point3(bevy::math::Vec3::X * 10.0), sk[0].matrix3 * bevy::math::Vec3A::X);
            println!("bone {b} local {:?} model +X {:.2?} +Y {:.2?} +Z {:.2?} at {:.2?}", local[b], m[b].matrix3 * bevy::math::Vec3A::X, m[b].matrix3 * bevy::math::Vec3A::Y, m[b].matrix3 * bevy::math::Vec3A::Z, m[b].translation);
        }
    }
    let Some(seq) = argv.get(1).and_then(|n| a.find(n)) else { return };
    let att = argv.get(2).cloned();
    println!("{}", a.describe(seq));
    let dur = a.duration(seq);
    let steps: Vec<f32> = if std::env::var_os("FINE").is_some() { (0..60).map(|k| k as f32 / 28.0).collect() } else { (0..=8).map(|k| dur * k as f32 / 8.0).collect() };
    for t in steps {
        let local = a.sample(seq, t);
        let mut to_model: Vec<bevy::math::Affine3A> = Vec::new();
        for (i, (p, q)) in local.iter().enumerate() {
            let m = bevy::math::Affine3A::from_rotation_translation(*q, *p);
            let parent = a.skeleton.bones[i].parent;
            to_model.push(if parent >= 0 && (parent as usize) < i { to_model[parent as usize] * m } else { m });
        }
        let root = local.first().map(|x| x.0);
        let attm = att.as_ref().and_then(|n| a.attachment(n, &to_model));
        let attp = attm.map(|x| x.0);
        // The attachment's axes when one is named, else bone 1's rotation.
        let q = match attm {
            Some((_, m)) => bevy::math::Quat::from_mat3(&m),
            None => local.get(1).map(|x| x.1).unwrap_or_default(),
        };
        println!("t {t:.2}: pos {attp:?} +X {:.2?} +Y {:.2?} +Z {:.2?}", q * bevy::math::Vec3::X, q * bevy::math::Vec3::Y, q * bevy::math::Vec3::Z);
        let _ = root;
    }
}
