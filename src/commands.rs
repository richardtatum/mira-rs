use crate::{Context, Error};

pub fn test() {
    println!("Hello from commands!");
}

#[poise::command(slash_command)]
pub async fn hello(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let response = format!("Hello {name}!");
    ctx.say(response).await?;
    Ok(())
}
