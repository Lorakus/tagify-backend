use crate::user_models::CreateUser;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Server {
    pub hostname: String,
    pub port: String,
    pub key: String,
    pub threads: usize
}

#[derive(Deserialize, Clone)]
pub struct TagifyData {
    pub path: String,
}

#[derive(Deserialize, Clone)]
pub struct MyConfig {
    pub postgres: deadpool_postgres::Config,
    pub server: Server,
    pub default_admin: CreateUser,
    pub default_user: CreateUser,
    pub tagify_data: TagifyData,
}

impl MyConfig {
    pub fn new(path: &str) -> Result<Self, config::ConfigError> {
        let mut settings = config::Config::default();
        settings.merge(config::File::with_name(path)).unwrap();
        settings.try_into()
    }
}
