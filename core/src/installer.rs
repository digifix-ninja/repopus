//! Putting a finished, verified copy in place.
//!
//! * Nothing there yet → the copy is moved in.
//! * Repopus's own previous copy there → swapped; if the swap fails, the old one is put back.
//! * Something Repopus didn't create there → renamed "… (kept <date>)", never deleted.

use std::fs;
use std::path::{Path, PathBuf};

pub struct Installer<'a> {
    /// The new copy, inside the temporary folder.
    staged: &'a Path,
    /// Where it should end up.
    target: &'a Path,
    /// Paths Repopus wrote earlier for this repository.
    own_paths: &'a [PathBuf],
    /// The temporary folder (same disk as the target, so moves are instant).
    scratch: &'a Path,
}

impl<'a> Installer<'a> {
    /// Prepares an installation.
    pub fn new(staged: &'a Path, target: &'a Path, own_paths: &'a [PathBuf], scratch: &'a Path) -> Self {
        Self {
            staged,
            target,
            own_paths,
            scratch,
        }
    }

    /// Moves the new copy into place and returns its final path.
    pub fn install(&self) -> Result<PathBuf, String> {
        if fs::symlink_metadata(self.target).is_err() {
            self.move_in()?;
        } else if self.own_paths.iter().any(|p| p == self.target) {
            self.swap_with_own_copy()?;
        } else {
            self.keep_foreign_item()?;
            self.move_in()?;
        }
        Ok(self.target.to_path_buf())
    }

    /// Moves the staged copy to the target.
    fn move_in(&self) -> Result<(), String> {
        fs::rename(self.staged, self.target)
            .map_err(|e| format!("Couldn't move the new copy into place: {e}"))
    }

    /// Replaces Repopus's previous copy. The old one goes into the temporary folder,
    /// which is cleaned up afterwards.
    fn swap_with_own_copy(&self) -> Result<(), String> {
        let old = self.scratch.join("previous-copy");
        fs::rename(self.target, &old)
            .map_err(|e| format!("Couldn't replace the old copy (is a file in it open?): {e}"))?;
        if let Err(e) = self.move_in() {
            let _ = fs::rename(&old, self.target);
            return Err(e);
        }
        Ok(())
    }

    /// Renames something unknown that's in the way, so it's kept.
    fn keep_foreign_item(&self) -> Result<(), String> {
        let aside = Self::free_name_next_to(self.target);
        fs::rename(self.target, &aside).map_err(|e| {
            format!("“{}” already exists and couldn't be moved aside: {e}", self.target.display())
        })?;
        log::info!("kept existing {} as {}", self.target.display(), aside.display());
        Ok(())
    }

    /// "name (kept 2026-10-16).zip", or "… 2)", "… 3)" if that's taken too.
    fn free_name_next_to(target: &Path) -> PathBuf {
        let parent = target.parent().unwrap_or(Path::new("."));
        let file = target
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (stem, ext) = match (target.is_file(), file.rfind('.')) {
            (true, Some(i)) if i > 0 => (file[..i].to_string(), file[i..].to_string()),
            _ => (file.clone(), String::new()),
        };
        let date = chrono::Local::now().format("%Y-%m-%d");
        let mut n = 1;
        loop {
            let suffix = if n == 1 {
                format!(" (kept {date})")
            } else {
                format!(" (kept {date} {n})")
            };
            let candidate = parent.join(format!("{stem}{suffix}{ext}"));
            if fs::symlink_metadata(&candidate).is_err() {
                return candidate;
            }
            n += 1;
        }
    }
}
