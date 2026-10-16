//! What every command can reach.

use crate::token::{Token, TokenSource};
use repopus_core::Engine;
use std::sync::Mutex;

pub struct AppState {
    pub engine: Engine,
    /// Where the current GitHub token came from.
    pub token_source: Mutex<TokenSource>,
}

impl AppState {
    /// Loads the GitHub token and hands it to the engine.
    pub fn new(engine: Engine) -> Self {
        let (token, source) = Token::load();
        engine.set_token(token);
        Self {
            engine,
            token_source: Mutex::new(source),
        }
    }

    /// Where the current token came from.
    pub fn token_source(&self) -> TokenSource {
        *self.token_source.lock().unwrap()
    }

    /// Remembers where the current token came from.
    pub fn set_token_source(&self, source: TokenSource) {
        *self.token_source.lock().unwrap() = source;
    }
}
