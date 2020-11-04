/*!
 * Admin command handlers
 */

use chrono::{DateTime, Utc};
use itertools::Itertools;
use diesel::prelude::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl, OptionalExtension};
use serenity::{
    builder::CreateMessage,
    client::Context,
    model::prelude::{Message, UserId},
    utils::{Colour, MessageBuilder},
};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::extensions::MessageExt;
use crate::models::*;
use crate::paginate::*;
use crate::PgPooledConn;
use super::*;

pub async fn reset(ctx: &Context, msg: &Message, conn: &PgPooledConn) -> StringResult {
    let game = msg.game(conn)?;
    let (game, part) = match game {
        Some((game, part)) => (game, part),
        None => return Ok(None),
    };

    match msg.content.split(' ').collect::<Vec<_>>().as_slice() {
        [_, "do"] => {
            let reset_id = Uuid::new_v4();
            let win_ids = par_dsl::participation
                .filter(par_dsl::is_win.eq(true))
                .filter(par_dsl::game_id.eq(&game.id))
                .select(par_dsl::win_id)
                .load::<Option<Uuid>>(conn)?
                .into_iter()
                .filter_map(|id| id)
                .collect::<Vec<_>>();
            diesel::update(
                dsl::win
                    .filter(dsl::reset.eq(false))
                    .filter(dsl::id.eq(diesel::dsl::any(win_ids))))
                .set((dsl::reset.eq(true),
                      dsl::reset_at.eq(diesel::dsl::now),
                      dsl::reset_id.eq(reset_id)))
                .execute(conn)?;

            // Mark the current participation as skipped (if any)
            if let Some(part) = part {
                part.skip(conn)?;
            }

            Ok(Some(format!("Scores reset avec ID {}", reset_id)))
        }
        [_, "list"] => {
            use diesel::sql_types::Timestamptz;
            let resets = dsl::win
                .select((dsl::reset_id, diesel::dsl::sql::<Timestamptz>("max(reset_at) as rst")))
                .inner_join(par_dsl::participation)
                .filter(par_dsl::game_id.eq(&game.id))
                .filter(dsl::reset.eq(true))
                .filter(diesel::dsl::sql("true group by reset_id"))
                .order_by(diesel::dsl::sql::<Timestamptz>("rst"))
                .load::<(Option<Uuid>, DateTime<Utc>)>(conn)?
                .into_iter()
                .enumerate()
                .filter_map(|(i, (id, at))| match id {
                    Some(id) => Some(format!("{}. {} à {}", i + 1, id, at)),
                    _ => None,
                })
                .join("\n");
            Ok(Some(format!("Resets:\n{}", resets)))
        }
        [_, "cancel", id] => {
            let reset_id: Uuid = id.parse().map_err(|_| Error::InvalidResetId)?;
            diesel::update(dsl::win.filter(dsl::reset.eq(true)).filter(dsl::reset_id.eq(reset_id)))
                .set((dsl::reset.eq(false),
                      dsl::reset_at.eq::<Option<DateTime<Utc>>>(None),
                      dsl::reset_id.eq::<Option<Uuid>>(None)))
                .execute(conn)
                .optional()?
                .ok_or(Error::InvalidResetId)?;
            Ok(Some(format!("Reset {} annulé", reset_id)))
        }
        [..] => Err(Error::UnknownArguments),
    }
}
