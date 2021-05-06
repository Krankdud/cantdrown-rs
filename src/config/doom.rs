use serde::Deserialize;
use serenity::client::{ClientBuilder, Context};
use serenity::prelude::TypeMapKey;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DoomConfigKey;

impl TypeMapKey for DoomConfigKey {
    type Value = DoomConfig;
}

#[derive(Deserialize, Clone)]
pub struct DoomConfig {
    pub executable: String,
    pub arguments: String,
    pub base_name: String,

    pub iwads: IWads,
    pub wads_path: String,

    pub idgames_mirror: String,

    pub timeout: u64,
}

#[derive(Deserialize, Clone)]
pub struct IWads {
    pub doom: String,
    pub doom2: String,
    pub tnt: String,
    pub plutonia: String,
}

pub fn register(client_builder: ClientBuilder) -> ClientBuilder {
    let path = Path::new("./config/doom.toml");
    let mut file = File::open(&path).expect("Could not open doom config");

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .expect("Could not read doom config");

    let config: DoomConfig = toml::from_str(&buffer).expect("Could not parse doom config");
    client_builder.type_map_insert::<DoomConfigKey>(config)
}

pub async fn get(context: &Context) -> DoomConfig {
    let data = context.data.read().await;
    let config = data
        .get::<DoomConfigKey>()
        .expect("Doom config is not in TypeMap");
    config.clone()
}

pub trait DoomConfigInit {
    fn register_doom(self) -> Self;
}

impl DoomConfigInit for ClientBuilder<'_> {
    fn register_doom(self) -> Self {
        register(self)
    }
}
