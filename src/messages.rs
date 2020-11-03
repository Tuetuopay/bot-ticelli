/*!
 * Pre-defined messages
 */

use serenity::{client::Context, Error, model::prelude::Message};

type Result = std::result::Result<Message, Error>;

pub async fn not_your_turn(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "❌ Tut tut tut, c'est pas toi qui a la main...").await
}

pub async fn no_participant(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "⁉️ Mais personne n'a la main ...").await
}

pub async fn you_posted_no_pic(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "🤦 Hrmpf t'as pas mis de photo toi ...").await
}

pub async fn stfu_bot(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "🤖 Tg le bot !").await
}

pub async fn pic_already_posted(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "🦜 T'as déjà mis une photo coco.").await
}

pub async fn new_pic_available(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "🔎 À vos claviers, une nouvelle photo est à trouver").await
}

pub async fn invalid_page(ctx: &Context, msg: &Message) -> Result {
    msg.channel_id.say(&ctx.http, "Page invalide").await
}
