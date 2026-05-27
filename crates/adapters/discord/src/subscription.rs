use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use mira_core::{CoreError, Host, PersistenceProvider, StreamStatus};
use mira_stream_watcher::StreamWatcher;
use poise::serenity_prelude::{ChannelId, Http, UserId};

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

    pub async fn get_hosts(&self, guild_id: i64) -> Result<Vec<Host>, CoreError> {
        self.persistence.get_hosts(guild_id).await
    }

    pub async fn subscribe(
        &self,
        host: Host,
        key: String,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<(), CoreError> {
        let subscription_id = self
            .persistence
            .add_subscription(key.clone(), host.host_guild_id, channel_id.get() as i64, user_id.get() as i64)
            .await?;

        let stream_url = format!("{}/{}", host.url, key);
        let callback = self.build_callback(subscription_id, channel_id, stream_url, key.clone());

        self.watcher.watch(host.url, host.auth_header, key, callback);

        println!("Subscribed! {subscription_id}");

        Ok(())
    }

    pub async fn restore_subscriptions(&self) -> Result<(), CoreError> {
        let subscriptions = self.persistence.get_all_subscriptions().await?;

        println!("Restoring {} subscription(s)...", subscriptions.len());

        for subscription in subscriptions {
            let host = subscription.host;
            let key = subscription.key;
            let subscription_id = subscription.subscription_id;
            let channel_id = ChannelId::new(subscription.channel_id as u64);
            let host_url = host.url.clone();

            let stream_url = format!("{}/{}", host.url, key);
            let callback = self.build_callback(subscription_id, channel_id, stream_url, key.clone());

            self.watcher.watch(host.url, host.auth_header, key.clone(), callback);

            println!("Restored {}/{} ({})", host_url, key, host.host_guild_id);
        }

        Ok(())
    }

    fn build_callback(
        &self,
        subscription_id: i64,
        channel_id: ChannelId,
        stream_url: String,
        key: String,
    ) -> impl Fn(StreamStatus) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync + 'static {
        let persistence = self.persistence.clone();
        let notifier = self.notifier.clone();

        move |status: StreamStatus| {
            let persistence = persistence.clone();
            let notifier = notifier.clone();
            let channel_id = channel_id;
            let stream_url = stream_url.clone();
            let key = key.clone();

            Box::pin(async move {
                let state = persistence.get_stream_state(subscription_id).await.unwrap();
                match notifier.notify(status, state, channel_id, stream_url, key).await.unwrap() {
                    Some(StateChange::Online { message_id }) => {
                        persistence.mark_subscription_online(subscription_id, message_id).await.unwrap()
                    }
                    Some(StateChange::Offline) => persistence.mark_subscription_offline(subscription_id).await.unwrap(),
                    None => {}
                }
            })
        }
    }
}
