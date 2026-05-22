use std::sync::Arc;

use mira_core::{CoreError, PersistenceProvider};
use mira_monitor::StreamMonitor;
use poise::serenity_prelude::Http;

use crate::notifier::DiscordNotifier;

pub async fn restore_subscriptions(
    monitor: &StreamMonitor,
    persistence: &Arc<dyn PersistenceProvider>,
    http: &Arc<Http>,
) -> Result<(), CoreError> {
    let subscriptions_to_restore = persistence.get_all_subscriptions().await?;

    println!(
        "Restoring {} subscription(s)...",
        subscriptions_to_restore.len()
    );

    for subscription in subscriptions_to_restore {
        let host = subscription.host;
        let key = subscription.key;
        let subscription_id = subscription.subscription_id;
        let channel_id = subscription.channel_id;

        let notifier = DiscordNotifier::new(
            host.clone(),
            key.clone(),
            subscription_id,
            channel_id,
            http.clone(),
            persistence.clone(),
        );

        let url = host.url.clone();
        let auth_header = host.auth_header.clone();
        monitor.register_stream(url, auth_header, key.clone(), notifier.into_callback());

        println!(
            "Restored {}/{} ({})",
            host.url.clone(),
            key,
            host.host_guild_id
        );
    }

    Ok(())
}
