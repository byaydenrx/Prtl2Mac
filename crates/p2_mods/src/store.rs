//! The managed store: every installed mod version in its own folder,
//! `store/<id>/<version>/`, copied in from a folder or unpacked from a zip.
//! The game reads mods from here; the player's originals are never touched.
use std::path::{Path, PathBuf};

use semver::Version;

use crate::manifest::{Manifest, MANIFEST};
use crate::Error;

/// One installed version of a mod.
#[derive(Clone, Debug, PartialEq)]
pub struct InstalledMod {
    pub manifest: Manifest,
    /// `store/<id>/<version>`.
    pub dir: PathBuf,
}

impl InstalledMod {
    /// The folder to mount ahead of the base game.
    pub fn content_dir(&self) -> PathBuf {
        if self.manifest.content == "." {
            self.dir.clone()
        } else {
            self.dir.join(&self.manifest.content)
        }
    }

    /// Every file in the content folder, as lowercase `/` paths (the game's
    /// file lookups ignore case).
    pub fn files(&self) -> Vec<String> {
        let root = self.content_dir();
        let mut out = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(&root) {
                    out.push(rel.to_string_lossy().replace('\\', "/").to_ascii_lowercase());
                }
            }
        }
        out.sort();
        out
    }
}

/// Copy a folder tree (files and folders; symlinks are followed).
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<u64, Error> {
    std::fs::create_dir_all(to)?;
    let mut n = 0;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let src = e.path();
        let dst = to.join(e.file_name());
        if src.is_dir() {
            n += copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|err| Error::Io(format!("{}: {err}", src.display())))?;
            n += 1;
        }
    }
    Ok(n)
}

/// The folder holding `mod.toml`: `dir` itself, or its one subfolder that
/// has it (archives often wrap everything in a top folder).
pub(crate) fn find_mod_root(dir: &Path) -> Result<PathBuf, Error> {
    if dir.join(MANIFEST).is_file() {
        return Ok(dir.to_path_buf());
    }
    let subs: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.join(MANIFEST).is_file()).collect();
    match subs.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(Error::Invalid(format!("no {MANIFEST} in {}", dir.display()))),
        _ => Err(Error::Invalid(format!("{} holds several mods; install them one at a time", dir.display()))),
    }
}

pub(crate) struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn dir(&self, id: &str, version: &Version) -> PathBuf {
        self.root.join(id).join(version.to_string())
    }

    /// Every installed version, sorted by id then version. Folders whose
    /// manifest doesn't match their place are left out.
    pub fn list(&self) -> Vec<InstalledMod> {
        let mut out = Vec::new();
        let Ok(ids) = std::fs::read_dir(&self.root) else { return out };
        for id_dir in ids.flatten() {
            let Ok(versions) = std::fs::read_dir(id_dir.path()) else { continue };
            for v in versions.flatten() {
                let dir = v.path();
                let name = v.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let Ok(manifest) = Manifest::read(&dir) else { continue };
                if manifest.id == id_dir.file_name().to_string_lossy() && manifest.version.to_string() == name {
                    out.push(InstalledMod { manifest, dir });
                }
            }
        }
        out.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id).then(a.manifest.version.cmp(&b.manifest.version)));
        out
    }

    pub fn get(&self, id: &str, version: &Version) -> Option<InstalledMod> {
        let dir = self.dir(id, version);
        let manifest = Manifest::read(&dir).ok()?;
        (manifest.id == id && &manifest.version == version).then_some(InstalledMod { manifest, dir })
    }

    pub fn newest(&self, id: &str) -> Option<InstalledMod> {
        self.list().into_iter().filter(|m| m.manifest.id == id).max_by(|a, b| a.manifest.version.cmp(&b.manifest.version))
    }

    /// Install from a mod folder or a `.zip`. The same id and version
    /// installed again replaces the old copy.
    pub fn install(&self, src: &Path, tmp_root: &Path) -> Result<InstalledMod, Error> {
        std::fs::create_dir_all(tmp_root)?;
        let stamp = format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        let unpacked = tmp_root.join(format!("unpack-{stamp}"));
        let result = (|| {
            let source = if src.is_file() {
                if !src.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")) {
                    return Err(Error::Invalid(format!("{}: install a mod folder or a .zip", src.display())));
                }
                std::fs::create_dir_all(&unpacked)?;
                crate::archive::extract(src, &unpacked)?;
                find_mod_root(&unpacked)?
            } else if src.is_dir() {
                find_mod_root(src)?
            } else {
                return Err(Error::Io(format!("{} not found", src.display())));
            };
            let manifest = Manifest::read(&source)?;
            let content = source.join(&manifest.content);
            if !content.is_dir() {
                return Err(Error::Invalid(format!("{}: content folder {:?} is missing", manifest.id, manifest.content)));
            }
            let dest = self.dir(&manifest.id, &manifest.version);
            // Copy next to the final place, then swap it in.
            let incoming = self.root.join(&manifest.id).join(format!(".incoming-{stamp}"));
            copy_dir(&source, &incoming)?;
            if dest.exists() {
                std::fs::remove_dir_all(&dest)?;
            }
            std::fs::rename(&incoming, &dest)?;
            Ok(InstalledMod { manifest, dir: dest })
        })();
        let _ = std::fs::remove_dir_all(&unpacked);
        result
    }

    pub fn remove(&self, id: &str, version: &Version) -> Result<(), Error> {
        let dir = self.dir(id, version);
        if !dir.is_dir() {
            return Err(Error::Invalid(format!("{id} {version} is not installed")));
        }
        std::fs::remove_dir_all(&dir)?;
        // The id's folder goes when its last version does.
        let parent = self.root.join(id);
        if std::fs::read_dir(&parent).map(|mut r| r.next().is_none()).unwrap_or(false) {
            let _ = std::fs::remove_dir(&parent);
        }
        Ok(())
    }
}
