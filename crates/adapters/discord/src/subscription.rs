use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use mira_core::{CoreError, Host, HostSubscription, PersistenceProvider, StreamStatus, Subscription};
use mira_stream_watcher::StreamWatcher;
use poise::serenity_prelude::{ChannelId, GuildId, Http, UserId};
use url::Url;
use uuid::Uuid;

use crate::notifier::{DiscordNotifier, StateChange};

pub struct SubscriptionHandler<P: PersistenceProvider + 'static> {
    watcher: StreamWatcher,
    notifier: DiscordNotifier,
    persistence: Arc<P>,
    thumbnail_interval: Option<Duration>,
    thumbnail_dir: PathBuf,
    cancel_tokens: Arc<DashMap<i64, tokio_util::sync::CancellationToken>>,
}

impl<P: PersistenceProvider> SubscriptionHandler<P> {
    pub fn new(http: Arc<Http>, persistence: Arc<P>) -> Self {
        let thumbnail_interval = std::env::var("THUMBNAIL_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        let thumbnail_dir = std::env::var("THUMBNAIL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("mira-thumbnails"));

        // Ensure thumbnail directory exists
        if thumbnail_interval.is_some() {
            std::fs::create_dir_all(&thumbnail_dir).ok();
        }

        Self {
            watcher: StreamWatcher::new(None),
            notifier: DiscordNotifier::new(http),
            persistence,
            thumbnail_interval,
            thumbnail_dir,
            cancel_tokens: Arc::new(DashMap::new()),
        }
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
        let callback = self.build_callback(subscription_id, channel_id, host.url.clone(), stream_url, key.clone());

        let token = self.watcher.watch(host.url.clone(), host.auth_header, key, callback)?;
        self.persistence.update_subscription_token(subscription_id, token).await?;

        Ok(())
    }

    pub async fn unsubscribe(&self, subscription: Subscription) -> Result<(), CoreError> {
        if let Some(token) = subscription.token {
            // If there is a token, deregister
            self.watcher.stop_watching(token)?;
        }

        // Cancel any running thumbnail capture loop for this subscription
        if let Some((_, token)) = self.cancel_tokens.remove(&subscription.id) {
            token.cancel();
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
            let callback = self.build_callback(subscription_id, channel_id, host.url.clone(), stream_url, key.clone());

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
        host_url: String,
        stream_url: String,
        key: String,
    ) -> impl Fn(StreamStatus) -> Pin<Box<dyn Future<Output = Result<(), CoreError>> + Send + 'static>> + Send + Sync + 'static
    {
        let persistence = self.persistence.clone();
        let notifier = self.notifier.clone();
        let thumbnail_interval = self.thumbnail_interval;
        let thumbnail_dir = self.thumbnail_dir.clone();
        let cancel_tokens = self.cancel_tokens.clone();

        move |status: StreamStatus| {
            let persistence = persistence.clone();
            let notifier = notifier.clone();
            let channel_id = channel_id;
            let stream_url = stream_url.clone();
            let key = key.clone();
            let host_url = host_url.clone();
            let cancel_tokens = cancel_tokens.clone();
            let thumbnail_dir = thumbnail_dir.clone();

            Box::pin(async move {
                let state = persistence.get_stream_state(subscription_id).await?;

                // Read thumbnail if configured
                let path = thumbnail_interval
                    .map(|_| thumbnail_path(&thumbnail_dir, &host_url, &key));
                let thumbnail = if let Some(ref p) = path {
                    tokio::fs::read(p).await.ok().filter(|b| !b.is_empty())
                } else {
                    None
                };

                match notifier.notify(status, state, channel_id, stream_url.clone(), key.clone(), thumbnail).await? {
                    Some(StateChange::Online { message_id }) => {
                        persistence.mark_subscription_online(subscription_id, message_id).await?;

                        if let (Some(interval), Some(path)) = (thumbnail_interval, path) {
                            let whep_url = format!("{host_url}/api/whep");
                            let auth = format!("Bearer {key}");

                            // Cancel any existing token for this subscription first (fixes C2)
                            if let Some((_, old)) = cancel_tokens.remove(&subscription_id) {
                                old.cancel();
                            }
                            let token = tokio_util::sync::CancellationToken::new();
                            cancel_tokens.insert(subscription_id, token.clone());

                            tokio::spawn(async move {
                                loop {
                                    tokio::select! {
                                        _ = token.cancelled() => break,
                                        _ = tokio::time::sleep(interval) => {
                                            maybe_capture(&path, interval, &whep_url, &auth).await;
                                        }
                                    }
                                }
                            });
                        }
                    }
                    Some(StateChange::Offline) => {
                        persistence.mark_subscription_offline(subscription_id).await?;

                        if let Some((_, token)) = cancel_tokens.remove(&subscription_id) {
                            token.cancel();
                        }
                        if let Some(path) = path {
                            tokio::fs::remove_file(&path).await.ok();
                        }
                    }
                    None => {}
                }
                Ok(())
            })
        }
    }
}

fn thumbnail_path(thumbnail_dir: &Path, host_url: &str, key: &str) -> PathBuf {
    let name = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("{host_url}/{key}").as_bytes(),
    );
    thumbnail_dir.join(format!("{name}.jpg"))
}

async fn maybe_capture(path: &Path, interval: Duration, whep_url: &str, auth: &str) {
    let is_fresh = path
        .metadata()
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().unwrap_or_default() < interval)
        .unwrap_or(false);

    if is_fresh { return; }

    // Touch before connecting — other tasks see a fresh mtime and skip this cycle.
    // ponytail: tiny check→touch race remains; window is microseconds and consequence
    // is at most two simultaneous WHEP connections once per interval.
    let _ = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .await;

    if let Ok(bytes) = mira_thumbnail::capture(whep_url, auth).await {
        tokio::fs::write(path, bytes).await.ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::fs;

    #[tokio::test]
    async fn maybe_capture_skips_fresh_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");

        // Write a fresh file
        fs::write(&path, b"fake jpeg content").await.unwrap();

        // Track whether capture was called by checking mtime doesn't change
        let mtime_before = path.metadata().unwrap().modified().unwrap();
        maybe_capture(&path, Duration::from_secs(300), "http://unused", "Bearer unused").await;
        let mtime_after = path.metadata().unwrap().modified().unwrap();

        assert_eq!(mtime_before, mtime_after, "should not have touched fresh file");
    }

    #[tokio::test]
    async fn maybe_capture_touches_stale_file_before_capture() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");

        // No file exists (stale)
        maybe_capture(&path, Duration::from_secs(1), "http://127.0.0.1:1/api/whep", "Bearer bad").await;

        // File should be created (touched) even though capture fails (no server)
        assert!(path.exists(), "file should be created before capture attempt");
    }
}
