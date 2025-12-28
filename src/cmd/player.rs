//! Regular player command handler

use diesel::{
    dsl::{not, now, sum},
    prelude::{ExpressionMethods, QueryDsl},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use rand::seq::IndexedRandom;
use serenity::{
    client::Context,
    model::prelude::{GuildId, Message},
    utils::{Colour, MessageBuilder},
};
use tracing::{Instrument, info_span, instrument};

use super::*;
use crate::{
    error::Error,
    extensions::{ContextExt, MessageExt},
    models::*,
};

#[instrument(skip(_ctx, msg, conn))]
pub async fn skip(_ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> StringResult {
    let game = msg.game(conn).await?;

    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    if part.player_id != msg.author.id.to_string() {
        return Err(Error::NotYourTurn);
    }

    part.skip(conn, true).await?;

    Ok(Some(
        MessageBuilder::new()
            .push("A vos photos, ")
            .mention(&msg.author)
            .push(" passe la main !")
            .build(),
    ))
}

#[instrument(skip(ctx, msg, conn))]
pub async fn win(
    ctx: Context,
    msg: Message,
    conn: &mut AsyncPgConnection,
    force: bool,
) -> StringResult {
    let game = msg.game(conn).await?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    // Check that participation is valid
    if !force && part.player_id != msg.author.id.to_string() {
        return Err(Error::NotYourTurn);
    }
    if part.picture_url.is_none() {
        return Err(Error::YouPostedNoPic);
    }

    // Check that a single winner is mentioned
    let winner = match msg.mentions.as_slice() {
        [] => {
            // TODO this should be an error
            return Ok(Some(
                MessageBuilder::new().mention(&msg.author).push(", cékiki le gagnant ?").build(),
            ));
        }
        [winner] => winner,
        [..] => {
            // TODO this should be an error
            return Ok(Some(
                MessageBuilder::new()
                    .push("Hé ")
                    .mention(&msg.author)
                    .push(", tu serai pas un peu fada ? Un seul gagnant, un seul !")
                    .build(),
            ));
        }
    };

    // Check that winner is valid (neither current participant nor a bot)
    if winner.bot {
        return Err(Error::StfuBot);
    }
    if winner.id == msg.author.id && !force {
        // TODO this should be an error
        return Ok(Some(
            MessageBuilder::new()
                .mention(&msg.author)
                .push(" be like https://i.imgflip.com/12w3f0.jpg")
                .build(),
        ));
    }

    // Save the win
    let win = NewWin {
        player_id: &msg.author.id.0.to_string(),
        winner_id: &winner.id.0.to_string(),
        score: 1,
    };
    let win: Win = diesel::insert_into(win::table).values(win).get_result(conn).await?;
    println!("Saved win {win:?}");

    // Mark participation as won
    diesel::update(&part)
        .set((
            participation::is_win.eq(true),
            participation::won_at.eq(now),
            participation::win_id.eq(&win.id),
        ))
        .execute(conn)
        .await?;

    // Mark winner as new participant
    let part = NewParticipation { player_id: &win.winner_id, picture_url: None, game_id: &game.id };
    diesel::insert_into(participation::table)
        .values(part)
        .get_result::<Participation>(conn)
        .await?;

    let def = vec![];
    let data = ctx.data.read().await;
    let sentence = data
        .get::<crate::WinSentences>()
        .unwrap_or(&def)
        .choose(&mut rand::rng())
        .map(String::as_str)
        .unwrap_or("Bravo {}, à vous la main.")
        .split("{}")
        .collect::<Vec<_>>();
    let (left, right) = match sentence.as_slice() {
        [l, r, ..] => (*l, *r),
        _ => ("Bravo ", ", à vous la main."),
    };

    Ok(Some(MessageBuilder::new().push(left).mention(winner).push(right).build()))
}

pub async fn show(ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> CreateMessageResult {
    tracing::info!("Show command invoked");
    let Some((game, _)) = msg.game(conn).await? else { return Ok(None) };

    let page = msg.content.split(' ').nth(1).and_then(|p| p.parse().ok()).unwrap_or(1);
    if page < 1 {
        return Err(Error::InvalidPage);
    }

    let (title, board) = scoreboard_message(&ctx, conn, game, msg.guild_id.unwrap(), page).await?;

    Ok(Some(Box::new(move |m| m.embed(|e| e.title(title).colour(Colour::GOLD).fields(board)))))
}

pub async fn scoreboard_message(
    ctx: &Context,
    conn: &mut AsyncPgConnection,
    game: Game,
    guild: GuildId,
    page: usize,
) -> Result<(String, Vec<(String, i64, bool)>)> {
    let wins = win::table
        .group_by(win::winner_id)
        .select((sum(win::score), win::winner_id))
        .filter(not(win::reset))
        .inner_join(participation::table)
        .filter(participation::game_id.eq(&game.id))
        .order_by(sum(win::score).desc())
        .load::<(Option<i64>, String)>(conn)
        .await?;

    let per_page = 10;
    let page_count = wins.len() / per_page + 1;

    if page > page_count {
        return Err(Error::InvalidPage);
    }

    let cache = ctx.cache().await;
    let board = wins
        .into_iter()
        .skip((page - 1) * per_page)
        .take(per_page)
        .filter_map(|(score, id)| Some((score?, id)))
        .map(|(score, id)| (score, id.parse::<u64>().unwrap(), cache.clone(), info_span!("map_fn")))
        .enumerate()
        .map(|(i, (score, id, cache, span))| {
            async move {
                tracing::debug!("Scoreboard entry ({i}, ({score}, {id}))");
                let position = match i + 1 + ((page - 1) * per_page) as usize {
                    1 => "🥇".to_owned(),
                    2 => "🥈".to_owned(),
                    3 => "🥉".to_owned(),
                    p => p.to_string(),
                };
                let member = cache.member(&ctx, guild, id).await;
                let name = match member {
                    Ok(member) => Ok(member.display_name().to_string()),
                    Err(e) => {
                        tracing::warn!(
                        "Failed to fetch member #{i} {id}: {e}, falling back to fetching the user. \
                        Maybe the user left the guild?",
                    );
                        cache.user(&ctx, id).await.map(|user| user.name)
                    }
                };
                name.map(|name| (format!("{position}. {name}"), score, false))
            }
            .instrument(span)
        });

    let span = info_span!("wins_map");
    let board = futures::future::join_all(board)
        .instrument(span)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok((format!("👑 👑 👑 Scores ({page}/{page_count}) 👑 👑 👑"), board))
}

//#[instrument(skip(ctx, msg, conn))]
pub async fn pic(ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> CreateMessageResult {
    let game = msg.game(conn).await?;
    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    let player = part.player();
    let Some(url) = part.picture_url else {
        return Ok(Some(Box::new(move |m| {
            m.content(
                MessageBuilder::new()
                    .push("C'est au tour de ")
                    .mention(&player)
                    .push(" qui n'a pas encore posté de photo.")
                    .build(),
            )
        })));
    };

    let player = player.to_user(&ctx.http).instrument(info_span!("UserId::to_user")).await?;
    let nick = player
        .nick_in(&ctx.http, msg.guild_id.unwrap())
        .instrument(info_span!("User::nick_in"))
        .await
        .unwrap_or_else(|| player.name.clone());

    Ok(Some(Box::new(move |m| {
        m.embed(|e| e.author(|a| a.name(nick).icon_url(player.face())).image(url))
    })))
}

#[instrument(skip(_ctx, msg, conn))]
pub async fn change(_ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> StringResult {
    let game = msg.game(conn).await?;

    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    if part.player_id != msg.author.id.to_string() {
        return Err(Error::NotYourTurn);
    }

    diesel::update(&part)
        .set((
            participation::picture_url.eq(Option::<String>::None),
            participation::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;

    Ok(Some("Ok, ok, puisque t'insistes ...".to_owned()))
}
