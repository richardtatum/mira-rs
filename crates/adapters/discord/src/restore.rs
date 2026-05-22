use std::sync::Arc;

use mira_core::{CoreError, PersistenceProvider};
use mira_monitor::StreamMonitor;
use poise::serenity_prelude::Http;

use crate::notifier::DiscordNotifier;

async fn restore_subscriptions(
    monitor: Arc<StreamMonitor>,
    persistence: Arc<dyn PersistenceProvider>,
    http: Arc<Http>,
) -> Result<(), CoreError> {
    let subscriptions_to_restore = persistence.get_all_subscriptions().await?;

    for subscription in subscriptions_to_restore {
        let host = subscription.host;
        let key = subscription.key;
        let subscription_id = subscription.subscription_id;

        let notifier = DiscordNotifier::new(
            host.clone(),
            key.clone(),
            subscription_id.clone(),
            http.clone(),
            persistence.clone(),
        );

        let url = host.url.clone();
        let auth_header = host.auth_header.clone();
        monitor.register_stream(url, auth_header, key.clone(), notifier.into_callback());
    }

    Ok(())
}
