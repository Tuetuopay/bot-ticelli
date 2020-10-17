/*!
 * Actual discord client
 */

use std::collections::HashSet;

use serenity::{
    client::{Context, EventHandler},
    framework::standard::{
        Args,
        CommandGroup,
        CommandResult,
        HelpOptions,
        help_commands,
        macros::{command, group, help},
    },
    model::prelude::{Message, UserId},
};

pub struct Bot;

impl EventHandler for Bot {}

#[command]
async fn win(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, format!("Yo looks like {:?} are winners", msg.mentions)).await?;

    Ok(())
}

#[help]
async fn cmd_help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>
) -> CommandResult {
    help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(win)]
pub struct General;
