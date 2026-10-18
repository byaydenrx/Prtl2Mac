//! Mod manager core for the Portal 2 remake. Pure Rust, no Bevy.
//!
//! Mods never replace the game's files: each installed mod lives in its own
//! folder in a managed store, and the game mounts the enabled mods' content
//! folders *ahead of* the install's own search paths ([`ModManager::mount_plan`]),
//! so a mod's `materials/foo.vmt` is found before the game's.
//!
//! Everything lives under one root (the game uses
//! `<data dir>/mods`, e.g. `~/Library/Application Support/Portal2Rust/mods`):
//!
//! ```text
//! store/<id>/<version>/       an installed mod (its mod.toml at the top)
//! profiles/<name>.toml        a profile: mods in load order, each on or off
//! active_profile              the profile the game loads ("default")
//! snapshots/<profile>/NNNNNN.toml   the profile before each change (rollback)
//! tmp/                        unpacking space
//! ```
//!
//! * [`manifest`]: the `mod.toml` format (id, name, version, dependencies
//!   with version ranges, conflicts).
//! * [`resolve`]: dependency and conflict checks, load order, warnings.
//! * [`profile`]: profiles and snapshots.
//! * [`archive`]: unpacking `.zip` mods.
//!
//! The `p2mods` binary drives all of it from a terminal.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use semver::Version;

pub mod archive;
pub mod manifest;
pub mod profile;
pub mod resolve;
mod store;

pub use manifest::{Conflict, Dependency, Manifest, MANIFEST};
pub use profile::{Profile, ProfileEntry};
pub use resolve::{Issue, Resolution, Severity};
pub use store::InstalledMod;
/// The version types in this API.
pub use semver;

/// The profile used until the player makes another.
pub const DEFAULT_PROFILE: &str = "default";
/// Snapshots kept per profile; the oldest go first.
pub const MAX_SNAPSHOTS: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Io(String),
    Manifest(String),
    Archive(String),
    /// A request that can't be done as asked (unknown mod, bad name...).
    Invalid(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(m) => write!(f, "{m}"),
            Error::Manifest(m) => write!(f, "mod.toml: {m}"),
            Error::Archive(m) => write!(f, "zip: {m}"),
            Error::Invalid(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// What the game should mount, and what it should tell the player.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MountPlan {
    pub profile: String,
    /// Content folders, highest priority first: mount them in this order,
    /// all before the base game's search paths.
    pub dirs: Vec<PathBuf>,
    /// The mods behind `dirs`, in load order (lowest priority first).
    pub loaded: Vec<(String, Version)>,
    pub issues: Vec<Issue>,
}

/// A snapshot as listed for the player.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotInfo {
    pub number: u32,
    pub label: String,
    pub time: u64,
    pub enabled: usize,
}

