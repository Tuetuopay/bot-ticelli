/*!
 * Actual discord client
 */

use serenity::{
    async_trait,
    framework::Framework,
    model::{channel::Message, gateway::Ready},
    prelude::*
};

pub struct Bot {}

#[async_trait]
impl Framework for Bot {
    async fn dispatch(&self, ctx: Context, msg: Message) {
        match msg.content.split(' ').next() {
            Some("!win") => println!("Marking winner..."),
            Some(cmd) => println!("Unknown command {}", cmd),
            None => println!("wut?"),
        }
    }
}

impl Bot {
}
