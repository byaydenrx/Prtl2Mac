//! Community maps: `.bsp` files in `<data dir>/maps/` (on macOS
//! `~/Library/Application Support/Portal2Rust/maps/`) and in the optional
//! folder set by `community_maps_dir` in settings.cfg. Chapter select lists
//! them on its Community page; they start through the normal [`LoadLevel`].
//!
//! The maps found at startup are mounted behind the game's own files as
//! `maps/<name>.bsp` (a folder of links, see [`prepare_mount`]), so loading
//! them by name works everywhere a campaign map does: restarting, dying,
//! Continue, trigger bounds. A map added while the game runs starts from its
//! path instead (until the next launch).
//!
//! [`LoadLevel`]: crate::level::LoadLevel
use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// Folders deeper than this inside a maps folder aren't searched.
const MAX_DEPTH: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunityMap {
    /// The file name without `.bsp`, lowercased (what the game calls it).
    pub name: String,
    pub path: PathBuf,
    /// Mounted as `maps/<name>.bsp` (so it loads by name).
    pub mounted: bool,
}

impl CommunityMap {
    /// What to put in `LoadLevel::map`.
    pub fn launch_name(&self) -> String {
        if self.mounted {
            self.name.clone()
        } else {
            self.path.to_string_lossy().into_owned()
        }
    }
}

/// The community maps and where they were looked for.
#[derive(Resource, Clone, Debug, Default)]
pub struct CommunityMaps {
    pub maps: Vec<CommunityMap>,
    pub folders: Vec<PathBuf>,
}

impl CommunityMaps {
    /// Look again (new files since startup start from their path).
    pub fn rescan(&mut self) {
        let mounted: Vec<String> = self.maps.iter().filter(|m| m.mounted).map(|m| m.name.clone()).collect();
        self.maps = discover(&self.folders);
        for m in &mut self.maps {
            m.mounted = mounted.contains(&m.name);
        }
    }
}

/// The folders to look in: `<data>/maps`, then the settings' extra folder.
pub fn folders(data_dir: &Path, extra: Option<&Path>) -> Vec<PathBuf> {
    let mut out = vec![data_dir.join("maps")];
    if let Some(e) = extra {
        if !out.iter().any(|f| f == e) {
            out.push(e.to_path_buf());
        }
    }
    out
}

/// Every `.bsp` in `folders` (and their subfolders), sorted by name. When
/// two files have the same name the one from the earlier folder wins.
pub fn discover(folders: &[PathBuf]) -> Vec<CommunityMap> {
    let mut out: Vec<CommunityMap> = Vec::new();
    for root in folders {
        let mut found = Vec::new();
        let mut stack = vec![(root.clone(), 0)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let hidden = e.file_name().to_string_lossy().starts_with('.');
                if p.is_dir() {
                    if depth < MAX_DEPTH && !hidden {
                        stack.push((p, depth + 1));
                    }
                } else if !hidden && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bsp")) {
                    if let Some(stem) = p.file_stem() {
                        found.push(CommunityMap { name: stem.to_string_lossy().to_ascii_lowercase(), path: p.clone(), mounted: false });
                    }
                }
            }
        }
        found.sort_by(|a, b| a.path.cmp(&b.path));
        for m in found {
            if !out.iter().any(|o| o.name == m.name) {
                out.push(m);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Make `<mount_root>/maps/<name>.bsp` links to `maps` (old links go) and
/// return the folder to add as a search path, or `None` when there's
/// nothing to mount or links can't be made here.
pub fn prepare_mount(maps: &mut [CommunityMap], mount_root: &Path) -> Option<PathBuf> {
    let dir = mount_root.join("maps");
    let _ = std::fs::remove_dir_all(&dir);
    if maps.is_empty() {
        return None;
    }
    std::fs::create_dir_all(&dir).ok()?;
    let mut any = false;
    for m in maps.iter_mut() {
        // Names the filesystem would see differently are left to load by path.
        if m.name.contains(['/', '\\']) || m.name.is_empty() {
            continue;
        }
        let link = dir.join(format!("{}.bsp", m.name));
        if make_link(&m.path, &link).is_ok() {
            m.mounted = true;
            any = true;
        }
    }
    any.then(|| mount_root.to_path_buf())
}

#[cfg(unix)]
fn make_link(target: &Path, link: &Path) -> std::io::Result<()> {
    let target = std::fs::canonicalize(target)?;
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::fs::hard_link(target, link)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("p2-community-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"VBSP").unwrap();
    }

    #[test]
    fn finds_bsps_in_both_folders() {
        let t = temp("find");
        let (a, b) = (t.join("data/maps"), t.join("extra"));
        touch(&a.join("Cool_Map.BSP"));
        touch(&a.join("pack/level2.bsp"));
        touch(&a.join("readme.txt"));
        touch(&a.join(".hidden.bsp"));
        touch(&a.join("a/b/c/d/too_deep.bsp"));
        touch(&b.join("cool_map.bsp"));
        touch(&b.join("another.bsp"));
        let found = discover(&folders(&t.join("data"), Some(&b)));
        let names: Vec<&str> = found.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["another", "cool_map", "level2"]);
        // The data folder's copy wins over the extra folder's.
        assert_eq!(found[1].path, a.join("Cool_Map.BSP"));
        assert!(found.iter().all(|m| !m.mounted));
        assert_eq!(found[0].launch_name(), b.join("another.bsp").to_string_lossy());
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn missing_folders_find_nothing() {
        let t = temp("missing");
        assert!(discover(&folders(&t.join("nope"), None)).is_empty());
        assert_eq!(folders(&t, Some(&t.join("maps"))).len(), 1);
        let _ = std::fs::remove_dir_all(&t);
    }

    #[cfg(unix)]
    #[test]
    fn mounts_as_links() {
        let t = temp("mount");
        touch(&t.join("maps/My_Map.bsp"));
        let mut maps = discover(&[t.join("maps")]);
        let root = prepare_mount(&mut maps, &t.join(".mount")).unwrap();
        let link = root.join("maps/my_map.bsp");
        assert_eq!(std::fs::read(&link).unwrap(), b"VBSP");
        assert!(maps[0].mounted);
        assert_eq!(maps[0].launch_name(), "my_map");
        // A rescan keeps it mounted and finds new ones unmounted.
        touch(&t.join("maps/new.bsp"));
        let mut cm = CommunityMaps { maps, folders: vec![t.join("maps")] };
        cm.rescan();
        assert_eq!(cm.maps.iter().map(|m| (m.name.as_str(), m.mounted)).collect::<Vec<_>>(), [("my_map", true), ("new", false)]);
        // Mounting nothing removes the old links.
        assert!(prepare_mount(&mut [], &t.join(".mount")).is_none());
        assert!(!link.exists());
        let _ = std::fs::remove_dir_all(&t);
    }
}