pub struct ModManager {
    root: PathBuf,
    store: store::Store,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn check_profile_name(name: &str) -> Result<(), Error> {
    if manifest::valid_id(name) {
        Ok(())
    } else {
        Err(Error::Invalid(format!("bad profile name {name:?}: use lowercase letters, digits, '-', '_' and '.'")))
    }
}

/// Write via a temporary file and a rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

impl ModManager {
    /// Open (creating if needed) the mod folder at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, Error> {
        let root = root.into();
        std::fs::create_dir_all(root.join("store"))?;
        std::fs::create_dir_all(root.join("profiles"))?;
        Ok(ModManager { store: store::Store { root: root.join("store") }, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ------------------------------------------------------------- store --

    /// Every installed mod version.
    pub fn installed(&self) -> Vec<InstalledMod> {
        self.store.list()
    }

    pub fn installed_version(&self, id: &str, version: &Version) -> Option<InstalledMod> {
        self.store.get(id, version)
    }

    /// Install a mod folder or `.zip` into the store (it isn't enabled).
    pub fn install(&self, src: &Path) -> Result<InstalledMod, Error> {
        self.store.install(src, &self.root.join("tmp"))
    }

    /// Remove an installed version. Refused while a profile has it enabled;
    /// disabled profile entries for it go (with a snapshot).
    pub fn uninstall(&self, id: &str, version: &Version) -> Result<(), Error> {
        let users: Vec<String> = self
            .profiles()
            .into_iter()
            .filter(|p| self.load_profile(p).is_ok_and(|pr| pr.get(id).is_some_and(|e| e.enabled && &e.version == version)))
            .collect();
        if !users.is_empty() {
            return Err(Error::Invalid(format!("{id} {version} is enabled in profile(s) {}; disable it first", users.join(", "))));
        }
        self.store.remove(id, version)?;
        for p in self.profiles() {
            let Ok(pr) = self.load_profile(&p) else { continue };
            if pr.get(id).is_some_and(|e| &e.version == version) {
                self.change(&p, &format!("uninstall {id} {version}"), |pr| {
                    pr.mods.retain(|e| !(e.id == id && &e.version == version));
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    /// Delete store versions no profile or snapshot refers to. Returns them.
    pub fn clean(&self) -> Result<Vec<(String, Version)>, Error> {
        let mut used: Vec<(String, Version)> = Vec::new();
        for p in self.profiles() {
            used.extend(self.load_profile(&p)?.mods.into_iter().map(|e| (e.id, e.version)));
            for s in self.snapshots(&p) {
                if let Ok(snap) = self.read_snapshot(&p, s.number) {
                    used.extend(snap.mods.into_iter().map(|e| (e.id, e.version)));
                }
            }
        }
        let mut removed = Vec::new();
        for m in self.installed() {
            let key = (m.manifest.id.clone(), m.manifest.version.clone());
            if !used.contains(&key) {
                self.store.remove(&key.0, &key.1)?;
                removed.push(key);
            }
        }
        Ok(removed)
    }

    // ----------------------------------------------------------- profiles --

    fn profile_path(&self, name: &str) -> PathBuf {
        self.root.join("profiles").join(format!("{name}.toml"))
    }

    /// Profile names, sorted; the default one is always there.
    pub fn profiles(&self) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(self.root.join("profiles"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".toml").map(str::to_string))
            .filter(|n| manifest::valid_id(n))
            .collect();
        if !out.iter().any(|p| p == DEFAULT_PROFILE) {
            out.push(DEFAULT_PROFILE.into());
        }
        out.sort();
        out
    }

    pub fn active_profile(&self) -> String {
        std::fs::read_to_string(self.root.join("active_profile"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|n| manifest::valid_id(n) && (n == DEFAULT_PROFILE || self.profile_path(n).is_file()))
            .unwrap_or_else(|| DEFAULT_PROFILE.into())
    }

    pub fn set_active_profile(&self, name: &str) -> Result<(), Error> {
        check_profile_name(name)?;
        if name != DEFAULT_PROFILE && !self.profile_path(name).is_file() {
            return Err(Error::Invalid(format!("no profile {name:?}")));
        }
        write_atomic(&self.root.join("active_profile"), &format!("{name}\n"))
    }

    /// A profile; one that was never saved is empty.
    pub fn load_profile(&self, name: &str) -> Result<Profile, Error> {
        check_profile_name(name)?;
        match std::fs::read_to_string(self.profile_path(name)) {
            Ok(text) => Profile::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profile::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// A new profile, empty or a copy of `from`.
    pub fn create_profile(&self, name: &str, from: Option<&str>) -> Result<(), Error> {
        check_profile_name(name)?;
        if self.profile_path(name).is_file() {
            return Err(Error::Invalid(format!("profile {name:?} exists")));
        }
        let mut p = match from {
            Some(f) => self.load_profile(f)?,
            None => Profile::default(),
        };
        p.label.clear();
        p.time = 0;
        write_atomic(&self.profile_path(name), &p.to_text())
    }

    /// Delete a profile and its snapshots (not the default one).
    pub fn delete_profile(&self, name: &str) -> Result<(), Error> {
        check_profile_name(name)?;
        if name == DEFAULT_PROFILE {
            return Err(Error::Invalid("the default profile can't be deleted".into()));
        }
        let was_active = self.active_profile() == name;
        std::fs::remove_file(self.profile_path(name)).map_err(|e| Error::Io(format!("profile {name}: {e}")))?;
        let _ = std::fs::remove_dir_all(self.root.join("snapshots").join(name));
        if was_active {
            let _ = std::fs::remove_file(self.root.join("active_profile"));
        }
        Ok(())
    }

    /// Change a profile, saving a snapshot of it first.
    fn change(&self, name: &str, label: &str, f: impl FnOnce(&mut Profile) -> Result<(), Error>) -> Result<(), Error> {
        let before = self.load_profile(name)?;
        let mut after = before.clone();
        f(&mut after)?;
        if after.mods == before.mods {
            return Ok(());
        }
        self.save_snapshot(name, &before, label)?;
        after.label.clear();
        after.time = 0;
        write_atomic(&self.profile_path(name), &after.to_text())
    }

    /// Turn a mod on: `version` or, if not given, the version the profile
    /// already pins (when still installed) or the newest installed. New
    /// entries go to the end of the load order (highest priority).
    pub fn enable(&self, profile: &str, id: &str, version: Option<&Version>) -> Result<Version, Error> {
        let current = self.load_profile(profile)?.get(id).map(|e| e.version.clone());
        let pick = match version {
            Some(v) => self.store.get(id, v).ok_or_else(|| Error::Invalid(format!("{id} {v} is not installed")))?,
            None => current
                .as_ref()
                .and_then(|v| self.store.get(id, v))
                .or_else(|| self.store.newest(id))
                .ok_or_else(|| Error::Invalid(format!("{id} is not installed")))?,
        };
        let v = pick.manifest.version.clone();
        self.change(profile, &format!("enable {id} {v}"), |p| {
            match p.mods.iter_mut().find(|e| e.id == id) {
                Some(e) => {
                    e.enabled = true;
                    e.version = v.clone();
                }
                None => p.mods.push(ProfileEntry { id: id.to_string(), version: v.clone(), enabled: true }),
            }
            Ok(())
        })?;
        Ok(v)
    }

    /// Turn a mod off (it keeps its place in the load order).
    pub fn disable(&self, profile: &str, id: &str) -> Result<(), Error> {
        self.change(profile, &format!("disable {id}"), |p| match p.mods.iter_mut().find(|e| e.id == id) {
            Some(e) => {
                e.enabled = false;
                Ok(())
            }
            None => Err(Error::Invalid(format!("{id} is not in profile {profile}"))),
        })
    }

    /// Take a mod out of a profile altogether.
    pub fn remove_from_profile(&self, profile: &str, id: &str) -> Result<(), Error> {
        self.change(profile, &format!("remove {id}"), |p| {
            let i = p.index(id).ok_or_else(|| Error::Invalid(format!("{id} is not in profile {profile}")))?;
            p.mods.remove(i);
            Ok(())
        })
    }

    /// Put a mod at `position` in the load order (0 = lowest priority).
    pub fn move_mod(&self, profile: &str, id: &str, position: usize) -> Result<(), Error> {
        self.change(profile, &format!("move {id} to {position}"), |p| {
            if p.move_to(id, position) {
                Ok(())
            } else {
                Err(Error::Invalid(format!("{id} is not in profile {profile}")))
            }
        })
    }

    /// Point every entry at the newest installed version of its mod.
    pub fn upgrade_all(&self, profile: &str) -> Result<Vec<(String, Version)>, Error> {
        let mut changed = Vec::new();
        self.change(profile, "upgrade all", |p| {
            for e in &mut p.mods {
                if let Some(n) = self.store.newest(&e.id) {
                    if n.manifest.version > e.version {
                        e.version = n.manifest.version.clone();
                        changed.push((e.id.clone(), e.version.clone()));
                    }
                }
            }
            Ok(())
        })?;
        Ok(changed)
    }

    // ---------------------------------------------------------- snapshots --

    fn snapshot_dir(&self, profile: &str) -> PathBuf {
        self.root.join("snapshots").join(profile)
    }

    fn snapshot_numbers(&self, profile: &str) -> Vec<u32> {
        let mut n: Vec<u32> = std::fs::read_dir(self.snapshot_dir(profile))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".toml").and_then(|s| s.parse().ok()))
            .collect();
        n.sort();
        n
    }

    fn save_snapshot(&self, profile: &str, p: &Profile, label: &str) -> Result<(), Error> {
        let numbers = self.snapshot_numbers(profile);
        let next = numbers.last().map_or(1, |n| n + 1);
        let snap = Profile { mods: p.mods.clone(), label: label.to_string(), time: now_secs() };
        write_atomic(&self.snapshot_dir(profile).join(format!("{next:06}.toml")), &snap.to_text())?;
        // Keep the newest MAX_SNAPSHOTS.
        let total = numbers.len() + 1;
        for old in numbers.iter().take(total.saturating_sub(MAX_SNAPSHOTS)) {
            let _ = std::fs::remove_file(self.snapshot_dir(profile).join(format!("{old:06}.toml")));
        }
        Ok(())
    }

    fn read_snapshot(&self, profile: &str, number: u32) -> Result<Profile, Error> {
        let path = self.snapshot_dir(profile).join(format!("{number:06}.toml"));
        Profile::parse(&std::fs::read_to_string(&path).map_err(|e| Error::Io(format!("snapshot {number}: {e}")))?)
    }

    /// Snapshots of a profile, newest first.
    pub fn snapshots(&self, profile: &str) -> Vec<SnapshotInfo> {
        let mut out: Vec<SnapshotInfo> = self
            .snapshot_numbers(profile)
            .into_iter()
            .filter_map(|n| self.read_snapshot(profile, n).ok().map(|s| SnapshotInfo { number: n, enabled: s.enabled().count(), label: s.label, time: s.time }))
            .collect();
        out.reverse();
        out
    }

    /// Undo the last change to a profile (or go back to snapshot `to`,
    /// dropping the snapshots after it). Returns what was undone.
    pub fn rollback(&self, profile: &str, to: Option<u32>) -> Result<Option<String>, Error> {
        check_profile_name(profile)?;
        let numbers = self.snapshot_numbers(profile);
        let target = match to {
            Some(n) if numbers.contains(&n) => n,
            Some(n) => return Err(Error::Invalid(format!("profile {profile} has no snapshot {n}"))),
            None => match numbers.last() {
                Some(n) => *n,
                None => return Ok(None),
            },
        };
        let mut snap = self.read_snapshot(profile, target)?;
        let label = std::mem::take(&mut snap.label);
        snap.time = 0;
        write_atomic(&self.profile_path(profile), &snap.to_text())?;
        for n in numbers.into_iter().filter(|n| *n >= target) {
            let _ = std::fs::remove_file(self.snapshot_dir(profile).join(format!("{n:06}.toml")));
        }
        Ok(Some(label))
    }

    // ------------------------------------------------------------ checks --

    /// Check a profile: what loads, in which order, and every problem.
    /// `engine` is the game's version (for the manifests' `engine` ranges).
    pub fn check(&self, profile: &str, engine: Option<&Version>) -> Result<(Resolution, Vec<InstalledMod>), Error> {
        let p = self.load_profile(profile)?;
        let installed = self.installed();
        let mut missing = Vec::new();
        let mut enabled: Vec<InstalledMod> = Vec::new();
        for e in p.enabled() {
            match installed.iter().find(|m| m.manifest.id == e.id && m.manifest.version == e.version) {
                Some(m) => enabled.push(m.clone()),
                None => missing.push(Issue { severity: Severity::Error, mod_id: e.id.clone(), message: format!("{} is enabled but not installed (reinstall it or roll back)", e.version) }),
            }
        }
        let en: Vec<&Manifest> = enabled.iter().map(|m| &m.manifest).collect();
        let all: Vec<&Manifest> = installed.iter().map(|m| &m.manifest).collect();
        let mut r = resolve::resolve(&en, &all, engine);
        let by_id: HashMap<String, &Manifest> = en.iter().map(|m| (m.id.clone(), *m)).collect();
        let files = |id: &str| enabled.iter().find(|m| m.manifest.id == id).map(|m| m.files()).unwrap_or_default();
        r.issues.extend(resolve::overlaps(&r.order, &by_id, &files));
        r.skipped.extend(missing.iter().map(|i| i.mod_id.clone()));
        missing.append(&mut r.issues);
        r.issues = missing;
        r.issues.sort_by_key(|i| std::cmp::Reverse(i.severity));
        let loaded = r.order.iter().filter_map(|id| enabled.iter().find(|m| &m.manifest.id == id).cloned()).collect();
        Ok((r, loaded))
    }

    /// The active profile's content folders to mount ahead of the game,
    /// highest priority first. Mods with errors are left out.
    pub fn mount_plan(&self, engine: Option<&Version>) -> MountPlan {
        let profile = self.active_profile();
        match self.check(&profile, engine) {
            Ok((r, loaded)) => MountPlan {
                dirs: loaded.iter().rev().map(|m| m.content_dir()).collect(),
                loaded: loaded.iter().map(|m| (m.manifest.id.clone(), m.manifest.version.clone())).collect(),
                issues: r.issues,
                profile,
            },
            Err(e) => MountPlan { issues: vec![Issue { severity: Severity::Error, mod_id: profile.clone(), message: e.to_string() }], profile, ..Default::default() },
        }
    }
}

/// The folders the game mounts ahead of the base game, highest priority
/// first, for the mods enabled in the active profile under `root`. Nothing
/// (and no folders created) when `root` doesn't exist.
pub fn enabled_content_dirs(root: &Path, engine: Option<&Version>) -> MountPlan {
    if !root.join("profiles").is_dir() {
        return MountPlan { profile: DEFAULT_PROFILE.into(), ..Default::default() };
    }
    match ModManager::open(root) {
        Ok(m) => m.mount_plan(engine),
        Err(e) => MountPlan { profile: DEFAULT_PROFILE.into(), issues: vec![Issue { severity: Severity::Error, mod_id: "mods".into(), message: e.to_string() }], ..Default::default() },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A folder under the system temp dir, removed when dropped.
    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("p2mods-test-{tag}-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A mod folder with a manifest and some files.
    fn make_mod(at: &Path, id: &str, version: &str, extra: &str, files: &[&str]) -> PathBuf {
        let dir = at.join(format!("{id}-{version}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST), format!("id = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\n{extra}")).unwrap();
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, format!("{id} {version} {f}")).unwrap();
        }
        dir
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn setup() -> (TempDir, ModManager) {
        let t = TempDir::new("mgr");
        let m = ModManager::open(t.path().join("mods")).unwrap();
        (t, m)
    }

    #[test]
    fn install_from_folder_and_zip() {
        let (t, m) = setup();
        let src = make_mod(t.path(), "cubes", "1.0.0", "", &["materials/cube.vmt"]);
        let a = m.install(&src).unwrap();
        assert_eq!(a.manifest.id, "cubes");
        assert!(a.dir.ends_with("store/cubes/1.0.0"));
        assert_eq!(a.files(), ["materials/cube.vmt", "mod.toml"]);
        // The original folder is untouched.
        assert!(src.join("materials/cube.vmt").is_file());

        // A zip that wraps the mod in a top folder, with a content subfolder.
        let zip = t.path().join("sounds.zip");
        let mut w = archive::ZipWriter::create(&zip).unwrap();
        w.add("Sounds v2/mod.toml", b"id = \"sounds\"\nname = \"Sounds\"\nversion = \"2.0.0\"\ncontent = \"content\"\n", false).unwrap();
        w.add("Sounds v2/content/scripts/game_sounds.txt", b"x", true).unwrap();
        w.finish().unwrap();
        let b = m.install(&zip).unwrap();
        assert_eq!(b.manifest.version, v("2.0.0"));
        assert_eq!(b.content_dir(), b.dir.join("content"));
        assert_eq!(b.files(), ["scripts/game_sounds.txt"]);
        assert_eq!(m.installed().len(), 2);
        // Unpacking space is cleaned up.
        assert_eq!(std::fs::read_dir(m.root().join("tmp")).unwrap().count(), 0);
    }

    #[test]
    fn install_errors() {
        let (t, m) = setup();
        let empty = t.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(m.install(&empty).unwrap_err().to_string().contains("no mod.toml"));
        let no_content = make_mod(t.path(), "x", "1.0.0", "content = \"nothing\"\n", &[]);
        assert!(m.install(&no_content).unwrap_err().to_string().contains("missing"));
        let txt = t.path().join("readme.txt");
        std::fs::write(&txt, "hi").unwrap();
        assert!(m.install(&txt).is_err());
        assert!(m.install(&t.path().join("nope")).is_err());
        assert!(m.installed().is_empty());
    }

    #[test]
    fn reinstall_replaces_and_versions_live_side_by_side() {
        let (t, m) = setup();
        let one = make_mod(&t.path().join("a"), "cubes", "1.0.0", "", &["a.txt"]);
        m.install(&one).unwrap();
        std::fs::write(one.join("a.txt"), "changed").unwrap();
        let again = m.install(&one).unwrap();
        assert_eq!(std::fs::read_to_string(again.dir.join("a.txt")).unwrap(), "changed");
        m.install(&make_mod(&t.path().join("b"), "cubes", "1.1.0", "", &[])).unwrap();
        let versions: Vec<String> = m.installed().iter().map(|i| i.manifest.version.to_string()).collect();
        assert_eq!(versions, ["1.0.0", "1.1.0"]);
    }

    #[test]
    fn profiles_enable_disable_order() {
        let (t, m) = setup();
        for id in ["a", "b", "c"] {
            m.install(&make_mod(t.path(), id, "1.0.0", "", &[])).unwrap();
        }
        let p = DEFAULT_PROFILE;
        for id in ["a", "b", "c"] {
            m.enable(p, id, None).unwrap();
        }
        m.move_mod(p, "c", 0).unwrap();
        m.disable(p, "a").unwrap();
        let pr = m.load_profile(p).unwrap();
        let order: Vec<(&str, bool)> = pr.mods.iter().map(|e| (e.id.as_str(), e.enabled)).collect();
        assert_eq!(order, [("c", true), ("a", false), ("b", true)]);
        // Mount order: highest priority (last in load order) first.
        let plan = m.mount_plan(None);
        assert_eq!(plan.dirs, [m.root().join("store/b/1.0.0"), m.root().join("store/c/1.0.0")]);
        assert_eq!(plan.loaded.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["c", "b"]);
        assert!(m.enable(p, "zzz", None).is_err());
        assert!(m.disable(p, "zzz").is_err());
        m.remove_from_profile(p, "a").unwrap();
        assert!(m.load_profile(p).unwrap().get("a").is_none());
    }

    #[test]
    fn enable_picks_versions() {
        let (t, m) = setup();
        m.install(&make_mod(&t.path().join("1"), "a", "1.0.0", "", &[])).unwrap();
        m.install(&make_mod(&t.path().join("2"), "a", "1.2.0", "", &[])).unwrap();
        assert_eq!(m.enable("default", "a", None).unwrap(), v("1.2.0"));
        assert_eq!(m.enable("default", "a", Some(&v("1.0.0"))).unwrap(), v("1.0.0"));
        // Enabling again keeps the pinned version.
        m.disable("default", "a").unwrap();
        assert_eq!(m.enable("default", "a", None).unwrap(), v("1.0.0"));
        assert_eq!(m.upgrade_all("default").unwrap(), [("a".to_string(), v("1.2.0"))]);
        assert!(m.enable("default", "a", Some(&v("9.0.0"))).is_err());
    }

    #[test]
    fn rollback_undoes_changes_in_order() {
        let (t, m) = setup();
        for id in ["a", "b"] {
            m.install(&make_mod(t.path(), id, "1.0.0", "", &[])).unwrap();
        }
        let p = "default";
        m.enable(p, "a", None).unwrap();
        m.enable(p, "b", None).unwrap();
        m.disable(p, "a").unwrap();
        // No-op changes don't make snapshots.
        m.disable(p, "a").unwrap();
        let snaps = m.snapshots(p);
        assert_eq!(snaps.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(), ["disable a", "enable b 1.0.0", "enable a 1.0.0"]);
        assert_eq!(snaps[0].enabled, 2);
        assert_eq!(m.rollback(p, None).unwrap().as_deref(), Some("disable a"));
        assert!(m.load_profile(p).unwrap().get("a").unwrap().enabled);
        // Straight back to before the first change.
        let first = m.snapshots(p).last().unwrap().number;
        assert_eq!(m.rollback(p, Some(first)).unwrap().as_deref(), Some("enable a 1.0.0"));
        assert!(m.load_profile(p).unwrap().mods.is_empty());
        assert!(m.snapshots(p).is_empty());
        assert_eq!(m.rollback(p, None).unwrap(), None);
        assert!(m.rollback(p, Some(99)).is_err());
    }

    #[test]
    fn snapshots_are_capped() {
        let (t, m) = setup();
        m.install(&make_mod(t.path(), "a", "1.0.0", "", &[])).unwrap();
        for _ in 0..(MAX_SNAPSHOTS / 2 + 3) {
            m.enable("default", "a", None).unwrap();
            m.disable("default", "a").unwrap();
        }
        assert_eq!(m.snapshots("default").len(), MAX_SNAPSHOTS);
    }

    #[test]
    fn several_profiles() {
        let (t, m) = setup();
        m.install(&make_mod(t.path(), "a", "1.0.0", "", &[])).unwrap();
        m.enable("default", "a", None).unwrap();
        m.create_profile("speedrun", None).unwrap();
        m.create_profile("copy", Some("default")).unwrap();
        assert!(m.create_profile("copy", None).is_err());
        assert!(m.create_profile("Bad Name", None).is_err());
        assert_eq!(m.profiles(), ["copy", "default", "speedrun"]);
        assert_eq!(m.load_profile("copy").unwrap().enabled().count(), 1);
        assert_eq!(m.active_profile(), "default");
        m.set_active_profile("speedrun").unwrap();
        assert_eq!(m.active_profile(), "speedrun");
        assert!(m.mount_plan(None).dirs.is_empty());
        assert!(m.set_active_profile("nope").is_err());
        m.delete_profile("speedrun").unwrap();
        assert_eq!(m.active_profile(), "default");
        assert!(m.delete_profile("default").is_err());
    }

    #[test]
    fn uninstall_and_clean() {
        let (t, m) = setup();
        m.install(&make_mod(&t.path().join("1"), "a", "1.0.0", "", &[])).unwrap();
        m.install(&make_mod(&t.path().join("2"), "a", "2.0.0", "", &[])).unwrap();
        m.install(&make_mod(t.path(), "b", "1.0.0", "", &[])).unwrap();
        m.enable("default", "a", Some(&v("2.0.0"))).unwrap();
        assert!(m.uninstall("a", &v("2.0.0")).unwrap_err().to_string().contains("enabled in profile(s) default"));
        // Nothing refers to a 1.0.0 or b: cleaning removes them.
        let mut removed = m.clean().unwrap();
        removed.sort();
        assert_eq!(removed, [("a".to_string(), v("1.0.0")), ("b".to_string(), v("1.0.0"))]);
        m.disable("default", "a").unwrap();
        m.uninstall("a", &v("2.0.0")).unwrap();
        assert!(m.installed().is_empty());
        assert!(m.load_profile("default").unwrap().mods.is_empty());
        assert!(!m.root().join("store/a").exists());
    }

    #[test]
    fn mount_plan_skips_broken_mods_and_warns() {
        let (t, m) = setup();
        m.install(&make_mod(t.path(), "lib", "1.0.0", "", &["materials/a.vmt"])).unwrap();
        m.install(&make_mod(t.path(), "app", "1.0.0", "[dependencies]\nlib = \"^1\"\n", &["materials/a.vmt"])).unwrap();
        m.install(&make_mod(t.path(), "needy", "1.0.0", "[dependencies]\nmissing = \"*\"\n", &[])).unwrap();
        m.install(&make_mod(t.path(), "skin", "1.0.0", "", &["materials/a.vmt"])).unwrap();
        for id in ["app", "lib", "needy", "skin"] {
            m.enable("default", id, None).unwrap();
        }
        let plan = m.mount_plan(Some(&v("0.1.0")));
        assert_eq!(plan.loaded.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["lib", "app", "skin"]);
        assert_eq!(plan.dirs.first(), Some(&m.root().join("store/skin/1.0.0")));
        let msgs: Vec<String> = plan.issues.iter().map(|i| i.to_string()).collect();
        assert!(msgs[0].starts_with("error: needy: needs missing"), "{msgs:?}");
        assert!(msgs.iter().any(|s| s.contains("app: was ordered before its dependency lib")), "{msgs:?}");
        // skin overrides app's file (no dependency between them): warned. app over lib: expected.
        assert!(msgs.iter().any(|s| s.contains("skin: replaces 1 file(s) of app")), "{msgs:?}");
        assert!(!msgs.iter().any(|s| s.contains("app: replaces")), "{msgs:?}");
    }

    #[test]
    fn enabled_but_deleted_from_store() {
        let (t, m) = setup();
        let a = m.install(&make_mod(t.path(), "a", "1.0.0", "", &[])).unwrap();
        m.enable("default", "a", None).unwrap();
        std::fs::remove_dir_all(&a.dir).unwrap();
        let plan = m.mount_plan(None);
        assert!(plan.dirs.is_empty());
        assert!(plan.issues[0].message.contains("not installed"));
    }

    #[test]
    fn no_mod_folder_means_nothing_to_mount() {
        let t = TempDir::new("none");
        let plan = enabled_content_dirs(&t.path().join("mods"), None);
        assert!(plan.dirs.is_empty() && plan.issues.is_empty());
        assert!(!t.path().join("mods").exists());
    }
}
