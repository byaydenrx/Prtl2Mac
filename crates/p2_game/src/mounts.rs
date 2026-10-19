//! What the game mounts besides the install: the enabled mods of the active
//! mod profile *ahead of* the game's own folders (so their files win, and no
//! game file is ever replaced), and the community maps *behind* them.
//! Decided once at startup, before the window opens. With no mods enabled
//! and no community maps, the game mounts exactly what it did before.
use std::path::PathBuf;

use bevy::prelude::*;
use source_assets::config::GameConfig;
use source_assets::GameFs;

use crate::community::{self, CommunityMaps};
use crate::settings::{data_dir, Settings};

/// What was mounted, for the console's `mods` command and the menus.
#[derive(Resource, Clone, Debug, Default)]
pub struct Mounts {
    /// The mod plan the game used (profile, folders, problems).
    pub mods: p2_mods::MountPlan,
    /// The folder of community map links, when any were mounted.
    pub community: Option<PathBuf>,
    /// Logged once logging is up (mounting happens before the app exists).
    notes: Vec<(bool, String)>,
}

/// Logs what was mounted and keeps it as resources.
pub struct MountsPlugin {
    pub mounts: Mounts,
    pub community: CommunityMaps,
}

impl Plugin for MountsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.mounts.clone()).insert_resource(self.community.clone()).add_systems(Startup, log_mounts);
    }
}

fn log_mounts(mounts: Res<Mounts>) {
    for (warning, n) in &mounts.notes {
        if *warning {
            warn!("portal2: {n}");
        } else {
            info!("portal2: {n}");
        }
    }
}

/// The mod store (`<data>/mods`, see `p2_mods`).
pub fn mods_root() -> PathBuf {
    data_dir().join("mods")
}

/// Mount the enabled mods and community maps around `game`. Returns the
/// filesystem to use, the config for `SourceMapPlugin` (`None` keeps its
/// default when nothing extra is mounted), what was mounted and the
/// community maps found.
pub fn mount(game: GameFs) -> (GameFs, Option<GameConfig>, Mounts, CommunityMaps) {
    let engine = p2_mods::semver::Version::parse(env!("CARGO_PKG_VERSION")).ok();
    let plan = p2_mods::enabled_content_dirs(&mods_root(), engine.as_ref());
    let mut notes: Vec<(bool, String)> = plan.issues.iter().map(|i| (true, format!("mods: {i}"))).collect();
    let settings = Settings::load();
    let folders = community::folders(&data_dir(), settings.tools.community_maps_dir.as_deref());
    let mut maps = community::discover(&folders);
    // Scripted test runs stay away from the player's data folder.
    let community_root = if crate::settings::may_write_data() { community::prepare_mount(&mut maps, &data_dir().join(".mount/community")) } else { None };
    let community = CommunityMaps { maps, folders };
    if plan.dirs.is_empty() && community_root.is_none() {
        // The maps mount the install that was found (Steam library or
        // depot), not only PORTAL2_DIR: a Finder-launched app has no env.
        let config = game.config.clone();
        return (game, Some(config), Mounts { mods: plan, community: None, notes }, community);
    }
    let mut config = game.config.clone();
    for d in plan.dirs.iter().rev() {
        config.search_paths.insert(0, d.clone());
    }
    if let Some(c) = &community_root {
        config.search_paths.push(c.clone());
    }
    notes.push((false, format!("mounted {} mod folder(s) (profile {}) and {} community map(s)", plan.dirs.len(), plan.profile, community.maps.iter().filter(|m| m.mounted).count())));
    match GameFs::mount(config.clone()) {
        Ok(fs) => (fs, Some(config), Mounts { mods: plan, community: community_root, notes }, community),
        Err(e) => {
            // Never let a mod stop the game from starting.
            notes.push((true, format!("can't mount mods/community maps ({e}); starting without them")));
            let mut community = community;
            community.maps.iter_mut().for_each(|m| m.mounted = false);
            let config = game.config.clone();
            (game, Some(config), Mounts { mods: p2_mods::MountPlan { profile: plan.profile, issues: plan.issues, ..Default::default() }, community: None, notes }, community)
        }
    }
}
