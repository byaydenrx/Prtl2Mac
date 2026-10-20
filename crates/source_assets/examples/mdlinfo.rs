//! Meshes of a model: material per skin, triangle count, bounds, main bones,
//! and the bounds of each `.phy` collision piece (Source units).
//!
//! cargo run -p source_assets --example mdlinfo -- models/npcs/personality_sphere/personality_sphere.mdl [skin]
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let path = argv.first().cloned().unwrap_or_default();
    let skin: i32 = argv.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let fs = GameFs::from_env()?;
    let m = source_assets::models::load_model(&path, &|p| fs.read(p), &|_| false)?;
    let sk = m.skeleton.as_ref();
    println!("{} meshes, skins {}, animated {:?}, contents {:#x}", m.meshes.len(), m.skins.len(), sk.map(|s| s.sequences.len()), m.contents);
    for (i, mesh) in m.meshes.iter().enumerate() {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &mesh.source_positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let mut bones = std::collections::BTreeMap::new();
        for w in &mesh.bone_weights {
            let b = w.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0).unwrap_or(0);
            *bones.entry(b).or_insert(0) += 1;
        }
        // Soft-skinned vertices (more than one bone with weight) and
        // triangles whose vertices follow different bones.
        let soft = mesh.bone_weights.iter().filter(|w| w.iter().filter(|x| x.1 > 0.01).count() > 1).count();
        let dom = |v: u32| mesh.bone_weights.get(v as usize).and_then(|w| w.iter().max_by(|a, b| a.1.total_cmp(&b.1))).map(|x| x.0);
        let torn = mesh.indices.chunks_exact(3).filter(|t| dom(t[0]) != dom(t[1]) || dom(t[0]) != dom(t[2])).count();
        let names: Vec<String> = bones.iter().take(4).map(|(b, n)| format!("{}x{}", sk.and_then(|s| s.bones.get(*b as usize)).map(|b| b.name.as_str()).unwrap_or("?"), n)).collect();
        println!(
            "mesh {i}: mat {:?} tris {} eyeball {:?} lo {:.1?} hi {:.1?} bones {:?} soft verts {soft}/{} mixed tris {torn}",
            m.material_for(skin, mesh.material_index),
            mesh.indices.len() / 3,
            mesh.eyeball,
            lo,
            hi,
            names,
            mesh.bone_weights.len()
        );
    }
    for (i, piece) in m.physics.iter().enumerate() {
        // Pieces are in world axes and metres: back to Source axes and units.
        let src: Vec<_> = piece.iter().map(|p| shared::world_to_source(*p)).collect();
        let lo = src.iter().fold(src[0], |a, b| a.min(*b));
        let hi = src.iter().fold(src[0], |a, b| a.max(*b));
        println!("phy piece {i}: {} points lo {:.1?} hi {:.1?}", piece.len(), lo, hi);
    }
    Ok(())
}
