//! Adding and removing what Repopus tracks: followed accounts and single repositories.

use super::{repositories, Engine, EngineEvent};
use crate::github::{ApiRepo, Fetch, GhError};
use crate::link::Link;
use crate::model::*;
use chrono::Utc;
use std::collections::HashSet;

impl Engine {
    /// Adds whatever the pasted link points at. A repository is added at once (the caller
    /// runs a check afterwards); an account comes back with its repositories, so the user
    /// can choose which ones to track (see `choose_repos`).
    pub async fn add_link(&self, input: &str) -> Result<AddOutcome, String> {
        match Link::parse(input)? {
            Link::Account { owner } => Ok(AddOutcome::ChooseRepos {
                account: self.account_choice(&owner).await?,
            }),
            Link::Repo { owner, name, branch } => {
                let message = self.add_single_repo(&owner, &name, branch).await?;
                Ok(AddOutcome::Added { message })
            }
        }
    }

    /// An account's repositories, marked with the ones tracked now. For an account that
    /// isn't followed yet, the ones the settings would include are suggested.
    pub async fn account_choice(&self, owner: &str) -> Result<AccountChoice, String> {
        let (login, kind, list) = self.fetch_account(owner).await?;
        let id = login.to_lowercase();
        let st = self.inner.store.lock().await;
        let source = st.sources.iter().find(|s| s.id == id);
        let s = &st.settings;
        let mut repos: Vec<RepoChoice> = list
            .iter()
            .map(|api| {
                let tracked = st.repos.contains_key(&api.full_name.to_lowercase());
                let suggested = (s.include_forks || !api.fork) && (s.include_archived || !api.archived);
                Self::choice(api, tracked || (source.is_none() && suggested))
            })
            .collect();
        repos.sort_by_key(|r| r.name.to_lowercase());
        Ok(AccountChoice {
            owner: login,
            account_type: kind,
            following: source.is_some(),
            add_new: source.is_none_or(|s| s.add_new),
            repos,
        })
    }

    /// Follows an account (if it isn't yet) and tracks exactly the `chosen` repositories.
    /// The others are remembered as not wanted; ones that were tracked before stop being
    /// tracked, but their files stay. `add_new`: repositories that appear on the account
    /// later are added on their own. Returns a short message; the caller runs a check.
    pub async fn choose_repos(&self, owner: &str, chosen: &[String], add_new: bool) -> Result<String, String> {
        let (login, kind, list) = self.fetch_account(owner).await?;
        let id = login.to_lowercase();
        let chosen: HashSet<String> = chosen.iter().map(|n| n.to_lowercase()).collect();
        let (newly_followed, added, dropped) = {
            let mut st = self.inner.store.lock().await;
            let defaults = st.settings.defaults.clone();
            let newly_followed = !st.sources.iter().any(|s| s.id == id);
            if newly_followed {
                st.sources.push(Source {
                    id: id.clone(),
                    owner: login.clone(),
                    account_type: None,
                    added_at: Utc::now(),
                    ignored: Vec::new(),
                    add_new,
                    last_error: None,
                    last_listed_at: None,
                    gone_since: None,
                });
            }
            if let Some(source) = st.sources.iter_mut().find(|s| s.id == id) {
                source.account_type = Some(kind);
                source.add_new = add_new;
                source.ignored = list
                    .iter()
                    .map(|api| api.name.to_lowercase())
                    .filter(|name| !chosen.contains(name))
                    .collect();
            }
            let (mut added, mut dropped) = (0, 0);
            for api in &list {
                let repo_id = api.full_name.to_lowercase();
                if chosen.contains(&api.name.to_lowercase()) {
                    if !st.repos.contains_key(&repo_id) {
                        st.repos.insert(repo_id, Repo::from_api(api, Some(id.clone()), &defaults));
                        added += 1;
                    }
                } else if st.repos.remove(&repo_id).is_some() {
                    dropped += 1;
                }
            }
            Self::merge_listing(&mut st, &id, &list);
            self.save(&st);
            (newly_followed, added, dropped)
        };
        self.emit(EngineEvent::StoreChanged);

        Ok(if newly_followed {
            format!("Following {login} · {} added", repositories(added))
        } else if added + dropped == 0 {
            format!("{login}: nothing changed")
        } else {
            let mut parts = Vec::new();
            if added > 0 {
                parts.push(format!("{} added", repositories(added)));
            }
            if dropped > 0 {
                parts.push(format!("{dropped} no longer tracked (files are kept)"));
            }
            format!("{login}: {}", parts.join(" · "))
        })
    }

