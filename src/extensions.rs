/*!
 * Extensions to some builtin or external types
 */

use diesel::result::Error;
use serenity::model::prelude::Message;

use crate::models::*;
use crate::PgPooledConn;

pub trait MessageExt {
    fn game(&self, conn: &PgPooledConn) -> Result<Option<(Game, Option<Participation>)>, Error>;
}

impl MessageExt for Message {
    fn game(&self, conn: &PgPooledConn) -> Result<Option<(Game, Option<Participation>)>, Error> {
        Ok(match self.guild_id {
            Some(id) => Game::get_with_part(conn, *id.as_u64(), *self.channel_id.as_u64())?,
            None => None,
        })
    }
}
