/*!
 * Extensions to some builtin or external types
 */

use std::future::Future;

use diesel::{pg::PgConnection, result::Error};
use serenity::{client::Context, model::prelude::Message};

use crate::cache::Cache;
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

pub trait ConnectionExt {
    fn async_transaction<T, E: From<Error>, F: Future<Output = Result<T, E>>>(&self, future: F) -> F::Output;
}

impl ConnectionExt for PgConnection {
    fn async_transaction<T, E: From<Error>, F: Future<Output = Result<T, E>>>(&self, future: F) -> F::Output {
        tokio::task::block_in_place(|| {
            self.build_transaction().serializable().run(|| {
                tokio::runtime::Handle::current().block_on(future)
            })
        })
    }
}

#[serenity::async_trait]
pub trait ContextExt {
    async fn cache(&self) -> Cache;
}

#[serenity::async_trait]
impl ContextExt for Context {
    async fn cache(&self) -> Cache {
        self.data.read().await.get::<Cache>().unwrap().clone()
    }
}
