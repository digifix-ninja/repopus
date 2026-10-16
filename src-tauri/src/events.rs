//! Turns engine events into window events (names the window listens for).

use repopus_core::{Busy, Emitter, EngineEvent};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter as _};

/// Payload of the "progress" event.
#[derive(Clone, Serialize)]
struct Progress {
    id: String,
    busy: Busy,
}

/// An emitter that forwards every engine event to the window.
pub fn forward_to(app: &AppHandle) -> Emitter {
    let app = app.clone();
    Arc::new(move |event: EngineEvent| {
        let _ = match event {
            EngineEvent::Repo(view) => app.emit("repo", view),
            EngineEvent::RepoRemoved(id) => app.emit("repo-removed", id),
            EngineEvent::Progress { id, busy } => app.emit("progress", Progress { id, busy }),
            EngineEvent::Activity(activity) => app.emit("activity", activity),
            EngineEvent::StoreChanged => app.emit("store-changed", ()),
        };
    })
}
