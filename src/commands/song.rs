use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use songbird::input::restartable::Restartable;

#[command]
#[description("Tell cantdrown to join your voice channel")]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            println!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel")
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();
    let _handler = manager.join(guild_id, connect_to).await;

    Ok(())
}

#[command]
#[description("Tell cantdrown to get out of your voice channel")]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            println!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;
    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            msg.channel_id
                .say(&ctx.http, format!("Failed: {:?}", e))
                .await?;
        }
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel to play in")
            .await?;
    }

    Ok(())
}

#[command]
#[aliases("add", "queue")]
#[description("Play a song in the voice channel")]
#[usage("<url>")]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Must provide a URL to a video or audio")
                .await?;
            return Ok(());
        }
    };

    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            println!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match Restartable::ytdl(url, true).await {
            Ok(source) => source,
            Err(why) => {
                println!("Error starting source: {:?}", why);
                msg.channel_id
                    .say(&ctx.http, "Error sourcing ffmpeg")
                    .await?;
                return Ok(());
            }
        };

        handler.enqueue_source(source.into());

        let queue_len = handler.queue().len();
        if queue_len > 1 {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Added song to queue (songs in queue: {})", queue_len),
                )
                .await?;
        }
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel to play in")
            .await?;
    }

    Ok(())
}

#[command]
#[description("Skip the current song in the queue")]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            println!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
    }

    Ok(())
}

#[command]
#[description("Stop playing music in the voice channel")]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            println!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();
    }

    Ok(())
}
