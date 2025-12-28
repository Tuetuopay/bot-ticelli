/*!
 * Errors that can be returned by bot commands and message handlers
 */

use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

use serenity::{http::client::Http, model::id::ChannelId};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    // Lib errors
    Db(diesel::result::Error),
    Pool(diesel_async::pooled_connection::deadpool::PoolError),
    Serenity(serenity::Error),

    // Errors from handlers
    NoParticipant,
    NotYourTurn,
    YouPostedNoPic,
    StfuBot,
    PicAlreadyPosted,
    InvalidPage,
    InvalidResetId,
    UnknownArguments,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if let Some(error) = self.source() { write!(f, "{error}") } else { Ok(()) }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Db(e) => Some(e),
            Self::Pool(e) => Some(e),
            Self::Serenity(e) => Some(e),
            _ => None,
        }
    }
}

impl Error {
    pub fn as_message(&self) -> Option<String> {
        let ret = match self {
            Self::Db(_) | Self::Pool(_) | Self::Serenity(_) => Some("Erreur interne"),
            Self::NoParticipant => Some("⁉️ Mais personne n'a la main ..."),
            Self::NotYourTurn => Some("❌ Tut tut tut, c'est pas toi qui a la main..."),
            Self::YouPostedNoPic => Some("🤦 Hrmpf t'as pas mis de photo toi ..."),
            Self::StfuBot => Some("🤖 Tg le bot !"),
            Self::PicAlreadyPosted => Some("🦜 T'as déjà mis une photo coco."),
            Self::InvalidPage => Some("Page invalide"),
            Self::InvalidResetId => Some("ID de reset invalide"),
            Self::UnknownArguments => Some("Arguments inconnus"),
        };
        ret.map(|s| s.to_owned())
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Error {
        Error::Db(e)
    }
}

impl From<diesel_async::pooled_connection::deadpool::PoolError> for Error {
    fn from(e: diesel_async::pooled_connection::deadpool::PoolError) -> Self {
        Error::Pool(e)
    }
}

impl From<serenity::Error> for Error {
    fn from(e: serenity::Error) -> Error {
        Error::Serenity(e)
    }
}

#[async_trait::async_trait]
pub trait ErrorResultExt: Send {
    async fn handle_err(self, chan: &ChannelId, http: &Http) -> Self;
}

#[async_trait::async_trait]
impl<T: Send> ErrorResultExt for Result<T> {
    async fn handle_err(self, chan: &ChannelId, http: &Http) -> Self {
        if let Err(ref e) = self {
            if let Some(s) = e.as_message() {
                chan.say(http, s).await?;
            }
        }
        self
    }
}
