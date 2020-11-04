/*!
 * Actual discord client
 */

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use diesel::prelude::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl};
use itertools::Itertools;
use serenity::{
    client::{Context, EventHandler},
    framework::standard::{
        Args,
        CommandGroup,
        CommandResult,
        HelpOptions,
        help_commands,
        macros::{command, group, hook, help},
    },
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};
use uuid::Uuid;

use crate::PgPool;
use crate::error::ErrorResultExt;
use crate::extensions::{ConnectionExt, MessageExt};
use crate::messages::*;
use crate::models::*;
use crate::paginate::*;

pub struct Bot;

impl EventHandler for Bot {}

#[command("skip")]
#[description("Passer son tour.")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_skip(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let res = conn.async_transaction(crate::cmd::player::skip(ctx, msg, &conn));
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

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
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let res = conn.async_transaction(crate::cmd::player::win(ctx, msg, &conn));
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("show")]
#[description("Afficher le scoreboard")]
#[min_args(0)]
#[max_args(1)]
#[usage("[page]")]
#[example("1")]
#[help_available]
#[only_in(guild)]
async fn cmd_show(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let res = crate::cmd::player::show(ctx, msg, conn).await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.send_message(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("reset")]
#[description("Gère le reset des scores")]
#[usage("[do|list|cancel <id>]")]
#[min_args(1)]
#[max_args(2)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_reset(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let res = conn.async_transaction(crate::cmd::admin::reset(ctx, msg, &conn));
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("pic")]
#[description("Affiche l'image à deviner")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_pic(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let res = crate::cmd::player::pic(ctx, msg, conn).await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.send_message(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("force_skip")]
#[description("Force la main à passer")]
#[num_args(0)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_force_skip(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;
    let game = msg.game(&conn)?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => {
            no_participant(ctx, msg).await?;
            return Ok(())
        }
        None => return Ok(()),
    };

    part.skip(&conn)?;

    let content = MessageBuilder::new()
        .push("A vos photos, ")
        .mention(&part.player())
        .push(" n'a plus la main, on y a coupé court !")
        .build();
    msg.channel_id.say(&ctx.http, content).await?;
    Ok(())
}

#[help]
#[no_help_available_text("Commande inexistante")]
#[usage_sample_label("Exemple")]
#[guild_only_text("Pas de DM p'tit coquin 😏")]
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
#[commands(cmd_win, cmd_skip, cmd_show, cmd_reset, cmd_pic, cmd_force_skip)]
pub struct General;

#[hook]
pub async fn on_message(ctx: &Context, msg: &Message) {
    tokio::spawn(log_message(ctx.clone(), msg.clone()));

    if let Err(e) = _on_message(ctx, msg).await {
        println!("Failed to handle message: {}", e);
    }
}

async fn _on_message(ctx: &Context, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
    // Find picture attachment
    let attachment = match msg.attachments.iter().filter(|a| a.height.is_some()).next() {
        Some(att) => att,
        None => return Ok(()),
    };

    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;
    let game = msg.game(&conn)?;
    let (game, part) = match game {
        Some(s) => s,
        None => return Ok(()),
    };

    let part: Participation = if let Some(part) = part {
        // Check the participant
        if part.player_id != msg.author.id.to_string() {
            // Don't send any error message as this is annoying when people post guess pics etc
            return Ok(())
        }

        if part.picture_url.is_none() {
            diesel::update(&part)
                .set(par_dsl::picture_url.eq(&attachment.proxy_url))
                .get_result(&conn)?
        } else {
            pic_already_posted(ctx, msg).await?;
            return Ok(())
        }
    } else {
        // Create the participation itself as nobody has a hand
        let part = NewParticipation {
            player_id: &msg.author.id.to_string(),
            picture_url: Some(&attachment.proxy_url),
            game_id: &game.id,
        };
        diesel::insert_into(crate::schema::participation::table)
            .values(part)
            .get_result(&conn)?
    };

    println!("Saved participation {:?}", part);

    new_pic_available(ctx, msg).await?;

    return Ok(())
}

async fn log_message(ctx: Context, msg: Message) {
    let guild = match msg.guild_id {
        Some(guild) => match guild.name(&ctx.cache).await {
            Some(name) => format!("[{}]", name),
            None => "(unknown)".to_owned(),
        },
        None => "(DM)".to_owned(),
    };
    let chan = match msg.channel_id.name(&ctx.cache) .await{
        Some(name) => format!("#{}", name),
        None => "?#".to_owned(),
    };
    println!(
        "({}) {} {} @{}: {}",
        msg.id,
        guild,
        chan,
        msg.author.tag(),
        msg.content_safe(&ctx.cache).await,
    );
}
