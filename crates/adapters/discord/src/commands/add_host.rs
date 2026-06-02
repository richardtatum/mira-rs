use mira_core::PersistenceProvider;
use url::Url;

use crate::{
    Context, Error,
    templates::{error_embed, success_embed},
};
use poise::CreateReply;

#[poise::command(slash_command)]
pub async fn add_host<P: PersistenceProvider>(
    ctx: Context<'_, P>,
    #[description = "The URL of the host you wish to add"] url: String,
    #[description = "(Optional) The auth header of the host you wish to add"] auth: Option<String>,
) -> Result<(), Error> {
    let Ok(parsed_url) = Url::parse(&url) else {
        let embed = error_embed(
            "Invalid URL",
            "The provided URL could not be parsed. Please include a scheme (e.g. https://).",
        );
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    };

    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Subscribe Failed", "/add_host can only be ran from a server channel currently.");
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    };

    let user_id = ctx.author().id;

    let embed = match ctx.data().subscription_handler.add_host(parsed_url, auth, guild_id, user_id).await {
        Ok(_) => success_embed("Success", format!("Host {} added.", &url)),
        Err(error) => error_embed("Failed", error.message()),
    };

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
