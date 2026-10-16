//! Zip files downloaded from GitHub.
//!
//! GitHub's archives contain one top folder named `<owner>-<repo>-<sha>/`. When unpacking,
//! that folder is dropped, so the copy looks like the repository itself.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// A zip file on disk.
pub struct ZipFile {
    path: PathBuf,
}

impl ZipFile {
    /// Wraps the zip at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Checks that the archive is complete. Its table of contents sits at the very end
    /// of the file, so a cut-off download fails here.
    pub fn validate(&self) -> Result<usize, String> {
        let archive = self.open()?;
        if archive.len() == 0 {
            return Err("The downloaded archive is empty".into());
        }
        Ok(archive.len())
    }

    /// Unpacks into `dest` without the top folder. Checksums are verified while reading.
    pub fn extract_stripped(&self, dest: &Path) -> Result<(), String> {
        let mut archive = self.open()?;
        fs::create_dir_all(dest).map_err(|e| format!("Couldn't create {}: {e}", dest.display()))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("Broken entry in archive: {e}"))?;
            // `enclosed_name` refuses unsafe paths such as "../x".
            let Some(inner) = entry.enclosed_name() else { continue };
            let Some(rel) = Self::without_top_folder(&inner) else { continue };
            let rel = Self::portable(&rel);
            let out = dest.join(&rel);

            if entry.is_dir() {
                fs::create_dir_all(&out).map_err(|e| format!("Couldn't create folder: {e}"))?;
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Couldn't create folder: {e}"))?;
            }

            let mode = entry.unix_mode();
            if mode.is_some_and(Self::is_symlink_mode) {
                let mut target = String::new();
                io::Read::read_to_string(&mut entry, &mut target)
                    .map_err(|e| format!("Couldn't read link: {e}"))?;
                Self::write_link(&rel, &out, &target)?;
                continue;
            }

            let mut file = fs::File::create(&out)
                .map_err(|e| format!("Couldn't write {}: {e}", out.display()))?;
            io::copy(&mut entry, &mut file).map_err(|e| format!("Archive damaged: {e}"))?;
            Self::apply_mode(&out, mode);
        }
        Ok(())
    }

    // ---------------------------------------------------------------- helpers

    /// Opens the archive for reading.
    fn open(&self) -> Result<zip::ZipArchive<fs::File>, String> {
        let file = fs::File::open(&self.path).map_err(|e| format!("Couldn't open download: {e}"))?;
        zip::ZipArchive::new(file)
            .map_err(|e| format!("The download is not a complete zip archive ({e})"))
    }

    /// "owner-repo-sha/src/main.rs" → "src/main.rs" (`None` for the top folder itself).
    fn without_top_folder(p: &Path) -> Option<PathBuf> {
        let mut parts = p.components();
        parts.next()?;
        let rest: PathBuf = parts.collect();
        (!rest.as_os_str().is_empty()).then_some(rest)
    }

    /// Makes a path valid on this system. On Windows, characters and names it forbids
    /// (`<>:"|?*`, "CON", "NUL", trailing dots…) are replaced, so one odd file can't stop
    /// a whole repository from unpacking. Elsewhere the path stays as it is.
    fn portable(rel: &Path) -> PathBuf {
        if !cfg!(windows) {
            return rel.to_path_buf();
        }
        rel.components()
            .map(|part| Self::windows_safe_name(&part.as_os_str().to_string_lossy()))
            .collect()
    }

    /// One file or folder name made acceptable to Windows.
    fn windows_safe_name(name: &str) -> String {
        const RESERVED: [&str; 22] = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        let mut safe: String = name
            .chars()
            .map(|c| if "<>:\"|?*".contains(c) || (c as u32) < 32 { '_' } else { c })
            .collect();
        if safe.ends_with('.') || safe.ends_with(' ') {
            safe.pop();
            safe.push('_');
        }
        let stem = safe.split('.').next().unwrap_or("").to_ascii_uppercase();
        if RESERVED.contains(&stem.as_str()) {
            safe.insert(0, '_');
        }
        if safe.is_empty() {
            safe.push('_');
        }
        safe
    }

    /// Whether unix permission bits describe a symbolic link.
    fn is_symlink_mode(mode: u32) -> bool {
        mode & 0o170000 == 0o120000
    }

    /// Links that stay inside the copy become real links; others become a small text
    /// file holding the target, so nothing can point outside the copy.
    fn write_link(rel: &Path, out: &Path, target: &str) -> Result<(), String> {
        #[cfg(unix)]
        {
            if Self::link_stays_inside(rel, Path::new(target)) {
                return std::os::unix::fs::symlink(target, out)
                    .map_err(|e| format!("Couldn't create link: {e}"));
            }
        }
        let _ = rel;
        fs::write(out, target.as_bytes()).map_err(|e| format!("Couldn't write file: {e}"))
    }

    /// Whether a relative link target stays within the copy.
    fn link_stays_inside(link: &Path, target: &Path) -> bool {
        if target.is_absolute() {
            return false;
        }
        let mut depth = link.components().count() as i64 - 1;
        for part in target.components() {
            match part {
                Component::ParentDir => depth -= 1,
                Component::Normal(_) => depth += 1,
                Component::CurDir => {}
                _ => return false,
            }
            if depth < 0 {
                return false;
            }
        }
        true
    }

    /// Keeps file permissions (e.g. executable scripts).
    fn apply_mode(out: &Path, mode: Option<u32>) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(perm) = mode.map(|m| m & 0o777).filter(|p| *p != 0) {
                let _ = fs::set_permissions(out, fs::Permissions::from_mode(perm | 0o600));
            }
        }
        let _ = (out, mode);
    }
}
