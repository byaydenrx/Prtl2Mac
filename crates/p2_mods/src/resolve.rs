//! Which enabled mods can run together, and in what order.
//!
//! Load order runs from the lowest priority to the highest: a mod later in
//! the order wins when two mods provide the same file, and every mod loads
//! after the mods it depends on (so it can override them). Mods with a
//! problem that would break them (a missing dependency, a wrong version, a
//! conflict, a dependency cycle) are left out, along with every mod that
//! needs them; the rest still load.
use std::collections::{HashMap, HashSet};

use semver::Version;

use crate::manifest::Manifest;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The mod works; something may look or behave oddly.
    Warning,
    /// The mod can't load.
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub mod_id: String,
    pub message: String,
}

impl Issue {
    fn error(id: &str, message: String) -> Self {
        Issue { severity: Severity::Error, mod_id: id.to_string(), message }
    }
    fn warning(id: &str, message: String) -> Self {
        Issue { severity: Severity::Warning, mod_id: id.to_string(), message }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{s}: {}: {}", self.mod_id, self.message)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    /// Mods that load, lowest priority first.
    pub order: Vec<String>,
    /// Enabled mods left out (see `issues` for why).
    pub skipped: Vec<String>,
    pub issues: Vec<Issue>,
}

impl Resolution {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }
}

/// Check the `enabled` mods (in the player's load order) against each other.
/// `installed` is every mod in the store (to tell "not installed" from "not
/// enabled"); `engine` is the game's version for the manifests' `engine` ranges.
pub fn resolve(enabled: &[&Manifest], installed: &[&Manifest], engine: Option<&Version>) -> Resolution {
    let mut issues = Vec::new();
    let by_id: HashMap<&str, &Manifest> = enabled.iter().map(|m| (m.id.as_str(), *m)).collect();
    let mut broken: HashSet<String> = HashSet::new();

    for m in enabled {
        if let (Some(req), Some(v)) = (&m.engine, engine) {
            if !req.matches(v) {
                issues.push(Issue::warning(&m.id, format!("made for game version {req}, this is {v}")));
            }
        }
        for d in &m.dependencies {
            match by_id.get(d.id.as_str()) {
                Some(dep) if d.req.matches(&dep.version) => {}
                Some(dep) if d.optional => {
                    issues.push(Issue::warning(&m.id, format!("works with {} {} (optional), but {} is enabled", d.id, d.req, dep.version)))
                }
                Some(dep) => {
                    broken.insert(m.id.clone());
                    issues.push(Issue::error(&m.id, format!("needs {} {}, but {} is enabled", d.id, d.req, dep.version)));
                }
                None if d.optional => {}
                None => {
                    broken.insert(m.id.clone());
                    let fits: Vec<&Version> = installed.iter().filter(|i| i.id == d.id && d.req.matches(&i.version)).map(|i| &i.version).collect();
                    let msg = match (fits.first(), installed.iter().any(|i| i.id == d.id)) {
                        (Some(v), _) => format!("needs {} {}: installed ({v}) but not enabled", d.id, d.req),
                        (None, true) => format!("needs {} {}: the installed versions don't match", d.id, d.req),
                        (None, false) => format!("needs {} {}, which is not installed", d.id, d.req),
                    };
                    issues.push(Issue::error(&m.id, msg));
                }
            }
        }
        for c in &m.conflicts {
            if let Some(other) = by_id.get(c.id.as_str()) {
                if c.req.matches(&other.version) {
                    broken.insert(m.id.clone());
                    issues.push(Issue::error(&m.id, format!("can't be used with {} {} (disable one of them)", other.id, other.version)));
                }
            }
        }
    }

    // Dependencies inside the enabled set (optional ones too: they still
    // decide the order).
    let deps_of = |m: &Manifest| -> Vec<String> { m.dependencies.iter().filter(|d| by_id.contains_key(d.id.as_str())).map(|d| d.id.clone()).collect() };

    // Placing: the first mod in the player's order whose dependencies are
    // all placed, again and again; what can never be placed is a cycle.
    let mut order: Vec<String> = Vec::new();
    let mut placed: HashSet<String> = HashSet::new();
    let mut left: Vec<&Manifest> = enabled.to_vec();
    while !left.is_empty() {
        let pick = left.iter().position(|m| deps_of(m).iter().all(|d| placed.contains(d)));
        match pick {
            Some(i) => {
                let m = left.remove(i);
                placed.insert(m.id.clone());
                order.push(m.id.clone());
            }
            None => {
                for m in &left {
                    broken.insert(m.id.clone());
                    let chain = cycle_from(m, &by_id, &placed);
                    issues.push(Issue::error(&m.id, format!("dependency cycle: {}", chain.join(" -> "))));
                }
                left.clear();
            }
        }
    }

    // A dependent the player put ahead of its dependency moved after it.
    let user_pos: HashMap<&str, usize> = enabled.iter().enumerate().map(|(i, m)| (m.id.as_str(), i)).collect();
    for m in enabled {
        for d in deps_of(m) {
            if user_pos[d.as_str()] > user_pos[m.id.as_str()] && !broken.contains(&m.id) {
                issues.push(Issue::warning(&m.id, format!("was ordered before its dependency {d}; loading it after {d}")));
            }
        }
    }

    // Mods that need a broken mod break too.
    loop {
        let more: Vec<String> = enabled
            .iter()
            .filter(|m| !broken.contains(&m.id))
            .filter_map(|m| m.dependencies.iter().find(|d| !d.optional && broken.contains(&d.id)).map(|d| (m.id.clone(), d.id.clone())))
            .map(|(id, dep)| {
                issues.push(Issue::error(&id, format!("needs {dep}, which can't load")));
                id
            })
            .collect();
        if more.is_empty() {
            break;
        }
        broken.extend(more);
    }

    let skipped = enabled.iter().filter(|m| broken.contains(&m.id)).map(|m| m.id.clone()).collect();
    order.retain(|id| !broken.contains(id));
    issues.sort_by_key(|i| std::cmp::Reverse(i.severity));
    Resolution { order, skipped, issues }
}

