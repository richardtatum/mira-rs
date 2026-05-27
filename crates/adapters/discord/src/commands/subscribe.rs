use core::time;
use std::collections::HashMap;

use mira_core::{Host, PersistenceProvider};

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
pub async fn subscribe<P: PersistenceProvider>(
    ctx: Context<'_, P>,
    #[description = "Which key to subscribe to"] key: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Subscribe Failed", "/subscribe can only be ran from a server channel currently.");
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    };

    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();

    println!("GuildId: {}", guild_id);

    let hosts: HashMap<i64, Host> = ctx
        .data()
        .subscription_handler
        .get_hosts(guild_id.get() as i64)
        .await?
        .into_iter()
        .map(|h| (h.id, h))
        .collect();

    if hosts.is_empty() {
        let embed = error_embed("Subscribe Failed", "No available hosts found. Please add a host first with /host");

        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    println!("Available hosts: {:?}", hosts);

    let options = hosts.values().map(|host| CreateSelectMenuOption::new(&host.url, host.id.to_string())).collect();

    let reply = {
        let menu =
            CreateSelectMenu::new("host-select", CreateSelectMenuKind::String { options }).placeholder("Choose a host");

        let components = vec![CreateActionRow::SelectMenu(menu)];

        poise::CreateReply::default().content("Pick an option").components(components)
    };

    ctx.send(reply).await?;

    while let Some(interaction) = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.user.id == user_id)
        .await
    {
        let selected = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values.get(0),
            _ => None,
        };

        if let Some(value) = selected {
            let host_id: i64 = value.parse().expect("Selected host_id must be an i64");
            let host = hosts[&host_id].clone();
            let host_url = host.url.clone();

            ctx.data().subscription_handler.subscribe(host, key.clone(), user_id, channel_id).await;

            let embed = success_embed(
                "Success",
                format!(
                    "Subscribed to {}/{}. You will be notified in this channel when they are next online.",
                    host_url, key
                ),
            );

            let message = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().embed(embed).content("").components(vec![]),
            );

            interaction.create_response(&ctx.serenity_context(), message).await?;
        };
    }

    Ok(())
}
