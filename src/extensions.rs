/*!
 * Extensions to some builtin or external types
 */

use async_trait::async_trait;
use diesel::result::Error;
use diesel_async::{
    pooled_connection::deadpool::{Object, Pool, PoolError},
    AsyncPgConnection,
};
use serenity::{client::Context, model::prelude::Message};

use crate::{cache::Cache, models::*, PgPool};

#[async_trait]
pub trait MessageExt {
    async fn game(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<(Game, Option<Participation>)>, Error>;
}

#[async_trait]
impl MessageExt for Message {
    async fn game(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<(Game, Option<Participation>)>, Error> {
        Ok(match self.guild_id {
            Some(id) => Game::get_with_part(conn, *id.as_u64(), *self.channel_id.as_u64()).await?,
            None => None,
        })
    }
}

#[serenity::async_trait]
pub trait ContextExt {
    async fn cache(&self) -> Cache;
    async fn pool(&self) -> Pool<AsyncPgConnection>;
    async fn conn(&self) -> Result<Object<AsyncPgConnection>, PoolError>;
}

#[serenity::async_trait]
impl ContextExt for Context {
    async fn cache(&self) -> Cache {
        self.data.read().await.get::<Cache>().unwrap().clone()
    }
    async fn pool(&self) -> Pool<AsyncPgConnection> {
        self.data.read().await.get::<PgPool>().unwrap().clone()
    }
    async fn conn(&self) -> Result<Object<AsyncPgConnection>, PoolError> {
        self.pool().await.get().await
    }
}
