//! Which BSP brushes contain a point, and their contents and models.
//!
//! cargo run -p source_assets --example probe -- sp_a1_intro1 x,y,z
//! (`x,y,z` is world metres, as the game crates use.)
use bevy::math::Vec3;
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let name = argv.first().cloned().unwrap_or_else(|| "sp_a1_intro1".into());
    let v: Vec<f32> = argv.get(1).map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let w = Vec3::new(v[0], v[1], v[2]);
    // world (Y-up metres) -> Source (Z-up units)
    let p = Vec3::new(w.x, -w.z, w.y) / shared::SOURCE_UNIT;
    println!("source point {p:?}");
    let fs = GameFs::from_env()?;
    let data = fs.read(&format!("maps/{name}.bsp")).ok_or_else(|| anyhow::anyhow!("no map"))?;
    let bsp = vbsp::Bsp::read(&data)?;
    let raw = source_assets::bsp_raw::RawBsp::new(&data)?;
    let leaves = source_assets::bsp_raw::parse_leaves(&raw);
    let owner = |bi: usize| -> Vec<usize> {
        let mut out = Vec::new();
        for (mi, m) in bsp.models.iter().enumerate() {
            let mut stack = vec![m.head_node];
            let mut seen = std::collections::HashSet::new();
            let mut found = false;
            while let Some(n) = stack.pop() {
                if n < 0 {
                    if let Some(l) = leaves.get((-1 - n) as usize) {
                        for k in 0..l.leaf_brush_count as usize {
                            if bsp.leaf_brushes.get(l.first_leaf_brush as usize + k).map(|b| b.brush as usize) == Some(bi) {
                                found = true;
                            }
                        }
                    }
                } else if seen.insert(n) {
                    if let Some(node) = bsp.nodes.get(n as usize) {
                        stack.extend_from_slice(&node.children);
                    }
                }
            }
            if found {
                out.push(mi);
            }
        }
        out
    };
    for (bi, b) in bsp.brushes.iter().enumerate() {
        let sides = &bsp.brush_sides[b.brush_side as usize..(b.brush_side + b.num_brush_sides) as usize];
        let inside = sides.iter().all(|s| {
            let pl = &bsp.planes[s.plane as usize];
            let n = Vec3::new(pl.normal.x, pl.normal.y, pl.normal.z);
            n.dot(p) - pl.dist <= 4.0
        });
        if inside {
            let bs = source_assets::collision::brush_sides(&bsp, bi, &|_| None);
            for s in &bs {
                println!("   side n {:?} d {} polygon {}", s.normal, s.dist, s.polygon.len());
            }
            for s in sides {
                let pl = &bsp.planes[s.plane as usize];
                println!("   raw plane {:?} {} bevel {} texinfo {}", pl.normal, pl.dist, s.bevel, s.texture_info);
            }
            println!("brush {bi} contents {:#x} sides {} models {:?}", b.flags.bits(), sides.len(), owner(bi));
        }
    }
    Ok(())
}
