use home::home_dir;
use serde::Deserialize;
use serenity::{
    client::{ClientBuilder, Context},
    prelude::TypeMapKey,
};
use std::{fs::File, io::Read, path::PathBuf};

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
    let path = get_config_path();
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

fn get_config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CANTDROWN_CONFIG_DIR") {
        let mut path = PathBuf::from(dir);
        path.push("doom.toml");
        path
    } else {
        let mut path = home_dir().expect("Could not find home directory");
        path.push(".config/cantdrown/doom.toml");
        path
    }
}

pub trait DoomConfigInit {
    fn register_doom(self) -> Self;
}

impl DoomConfigInit for ClientBuilder<'_> {
    fn register_doom(self) -> Self {
        register(self)
    }
}
