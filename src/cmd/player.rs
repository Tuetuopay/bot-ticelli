/*!
 * Regular player command handler
 */

use serenity::{
    client::Context,
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};

use crate::extensions::MessageExt;
use crate::messages::*;
use crate::PgPooledConn;

type Result = crate::error::Result<()>;

pub async fn skip(ctx: &Context, msg: &Message, conn: &PgPooledConn) -> Result {
    let game = msg.game(conn)?;

    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => {
            no_participant(ctx, msg).await?;
            return Ok(())
        }
        None => return Ok(()),
    };

    if part.player_id != msg.author.id.to_string() {
        not_your_turn(ctx, msg).await?;
        return Ok(())
    }

    part.skip(conn)?;

    let content = MessageBuilder::new()
        .push("A vos photos, ")
        .mention(&msg.author)
        .push(" passe la main !")
        .build();
    msg.channel_id.say(&ctx.http, content).await?;
    Ok(())
}
