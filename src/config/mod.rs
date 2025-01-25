use serde::Deserialize;
use std::env;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub cognito_domain: String,
    pub cognito_client_id: String,
    pub cognito_client_secret: String,
    pub server_domain: String,
    pub protected_website_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            cognito_domain: env::var("COGNITO_DOMAIN")?,
            cognito_client_id: env::var("COGNITO_CLIENT_ID")?,
            cognito_client_secret: env::var("COGNITO_CLIENT_SECRET")?,
            server_domain: env::var("SERVER_DOMAIN")?,
            protected_website_url: env::var("PROTECTED_WEBSITE_URL")?,
            port: env::var("PORT")?.parse().unwrap_or(3000),
        })
    }
}