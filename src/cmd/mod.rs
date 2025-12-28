/*!
 * Actual command handlers
 */

use serenity::builder::CreateMessage;

use crate::error::Result;

pub mod admin;
pub mod player;

pub type StringResult = Result<Option<String>>;
type CreateMessageClosure =
    Box<dyn for<'a, 'b> FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> + Send>;
type CreateMessageResult = Result<Option<CreateMessageClosure>>;
