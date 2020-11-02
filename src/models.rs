/*!
 * DB models for the bot
 */

use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, result::Error as DError};
use uuid::Uuid;

pub use crate::schema::{win, win::dsl, participation, participation::dsl as par_dsl};

#[derive(Queryable, Identifiable, Debug, Clone)]
#[table_name = "win"]
pub struct Win {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub player_id: String,
    pub winner_id: String,
    pub reset: bool,
    pub reset_at: Option<DateTime<Utc>>,
    pub reset_id: Option<Uuid>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "win"]
pub struct NewWin<'a> {
    pub player_id: &'a str,
    pub winner_id: &'a str,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[table_name = "participation"]
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
}

impl Participation {
    pub fn get_current(conn: &PgConnection) -> Result<Option<Participation>, DError> {
        let part = par_dsl::participation
            .filter(par_dsl::is_win.eq(false))
            .filter(par_dsl::is_skip.eq(false))
            .first::<Self>(conn);
        match part {
            Ok(part) => Ok(Some(part)),
            Err(e) => match e {
                DError::NotFound => Ok(None),
                e => Err(e),
            },
        }
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "participation"]
pub struct NewParticipation<'a> {
    pub player_id: &'a str,
    pub picture_url: Option<&'a str>,
}
