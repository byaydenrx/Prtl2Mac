//! Profiles: named lists of mods (each pinned to an installed version, on or
//! off) in load order, and snapshots of them for rollback.
//!
//! `profiles/<name>.toml`:
//!
//! ```toml
//! format = 1
//!
//! [[mods]]
//! id = "core-textures"
//! version = "1.0.0"
//! enabled = true
//! ```
//!
//! Snapshots (`snapshots/<profile>/000001.toml`) are the same file plus a
//! `label` (what was about to change) and the `time` (Unix seconds).
use semver::Version;
use toml_edit::DocumentMut;

use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEntry {
    pub id: String,
    pub version: Version,
    pub enabled: bool,
}

/// Load order: the first entry has the lowest priority.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub mods: Vec<ProfileEntry>,
    /// Snapshots only: what the change after it was.
    pub label: String,
    /// Snapshots only: Unix seconds.
    pub time: u64,
}

/// A TOML basic string.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Profile {
    pub fn index(&self, id: &str) -> Option<usize> {
        self.mods.iter().position(|m| m.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&ProfileEntry> {
        self.mods.iter().find(|m| m.id == id)
    }

    pub fn enabled(&self) -> impl Iterator<Item = &ProfileEntry> {
        self.mods.iter().filter(|m| m.enabled)
    }

    /// Move `id` to position `to` (clamped).
    pub fn move_to(&mut self, id: &str, to: usize) -> bool {
        let Some(i) = self.index(id) else { return false };
        let e = self.mods.remove(i);
        let to = to.min(self.mods.len());
        self.mods.insert(to, e);
        true
    }

    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = |m: String| Error::Invalid(format!("profile: {m}"));
        let doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| bad(e.message().to_string()))?;
        let mut p = Profile {
            label: doc.get("label").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            time: doc.get("time").and_then(|v| v.as_integer()).unwrap_or(0).max(0) as u64,
            ..Default::default()
        };
        if let Some(item) = doc.get("mods") {
            let arr = item.as_array_of_tables().ok_or_else(|| bad("`mods` must be [[mods]] tables".into()))?;
            for t in arr.iter() {
                let id = t.get("id").and_then(|v| v.as_str()).ok_or_else(|| bad("a mod without an id".into()))?;
                let version = t.get("version").and_then(|v| v.as_str()).ok_or_else(|| bad(format!("{id}: no version")))?;
                let version = Version::parse(version).map_err(|e| bad(format!("{id}: {e}")))?;
                let enabled = t.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                if !crate::manifest::valid_id(id) {
                    return Err(bad(format!("bad id {id:?}")));
                }
                if p.index(id).is_some() {
                    return Err(bad(format!("{id} is listed twice")));
                }
                p.mods.push(ProfileEntry { id: id.to_string(), version, enabled });
            }
        }
        Ok(p)
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("format = 1\n");
        if !self.label.is_empty() {
            s += &format!("label = {}\n", quote(&self.label));
        }
        if self.time != 0 {
            s += &format!("time = {}\n", self.time);
        }
        for m in &self.mods {
            s += &format!("\n[[mods]]\nid = {}\nversion = {}\nenabled = {}\n", quote(&m.id), quote(&m.version.to_string()), m.enabled);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, v: &str, enabled: bool) -> ProfileEntry {
        ProfileEntry { id: id.into(), version: Version::parse(v).unwrap(), enabled }
    }

    #[test]
    fn round_trip() {
        let p = Profile { mods: vec![entry("a", "1.0.0", true), entry("b", "2.1.3-beta.1", false)], label: "enable \"b\"\n".into(), time: 1_700_000_000 };
        let text = p.to_text();
        assert_eq!(Profile::parse(&text).unwrap(), p);
        assert_eq!(Profile::parse(&Profile::default().to_text()).unwrap(), Profile::default());
    }

    #[test]
    fn rejects_bad_profiles() {
        assert!(Profile::parse("[[mods]]\nversion = \"1.0.0\"\n").is_err());
        assert!(Profile::parse("[[mods]]\nid = \"a\"\n").is_err());
        assert!(Profile::parse("[[mods]]\nid = \"a\"\nversion = \"x\"\n").is_err());
        assert!(Profile::parse("[[mods]]\nid = \"a\"\nversion = \"1.0.0\"\n[[mods]]\nid = \"a\"\nversion = \"1.0.0\"\n").unwrap_err().to_string().contains("twice"));
        assert!(Profile::parse("mods = 3").is_err());
    }

    #[test]
    fn moving() {
        let mut p = Profile { mods: vec![entry("a", "1.0.0", true), entry("b", "1.0.0", true), entry("c", "1.0.0", true)], ..Default::default() };
        assert!(p.move_to("c", 0));
        assert_eq!(p.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["c", "a", "b"]);
        assert!(p.move_to("c", 99));
        assert_eq!(p.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        assert!(!p.move_to("zzz", 0));
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("a\"b\\c\td\u{1}"), "\"a\\\"b\\\\c\\td\\u0001\"");
    }
}
