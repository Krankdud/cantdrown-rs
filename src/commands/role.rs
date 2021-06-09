use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[description("Add a role.")]
#[aliases("create")]
#[usage("<role name>")]
#[only_in(guilds)]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let name = match args.single_quoted::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Must provide a name for the role")
                .await?;
            return Ok(());
        }
    };

    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };

    if let Some(role) = guild.role_by_name(&name) {
        msg.channel_id
            .say(&ctx.http, format!("\"{}\" already exists.", role.name))
            .await?;
        return Ok(());
    }

    let role = guild
        .create_role(&ctx.http, |r| r.name(name).hoist(false).mentionable(true))
        .await?;

    msg.channel_id
        .say(&ctx.http, format!("<@&{}> has been created!", role.id))
        .await?;

    Ok(())
}

#[command]
#[description("Assign yourself a role")]
#[aliases("give")]
#[usage("<role name>")]
#[only_in(guilds)]
async fn assign(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let name = match args.single_quoted::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Must provide a name for the role")
                .await?;
            return Ok(());
        }
    };

    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => {
            log::error!("Could not get guild");
            return Ok(());
        }
    };

    if let Some(role) = guild.role_by_name(&name) {
        let has_role = match msg.author.has_role(&ctx.http, guild.id, role).await {
            Ok(has_role) => has_role,
            Err(why) => {
                msg.channel_id
                    .say(&ctx.http, "Error finding user's roles")
                    .await?;
                log::error!("Couldn't get user's roles: {:?}", why);
                return Ok(());
            }
        };

        if let Ok(mut member) = guild.member(&ctx.http, msg.author.id).await {
            if has_role {
                member.remove_role(&ctx.http, role.id).await?;
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "<@{}>: You no longer have the \"{}\" role",
                            msg.author.id, role.name
                        ),
                    )
                    .await?;
            } else {
                member.add_role(&ctx.http, role.id).await?;
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "<@{}>: You now have the \"{}\" role",
                            msg.author.id, role.name
                        ),
                    )
                    .await?;
            }
        }
    } else {
        msg.channel_id.say(&ctx.http, "Could not find role").await?;
    }

    Ok(())
}
