/*!
 * DB models for the bot
 */

use chrono::{DateTime, Utc};
use diesel::{
    dsl::{not, now},
    prelude::*,
    result::Error as DError,
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serenity::model::id::{ChannelId, UserId};
use uuid::Uuid;

pub use crate::schema::{game, participation, win};

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = win)]
pub struct Win {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub player_id: String,
    pub winner_id: String,
    pub reset: bool,
    pub reset_at: Option<DateTime<Utc>>,
    pub reset_id: Option<Uuid>,
    pub score: i32,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = win)]
pub struct NewWin<'a> {
    pub player_id: &'a str,
    pub winner_id: &'a str,
    pub score: i32,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = participation)]
pub struct Participation {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub player_id: String,
    pub is_win: bool,
    pub won_at: Option<DateTime<Utc>>,
    pub win_id: Option<Uuid>,
    pub is_skip: bool,
    pub skipped_at: Option<DateTime<Utc>>,
    pub picture_url: Option<String>,
    pub game_id: Uuid,
    pub warned_at: Option<DateTime<Utc>>,
}

impl Participation {
    pub async fn get_current(
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<Participation>, DError> {
        let part = participation::table
            .filter(not(participation::is_win))
            .filter(not(participation::is_skip))
            .first::<Self>(conn)
            .await;
        match part {
            Ok(part) => Ok(Some(part)),
            Err(e) => match e {
                DError::NotFound => Ok(None),
                e => Err(e),
            },
        }
    }

    pub async fn skip(&self, conn: &mut AsyncPgConnection) -> Result<Self, DError> {
        diesel::update(self)
            .set((participation::is_skip.eq(true), participation::skipped_at.eq(now)))
            .get_result(conn)
            .await
    }

    pub fn player(&self) -> UserId {
        UserId(self.player_id.parse().unwrap())
    }
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = participation)]
pub struct NewParticipation<'a> {
    pub player_id: &'a str,
    pub picture_url: Option<&'a str>,
    pub game_id: &'a Uuid,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = game)]
pub struct Game {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub guild_id: String,
    pub channel_id: String,
    pub creator_id: String,
}

impl Game {
    pub async fn get(
        conn: &mut AsyncPgConnection,
        guild_id: u64,
        chan_id: u64,
    ) -> Result<Option<Game>, DError> {
        game::table
            .filter(game::guild_id.eq(guild_id.to_string()))
            .filter(game::channel_id.eq(chan_id.to_string()))
            .first(conn)
            .await
            .optional()
    }

    pub async fn get_with_part(
        conn: &mut AsyncPgConnection,
        guild_id: u64,
        chan_id: u64,
    ) -> Result<Option<(Game, Option<Participation>)>, DError> {
        game::table
            .filter(game::guild_id.eq(guild_id.to_string()))
            .filter(game::channel_id.eq(chan_id.to_string()))
            .left_join(
                participation::table.on(game::id
                    .eq(participation::game_id)
                    .and(not(participation::is_skip))
                    .and(not(participation::is_win))),
            )
            .first(conn)
            .await
            .optional()
    }

    pub fn channel(&self) -> ChannelId {
        ChannelId(self.channel_id.parse().unwrap())
    }
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = game)]
pub struct NewGame<'a> {
    pub guild_id: &'a str,
    pub channel_id: &'a str,
    pub creator_id: &'a str,
}
