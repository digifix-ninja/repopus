//! Where on disk a copy goes.
//!
//! Inside the repository's folder (default `<main folder>/<owner>`):
//!
//! | keep as | replace       | keep versions                                  |
//! |---------|---------------|------------------------------------------------|
//! | zip     | `<name>.zip`  | `<name> versions/<name>_<date>_<sha7>.zip`     |
//! | folder  | `<name>/`     | `<name> versions/<date>_<sha7>/`               |
//!
//! "Keep versions" has its own folder, so switching modes never overwrites anything.

use crate::model::{Repo, StorageFormat, UpdateMode};
use chrono::{DateTime, Local, Utc};
use std::path::{Path, PathBuf};

/// The folder a copy lands in, and its exact path.
pub struct Placement {
    /// Holds the copy (and Repopus's temporary download folder while updating).
    pub dir: PathBuf,
    pub target: PathBuf,
}

impl Placement {
    /// The folder that holds a repository's copies.
    pub fn root(repo: &Repo, base: &Path) -> PathBuf {
        repo.settings
            .destination
            .clone()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| base.join(&repo.owner))
    }

    /// Where a copy of commit `sha`, made at `when`, goes with the current settings.
    pub fn for_copy(repo: &Repo, base: &Path, sha: &str, when: DateTime<Utc>) -> Self {
        let root = Self::root(repo, base);
        let name = &repo.name;
        let short = &sha[..sha.len().min(7)];
        let date = when.with_timezone(&Local).format("%Y-%m-%d");
        let versions = root.join(format!("{name} versions"));

        match (repo.settings.format, repo.settings.mode) {
            (StorageFormat::Zip, UpdateMode::Replace) => Self {
                target: root.join(format!("{name}.zip")),
                dir: root,
            },
            (StorageFormat::Folder, UpdateMode::Replace) => Self {
                target: root.join(name),
                dir: root,
            },
            (StorageFormat::Zip, UpdateMode::KeepVersions) => Self {
                target: versions.join(format!("{name}_{date}_{short}.zip")),
                dir: versions,
            },
            (StorageFormat::Folder, UpdateMode::KeepVersions) => Self {
                target: versions.join(format!("{date}_{short}")),
                dir: versions,
            },
        }
    }
}
