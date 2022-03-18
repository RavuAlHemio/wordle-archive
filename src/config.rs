use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;

use log::error;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;


pub(crate) static CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();
pub(crate) static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();


#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Config {
    pub listen_addr: SocketAddr,
    pub base_path: String,
    pub db_conn_string: String,
    #[serde(default)] pub write_tokens: HashSet<String>,
}

pub(crate) fn load_config() -> Option<Config> {
    let config_path = match CONFIG_PATH.get() {
        Some(cp) => cp,
        None => {
            error!("cannot load config: CONFIG_PATH not set");
            return None;
        },
    };

    let mut config_file = match File::open(config_path) {
        Ok(cf) => cf,
        Err(e) => {
            error!("cannot load config: cannot open config file {}: {}", config_path.display(), e);
            return None;
        },
    };

    let mut buf = Vec::new();
    if let Err(e) = config_file.read_to_end(&mut buf) {
        error!("cannot load config: failed to read config file {}: {}", config_path.display(), e);
        return None;
    }

    let config = match toml::from_slice(&buf) {
        Ok(c) => c,
        Err(e) => {
            error!("cannot load config: failed to parse config file {}: {}", config_path.display(), e);
            return None;
        },
    };

    Some(config)
}
