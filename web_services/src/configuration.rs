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
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub oauth_base_url: Option<String>,
    pub jwt_secret: Option<String>,
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
            other => Err(format!(
                "'{}' is not a supported environment. Use 'local' or 'production'.",
                other
            )),
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
    dotenvy::dotenv().ok(); // 加载 .env

    // 配置目录
    let config_directory = config_dir
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configuration"));

    // 获取 APP_ENV=local/production
    let environment = get_environment()?;

    // 构造加载器
    let settings = config::Config::builder()
        .add_source(config::File::from(config_directory.join("base.yaml")))
        .add_source(config::File::from(
            config_directory.join(environment.config_filename()),
        ))
        // ⬇ 支持嵌套环境变量 APP_FOO__BAR
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        // ⬇ 非 APP 前缀的环境变量也支持（如 POSTGRES_PASSWORD、DB_HOST）
        .add_source(config::Environment::default().separator("__"))
        .build()?;

    settings.try_deserialize()
}

/// 获取 APP_ENV 环境变量
fn get_environment() -> Result<Environment, config::ConfigError> {
    let env_str = std::env::var("APP_ENV").unwrap_or_else(|_| "production".into());

    env_str.parse().map_err(config::ConfigError::Message)
}

///---------------------- 测试 ----------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_environment_from_str() {
        assert_eq!("local".parse(), Ok(Environment::Local));
        assert_eq!("production".parse(), Ok(Environment::Production));
        assert!("unknown".parse::<Environment>().is_err());
    }

    #[test]
    fn test_token_config() {
        let config = get_configuration(Some(PathBuf::from("../configuration"))).unwrap();
        assert_eq!(config.token["access_token"], 20);
        assert_eq!(config.token["refresh_token"], 15);
    }

    #[test]
    fn test_database_settings_env_overrides() {
        let origin_settings = get_configuration(Some(PathBuf::from("../configuration"))).unwrap();

        let original: DatabaseSettings = origin_settings.database;

        // 设置环境变量 覆盖从配置文件中加载的配置
        unsafe {
            env::set_var("APP_DATABASE__HOST", "override_host");
            env::set_var("APP_DATABASE__USERNAME", "override_user");
            env::set_var("APP_DATABASE__MAX_CONNECTIONS", "20");
        }

        let overridden = get_configuration(Some(PathBuf::from("../configuration")))
            .unwrap()
            .database;

        assert_eq!(overridden.host, "override_host");
        assert_eq!(overridden.username, "override_user");
        assert_eq!(overridden.max_connections, 20);
        // 其他字段应该保持不变
        assert_eq!(overridden.port, original.port);
        assert_eq!(overridden.database_name, original.database_name);

        // 清理环境变量
        unsafe {
            env::remove_var("APP_DATABASE__HOST");
            env::remove_var("APP_DATABASE__USERNAME");
            env::remove_var("APP_DATABASE__MAX_CONNECTIONS");
        }
    }
}
