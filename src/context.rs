//! Context type definition.

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use diesel_async::{AsyncPgConnection, pooled_connection::deadpool::Pool};
use serenity::all::UserId;

use crate::{cache::Cache, config::Config, error::Error};

#[derive(Clone)]
pub struct Data {
    pub pool: Pool<AsyncPgConnection>,
    pub win_sentences: Arc<Vec<String>>,
    pub cache: Cache,
    pub bot_user_id: Arc<Mutex<Option<UserId>>>,
}

impl Data {
    pub fn new(conf: &Config, pool: Pool<AsyncPgConnection>) -> Self {
        Self {
            pool,
            win_sentences: Arc::new(conf.bot_config.win_sentences.clone()),
            cache: Cache::default(),
            bot_user_id: Arc::new(Mutex::new(None)),
        }
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data").finish_non_exhaustive()
    }
}

pub type Ctx<'a> = poise::Context<'a, Data, Error>;
