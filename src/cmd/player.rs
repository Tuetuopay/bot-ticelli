/*!
 * Regular player command handler
 */

use serenity::{
    client::Context,
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};

use crate::error::{Error, Result};
use crate::extensions::MessageExt;
use crate::messages::*;
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
