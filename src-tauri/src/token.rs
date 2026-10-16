//! The GitHub token. Without one GitHub allows 60 requests an hour; with one, 5,000.
//!
//! Order of preference: a token saved in Repopus (in the system keychain: macOS Keychain,
//! Windows Credential Manager, or the Secret Service on Linux), then the GitHub CLI's login
//! (`gh auth token`) if it's installed, then none.

use keyring::Entry;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

const SERVICE: &str = "Repopus";
const ACCOUNT: &str = "github-token";

/// Where the token came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    Keychain,
    GithubCli,
    None,
}

/// What the settings panel shows about the token.
#[derive(Debug, Clone, Serialize)]
pub struct TokenStatus {
    pub source: TokenSource,
    pub login: Option<String>,
    pub error: Option<String>,
}

pub struct Token;

impl Token {
    /// Finds the best available token.
    pub fn load() -> (Option<String>, TokenSource) {
        if let Some(t) = Self::from_keychain() {
            return (Some(t), TokenSource::Keychain);
        }
        if let Some(t) = Self::from_github_cli() {
            return (Some(t), TokenSource::GithubCli);
        }
        (None, TokenSource::None)
    }

    /// `Some` saves the token in the system keychain, `None` removes it.
    pub fn save(token: Option<&str>) -> Result<(), String> {
        let entry = Self::entry()?;
        match token {
            Some(t) => entry
                .set_password(t)
                .map_err(|e| format!("Couldn't save the token in the system keychain: {e}")),
            None => match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(format!("Couldn't remove the token from the system keychain: {e}")),
            },
        }
    }

    /// The keychain item Repopus uses.
    fn entry() -> Result<Entry, String> {
        Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
    }

    /// The token saved in the keychain, if any.
    fn from_keychain() -> Option<String> {
        let token = Self::entry().ok()?.get_password().ok()?;
        (!token.trim().is_empty()).then_some(token)
    }

    /// The GitHub CLI's token. Apps started from the Dock or a launcher may not get the
    /// shell's PATH, so the usual install locations are tried before plain `gh`.
    fn from_github_cli() -> Option<String> {
        Self::github_cli_candidates()
            .into_iter()
            .filter(|gh| gh == "gh" || gh == "gh.exe" || Path::new(gh).exists())
            .find_map(|gh| {
                let out = Self::quiet_command(&gh).args(["auth", "token"]).output().ok()?;
                let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
                (out.status.success() && !token.is_empty()).then_some(token)
            })
    }

    /// Where the GitHub CLI usually lives on this platform.
    fn github_cli_candidates() -> Vec<String> {
        let mut list: Vec<String> = Vec::new();
        if cfg!(windows) {
            for base in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
                if let Ok(dir) = std::env::var(base) {
                    list.push(format!("{dir}\\GitHub CLI\\gh.exe"));
                }
            }
            list.push("gh.exe".into());
        } else {
            list.extend(
                ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "/usr/bin/gh", "/snap/bin/gh"]
                    .map(String::from),
            );
            if let Some(home) = dirs_home() {
                list.push(format!("{home}/.local/bin/gh"));
            }
            list.push("gh".into());
        }
        list
    }

    /// A command that doesn't flash a console window on Windows.
    fn quiet_command(program: &str) -> Command {
        #[allow(unused_mut)]
        let mut cmd = Command::new(program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }
}

/// The user's home folder as text (for the GitHub CLI lookup).
fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| !h.is_empty())
}
