//! Print a particle system's definition (and its children's): material,
//! counts, every emitter/initializer/operator/renderer/force/constraint with
//! its parameters.
//!
//! ```text
//! PORTAL2_DIR=... cargo run --release -p p2_fx --example pcfdump -- particles/scene_fx.pcf glados_head_drag
//! ```
use p2_fx::pcf::{OpDef, ParticleLibrary};
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let (Some(file), Some(name)) = (argv.first(), argv.get(1)) else {
        anyhow::bail!("usage: pcfdump <file.pcf> <system>");
    };
    let fs = GameFs::from_env()?;
    let data = fs.read(file).ok_or_else(|| anyhow::anyhow!("{file} missing"))?;
    let mut lib = ParticleLibrary::default();
    lib.load_pcf(&data)?;
    dump(&lib, name, 0);
    Ok(())
}

fn dump(lib: &ParticleLibrary, name: &str, depth: usize) {
    let pad = "  ".repeat(depth);
    let Some(s) = lib.get(name) else {
        println!("{pad}{name}: not in this file");
        return;
    };
    println!(
        "{pad}== {} material {} max {} initial {} radius {} color {:?}",
        s.name, s.material, s.max_particles, s.initial_particles, s.radius, s.color
    );
    let ops = |kind: &str, list: &[OpDef]| {
        for o in list {
            let mut params: Vec<String> = o.params.iter().map(|(k, v)| format!("{k}={v:?}")).collect();
            params.sort();
            println!("{pad}  {kind} {}: {}", o.function, params.join(", "));
        }
    };
    ops("emitter", &s.emitters);
    ops("init", &s.initializers);
    ops("op", &s.operators);
    ops("render", &s.renderers);
    ops("force", &s.forces);
    ops("constraint", &s.constraints);
    for c in &s.children {
        println!("{pad}  child {} delay {} end_cap {}", c.name, c.delay, c.end_cap);
        dump(lib, &c.name, depth + 1);
    }
}
