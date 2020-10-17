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
    utils::MessageBuilder,
};

pub struct Bot;

impl EventHandler for Bot {}

#[command("skip")]
#[description("Passer son tour.")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_skip(ctx: &Context, msg: &Message) -> CommandResult {
    let content = MessageBuilder::new()
        .push("A vos photos, ")
        .mention(&msg.author)
        .push(" passe la main !")
        .build();
    msg.channel_id.say(&ctx.http, content).await?;
    Ok(())
}

#[command]
#[description("Marquer un joueur comme gagnant")]
#[usage("<joueur>")]
#[example("@Tuetuopay#2939")]
#[num_args(1)]
#[help_available]
#[only_in(guild)]
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
#[commands(win, cmd_skip)]
pub struct General;
