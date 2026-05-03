//! Actual discord client

use diesel::{dsl::now, prelude::ExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poise::{FrameworkContext, FrameworkError};
use serenity::{
    all::{Colour, CreateEmbed, EditMessage, FullEvent, Interaction},
    client::{Context, EventHandler},
    model::prelude::{Attachment, Message},
};
use tracing::{debug, error, instrument};

use crate::{
    cmd::player::scoreboard_message,
    context::Data,
    error::{Error, Result},
    extensions::*,
    models::*,
};

pub const BTN_PREV: &str = "prev";
pub const BTN_NEXT: &str = "next";

pub struct Bot;

#[async_trait::async_trait]
impl EventHandler for Bot {}

#[instrument(skip(err))]
pub async fn on_error(err: FrameworkError<'_, Data, Error>) {
    let FrameworkError::Command { error, ctx, .. } = err else { return };

    if let Some(s) = error.as_message()
        && let Err(e) = ctx.say(s).await
    {
        error!("Failed to send error message: {e}");
    }
}

#[instrument(skip_all, err)]
pub async fn on_event<'a>(
    ctx: &'a Context,
    event: &'a FullEvent,
    _fw_ctx: FrameworkContext<'a, Data, Error>,
    data: &Data,
) -> Result<()> {
    match event {
        FullEvent::Ready { data_about_bot } => {
            *data.bot_user_id.lock().unwrap() = Some(data_about_bot.user.id);
        }
        FullEvent::GuildCreate { guild, .. } => {
            debug!("Guild created {:?}", guild.id);
            // List guild members
            let members = guild.members(ctx, Some(200), None).await?;
            data.cache.batch_update(members).await;
        }
        FullEvent::GuildMemberAddition { new_member } => {
            debug!("guild member added");
            data.cache.update(new_member.clone()).await;
        }
        FullEvent::GuildMemberUpdate { new, .. } => {
            debug!("guild member updated");
            if let Some(new) = new {
                data.cache.update(new.clone()).await;
            }
        }
        FullEvent::GuildMembersChunk { chunk } => {
            debug!("recieved guild member chunk with {} members", chunk.members.len());
            let members = chunk.members.values().cloned().collect();
            data.cache.batch_update(members).await;
        }
        FullEvent::InteractionCreate { interaction } => {
            on_interaction(ctx, interaction, data).await?
        }
        FullEvent::Message { new_message } => on_message(ctx, new_message, data).await,
        _ => (),
    }

    Ok(())
}

#[instrument(skip(ctx, msg, data))]
pub async fn on_message(ctx: &Context, msg: &Message, data: &Data) {
    tokio::spawn(log_message(ctx.clone(), msg.clone()));

    let Ok(mut conn) = data.pool.get().await else {
        // TODO raise to sentry
        msg.channel_id.say(ctx, "Erreur interne".to_owned()).await.unwrap();
        return;
    };

    let res = conn
        .build_transaction()
        .serializable()
        .run(async |conn| {
            // Find picture attachment
            let Some(attachment) = msg.attachments.iter().find(|a| a.height.is_some()) else {
                return Ok(None);
            };
            on_participation(msg, conn, attachment).await
        })
        .await;

    match res {
        Ok(Some(reply)) => {
            if let Err(e) = msg.channel_id.say(ctx, reply).await {
                error!("{e}");
            }
        }
        Ok(None) => (),
        Err(ref e) => {
            if let Some(s) = e.as_message()
                && let Err(e) = msg.channel_id.say(ctx, s).await
            {
                error!("{e}");
            }
        }
    }
}

#[instrument(skip(msg, conn, attachment))]
async fn on_participation(
    msg: &Message,
    conn: &mut AsyncPgConnection,
    attachment: &Attachment,
) -> Result<Option<String>> {
    // Find game itself
    let Some((game, part)) = msg.game(conn).await? else { return Ok(None) };

    let part: Participation = if let Some(part) = part {
        // Check the participant
        if part.player_id != msg.author.id.to_string() {
            // Don't send any error message as this is annoying when people post guess pics etc
            return Ok(None);
        }

        if part.picture_url.is_none() {
            diesel::update(&part)
                .set((
                    participation::picture_url.eq(&attachment.proxy_url),
                    participation::updated_at.eq(now),
                ))
                .get_result(conn)
                .await?
        } else {
            return Err(Error::PicAlreadyPosted);
        }
    } else {
        // Create the participation itself as nobody has a hand
        let part = NewParticipation {
            player_id: &msg.author.id.to_string(),
            picture_url: Some(&attachment.proxy_url),
            game_id: &game.id,
        };
        diesel::insert_into(participation::table).values(part).get_result(conn).await?
    };

    println!("Saved participation {part:?}");

    Ok(Some("🔎 À vos claviers, une nouvelle photo est à trouver".to_owned()))
}

async fn log_message(ctx: Context, msg: Message) {
    let guild = match msg.guild_id {
        Some(guild) => match guild.name(&ctx) {
            Some(name) => format!("[{name}]"),
            None => "(unknown)".to_owned(),
        },
        None => "(DM)".to_owned(),
    };
    let chan = match msg.channel_id.name(&ctx).await {
        Ok(name) => format!("#{name}"),
        Err(_) => "?#".to_owned(),
    };
    println!("({}) {guild} {chan} @{}: {}", msg.id, msg.author.tag(), msg.content_safe(&ctx));
}

async fn on_interaction(ctx: &Context, inter: &Interaction, data: &Data) -> Result<(), Error> {
    let Some(bot_id) = data.bot_user_id.lock().unwrap().as_ref().copied() else {
        tracing::warn!("Got interaction on message but bot is not cached");
        return Ok(());
    };

    let Interaction::Component(inter) = inter else { return Ok(()) };
    if inter.data.custom_id != BTN_PREV && inter.data.custom_id != BTN_NEXT {
        return Ok(());
    };
    // Discord lingo to ack the interaction and tell we'll edit the message.
    inter.defer(ctx).await?;

    let guild_id = inter.guild_id.unwrap();
    let Ok(mut conn) = data.pool.get().await else {
        // TODO raise to sentry
        inter.channel_id.say(ctx, "Erreur interne".to_owned()).await.unwrap();
        return Ok(());
    };
    let Some(game) = Game::get(&mut conn, guild_id.get(), inter.channel_id.get()).await? else {
        return Ok(());
    };

    if inter.message.author.id != bot_id {
        return Ok(());
    }

    let page = if let Some(embed) = inter.message.embeds.as_slice().first()
        && let Some(ref title) = embed.title
        && title.contains("Scores")
        && let Some(page) = title.split(['(', '/']).nth(1)
        && let Ok(page) = page.parse::<usize>()
    {
        page
    } else {
        return Ok(());
    };

    let page = if inter.data.custom_id == BTN_NEXT {
        page + 1
    } else if inter.data.custom_id == BTN_PREV && page > 1 {
        page - 1
    } else {
        return Ok(());
    };

    let (title, board) =
        match scoreboard_message(ctx, &mut conn, &data.cache, game, guild_id, page).await {
            Ok(res) => res,
            Err(Error::InvalidPage) => return Ok(()),
            Err(e) => return Err(e),
        };
    let embed = CreateEmbed::new().title(title).colour(Colour::GOLD).fields(board);
    let edit = EditMessage::new().embed(embed);
    inter.message.clone().edit(ctx, edit).await?;

    Ok(())
}
