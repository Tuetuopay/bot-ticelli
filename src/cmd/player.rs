//! Regular player command handler

use std::num::NonZeroUsize;

use diesel::{
    dsl::{not, now, sum},
    prelude::{ExpressionMethods, QueryDsl},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poise::CreateReply;
use rand::seq::IndexedRandom;
use serenity::{
    all::{Colour, Context, CreateEmbed, CreateEmbedAuthor, User},
    model::prelude::GuildId,
    utils::MessageBuilder,
};
use tracing::{Instrument, info_span, instrument};

use crate::{
    cache::Cache,
    context::Ctx,
    error::{Error, Result},
    extensions::HasGame,
    models::*,
};

#[instrument(skip(ctx, conn))]
pub async fn skip(ctx: Ctx<'_>, conn: &mut AsyncPgConnection) -> Result<Option<String>> {
    let game = ctx.game(conn).await?;

    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    if part.player_id != ctx.author().id.to_string() {
        return Err(Error::NotYourTurn);
    }

    part.skip(conn, true).await?;

    let msg = MessageBuilder::new()
        .push("A vos photos, ")
        .mention(ctx.author())
        .push(" passe la main !")
        .build();
    Ok(Some(msg))
}

#[instrument(skip(ctx, conn, winner))]
pub async fn win(
    ctx: Ctx<'_>,
    conn: &mut AsyncPgConnection,
    winner: User,
    force: bool,
) -> Result<Option<String>> {
    let game = ctx.game(conn).await?;
    let (game, part) = match game {
        Some((game, Some(part))) => (game, part),
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    // Check that participation is valid
    if !force && part.player_id != ctx.author().id.to_string() {
        return Err(Error::NotYourTurn);
    }
    if part.picture_url.is_none() {
        return Err(Error::YouPostedNoPic);
    }

    // Check that winner is valid (neither current participant nor a bot)
    if winner.bot {
        return Err(Error::StfuBot);
    }
    if winner.id == ctx.author().id && !force {
        // TODO this should be an error
        let msg = MessageBuilder::new()
            .mention(ctx.author())
            .push(" be like https://i.imgflip.com/12w3f0.jpg")
            .build();
        return Ok(Some(msg));
    }

    // Save the win
    let win = NewWin {
        player_id: &ctx.author().id.get().to_string(),
        winner_id: &winner.id.get().to_string(),
        score: 1,
    };
    let win: Win = diesel::insert_into(win::table).values(win).get_result(conn).await?;
    println!("Saved win {win:?}");

    // Mark participation as won
    diesel::update(&part)
        .set((
            participation::is_win.eq(true),
            participation::won_at.eq(now),
            participation::win_id.eq(&win.id),
        ))
        .execute(conn)
        .await?;

    // Mark winner as new participant
    let part = NewParticipation { player_id: &win.winner_id, picture_url: None, game_id: &game.id };
    diesel::insert_into(participation::table)
        .values(part)
        .get_result::<Participation>(conn)
        .await?;

    let data = ctx.data();
    let sentence = data
        .win_sentences
        .choose(&mut rand::rng())
        .map(String::as_str)
        .unwrap_or("Bravo {}, à vous la main.")
        .split("{}")
        .collect::<Vec<_>>();
    let (left, right) = match sentence.as_slice() {
        [l, r, ..] => (*l, *r),
        _ => ("Bravo ", ", à vous la main."),
    };

    Ok(Some(MessageBuilder::new().push(left).mention(&winner).push(right).build()))
}

#[instrument(skip(ctx, conn), err)]
pub async fn show(
    ctx: Ctx<'_>,
    conn: &mut AsyncPgConnection,
    page: Option<NonZeroUsize>,
) -> Result<Option<CreateReply>> {
    let Some((game, _)) = ctx.game(conn).await? else { return Ok(None) };

    let cache = &ctx.data().cache;
    let guild_id = ctx.guild_id().unwrap();
    let ctx = ctx.serenity_context();
    let page = page.map(|page| page.get()).unwrap_or(1);

    let (title, board) = scoreboard_message(ctx, conn, cache, game, guild_id, page).await?;

    let embed = CreateEmbed::new().title(title).colour(Colour::GOLD).fields(board);
    Ok(Some(CreateReply::default().embed(embed)))
}

pub async fn scoreboard_message(
    ctx: &Context,
    conn: &mut AsyncPgConnection,
    cache: &Cache,
    game: Game,
    guild: GuildId,
    page: usize,
) -> Result<(String, Vec<(String, String, bool)>)> {
    let wins = win::table
        .group_by(win::winner_id)
        .select((sum(win::score), win::winner_id))
        .filter(not(win::reset))
        .inner_join(participation::table)
        .filter(participation::game_id.eq(&game.id))
        .order_by(sum(win::score).desc())
        .load::<(Option<i64>, String)>(conn)
        .await?;

    let per_page = 10;
    let page_count = wins.len() / per_page + 1;

    if page > page_count {
        return Err(Error::InvalidPage);
    }

    let board = wins
        .into_iter()
        .skip((page - 1) * per_page)
        .take(per_page)
        .filter_map(|(score, id)| Some((score?, id)))
        .map(|(score, id)| (score, id.parse::<u64>().unwrap(), cache.clone(), info_span!("map_fn")))
        .enumerate()
        .map(|(i, (score, id, cache, span))| {
            async move {
                tracing::debug!("Scoreboard entry ({i}, ({score}, {id}))");
                let position = match i + 1 + ((page - 1) * per_page) {
                    1 => "🥇".to_owned(),
                    2 => "🥈".to_owned(),
                    3 => "🥉".to_owned(),
                    p => p.to_string(),
                };
                let member = cache.member(ctx, guild, id).await;
                let name = match member {
                    Ok(member) => Ok(member.display_name().to_string()),
                    Err(e) => {
                        tracing::warn!(
                        "Failed to fetch member #{i} {id}: {e}, falling back to fetching the user. \
                        Maybe the user left the guild?",
                    );
                        cache.user(ctx, id).await.map(|user| user.name)
                    }
                };
                name.map(|name| (format!("{position}. {name}"), score.to_string(), false))
            }
            .instrument(span)
        });

    let span = info_span!("wins_map");
    let board = futures::future::join_all(board)
        .instrument(span)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok((format!("👑 👑 👑 Scores ({page}/{page_count}) 👑 👑 👑"), board))
}

#[instrument(skip_all, err)]
pub async fn pic(ctx: Ctx<'_>, conn: &mut AsyncPgConnection) -> Result<Option<CreateReply>> {
    let game = ctx.game(conn).await?;
    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    let player = part.player();
    let Some(url) = part.picture_url else {
        let msg = MessageBuilder::new()
            .push("C'est au tour de ")
            .mention(&player)
            .push(" qui n'a pas encore posté de photo.")
            .build();
        return Ok(Some(CreateReply::default().content(msg)));
    };

    let player = player.to_user(&ctx).instrument(info_span!("UserId::to_user")).await?;
    let nick = player
        .nick_in(&ctx, ctx.guild_id().unwrap())
        .instrument(info_span!("User::nick_in"))
        .await
        .unwrap_or_else(|| player.name.clone());

    let author = CreateEmbedAuthor::new(nick).icon_url(player.face());
    let embed = CreateEmbed::new().author(author).image(url);
    let msg = CreateReply::default().embed(embed);
    Ok(Some(msg))
}

#[instrument(skip_all, err)]
pub async fn change(ctx: Ctx<'_>, conn: &mut AsyncPgConnection) -> Result<Option<String>> {
    let game = ctx.game(conn).await?;

    let part = match game {
        Some((_, Some(part))) => part,
        Some(_) => return Err(Error::NoParticipant),
        None => return Ok(None),
    };

    if part.player_id != ctx.author().id.to_string() {
        return Err(Error::NotYourTurn);
    }

    diesel::update(&part)
        .set((
            participation::picture_url.eq(Option::<String>::None),
            participation::updated_at.eq(now),
        ))
        .execute(conn)
        .await?;

    Ok(Some("Ok, ok, puisque t'insistes ...".to_owned()))
}
