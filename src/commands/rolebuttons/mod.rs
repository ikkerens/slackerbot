use anyhow::Result;
use serenity::{
    all::{Command, CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
    model::Permissions,
};

use crate::commands::send_ephemeral_message;

mod add;
pub(crate) mod button;
pub(crate) mod post;
mod remove;

pub(super) async fn register(ctx: &Context) -> Result<()> {
    Command::create_global_command(
        ctx,
        CreateCommand::new("rolebuttons")
            .description("All role actions")
            .dm_permission(false)
            .default_member_permissions(Permissions::MANAGE_ROLES)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Adds a role to the rolebutton")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Role, "role", "The role to add to the rolebuttons")
                            .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::String, "emoji", "The emoji to use as the icon")
                            .required(true),
                    ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "remove",
                    "Removes a role from the rolebuttons",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "The role to remove from the rolebuttons",
                    )
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "post",
                "Re-creates the post with the role selection buttons in the current channel",
            )),
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_command(ctx: Context, cmd: CommandInteraction) -> Result<()> {
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
