//! `mod.toml`: what a mod is, what it needs and what it can't run with.
//!
//! ```toml
//! id = "better-cubes"            # required: a-z 0-9 - _ . (1-64 chars)
//! name = "Better Cubes"          # required
//! version = "1.2.0"              # required, semver
//! description = "Shinier cubes"  # optional
//! authors = ["Someone"]          # optional
//! content = "content"            # optional: the folder to mount (default: the mod's own folder)
//! engine = "^0.1"                # optional: the game versions it was made for
//!
//! [dependencies]
//! core-textures = "^1.0"                               # needs it, at these versions
//! extra-sounds = { version = ">=2, <3", optional = true } # only checked when enabled
//!
//! [conflicts]
//! old-cubes = "*"                # can't be enabled together (any version)
//! ```
//!
//! `conflicts` may also be a plain list of ids (`conflicts = ["old-cubes"]`).
use semver::{Version, VersionReq};
use toml_edit::{DocumentMut, Item, Value};

use crate::Error;

/// The manifest file name, at the root of every mod.
pub const MANIFEST: &str = "mod.toml";

#[derive(Clone, Debug, PartialEq)]
pub struct Dependency {
    pub id: String,
    pub req: VersionReq,
    /// Checked only when the dependency is enabled; never required.
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub id: String,
    /// The versions of `id` this mod can't run with.
    pub req: VersionReq,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub authors: Vec<String>,
    /// Folder inside the mod to mount, relative ("." is the mod folder).
    pub content: String,
    /// Game versions the mod was made for (`None`: any).
    pub engine: Option<VersionReq>,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<Conflict>,
}

/// Mod ids: lowercase letters, digits, `-`, `_`, `.`; 1 to 64 characters,
/// not starting with `.` (they name folders in the store).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('.')
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

fn bad(msg: impl Into<String>) -> Error {
    Error::Manifest(msg.into())
}

fn req(s: &str, what: &str) -> Result<VersionReq, Error> {
    VersionReq::parse(s.trim()).map_err(|e| bad(format!("{what}: bad version range {s:?}: {e}")))
}

/// A string field, or an error naming it.
fn string(doc: &DocumentMut, key: &str) -> Result<Option<String>, Error> {
    match doc.get(key) {
        None => Ok(None),
        Some(i) => i.as_str().map(|s| Some(s.to_string())).ok_or_else(|| bad(format!("`{key}` must be a string"))),
    }
}

/// `"^1"` or `{ version = "^1", optional = true }`.
fn dependency(id: &str, item: &Item) -> Result<Dependency, Error> {
    let what = format!("dependency {id}");
    if let Some(s) = item.as_str() {
        return Ok(Dependency { id: id.to_string(), req: req(s, &what)?, optional: false });
    }
    let t = item.as_table_like().ok_or_else(|| bad(format!("{what} must be a version range or a table")))?;
    let version = match t.get("version") {
        Some(v) => req(v.as_str().ok_or_else(|| bad(format!("{what}: `version` must be a string")))?, &what)?,
        None => VersionReq::STAR,
    };
    let optional = match t.get("optional") {
        Some(v) => v.as_bool().ok_or_else(|| bad(format!("{what}: `optional` must be true or false")))?,
        None => false,
    };
    Ok(Dependency { id: id.to_string(), req: version, optional })
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| bad(format!("not valid TOML: {}", e.message())))?;
        let id = string(&doc, "id")?.ok_or_else(|| bad("missing `id`"))?;
        if !valid_id(&id) {
            return Err(bad(format!("bad id {id:?}: use lowercase letters, digits, '-', '_' and '.'")));
        }
        let name = string(&doc, "name")?.ok_or_else(|| bad("missing `name`"))?;
        let version = string(&doc, "version")?.ok_or_else(|| bad("missing `version`"))?;
        let version = Version::parse(version.trim()).map_err(|e| bad(format!("bad version {version:?}: {e}")))?;
        let description = string(&doc, "description")?.unwrap_or_default();
        let authors = match doc.get("authors") {
            None => Vec::new(),
            Some(i) => {
                let a = i.as_array().ok_or_else(|| bad("`authors` must be a list of strings"))?;
                a.iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| bad("`authors` must be a list of strings"))).collect::<Result<_, _>>()?
            }
        };
        let content = string(&doc, "content")?.unwrap_or_else(|| ".".into());
        if !safe_relative(&content) {
            return Err(bad(format!("`content` {content:?} must be a folder inside the mod")));
        }
        let engine = string(&doc, "engine")?.map(|s| req(&s, "engine")).transpose()?;
        let mut dependencies = Vec::new();
        if let Some(item) = doc.get("dependencies") {
            let t = item.as_table_like().ok_or_else(|| bad("`dependencies` must be a table"))?;
            for (k, v) in t.iter() {
                if !valid_id(k) {
                    return Err(bad(format!("bad dependency id {k:?}")));
                }
                if k == id {
                    return Err(bad("a mod can't depend on itself"));
                }
                dependencies.push(dependency(k, v)?);
            }
        }
        let mut conflicts = Vec::new();
        match doc.get("conflicts") {
            None => {}
            Some(Item::Value(Value::Array(a))) => {
                for v in a.iter() {
                    let k = v.as_str().ok_or_else(|| bad("`conflicts` must list mod ids"))?;
                    conflicts.push(Conflict { id: k.to_string(), req: VersionReq::STAR });
                }
            }
            Some(item) => {
                let t = item.as_table_like().ok_or_else(|| bad("`conflicts` must be a list of ids or a table of version ranges"))?;
                for (k, v) in t.iter() {
                    let s = v.as_str().ok_or_else(|| bad(format!("conflict {k}: must be a version range")))?;
                    conflicts.push(Conflict { id: k.to_string(), req: req(s, &format!("conflict {k}"))? });
                }
            }
        }
        if let Some(c) = conflicts.iter().find(|c| !valid_id(&c.id) || c.id == id) {
            return Err(bad(format!("bad conflict {:?}", c.id)));
        }
        if let Some(d) = dependencies.iter().find(|d| conflicts.iter().any(|c| c.id == d.id)) {
            return Err(bad(format!("{} is both a dependency and a conflict", d.id)));
        }
        Ok(Manifest { id, name, version, description, authors, content, engine, dependencies, conflicts })
    }

    pub fn read(dir: &std::path::Path) -> Result<Self, Error> {
        let path = dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }
}

