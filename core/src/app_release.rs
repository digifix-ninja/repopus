//! Whether a newer Repopus has been published on GitHub.
//!
//! Asks for the latest release of the public repository (drafts and pre-releases don't
//! count) and compares its tag (`v0.4.0`) with the running version.

use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A published version of Repopus.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppRelease {
    /// Version number without the leading "v", like "0.4.0".
    pub version: String,
    /// The release page on GitHub (release notes and installers).
    pub url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
}

/// The latest published release of `repo` ("owner/name"); `None` if there is none yet.
pub async fn latest(repo: &str) -> Result<Option<AppRelease>, String> {
    let http = Client::builder()
        .user_agent(concat!("Repopus/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let response = http
        .get(format!("https://api.github.com/repos/{repo}/releases/latest"))
        .header(header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Couldn't reach GitHub: {e}"))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("GitHub answered {}", response.status().as_u16()));
    }
    let release: ApiRelease = response.json().await.map_err(|e| e.to_string())?;
    Ok(Some(AppRelease {
        version: release.tag_name.trim_start_matches(['v', 'V']).to_string(),
        url: release.html_url,
    }))
}

/// Whether version `candidate` comes after `current` ("0.10.0" is newer than "0.9.2").
/// Anything after a "-" or "+" is ignored; a version that can't be read is never newer.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (numbers(candidate), numbers(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// "v1.2.3-beta" → [1, 2, 3]; missing parts count as 0.
fn numbers(version: &str) -> Option<[u64; 3]> {
    let core = version.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next()?;
    let mut parts = [0u64; 3];
    for (i, part) in core.split('.').enumerate() {
        if i >= 3 {
            break;
        }
        parts[i] = part.parse().ok()?;
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.4.0", "0.3.0"));
        assert!(is_newer("v0.10.0", "0.9.2"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.1", "0.3.0"));
        assert!(!is_newer("latest", "0.3.0"));
        assert!(!is_newer("0.3.0-beta", "0.3.0"));
    }
}
