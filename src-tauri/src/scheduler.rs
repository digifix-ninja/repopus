//! Runs the automatic check when it's due.
//!
//! It wakes every minute and compares with the time of the last check (instead of using
//! a fixed timer), so a computer that was asleep catches up as soon as it wakes.

use repopus_core::Engine;
use std::time::Duration;

/// Wait after launch before the first look, so the window can appear first.
const STARTUP_DELAY: Duration = Duration::from_secs(5);
const TICK: Duration = Duration::from_secs(60);

pub struct Scheduler {
    engine: Engine,
}

impl Scheduler {
    /// A scheduler for this engine.
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    /// Starts the background loop (runs until the app quits).
    pub fn start(self) {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(STARTUP_DELAY).await;
            loop {
                if self.engine.is_check_due().await {
                    let _ = self.engine.check_all().await;
                }
                tokio::time::sleep(TICK).await;
            }
        });
    }
}
