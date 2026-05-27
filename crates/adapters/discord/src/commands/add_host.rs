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
    Ok(())
}
