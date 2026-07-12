use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use mira_core::{CoreError, Host, HostSubscription, PersistenceProvider, StreamStatus, Subscription};
use mira_stream_watcher::StreamWatcher;
use poise::serenity_prelude::{ChannelId, GuildId, Http, MessageId, UserId};
use url::Url;

use crate::notifier::{DiscordNotifier, StateChange};

pub struct SubscriptionHandler<P: PersistenceProvider + 'static> {
    watcher: StreamWatcher,
    notifier: DiscordNotifier,
    persistence: Arc<P>,
}

impl<P: PersistenceProvider> SubscriptionHandler<P> {
    pub fn new(http: Arc<Http>, persistence: Arc<P>) -> Self {
        Self { watcher: StreamWatcher::new(None), notifier: DiscordNotifier::new(http), persistence }
    }

    pub async fn add_host(
        &self,
        url: Url,
        auth_header: Option<String>,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<(), CoreError> {
        let guild = guild_id.get() as i64;
        let user = user_id.get() as i64;
        let url_str = url.as_str().trim_end_matches('/').to_owned();

        let guild_hosts = self.persistence.get_hosts(guild).await?;
        if guild_hosts.iter().any(|h| h.url == url_str) {
            return Err(CoreError::StreamError("Host already exists!".to_string()));
        }

        // Check the host connects cleanly with the provided data
        self.watcher.test_host(url_str.clone(), auth_header.clone()).await?;

        // Add the host, if it already exists in the database then its id is returned
        let host_id = self.persistence.add_host(url_str, user).await?;

        // Link the host to this guild
        let _ = self.persistence.link_host(host_id, guild, auth_header, user).await?;

        Ok(())
    }

    pub async fn get_hosts(&self, guild_id: GuildId) -> Result<Vec<Host>, CoreError> {
        self.persistence.get_hosts(guild_id.get() as i64).await
    }

    pub async fn subscribe(
        &self,
        host: Host,
        key: String,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<(), CoreError> {
        let existing_sub = self.persistence.get_subscription(host.host_guild_id, key.clone()).await?;
        if let Some(_) = existing_sub {
            // Subscription already exists for this host/key
            return Err(CoreError::AlreadyExistsError("Subscription already exists.".into()));
        }

        let subscription_id = self
            .persistence
            .add_subscription(key.clone(), host.host_guild_id, channel_id.get() as i64, user_id.get() as i64)
            .await?;

        let stream_url = format!("{}/{}", host.url, key);
        let callback = self.build_callback(subscription_id, channel_id, stream_url, key.clone());

        let token = self.watcher.watch(host.url.clone(), host.auth_header, key, callback)?;
        self.persistence.update_subscription_token(subscription_id, token).await?;

        Ok(())
    }

    pub async fn unsubscribe(&self, subscription: Subscription) -> Result<(), CoreError> {
        if let Some(token) = subscription.token {
            // If there is a token, deregister
            self.watcher.stop_watching(token)?;
        }

        // remove from the DB regardless
        self.persistence.delete_subscription(subscription.id).await
    }

    pub async fn remove_host(&self, host: Host, guild_id: GuildId) -> Result<(), CoreError> {
        let subscriptions = self.persistence.get_subscriptions(guild_id.get() as i64).await?;

        // Filter the subscriptions to just this host, and deregister any tokens
        for hs in subscriptions.iter().filter(|hs| hs.host.host_guild_id == host.host_guild_id) {
            if let Some(token) = hs.subscription.token {
                let _ = self.watcher.stop_watching(token);
            }
        }

        // Remove the host from the guild
        self.persistence.unlink_host(host.id, guild_id.get() as i64).await?;

        // If there are no more guilds for this host, remove the host
        self.persistence.delete_host_if_orphaned(host.id).await?;

        Ok(())
    }

    /// Cleans up all hosts/subscriptions for a guild, e.g. when the bot is removed from it.
    pub async fn remove_guild(&self, guild_id: GuildId) -> Result<(), CoreError> {
        let hosts = self.persistence.get_hosts(guild_id.get() as i64).await?;
        for host in hosts {
            self.remove_host(host, guild_id).await?;
        }
        Ok(())
    }

    // Cleans up all subscriptions from a given channel
    pub async fn remove_channel(&self, guild_id: GuildId, channel_id: ChannelId) -> Result<(), CoreError> {
        let subscriptions = self.get_subscriptions(guild_id).await?;
        let channel = channel_id.get() as i64;

        for sub in subscriptions.into_iter().map(|hs| hs.subscription).filter(|s| s.channel_id == channel) {
            // Unsubscribe each that match, but just log if they fail and move onto the next one
            if let Err(e) = self.unsubscribe(sub.clone()).await {
                println!(
                    "Failed to remove subscription '{}' after it's channel '{}' was deleted! Error: {}",
                    &sub.id,
                    &channel_id,
                    &e.message()
                )
            }
        }

        Ok(())
    }

    pub async fn remove_message(&self, guild_id: GuildId, message_id: MessageId) -> Result<(), CoreError> {
        let message = message_id.get() as i64;
        let guild = guild_id.get() as i64;

        // Check if we have anything attached to the message_id
        let Some(subscription) = self.persistence.get_subscription_by_message(guild, message).await? else {
            return Ok(());
        };

        // Mark the subscription as offline to wipe the stored message_id and prompt a new message to be generated
        if let Err(e) = self.persistence.mark_subscription_offline(subscription.id).await {
            println!(
                "Failed to mark subscription '{}' as offline to recreate the message after it was deleted! Error: {}",
                subscription.id,
                &e.message()
            )
        }

        Ok(())
    }

    pub async fn get_subscriptions(&self, guild_id: GuildId) -> Result<Vec<HostSubscription>, CoreError> {
        self.persistence.get_subscriptions(guild_id.get() as i64).await
    }

    pub async fn restore_subscriptions(&self) -> Result<(), CoreError> {
        let to_restore = self.persistence.get_all_subscriptions().await?;

        println!("Restoring {} subscription(s)...", to_restore.len());

        for entry in to_restore {
            let host = entry.host;
            let subscription = entry.subscription;
            let key = subscription.key;
            let subscription_id = subscription.id;
            let channel_id = ChannelId::new(subscription.channel_id as u64);
            let host_url = host.url.clone();

            let stream_url = format!("{}/{}", host.url, key);
            let callback = self.build_callback(subscription_id, channel_id, stream_url, key.clone());

            let token = self.watcher.watch(host.url, host.auth_header, key.clone(), callback)?;
            self.persistence.update_subscription_token(subscription_id, token).await?;

            println!("Restored {}/{} ({})", host_url, key, host.host_guild_id);
        }

        Ok(())
    }

    pub async fn set_playing(&self, subscription_id: i64, playing: String) -> Result<(), CoreError> {
        self.persistence.set_playing(subscription_id, playing).await
    }

    fn build_callback(
        &self,
        subscription_id: i64,
        channel_id: ChannelId,
        stream_url: String,
        key: String,
    ) -> impl Fn(StreamStatus) -> Pin<Box<dyn Future<Output = Result<(), CoreError>> + Send + 'static>> + Send + Sync + 'static
    {
        let persistence = self.persistence.clone();
        let notifier = self.notifier.clone();

        move |status: StreamStatus| {
            let persistence = persistence.clone();
            let notifier = notifier.clone();
            let channel_id = channel_id;
            let stream_url = stream_url.clone();
            let key = key.clone();

            Box::pin(async move {
                let state = persistence.get_stream_state(subscription_id).await?;
                match notifier.notify(status, state, channel_id, stream_url, key).await? {
                    Some(StateChange::Online { message_id }) => {
                        persistence.mark_subscription_online(subscription_id, message_id).await?
                    }
                    Some(StateChange::Offline) => persistence.mark_subscription_offline(subscription_id).await?,
                    None => {}
                }
                Ok(())
            })
        }
    }
}
