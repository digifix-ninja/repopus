//! Adding and removing what Repopus tracks: followed accounts and single repositories.

use super::{repositories, Engine, EngineEvent};
use crate::github::{ApiRepo, Fetch};
use crate::link::Link;
use crate::model::*;
use chrono::Utc;
use std::collections::HashSet;

impl Engine {
    /// Adds whatever the pasted link points at. Returns a short message for the window.
    /// The caller runs a check afterwards.
    pub async fn add_link(&self, input: &str) -> Result<String, String> {
        match Link::parse(input)? {
            Link::Account { owner } => self.follow_account(&owner).await,
            Link::Repo { owner, name, branch } => self.add_single_repo(&owner, &name, branch).await,
        }
    }

    /// Follows an account: all its repositories now, and new ones later.
    async fn follow_account(&self, owner: &str) -> Result<String, String> {
        let id = owner.to_lowercase();
        if self.is_following(&id).await {
            return Err(format!("You're already following {owner}."));
        }
        let user = match self.inner.github.user(owner).await.map_err(|e| e.to_string())? {
            Fetch::Modified { data, .. } => data,
            _ => return Err(format!("There is no GitHub account called “{owner}”.")),
        };
        {
            let mut st = self.inner.store.lock().await;
            st.sources.push(Source {
                id: id.clone(),
                owner: user.login.clone(),
                account_type: Some(user.kind.clone()),
                added_at: Utc::now(),
                ignored: Vec::new(),
                last_error: None,
                last_listed_at: None,
                gone_since: None,
            });
            self.save(&st);
        }
        self.emit(EngineEvent::StoreChanged);
        let added = self.refresh_account(&id).await?;
        Ok(format!("Following {} · {} added", user.login, repositories(added)))
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
        let own = self
            .token_login()
            .await
            .ok()
            .flatten()
            .is_some_and(|me| me.eq_ignore_ascii_case(&owner));
        let listing = self.inner.github.list_repos(&owner, is_org, own).await;

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

    /// Whether this account is already followed.
    async fn is_following(&self, id: &str) -> bool {
        self.inner.store.lock().await.sources.iter().any(|s| s.id == id)
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
        let ignored: HashSet<String> = st
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .map(|s| s.ignored.iter().cloned().collect())
            .unwrap_or_default();
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
            let wanted = !ignored.contains(&api.name.to_lowercase())
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
