/*!
 * Actual discord client
 */

use std::collections::HashSet;

use diesel::{dsl::now, prelude::ExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures::future::FutureExt;
use serenity::{
    client::{Context, EventHandler},
    framework::standard::{
        Args, CommandGroup, CommandResult, HelpOptions, help_commands,
        macros::{command, group, help, hook},
    },
    model::prelude::{
        Attachment, Guild, GuildMembersChunkEvent, Member, Message, Reaction, ReactionType, UserId,
    },
    utils::Colour,
};
use tracing::{Instrument, instrument};

use crate::{
    BotUserId,
    cmd::{StringResult, player::scoreboard_message},
    error::{Error, ErrorResultExt},
    extensions::*,
    models::*,
};

pub struct Bot;

#[serenity::async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, data: serenity::model::gateway::Ready) {
        ctx.data.write().await.insert::<BotUserId>(data.user.id);
    }

    #[instrument(skip(self, ctx, guild))]
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
        tracing::debug!("Guild created {:?}", guild.id);

        // List guild members
        match guild.members(&ctx, Some(200), None).await {
            Ok(members) => ctx.cache().await.batch_update(members).await,
            Err(e) => tracing::error!("Failed to fetch guild members: {e}"),
        }
    }

    #[instrument(skip(self, ctx))]
    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        tracing::debug!("guild member added");
        ctx.cache().await.update(member).await;
    }

    #[instrument(skip(self, ctx))]
    async fn guild_member_update(&self, ctx: Context, _old: Option<Member>, new: Member) {
        tracing::debug!("guild member updated");
        ctx.cache().await.update(new).await;
    }

    #[instrument(skip(self, ctx, chunk))]
    async fn guild_members_chunk(&self, ctx: Context, chunk: GuildMembersChunkEvent) {
        tracing::debug!("recieved guild member chunk with {} members", chunk.members.len());
        let members = chunk.members.into_iter().map(|(_, v)| v).collect();
        ctx.cache().await.batch_update(members).await;
    }

    #[instrument(skip(self, ctx, react))]
    async fn reaction_add(&self, ctx: Context, react: Reaction) {
        let res = on_reaction(&ctx, &react).await;
        if let Err(e) = res.handle_err(&react.channel_id, &ctx.http).await {
            tracing::error!("{e:?}");
        }
    }
}

#[hook]
pub async fn filter_command(_: &Context, msg: &Message, _: &str) -> bool {
    msg.attachments.is_empty()
}

#[command("skip")]
#[description("Passer son tour.")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
async fn cmd_skip(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_skip_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_skip")]
async fn cmd_skip_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::player::skip(ctx.clone(), msg.clone(), conn).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("win")]
#[description("Marquer un joueur comme gagnant")]
#[usage("<joueur>")]
#[example("@Tuetuopay#2939")]
#[num_args(1)]
#[help_available]
#[only_in(guild)]
async fn cmd_win(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_win_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_win")]
async fn cmd_win_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::player::win(ctx.clone(), msg.clone(), conn, false).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("show")]
#[description("Afficher le scoreboard")]
#[min_args(0)]
#[max_args(1)]
#[usage("[page]")]
#[example("1")]
#[help_available]
#[only_in(guild)]
#[bucket(show_limiter)]
async fn cmd_show(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_show_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_show")]
async fn cmd_show_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = crate::cmd::player::show(ctx.clone(), msg.clone(), &mut conn)
        .instrument(tracing::info_span!("show"))
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        let msg = msg.channel_id.send_message(&ctx.http, reply).await?;
        msg.react(&ctx, ReactionType::Unicode("⬅️".to_owned())).await?;
        msg.react(&ctx, ReactionType::Unicode("➡️".to_owned())).await?;
    }

    Ok(())
}

#[command("reset")]
#[description("Gère le reset des scores")]
#[usage("[do|list|cancel <id>]")]
#[min_args(1)]
#[max_args(2)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_reset(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_reset_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_reset")]
async fn cmd_reset_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::admin::reset(ctx.clone(), msg.clone(), conn).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("pic")]
#[description("Affiche l'image à deviner")]
#[num_args(0)]
#[help_available]
#[only_in(guild)]
#[bucket(pic_limiter)]
async fn cmd_pic(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_pic_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_pic")]
async fn cmd_pic_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = crate::cmd::player::pic(ctx.clone(), msg.clone(), &mut conn)
        .instrument(tracing::info_span!("pic"))
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.send_message(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("change")]
#[description("Changer de photo, pour les indécis")]
#[num_args(0)]
#[only_in(guild)]
async fn cmd_change(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_change_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_change")]
async fn cmd_change_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = crate::cmd::player::change(ctx.clone(), msg.clone(), &mut conn).await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("force_skip")]
#[description("Force la main à passer")]
#[num_args(0)]
#[only_in(guild)]
#[required_permissions(KICK_MEMBERS)]
async fn cmd_force_skip(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_force_skip_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_force_skip")]
async fn cmd_force_skip_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::admin::force_skip(ctx.clone(), msg.clone(), conn).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("start")]
#[description("Démarre une nouvelle partie")]
#[num_args(0)]
#[only_in(guild)]
#[required_permissions(ADMINISTRATOR)]
async fn cmd_start(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_start_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_start")]
async fn cmd_start_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::admin::start(ctx.clone(), msg.clone(), conn).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[command("force_win")]
#[description("Force une victoire d'un joueur")]
#[num_args(1)]
#[only_in(guild)]
#[required_permissions(KICK_MEMBERS)]
async fn cmd_force_win(ctx: &Context, msg: &Message) -> CommandResult {
    cmd_force_win_(ctx, msg).await
}

#[instrument(skip(ctx, msg), name = "cmd_force_win")]
async fn cmd_force_win_(ctx: &Context, msg: &Message) -> CommandResult {
    let mut conn = ctx.conn().await?;

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| crate::cmd::player::win(ctx.clone(), msg.clone(), conn, true).boxed())
        .await;
    if let Some(reply) = res.handle_err(&msg.channel_id, &ctx.http).await? {
        msg.channel_id.say(&ctx.http, reply).await?;
    }

    Ok(())
}

#[help]
#[no_help_available_text("Commande inexistante")]
#[usage_sample_label("Exemple")]
#[guild_only_text("Pas de DM p'tit coquin 😏")]
#[command_not_found_text(
    "V'là qu'il utilise une commande inexistante. Y'en a vraiment qui ont pas \
    la lumière à tous les étages ..."
)]
#[strikethrough_commands_tip_in_guild(
    "~~`Les commandes barrées`~~ sont indispo parce qu'on avait pas envie."
)]
async fn cmd_help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let span = tracing::info_span!("cmd_help", ?args, ?help_options, ?groups, ?owners);
    help_commands::with_embeds(ctx, msg, args, help_options, groups, owners)
        .instrument(span)
        .await?;
    Ok(())
}

