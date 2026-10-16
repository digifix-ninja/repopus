//! The state file: one JSON file holding everything Repopus remembers.

use crate::model::{Store, SCHEMA_VERSION};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct StateFile {
    path: PathBuf,
}

impl StateFile {
    /// Points at the state file (it doesn't have to exist yet).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Reads the state. A missing file gives a fresh start; an unreadable one is
    /// renamed `*.corrupt-<time>.json` and kept, never overwritten.
    pub fn load(&self) -> Result<Store, String> {
        match fs::read(&self.path) {
            Ok(bytes) => match serde_json::from_slice::<Store>(&bytes) {
                Ok(mut store) => {
                    store.schema = SCHEMA_VERSION;
                    Ok(store)
                }
                Err(e) => {
                    self.set_aside(&e.to_string());
                    Ok(Self::fresh())
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::fresh()),
            Err(e) => Err(format!("Couldn't read {}: {e}", self.path.display())),
        }
    }

    /// Writes a temporary file and renames it over the old one, so a crash mid-write
    /// can't leave a half-written state file.
    pub fn save(&self, store: &Store) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
        }
        let json = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| format!("Couldn't write state: {e}"))?;
            f.write_all(&json).map_err(|e| format!("Couldn't write state: {e}"))?;
            f.sync_all().ok();
        }
        fs::rename(&tmp, &self.path).map_err(|e| format!("Couldn't save state: {e}"))
    }

    /// Where the file lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// An empty state for a first launch.
    fn fresh() -> Store {
        Store {
            schema: SCHEMA_VERSION,
            ..Store::default()
        }
    }

    /// Renames an unreadable state file so it isn't lost.
    fn set_aside(&self, reason: &str) {
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let aside = self.path.with_extension(format!("corrupt-{stamp}.json"));
        let _ = fs::rename(&self.path, &aside);
        log::error!("state file unreadable ({reason}); kept as {}", aside.display());
    }
}
