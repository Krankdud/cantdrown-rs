use crate::audio::{normalize::restartable_ytdl_normalized, playlist::get_playlist_videos};
use crate::util::get_ytdl_limiter;
use ratelimit_meter::NonConformance;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use tokio::time;

#[command]
#[description("Tell cantdrown to join your voice channel")]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
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
            log::error!("Could not get guild");
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

    let mut queue_len: usize = 0;
    if url.contains("youtube.com/playlist") {
        let videos = get_playlist_videos(&url).await?;
        for url in videos {
            if let Some(url) = url {
                queue_len = queue_song(ctx, msg, url).await?;
            }
        }
    } else {
        queue_len = queue_song(ctx, msg, url).await?;
    }

    if queue_len > 1 {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Added song to queue (songs in queue: {})", queue_len),
            )
            .await?;
    }

    Ok(())
}

async fn queue_song(ctx: &Context, msg: &Message, url: String) -> anyhow::Result<usize> {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(0);
        }
    };
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Couldn't get Songbird voice client")
        .clone();

    let mut limiter = get_ytdl_limiter(&ctx).await;
    let mut res = limiter.check();
    while let Err(negative) = res {
        let sleep_time = negative.wait_time_from(std::time::Instant::now());
        time::sleep(sleep_time).await;
        res = limiter.check();
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match restartable_ytdl_normalized(url, true).await {
            Ok(source) => source,
            Err(why) => {
                log::error!("Error starting source: {:?}", why);
                msg.channel_id.say(&ctx.http, "Couldn't queue song").await?;
                return Ok(0);
            }
        };

        handler.enqueue_source(source.into());

        let queue_len = handler.queue().len();
        Ok(queue_len)
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel to play in")
            .await?;
        Ok(0)
    }
}

#[command]
#[description("Skip the current song in the queue")]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
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
            log::error!("Could not get guild");
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

#[command]
#[description("Get the URL for the current song")]
#[only_in(guilds)]
async fn current(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
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

        if queue.is_empty() {
            msg.channel_id
                .say(&ctx.http, "There are no songs in the queue")
                .await?;
        } else {
            if let Some(track) = queue.current() {
                if let Some(url) = &track.metadata().source_url {
                    msg.channel_id.say(&ctx.http, format!("{}", url)).await?;
                } else {
                    msg.channel_id
                        .say(&ctx.http, "I have no idea what I'm playing right now!")
                        .await?;
                }
            }
        }
    }

    Ok(())
}
