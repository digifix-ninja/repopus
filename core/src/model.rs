//! Data types: what Repopus stores on disk, and what the window is sent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ==================================================================== settings

/// How a downloaded version is kept on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageFormat {
    /// The zip archive exactly as GitHub produces it.
    Zip,
    /// The archive unpacked into a plain folder.
    Folder,
}

/// What happens to the previous copy when a newer version arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    /// One copy only; the newer version takes its place.
    Replace,
    /// Every version is saved next to the earlier ones.
    KeepVersions,
}

/// Light or dark window. "System" follows the operating system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// Settings of one repository (also used as defaults for new ones).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RepoSettings {
    /// Folder for the copies. `None` = `<main folder>/<owner>`.
    pub destination: Option<PathBuf>,
    pub auto_update: bool,
    pub format: StorageFormat,
    pub mode: UpdateMode,
    /// For "keep versions": how many to keep. 0 = all of them.
    pub keep_versions: u32,
    /// `None` = the repository's default branch.
    pub branch: Option<String>,
}

impl Default for RepoSettings {
    fn default() -> Self {
        Self {
            destination: None,
            auto_update: true,
            format: StorageFormat::Zip,
            mode: UpdateMode::Replace,
            keep_versions: 0,
            branch: None,
        }
    }
}

/// App-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub base_folder: PathBuf,
    pub auto_check: bool,
    pub check_interval_minutes: u32,
    /// Given to repositories added from now on.
    pub defaults: RepoSettings,
    pub include_forks: bool,
    pub include_archived: bool,
    pub concurrent_downloads: u32,
    /// Closing the window keeps Repopus running in the menu bar / system tray.
    pub run_in_background: bool,
    pub launch_at_login: bool,
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            base_folder: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join("Repopus"),
            auto_check: true,
            check_interval_minutes: 60,
            defaults: RepoSettings::default(),
            include_forks: true,
            include_archived: true,
            concurrent_downloads: 3,
            run_in_background: true,
            launch_at_login: false,
            appearance: Appearance::System,
        }
    }
}

// ==================================================================== stored state

/// A followed user or organisation. New repositories are added automatically;
/// ones that disappear are marked as gone, never deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Lower-case account name.
    pub id: String,
    pub owner: String,
    /// "User" or "Organization", as GitHub reports it.
    #[serde(default)]
    pub account_type: Option<String>,
    pub added_at: DateTime<Utc>,
    /// Repository names the user stopped tracking; they won't come back on their own.
    #[serde(default)]
    pub ignored: Vec<String>,
    /// Repositories that appear on the account later are added on their own.
    #[serde(default = "yes")]
    pub add_new: bool,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_listed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gone_since: Option<DateTime<Utc>>,
}

/// Default for switches that start on.
fn yes() -> bool {
    true
}

/// A copy Repopus wrote to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalCopy {
    pub sha: String,
    pub branch: String,
    pub path: PathBuf,
    pub downloaded_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub format: StorageFormat,
}

/// What GitHub last told us, plus the ETags that make re-checking free.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Remote {
    pub sha: Option<String>,
    pub branch: Option<String>,
    /// `pushed_at` when `sha` was fetched. Unchanged = nothing was pushed since.
    pub pushed_at_seen: Option<String>,
    pub sha_etag: Option<String>,
    pub repo_etag: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
}

/// One tracked repository. Its behaviour lives in `repo.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    /// Lower-case "owner/name".
    pub id: String,
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub archived: bool,
    pub default_branch: String,
    #[serde(default)]
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub remote_size_kb: Option<u64>,
    /// Followed account it came from. `None` = added on its own.
    #[serde(default)]
    pub source: Option<String>,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub settings: RepoSettings,
    #[serde(default)]
    pub remote: Remote,
    /// The newest copy on disk.
    #[serde(default)]
    pub local: Option<LocalCopy>,
    /// Copies kept by "keep versions", oldest first (includes `local`).
    #[serde(default)]
    pub versions: Vec<LocalCopy>,
    /// Set when it disappeared from GitHub. Local files are kept.
    #[serde(default)]
    pub gone_since: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<String>,
}

pub const SCHEMA_VERSION: u32 = 1;

