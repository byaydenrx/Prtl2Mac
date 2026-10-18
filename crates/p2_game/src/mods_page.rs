//! The Options > Mods page's data and actions: the installed mods with
//! their state in the active profile, the profile itself, rollback, and
//! the problems `p2_mods` finds. Changes are written straight to the mod
//! folder and take effect the next time the game starts (mods are mounted
//! at startup, see `mounts.rs`). Installing stays with the `p2mods` tool.
use bevy::prelude::*;

use crate::mounts::mods_root;

/// One installed mod as the page shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ModsPage {
    pub profile: String,
    pub profiles: Vec<String>,
    pub mods: Vec<ModRow>,
    /// (is an error, text), errors first.
    pub issues: Vec<(bool, String)>,
    /// What rollback would undo.
    pub undo: Option<String>,
    /// The last action's result, shown on the page.
    pub note: String,
    /// The profile differs from what the game started with.
    pub restart_needed: bool,
}

fn engine() -> Option<p2_mods::semver::Version> {
    p2_mods::semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()
}

impl ModsPage {
    /// Read the mod folder again. `started` is what the game mounted.
    pub fn refresh(&mut self, started: &p2_mods::MountPlan) {
        let note = std::mem::take(&mut self.note);
        *self = ModsPage { note, ..Default::default() };
        // Don't create the mod folder just by looking at the page.
        if !mods_root().join("profiles").is_dir() {
            self.profile = p2_mods::DEFAULT_PROFILE.into();
            self.profiles = vec![self.profile.clone()];
            return;
        }
        let Ok(m) = p2_mods::ModManager::open(mods_root()) else { return };
        self.profile = m.active_profile();
        self.profiles = m.profiles();
        let p = m.load_profile(&self.profile).unwrap_or_default();
        // One row per mod id: the version the profile pins, else the newest.
        for i in m.installed().into_iter().rev() {
            if self.mods.iter().any(|r| r.id == i.manifest.id) {
                continue;
            }
            let pinned = p.get(&i.manifest.id);
            let (version, enabled) = match pinned {
                Some(e) => (e.version.to_string(), e.enabled),
                None => (i.manifest.version.to_string(), false),
            };
            self.mods.push(ModRow { id: i.manifest.id.clone(), name: i.manifest.name.clone(), version, enabled });
        }
        self.mods.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
        if let Ok((r, loaded)) = m.check(&self.profile, engine().as_ref()) {
            self.issues = r.issues.iter().map(|i| (i.severity == p2_mods::Severity::Error, format!("{}: {}", i.mod_id, i.message))).collect();
            let now: Vec<(String, p2_mods::semver::Version)> = loaded.iter().map(|l| (l.manifest.id.clone(), l.manifest.version.clone())).collect();
            self.restart_needed = now != started.loaded || self.profile != started.profile;
        }
        self.undo = m.snapshots(&self.profile).first().map(|s| s.label.clone());
    }

    fn act(&mut self, started: &p2_mods::MountPlan, f: impl FnOnce(&p2_mods::ModManager, &str) -> Result<String, p2_mods::Error>) {
        if !crate::settings::may_write_data() {
            self.note = "Scripted test runs don't change mods.".into();
            return;
        }
        self.note = match p2_mods::ModManager::open(mods_root()) {
            Ok(m) => {
                let profile = m.active_profile();
                f(&m, &profile).unwrap_or_else(|e| e.to_string())
            }
            Err(e) => e.to_string(),
        };
        self.refresh(started);
    }

    /// Turn the `i`th row on or off.
    pub fn toggle(&mut self, i: usize, started: &p2_mods::MountPlan) {
        let Some(row) = self.mods.get(i).cloned() else { return };
        self.act(started, |m, p| {
            if row.enabled {
                m.disable(p, &row.id).map(|_| format!("{} off", row.name))
            } else {
                m.enable(p, &row.id, None).map(|v| format!("{} {v} on", row.name))
            }
        });
    }

    /// Make the profile `d` steps along the list active.
    pub fn step_profile(&mut self, d: i32, started: &p2_mods::MountPlan) {
        if self.profiles.len() < 2 {
            return;
        }
        let i = self.profiles.iter().position(|p| *p == self.profile).unwrap_or(0) as i32;
        let next = self.profiles[(i + d).clamp(0, self.profiles.len() as i32 - 1) as usize].clone();
        self.act(started, |m, _| m.set_active_profile(&next).map(|_| format!("profile {next}")));
    }

    pub fn rollback(&mut self, started: &p2_mods::MountPlan) {
        self.act(started, |m, p| m.rollback(p, None).map(|l| l.map_or("Nothing to undo.".into(), |l| format!("Undid: {l}"))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_profile_and_rollback() {
        // The only test that moves the data folder.
        let dir = std::env::temp_dir().join(format!("p2-modspage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("P2_DATA_DIR", &dir);
        let started = p2_mods::MountPlan { profile: "default".into(), ..Default::default() };
        let mut page = ModsPage::default();
        page.refresh(&started);
        assert!(page.mods.is_empty() && !mods_root().exists(), "looking doesn't create the mod folder");

        let src = dir.join("src");
        std::fs::create_dir_all(src.join("materials")).unwrap();
        std::fs::write(src.join("mod.toml"), "id = \"cubes\"\nname = \"Better Cubes\"\nversion = \"1.0.0\"\n").unwrap();
        let m = p2_mods::ModManager::open(mods_root()).unwrap();
        m.install(&src).unwrap();
        m.create_profile("speedrun", None).unwrap();

        page.refresh(&started);
        assert_eq!(page.mods, [ModRow { id: "cubes".into(), name: "Better Cubes".into(), version: "1.0.0".into(), enabled: false }]);
        assert!(!page.restart_needed && page.undo.is_none());
        page.toggle(0, &started);
        assert!(page.mods[0].enabled);
        assert_eq!(page.note, "Better Cubes 1.0.0 on");
        assert!(page.restart_needed);
        assert_eq!(page.undo.as_deref(), Some("enable cubes 1.0.0"));
        page.rollback(&started);
        assert!(!page.mods[0].enabled && !page.restart_needed);
        assert_eq!(page.note, "Undid: enable cubes 1.0.0");
        page.step_profile(1, &started);
        assert_eq!(page.profile, "speedrun");
        assert!(page.restart_needed);
        std::env::remove_var("P2_DATA_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
