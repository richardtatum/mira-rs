use std::env;
use std::sync::Arc;

use mira_discord::{Data, Error, commands};
use mira_monitor::StreamMonitor;
use poise::serenity_prelude;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
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

#[tokio::main]
async fn main() {
    let options = poise::FrameworkOptions {
        commands: commands::all(),
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!(
                    "Executing command {} for user {}",
                    ctx.command().qualified_name,
                    ctx.author().name
                )
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let data = Data {
                    monitor: Arc::new(StreamMonitor::new(None)),
                };
                Ok(data)
            })
        })
        .options(options)
        .build();

    let token = env::var("DISCORD_TOKEN").expect("Missing 'DISCORD_TOKEN' env var!");
    let intents = serenity_prelude::GatewayIntents::non_privileged()
        | serenity_prelude::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity_prelude::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
