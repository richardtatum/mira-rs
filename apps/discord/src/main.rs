use std::env;
use std::sync::Arc;

use mira_core::PersistenceProvider;
use mira_discord::{Data, Error, commands, subscription::SubscriptionHandler};
use mira_storage::SqliteClient;
use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() {
    let options = poise::FrameworkOptions {
        commands: commands::all(),
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {} for user {}", ctx.command().qualified_name, ctx.author().name)
            })
        },
        event_handler: |_ctx, event, _framework, data| Box::pin(event_handler(event, data)),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let database_url = env::var("DATABASE_URL").expect("Missing database url!");
                let persistence = SqliteClient::new(database_url).await?;
                persistence.migrate().await?;
                let persistence = Arc::new(persistence);
                let subscription_handler = SubscriptionHandler::new(ctx.http.clone(), persistence);

                // Restore any existing subscriptions from the db
                subscription_handler.restore_subscriptions().await?;

                Ok(Data { subscription_handler })
            })
        })
        .options(options)
        .build();

    let token = env::var("DISCORD_TOKEN").expect("Missing 'DISCORD_TOKEN' env var!");
    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client =
        serenity::ClientBuilder::new(token, intents).framework(framework).await.expect("Failed to create client!");

    client.start().await.expect("Failed to start client!");
}

async fn on_error<P: PersistenceProvider>(error: poise::FrameworkError<'_, Data<P>, Error>) {
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

pub async fn event_handler<P: PersistenceProvider>(event: &serenity::FullEvent, data: &Data<P>) -> Result<(), Error> {
    match event {
        serenity::FullEvent::GuildDelete { incomplete, full: _ } => {
            // If true, means discord outage and not an actual removal
            if incomplete.unavailable {
                return Ok(());
            }

            data.subscription_handler.remove_guild(incomplete.id).await?;
            return Ok(());
        }
        serenity::FullEvent::ChannelDelete { channel, messages: _ } => {
            data.subscription_handler.remove_channel(channel.guild_id, channel.id).await?;
            return Ok(());
        }
        serenity::FullEvent::MessageDelete { channel_id: _, deleted_message_id, guild_id } => {
            // If it wasn't a message in a guild, just ignore as thats all we support (currently)
            let Some(guild_id) = guild_id else {
                return Ok(());
            };

            data.subscription_handler.remove_message(*guild_id, *deleted_message_id).await?;
            return Ok(());
        }
        _ => return Ok(()),
    }
}
