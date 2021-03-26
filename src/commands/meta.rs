use rand::seq::SliceRandom;
use rand::thread_rng;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn drown(ctx: &Context, msg: &Message) -> CommandResult {
    let messages = [
        "I can't",
        "Impossible",
        "I tried, but it didn't work",
        "Is this Worms?",
    ];
    let choice: &str;
    {
        let mut rng = thread_rng();
        choice = messages.choose(&mut rng).unwrap();
    }

    msg.channel_id.say(&ctx.http, choice).await?;
    Ok(())
}
