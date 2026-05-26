use mira_core::PersistenceProvider;

use crate::subscription::SubscriptionHandler;

pub struct Data<P: PersistenceProvider + 'static> {
    pub subscription_handler: SubscriptionHandler<P>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a, P> = poise::Context<'a, Data<P>, Error>;
pub type ApplicationContext<'a, P> = poise::ApplicationContext<'a, Data<P>, Error>;
