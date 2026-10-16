//! Understanding whatever the user pastes into the "add" field.
//!
//! Accepted, for example:
//!   username                         github.com/username
//!   https://github.com/orgs/name     username/repository
//!   https://github.com/username/repository.git
//!   https://github.com/username/repository/tree/develop   (branch is remembered)
//!   git@github.com:username/repository.git

/// What a pasted link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// A user or organisation: follow all of its repositories.
    Account { owner: String },
    /// One repository, optionally on a specific branch.
    Repo {
        owner: String,
        name: String,
        branch: Option<String>,
    },
}

/// First path parts that are GitHub pages, not accounts.
const RESERVED: &[&str] = &[
    "about", "codespaces", "collections", "explore", "features", "issues", "login", "logout",
    "marketplace", "new", "notifications", "pricing", "pulls", "search", "settings", "signup",
    "sponsors", "topics", "trending",
];

impl Link {
    /// Turns user input into a `Link`, or explains what's wrong with it.
    pub fn parse(input: &str) -> Result<Link, String> {
        let path = Self::path_part(input)?;
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

        // "orgs/<name>/…" and "users/<name>/…" always mean the account.
        let account_only = parts
            .first()
            .is_some_and(|p| p.eq_ignore_ascii_case("orgs") || p.eq_ignore_ascii_case("users"));
        if account_only {
            parts.remove(0);
        }

        let owner = *parts
            .first()
            .ok_or("That link doesn't contain an account or repository.")?;
        Self::check_owner(owner)?;

        if account_only || parts.len() == 1 {
            return Ok(Link::Account {
                owner: owner.to_string(),
            });
        }

        let name = parts[1].trim_end_matches(".git");
        if !Self::is_valid_repo_name(name) {
            return Err(format!("“{name}” doesn't look like a repository name."));
        }
        let branch = match parts.get(2) {
            Some(&"tree") => parts.get(3).map(|b| b.to_string()),
            _ => None,
        };
        Ok(Link::Repo {
            owner: owner.to_string(),
            name: name.to_string(),
            branch,
        })
    }

    /// Strips scheme, host, query and fragment, leaving "owner/repo/…".
    fn path_part(input: &str) -> Result<String, String> {
        let mut s = input.trim().to_string();
        if s.is_empty() {
            return Err("Paste a GitHub link first.".into());
        }

        if let Some(rest) = s.strip_prefix("git@github.com:") {
            s = rest.to_string();
        } else {
            for prefix in ["https://", "http://", "ssh://git@", "git://"] {
                if let Some(rest) = s.strip_prefix(prefix) {
                    s = rest.to_string();
                    break;
                }
            }
            let lower = s.to_ascii_lowercase();
            if lower == "github.com" || lower == "www.github.com" {
                return Err("That is GitHub's front page. Add an account or a repository link.".into());
            }
            if let Some(rest) = lower
                .strip_prefix("www.github.com/")
                .or_else(|| lower.strip_prefix("github.com/"))
            {
                // Keep the original spelling, only cut the host off.
                s = s[s.len() - rest.len()..].to_string();
            } else if s.split('/').next().is_some_and(|host| host.contains('.')) {
                return Err("Only github.com links are supported.".into());
            }
        }

        if let Some(i) = s.find(['?', '#']) {
            s.truncate(i);
        }
        Ok(s)
    }

    /// Rejects names GitHub wouldn't allow, and GitHub's own pages.
    fn check_owner(owner: &str) -> Result<(), String> {
        let valid = !owner.is_empty()
            && owner.len() <= 39
            && !owner.starts_with('-')
            && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid {
            return Err(format!("“{owner}” doesn't look like a GitHub account name."));
        }
        if RESERVED.contains(&owner.to_ascii_lowercase().as_str()) {
            return Err("That's a GitHub page, not an account or repository.".into());
        }
        Ok(())
    }

    /// Repository names: letters, digits, '-', '_' and '.'.
    fn is_valid_repo_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 100
            && name != "."
            && name != ".."
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    }
}
