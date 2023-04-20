use anyhow::Result;
use serenity::{
    client::Context,
    model::{
        application::{
            command::{Command, CommandOptionType},
            interaction::application_command::ApplicationCommandInteraction,
        },
        Permissions,
    },
};

use crate::commands::send_ephemeral_message;

mod add;
pub(crate) mod button;
pub(crate) mod post;
mod remove;

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_application_command(ctx, |command| {
        command
            .name("rolebuttons")
            .description("All role actions")
            .dm_permission(false)
            .default_member_permissions(Permissions::MANAGE_ROLES)
            .create_option(|option| {
                option
                    .name("add")
                    .description("Adds a role to the rolebuttons")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|option| {
                        option
                            .name("role")
                            .description("The role to add to the rolebuttons")
                            .kind(CommandOptionType::Role)
                            .required(true)
                    })
                    .create_sub_option(|option| {
                        option
                            .name("emoji")
                            .description("The emoji to use as the icon")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name("remove")
                    .description("Removes a role from the rolebuttons")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|option| {
                        option
                            .name("role")
                            .description("The role to remove from the rolebuttons")
                            .kind(CommandOptionType::Role)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name("post")
                    .description("Re-creates the post with the role selection buttons in the current channel")
                    .kind(CommandOptionType::SubCommand)
            })
    })
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: ApplicationCommandInteraction) -> Result<()> {
    let Some(guild_id) = cmd.guild_id else {return send_ephemeral_message(ctx, cmd, "This command can only be used in servers.").await};
    let Some(subcmd) = cmd.data.options.first().map(|o| o.name.as_str()) else {return send_ephemeral_message(ctx, cmd, "No subcommand passed").await};
    let channel_id = cmd.channel_id;
    match subcmd {
        "post" => post::handle(ctx, cmd, guild_id, channel_id).await?,
        "add" => add::handle(ctx, cmd, guild_id).await?,
        "remove" => remove::handle(ctx, cmd, guild_id).await?,
        _ => return send_ephemeral_message(ctx, cmd, "Unknown subcommand passed").await,
    }
    Ok(())
}
