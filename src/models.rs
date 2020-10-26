/*!
 * DB models for the bot
 */

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use crate::schema::{win, win::dsl};

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
