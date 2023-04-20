use anyhow::Result;
use sea_orm::{ConnectionTrait, EntityTrait, Statement};
use serenity::{
    client::Context,
    model::application::{
        command::{Command, CommandOptionType},
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
    },
};

use entity::prelude::Quote;

use crate::{commands::send_ephemeral_message, quote::post_quote, util::DatabaseTypeMapKey};

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("uquote")
            .description("Posts a random command by the specified user")
            .dm_permission(false)
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to get a random quote from")
                    .kind(CommandOptionType::User)
                    .required(true)
            })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let db = ctx.data.read().await.get::<DatabaseTypeMapKey>().unwrap().clone();
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};
    let user = match cmd.data.options.first().and_then(|id| id.resolved.clone()) {
        Some(CommandDataOptionValue::User(user, _)) => user,
        _ => return send_ephemeral_message(ctx, cmd, "No user received").await,
    };

    let quote = Quote::find()
        .from_raw_sql(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT * FROM "quote" WHERE "server_id" = $1 AND "author_id" = $2 ORDER BY RANDOM() LIMIT 1"#,
            vec![guild_id.0.into(), (user.id.0 as i64).into()],
        ))
        .one(&db)
        .await?;

    match quote {
        Some(quote) => post_quote(&ctx, quote, cmd.channel_id, Some(cmd)).await,
        None => send_ephemeral_message(ctx, cmd, "Could not find any random quotes, do none exist?").await,
    }
}
