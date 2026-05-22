use core::time;
use std::collections::HashMap;

use mira_core::Host;

use crate::{
    Context, Error,
    notifier::DiscordNotifier,
    templates::{error_embed, success_embed},
};
use poise::{
    CreateReply,
    serenity_prelude::{
        self as serenity, ComponentInteractionDataKind, CreateInteractionResponseMessage,
        CreateSelectMenuOption,
    },
};

#[poise::command(slash_command)]
pub async fn subscribe(
    ctx: Context<'_>,
    #[description = "Which key to subscribe to"] key: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // TODO: This should return an error instead
    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();

    println!("GuildId: {}", guild_id);

    let hosts: HashMap<i64, Host> = ctx
        .data()
        .persistence
        .get_hosts(guild_id.get() as i64)
        .await
        .unwrap() // TODO: Map this into an error
        .into_iter()
        .map(|h| (h.id, h))
        .collect();

    if hosts.is_empty() {
        let embed = error_embed(
            "Subscribe Failed",
            "No available hosts found. Please add a host first with /host",
        );

        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    println!("Available hosts: {:?}", hosts);

    let options = hosts
        .values()
        .map(|host| CreateSelectMenuOption::new(&host.url, host.id.to_string()))
        .collect();

    let reply = {
        let menu = serenity::CreateSelectMenu::new(
            "host-select",
            serenity::CreateSelectMenuKind::String { options },
        )
        .placeholder("Choose a host");

        let components = vec![serenity::CreateActionRow::SelectMenu(menu)];

        poise::CreateReply::default()
            .content("Pick an option")
            .components(components)
    };

    ctx.send(reply).await?;

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(ctx.serenity_context())
            .timeout(time::Duration::from_secs(120))
            .filter(move |i| i.user.id == user_id)
            .await
    {
        let selected = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values.get(0),
            _ => None,
        };

        if let Some(value) = selected {
            let host_id: i64 = value.parse().unwrap();
            let host = hosts[&host_id].clone();

            let http = ctx.serenity_context().http.clone();
            let persistence = ctx.data().persistence.clone();

            let subscription_id = ctx
                .data()
                .persistence
                .add_subscription(
                    key.clone(),
                    host.host_guild_id.clone(),
                    channel_id.get() as i64,
                    user_id.get() as i64,
                )
                .await
                .unwrap(); // TODO: Fix this

            let notifier = DiscordNotifier::new(
                host.clone(),
                key.clone(),
                subscription_id,
                http,
                persistence,
            );

            ctx.data().monitor.register_stream(
                host.url.clone(),
                host.auth_header.clone(),
                key.clone(),
                notifier.into_callback(),
            );

            println!("Subscribed! {subscription_id}");

            let embed = success_embed(
                "Success",
                format!(
                    "Subscribed to {}/{}. You will be notified in this channel when they are next online.",
                    host.url, key
                ),
            );

            let message = serenity::CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .content("")
                    .components(vec![]),
            );

            interaction
                .create_response(&ctx.serenity_context(), message)
                .await?;
        };
    }

    Ok(())
}
