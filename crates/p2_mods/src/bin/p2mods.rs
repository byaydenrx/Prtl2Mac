//! `p2mods`: the mod manager from a terminal.
//!
//! ```text
//! p2mods [--root DIR] [--profile NAME] COMMAND
//!   list                       installed mods and the profile's load order
//!   install PATH [--enable]    a mod folder or .zip
//!   uninstall ID VERSION
//!   enable ID[@VERSION] | disable ID | remove ID | move ID POSITION
//!   upgrade                    point the profile at the newest versions
//!   check                      dependency / conflict / overlap report
//!   plan                       the folders the game will mount, in order
//!   profiles | profile create NAME [--from P] | profile use NAME | profile delete NAME
//!   snapshots | rollback [NUMBER]
//!   clean                      delete versions nothing refers to
//!   pack FOLDER OUT.zip        zip a mod folder for sharing
//! ```
//!
//! The root defaults to the game's: `$P2_DATA_DIR/mods`, else
//! `~/Library/Application Support/Portal2Rust/mods` on macOS
//! (`~/.local/share/portal2rust/mods` elsewhere).
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use p2_mods::{ModManager, Severity};
use semver::Version;

fn default_root() -> PathBuf {
    if let Some(d) = std::env::var_os("P2_DATA_DIR") {
        return PathBuf::from(d).join("mods");
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Portal2Rust/mods")
    } else {
        home.join(".local/share/portal2rust/mods")
    }
}

const ENGINE: &str = "0.1.0";

fn usage() -> ExitCode {
    eprintln!("{}", include_str!("p2mods.rs").lines().skip(2).take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").strip_prefix(' ').unwrap_or("")).filter(|l| !l.starts_with("```")).collect::<Vec<_>>().join("\n"));
    ExitCode::from(2)
}

fn pack(folder: &Path, out: &Path) -> Result<usize, p2_mods::Error> {
    let m = p2_mods::Manifest::read(folder)?;
    let mut w = p2_mods::archive::ZipWriter::create(out)?;
    let mut n = 0;
    let mut stack = vec![folder.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&d)?.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(folder).unwrap().to_string_lossy().replace('\\', "/");
                w.add(&format!("{}/{rel}", m.id), &std::fs::read(&p)?, true)?;
                n += 1;
            }
        }
    }
    w.finish()?;
    Ok(n)
}

fn run(args: &[String]) -> Result<bool, p2_mods::Error> {
    let mut root = default_root();
    let mut profile: Option<String> = None;
    let mut rest: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--root" => root = it.next().map(PathBuf::from).unwrap_or(root),
            "--profile" => profile = it.next().cloned(),
            _ => rest.push(a),
        }
    }
    let m = ModManager::open(&root)?;
    let profile = profile.unwrap_or_else(|| m.active_profile());
    let engine = Version::parse(ENGINE).ok();
    let id_version = |s: &str| -> Result<(String, Option<Version>), p2_mods::Error> {
        match s.split_once('@') {
            Some((id, v)) => Ok((id.into(), Some(Version::parse(v).map_err(|e| p2_mods::Error::Invalid(format!("{v}: {e}")))?))),
            None => Ok((s.into(), None)),
        }
    };
    let bad = || p2_mods::Error::Invalid("missing arguments (run with no arguments for help)".into());
    match rest.as_slice() {
        ["list"] => {
            println!("store: {}", root.display());
            for i in m.installed() {
                println!("  {} {}  {}", i.manifest.id, i.manifest.version, i.manifest.name);
            }
            println!("profile {profile} (load order, lowest priority first):");
            for (n, e) in m.load_profile(&profile)?.mods.iter().enumerate() {
                println!("  {n}. [{}] {} {}", if e.enabled { "x" } else { " " }, e.id, e.version);
            }
        }
        ["install", path, flags @ ..] => {
            let i = m.install(Path::new(path))?;
            println!("installed {} {} ({})", i.manifest.id, i.manifest.version, i.dir.display());
            if flags.contains(&"--enable") {
                m.enable(&profile, &i.manifest.id, Some(&i.manifest.version))?;
                println!("enabled in {profile}");
            }
        }
        ["uninstall", id, v] => {
            let v = Version::parse(v).map_err(|e| p2_mods::Error::Invalid(e.to_string()))?;
            m.uninstall(id, &v)?;
            println!("removed {id} {v}");
        }
        ["enable", spec] => {
            let (id, v) = id_version(spec)?;
            let v = m.enable(&profile, &id, v.as_ref())?;
            println!("enabled {id} {v} in {profile}");
        }
        ["disable", id] => m.disable(&profile, id)?,
        ["remove", id] => m.remove_from_profile(&profile, id)?,
        ["move", id, pos] => m.move_mod(&profile, id, pos.parse().map_err(|_| p2_mods::Error::Invalid(format!("bad position {pos}")))?)?,
        ["upgrade"] => {
            for (id, v) in m.upgrade_all(&profile)? {
                println!("{id} -> {v}");
            }
        }
        ["check"] | ["plan"] => {
            let plan = m.mount_plan(engine.as_ref());
            let (r, _) = m.check(&profile, engine.as_ref())?;
            for i in &r.issues {
                println!("{i}");
            }
            if rest[0] == "plan" {
                println!("mount order (highest priority first), then the game:");
                for d in &plan.dirs {
                    println!("  {}", d.display());
                }
            } else {
                println!("loads: {}", if r.order.is_empty() { "nothing".into() } else { r.order.join(", ") });
                if !r.skipped.is_empty() {
                    println!("left out: {}", r.skipped.join(", "));
                }
            }
            return Ok(!r.issues.iter().any(|i| i.severity == Severity::Error));
        }
        ["profiles"] => {
            let active = m.active_profile();
            for p in m.profiles() {
                println!("{} {p}", if p == active { "*" } else { " " });
            }
        }
        ["profile", "create", name, more @ ..] => {
            let from = match more {
                ["--from", f] => Some(*f),
                [] => None,
                _ => return Err(bad()),
            };
            m.create_profile(name, from)?;
        }
        ["profile", "use", name] => m.set_active_profile(name)?,
        ["profile", "delete", name] => m.delete_profile(name)?,
        ["snapshots"] => {
            for s in m.snapshots(&profile) {
                println!("{:>4}  before: {}  ({} enabled)", s.number, s.label, s.enabled);
            }
        }
        ["rollback", more @ ..] => {
            let to = match more {
                [n] => Some(n.parse().map_err(|_| p2_mods::Error::Invalid(format!("bad snapshot {n}")))?),
                _ => None,
            };
            match m.rollback(&profile, to)? {
                Some(label) => println!("undid: {label}"),
                None => println!("nothing to roll back"),
            }
        }
        ["clean"] => {
            for (id, v) in m.clean()? {
                println!("deleted {id} {v}");
            }
        }
        ["pack", folder, out] => {
            let n = pack(Path::new(folder), Path::new(out))?;
            println!("packed {n} files into {out}");
        }
        _ => return Err(bad()),
    }
    Ok(true)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        return usage();
    }
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("p2mods: {e}");
            ExitCode::FAILURE
        }
    }
}
