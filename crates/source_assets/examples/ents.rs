//! Entity classes in a map, and the outputs of one class.
//!
//! cargo run -p source_assets --example ents -- sp_a1_intro2 [classname]
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let name = argv.first().cloned().unwrap_or_else(|| "sp_a1_intro2".into());
    let fs = GameFs::from_env()?;
    let data = fs.read(&format!("maps/{name}.bsp")).ok_or_else(|| anyhow::anyhow!("no map"))?;
    let bsp = vbsp::Bsp::read(&data)?;
    let ents = source_assets::entities::parse_entities(&bsp.entities);
    let mut counts = std::collections::BTreeMap::new();
    for e in &ents {
        *counts.entry(e.classname.clone()).or_insert(0) += 1;
    }
    match argv.get(1) {
        None => {
            for (c, n) in counts {
                println!("{n:4} {c}");
            }
        }
        Some(class) => {
            for e in ents.iter().filter(|e| &e.classname == class || e.get("model") == Some(class.as_str()) || e.targetname.as_deref() == Some(class.as_str())) {
                println!("{} {:?} at {:?}", e.classname, e.targetname, e.origin);
                for (k, v) in &e.keyvalues {
                    println!("    {k} = {}", v.replace('\u{1b}', " | "));
                }
            }
        }
    }
    Ok(())
}
