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
        macros::{command, group, help},
    },
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};
use uuid::Uuid;

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
    let conn = ctx.data.write().await
        .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
        .get().expect("Failed to connect to database");
    let part = Participation::get_current(&conn)
        .expect("Failed to fetch data from database");

    let part = if let Some(part) = part {
        if part.player_id != msg.author.id.to_string() {
            msg.channel_id.say(&ctx.http, "Tut tut tut, c'est pas toi qui a la main...").await?;
            return Ok(())
        }
        part
    } else {
        msg.channel_id.say(&ctx.http, "Mais personne n'a la main ...").await?;
        return Ok(())
    };

    diesel::update(&part)
        .set((par_dsl::is_skip.eq(true), par_dsl::skipped_at.eq(diesel::dsl::now)))
        .execute(&conn)
        .expect("Failed to save skip");

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

    let conn = ctx.data.write().await
        .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
        .get().expect("Failed to connect to database");

    let part = Participation::get_current(&conn).expect("Failed to fetch data from database");
    let part = if let Some(part) = part {
        if part.player_id != msg.author.id.to_string() {
            msg.channel_id.say(&ctx.http, "Tut tut tut, c'est pas toi qui a la main...").await?;
            return Ok(())
        }
        part
    } else {
        msg.channel_id.say(&ctx.http, "Mais personne n'a la main ...").await?;
        return Ok(())
    };

    let win = NewWin {
        player_id: &msg.author.id.0.to_string(),
        winner_id: &winner.id.0.to_string(),
    };
    let win: Win = diesel::insert_into(dsl::win).values(win).get_result(&conn)
        .expect("Failed to save win to database");
    println!("Saved win {:?}", win);

    diesel::update(&part)
        .set((par_dsl::is_win.eq(true),
              par_dsl::won_at.eq(diesel::dsl::now),
              par_dsl::win_id.eq(&win.id)))
        .execute(&conn)
        .expect("Failed to update participation in database");

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
    let conn = ctx.data.write().await
        .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
        .get().expect("Failed to connect to database");

    let wins = dsl::win.select((diesel::dsl::sql("count(id) as cnt"), dsl::winner_id))
        .filter(dsl::reset.eq(false))
        .group_by(dsl::winner_id)
        .order_by(diesel::dsl::sql::<diesel::sql_types::BigInt>("cnt").desc())
        .limit(10)
        .load::<(i64, String)>(&conn)
        .expect("Failed to load wins from the database");

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
        .into_iter().collect::<Result<Vec<_>, _>>()
        .expect("Failed to fetch users");

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("👑👑👑 Scores 👑👑👑");
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
async fn cmd_reset(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await
        .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
        .get().expect("Failed to connect to database");

    let reset_id = Uuid::new_v4();
    diesel::update(dsl::win.filter(dsl::reset.eq(false)))
        .set((dsl::reset.eq(true),
              dsl::reset_at.eq(diesel::dsl::now),
              dsl::reset_id.eq(reset_id)))
        .execute(&conn)
        .expect("Failed to update wins");

    msg.channel_id.say(&ctx.http, format!("Scores reset avec ID {}", reset_id)).await?;

    Ok(())
}

// TODO only allow admins
#[command("cancel_reset")]
#[description("Annule un reset des scores")]
#[num_args(1)]
#[only_in(guild)]
async fn cmd_cancel_reset(ctx: &Context, msg: &Message) -> CommandResult {
    let conn = ctx.data.write().await
        .get_mut::<PgPool>().expect("Failed to retrieve connection pool")
        .get().expect("Failed to connect to database");

    let reset_id: Uuid = msg.content.split(' ').nth(1).unwrap().parse().unwrap();
    diesel::update(dsl::win.filter(dsl::reset.eq(true)).filter(dsl::reset_id.eq(reset_id)))
        .set((dsl::reset.eq(false),
              dsl::reset_at.eq::<Option<chrono::DateTime<chrono::Utc>>>(None),
              dsl::reset_id.eq::<Option<Uuid>>(None)))
        .execute(&conn)
        .expect("Failed to update wins");

    msg.channel_id.say(&ctx.http, format!("Reset {} annulé", reset_id)).await?;

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
#[commands(cmd_win, cmd_skip, cmd_show, cmd_reset, cmd_cancel_reset)]
pub struct General;