/// A relative path that stays inside its folder: no root, no `..`.
pub fn safe_relative(p: &str) -> bool {
    let p = p.replace('\\', "/");
    !p.starts_with('/') && !p.contains(':') && p.split('/').all(|c| c != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r#"
id = "better-cubes"
name = "Better Cubes"
version = "1.2.0"
description = "Shinier cubes"
authors = ["A", "B"]
content = "content"
engine = "^0.1"

[dependencies]
core-textures = "^1.0"
extra-sounds = { version = ">=2, <3", optional = true }
anything = {}

[conflicts]
old-cubes = "*"
"#;

    #[test]
    fn parses_every_field() {
        let m = Manifest::parse(FULL).unwrap();
        assert_eq!(m.id, "better-cubes");
        assert_eq!(m.name, "Better Cubes");
        assert_eq!(m.version, Version::new(1, 2, 0));
        assert_eq!(m.authors, ["A", "B"]);
        assert_eq!(m.content, "content");
        assert!(m.engine.as_ref().unwrap().matches(&Version::new(0, 1, 5)));
        assert_eq!(m.dependencies.len(), 3);
        let core = &m.dependencies[0];
        assert_eq!((core.id.as_str(), core.optional), ("core-textures", false));
        assert!(core.req.matches(&Version::new(1, 9, 0)) && !core.req.matches(&Version::new(2, 0, 0)));
        let extra = &m.dependencies[1];
        assert!(extra.optional && extra.req.matches(&Version::new(2, 5, 0)) && !extra.req.matches(&Version::new(3, 0, 0)));
        assert_eq!(m.dependencies[2].req, VersionReq::STAR);
        assert_eq!(m.conflicts, vec![Conflict { id: "old-cubes".into(), req: VersionReq::STAR }]);
    }

    #[test]
    fn minimal_manifest_has_defaults() {
        let m = Manifest::parse("id = \"x\"\nname = \"X\"\nversion = \"0.1.0\"\nconflicts = [\"y\"]\n").unwrap();
        assert_eq!(m.content, ".");
        assert!(m.engine.is_none() && m.dependencies.is_empty() && m.description.is_empty());
        assert_eq!(m.conflicts[0].id, "y");
    }

    #[test]
    fn rejects_bad_manifests() {
        let base = "name = \"X\"\nversion = \"1.0.0\"\n";
        for (text, why) in [
            (base.to_string(), "missing `id`"),
            (format!("id = \"Bad Id\"\n{base}"), "bad id"),
            ("id = \"x\"\nversion = \"1.0.0\"\n".to_string(), "missing `name`"),
            ("id = \"x\"\nname = \"X\"\nversion = \"one\"\n".to_string(), "bad version"),
            (format!("id = \"x\"\n{base}[dependencies]\ny = \"not a range\"\n"), "bad version range"),
            (format!("id = \"x\"\n{base}[dependencies]\nx = \"*\"\n"), "itself"),
            (format!("id = \"x\"\n{base}content = \"../outside\"\n"), "inside the mod"),
            (format!("id = \"x\"\n{base}[dependencies]\ny = \"*\"\n[conflicts]\ny = \"*\"\n"), "both"),
            (format!("id = \"x\"\n{base}conflicts = [\"x\"]\n"), "bad conflict"),
            ("id = = 3".to_string(), "not valid TOML"),
        ] {
            let e = Manifest::parse(&text).unwrap_err().to_string();
            assert!(e.contains(why), "{text:?}: expected {why:?}, got {e:?}");
        }
    }

    #[test]
    fn ids() {
        assert!(valid_id("a") && valid_id("my-mod_2.0"));
        assert!(!valid_id("") && !valid_id("Upper") && !valid_id(".hidden") && !valid_id("a/b") && !valid_id(&"x".repeat(65)));
    }

    #[test]
    fn relative_paths() {
        assert!(safe_relative(".") && safe_relative("content") && safe_relative("a/b"));
        assert!(!safe_relative("/abs") && !safe_relative("../x") && !safe_relative("a/../../x") && !safe_relative("C:\\x") && !safe_relative("a\\..\\..\\b"));
    }
}
