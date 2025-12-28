//! Hand-made cache for users and guild members as discord's apis are doing shit and the serenity
//! cache is empty.

use std::{collections::HashMap, sync::Arc};

use serenity::{
    Result,
    http::CacheHttp,
    model::prelude::{GuildId, Member, User, UserId},
    prelude::TypeMapKey,
};
use tokio::sync::RwLock;
use tracing::{instrument, trace};

#[derive(Clone, Debug, Default)]
pub struct Cache {
    users: Arc<RwLock<HashMap<UserId, User>>>,
    members: Arc<RwLock<HashMap<(GuildId, UserId), Member>>>,
}

impl Cache {
    #[instrument(skip(self, http, guild_id, user_id), err)]
    pub async fn member(
        &self,
        http: impl CacheHttp,
        guild_id: impl Into<GuildId>,
        user_id: impl Into<UserId>,
    ) -> Result<Member> {
        let guild_id = guild_id.into();
        let user_id = user_id.into();

        // Block to ensure the rwlock guard does not live further than this check
        {
            // Check the cache
            if let Some(member) = self.members.read().await.get(&(guild_id, user_id)) {
                trace!("Cache hit");
                return Ok(member.clone());
            }
        }

        trace!("Cache miss, fetching member from Discord");

        // Fetch the member from Discord
        let member = guild_id.member(http, user_id).await?;

        // Cache it and the user
        self.members.write().await.insert((guild_id, user_id), member.clone());
        self.users.write().await.insert(user_id, member.user.clone());

        Ok(member)
    }

    #[instrument(skip(self, http, user_id), err)]
    pub async fn user(&self, http: impl CacheHttp, user_id: impl Into<UserId>) -> Result<User> {
        let user_id = user_id.into();

        // Block to ensure the rwlock guard does not live further than this check
        {
            // Check the cache
            if let Some(user) = self.users.read().await.get(&user_id) {
                trace!("Cache hit");
                return Ok(user.clone());
            }
        }

        trace!("Cache miss, fetching user from Discord");

        // Fetch user from Discord
        let user = user_id.to_user(http).await?;

        // Cache it
        self.users.write().await.insert(user_id, user.clone());

        Ok(user)
    }

    pub(crate) async fn update(&self, member: Member) -> Option<Member> {
        self.users.write().await.insert(member.user.id, member.user.clone());
        self.members.write().await.insert((member.guild_id, member.user.id), member)
    }

    pub(crate) async fn batch_update(&self, members: Vec<Member>) {
        let mut user_cache = self.users.write().await;
        let mut member_cache = self.members.write().await;

        for member in members {
            user_cache.insert(member.user.id, member.user.clone());
            member_cache.insert((member.guild_id, member.user.id), member);
        }
    }
}

impl TypeMapKey for Cache {
    type Value = Cache;
}
