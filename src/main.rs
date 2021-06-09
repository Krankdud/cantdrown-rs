use flexi_logger::{Duplicate, Logger};
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

mod audio;
mod commands;
mod config;
mod util;

use commands::{doom::*, meta::*, role::*, song::*};
use config::doom::DoomConfigInit;
use util::RateLimiterInit;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        log::info!("Resumed");
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

    Logger::with_env_or_str("info")
        .log_to_file()
        .directory("logs")
        .duplicate_to_stderr(Duplicate::Warn)
        .start()
        .expect("Failed to initialize logger");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);
    let (owners, _bot_id) = match http.get_current_application_info().await {
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
        .register_ratelimiters()
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}
