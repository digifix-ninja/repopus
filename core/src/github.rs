//! A small GitHub REST client.
//!
//! Checks are designed to be almost free:
//! * an account is listed in pages of 100 (usually one request), and each entry carries
//!   `pushed_at`; a repository whose `pushed_at` didn't move isn't asked again;
//! * single repositories and branch heads are fetched with `If-None-Match` (ETags).
//!   A "304 Not Modified" answer doesn't count against GitHub's rate limit.

use chrono::{DateTime, Local};
use futures_util::StreamExt;
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::Path;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

const API: &str = "https://api.github.com";
const JSON: &str = "application/vnd.github+json";
const TIMEOUT: Duration = Duration::from_secs(30);

// ==================================================================== answers

/// Things that can go wrong talking to GitHub, worded for the user.
#[derive(Debug, thiserror::Error)]
pub enum GhError {
    #[error("GitHub's hourly request limit is used up{0}. Adding a GitHub token in Settings raises it a lot.")]
    RateLimited(String),
    #[error("The GitHub token was rejected. Check it in Settings.")]
    BadToken,
    #[error("Network problem: {0}")]
    Network(String),
    #[error("GitHub answered {status}: {message}")]
    Http { status: u16, message: String },
    #[error("Unexpected answer from GitHub: {0}")]
    Decode(String),
    #[error("Disk problem: {0}")]
    Io(String),
}

