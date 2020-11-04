/*!
 * Regular player command handler
 */

use diesel::prelude::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl};
use serenity::{
    client::Context,
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};

use crate::error::{Error, Result};
use crate::extensions::MessageExt;
use crate::messages::*;
use crate::models::*;
use crate::PgPooledConn;

type StringResult = Result<Option<String>>;

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

pub async fn win(ctx: &Context, msg: &Message, conn: &PgPooledConn) -> StringResult {
    let game = msg.game(conn)?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    // Check that participation is valid
    if part.player_id != msg.author.id.to_string() {
        return Err(Error::NotYourTurn)
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

    Ok(Some(MessageBuilder::new()
        .push("Bravo ")
        .mention(winner)
        .push(", plus un dans votre pot à moutarde. A vous la main.")
        .build()))
}
