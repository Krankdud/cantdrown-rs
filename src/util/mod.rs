use lavalink_rs::LavalinkClient;
use serenity::prelude::TypeMapKey;

pub struct LavalinkKey;

impl TypeMapKey for LavalinkKey {
    type Value = LavalinkClient;
}
