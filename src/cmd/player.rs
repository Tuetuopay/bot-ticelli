/*!
 * Regular player command handler
 */

use diesel::prelude::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl};
use rand::seq::SliceRandom;
use serenity::{
    client::Context,
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};

use crate::error::Error;
use crate::extensions::MessageExt;
use crate::models::*;
use crate::paginate::*;
use crate::PgPooledConn;
use super::*;

pub async fn skip(ctx: &Context, msg: &Message, conn: &PgPooledConn) -> StringResult {
    let game = msg.game(conn)?;

    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    if part.player_id != msg.author.id.to_string() {
        return Err(Error::NotYourTurn)
    }

    part.skip(conn)?;

    Ok(Some(MessageBuilder::new()
        .push("A vos photos, ")
        .mention(&msg.author)
        .push(" passe la main !")
        .build()))
}

pub async fn win(ctx: &Context, msg: &Message, conn: &PgPooledConn, force: bool) -> StringResult {
    let game = msg.game(conn)?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    // Check that participation is valid
    if !force {
        if part.player_id != msg.author.id.to_string() {
            return Err(Error::NotYourTurn)
        }
    }
    if part.picture_url.is_none() {
        return Err(Error::YouPostedNoPic)
    }

    // Check that a single winner is mentioned
    let winner = match msg.mentions.as_slice() {
        [] => {
            // TODO this should be an error
            return Ok(Some(MessageBuilder::new()
                .mention(&msg.author)
                .push(", cékiki le gagnant ?")
                .build()))
        }
        [winner] => winner,
        [..] => {
            // TODO this should be an error
            return Ok(Some(MessageBuilder::new()
                .push("Hé ")
                .mention(&msg.author)
                .push(", tu serai pas un peu fada ? Un seul gagnant, un seul !")
                .build()))
        }
    };

    // Check that winner is valid (neither current participant nor a bot)
    if winner.bot {
        return Err(Error::StfuBot)
    }
    if winner.id == msg.author.id {
        // TODO this should be an error
        return Ok(Some(MessageBuilder::new()
            .mention(&msg.author)
            .push(" be like https://i.imgflip.com/12w3f0.jpg")
            .build()))
    }

    // Save the win
    let win = NewWin {
        player_id: &msg.author.id.0.to_string(),
        winner_id: &winner.id.0.to_string(),
    };
    let win: Win = diesel::insert_into(dsl::win).values(win).get_result(conn)?;
    println!("Saved win {:?}", win);

    // Mark participation as won
    diesel::update(&part)
        .set((par_dsl::is_win.eq(true),
              par_dsl::won_at.eq(diesel::dsl::now),
              par_dsl::win_id.eq(&win.id)))
        .execute(conn)?;

    // Mark winner as new participant
    let part = NewParticipation {
        player_id: &win.winner_id,
        picture_url: None,
        game_id: &game.id,
    };
    diesel::insert_into(crate::schema::participation::table)
        .values(part)
        .get_result::<Participation>(conn)?;

    let def = vec![];
    let data = ctx.data.read().await;
    let sentence = data.get::<crate::WinSentences>()
        .unwrap_or(&def)
        .choose(&mut rand::thread_rng())
        .map(String::as_str)
        .unwrap_or("Bravo {}, à vous la main.")
        .split("{}")
        .collect::<Vec<_>>();
    let (left, right) = match sentence.as_slice() {
        [l, r, ..] => (*l, *r),
        _ => ("Bravo ", ", à vous la main."),
    };

    Ok(Some(MessageBuilder::new()
        .push(left)
        .mention(winner)
        .push(right)
        .build()))
}

pub async fn show(ctx: &Context, msg: &Message, conn: PgPooledConn) -> CreateMessageResult {
    let game = msg.game(&conn)?;
    let game = match game {
        Some((game, _)) => game,
        None => return Ok(None),
    };

    let page = msg.content.split(' ').nth(1).map(|p| p.parse().ok()).flatten().unwrap_or(1);
    let per_page = 10;

    if page < 1 {
        return Err(Error::InvalidPage)
    }

    let (wins, count) = dsl::win.select((diesel::dsl::sql("count(win.id) as cnt"), dsl::winner_id))
        .filter(dsl::reset.eq(false))
        .inner_join(par_dsl::participation)
        .filter(par_dsl::game_id.eq(&game.id))
        .group_by(dsl::winner_id)
        .order_by(diesel::dsl::sql::<diesel::sql_types::BigInt>("cnt").desc())
        .paginate(page)
        .per_page(per_page)
        .load_and_count::<_, (i64, String)>(&conn)?;
    let page_count = count / per_page + 1;

    if  page > page_count {
        return Err(Error::InvalidPage)
    }

    let board = wins.into_iter()
        .enumerate()
        .map(|(i, (score, id))| async move {
            let position = match i + 1 + ((page - 1) * per_page) as usize {
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
        .into_iter().collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Some(Box::new(move |m| {
        m.embed(|e| {
            e.title(format!("👑 👑 👑 Scores ({}/{}) 👑 👑 👑", page, page_count));
            e.colour(Colour::GOLD);
            e.fields(board);
            e
        });
        m
    })))
}

pub async fn pic(ctx: &Context, msg: &Message, conn: PgPooledConn) -> CreateMessageResult {
    let game = msg.game(&conn)?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    let player = part.player();
    let url = match part.picture_url {
        Some(url) => url,
        None => {
            return Ok(Some(Box::new(move |m| m.content(MessageBuilder::new()
                .push("C'est au tour de ")
                .mention(&player)
                .push(" qui n'a pas encore posté de photo.")
                .build()))))
        }
    };

    let player = player.to_user(&ctx.http).await?;

    Ok(Some(Box::new(move |m| {
        m.embed(|e| e.author(|a| a.name(player.tag()).icon_url(player.face())).image(url))
    })))
}
