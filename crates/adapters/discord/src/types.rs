use std::sync::Arc;

use mira_monitor::StreamMonitor;

pub struct Data {
    pub monitor: Arc<StreamMonitor>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;
