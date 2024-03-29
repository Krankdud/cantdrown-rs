use flexi_logger::{Duplicate, Logger};
use lavalink_rs::{gateway::LavalinkEventHandler, LavalinkClient};
use serenity::{
    async_trait,
    framework::standard::{
        help_commands,
        macros::{group, help},
        Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
    },
    http::Http,
    model::{channel::Message, event::ResumedEvent, gateway::Ready, id::UserId},
    prelude::*,
};
use songbird::SerenityInit;
use std::{collections::HashSet, env};

mod commands;
mod config;
mod util;

use commands::{doom::*, meta::*, role::*, song::*};
use config::doom::DoomConfigInit;
use util::LavalinkKey;

struct Handler;
struct LavalinkHandler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        log::info!("Resumed");
    }
}

#[async_trait]
impl LavalinkEventHandler for LavalinkHandler {
    async fn track_start(&self, _client: LavalinkClient, event: lavalink_rs::model::TrackStart) {
        log::info!("Track started at guild: {}", event.guild_id);
    }

    async fn track_finish(&self, _client: LavalinkClient, event: lavalink_rs::model::TrackFinish) {
        log::info!("Track ended at guild: {}", event.guild_id);
    }
}

#[group]
#[commands(drown, host)]
struct General;

#[group]
#[description("Play music in voice channels\nUse 'join' to have cantdrown join your channel first, then use 'play'")]
#[commands(join, leave, play, skip, stop, current)]
#[default_command(current)]
#[prefix("song")]
struct Song;

#[group]
#[description("Add roles and assign roles to yourself")]
#[commands(add, assign)]
#[default_command(assign)]
#[prefix("role")]
struct Role;

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let log_path = env::var("CANTDROWN_LOG_DIR").unwrap_or(String::from("logs"));
    Logger::with_env_or_str("info")
        .log_to_file()
        .directory(log_path)
        .duplicate_to_stderr(Duplicate::Warn)
        .start()
        .expect("Failed to initialize logger");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&SONG_GROUP)
        .group(&ROLE_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .register_songbird()
        .register_doom()
        .await
        .expect("Error creating client");

    let lavalink_client = LavalinkClient::builder(bot_id)
        .set_password(env::var("LAVALINK_PASSWORD").unwrap_or(String::from("youshallnotpass")))
        .build(LavalinkHandler)
        .await
        .expect("Couldn't create lavalink client");

    {
        let mut data = client.data.write().await;
        data.insert::<LavalinkKey>(lavalink_client);
    }

    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}
