/*!
 * Actual discord client
 */

use std::collections::HashSet;

use diesel::prelude::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl};
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
use crate::messages::*;
use crate::models::*;

pub struct Bot;

impl EventHandler for Bot {}

#[command("skip")]
#[description("Passer son tour.")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_skip(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;
    let part = Participation::get_current(&conn)?;

    let part = if let Some(part) = part {
        if part.player_id != msg.author.id.to_string() {
            not_your_turn(ctx, msg).await?;
            return Ok(())
        }
        part
    } else {
        no_participant(ctx, msg).await?;
        return Ok(())
    };

    diesel::update(&part)
        .set((par_dsl::is_skip.eq(true), par_dsl::skipped_at.eq(diesel::dsl::now)))
        .execute(&conn)?;

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
    // Check that a single winner is mentioned
    let winner = match msg.mentions.as_slice() {
        [] => {
            let content = MessageBuilder::new()
                .mention(&msg.author)
                .push(", cékiki le gagnant ?")
                .build();
            msg.channel_id.say(&ctx.http, content).await?;
            return Ok(())
        }
        [winner] => winner,
        [..] => {
            msg.channel_id.say(&ctx.http, MessageBuilder::new()
                .push("Hé ")
                .mention(&msg.author)
                .push(", tu serai pas un peu fada ? Un seul gagnant, un seul !")
                .build()
            ).await?;
            return Ok(())
        }
    };

    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let part = Participation::get_current(&conn)?;
    let part = if let Some(part) = part {
        part
    } else {
        no_participant(ctx, msg).await?;
        return Ok(())
    };

    // Check that participation is valid
    if part.player_id != msg.author.id.to_string() {
        not_your_turn(ctx, msg).await?;
        return Ok(())
    }
    if part.picture_url.is_none() {
        you_posted_no_pic(ctx, msg).await?;
        return Ok(())
    }

    // Check that winner is valid (neither current participant nor a bot)
    if winner.bot {
        stfu_bot(ctx, msg).await?;
        return Ok(())
    }
    if winner.id == msg.author.id {
        let contents = MessageBuilder::new()
            .mention(&msg.author)
            .push(" be like https://i.imgflip.com/12w3f0.jpg")
            .build();
        msg.channel_id.say(&ctx.http, contents).await?;
        return Ok(())
    }

    // Save the win
    let win = NewWin {
        player_id: &msg.author.id.0.to_string(),
        winner_id: &winner.id.0.to_string(),
    };
    let win: Win = diesel::insert_into(dsl::win).values(win).get_result(&conn)?;
    println!("Saved win {:?}", win);

    // Mark participation as won
    diesel::update(&part)
        .set((par_dsl::is_win.eq(true),
              par_dsl::won_at.eq(diesel::dsl::now),
              par_dsl::win_id.eq(&win.id)))
        .execute(&conn)?;

    // Mark winner as new participant
    let part = NewParticipation {
        player_id: &win.winner_id,
        picture_url: None,
    };
    diesel::insert_into(crate::schema::participation::table)
        .values(part)
        .get_result::<Participation>(&conn)?;

    let content = MessageBuilder::new()
        .push("Bravo ")
        .mention(winner)
        .push(", plus un dans votre pot à moutarde. A vous la main.")
        .build();
    msg.channel_id.say(&ctx.http, content).await?;

    Ok(())
}

#[command("show")]
#[description("Afficher le scoreboard")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_show(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let wins = dsl::win.select((diesel::dsl::sql("count(id) as cnt"), dsl::winner_id))
        .filter(dsl::reset.eq(false))
        .group_by(dsl::winner_id)
        .order_by(diesel::dsl::sql::<diesel::sql_types::BigInt>("cnt").desc())
        .limit(10)
        .load::<(i64, String)>(&conn)?;

    let board = wins.into_iter()
        .enumerate()
        .map(|(i, (score, id))| async move {
            let position = match i + 1 {
                1 => "🥇".to_owned(),
                2 => "🥈".to_owned(),
                3 => "🥉".to_owned(),
                p => p.to_string(),
            };
            let user_id = UserId(id.parse().unwrap());
            match user_id.to_user(ctx.http.clone()).await {
                Ok(user) => Ok((format!("{}. @{}", position, user.tag()), score.to_string(), false)),
                Err(e) => Err(e),
            }
        });

    let board = futures::future::join_all(board).await
        .into_iter().collect::<Result<Vec<_>, _>>()?;

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("👑 👑 👑 Scores 👑 👑 👑");
            e.colour(Colour::GOLD);
            e.fields(board);
            e
        });
        m
    }).await?;
    Ok(())
}

// TODO only allow admins
#[command("reset")]
#[description("Fait un reset des scores")]
#[num_args(0)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_reset(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let reset_id = Uuid::new_v4();
    diesel::update(dsl::win.filter(dsl::reset.eq(false)))
        .set((dsl::reset.eq(true),
              dsl::reset_at.eq(diesel::dsl::now),
              dsl::reset_id.eq(reset_id)))
        .execute(&conn)?;

    msg.channel_id.say(&ctx.http, format!("Scores reset avec ID {}", reset_id)).await?;

    Ok(())
}

// TODO only allow admins
#[command("cancel_reset")]
#[description("Annule un reset des scores")]
#[num_args(1)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_cancel_reset(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;

    let reset_id: Uuid = msg.content.split(' ').nth(1).unwrap().parse().unwrap();
    diesel::update(dsl::win.filter(dsl::reset.eq(true)).filter(dsl::reset_id.eq(reset_id)))
        .set((dsl::reset.eq(false),
              dsl::reset_at.eq::<Option<chrono::DateTime<chrono::Utc>>>(None),
              dsl::reset_id.eq::<Option<Uuid>>(None)))
        .execute(&conn)?;

    msg.channel_id.say(&ctx.http, format!("Reset {} annulé", reset_id)).await?;

    Ok(())
}

#[command("pic")]
#[description("Affiche l'image à deviner")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_pic(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await.get_mut::<PgPool>().unwrap().get()?;
    let part = Participation::get_current(&conn)?;

    let (part, url) = if let Some(part) = part {
        match part.picture_url.clone() {
            Some(url) => (part, url),
            None => {
                let content = MessageBuilder::new()
                    .push("C'est au tour de ")
                    .mention(&UserId(part.player_id.parse().unwrap()))
                    .push(", mais il a pas posté de photo.")
                    .build();
                msg.channel_id.say(&ctx.http, content).await?;
                return Ok(())
            }
        }
    } else {
        no_participant(ctx, msg).await?;
        return Ok(())
    };

    let player = UserId(part.player_id.parse().unwrap()).to_user(&ctx.http).await?;

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.author(|a| a.name(player.tag()).icon_url(player.face()))
                .image(url)
        })
    }).await?;

    Ok(())
}

#[help]
#[no_help_available_text("On a pas le cul sorti des ronces, y'a pas d'aide ...")]
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
#[commands(cmd_win, cmd_skip, cmd_show, cmd_reset, cmd_cancel_reset, cmd_pic)]
pub struct General;

#[hook]
pub async fn on_message(ctx: &Context, msg: &Message) {
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
    let part = Participation::get_current(&conn)?;

    let part: Participation = if let Some(part) = part {
        // Check the participant
        if part.player_id != msg.author.id.to_string() {
            not_your_turn(ctx, msg).await?;
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
            picture_url: Some(&attachment.proxy_url)
        };
        diesel::insert_into(crate::schema::participation::table)
            .values(part)
            .get_result(&conn)?
    };

    println!("Saved participation {:?}", part);

    new_pic_available(ctx, msg).await?;

    return Ok(())
}