/// A dependency path from `m` back to a mod already on it.
fn cycle_from(m: &Manifest, by_id: &HashMap<&str, &Manifest>, placed: &HashSet<String>) -> Vec<String> {
    let mut chain = vec![m.id.clone()];
    let mut cur = m;
    while let Some(next) = cur.dependencies.iter().filter(|d| !placed.contains(&d.id)).find_map(|d| by_id.get(d.id.as_str())) {
        let seen = chain.contains(&next.id);
        chain.push(next.id.clone());
        if seen || chain.len() > by_id.len() + 1 {
            break;
        }
        cur = next;
    }
    chain
}

/// Warnings for files more than one loaded mod provides: the later one
/// wins, which is usually a surprise unless it depends on the other.
/// `files` lists a mod's files (normalised relative paths).
pub fn overlaps(order: &[String], manifests: &HashMap<String, &Manifest>, files: &dyn Fn(&str) -> Vec<String>) -> Vec<Issue> {
    let mut owner: HashMap<String, usize> = HashMap::new();
    // (earlier, later) -> files the later one overrides
    let mut clashes: Vec<((usize, usize), Vec<String>)> = Vec::new();
    for (i, id) in order.iter().enumerate() {
        for f in files(id) {
            if f == crate::manifest::MANIFEST {
                continue;
            }
            if let Some(&j) = owner.get(&f) {
                match clashes.iter_mut().find(|(k, _)| *k == (j, i)) {
                    Some((_, list)) => list.push(f.clone()),
                    None => clashes.push(((j, i), vec![f.clone()])),
                }
            }
            owner.insert(f, i);
        }
    }
    let depends = |a: &str, b: &str| manifests.get(a).is_some_and(|m| m.dependencies.iter().any(|d| d.id == b));
    clashes
        .into_iter()
        .filter(|((j, i), _)| !depends(&order[*i], &order[*j]))
        .map(|((j, i), mut list)| {
            list.sort();
            let n = list.len();
            let example = &list[0];
            Issue::warning(&order[i], format!("replaces {n} file(s) of {} (e.g. {example}); {} wins because it loads later", order[j], order[i]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, version: &str, deps: &[(&str, &str)], conflicts: &[(&str, &str)]) -> Manifest {
        let mut t = format!("id = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\n[dependencies]\n");
        for (d, r) in deps {
            if let Some(r) = r.strip_prefix('?') {
                t += &format!("{d} = {{ version = \"{r}\", optional = true }}\n");
            } else {
                t += &format!("{d} = \"{r}\"\n");
            }
        }
        t += "[conflicts]\n";
        for (c, r) in conflicts {
            t += &format!("{c} = \"{r}\"\n");
        }
        Manifest::parse(&t).unwrap()
    }

    fn ids(v: &[String]) -> Vec<&str> {
        v.iter().map(String::as_str).collect()
    }

    #[test]
    fn independent_mods_keep_the_players_order() {
        let (a, b, c) = (m("a", "1.0.0", &[], &[]), m("b", "1.0.0", &[], &[]), m("c", "1.0.0", &[], &[]));
        let r = resolve(&[&c, &a, &b], &[&a, &b, &c], None);
        assert_eq!(ids(&r.order), ["c", "a", "b"]);
        assert!(r.issues.is_empty() && r.skipped.is_empty());
    }

    #[test]
    fn dependencies_load_first_with_a_warning() {
        let lib = m("lib", "1.4.0", &[], &[]);
        let app = m("app", "2.0.0", &[("lib", "^1.2")], &[]);
        let r = resolve(&[&app, &lib], &[&app, &lib], None);
        assert_eq!(ids(&r.order), ["lib", "app"]);
        assert_eq!(r.issues.len(), 1);
        assert_eq!(r.issues[0].severity, Severity::Warning);
        assert!(r.issues[0].message.contains("before its dependency lib"));
        // In the right order already: no warning.
        let r = resolve(&[&lib, &app], &[&app, &lib], None);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn missing_disabled_and_wrong_version_dependencies() {
        let app = m("app", "1.0.0", &[("lib", "^2")], &[]);
        let lib1 = m("lib", "1.0.0", &[], &[]);
        let lib2 = m("lib", "2.1.0", &[], &[]);
        // Not installed at all.
        let r = resolve(&[&app], &[&app], None);
        assert_eq!(r.skipped, ["app"]);
        assert!(r.issues[0].message.contains("not installed"), "{:?}", r.issues);
        // Installed in a fitting version but not enabled.
        let r = resolve(&[&app], &[&app, &lib1, &lib2], None);
        assert!(r.issues[0].message.contains("installed (2.1.0) but not enabled"), "{:?}", r.issues);
        // Only a wrong version installed.
        let r = resolve(&[&app], &[&app, &lib1], None);
        assert!(r.issues[0].message.contains("don't match"));
        // The wrong version enabled.
        let r = resolve(&[&lib1, &app], &[&app, &lib1], None);
        assert_eq!(ids(&r.order), ["lib"]);
        assert!(r.issues[0].message.contains("but 1.0.0 is enabled"));
        assert!(r.has_errors());
    }

    #[test]
    fn optional_dependencies() {
        let app = m("app", "1.0.0", &[("extra", "?^1")], &[]);
        let extra1 = m("extra", "1.5.0", &[], &[]);
        let extra2 = m("extra", "2.0.0", &[], &[]);
        // Absent: fine.
        let r = resolve(&[&app], &[&app], None);
        assert_eq!(ids(&r.order), ["app"]);
        assert!(r.issues.is_empty());
        // Present: ordered first.
        let r = resolve(&[&app, &extra1], &[&app, &extra1], None);
        assert_eq!(ids(&r.order), ["extra", "app"]);
        // Present in another version: a warning only.
        let r = resolve(&[&extra2, &app], &[&app, &extra2], None);
        assert_eq!(ids(&r.order), ["extra", "app"]);
        assert_eq!(r.issues.len(), 1);
        assert_eq!(r.issues[0].severity, Severity::Warning);
    }

    #[test]
    fn conflicts_by_version() {
        let new = m("new-cubes", "1.0.0", &[], &[("old-cubes", "<2")]);
        let old1 = m("old-cubes", "1.9.0", &[], &[]);
        let old2 = m("old-cubes", "2.0.0", &[], &[]);
        let r = resolve(&[&old1, &new], &[&new, &old1], None);
        assert_eq!(ids(&r.order), ["old-cubes"]);
        assert_eq!(r.skipped, ["new-cubes"]);
        assert!(r.issues[0].message.contains("can't be used with old-cubes 1.9.0"));
        let r = resolve(&[&old2, &new], &[&new, &old2], None);
        assert_eq!(ids(&r.order), ["old-cubes", "new-cubes"]);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn breakage_spreads_to_dependents_only() {
        let base = m("base", "1.0.0", &[("missing", "*")], &[]);
        let mid = m("mid", "1.0.0", &[("base", "*")], &[]);
        let top = m("top", "1.0.0", &[("mid", "*")], &[]);
        let other = m("other", "1.0.0", &[], &[]);
        let r = resolve(&[&base, &mid, &top, &other], &[&base, &mid, &top, &other], None);
        assert_eq!(ids(&r.order), ["other"]);
        assert_eq!(r.skipped, ["base", "mid", "top"]);
        assert!(r.issues.iter().any(|i| i.mod_id == "top" && i.message.contains("needs mid, which can't load")));
    }

    #[test]
    fn cycles_are_errors() {
        let a = m("a", "1.0.0", &[("b", "*")], &[]);
        let b = m("b", "1.0.0", &[("c", "*")], &[]);
        let c = m("c", "1.0.0", &[("a", "*")], &[]);
        let d = m("d", "1.0.0", &[], &[]);
        let r = resolve(&[&a, &b, &c, &d], &[&a, &b, &c, &d], None);
        assert_eq!(ids(&r.order), ["d"]);
        assert_eq!(r.skipped.len(), 3);
        let msg = &r.issues.iter().find(|i| i.mod_id == "a").unwrap().message;
        assert!(msg.contains("a -> b -> c -> a"), "{msg}");
    }

    #[test]
    fn engine_version_warning() {
        let mut a = m("a", "1.0.0", &[], &[]);
        a.engine = Some(semver::VersionReq::parse("^0.2").unwrap());
        let r = resolve(&[&a], &[&a], Some(&Version::new(0, 1, 0)));
        assert_eq!(ids(&r.order), ["a"]);
        assert_eq!(r.issues[0].severity, Severity::Warning);
        assert!(resolve(&[&a], &[&a], Some(&Version::new(0, 2, 3))).issues.is_empty());
    }

    #[test]
    fn errors_sort_before_warnings() {
        let lib = m("lib", "1.0.0", &[], &[]);
        let app = m("app", "1.0.0", &[("lib", "*")], &[]);
        let bad = m("bad", "1.0.0", &[("nope", "*")], &[]);
        let r = resolve(&[&app, &lib, &bad], &[&app, &lib, &bad], None);
        assert_eq!(r.issues[0].severity, Severity::Error);
        assert_eq!(r.issues.last().unwrap().severity, Severity::Warning);
    }

    #[test]
    fn overlapping_files() {
        let a = m("a", "1.0.0", &[], &[]);
        let b = m("b", "1.0.0", &[], &[]);
        let c = m("c", "1.0.0", &[("a", "*")], &[]);
        let man: HashMap<String, &Manifest> = [("a".to_string(), &a), ("b".to_string(), &b), ("c".to_string(), &c)].into();
        let files = |id: &str| -> Vec<String> {
            match id {
                "a" => vec!["mod.toml".into(), "materials/x.vmt".into(), "materials/y.vmt".into()],
                "b" => vec!["mod.toml".into(), "materials/y.vmt".into()],
                "c" => vec!["materials/x.vmt".into()],
                _ => vec![],
            }
        };
        let order: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        let w = overlaps(&order, &man, &files);
        // b overrides a's y.vmt (warned); c overrides a's x.vmt but depends on a (expected).
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].mod_id, "b");
        assert!(w[0].message.contains("1 file(s) of a (e.g. materials/y.vmt)"), "{}", w[0].message);
    }
}
