//! Actual command handlers

use std::num::NonZeroUsize;

use poise::{Command, CreateReply, builtins, command};
use serenity::all::{ReactionType, User};
use tracing::instrument;
use uuid::Uuid;

use crate::{context::Ctx, error::Result, extensions::CtxExt};

pub mod admin;
pub mod player;

pub fn commands() -> Vec<Command<crate::context::Data, crate::error::Error>> {
    vec![
        help(),
        skip(),
        win(),
        force_win(),
        show(),
        pic(),
        change(),
        reset(),
        force_skip(),
        start(),
    ]
}

/// Affiche ce menu
#[command(slash_command, prefix_command, track_edits, category = "Player")]
pub async fn help(ctx: Ctx<'_>, command: Option<String>) -> Result<()> {
    let config = builtins::HelpConfiguration {
        extra_text_at_bottom: "Tapez 'help <command>' pour plus d'informations sur la commande.",
        ..Default::default()
    };
    builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

/// Passer son tour
#[command(prefix_command, slash_command, category = "Player")]
#[instrument(skip(ctx), err, name = "skip")]
pub async fn skip(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let res = conn
        .build_transaction()
        .serializable()
        .run(async |conn| player::skip(ctx, conn).await)
        .await?;

    if let Some(reply) = res {
        ctx.say(reply).await?;
    }

    Ok(())
}

/// Marquer un joueur comme gagnant
#[command(prefix_command, slash_command, category = "Player")]
#[instrument(skip(ctx), err, name = "win")]
pub async fn win(ctx: Ctx<'_>, #[description = "Gagnant"] user: User) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let res = conn
        .build_transaction()
        .serializable()
        .run(async |conn| player::win(ctx, conn, user, false).await)
        .await?;
    if let Some(reply) = res {
        ctx.say(reply).await?;
    }

    Ok(())
}

/// Force la victoire d'un joueur
#[command(prefix_command, slash_command, required_permissions = "KICK_MEMBERS", category = "Admin")]
#[instrument(skip(ctx), err, name = "force_win")]
pub async fn force_win(ctx: Ctx<'_>, #[description = "Gagnant"] user: User) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let res = conn
        .build_transaction()
        .serializable()
        .run(async |conn| player::win(ctx, conn, user, true).await)
        .await?;
    if let Some(reply) = res {
        ctx.say(reply).await?;
    }

    Ok(())
}

/// Affiche le scoreboard
#[command(prefix_command, slash_command, category = "Player")]
#[instrument(skip(ctx), err, name = "show")]
pub async fn show(
    ctx: Ctx<'_>,
    #[description = "Page du scoreboard"] page: Option<NonZeroUsize>,
) -> Result<()> {
    let mut conn = ctx.conn().await?;
    if let Some(reply) = player::show(ctx, &mut conn, page).await? {
        let reply = ctx.send(reply).await?;
        let msg = reply.message().await?;
        msg.react(&ctx, ReactionType::Unicode("⬅️".to_owned())).await?;
        msg.react(&ctx, ReactionType::Unicode("➡️".to_owned())).await?;
    }
    Ok(())
}

/// Affiche l'image à deviner
#[command(prefix_command, slash_command, category = "Player")]
#[instrument(skip(ctx), err, name = "pic")]
async fn pic(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    if let Some(reply) = player::pic(ctx, &mut conn).await? {
        ctx.send(reply).await?;
    }
    Ok(())
}

/// Changer de photo, pour les indécis
#[command(prefix_command, slash_command, category = "Player")]
#[instrument(skip(ctx), err, name = "change")]
async fn change(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    if let Some(reply) = player::change(ctx, &mut conn).await? {
        ctx.reply(reply).await?;
    }
    Ok(())
}

/// Gère le reset des scores
#[command(
    prefix_command,
    slash_command,
    subcommands("reset_do", "reset_list", "reset_cancel"),
    required_permissions = "KICK_MEMBERS",
    category = "Admin"
)]
#[instrument(skip(ctx), err, name = "reset")]
async fn reset(ctx: Ctx<'_>) -> Result<()> {
    let reply = CreateReply::default().ephemeral(true).content("Usage: reset <do|list|cancel>");
    ctx.send(reply).await?;
    Ok(())
}

/// Effectue un reset des scores
#[command(
    prefix_command,
    slash_command,
    required_permissions = "KICK_MEMBERS",
    category = "Admin",
    rename = "do"
)]
#[instrument(skip(ctx), err, name = "do")]
async fn reset_do(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let reply = conn
        .build_transaction()
        .serializable()
        .run(async |conn| admin::reset_do(ctx, conn).await)
        .await?;
    if let Some(reply) = reply {
        ctx.send(CreateReply::default().ephemeral(true).content(reply)).await?;
    }
    Ok(())
}

/// Liste les différents reset du jeu
#[command(
    prefix_command,
    slash_command,
    required_permissions = "KICK_MEMBERS",
    category = "Admin",
    rename = "list"
)]
#[instrument(skip(ctx), err, name = "list")]
async fn reset_list(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    if let Some(reply) = admin::reset_list(ctx, &mut conn).await? {
        ctx.send(CreateReply::default().ephemeral(true).content(reply)).await?;
    }
    Ok(())
}

/// Annule un reset des scores
#[command(
    prefix_command,
    slash_command,
    required_permissions = "KICK_MEMBERS",
    category = "Admin",
    rename = "cancel"
)]
#[instrument(skip(ctx), err, name = "cancel")]
async fn reset_cancel(
    ctx: Ctx<'_>,
    #[description = "ID du reset à annuler"] id: Uuid,
) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let reply = conn
        .build_transaction()
        .serializable()
        .run(async |conn| admin::reset_cancel(ctx, conn, id).await)
        .await?;
    if let Some(reply) = reply {
        ctx.send(CreateReply::default().ephemeral(true).content(reply)).await?;
    }
    Ok(())
}

/// Force la main à passer
#[command(prefix_command, slash_command, required_permissions = "KICK_MEMBERS", category = "Admin")]
#[instrument(skip(ctx), err, name = "force_skip")]
async fn force_skip(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let reply = conn
        .build_transaction()
        .serializable()
        .run(async |conn| admin::force_skip(ctx, conn).await)
        .await?;
    if let Some(reply) = reply {
        ctx.say(reply).await?;
    }
    Ok(())
}

/// Démarre une nouvelle partie
#[command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Admin"
)]
#[instrument(skip(ctx), err, name = "start")]
async fn start(ctx: Ctx<'_>) -> Result<()> {
    let mut conn = ctx.conn().await?;
    let reply = conn
        .build_transaction()
        .serializable()
        .run(async |conn| admin::start(ctx, conn).await)
        .await?;
    if let Some(reply) = reply {
        ctx.say(reply).await?;
    }
    Ok(())
}