    /// Account name as GitHub spells it, its type, and its repositories.
    async fn fetch_account(&self, owner: &str) -> Result<(String, String, Vec<ApiRepo>), String> {
        let id = owner.to_lowercase();
        let followed = self.inner.store.lock().await.sources.iter().any(|s| s.id == id);
        let (login, kind) = if followed {
            let (login, is_org) = self.account_kind(&id).await?;
            (login, if is_org { "Organization" } else { "User" }.to_string())
        } else {
            match self.inner.github.user(owner).await.map_err(|e| e.to_string())? {
                Fetch::Modified { data, .. } => (data.login, data.kind),
                _ => return Err(format!("There is no GitHub account called “{owner}”.")),
            }
        };
        let list = self
            .list_account(&login, kind == "Organization")
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("There is no GitHub account called “{owner}”."))?;
        Ok((login, kind, list))
    }

    /// Lists an account's repositories; private ones too when it's the token owner's own.
    /// `None` = the account doesn't exist.
    async fn list_account(&self, owner: &str, is_org: bool) -> Result<Option<Vec<ApiRepo>>, GhError> {
        let own = self
            .token_login()
            .await
            .ok()
            .flatten()
            .is_some_and(|me| me.eq_ignore_ascii_case(owner));
        self.inner.github.list_repos(owner, is_org, own).await
    }

    /// One row of the "choose repositories" window.
    fn choice(api: &ApiRepo, selected: bool) -> RepoChoice {
        RepoChoice {
            name: api.name.clone(),
            description: api.description.clone(),
            private: api.private,
            fork: api.fork,
            archived: api.archived,
            pushed_at: api.pushed_at.clone(),
            size_kb: api.size,
            selected,
        }
    }

    /// Adds one repository (it appears under its owner's category).
    async fn add_single_repo(
        &self,
        owner: &str,
        name: &str,
        branch: Option<String>,
    ) -> Result<String, String> {
        let wanted = format!("{owner}/{name}").to_lowercase();
        if self.snapshot(&wanted).await.is_some() {
            return Err(format!("{owner}/{name} is already in Repopus."));
        }
        let fetched = self.inner.github.repo(owner, name, None).await.map_err(|e| e.to_string())?;
        let Fetch::Modified { data: api, etag } = fetched else {
            return Err(format!(
                "Couldn't find {owner}/{name}. If it's private, add a GitHub token in Settings."
            ));
        };

        let id = api.full_name.to_lowercase();
        {
            let mut st = self.inner.store.lock().await;
            if st.repos.contains_key(&id) {
                return Err(format!("{} is already in Repopus.", api.full_name));
            }
            // If its owner is followed, it simply belongs to that account.
            let source = st.sources.iter().find(|s| s.id == api.owner.login.to_lowercase()).map(|s| s.id.clone());
            let mut repo = Repo::from_api(&api, source, &st.settings.defaults);
            repo.settings.branch = branch;
            repo.remote.repo_etag = etag;
            st.repos.insert(id.clone(), repo);
            self.save(&st);
        }
        self.emit_repo(&id).await;
        Ok(format!("Added {}", api.full_name))
    }

    /// Stops tracking a repository; its files stay. One that came from a followed
    /// account is remembered as ignored, so it doesn't come back on its own.
    pub async fn remove_repo(&self, id: &str) -> Result<(), String> {
        {
            let mut st = self.inner.store.lock().await;
            let repo = st.repos.remove(id).ok_or("Already removed.")?;
            if let Some(source_id) = repo.source {
                if let Some(source) = st.sources.iter_mut().find(|s| s.id == source_id) {
                    let name = repo.name.to_lowercase();
                    if !source.ignored.contains(&name) {
                        source.ignored.push(name);
                    }
                }
            }
            self.save(&st);
        }
        self.emit(EngineEvent::RepoRemoved(id.to_string()));
        Ok(())
    }

    /// Unfollows an account and stops tracking its repositories; files stay.
    pub async fn remove_source(&self, id: &str) -> Result<(), String> {
        {
            let mut st = self.inner.store.lock().await;
            st.sources.retain(|s| s.id != id);
            st.repos.retain(|_, r| r.source.as_deref() != Some(id));
            self.save(&st);
        }
        self.emit(EngineEvent::StoreChanged);
        Ok(())
    }

    /// Lets an ignored repository of a followed account come back on the next check.
    pub async fn restore_ignored(&self, source_id: &str, name: &str) -> Result<(), String> {
        {
            let mut st = self.inner.store.lock().await;
            let source = st
                .sources
                .iter_mut()
                .find(|s| s.id == source_id)
                .ok_or("Account not followed.")?;
            let name = name.to_lowercase();
            source.ignored.retain(|i| *i != name);
            self.save(&st);
        }
        self.emit(EngineEvent::StoreChanged);
        Ok(())
    }

    /// Lists a followed account: adds new repositories, refreshes the others and marks
    /// the ones that disappeared as gone. Returns how many were added.
    pub(super) async fn refresh_account(&self, source_id: &str) -> Result<usize, String> {
        let (owner, is_org) = self.account_kind(source_id).await?;
        let listing = self.list_account(&owner, is_org).await;

        let added = {
            let mut st = self.inner.store.lock().await;
            let result = match listing {
                Err(e) => {
                    let message = e.to_string();
                    if let Some(source) = st.sources.iter_mut().find(|s| s.id == source_id) {
                        source.last_error = Some(message.clone());
                    }
                    Err(message)
                }
                Ok(None) => {
                    Self::mark_account_gone(&mut st, source_id);
                    Ok(0)
                }
                Ok(Some(list)) => Ok(Self::merge_listing(&mut st, source_id, &list)),
            };
            self.save(&st);
            result
        };
        self.emit(EngineEvent::StoreChanged);
        added
    }

    /// Owner name and whether it's an organisation (asks GitHub once, then remembers).
    async fn account_kind(&self, source_id: &str) -> Result<(String, bool), String> {
        let (owner, known) = {
            let st = self.inner.store.lock().await;
            let source = st
                .sources
                .iter()
                .find(|s| s.id == source_id)
                .ok_or("Account not followed.")?;
            (source.owner.clone(), source.account_type.clone())
        };
        let kind = match known {
            Some(kind) => kind,
            None => match self.inner.github.user(&owner).await.map_err(|e| e.to_string())? {
                Fetch::Modified { data, .. } => {
                    let mut st = self.inner.store.lock().await;
                    if let Some(source) = st.sources.iter_mut().find(|s| s.id == source_id) {
                        source.account_type = Some(data.kind.clone());
                    }
                    data.kind
                }
                _ => "User".into(),
            },
        };
        Ok((owner, kind == "Organization"))
    }

    /// The account no longer exists: mark it and all its repositories as gone.
    fn mark_account_gone(st: &mut Store, source_id: &str) {
        let now = Utc::now();
        if let Some(source) = st.sources.iter_mut().find(|s| s.id == source_id) {
            source.gone_since.get_or_insert(now);
            source.last_error =
                Some("This account no longer exists on GitHub. Local copies are kept.".into());
        }
        for repo in st.repos.values_mut().filter(|r| r.source.as_deref() == Some(source_id)) {
            repo.gone_since.get_or_insert(now);
        }
    }

    /// Applies an account listing to the store. Returns how many repositories were added.
    fn merge_listing(st: &mut Store, source_id: &str, list: &[ApiRepo]) -> usize {
        let now = Utc::now();
        let settings = st.settings.clone();
        let source = st.sources.iter().find(|s| s.id == source_id);
        let ignored: HashSet<String> = source.map(|s| s.ignored.iter().cloned().collect()).unwrap_or_default();
        let add_new = source.is_none_or(|s| s.add_new);
        if let Some(source) = st.sources.iter_mut().find(|s| s.id == source_id) {
            source.gone_since = None;
            source.last_error = None;
            source.last_listed_at = Some(now);
        }

        let mut added = 0;
        let mut listed = HashSet::new();
        for api in list {
            let id = api.full_name.to_lowercase();
            listed.insert(id.clone());
            if let Some(repo) = st.repos.get_mut(&id) {
                repo.apply_api(api);
                // A single repository whose owner is now followed joins that account.
                repo.source.get_or_insert_with(|| source_id.to_string());
                continue;
            }
            let wanted = add_new
                && !ignored.contains(&api.name.to_lowercase())
                && (settings.include_forks || !api.fork)
                && (settings.include_archived || !api.archived);
            if wanted {
                st.repos.insert(id, Repo::from_api(api, Some(source_id.to_string()), &settings.defaults));
                added += 1;
            }
        }

        // Tracked but no longer listed: gone from GitHub (files are kept).
        for repo in st.repos.values_mut() {
            if repo.source.as_deref() == Some(source_id) && !listed.contains(&repo.id) {
                repo.gone_since.get_or_insert(now);
            }
        }
        added
    }
}