#[group]
#[commands(
    cmd_win,
    cmd_skip,
    cmd_show,
    cmd_reset,
    cmd_pic,
    cmd_force_skip,
    cmd_start,
    cmd_force_win,
    cmd_change
)]
pub struct General;

#[hook]
pub async fn on_message(ctx: &Context, msg: &Message) {
    on_message_(ctx.clone(), msg.clone()).await
}

#[instrument(skip(ctx, msg), name = "on_message")]
pub async fn on_message_(ctx: Context, msg: Message) {
    tokio::spawn(log_message(ctx.clone(), msg.clone()));

    let mut conn = match ctx.conn().await {
        Ok(conn) => conn,
        Err(_e) => {
            // TODO raise to sentry
            msg.channel_id.say(&ctx.http, "Erreur interne".to_owned()).await.unwrap();
            return;
        }
    };

    let res = conn
        .build_transaction()
        .serializable()
        .run(|conn| {
            let msg = msg.clone();
            async move {
                // Find picture attachment
                let attachment = match msg.attachments.iter().find(|a| a.height.is_some()) {
                    Some(att) => att,
                    None => return Ok(None),
                };
                on_participation(&msg, conn, attachment).await
            }
            .boxed()
        })
        .await;

    if let Ok(Some(reply)) = res.handle_err(&msg.channel_id, &ctx.http).await {
        msg.channel_id.say(&ctx.http, reply).await.expect("Failed to send message");
    }
}

#[instrument(skip(msg, conn, attachment))]
async fn on_participation(
    msg: &Message,
    conn: &mut AsyncPgConnection,
    attachment: &Attachment,
) -> StringResult {
    // Find game itself
    let game = msg.game(conn).await?;
    let (game, part) = match game {
        Some(s) => s,
        None => return Ok(None),
    };

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
        Some(guild) => match guild.name(&ctx.cache) {
            Some(name) => format!("[{name}]"),
            None => "(unknown)".to_owned(),
        },
        None => "(DM)".to_owned(),
    };
    let chan = match msg.channel_id.name(&ctx.cache).await {
        Some(name) => format!("#{name}"),
        None => "?#".to_owned(),
    };
    println!("({}) {guild} {chan} @{}: {}", msg.id, msg.author.tag(), msg.content_safe(&ctx.cache));
}

async fn on_reaction(ctx: &Context, react: &Reaction) -> Result<(), Error> {
    let bot_id = match ctx.data.read().await.get::<BotUserId>() {
        Some(id) => id.to_owned(),
        None => {
            tracing::warn!("Got react on message but bot it not cached");
            return Ok(());
        }
    };
    if react.user_id == Some(bot_id) {
        return Ok(());
    }
    let guild_id = react.guild_id.unwrap();

    let mut conn = match ctx.conn().await {
        Ok(conn) => conn,
        Err(_e) => {
            // TODO raise to sentry
            react.channel_id.say(&ctx.http, "Erreur interne".to_owned()).await.unwrap();
            return Ok(());
        }
    };
    let game = match Game::get(&mut conn, guild_id.0, react.channel_id.0).await? {
        Some(game) => game,
        None => return Ok(()),
    };

    let mut msg = match ctx.cache.message(react.channel_id, react.message_id) {
        Some(msg) => msg,
        None => ctx.http.get_message(react.channel_id.0, react.message_id.0).await?,
    };
    // bug in serenity / discord: the fetched message has guild_id set to none. override it.
    msg.guild_id = Some(guild_id);

    if msg.author.id != bot_id {
        return Ok(());
    }
    let page = msg
        .embeds
        .get(0)
        .and_then(|embed| embed.title.as_ref())
        .filter(|title| title.contains("Scores"))
        .and_then(|title| title.split(|c| c == '(' || c == '/').nth(1))
        .and_then(|page| page.parse::<i64>().ok());
    let page = match page {
        Some(page) => page,
        None => return Ok(()),
    };

    let page = if react.emoji == ReactionType::Unicode("➡️".to_owned()) {
        page + 1
    } else if react.emoji == ReactionType::Unicode("⬅️".to_owned()) && page > 1 {
        page - 1
    } else {
        return Ok(());
    };

    let (title, board) = match scoreboard_message(&ctx, &mut conn, game, guild_id, page).await {
        Ok(res) => res,
        Err(Error::InvalidPage) => return Ok(()),
        Err(e) => return Err(e),
    };
    msg.edit(&ctx, |m| m.embed(|e| e.title(title).colour(Colour::GOLD).fields(board))).await?;

    Ok(())
}
