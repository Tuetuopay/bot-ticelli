//! Background jobs, like auto-skip

use std::{sync::Arc, time::Duration};

use diesel::{dsl::*, ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use serenity::{http::Http, utils::MessageBuilder};
use tokio::time::interval;
use tracing::{error, info, instrument};

use crate::{
    config::AutoskipConfig,
    error::Result,
    models::{game, participation, win, Game, NewWin, Participation, Win},
};

pub async fn task_auto_skip(
    http: Arc<Http>,
    pool: Pool<AsyncPgConnection>,
    config: AutoskipConfig,
) -> ! {
    let mut timer = interval(Duration::from_secs(60));
    loop {
        timer.tick().await;
        if let Err(e) = try_task_auto_skip(&http, &pool, &config).await {
            error!("task auto skip failed: {e}");
        }
    }
}

#[instrument(skip_all, err)]
async fn try_task_auto_skip(
    http: &Http,
    pool: &Pool<AsyncPgConnection>,
    config: &AutoskipConfig,
) -> Result<()> {
    let mut conn = pool.get().await?;

    let autoskip_delay = i64::from(config.autoskip_delay).seconds();
    let warn_delay = i64::from(config.warn_delay).seconds();

    let parts = participation::table
        .filter(not(participation::is_win))
        .filter(not(participation::is_skip))
        .filter(participation::warned_at.is_null())
        .filter((participation::updated_at + autoskip_delay - warn_delay).le(now))
        .filter((participation::updated_at + autoskip_delay).gt(now))
        .inner_join(game::table)
        .load::<(Participation, Game)>(&mut conn)
        .await?;
    for (part, game) in parts {
        diesel::update(&part).set(participation::warned_at.eq(now)).execute(&mut conn).await?;
        let m = MessageBuilder::new()
            .push("⏰ ")
            .mention(&part.player())
            .push(" ça va autoskip !")
            .build();
        game.channel().say(http, m).await?;
    }

    // Participations that we need to skip *now*
    let parts = participation::table
        .filter(not(participation::is_win))
        .filter(not(participation::is_skip))
        .filter((participation::updated_at + autoskip_delay).le(now))
        .inner_join(game::table)
        .load::<(Participation, Game)>(&mut conn)
        .await?;
    for (part, game) in parts {
        part.skip(&mut conn).await?;
        let win = NewWin { player_id: &part.player_id, winner_id: &part.player_id, score: -1 };
        let win = diesel::insert_into(win::table).values(win).get_result::<Win>(&mut conn).await?;

        diesel::update(&part)
            .set((
                participation::is_skip.eq(true),
                participation::skipped_at.eq(now),
                // Record negative win
                participation::win_id.eq(win.id),
            ))
            .execute(&mut conn)
            .await?;

        info!("Saved (negative) win {win:?}");

        let m = MessageBuilder::new()
            .push("Sorry ")
            .mention(&part.player())
            .push(", les gens sont nuls, prends ton point en moins ¯\\_(ツ)_/¯.")
            .build();
        game.channel().say(http, m).await?;
    }

    Ok(())
}
