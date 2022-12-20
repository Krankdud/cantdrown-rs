use lavalink_rs::LavalinkClient;
use nonzero_ext::nonzero;
use ratelimit_meter::{DirectRateLimiter, GCRA};
use serenity::{
    client::{ClientBuilder, Context},
    prelude::TypeMapKey,
};
use std::time::Duration;

pub struct YtdlRateLimitKey;

impl TypeMapKey for YtdlRateLimitKey {
    type Value = DirectRateLimiter<GCRA<std::time::Instant>>;
}

pub struct LavalinkKey;

impl TypeMapKey for LavalinkKey {
    type Value = LavalinkClient;
}

pub fn register(client_builder: ClientBuilder) -> ClientBuilder {
    let limiter = DirectRateLimiter::<GCRA>::new(nonzero!(5u32), Duration::from_secs(60));
    client_builder.type_map_insert::<YtdlRateLimitKey>(limiter)
}

pub async fn get_ytdl_limiter(ctx: &Context) -> DirectRateLimiter<GCRA<std::time::Instant>> {
    let data = ctx.data.read().await;
    let limiter = data
        .get::<YtdlRateLimitKey>()
        .expect("Rate limiter not initialized");
    limiter.clone()
}

pub trait RateLimiterInit {
    fn register_ratelimiters(self) -> Self;
}

impl RateLimiterInit for ClientBuilder<'_> {
    fn register_ratelimiters(self) -> Self {
        register(self)
    }
}
