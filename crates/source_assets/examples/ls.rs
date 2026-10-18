//! List files under a prefix in the mounted game, or print one text file.
//!
//! cargo run -p source_assets --example ls -- sound/music [--cat path]
use source_assets::GameFs;

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let fs = GameFs::from_env()?;
    if argv.first().map(String::as_str) == Some("--cat") {
        let d = fs.read(&argv[1]).ok_or_else(|| anyhow::anyhow!("missing"))?;
        use std::io::Write;
        std::io::stdout().write_all(&d)?;
        return Ok(());
    }
    if argv.first().map(String::as_str) == Some("--vtf") {
        // --vtf path out.ppm: level 0 as an RGB image, alpha beside it.
        let d = fs.read(&argv[1]).ok_or_else(|| anyhow::anyhow!("missing"))?;
        let t = source_assets::vtf::VtfTexture::parse(&d)?;
        let rgba = t.rgba8_level0().ok_or_else(|| anyhow::anyhow!("undecodable"))?;
        let (w, h) = (t.width as usize, t.height as usize);
        let mut out = format!("P6\n{} {}\n255\n", w * 2, h).into_bytes();
        for y in 0..h {
            for x in 0..w {
                out.extend_from_slice(&rgba[(y * w + x) * 4..][..3]);
            }
            for x in 0..w {
                out.extend_from_slice(&[rgba[(y * w + x) * 4 + 3]; 3]);
            }
        }
        std::fs::write(&argv[2], out)?;
        println!("{w}x{h} {:?} has_alpha {} avg {:?}", t.format, t.has_alpha, t.average_color());
        return Ok(());
    }
    if argv.first().map(String::as_str) == Some("--fmt") {
        let mut counts = std::collections::BTreeMap::new();
        for p in fs.list(&argv[1]) {
            let Some(b) = fs.read(&p) else { continue };
            let tag = if b.len() > 22 && &b[0..4] == b"RIFF" {
                let mut i = 12;
                let mut t = 0u16;
                while i + 8 <= b.len() {
                    let size = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
                    if &b[i..i + 4] == b"fmt " {
                        t = u16::from_le_bytes([b[i + 8], b[i + 9]]);
                        break;
                    }
                    i += 8 + size + (size & 1);
                }
                format!("wav fmt {t:#x}")
            } else {
                p.rsplit('.').next().unwrap_or("").to_string()
            };
            *counts.entry(tag).or_insert(0) += 1;
        }
        println!("{counts:?}");
        return Ok(());
    }
    for p in fs.list(argv.first().map(String::as_str).unwrap_or("")) {
        println!("{p}");
    }
    Ok(())
}