/// Everything Repopus remembers, saved as one JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub schema: u32,
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub repos: BTreeMap<String, Repo>,
    pub last_check: Option<DateTime<Utc>>,
    /// Categories (lower-case owner names) folded shut in the window.
    pub collapsed_groups: Vec<String>,
}

// ==================================================================== sent to the window

/// The mark shown at the start of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Green check.
    UpToDate,
    /// Orange [!].
    UpdateAvailable,
    /// Red [!]: gone from GitHub, or the local copy disappeared.
    Missing,
    /// Red [!] with an error message.
    Error,
    /// Grey: not checked yet.
    Unknown,
    /// Spinner: downloading / unpacking right now.
    Busy,
}

/// What a busy row is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BusyPhase {
    /// Queued in "Update all", not started yet (spinner).
    Waiting,
    /// Checking, verifying, unpacking or installing (spinner + text).
    Working,
    /// Receiving the archive (progress bar).
    Downloading,
}

/// Progress of a busy row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Busy {
    pub phase: BusyPhase,
    pub text: String,
    /// Bytes received so far (downloading only).
    pub bytes: u64,
    /// Expected size, if known: GitHub's Content-Length, or the size of the last copy.
    pub total: Option<u64>,
}

impl Busy {
    /// Queued, waiting for its turn.
    pub fn waiting() -> Self {
        Self::working("Waiting…").with_phase(BusyPhase::Waiting)
    }

    /// A step without measurable progress ("Verifying…").
    pub fn working(text: &str) -> Self {
        Self {
            phase: BusyPhase::Working,
            text: text.to_string(),
            bytes: 0,
            total: None,
        }
    }

    /// Download progress.
    pub fn downloading(bytes: u64, total: Option<u64>) -> Self {
        Self {
            phase: BusyPhase::Downloading,
            text: "Downloading…".into(),
            bytes,
            total,
        }
    }

    /// Same, with another phase.
    fn with_phase(mut self, phase: BusyPhase) -> Self {
        self.phase = phase;
        self
    }
}

/// One row in the list.
#[derive(Debug, Clone, Serialize)]
pub struct RepoView {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub branch: String,
    pub default_branch: String,
    pub source: Option<String>,
    pub status: Status,
    pub status_text: String,
    pub busy: Option<Busy>,
    pub can_update: bool,
    pub remote_sha: Option<String>,
    pub remote_checked_at: Option<DateTime<Utc>>,
    pub pushed_at: Option<String>,
    pub remote_size_kb: Option<u64>,
    pub local: Option<LocalCopy>,
    pub local_exists: bool,
    pub versions: Vec<LocalCopy>,
    pub settings: RepoSettings,
    /// Folder the next copy goes into.
    pub destination: String,
    /// Path of the next copy (for "keep versions": an example).
    pub target_hint: String,
    pub gone_since: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// Everything the window needs to draw itself.
#[derive(Debug, Clone, Serialize)]
pub struct AppView {
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub repos: Vec<RepoView>,
    pub last_check: Option<DateTime<Utc>>,
    pub checking: bool,
    pub updating: usize,
    pub collapsed_groups: Vec<String>,
}

/// One repository in the "choose repositories" window.
#[derive(Debug, Clone, Serialize)]
pub struct RepoChoice {
    pub name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub pushed_at: Option<String>,
    pub size_kb: Option<u64>,
    /// Tracked now (or, for an account not followed yet, suggested).
    pub selected: bool,
}

/// An account's repositories, for choosing which ones Repopus tracks.
#[derive(Debug, Clone, Serialize)]
pub struct AccountChoice {
    pub owner: String,
    /// "User" or "Organization".
    pub account_type: String,
    pub following: bool,
    pub add_new: bool,
    pub repos: Vec<RepoChoice>,
}

/// What a pasted link led to.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddOutcome {
    /// A single repository was added.
    Added { message: String },
    /// An account: the window lets the user choose its repositories first.
    ChooseRepos { account: AccountChoice },
}

/// Progress of the app as a whole (checking, downloads running, last message).
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub checking: bool,
    pub updating: usize,
    pub message: Option<String>,
    pub error: bool,
}
