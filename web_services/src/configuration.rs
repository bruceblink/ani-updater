use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use std::path::PathBuf;

pub type TokenConfig = HashMap<String, i16>;

///---------------------- 顶层 Setting ----------------------
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
    pub token: TokenConfig,
}

///---------------------- 各类配置项 ----------------------
#[derive(serde::Deserialize, Clone, Debug)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
    pub max_connections: u32, // 数据库连接池最大连接数
}

/// 构造 PgConnectOptions
impl DatabaseSettings {
    /// 构造 PgConnectOptions
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }
}

///---------------------- 环境 ----------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }

    pub fn config_filename(&self) -> String {
        format!("{}.yaml", self.as_str())
    }
}

impl std::str::FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!("Unsupported environment: {}", other)),
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

///---------------------- 配置加载主函数 ----------------------
pub fn get_configuration(config_dir: Option<PathBuf>) -> Result<Setting, config::ConfigError> {
    dotenvy::dotenv().ok(); // 自动加载 .env（可选）

    // 配置目录
    let config_directory = config_dir
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configuration"));

    // 获取 APP_ENV=local/production
    let environment = get_environment()?;

    // 构造配置
    let settings = config::Config::builder()
        .add_source(config::File::from(config_directory.join("base.yaml")))
        .add_source(config::File::from(
            config_directory.join(environment.config_filename()),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize()
}

/// 从环境变量读取当前环境
fn get_environment() -> Result<Environment, config::ConfigError> {
    let env_str = std::env::var("APP_ENV").unwrap_or_else(|_| "production".into());

    env_str.parse().map_err(config::ConfigError::Message)
}

///---------------------- 测试 ----------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_from_str() {
        assert_eq!("local".parse(), Ok(Environment::Local));
        assert_eq!("production".parse(), Ok(Environment::Production));
        assert!("unknown".parse::<Environment>().is_err());
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Local.to_string(), "local");
        assert_eq!(Environment::Production.to_string(), "production");
    }

    #[test]
    fn test_token_config() {
        let config = get_configuration(Some(PathBuf::from("../configuration"))).unwrap();
        assert_eq!(config.token["access_token"], 20);
        assert_eq!(config.token["refresh_token"], 15);
    }
}
