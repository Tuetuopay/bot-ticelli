/*!
 * Errors that can be returned by bot commands and message handlers
 */

use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};

use serenity::{model::id::ChannelId, http::client::Http};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // Lib errors
    Db(diesel::result::Error),
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
        if let Some(error) = self.source() {
            write!(f, "{}", error)
        } else {
            Ok(())
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Db(e) => Some(e),
            Self::Serenity(e) => Some(e),
            _ => None,
        }
    }
}

impl Error {
    pub fn as_message(&self) -> Option<String> {
        match self {
            Self::Db(_) | Self::Serenity(_) => Some("Erreur interne".to_owned()),
            Self::NoParticipant => Some("⁉️ Mais personne n'a la main ...".to_owned()),
            Self::NotYourTurn => Some("❌ Tut tut tut, c'est pas toi qui a la main...".to_owned()),
            Self::YouPostedNoPic => Some("🤦 Hrmpf t'as pas mis de photo toi ...".to_owned()),
            Self::StfuBot => Some("🤖 Tg le bot !".to_owned()),
            Self::PicAlreadyPosted => Some("🦜 T'as déjà mis une photo coco.".to_owned()),
            Self::InvalidPage => Some("Page invalide".to_owned()),
            Self::InvalidResetId => Some("ID de reset invalide".to_owned()),
            Self::UnknownArguments => Some("Arguments inconnus".to_owned()),
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Error {
        Error::Db(e)
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
