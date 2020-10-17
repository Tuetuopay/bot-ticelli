/*!
 * Actual discord client
 */

use std::collections::HashSet;

use diesel::prelude::RunQueryDsl;
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

use crate::PgPool;
use crate::models::*;

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

#[command("win")]
#[description("Marquer un joueur comme gagnant")]
#[usage("<joueur>")]
#[example("@Tuetuopay#2939")]
#[num_args(1)]
#[help_available]
#[only_in(guild)]
async fn cmd_win(ctx: &Context, msg: &Message) -> CommandResult {
    match msg.mentions.as_slice() {
        [] => {
            let content = MessageBuilder::new()
                .mention(&msg.author)
                .push(", cékiki le gagnant ?")
                .build();
            msg.channel_id.say(&ctx.http, content).await?;
        }
        [winner] => {
            let content = MessageBuilder::new()
                .push("Bravo ")
                .mention(winner)
                .push(", plus un dans votre pot à moutarde. A vous la main.")
                .build();
            msg.channel_id.say(&ctx.http, content).await?;

            let conn = ctx.data.write().await
                .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
                .get().expect("Failed to connect to database");
            let win = NewWin {
                player_id: &msg.author.id.0.to_string(),
                winner_id: &winner.id.0.to_string(),
            };
            let win: Win = diesel::insert_into(dsl::win).values(win).get_result(&conn)
                .expect("Failed to save win to database");
            println!("Saved win {:?}", win);
        }
        [..] => {
            msg.channel_id.say(&ctx.http, MessageBuilder::new()
                .push("Hé ")
                .mention(&msg.author)
                .push(", tu serai pas un peu fada ? Un seul gagnant, un seul !")
                .build()
            ).await?;
        }
    }

    Ok(())
}

#[help]
#[no_help_available_text("On a pas le cul sorti des ronces, y'a pas d'aide ...")]
#[usage_sample_label("Exemple")]
#[guild_only_text("Pas de DM p'tit coquin :smirk:")]
#[command_not_found_text("V'là qu'il utilise une commande inexistante. Y'en a vraiment qui ont pas \
    la lumière à tous les étages ...")]
#[strikethrough_commands_tip_in_guild("~~`Les commandes barrées`~~ sont indispo parce qu'on avait pas envie.")]
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
#[commands(cmd_win, cmd_skip)]
pub struct General;
