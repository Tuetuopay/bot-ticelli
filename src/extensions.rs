//! Extensions to some builtin or external types

use diesel::result::Error;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::deadpool::{Object, PoolError},
};
use serenity::model::prelude::Message;

use crate::{context::Ctx, models::*};

pub trait HasGame {
    async fn game(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<(Game, Option<Participation>)>, Error>;
}

impl HasGame for Message {
    async fn game(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<(Game, Option<Participation>)>, Error> {
        Ok(match self.guild_id {
            Some(id) => Game::get_with_part(conn, id.get(), self.channel_id.get()).await?,
            None => None,
        })
    }
}

impl HasGame for Ctx<'_> {
    async fn game(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<(Game, Option<Participation>)>, Error> {
        let Some(guild_id) = self.guild_id() else { return Ok(None) };
        Game::get_with_part(conn, guild_id.get(), self.channel_id().get()).await
    }
}

pub trait CtxExt {
    async fn conn(&self) -> Result<Object<AsyncPgConnection>, PoolError>;
}

impl CtxExt for Ctx<'_> {
    async fn conn(&self) -> Result<Object<AsyncPgConnection>, PoolError> {
        self.data().pool.get().await
    }
}
