//! Repopus core: everything that isn't user interface.
//!
//! | module      | responsibility                                                    |
//! |-------------|-------------------------------------------------------------------|
//! | `model`     | the data Repopus stores and the rows the window receives         |
//! | `app_release` | whether a newer Repopus has been published on GitHub          |
//! | `link`      | turning a pasted GitHub link into an account or a repository     |
//! | `github`    | talking to GitHub (cheap conditional requests, zip downloads)    |
//! | `repo`      | behaviour of one tracked repository (branch, status, view)       |
//! | `placement` | where on disk a copy goes                                        |
//! | `archive`   | checking and unpacking zip files                                 |
//! | `installer` | putting a finished copy in place without breaking the old one    |
//! | `files`     | small disk helpers                                                |
//! | `store`     | reading and safely writing the state file                        |
//! | `engine`    | the orchestration: follow, check, update, never delete           |

pub mod app_release;
pub mod archive;
pub mod engine;
pub mod files;
pub mod github;
pub mod installer;
pub mod link;
pub mod model;
pub mod placement;
pub mod repo;
pub mod store;

pub use engine::{Emitter, Engine, EngineEvent};
pub use model::*;
