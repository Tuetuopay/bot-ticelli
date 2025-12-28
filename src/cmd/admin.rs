/*!
 * Admin command handlers
 */

use chrono::{DateTime, Utc};
use diesel::{
    dsl::{not, now, sql},
    prelude::{ExpressionMethods, OptionalExtension, QueryDsl},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use itertools::Itertools;
use serenity::{client::Context, model::prelude::Message, utils::MessageBuilder};
use tracing::info;
use uuid::Uuid;

use super::*;
use crate::error::Error;
use crate::extensions::MessageExt;
use crate::models::*;

#[tracing::instrument(skip(_ctx, msg, conn))]
pub async fn reset(_ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> StringResult {
    tracing::info!("in reset handler");
    let game = msg.game(conn).await?;
    let (game, part) = match game {
        Some((game, part)) => (game, part),
        None => return Ok(None),
    };

    match msg.content.split(' ').collect::<Vec<_>>().as_slice() {
        [_] => Ok(Some("Pour confirmer le reset, envoie `!reset do`.".to_owned())),
        [_, "do"] => {
            let reset_id = Uuid::new_v4();
            let win_ids = participation::table
                .filter(participation::win_id.is_not_null())
                .filter(participation::game_id.eq(&game.id))
                .select(participation::win_id)
                .load::<Option<Uuid>>(conn)
                .await?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            diesel::update(win::table.filter(not(win::reset)).filter(win::id.eq_any(win_ids)))
                .set((win::reset.eq(true), win::reset_at.eq(now), win::reset_id.eq(reset_id)))
                .execute(conn)
                .await?;

            // Mark the current participation as skipped (if any)
            if let Some(part) = part {
                part.skip(conn, false).await?;
            }

            Ok(Some(format!("Scores reset avec ID {reset_id}")))
        }
        [_, "list"] => {
            use diesel::sql_types::Timestamptz;
            let resets = win::table
                .select((win::reset_id, sql::<Timestamptz>("max(reset_at) as rst")))
                .inner_join(participation::table)
                .filter(participation::game_id.eq(&game.id))
                .filter(win::reset)
                .group_by(win::reset_id)
                .order_by(sql::<Timestamptz>("rst"))
                .load::<(Option<Uuid>, DateTime<Utc>)>(conn)
                .await?
                .into_iter()
                .enumerate()
                .filter_map(|(i, (id, at))| id.map(|id| format!("{}. {id} à {at}", i + 1)))
                .join("\n");
            Ok(Some(format!("Resets:\n{resets}")))
        }
        [_, "cancel", id] => {
            let reset_id: Uuid = id.parse().map_err(|_| Error::InvalidResetId)?;
            diesel::update(win::table.filter(win::reset).filter(win::reset_id.eq(reset_id)))
                .set((
                    win::reset.eq(false),
                    win::reset_at.eq::<Option<DateTime<Utc>>>(None),
                    win::reset_id.eq::<Option<Uuid>>(None),
                ))
                .execute(conn)
                .await
                .optional()?
                .ok_or(Error::InvalidResetId)?;
            Ok(Some(format!("Reset {reset_id} annulé")))
        }
        [..] => Err(Error::UnknownArguments),
    }
}

#[tracing::instrument(skip(_ctx, msg, conn))]
pub async fn force_skip(_ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> StringResult {
    let game = msg.game(conn).await?;
    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    part.skip(conn, true).await?;

    Ok(Some(
        MessageBuilder::new()
            .push("A vos photos, ")
            .mention(&part.player())
            .push(" n'a plus la main, on y a coupé court !")
            .build(),
    ))
}

#[tracing::instrument(skip(_ctx, msg, conn))]
pub async fn start(_ctx: Context, msg: Message, conn: &mut AsyncPgConnection) -> StringResult {
    let game = msg.game(conn).await?;

    if game.is_some() {
        return Ok(Some("Il y a déjà une partie en cours dans ce chan".to_owned()));
    }

    let guild = match msg.guild_id {
        Some(guild) => guild,
        None => return Ok(None),
    };

    let game = NewGame {
        guild_id: &guild.to_string(),
        channel_id: &msg.channel_id.to_string(),
        creator_id: &msg.author.id.to_string(),
    };
    let game: Game = diesel::insert_into(game::table).values(game).get_result(conn).await?;
    info!("Created new game: {game:?}");

    Ok(Some("Partie démarrée !".to_owned()))
}
