use mira_core::PersistenceProvider;

use crate::{
    Context, Error,
    templates::{error_embed, success_embed},
};
use poise::{
    CreateReply,
    serenity_prelude::{
        ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
};

#[poise::command(slash_command)]
pub async fn add_host<P: PersistenceProvider>(ctx: Context<'_, P>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Subscribe Failed", "/subscribe can only be ran from a server channel currently.");
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    };

    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();

    Ok(())
}
