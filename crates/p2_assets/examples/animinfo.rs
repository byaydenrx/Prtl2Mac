//! Print a model's sequences and sample its bones, to check animation decoding.
//!
//! cargo run -p p2_assets --example animinfo -- models/props/ghostanim.mdl [sequence] [attachment]
use std::sync::Arc;

use p2_assets::anim::Animator;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let fs = source_assets::GameFs::from_env().expect("game");
    let model = &argv[0];
    let mut a = Animator::parse(Arc::new(fs.read(model).expect("model"))).expect("parse");
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
        println!("  seq {i} {} duration {:.2}", s.label, a.duration(i));
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
        let attp = att.as_ref().and_then(|n| a.attachment(n, &to_model)).map(|x| x.0);
        let q = local.get(1).map(|x| x.1).unwrap_or_default();
        println!("t {t:.2}: pos {attp:?} +X {:.2?} +Y {:.2?} +Z {:.2?}", q * bevy::math::Vec3::X, q * bevy::math::Vec3::Y, q * bevy::math::Vec3::Z);
        let _ = root;
    }
}
