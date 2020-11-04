/*!
 * Actual command handlers
 */

use crate::error::Result;
use serenity::builder::CreateMessage;

pub mod admin;
pub mod player;

type StringResult = Result<Option<String>>;
type CreateMessageClosure = Box<
    dyn for <'a, 'b> FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> + Send
>;
type CreateMessageResult = Result<Option<CreateMessageClosure>>;