/// Result of a conditional request.
pub enum Fetch<T> {
    /// New data (with the ETag to send next time).
    Modified { data: T, etag: Option<String> },
    /// Same as last time.
    NotModified,
    /// Doesn't exist, or no longer accessible.
    NotFound,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiOwner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiUser {
    pub login: String,
    /// "User" or "Organization".
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRepo {
    pub name: String,
    pub full_name: String,
    pub owner: ApiOwner,
    pub description: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub default_branch: Option<String>,
    pub pushed_at: Option<String>,
    /// Kilobytes.
    pub size: Option<u64>,
}

// ==================================================================== client

/// Holds the HTTP client and the (optional) token.
pub struct GitHub {
    http: Client,
    token: RwLock<Option<String>>,
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHub {
    /// Creates a client without a token.
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent(concat!("Repopus/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .expect("HTTP client");
        Self {
            http,
            token: RwLock::new(None),
        }
    }

    /// Sets or clears the token used for every request.
    pub fn set_token(&self, token: Option<String>) {
        let clean = token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        *self.token.write().unwrap() = clean;
    }

    /// Whether a token is set.
    pub fn has_token(&self) -> bool {
        self.token.read().unwrap().is_some()
    }

    // ---------------------------------------------------------------- public calls

    /// Looks up a user or organisation.
    pub async fn user(&self, login: &str) -> Result<Fetch<ApiUser>, GhError> {
        self.fetch_json(&format!("{API}/users/{login}"), None).await
    }

    /// Login of the token's owner, or `None` without a token.
    pub async fn me(&self) -> Result<Option<String>, GhError> {
        if !self.has_token() {
            return Ok(None);
        }
        match self.fetch_json::<ApiUser>(&format!("{API}/user"), None).await? {
            Fetch::Modified { data, .. } => Ok(Some(data.login)),
            _ => Ok(None),
        }
    }

    /// All repositories of an account (`None` = the account doesn't exist).
    /// `own` lists the token owner's repositories, private ones included.
    pub async fn list_repos(
        &self,
        owner: &str,
        is_org: bool,
        own: bool,
    ) -> Result<Option<Vec<ApiRepo>>, GhError> {
        let mut url = Self::listing_url(owner, is_org, own);
        let mut all = Vec::new();
        let mut first_page = true;
        loop {
            let resp = self.send(self.get(&url, JSON, None).timeout(TIMEOUT)).await?;
            if first_page && resp.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if !resp.status().is_success() {
                return Err(Self::error_from(resp).await);
            }
            let next = Self::next_page(resp.headers());
            let page: Vec<ApiRepo> = resp
                .json()
                .await
                .map_err(|e| GhError::Decode(e.to_string()))?;
            all.extend(page);
            first_page = false;
            match next {
                Some(n) if all.len() < 10_000 => url = n,
                _ => break,
            }
        }
        Ok(Some(all))
    }

    /// One repository's details (free when unchanged, thanks to the ETag).
    pub async fn repo(
        &self,
        owner: &str,
        name: &str,
        etag: Option<&str>,
    ) -> Result<Fetch<ApiRepo>, GhError> {
        self.fetch_json(&format!("{API}/repos/{owner}/{name}"), etag).await
    }

    /// Commit id at the head of `branch` (`NotFound` = no such branch).
    pub async fn branch_sha(
        &self,
        owner: &str,
        name: &str,
        branch: &str,
        etag: Option<&str>,
    ) -> Result<Fetch<String>, GhError> {
        let url = format!("{API}/repos/{owner}/{name}/commits/{}", Self::encode_ref(branch));
        let request = self.get(&url, "application/vnd.github.sha", etag).timeout(TIMEOUT);
        let resp = self.send(request).await?;
        match resp.status() {
            StatusCode::NOT_MODIFIED => Ok(Fetch::NotModified),
            StatusCode::NOT_FOUND | StatusCode::UNPROCESSABLE_ENTITY => Ok(Fetch::NotFound),
            StatusCode::CONFLICT => Err(GhError::Http {
                status: 409,
                message: "this repository is empty".into(),
            }),
            s if s.is_success() => {
                let etag = Self::etag_of(&resp);
                let sha = resp.text().await.map_err(Self::network_error)?.trim().to_string();
                if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(GhError::Decode("no commit id in answer".into()));
                }
                Ok(Fetch::Modified { data: sha, etag })
            }
            _ => Err(Self::error_from(resp).await),
        }
    }

    /// Streams the zip of commit `sha` into `dest`, reporting bytes received so far
    /// and the total size when GitHub sends one.
    pub async fn download_zip(
        &self,
        owner: &str,
        name: &str,
        sha: &str,
        dest: &Path,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<u64, GhError> {
        let url = format!("{API}/repos/{owner}/{name}/zipball/{sha}");
        let resp = self.send(self.get(&url, JSON, None)).await?;
        if !resp.status().is_success() {
            return Err(Self::error_from(resp).await);
        }

        let size = resp.content_length().filter(|n| *n > 0);
        let io = |e: std::io::Error| GhError::Io(e.to_string());
        let mut file = tokio::fs::File::create(dest).await.map_err(io)?;
        let mut stream = resp.bytes_stream();
        let mut total = 0u64;
        let mut last_report = Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(Self::network_error)?;
            file.write_all(&chunk).await.map_err(io)?;
            total += chunk.len() as u64;
            if last_report.elapsed() >= Duration::from_millis(250) {
                progress(total, size);
                last_report = Instant::now();
            }
        }
        file.flush().await.map_err(io)?;
        file.sync_all().await.map_err(io)?;
        progress(total, size);
        Ok(total)
    }

    // ---------------------------------------------------------------- plumbing

    /// Builds a GET request with GitHub's headers, the token and an optional ETag.
    fn get(&self, url: &str, accept: &str, etag: Option<&str>) -> RequestBuilder {
        let mut request = self
            .http
            .get(url)
            .header(header::ACCEPT, accept)
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = self.token.read().unwrap().as_ref() {
            request = request.bearer_auth(t);
        }
        if let Some(e) = etag {
            request = request.header(header::IF_NONE_MATCH, e);
        }
        request
    }

    /// Sends a request and turns rate-limit and token problems into clear errors.
    async fn send(&self, request: RequestBuilder) -> Result<Response, GhError> {
        let resp = request.send().await.map_err(Self::network_error)?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(GhError::BadToken);
        }
        if Self::is_rate_limited(&resp) {
            return Err(GhError::RateLimited(Self::reset_time(&resp)));
        }
        Ok(resp)
    }

    /// GET + JSON decoding, with ETag support.
    async fn fetch_json<T: DeserializeOwned>(
        &self,
        url: &str,
        etag: Option<&str>,
    ) -> Result<Fetch<T>, GhError> {
        let resp = self.send(self.get(url, JSON, etag).timeout(TIMEOUT)).await?;
        match resp.status() {
            StatusCode::NOT_MODIFIED => Ok(Fetch::NotModified),
            StatusCode::NOT_FOUND | StatusCode::GONE => Ok(Fetch::NotFound),
            // 403 / 451 here mean "access blocked" (e.g. a disabled repository).
            StatusCode::FORBIDDEN | StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS => {
                Ok(Fetch::NotFound)
            }
            s if s.is_success() => {
                let etag = Self::etag_of(&resp);
                let data = resp
                    .json::<T>()
                    .await
                    .map_err(|e| GhError::Decode(e.to_string()))?;
                Ok(Fetch::Modified { data, etag })
            }
            _ => Err(Self::error_from(resp).await),
        }
    }

    /// The first listing page for an account.
    fn listing_url(owner: &str, is_org: bool, own: bool) -> String {
        if own {
            format!("{API}/user/repos?affiliation=owner&per_page=100&sort=full_name")
        } else if is_org {
            format!("{API}/orgs/{owner}/repos?type=all&per_page=100&sort=full_name")
        } else {
            format!("{API}/users/{owner}/repos?type=owner&per_page=100&sort=full_name")
        }
    }

    /// Whether GitHub refused the request because the hourly limit is used up.
    fn is_rate_limited(resp: &Response) -> bool {
        let status = resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
        status == StatusCode::FORBIDDEN
            && (Self::header(resp, "x-ratelimit-remaining").as_deref() == Some("0")
                || resp.headers().contains_key(header::RETRY_AFTER))
    }

    /// " (it resets at 14:05)", or empty if GitHub didn't say.
    fn reset_time(resp: &Response) -> String {
        Self::header(resp, "x-ratelimit-reset")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|d| d.with_timezone(&Local).format(" (it resets at %H:%M)").to_string())
            .unwrap_or_default()
    }

    /// Reads a response header as text.
    fn header(resp: &Response, name: &str) -> Option<String> {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    /// The ETag to send next time.
    fn etag_of(resp: &Response) -> Option<String> {
        Self::header(resp, "etag")
    }

    /// Turns an unexpected answer into an error with GitHub's own message.
    async fn error_from(resp: Response) -> GhError {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_string))
            .unwrap_or_else(|| body.chars().take(200).collect());
        GhError::Http { status, message }
    }

    /// Words a network failure for the user.
    fn network_error(e: reqwest::Error) -> GhError {
        if e.is_timeout() {
            GhError::Network("GitHub took too long to answer".into())
        } else if e.is_connect() {
            GhError::Network("couldn't reach GitHub (offline?)".into())
        } else {
            GhError::Network(e.to_string())
        }
    }

    /// URL of the next listing page, from the `Link` header.
    fn next_page(headers: &header::HeaderMap) -> Option<String> {
        let link = headers.get(header::LINK)?.to_str().ok()?;
        link.split(',').find_map(|part| {
            let (url, rel) = part.split_once(';')?;
            rel.contains("rel=\"next\"")
                .then(|| url.trim().trim_start_matches('<').trim_end_matches('>').to_string())
        })
    }

    /// Escapes a branch name for the URL ('/' stays, GitHub accepts it).
    fn encode_ref(r: &str) -> String {
        let mut out = String::with_capacity(r.len());
        for b in r.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'/' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
}
