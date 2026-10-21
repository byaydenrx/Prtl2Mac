//! Which particle operators the game's effects need that p2_fx doesn't
//! simulate. Reads `<map> <system>` lines on stdin (e.g. from the `ents`
//! example's info_particle_system effect names) and prints, per unsupported
//! operator, how many systems and maps use it, plus systems not found.
//!
//! ```text
//! PORTAL2_DIR=... cargo run --release -p p2_fx --example fxaudit < map_systems.txt
//! ```
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;

use p2_fx::pcf::ParticleLibrary;
use p2_fx::sim::SystemInstance;
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let fs = GameFs::from_env()?;
    let manifest = String::from_utf8_lossy(&fs.read("particles/particles_manifest.txt").unwrap_or_default()).to_string();
    let mut base = ParticleLibrary::default();
    base.load_manifest(&manifest, &|f| fs.read(f));
    let mut libs: BTreeMap<String, ParticleLibrary> = BTreeMap::new();
    let mut uses: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    let mut missing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let mut it = line.split_whitespace();
        let (Some(map), Some(sys)) = (it.next(), it.next()) else { continue };
        let lib = libs.entry(map.to_string()).or_insert_with(|| {
            let mut lib = base.clone();
            if let Some(t) = fs.read(&format!("maps/{map}_particles.txt")) {
                lib.load_manifest(&String::from_utf8_lossy(&t), &|f| fs.read(f));
            }
            lib
        });
        let Some(def) = lib.get(sys) else {
            missing.entry(sys.to_string()).or_default().insert(map.to_string());
            continue;
        };
        let mut todo = vec![def];
        let mut seen = BTreeSet::new();
        while let Some(d) = todo.pop() {
            if !seen.insert(d.name.clone()) {
                continue;
            }
            for op in SystemInstance::new(d.clone(), 1).unsupported {
                let e = uses.entry(op).or_default();
                e.0.insert(d.name.clone());
                e.1.insert(map.to_string());
            }
            todo.extend(d.children.iter().filter_map(|c| lib.get(&c.name)));
        }
    }
    let mut rows: Vec<_> = uses.into_iter().collect();
    rows.sort_by_key(|(_, (s, m))| std::cmp::Reverse((m.len(), s.len())));
    println!("maps systems  unsupported operator (example systems)");
    for (op, (systems, maps)) in rows {
        let ex: Vec<&String> = systems.iter().take(3).collect();
        println!("{:4} {:7}  {op}  {ex:?}", maps.len(), systems.len());
    }
    println!("\nnot found: {}", missing.len());
    for (s, maps) in missing.iter().take(30) {
        println!("  {s} ({} maps, e.g. {:?})", maps.len(), maps.iter().next());
    }
    Ok(())
}
