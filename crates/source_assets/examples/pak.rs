//! List (or print one of) the files packed inside a map.
//!
//! cargo run -p source_assets --example pak -- sp_a1_intro1 [path]
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let fs = GameFs::from_env()?;
    let data = fs.read(&format!("maps/{}.bsp", argv[0])).ok_or_else(|| anyhow::anyhow!("no map"))?;
    let bsp = vbsp::Bsp::read(&data)?;
    let pak = source_assets::fs::PakFile::from_vbsp(&bsp.pack);
    match argv.get(1) {
        Some(p) => {
            use std::io::Write;
            std::io::stdout().write_all(pak.get(p).unwrap_or_default())?;
        }
        None => {
            for n in pak.names() {
                println!("{n}");
            }
        }
    }
    Ok(())
}
