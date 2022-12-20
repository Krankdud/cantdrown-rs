use crate::util::LavalinkKey;
use lavalink_rs::LavalinkClient;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[description("Tell cantdrown to join your voice channel")]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
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
    let (_, handler) = manager.join_gateway(guild_id, connect_to).await;

    match handler {
        Ok(connection_info) => {
            let data = ctx.data.read().await;
            let lavalink_client = data.get::<LavalinkKey>().unwrap().clone();
            lavalink_client
                .create_session_with_songbird(&connection_info)
                .await?;
        }
        Err(why) => {
            msg.channel_id
                .say(&ctx.http, format!("Error joining channel: {}", why))
                .await?;
        }
    };

    Ok(())
}

#[command]
#[description("Tell cantdrown to get out of your voice channel")]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
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

        {
            let data = ctx.data.read().await;
            let lavalink_client = data.get::<LavalinkKey>().unwrap().clone();
            lavalink_client.destroy(guild_id).await?;
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
    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Must provide a URL to a video or audio")
                .await?;
            return Ok(());
        }
    };

    let lavalink_client: LavalinkClient = {
        let data = ctx.data.read().await;
        data.get::<LavalinkKey>().unwrap().clone()
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    if let Some(_handler) = manager.get(guild_id) {
        let query_info = lavalink_client.auto_search_tracks(&url).await?;

        if query_info.tracks.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Could not find video")
                .await?;
            return Ok(());
        }

        for track in query_info.tracks.iter() {
            if let Err(why) = &lavalink_client.play(guild_id, track.clone()).queue().await {
                log::error!("{}", why);
                return Ok(());
            }
        }

        if query_info.tracks.len() > 1 {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Added {} songs to the queue", query_info.tracks.len()),
                )
                .await?;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Added song to queue: {}",
                        query_info.tracks[0].info.as_ref().unwrap().title
                    ),
                )
                .await?;
        }
    }

    Ok(())
}

#[command]
#[description("Skip the current song in the queue")]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let data = ctx.data.read().await;
    let lavalink_client = data.get::<LavalinkKey>().unwrap().clone();

    if let Some(track) = lavalink_client.skip(guild_id).await {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Skipped: {}", track.track.info.as_ref().unwrap().title),
            )
            .await?;
    }

    Ok(())
}

#[command]
#[description("Stop playing music in the voice channel")]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let data = ctx.data.read().await;
    let lavalink_client = data.get::<LavalinkKey>().unwrap().clone();

    lavalink_client.stop(guild_id).await?;

    Ok(())
}

#[command]
#[description("Get the URL for the current song")]
#[only_in(guilds)]
async fn current(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };
    let guild_id = guild.id;

    let data = ctx.data.read().await;
    let lavalink_client = data.get::<LavalinkKey>().unwrap().clone();

    if let Some(node) = lavalink_client.nodes().await.get(guild_id.as_u64()) {
        if let Some(track) = &node.now_playing {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Now playing: {}", track.track.info.as_ref().unwrap().title),
                )
                .await?;
        }
    }

    Ok(())
}
