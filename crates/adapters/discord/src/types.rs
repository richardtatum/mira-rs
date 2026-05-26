use std::sync::Arc;

use mira_core::PersistenceProvider;
use mira_stream_watcher::StreamWatcher;

pub struct Data {
    pub monitor: Arc<StreamWatcher>,
    pub persistence: Arc<dyn PersistenceProvider>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;
