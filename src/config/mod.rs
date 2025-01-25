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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_from_env() {
        // Set up test environment variables
        env::set_var("COGNITO_DOMAIN", "https://test.auth.region.amazoncognito.com");
        env::set_var("COGNITO_CLIENT_ID", "test-client-id");
        env::set_var("COGNITO_CLIENT_SECRET", "test-client-secret");
        env::set_var("SERVER_DOMAIN", "http://localhost:3000");
        env::set_var("PROTECTED_WEBSITE_URL", "https://test-website.com");
        env::set_var("PORT", "3000");

        // Test successful config creation
        let config = Config::from_env().unwrap();
        assert_eq!(config.cognito_domain, "https://test.auth.region.amazoncognito.com");
        assert_eq!(config.cognito_client_id, "test-client-id");
        assert_eq!(config.cognito_client_secret, "test-client-secret");
        assert_eq!(config.server_domain, "http://localhost:3000");
        assert_eq!(config.protected_website_url, "https://test-website.com");
        assert_eq!(config.port, 3000);

        // Test default port when PORT is not a valid number
        env::set_var("PORT", "invalid");
        let config = Config::from_env().unwrap();
        assert_eq!(config.port, 3000);

        // Test error when required variable is missing
        env::remove_var("COGNITO_DOMAIN");
        assert!(Config::from_env().is_err());
    }
}