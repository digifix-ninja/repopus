//! Facts shown in the About panel. Change them here (and in LICENSE) when needed.

use serde::Serialize;
use tauri::AppHandle;

/// The app's own website.
pub const APP_WEBSITE: &str = "https://repopus.app";
pub const DEVELOPER: &str = "stean.DEV";
/// The developer's website.
pub const WEBSITE: &str = "https://stean.dev";
/// The developer's email address.
pub const EMAIL: &str = "a.s@stean.dev";
/// Public source code on GitHub (an empty value hides the link).
pub const SOURCE_CODE: &str = "https://github.com/steanart/repopus";
/// The GitHub repository ("owner/name") whose releases are the app's updates.
pub const RELEASES_REPO: &str = "steanart/repopus";
pub const LICENCE: &str = "MIT";
pub const COPYRIGHT_YEAR: &str = "2026";

/// Everything the About panel shows.
#[derive(Debug, Clone, Serialize)]
pub struct About {
    name: String,
    version: String,
    app_website: &'static str,
    developer: &'static str,
    website: &'static str,
    email: &'static str,
    source_code: Option<&'static str>,
    licence: &'static str,
    copyright_year: &'static str,
}

impl About {
    /// Name and version come from the app bundle, the rest from the constants above.
    pub fn of(app: &AppHandle) -> Self {
        let info = app.package_info();
        Self {
            name: info.name.clone(),
            version: info.version.to_string(),
            app_website: APP_WEBSITE,
            developer: DEVELOPER,
            website: WEBSITE,
            email: EMAIL,
            source_code: (!SOURCE_CODE.is_empty()).then_some(SOURCE_CODE),
            licence: LICENCE,
            copyright_year: COPYRIGHT_YEAR,
        }
    }

    /// Links the app may open: GitHub, the app's and the developer's sites, the source
    /// code, and an email to the developer.
    pub fn may_open(url: &str) -> bool {
        if url == format!("mailto:{EMAIL}") {
            return true;
        }
        let allowed = ["https://github.com", APP_WEBSITE, WEBSITE, SOURCE_CODE];
        allowed.iter().filter(|site| !site.is_empty()).any(|site| {
            url == *site || url.starts_with(&format!("{site}/"))
        })
    }
}
