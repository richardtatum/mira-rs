use core::time;

use crate::{Context, Error};
use poise::serenity_prelude::{
    self as serenity, ComponentInteractionDataKind, CreateInteractionResponseMessage,
    CreateSelectMenuOption,
};

#[poise::command(slash_command)]
pub async fn hello(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let response = format!("Hello {name}!");
    ctx.say(response).await?;
    Ok(())
}

#[derive(Debug, poise::Modal)]
struct SubscribeModal {
    token: Option<String>,
}

#[poise::command(slash_command)]
pub async fn subscribe(ctx: Context<'_>) -> Result<(), Error> {
    let hosts = vec!["https://b.siobud.com", "https://stream.tatum.sh"];
    let options = hosts
        .into_iter()
        .map(|host| CreateSelectMenuOption::new(host, host))
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

    let user_id = ctx.author().id;
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

        if let Some(host) = selected {
            println!("Selected host: {}", host);
            let message = serenity::CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(format!("Selected host: {host}"))
                    .components(vec![]),
            );

            interaction
                .create_response(&ctx.serenity_context(), message)
                .await?;
        };
    }

    Ok(())
}
